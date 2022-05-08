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
use crate::sync::{atomic::AtomicId, shutdown::Shutdown};

use super::{
    WebSocketStream,
    KEEPALIVE_INTERVAL_SEC,
};

/// Represents commands understood by the `WsApiMessenger`.
#[derive(Debug)]
pub enum Command {
    Message(WsMessage),
    Register(Id, mpsc::Sender<WsMessage>),
}


pub struct WsApiMessenger {
    rx: mpsc::Receiver<Command>,
    socket: WebSocketStream,
    id: Arc<AtomicId>,
    receivers: BTreeMap<Id, mpsc::Sender<WsMessage>>,
    unhandled: Option<mpsc::Sender<WsMessage>>,
    
    /// Receives shutdown signal and notifies back about completed shutdown
    /// once dropped.
    shutdown: Shutdown,
}

impl WsApiMessenger {
    pub fn new(rx: mpsc::Receiver<Command>, socket: WebSocketStream, id: Arc<AtomicId>, unhandled: Option<mpsc::Sender<WsMessage>>, shutdown: Shutdown) -> WsApiMessenger {
        WsApiMessenger {
            rx,
            socket,
            id,
            unhandled,
            shutdown,
            receivers: BTreeMap::new(),
        }
    }

    pub async fn run(mut self) -> Result<()> {
        let mut keepalive = time::interval(Duration::from_secs(KEEPALIVE_INTERVAL_SEC));

        loop {
            tokio::select! {
                // Event on the command channel
                cmd = self.rx.recv() => match cmd {
                    Some(cmd) => match cmd {
                        Command::Message(msg) => {
                            self.send(msg).await?;
                        },
                        Command::Register(id, reg_sender) => {
                            self.register(id, reg_sender);
                        }
                    },
                    None => {
                        // Termination due to end of commands
                        break;
                    }
                },

                // Event on the HA socket
                rcv = self.socket.next() => match rcv {
                    Some(Ok(rcv)) => {
                        if rcv.is_text() {
                            let msg = &rcv.into_text().unwrap();
                            let msg = json::deserialize(msg).unwrap();
                            self.dispatch(msg).await;
                        } else {
                            // We usually only expect text messages from HA
                            tracing::error!("unexpected messaage: {:?}", rcv);
                        }
                    },
                    Some(Err(e)) => {
                        tracing::error!("websocket error: {:?}", e);
                        break;
                    },
                    None => {
                        tracing::warn!("websocket closed by peer");
                        break;
                    }
                },

                // Keepalive ping event
                // HA will close the connection should it stop receiving messages
                _ = keepalive.tick() => {
                    self.send_ping().await?;
                },

                _ = self.shutdown.recv() => {
                    tracing::info!("shutdown request");
                    break;
                }

                else => {
                    tracing::warn!("select! reached else branch");
                    break;
                }
            };
        }

        self.rx.close();
        let _ = self.socket.close(None).await;

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
        let id = msg.id();

        // This commented variant dispatches to self.unhandled, if defined,
        // even messages with and id. I'd rather not to however, because
        // there must be a reason why nobody registered to wait for them
        ////let receiver = id
        ////    .and_then(|id| { self.receivers.get(&id) })
        ////    .or_else(|| self.unhandled.as_ref());

        let receiver = id.map_or_else(
            || self.unhandled.as_ref(),
            |id| { self.receivers.get(&id) });

        if let Some(receiver) = receiver {
            if let Err(e) = receiver.send(msg).await {
                if let Some(id) = id {
                    self.receivers.remove(&id);
                } else {
                    self.unhandled.take();
                }
                return Err(Error::InternalError {
                    cause: anyhow!("could not dispatch message: send failed: {}", e.0)
                });
            }
        } else {
            return Err(Error::InternalError {
                cause: anyhow!("could not dispatch message: no receiver: {}", &msg)
            });
        }

        Ok(())
    }

}
