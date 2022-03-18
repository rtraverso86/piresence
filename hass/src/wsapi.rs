use tungstenite::{
    connect,
    Message,
    stream::{MaybeTlsStream},
};

use std::net::TcpStream;
use url::Url;

use crate::error::Error;
use crate::json::{serialize, deserialize, WsMessage};

type WebSocket = tungstenite::protocol::WebSocket<MaybeTlsStream<TcpStream>>;



/// Home Assistant Interface
///
/// Manages the connection to a remote Home Assistance instance via its
/// [WebSocket interface](https://developers.home-assistant.io/docs/api/websocket).
#[derive(Debug)]
pub struct WsApi {
    socket: WebSocket,
    auth_token: String,
    url: Url,
}

impl WsApi {

    /// Connects to a given `host` and `port` with the provided authentication
    /// token `auth_token`.
    pub fn new(secure: bool, host: &str, port: u16, auth_token: &str) -> Result<WsApi, Error> {
        let scheme = if secure { "wss" } else { "ws" };
        let url = Url::parse(&format!("{}://{}:{}/api/websocket", scheme, host, port))?;

        let mut ws = WsApi {
            socket: connect_ws(&url)?,
            auth_token: String::from(auth_token),
            url: url,
        };
        ws.authenticate()?;
        Ok(ws)
    }

    /// Connects to a given `host` and `port` with the provided authentication
    /// token `auth_token`.
    pub fn new_unsecure(host: &str, port: u16, auth_token: &str) -> Result<WsApi, Error> {
        Self::new(false, host, port, auth_token)
    }

    /// Connects to a given `host` and `port` with the provided authentication
    /// token `auth_token`.
    pub fn new_secure(host: &str, port: u16, auth_token: &str) -> Result<WsApi, Error> {
        Self::new(true, host, port, auth_token)
    }

    pub fn connect(&mut self) -> Result<(), Error> {
        self.socket = connect_ws(&self.url)?;
        Ok(())
    }

    pub fn close(mut self) {
        self.socket.close(Option::None).unwrap();
    }

    pub fn url(&self) -> &Url {
        &self.url
    }

    fn read_message(&mut self) -> Result<WsMessage, Error> {
        let json = self.socket.read_message().unwrap();
        tracing::trace!("connect(..): received: {}", &json);
        deserialize(&json.into_text().unwrap())
    }

    fn write_message(&mut self, msg: &WsMessage) -> Result<(), Error> {
        let msg = serialize(&msg)?;
        tracing::trace!("connect(..): sending: {}", &msg);
        self.socket.write_message(Message::Text(msg))?;
        Ok(())
    }

    fn authenticate(&mut self) -> Result<(), Error> {
        match self.read_message()? {
            WsMessage::AuthRequired { ha_version } => {
                tracing::info!("authentication: received auth_required message from HA {}", &ha_version);
            },
            u => {
                return Err(Error::UnexpectedMessage(u));
            }
        }

        let msg = WsMessage::Auth { access_token: String::from(&self.auth_token) };
        self.write_message(&msg)?;
        tracing::info!("authentication: auth_token sent");

        match self.read_message()? {
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

}


fn connect_ws(url: &Url) -> Result<WebSocket, Error> {
    let (socket, response) = connect(url)?;
    tracing::trace!("connect(..): {:?}", response);
    Ok(socket)
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic]
    fn new_unknown_host() {
        WsApi::new_unsecure("i.do.not.exist", 8123, "auth_token").unwrap();
    }

    #[test]
    #[should_panic]
    fn new_wrong_port() {
        WsApi::new_unsecure("localhost", 18123, "auth_token").unwrap();
    }
}
