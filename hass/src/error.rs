use anyhow;
use serde_json;
use thiserror::Error;
use tokio_tungstenite::tungstenite;
use url;
use crate::json::{WsMessage, ErrorObject};

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

    #[error("Could not send message: {0}")]
    SendError(WsMessage),

    #[error("Could not subscribe")]
    SubscribeError,

    #[error("Internal error: {cause:?}")]
    InternalError {
        cause: anyhow::Error,
    },
}

impl From<Option<ErrorObject>> for Error {
    fn from(err: Option<ErrorObject>) -> Self {
        if let Some(e) = err {
            Error::from(e)
        } else {
            Error::ProtocolError(String::from("None"), String::from("None"))
        }
    }
}

impl From<ErrorObject> for Error {
    fn from(err: ErrorObject) -> Self {
        Error::ProtocolError(err.code, err.message)
    }
}
