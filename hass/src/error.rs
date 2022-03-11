use thiserror::Error;
use tungstenite;
use url;

#[derive(Error, Debug)]
pub enum Error {
    #[error("authentication failed")]
    Authentication,

    #[error("websocket error")]
    WebSocket(#[from] tungstenite::Error),

    #[error("parsing error")]
    Parsing(#[from] url::ParseError),
}
