use std::fs::File;
use std::{io, sync::Arc};
use hass::serde::Deserialize;
use hass::serde_yaml;
use hass::sync::shutdown;
use hass::{WsMessage, json::Id};
use hast::{self, HastConfig, Hast};
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

impl CmdArgs {
    fn to_hast_config(&self) -> HastConfig {
        HastConfig {
            port: self.port,
            token: self.token.clone(),
            yaml_event_log: self.yaml_event_log.clone(),
        }
    }
}


#[tokio::main]
async fn main() -> Result<(), io::Error> {
    // Initialize logging framework
    tracing_subscriber::fmt::init();

    let args = CmdArgs::parse();
    let hast_cfg = args.to_hast_config();
    let manager = shutdown::Manager::new();

    let hast = Hast::new(hast_cfg, manager.subscribe());

    hast.start().await;

    Ok(())
}