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
pub struct HAInterface {
    socket: WebSocket,
    auth_token: String,
    url: String
}

impl HAInterface {

    /// Connects to a given `host` and `port` with the provided authentication
    /// token `auth_token`.
    pub fn new(secure: bool, host: &str, port: u16, auth_token: &str) -> Result<HAInterface, Error> {
        let scheme = if secure { "wss" } else { "ws" };
        let url = Url::parse(&format!("{}://{}:{}/api/websocket", scheme, host, port)).unwrap();
        let (socket, response) = match connect(&url) {
            Ok(t) => t,
            Err(error) => {
                return Err(Error::WebSocket(error));
            },
        };

        Ok(HAInterface {
            socket: socket,
            auth_token: String::from(auth_token),
            url: String::from(url),
        })
    }

    /// Connects to a given `host` and `port` with the provided authentication
    /// token `auth_token`.
    pub fn new_unsecure(host: &str, port: u16, auth_token: &str) -> Result<HAInterface, Error> {
        Self::new(false, host, port, auth_token)
    }

    /// Connects to a given `host` and `port` with the provided authentication
    /// token `auth_token`.
    pub fn new_secure(host: &str, port: u16, auth_token: &str) -> Result<HAInterface, Error> {
        Self::new(true, host, port, auth_token)
    }
}

#[cfg(test)]
mod tests {
    //use super::*;

    //#[test]
    //fn unexisting_url() {
    //    let err = HAInterface::new_unsecure("i.do.not.exist", 8123, "auth_token")
    //        .unwrap_err();
    //    assert_eq!(err, Error::WebSocket);
    //}
}
