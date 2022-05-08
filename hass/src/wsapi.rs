mod messenger;

use std::sync::{
    Arc,
};

use anyhow::anyhow;
use futures_util::{SinkExt, StreamExt};
use tokio::{
    net::TcpStream,
    sync::mpsc,
};
use tokio_tungstenite::{
    self,
    tungstenite,
    connect_async,
    MaybeTlsStream,
};
use tracing;
use tungstenite::{
    Message,
};
use url::Url;

use crate::error::{Error, Result};
use crate::json::{self, Id, WsMessage};
use crate::sync::{atomic::AtomicId, shutdown::Shutdown};

use messenger::{
    Command,
    WsApiMessenger
};

type WebSocketStream = tokio_tungstenite::WebSocketStream<MaybeTlsStream<TcpStream>>;

const MPSC_CHANNEL_BOUND: usize = 128;
const KEEPALIVE_INTERVAL_SEC: u64 = 15;



#[derive(Debug)]
pub struct WsApi {
    /// Endpoint URL
    url: Url,
    // Endpoint authentication token
    access_token: String,

    /// Transmission channel to send commands to the `WsApiMessenger`
    tx: mpsc::Sender<Command>,

    /// Receiver for all unhandled WsMessage, i.e. those not associated 
    /// to any `WsApi::registration()`
    unhandled_rx: Option<mpsc::Receiver<WsMessage>>,

    /// Next available identifier, to be used for `WsMessage` requests
    id: Arc<AtomicId>,
}

impl WsApi {

    /// Connects to a given `host` and `port` HA WebSocket endpoint with the provided
    /// and performs authentication with the `auth_token`.
    pub async fn new(secure: bool, host: &str, port: u16, access_token: &str, shutdown: Shutdown) -> Result<WsApi>
    {
        let scheme = if secure { "wss" } else { "ws" };
        let url = Url::parse(&format!("{}://{}:{}/api/websocket", scheme, host, port))?;

        //? What to do with you? I need to guarantee all new messages sent requiring IDs are
        //? properly taking new ids from here.
        let id = Arc::new(AtomicId::new());

        let id2 = id.clone();
        let socket = connect_ws(&url).await?;
        let (tx, rx) = mpsc::channel(MPSC_CHANNEL_BOUND);
        let (unhandled_tx, unhandled_rx) = mpsc::channel(MPSC_CHANNEL_BOUND);
        tokio::spawn(async move {
            let messenger = WsApiMessenger::new(rx, socket, id2, Some(unhandled_tx), shutdown);
            messenger.run().await;
            tracing::info!("messenger task terminated");
        });

        let mut api = WsApi {
            access_token: access_token.to_owned(),
            url,
            tx,
            id,
            unhandled_rx: Some(unhandled_rx),
        };

        api.authenticate().await?;

        Ok(api)
    }

    /// Connects to a given `host` and `port` with the provided authentication
    /// token `auth_token`.
    pub async fn new_unsecure(host: &str, port: u16, access_token: &str, shutdown: Shutdown) -> Result<WsApi> {
        Self::new(false, host, port, access_token, shutdown).await
    }

    /// Connects to a given `host` and `port` with the provided authentication
    /// token `auth_token`.
    pub async fn new_secure(host: &str, port: u16, access_token: &str, shutdown: Shutdown) -> Result<WsApi> {
        Self::new(true, host, port, access_token, shutdown).await
    }

    /// Returns the full URL of the WebSocket endpoint `self` is connected to.
    pub fn url(&self) -> &Url {
        &self.url
    }

    async fn recv_unhandled(&mut self) -> Result<WsMessage> {
        if let Some(unhandled_rx) = self.unhandled_rx.as_mut() {
            match unhandled_rx.recv().await {
                Some(msg) => {
                    Ok(msg)
                },
                None => {
                    self.unhandled_rx.take();
                    Err(Error::NoNextMessage)
                }
            }
        } else {
            Err(Error::InternalError {
                cause: anyhow!("unhandled_rx channel already closed")
            })
        }

    }

    async fn authenticate(&mut self) -> Result<()> {
        // Step 1. HA sends an auth_required message
        match self.recv_unhandled().await? {
            WsMessage::AuthRequired { ha_version } => {
                tracing::info!("authentication: received auth_required message from HA {}", &ha_version);
            },
            unexp => {
                tracing::error!("authentication: failed for unexpected message: {:?}", unexp);
                return Err(Error::UnexpectedMessage(unexp));
            }
        }

        // Step 2. We reply with an auth message complete with auth_token
        let auth_cmd = Command::Message(WsMessage::Auth { access_token: self.access_token.clone() });
        self.send_command(auth_cmd).await?;
        tracing::info!("authentication: auth_token sent");

        // Step 3. HA either validates the authentication with an auth_ok message, or
        //         rejects it with an auth_invalid message.
        match self.recv_unhandled().await? {
            WsMessage::AuthOk { .. } => {
                tracing::info!("authentication: successful");
                Ok(())
            },
            WsMessage::AuthInvalid {message} => {
                tracing::error!("authentication: failed ({})", message);
                Err(Error::Authentication(message))
            },
            unexp => {
                tracing::error!("authentication: failed for unexpected message: {:?}", unexp);
                Err(Error::UnexpectedMessage(unexp))
            },
        }
    }

    async fn send_command(&self, cmd: Command) -> Result<()> {
        match self.tx.send(cmd).await {
            Ok(()) => {
                Ok(())
            },
            Err(mpsc::error::SendError(cmd)) => Err(match cmd {
                Command::Message(msg) => Error::SendError(msg),
                cmd => Error::InternalError {
                    cause: anyhow!(mpsc::error::SendError(cmd))
                },
            }),
        }
    }

    async fn registration_ch(&self, tx: mpsc::Sender<WsMessage>) -> Result<Id> {
        let id = self.id.next();
        self.send_command(Command::Register(id, tx)).await?;
        Ok(id)
    }

    async fn registration(&self) -> Result<(Id, mpsc::Receiver<WsMessage>)> {
        let (tx, rx) = mpsc::channel(MPSC_CHANNEL_BOUND);
        self.registration_ch(tx).await.map(|(id)| { (id, rx) })
    }

    pub async fn subscribe_event(&self, event_type: Option<json::EventType>) -> Result<mpsc::Receiver<WsMessage>> {
        let (id, mut rx) = self.registration().await?;
        self.send_command(Command::Message(WsMessage::SubscribeEvents { id, event_type })).await?;

        let reply = rx.recv().await
            .ok_or(Error::InternalError { cause: anyhow!("missing response")})?;

        result_or_error(reply, rx)
    }

    pub async fn subscribe_events(&self, event_types: &[json::EventType]) -> Result<mpsc::Receiver<WsMessage>> {
        let (tx, mut rx) = mpsc::channel(MPSC_CHANNEL_BOUND);
        for event_type in event_types {
            let id = self.registration_ch(tx.clone()).await?;
            self.send_command(Command::Message(WsMessage::SubscribeEvents {
                id, event_type: Some(*event_type)
            })).await?;

            loop {
                let reply = rx.recv().await
                    .ok_or(Error::InternalError { cause: anyhow!("missing response") })?;
                match result_or_error(reply, ()) {
                    Ok(_) => break,
                    Err(Error::UnexpectedMessage(e)) => {
                        tracing::warn!("unexpected message dropped: {:?}", e);
                        continue;
                    },
                    Err(e) => return Err(e)
                }
            }
        }
        Ok(rx)
    }

}

fn result_or_error<T>(reply: WsMessage, result: T) -> Result<T> {
    match reply {
        WsMessage::Result { data: json::ResultBody { success: true, .. } } => {
            Ok(result)
        },
        WsMessage::Result { data: json::ResultBody { success: false, error, .. } } => {
            Err(Error::from(error))
        },
        u => {
            Err(Error::UnexpectedMessage(u))
        },
    }
}


async fn connect_ws(url: &Url) -> Result<WebSocketStream> {
    let (socket, response) = connect_async(url).await?;
    tracing::trace!("connect({}): {:?}", url, response);
    Ok(socket)
}




#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::shutdown;

    #[tokio::test]
    async fn new_unknown_host() {
        let manager = shutdown::Manager::new();
        match WsApi::new_unsecure("i.do.not.exist", 8123, "auth_token", manager.subscribe()).await {
            Err(Error::WebSocket(_)) => (), // OK
            x => panic!("unexpected result: {:?}", x),
        };
        manager.shutdown().await;
    }

    #[tokio::test]
    async fn new_wrong_port() {
        let manager = shutdown::Manager::new();
        match WsApi::new_unsecure("localhost", 18123, "auth_token", manager.subscribe()).await {
            Err(Error::WebSocket(_)) => (), // OK
            x => panic!("unexpected result: {:?}", x),
        };
        manager.shutdown().await;
        println!("completed shutdown? how?");
    }
}


// TODO -- pialla tutto sotto questa riga -- TODO //

/// Home Assistant Interface
///
/// Manages the connection to a remote Home Assistance instance via its
/// [WebSocket interface](https://developers.home-assistant.io/docs/api/websocket).
#[derive(Debug)]
pub struct WsApi2 {
    socket: WebSocketStream,
    auth_token: String,
    url: Url,
    last_id: u64,
}

impl WsApi2 {

    pub async fn read_message(&mut self) -> Result<WsMessage> {
        let nxt = self.socket.next().await;
        match nxt {
            Some(nxt) => {
                let json = nxt?;
                tracing::trace!("read_message(..): received: {}", &json);
                if json.is_text() {
                    json::deserialize(&json.into_text().unwrap())
                } else {
                    Err(Error::UnexpectedBinaryMessage)
                }
            },
            None => Err(Error::NoNextMessage)
        }
    }

    async fn write_message(&mut self, msg: &WsMessage) -> Result<()> {
        let msg = json::serialize(&msg)?;
        tracing::trace!("write_message(..): sending: {}", &msg);
        self.socket.send(Message::Text(msg)).await?;
        Ok(())
    }

    async fn authenticate(&mut self) -> Result<()> {
        match self.read_message().await? {
            WsMessage::AuthRequired { ha_version } => {
                tracing::info!("authentication: received auth_required message from HA {}", &ha_version);
            },
            u => {
                return Err(Error::UnexpectedMessage(u));
            }
        }

        let msg = WsMessage::Auth { access_token: String::from(&self.auth_token) };
        self.write_message(&msg).await?;
        tracing::info!("authentication: auth_token sent");

        match self.read_message().await? {
            WsMessage::AuthOk { .. } => {
                tracing::info!("authentication: successful");
                Ok(())
            },
            WsMessage::AuthInvalid {message} => {
                tracing::error!("authentication: failed ({})", message);
                Err(Error::Authentication(message))
            },
            u => {
                tracing::error!("authentication: failed for unexpected message: {:?}", u);
                Err(Error::UnexpectedMessage(u))
            },
        }
    }

    fn get_next_id(&mut self) -> Id {
        self.last_id += 1;
        self.last_id
    }

    pub async fn subscribe_event(&mut self, event_type: Option<json::EventType>) -> Result<Id> {
        let id = self.get_next_id(); // TODO: we should check this id in next messages later
        let sub_msg = WsMessage::SubscribeEvents {
            id: id,
            event_type: event_type
        };
        self.write_message(&sub_msg).await?;

        match self.read_message().await? {
            WsMessage::Result { data: json::ResultBody { success: true, .. } } => {
                if let Some(t) = event_type {
                    tracing::info!("subscribe_events: {:?}: successful", t);
                } else {
                    tracing::info!("subscribe_events: successful");
                }
                Ok(id)
            },
            WsMessage::Result { data: json::ResultBody { success: false, error, .. } } => {
                if let Some(e) = error {
                    tracing::error!("subscribe_events: failed with error code {}: {}", e.code, e.message);
                    Err(Error::ProtocolError(e.code, e.message))
                } else {
                    tracing::error!("subscribe_events: failed (no error information available)");
                    Err(Error::ProtocolError(String::from("none"), String::from("none")))
                }
            },
            u => {
                tracing::error!("subscribe_events: failed for unexpected message: {:?}", u);
                Err(Error::UnexpectedMessage(u))
            },
        }
    }

    pub async fn subscribe_events(&mut self, event_types: &[json::EventType]) -> core::result::Result<Vec<Id>, (usize, Error)> {
        let mut ids : Vec<Id> = Vec::with_capacity(event_types.len());
        for (i, event_type) in event_types.iter().enumerate() {
            match self.subscribe_event(Some(*event_type)).await {
                Ok(id) => ids.push(id),
                Err(e) => {
                    return Err((i, e));
                }
            }
        }
        Ok(ids)
    }

    pub async fn receive_events(&mut self) -> Result<()> {
        loop {
            let msg = self.read_message().await?;
            tracing::info!("receive_events: {:?}", &msg);
        }
    }

}

