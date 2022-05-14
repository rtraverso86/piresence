use clap::{self, StructOpt};
use hass::sync::shutdown;
use hass::hast::{HastConfig, Hast};
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

    /// Base directory where YAML event log files are stored
    #[clap(long, default_value = ".")]
    pub yaml_dir: String,

    /// Filename of the YAML event log to run
    pub yaml_scenario: Option<String>,

}

impl CmdArgs {
    fn to_hast_config(&self) -> HastConfig {
        let mut hc = HastConfig::new(self.port,
                self.token.clone(),
                self.yaml_dir.clone());
        if let Some(scenario) = self.yaml_scenario.as_ref() {
            hc.yaml_scenario = Some(scenario.clone());
            hc.skip_hast_messages = true;
        }
        hc
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