use serde_json;
use thiserror::Error;
use tokio_tungstenite::tungstenite;
use url;
use crate::json::WsMessage;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("authentication failed: {0}")]
    Authentication(String),

    #[error("websocket error: {0}")]
    WebSocket(#[from] tungstenite::Error),

    #[error("URL parsing error: {0}")]
    UrlParsing(#[from] url::ParseError),

    #[error("Serde/JSON parsing error: {0}")]
    SerdeJsonParsing(#[from] serde_json::Error),

    #[error("generic JSON parsing error: {0}")]
    JsonParsing(&'static str),

    #[error("unexpected message from HA server: {0:?}")]
    UnexpectedMessage(WsMessage),

    #[error("unexpected binary message from HA server")]
    UnexpectedBinaryMessage,

    #[error("HA sent error code {0}: {1}")]
    ProtocolError(String, String),

    #[error("Next message not found")]
    NoNextMessage,
}
