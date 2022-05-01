use std::sync::{
    Arc,
};
use std::time::Duration;

use anyhow::anyhow;
use futures_util::{SinkExt, StreamExt};
use tokio::{
    net::TcpStream,
    sync::mpsc,
    time,
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

use crate::error::{self, Error, Result};
use crate::json::{self, Id, WsMessage};
use crate::sync::{atomic::AtomicId};


type WebSocketStream = tokio_tungstenite::WebSocketStream<MaybeTlsStream<TcpStream>>;

const MPSC_CHANNEL_BOUND: usize = 128;
const KEEPALIVE_INTERVAL_SEC: u64 = 15;


/// Represents commands understood by the `WsApiMessenger`.
#[derive(Debug)]
enum Command {
    Message(WsMessage),
    Register(Id, mpsc::Sender<WsMessage>),
    Quit,
}

#[derive(Debug)]
pub struct WsApi {
    /// Endpoint URL
    url: Url,
    // Endpoint authentication token
    auth_token: String,

    /// Transmission channel to send commands to the `WsApiMessenger`
    tx: mpsc::Sender<Command>,

    /// Next available identifier, to be used for `WsMessage` requests
    id: Arc<AtomicId>,
}

impl WsApi {

    /// Connects to a given `host` and `port` HA WebSocket endpoint with the provided
    /// and performs authentication with the `auth_token`.
    pub async fn new(secure: bool, host: &str, port: u16, auth_token: &str) -> Result<WsApi>
    {
        let scheme = if secure { "wss" } else { "ws" };
        let url = Url::parse(&format!("{}://{}:{}/api/websocket", scheme, host, port))?;

        //? What to do with you? I need to guarantee all new messages sent requiring IDs are
        //? properly taking new ids from here.
        let id = Arc::new(AtomicId::new());

        let id2 = id.clone();
        let socket = connect_ws(&url).await?;
        let (tx, rx) = mpsc::channel(MPSC_CHANNEL_BOUND);
        tokio::spawn(async move {
            let messenger = WsApiMessenger::new(rx, socket, id2);
            messenger_task(messenger).await;
        });

        let api = WsApi {
            auth_token: auth_token.to_owned(),
            url,
            tx,
            id
        };

        Ok(api)
    }

    /// Connects to a given `host` and `port` with the provided authentication
    /// token `auth_token`.
    pub async fn new_unsecure(host: &str, port: u16, auth_token: &str) -> Result<WsApi> {
        Self::new(false, host, port, auth_token).await
    }

    /// Connects to a given `host` and `port` with the provided authentication
    /// token `auth_token`.
    pub async fn new_secure(host: &str, port: u16, auth_token: &str) -> Result<WsApi> {
        Self::new(true, host, port, auth_token).await
    }

    /// Returns the full URL of the WebSocket endpoint `self` is connected to.
    pub fn url(&self) -> &Url {
        &self.url
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

    async fn registration(&self) -> Result<(Id, mpsc::Receiver<WsMessage>)> {
        let id = self.id.next();
        let (tx, rx) = mpsc::channel(MPSC_CHANNEL_BOUND);
        let cmd = Command::Register(id, tx);
        self.send_command(cmd).await?;
        Ok((id, rx))
    }

    pub async fn subscribe_events(&self, event_type: Option<json::EventType>) -> Result<mpsc::Receiver<WsMessage>> {
        let (id, mut rx) = self.registration().await?;
        self.send_command(Command::Message(WsMessage::SubscribeEvents { id, event_type })).await?;

        let reply = rx.recv()
            .await
            .ok_or(Error::InternalError { cause: anyhow!("missing response")})?;

        reply.receiver_or_error(rx)
    }

    // Consumes `self` closing all connections and resources.
    pub async fn close(self) -> Result<()> {
        self.tx.send(Command::Quit).await;
        // TODO: wait for confirmation (*1)
        Ok(())
    }
}

trait FromReply {
    fn receiver_or_error(&self, rx: mpsc::Receiver<Self>) -> Result<mpsc::Receiver<Self>>;
}

impl FromReply for WsMessage {
    fn receiver_or_error(&self, rx: mpsc::Receiver<Self>) -> Result<mpsc::Receiver<Self>> {
        match self {
            WsMessage::Result { data: json::ResultBody { success: true, .. } } => {
                Ok(rx)
            },
            WsMessage::Result { data: json::ResultBody { success: false, error, .. } } => {
                Err(Error::from(*error))
            },
            u => {
                Err(Error::UnexpectedMessage(*u))
            },
        }
    }
}


struct WsApiMessenger {
    rx: mpsc::Receiver<Command>,
    socket: WebSocketStream,
    id: Arc<AtomicId>,
}

impl WsApiMessenger {
    fn new(rx: mpsc::Receiver<Command>, socket: WebSocketStream, id: Arc<AtomicId>) -> WsApiMessenger {
        WsApiMessenger {
            rx,
            socket,
            id,
        }
    }

    async fn send(&mut self, msg: WsMessage) -> Result<()> {
        let msg = json::serialize(&msg)?;
        tracing::trace!("send({})", &msg);
        self.socket.send(Message::Text(msg)).await?;
        Ok(())
    }

    async fn send_ping(&mut self) -> Result<()> {
        let msg = WsMessage::Ping { id: self.id.next() };
        self.send(msg).await?;
        Ok(())
    }

    async fn read_message(&mut self) -> Result<WsMessage> {
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

    fn dispatch(&mut self, msg: WsMessage) -> Result<()> {
        //TODO: develop message dispatching (*2)
        Ok(())
    }
}

async fn messenger_task(mut messenger: WsApiMessenger) -> Result<()> {
    let mut keepalive = time::interval(Duration::from_secs(KEEPALIVE_INTERVAL_SEC));

    let mut done = false;

    while !done {
        tokio::select! {
            // Keepalive ping event - HA will close the connection should it stop receiving messages
            _ = keepalive.tick() => {
                messenger.send_ping().await?;
            },

            // Event on the command channel
            cmd = messenger.rx.recv() => match cmd {
                Some(cmd) => match cmd {
                    Command::Message(msg) => {
                        // Send a new message to HA
                        messenger.send(msg).await?;
                    },
                    Command::Register(id, registered) {
                        // TODO: handle response registration
                        unimplemented!()
                    },
                    Command::Quit => {
                        // Explicit termination request
                        // TODO: handle termination request (*1)
                        done = true;
                    },
                },
                None => {
                    // Termination due to end of commands
                    //TODO: handle channel closed (*1)
                    done = true;
                }
            },

            // Event on the HA socket
            rcv = messenger.socket.next() => match rcv {
                Some(rcv) => match rcv {
                    Ok(rcv) => {
                        if rcv.is_text() {
                            let msg = &rcv.into_text().unwrap();
                            let msg = json::deserialize(msg).unwrap();
                            messenger.dispatch(msg);
                        } else {
                            // TODO: handle message type error
                        }
                    },
                    Err(err) => {
                        // TODO: handle tungstenite receive error
                    },
                },
                None => {
                    //TODO: handle socket closed (*1)
                    done = true;
                }
            }
        }
    }
    // TODO: send termination confirmation (*1)
    Ok(())
}

async fn connect_ws(url: &Url) -> Result<WebSocketStream> {
    let (socket, response) = connect_async(url).await?;
    tracing::trace!("connect({}): {:?}", url, response);
    Ok(socket)
}




#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn new_unknown_host() {
        match WsApi::new_unsecure("i.do.not.exist", 8123, "auth_token").await {
            Err(Error::WebSocket(_)) => (), // OK
            x => panic!("unexpected result: {:?}", x),
        }
    }

    #[tokio::test]
    async fn new_wrong_port() {
        match WsApi::new_unsecure("localhost", 18123, "auth_token").await {
            Err(Error::WebSocket(_)) => (), // OK
            x => panic!("unexpected result: {:?}", x),
        }
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

