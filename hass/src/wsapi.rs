
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::{
    self,
    tungstenite,
    connect_async,
    MaybeTlsStream,
};
use tungstenite::{
    Message,
};

use tokio::net::TcpStream;
use url::Url;

use crate::error::Error;
use crate::json::{self, Id, WsMessage};

type WebSocketStream = tokio_tungstenite::WebSocketStream<MaybeTlsStream<TcpStream>>;


/// Home Assistant Interface
///
/// Manages the connection to a remote Home Assistance instance via its
/// [WebSocket interface](https://developers.home-assistant.io/docs/api/websocket).
#[derive(Debug)]
pub struct WsApi {
    socket: WebSocketStream,
    auth_token: String,
    url: Url,
    last_id: u64,
}

impl WsApi {

    /// Connects to a given `host` and `port` with the provided authentication
    /// token `auth_token`.
    pub async fn new(secure: bool, host: &str, port: u16, auth_token: &str) -> Result<WsApi, Error> {
        let scheme = if secure { "wss" } else { "ws" };
        let url = Url::parse(&format!("{}://{}:{}/api/websocket", scheme, host, port))?;

        let mut ws = WsApi {
            socket: connect_ws(&url).await?,
            auth_token: String::from(auth_token),
            url: url,
            last_id: 0,
        };
        ws.authenticate().await?;
        Ok(ws)
    }

    /// Connects to a given `host` and `port` with the provided authentication
    /// token `auth_token`.
    pub async fn new_unsecure(host: &str, port: u16, auth_token: &str) -> Result<WsApi, Error> {
        Self::new(false, host, port, auth_token).await
    }

    /// Connects to a given `host` and `port` with the provided authentication
    /// token `auth_token`.
    pub async fn new_secure(host: &str, port: u16, auth_token: &str) -> Result<WsApi, Error> {
        Self::new(true, host, port, auth_token).await
    }

    pub async fn connect(&mut self) -> Result<(), Error> {
        self.socket = connect_ws(&self.url).await?;
        Ok(())
    }

    pub async fn close(mut self) {
        self.socket.close(Option::None).await.unwrap();
    }

    pub fn url(&self) -> &Url {
        &self.url
    }

    pub async fn read_message(&mut self) -> Result<WsMessage, Error> {
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

    async fn write_message(&mut self, msg: &WsMessage) -> Result<(), Error> {
        let msg = json::serialize(&msg)?;
        tracing::trace!("write_message(..): sending: {}", &msg);
        self.socket.send(Message::Text(msg)).await?;
        Ok(())
    }

    async fn authenticate(&mut self) -> Result<(), Error> {
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

    pub async fn subscribe_event(&mut self, event_type: Option<json::EventType>) -> Result<Id, Error> {
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

    pub async fn subscribe_events(&mut self, event_types: &[json::EventType]) -> Result<Vec<Id>, (usize, Error)> {
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

    pub async fn receive_events(&mut self) -> Result<(), Error> {
        loop {
            let msg = self.read_message().await?;
            tracing::info!("receive_events: {:?}", &msg);
        }
    }

}



async fn connect_ws(url: &Url) -> Result<WebSocketStream, Error> {
    let (socket, response) = connect_async(url).await?;
    tracing::trace!("connect({}): {:?}", url, response);
    Ok(socket)
}


#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[should_panic]
    async fn new_unknown_host() {
        WsApi::new_unsecure("i.do.not.exist", 8123, "auth_token").await.unwrap();
    }

    #[tokio::test]
    #[should_panic]
    async fn new_wrong_port() {
        WsApi::new_unsecure("localhost", 18123, "auth_token").await.unwrap();
    }
}
