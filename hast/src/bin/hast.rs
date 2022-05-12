use std::fs::File;
use std::{io, sync::Arc};
use hass::serde::Deserialize;
use hass::serde_yaml;
use hass::{WsMessage, json::Id};
use hast;
use clap::{self, StructOpt};
use tracing_subscriber;
use tokio::{self, net::{TcpListener, TcpStream}, sync::mpsc::{self, UnboundedSender}};
use tokio_tungstenite::tungstenite::{Result, Message};
use futures_util::{StreamExt, SinkExt};

#[derive(clap::Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct CmdArgs {

    /// Port where Home Assistant websockets are located
    #[clap(long, default_value_t = 8123)]
    pub port: u16,

    /// Authentication token required to connect
    #[clap(long, default_value = "letmein")]
    pub token: String,

    /// Path to YAML event log file
    pub yaml_event_log: String,

}

const HA_VERSION : &str = "0.1-STUB";

fn result_success(id: Id) -> WsMessage {
    WsMessage::Result {
        id,
        success: true,
        data: hass::json::ResultBody::Result {
            result: None
        }
    }
}

async fn handle_message(wsmsg: WsMessage, tx: UnboundedSender<WsMessage>, args : Arc<Box<CmdArgs>>) -> Result<()> {
    use hass::json::{WsMessage::*, ResultBody, ErrorObject};
    match wsmsg {

        Auth { access_token } => {
            let msg = if access_token == args.token {
                AuthOk { ha_version: HA_VERSION.to_string() }
            } else {
                AuthInvalid { message: "wrong token".to_string() }
            };
            tx.send(msg).unwrap();
        },

        SubscribeEvents { id, .. } => {
            tx.send(result_success(id)).unwrap();
            let event_log_file = File::open(&args.yaml_event_log).unwrap();
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

async fn accept_connection<'a>(stream: TcpStream, args: Arc<Box<CmdArgs>>) -> Result<()> {
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
            let args_clone = args.clone();
            tokio::spawn(async move {
                handle_message(wsmsg, tx_clone, args_clone).await.unwrap();
            });
        }
    }

    drop(tx);

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), io::Error> {
    // Initialize logging framework
    tracing_subscriber::fmt::init();

    let args = Arc::new(Box::new(CmdArgs::parse()));
    let addr = format!("127.0.0.1:{}", args.port);

    // Create the event loop and TCP listener we'll accept connections on.
    let try_socket = TcpListener::bind(&addr).await;
    let listener = try_socket.expect("Failed to bind");
    tracing::info!("listening on: {}", addr);

    while let Ok((stream, _)) = listener.accept().await {
        let args_clone = args.clone();
        tokio::spawn(accept_connection(stream, args_clone));
    }

    hast::hast();

    Ok(())
}