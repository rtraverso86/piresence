use std::process;
use clap::Parser;
use hass::error::Error;
use hass::wsapi::WsApi;
use hass::json::EventType;
use hass::json::Id;

/// Command-line arguments for the binary
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct CmdArgs {
    /// Host running Home Assistant
    #[clap(long)]
    pub host: String,

    /// Port where Home Assistant websockets are located
    #[clap(long, default_value_t = 8123)]
    pub port: u16,

    /// Authentication token for Home Assistant
    #[clap(long)]
    pub token: String,

    /// Enable event logging start/stop via HA events.
    /// When enabled, generate a custom event `haevlo_start`
    /// from Home Assistant to start logging, and `haevlo_stop`
    /// to quit.
    #[clap(long)]
    pub use_events: bool,
}

impl CmdArgs {
    pub fn parse_args() -> CmdArgs {
        CmdArgs::parse()
    }
}

#[derive(Copy, Clone)]
pub enum ExitCodes {
    Success = 0,
    CouldNotConnect = 1,
}

pub fn exit_error(err: Error, when: &str) {
    tracing::error!("error while {}: {}", when, err);
    process::exit(-1);
}

pub async fn register_events(ws: &mut WsApi) -> Result<Vec<Id>, (Error, String)> {
    let events = [EventType::HaevloStart, EventType::HaevloStop];
    ws.subscribe_events(&events).await.map_err(|(idx, err)| {
        (
            err,
            format!("registering to event {}", events[idx]),
        )
    })
}
