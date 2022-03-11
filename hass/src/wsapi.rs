use tungstenite::{
    connect,
    Message,
    stream::{MaybeTlsStream},
};

use std::net::TcpStream;
use url::Url;

use crate::error::Error;

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
        let url = match Url::parse(&format!("{}://{}:{}/api/websocket", scheme, host, port)) {
            Ok(u) => u,
            Err(e) => {
                return Err(Error::Parsing(format!("couldn't parse URL: {}", e)));
            },
        };

        Ok(WsApi {
            socket: connect_ws(&url)?,
            auth_token: String::from(auth_token),
            url: url,
        })
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

    pub fn url(&self) -> &Url {
        &self.url
    }
}


fn connect_ws(url: &Url) -> Result<WebSocket, Error> {
    let (socket, response) = match connect(url) {
        Ok(t) => t,
        Err(error) => {
            return Err(Error::WebSocket(error));
        },
    };
    // let (socket, response) = connect(url)?; // !!! THIS ONE DOES NOT COMPILE 
    tracing::trace!("connect(..): {:?}", response);

    //let msg = socket.read_message()

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
