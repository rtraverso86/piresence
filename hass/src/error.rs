use serde_json;
use thiserror::Error;
use tungstenite;
use url;
use crate::json::WsMessage;

#[derive(Error, Debug)]
pub enum Error {
    #[error("authentication failed")]
    Authentication(String),

    #[error("websocket error")]
    WebSocket(#[from] tungstenite::Error),

    #[error("URL parsing error")]
    UrlParsing(#[from] url::ParseError),

    #[error("Serde/JSON parsing error")]
    SerdeJsonParsing(#[from] serde_json::Error),

    #[error("generic JSON parsing error")]
    JsonParsing(&'static str),

    #[error("unexpected message from HA server")]
    UnexpectedMessage(WsMessage),

    #[error("HA sent error code and message in response to a request")]
    ProtocolError(String, String)
}
