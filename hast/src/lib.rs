use std::fs::File;
use std::net::SocketAddr;
use std::{io, sync::Arc};
use hass::serde::Deserialize;
use hass::serde_yaml;
use hass::sync::shutdown::Shutdown;
use hass::WsMessage;
use tokio::{self, net::{TcpListener, TcpStream}, sync::mpsc::{self, UnboundedSender}};
use tokio_tungstenite::tungstenite::{Result, Message};
use futures_util::{StreamExt, SinkExt};
use tracing;


pub struct HastConfig {
    pub port: u16,
    pub token: String,
    pub yaml_event_log: String,
    ha_version: String,
}

impl HastConfig {
    pub fn new(port: u16, token: String, yaml_event_log: String) -> HastConfig {
        HastConfig {
            port,
            token,
            yaml_event_log,
            ha_version: format!("{}-{}", env!("CARGO_PKG_VERSION"), env!("CARGO_PKG_NAME")),
        }
    }
}

/// # Home Assistant Surrogate Tool
/// 
/// Listens on a WebSocket for incoming connections, accepts them, and
/// spawns tokio tasks handling each with basic Home Assistant WebSocket
/// functionality such as authentication and event subscription.
pub struct Hast {
    cfg: Arc<Box<HastConfig>>,
    shutdown: Shutdown,
}

impl Hast {
    pub fn new(cfg: HastConfig, shutdown: Shutdown) -> Hast {
        Hast {
            cfg: Arc::new(Box::new(cfg)),
            shutdown,
        }
    }

    pub async fn run(mut self) -> Result<(), io::Error> {
        let addr = format!("127.0.0.1:{}", self.cfg.port);

        let listener = TcpListener::bind(&addr).await?;
        tracing::info!("hast: listening on {}", addr);

        loop {
            tokio::select! {
                Ok((stream, _)) = listener.accept() => {
                    let args_cl = self.cfg.clone();
                    let shutdown_cl = self.shutdown.clone();
                    tokio::spawn(accept_connection(stream, args_cl, shutdown_cl));
                },

                _ = self.shutdown.recv() => {
                    tracing::info!("hast: received shutdown request");
                    break;
                },

                else => break,
            }
        }

        tracing::info!("hast: shutdown");
        Ok(())
    }
}



async fn accept_connection(stream: TcpStream, cfg: Arc<Box<HastConfig>>, shutdown: Shutdown) -> Result<()> {
    let mut shutdown = shutdown;
    let addr = stream.peer_addr().expect("connected streams should have a peer address");
    tracing::info!("{}: connected", addr);

    let ws_stream = tokio_tungstenite::accept_async(stream)
        .await
        .expect("Error during the websocket handshake occurred");
    tracing::info!("{}: new WebSocket connection", addr);

    let (mut write, mut read) = ws_stream.split();

    let (tx, mut rx) = mpsc::unbounded_channel();

    tx.send(WsMessage::AuthRequired { ha_version: cfg.ha_version.clone() }).unwrap();

    loop {
        tokio::select! {
            msg = rx.recv() => {
                if msg.is_none() {
                    break;
                }
                tracing::info!("{}: SENDING:\n{:?}", addr, &msg);
                let msg = hass::json::serialize(&msg.unwrap()).unwrap();
                write.send(Message::Text(msg)).await.unwrap();
            },

            msg = read.next() => {
                if msg.is_none() {
                    break;
                }
                let msg = msg.unwrap()?;
                if !msg.is_text() {
                    continue;
                }
                if let Ok(wsmsg) = hass::json::deserialize(msg.to_text()?) {
                    tracing::info!("{}: RECEIVED:\n{:?}", addr, wsmsg);
                    let tx_cl = tx.clone();
                    let args_cl = cfg.clone();
                    let shutdown_cl = shutdown.clone();
                    tokio::spawn(async move {
                        handle_message(wsmsg, tx_cl, args_cl, &addr, shutdown_cl).await.unwrap();
                    });
                }
            },

            _ = shutdown.recv() => {
                tracing::info!("{}: received shutdown request", addr);
                break;
            }

            else => break,
        }
    }

    drop(tx);

    tracing::info!("{}: shutdown", addr);
    Ok(())
}

async fn handle_message(wsmsg: WsMessage, tx: UnboundedSender<WsMessage>, cfg: Arc<Box<HastConfig>>, addr: &SocketAddr, _shutdown: Shutdown) -> Result<()> {
    use hass::json::{WsMessage::*, ResultBody, ErrorObject};

    let send = |msg| {
        if let Err(e) = tx.send(msg) {
            tracing::error!("{}: handle_message: could not send event: {}", addr, e);
        }
    };

    match wsmsg {

        Auth { access_token } => {
            let msg = if access_token == cfg.token {
                AuthOk { ha_version: cfg.ha_version.clone() }
            } else {
                AuthInvalid { message: "wrong token".to_string() }
            };
            send(msg);
        },

        SubscribeEvents { id, .. } => {
            send(WsMessage::new_result_success(id));
            let event_log_file = File::open(&cfg.yaml_event_log);
            if let Err(e) = event_log_file {
                tracing::error!("{}: handle message: could not open YAML event log file: {}", addr, e);
            } else {
                let event_log_reader = io::BufReader::new(event_log_file.unwrap());
                for document in serde_yaml::Deserializer::from_reader(event_log_reader) {
                    let _ = WsMessage::deserialize(document)
                        .and_then(|ev| { Ok(send(ev.set_id(id))) })
                        .or_else(|err| {
                            tracing::error!("{}: handle message: could not deserialize YAML document from event log file: {}", addr, err);
                            Err(err)
                        });
                }
            }
        },

        Ping { id } => {
            send(Pong { id });
        },

        m => {
            send(Result {
                id: m.id().unwrap_or(0),
                success: false,
                data: ResultBody::Error {
                    error: ErrorObject {
                        code: "000".to_string(),
                        message: "unexpected message".to_string()
                    }
                },
            });
        }
    };

    tracing::info!("{}: handle message: done", addr);
    Ok(())
}