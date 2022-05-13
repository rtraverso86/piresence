use clap::{self, StructOpt};
use hass::sync::shutdown;
use hast::{self, HastConfig, Hast};
use std::io;
use tokio::{self, signal};
use tokio_tungstenite::tungstenite::Result;
use tracing_subscriber;

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
        HastConfig::new(self.port,
            self.token.clone(),
            self.yaml_event_log.clone())
    }
}

#[tokio::main]
async fn main() -> Result<(), io::Error> {
    tracing_subscriber::fmt::init();

    let args = CmdArgs::parse();
    tracing::info!("args: {:?}", args);

    let hast_cfg = args.to_hast_config();
    let manager = shutdown::Manager::new();

    let hast = Hast::new(hast_cfg, manager.subscribe());

    tokio::spawn(async move {
        if let Err(e) = hast.run().await {
            tracing::error!("hast terminated with error: {}", e);
        }
    });

    if let Err(e) = signal::ctrl_c().await {
        tracing::error!("failed to wait for ctrl-c signal: {}", e);
    }
    manager.shutdown().await;

    tracing::info!("all task terminated, quitting");
    Ok(())
}