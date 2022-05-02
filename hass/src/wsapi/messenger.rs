use std::collections::BTreeMap;
use std::sync::{
    Arc,
};
use std::time::Duration;

use anyhow::anyhow;
use futures_util::{SinkExt, StreamExt};
use tokio::{
    sync::mpsc,
    time,
};
use tokio_tungstenite::{
    self,
    tungstenite,
};
use tungstenite::{
    Message,
};


use crate::error::{Error, Result};
use crate::json::{self, Id, WsMessage};
use crate::sync::{atomic::AtomicId};

use super::{
    WebSocketStream,
    KEEPALIVE_INTERVAL_SEC,
};

/// Represents commands understood by the `WsApiMessenger`.
#[derive(Debug)]
pub enum Command {
    Message(WsMessage),
    Register(Id, mpsc::Sender<WsMessage>),
    Quit,
}


pub struct WsApiMessenger {
    rx: mpsc::Receiver<Command>,
    socket: WebSocketStream,
    id: Arc<AtomicId>,
    receivers: BTreeMap<Id, mpsc::Sender<WsMessage>>,
}

impl WsApiMessenger {
    pub fn new(rx: mpsc::Receiver<Command>, socket: WebSocketStream, id: Arc<AtomicId>) -> WsApiMessenger {
        WsApiMessenger {
            rx,
            socket,
            id,
            receivers: BTreeMap::new(),
        }
    }

    pub async fn run(mut self) -> Result<()> {
        let mut keepalive = time::interval(Duration::from_secs(KEEPALIVE_INTERVAL_SEC));
    
        let mut done = false;
    
        while !done {
            tokio::select! {
                // Keepalive ping event - HA will close the connection should it stop receiving messages
                _ = keepalive.tick() => {
                    self.send_ping().await?;
                },
    
                // Event on the command channel
                cmd = self.rx.recv() => match cmd {
                    Some(cmd) => match cmd {
                        Command::Message(msg) => {
                            self.send(msg).await?;
                        },
                        Command::Register(id, reg_sender) => {
                            self.register(id, reg_sender);
                        },
                        Command::Quit => {
                            // Explicit termination request
                            // TODO: handle termination request (*1)
                            done = true;
                        },
                    },
                    None => {
                        // Termination due to end of commands
                        //TODO: handle channel closed (*1)
                        done = true;
                    }
                },
    
                // Event on the HA socket
                rcv = self.socket.next() => match rcv {
                    Some(rcv) => match rcv {
                        Ok(rcv) => {
                            if rcv.is_text() {
                                let msg = &rcv.into_text().unwrap();
                                let msg = json::deserialize(msg).unwrap();
                                self.dispatch(msg).await;
                            } else {
                                // TODO: handle message type error
                            }
                        },
                        Err(err) => {
                            // TODO: handle tungstenite receive error
                        },
                    },
                    None => {
                        //TODO: handle socket closed (*1)
                        done = true;
                    }
                }
            }
        }
        // TODO: send termination confirmation (*1)
        Ok(())
    }

    /// Send the given `msg` to HA
    async fn send(&mut self, msg: WsMessage) -> Result<()> {
        let msg = json::serialize(&msg)?;
        tracing::trace!("send({})", &msg);
        self.socket.send(Message::Text(msg)).await?;
        Ok(())
    }

    /// Send a ping message to HA
    async fn send_ping(&mut self) -> Result<()> {
        let msg = WsMessage::Ping { id: self.id.next() };
        self.send(msg).await?;
        Ok(())
    }

    fn register(&mut self, id: Id, reg_sender: mpsc::Sender<WsMessage>) {
        // drop the old sender, if present
        let _ = self.receivers.insert(id, reg_sender);
    }

    async fn dispatch(&mut self, msg: WsMessage) -> Result<()> {
        let id = msg.id()
            .ok_or(Error::InternalError {
                cause: anyhow!("could not dispatch message: no id:  {}", &msg)
            })?;

        let receiver = self.receivers.get(&id)
            .ok_or(Error::InternalError {
                cause: anyhow!("could not dispatch message: no receiver: {}", &msg)
            })?;

        if let Err(e) = receiver.send(msg).await {
            self.receivers.remove(&id);
            return Err(Error::InternalError {
                cause: anyhow!("could not dispatch message: send failed: {}", e.0)
            });
        }
        Ok(())
    }

}
