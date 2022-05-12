use std::fs::File;
use std::{io, sync::Arc};
use hass::serde::Deserialize;
use hass::serde_yaml;
use hass::sync::shutdown::Shutdown;
use hass::{WsMessage, json::Id};
use clap::{self, StructOpt};
use tracing_subscriber;
use tokio::{self, net::{TcpListener, TcpStream}, sync::mpsc::{self, UnboundedSender}};
use tokio_tungstenite::tungstenite::{Result, Message};
use futures_util::{StreamExt, SinkExt};


pub const HA_VERSION : &str = "0.1-STUB";

pub struct HastConfig {
    pub port: u16,
    pub token: String,
    pub yaml_event_log: String,
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

    pub async fn start(self) -> Result<(), io::Error> {
        let addr = format!("127.0.0.1:{}", self.cfg.port);

        let listener = TcpListener::bind(&addr).await?;
        tracing::info!("listening on: {}", addr);
    
        while let Ok((stream, _)) = listener.accept().await {
            let args_clone = self.cfg.clone();
            tokio::spawn(accept_connection(stream, args_clone));
        }

        Ok(())
    }
}



async fn accept_connection<'a>(stream: TcpStream, cfg: Arc<Box<HastConfig>>) -> Result<()> {
    let addr = stream.peer_addr().expect("connected streams should have a peer address");
    tracing::info!("{}: connected", addr);

    let ws_stream = tokio_tungstenite::accept_async(stream)
        .await
        .expect("Error during the websocket handshake occurred");

    tracing::info!("{}: new WebSocket connection", addr);

    let (mut write, mut read) = ws_stream.split();

    let (tx, mut rx) = mpsc::unbounded_channel();
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            tracing::info!("{}: send({:?})", addr, &msg);
            let msg = hass::json::serialize(&msg).unwrap();
            write.send(Message::Text(msg)).await.unwrap();
        }
        tracing::info!("{}: messenger quit", addr);
    });

    tx.send(WsMessage::AuthRequired { ha_version: HA_VERSION.to_string() }).unwrap();

    while let Some(msg) = read.next().await {
        let msg = msg?;
        if !msg.is_text() {
            continue;
        }
        if let Ok(wsmsg) = hass::json::deserialize(msg.to_text()?) {
            tracing::info!("{}: received({:?})", addr, wsmsg);
            let tx_clone = tx.clone();
            let args_clone = cfg.clone();
            tokio::spawn(async move {
                handle_message(wsmsg, tx_clone, args_clone).await.unwrap();
            });
        }
    }

    drop(tx);

    Ok(())
}

async fn handle_message(wsmsg: WsMessage, tx: UnboundedSender<WsMessage>, cfg: Arc<Box<HastConfig>>) -> Result<()> {
    use hass::json::{WsMessage::*, ResultBody, ErrorObject};
    match wsmsg {

        Auth { access_token } => {
            let msg = if access_token == cfg.token {
                AuthOk { ha_version: HA_VERSION.to_string() }
            } else {
                AuthInvalid { message: "wrong token".to_string() }
            };
            tx.send(msg).unwrap();
        },

        SubscribeEvents { id, .. } => {
            tx.send(WsMessage::new_result_success(id)).unwrap();
            let event_log_file = File::open(&cfg.yaml_event_log).unwrap();
            let event_log_reader = io::BufReader::new(event_log_file);
            for document in serde_yaml::Deserializer::from_reader(event_log_reader) {
                let ev = WsMessage::deserialize(document).unwrap();
                if let Err(e) = tx.send(ev.set_id(id)) {
                    tracing::error!("could not send event: {}", e);
                }
            }
        },

        Ping { id } => {
            tx.send(Pong { id }).unwrap();
        },

        wsmsg => {
            tx.send(Result {
                id: wsmsg.id().unwrap_or(0),
                success: false,
                data: ResultBody::Error {
                    error: ErrorObject {
                        code: "000".to_string(),
                        message: "unexpected message".to_string()
                    }
                },
            }).unwrap();
        }
    };
    Ok(())
}