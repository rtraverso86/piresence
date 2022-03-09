use thiserror::Error;

//#[derive(Error, Debug)]
#[derive(Error, Debug)]
pub enum Error {
    #[error("authentication failed")]
    Authentication,

    #[error("websocket error")]
    WebSocket(#[source] tungstenite::Error),
}

//impl PartialEq for Error {
//    fn eq(&self, other: &Self) -> bool {
//        use Error::*;
//        match (self, other) {
//            (&Url, &Url) => true,
//            (&Authentication, &Authentication) => true,
//            (&WebSocket(_), &WebSocket(_)) => true,
//            _ => false,
//        }
//    }
//}