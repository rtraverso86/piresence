use serde_json;
use thiserror::Error;
use tungstenite;
use url;

#[derive(Error, Debug)]
pub enum Error {
    #[error("authentication failed")]
    Authentication,

    #[error("websocket error")]
    WebSocket(#[from] tungstenite::Error),

    #[error("URL parsing error")]
    UrlParsing(#[from] url::ParseError),

    #[error("Serde JSON parsing error")]
    SerdeJsonParsing(#[from] serde_json::Error),

    #[error("Generic JSON parsing error")]
    JsonParsing(&'static str),
}
