use clap::Args;
use clap::Parser;
use hass::WsMessage;
use hass::error;
use hass::json::{EventType, EventObj};
use hass::wsapi::WsApi;
use tokio::fs::File;
use tokio::io;
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

    #[clap(long, default_value = ".")]
    pub output_folder: String,

    pub test_name: String,
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
    OpenFileError,
}

impl ExitCode {
    pub fn is_success(&self) -> bool {
        *self == ExitCode::Success
    }
}

pub async fn register_control_events(api: &WsApi) -> error::Result<Receiver<WsMessage>> {
    let events = [EventType::HaevloStart, EventType::HaevloStop];
    api.subscribe_events(&events).await
}

pub fn filter_event(msg: WsMessage) -> Option<WsMessage> {
    use hass::serde_json::value::Value;
    if let WsMessage::Event { event: EventObj::Event { data, ..}, .. } = &msg {
        if let Some(Value::String(device_class)) = data.pointer("/new_state/attributes/device_class") {
            if device_class == "motion" {
                return Some(msg);
            }
        }
    }
    None
}

pub async fn append_event(msg: WsMessage , file: &mut Option<File>) -> io::Result<()> {
    use hass::serde_yaml;
    use tokio::io::AsyncWriteExt;
    if let Some(msg) = filter_event(msg) {
        tracing::debug!("raw state_changed event:\n{}", msg);
        if let Ok(yaml) = serde_yaml::to_string(&msg) {
            tracing::info!("received new state_change event:\n{}", yaml);
            if let Some(file) = file {
                let _ = file.write(yaml.as_bytes()).await?;
            }
        }
    }
    Ok(())
}
