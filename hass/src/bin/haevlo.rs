use clap::Parser;
use hass;
use hass::error::{self, Error};
use hass::sync::shutdown;
use hass::wsapi::WsApi;
use hass::json::{WsMessage, EventType, EventObj};
use tokio;
use tokio::io::{self, AsyncWriteExt};
use tokio::fs::{OpenOptions, File};
use tokio::sync::mpsc::Receiver;
use tokio::signal;
use tracing_subscriber;

type AppError<'a> = (ExitCode, Option<(Error, &'a str)>);
type AppResult<'a> = Result<(), AppError<'a>>;


/// Command-line arguments for the binary
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct CmdArgs {
    /// Host running Home Assistant
    #[clap(long)]
    host: String,

    /// Port where Home Assistant websockets are located
    #[clap(long, default_value_t = 8123)]
    port: u16,

    /// Authentication token for Home Assistant
    #[clap(long)]
    token: String,

    /// Enable event logging start/stop via HA events.
    /// When enabled, generate a custom event `haevlo_start`
    /// from Home Assistant to start logging, and `haevlo_stop`
    /// to quit.
    #[clap(long)]
    use_events: bool,

    #[clap(long, default_value = ".")]
    output_folder: String,

    test_name: String,
}

impl CmdArgs {
    fn parse_args() -> CmdArgs {
        CmdArgs::parse()
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum ExitCode {
    Success = 0,
    ConnectionError,
    ControlSubscriptionError,
    StateSubscriptionError,
    OpenFileError,
}

impl ExitCode {
    fn is_success(&self) -> bool {
        *self == ExitCode::Success
    }
}

async fn register_control_events(api: &WsApi) -> error::Result<Receiver<WsMessage>> {
    let events = [EventType::HaevloStart, EventType::HaevloStop];
    api.subscribe_events(&events).await
}

fn filter_event(msg: WsMessage) -> Option<WsMessage> {
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

async fn append_event(msg: WsMessage , file: &mut Option<File>) -> io::Result<()> {
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


async fn recv_ctrl_events(rx_opt: &mut Option<Receiver<WsMessage>>) -> Option<EventType> {
    match rx_opt.as_mut() {
        None => None,
        Some(rx) => match rx.recv().await {
            Some(WsMessage::Event { event: EventObj::Event { event_type, .. } ,.. }) => {
                Some(event_type)
            },
            _ => None
        },
    }
}

fn app_err<'a>(code: ExitCode, err: Error, msg: &'a str) -> AppResult<'a> {
    Err(( code, Some((err, msg)) ))
}

async fn run_app() -> AppResult<'static> {
    // Initialize logging framework
    tracing_subscriber::fmt::init();

    let args = CmdArgs::parse_args();
    tracing::debug!("commandline args: {:?}", args);

    let manager = shutdown::Manager::new();

    let api = match WsApi::new_unsecure(&args.host, args.port, &args.token, manager.subscribe()).await {
        Ok(u) => u,
        Err(e) => {
            return app_err(ExitCode::ConnectionError, e, "could not connect to HA WebSocket");
        }
    };

    let mut control_events = if args.use_events {
        match register_control_events(&api).await {
            Ok(rx) => Some(rx),
            Err(e) => {
                return app_err(ExitCode::ControlSubscriptionError, e, "could not subscribe to events: haevlo_start, haevlo_stop");
            }
        }
    } else {
        None
    };

    let mut state_events = match api.subscribe_event(Some(EventType::StateChanged)).await {
        Ok(rx) => rx,
        Err(err) => {
            return app_err(ExitCode::StateSubscriptionError, err, "could not subscribe to events: state_changed");
        }
    };

    let mut recording = !args.use_events;
    let mut recording_index = 0;
    let mut file_opt = if recording {
        Some(open_file(&args, recording_index).await?)
    } else {
        None
    };

    loop {
        tokio::select! {
            Some(ev) = recv_ctrl_events(&mut control_events), if args.use_events => match ev {
                EventType::HaevloStart => {
                    recording = true;
                    recording_index += 1;
                    if let Some(mut prev_file) = file_opt.replace(open_file(&args, recording_index).await?) {
                        prev_file.flush().await;
                    }
                    tracing::info!("haevlo_start event: started logging: #{}", recording_index);
                },
                EventType::HaevloStop => {
                    recording = false;
                    tracing::info!("haevlo_stop event: stopped logging: #{}", recording_index);
                },
                _ => (),
            },
            Some(st) = state_events.recv() => {
                if !recording {
                    continue;
                }
                if let Err(e) = append_event(st, &mut file_opt).await {
                    tracing::error!("IO error appending event to output file: {}", e);
                }
            },
            _ = signal::ctrl_c() => {
                tracing::info!("CTRL-C detected, shutting down");
                break;
            },
            else => break,
        }
    }
    manager.shutdown().await;

    Ok(())
}

async fn open_file(args: &CmdArgs, idx: i32) -> Result<File, AppError<'static>> {
    let file_name = format!("{}/{}-{}.yaml", args.output_folder, args.test_name, idx);
    tracing::info!("opened {} for writing", file_name);
    OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(file_name)
        .await
        .or_else(|e| {
            tracing::error!("{}", e);
            Err((ExitCode::OpenFileError, None))
        })
}

#[tokio::main]
async fn main() {
    std::process::exit(match run_app().await {
        Ok(_) => {
            let code = ExitCode::Success;
            tracing::info!("exit: {:?} ({})", code, code as i32);
            code
        },
        Err((code, err_opt)) => {
            if let Some((e, msg)) = err_opt {
                tracing::error!("{}: {}", e, msg)
            }
            if code.is_success() {
                tracing::info!("exit: {:?} ({})", code, code as i32);
            } else {
                tracing::error!("exit with error: {:?} ({})", code, code as i32);
            }
            code
        }
    } as i32);

}