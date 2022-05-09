use std::process;
use clap::Parser;
use hass::WsMessage;
use hass::error::{self, Error};
use hass::json::EventType;
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

#[derive(Copy, Clone)]
pub enum ExitCodes {
    Success = 0,
    CouldNotConnect = 1,
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

pub async fn run(args: CmdArgs, shutdown: Shutdown) {
    let use_events = self.args.use_events;
    let mut recording = !use_events;
    loop {
        tokio::select! {
            Some(ev) = self.registered_events(), if use_events => match ev {
                EventType::HaevloStart => {
                    recording = true;
                },
                EventType::HaevloStop => {
                    recording = false;
                },
                _ => (),
            },
            Some(st) = self.state_events.recv() => {
                if !recording {
                    continue;
                }
                
            },
            else => break,
        }
    }
}

async fn registered_events(&mut self) -> Option<EventType> {
    match self.control_events.as_mut() {
        None => None,
        Some(rx) => match rx.recv().await {
            Some(WsMessage::Event { data }) => {
                data.event.event_type
            },
            _ => None
        },
    }
}



async fn register_control_events(api: &WsApi) -> error::Result<Receiver<WsMessage>> {
    let events = [EventType::HaevloStart, EventType::HaevloStop];
    api.subscribe_events(&events).await
}

pub fn exit_error(err: Error, when: &str) {
    tracing::error!("error while {}: {}", when, err);
    process::exit(-1);
}

