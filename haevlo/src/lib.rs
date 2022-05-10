use std::fmt::{Display, Write};
use std::process;
use clap::Parser;
use hass::WsMessage;
use hass::error::{self, Error};
use hass::json::{EventType, EventObj};
use hass::sync::shutdown::Shutdown;
use hass::wsapi::WsApi;
use tokio::sync::mpsc::Receiver;

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

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum ExitCode {
    Success = 0,
    ConnectionError,
    ControlSubscriptionError,
    StateSubscriptionError,
}

impl ExitCode {
    pub fn is_success(&self) -> bool {
        *self == ExitCode::Success
    }
}

pub async fn build_wsapi(args: &CmdArgs, shutdown: Shutdown) -> error::Result<WsApi> {
    let api = WsApi::new_unsecure(&args.host, args.port, &args.token, shutdown).await?;

    let control_events = if args.use_events {
        Some(register_control_events(&api).await?)
    } else {
        None
    };

    let state_events = api.subscribe_event(Some(EventType::StateChanged)).await?;

    Ok(api)
}

pub async fn register_control_events(api: &WsApi) -> error::Result<Receiver<WsMessage>> {
    let events = [EventType::HaevloStart, EventType::HaevloStop];
    api.subscribe_events(&events).await
}

pub fn filter_event(msg: WsMessage) -> Option<WsMessage> {
    use hass::serde_json::value::{self, Value};
    match &msg {
        WsMessage::Event { event: EventObj::Event { data, ..}, .. } => {
            if let Some(device_class) = data.pointer("/new_state/attributes/device_class") {
                if device_class.is_string() && device_class.as_str().unwrap() == "motion" {
                    return Some(msg);
                }
            }
        },
        _ => ()
    };
    None
}
