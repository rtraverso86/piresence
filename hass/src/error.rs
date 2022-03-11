use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("authentication failed")]
    Authentication,

    #[error("websocket error")]
    WebSocket(#[source] tungstenite::Error),

    #[error("parsing error")]
    Parsing(String),
}
