use clap::{self, StructOpt};
use hass::sync::shutdown;
use hass::hast::{HastConfig, Hast};
use std::io;
use tokio::{self, signal};
use tokio_tungstenite::tungstenite::Result;
use tracing_subscriber;

/// Home Assistant Surrogate Tool
///
/// The Home Assistant Surrogate Tool, hast for short, is a mock Home Assistant (HA)
/// WebSocket server that supports basic end-to-end testing features.
///
/// Clients connecting to the mock service should expect the same behaviour of a real
/// HA instance, with the difference that upon subscription, all events are sent in
/// a single burst of messages to speed things up. The client should be aware of this
/// and adjust any time-based calculation on the timestamps included in messages, rather
/// than on real system clocks.
/// 
/// Another difference with real HA, is the preliminary setup phase which include new
/// kinds of messages to customize the behaviour of the mock before actually starting
/// the HA simulation. This phase is only available when the optional YAML_SCENARIO
/// positional argument is not provided.
/// Please refer to the [hass::hast] module for more details.
#[derive(clap::Parser, Debug)]
#[clap(author, version)]
struct CmdArgs {

    /// Port used to expose the mock HA WebSocket service
    #[clap(long, default_value_t = 8123)]
    pub port: u16,

    /// Authentication token required by the mock HA WebSocket service
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