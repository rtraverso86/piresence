use std::fs::File;
use std::net::SocketAddr;
use std::sync::Mutex;
use std::{io, sync::Arc};
use serde::{Serialize, Deserialize};
use serde_yaml;
use crate::sync::shutdown::Shutdown;
use crate::WsMessage;
use tokio::{self, net::{TcpListener, TcpStream}, sync::mpsc::{self, UnboundedSender}};
use tokio_tungstenite::tungstenite::{Result, Message};
use futures_util::{StreamExt, SinkExt};
use tracing;

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HastMessage {
    Name(String),
    Token(String),
    Scenario(String),
    Start,
}

#[derive(Debug)]
pub struct HastConfig {
    pub port: u16,
    pub token: String,
    pub yaml_dir: String,
    pub yaml_scenario: Option<String>,
    pub skip_hast_messages: bool,
    ha_version: String,
}

impl HastConfig {
    pub fn new(port: u16, token: String, yaml_dir: String) -> HastConfig {
        HastConfig::new_with_scenario(port, token, yaml_dir, None)
    }

    pub fn new_with_scenario(port: u16, token: String, yaml_dir: String, yaml_scenario: Option<String>) -> HastConfig {
        HastConfig {
            port,
            token,
            yaml_dir,
            yaml_scenario,
            ha_version: format!("{}-{}", env!("CARGO_PKG_VERSION"), env!("CARGO_PKG_NAME")),
            skip_hast_messages: false,
        }
    }
}

#[derive(Debug)]
struct HastConnConfig {
    pub token: String,
    pub yaml_scenario: Option<String>,
    pub name: Option<String>,
    common_cfg: Arc<Box<HastConfig>>,
}

impl HastConnConfig {
    fn new(hc: Arc<Box<HastConfig>>) -> HastConnConfig {
        HastConnConfig {
            token: hc.token.clone(),
            common_cfg: hc.clone(),
            yaml_scenario: hc.yaml_scenario.clone(),
            name: None,
        }
    }

    fn yaml_dir(&self) -> &str {
        &self.common_cfg.yaml_dir
    }

    fn ha_version(&self) -> &str {
        &self.common_cfg.ha_version
    }

    fn test_name(&self) -> String {
        if let Some(scenario) = self.yaml_scenario.as_ref() {
            return if let Some(name) = self.name.as_ref() {
                format!("{}[{}]", scenario, name)
            } else {
                format!("{}[]", scenario)
            }
        }
        "".to_string()
    }

    fn skip_hast_messages(&self) -> bool {
        self.common_cfg.skip_hast_messages
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
                    let conn_cfg = HastConnConfig::new(self.cfg.clone());
                    let shutdown_cl = self.shutdown.clone();
                    tokio::spawn(accept_connection(stream, conn_cfg, shutdown_cl));
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



async fn accept_connection(stream: TcpStream, cfg: HastConnConfig, shutdown: Shutdown) -> Result<()> {
    let mut shutdown = shutdown;
    let mut cfg = cfg;
    let addr = stream.peer_addr().expect("connected streams should have a peer address");
    tracing::info!("{}: connected, configuration: {:?}", addr, cfg);

    let ws_stream = tokio_tungstenite::accept_async(stream)
        .await
        .expect("Error during the websocket handshake occurred");
    tracing::info!("{}: new WebSocket connection", addr);

    let (mut sk_write, mut sk_read) = ws_stream.split();

    // Hast configuration loop
    while ! cfg.skip_hast_messages() {
        tokio::select! {
            Some(msg) = sk_read.next() => {
                match hass::serde_json::from_str(msg?.to_text()?).unwrap() {
                    HastMessage::Name(n) => {
                        cfg.name = Some(n);
                    }
                    HastMessage::Token(t) => {
                        cfg.token = t;
                    },
                    HastMessage::Scenario(p) => {
                        cfg.yaml_scenario = Some(p);
                    },
                    HastMessage::Start => break,
                }
            },
            else => break,
        }
    }
    let (tx, mut rx) = mpsc::unbounded_channel();
    tx.send(WsMessage::AuthRequired { ha_version: cfg.ha_version().to_string() }).unwrap();
    
    let test_name = cfg.test_name();
    let cfg = Arc::new(cfg);
    loop {
        tokio::select! {
            msg = rx.recv() => {
                if msg.is_none() {
                    break;
                }
                tracing::info!("{}: {}: SENDING:\n{:?}", addr, test_name, &msg);
                let msg = hass::json::serialize(&msg.unwrap()).unwrap();
                sk_write.send(Message::Text(msg)).await.unwrap();
            },

            msg = sk_read.next() => {
                if msg.is_none() {
                    break;
                }
                let msg = msg.unwrap()?;
                if !msg.is_text() {
                    continue;
                }
                if let Ok(wsmsg) = hass::json::deserialize(msg.to_text()?) {
                    tracing::info!("{}: {}: RECEIVED:\n{:?}", addr, test_name, wsmsg);
                    let tx_cl = tx.clone();
                    let cfg_cl = cfg.clone();
                    let shutdown_cl = shutdown.clone();
                    tokio::spawn(async move {
                        handle_message(wsmsg, tx_cl, cfg_cl, &addr, shutdown_cl).await.unwrap();
                    });
                }
            },

            _ = shutdown.recv() => {
                tracing::info!("{}: {}: received shutdown request", addr, test_name);
                break;
            }

            else => break,
        }
    }

    drop(tx);

    tracing::info!("{}: {}: shutdown", addr, test_name);
    Ok(())
}

async fn handle_message(wsmsg: WsMessage, tx: UnboundedSender<WsMessage>, cfg: Arc<HastConnConfig>, addr: &SocketAddr, _shutdown: Shutdown) -> Result<()> {
    use hass::json::{WsMessage::*, ResultBody, ErrorObject};

    let test_name = &cfg.test_name();
    let send = |msg| {
        if let Err(e) = tx.send(msg) {
            tracing::error!("{}: {}: handle_message: could not send event: {}", addr, test_name, e);
        }
    };

    match wsmsg {

        Auth { access_token } => {
            let msg = if access_token == cfg.token {
                AuthOk { ha_version: cfg.ha_version().to_string() }
            } else {
                AuthInvalid { message: "wrong token".to_string() }
            };
            send(msg);
        },

        SubscribeEvents { id, .. } => {
            send(WsMessage::new_result_success(id));
            let yaml_scenario = cfg.yaml_scenario.as_ref();
            //let yaml_scenario = cfg.yaml_scenario.as_ref().unwrap();
            let file = format!("{}/{}", cfg.yaml_dir(), yaml_scenario.unwrap());
            let event_log_file = File::open(file);
            if let Err(e) = event_log_file {
                tracing::error!("{}: {}: handle message: could not open YAML event log file: {}", addr, test_name, e);
            } else {
                let event_log_reader = io::BufReader::new(event_log_file.unwrap());
                for document in serde_yaml::Deserializer::from_reader(event_log_reader) {
                    let _ = WsMessage::deserialize(document)
                        .and_then(|ev| { Ok(send(ev.set_id(id))) })
                        .or_else(|err| {
                            tracing::error!("{}: {}: handle message: could not deserialize YAML document from event log file: {}", addr, test_name, err);
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

    tracing::info!("{}: {}: handle message: done", addr, test_name);
    Ok(())
}
