use hass;
use hass::error::Error;
use hass::sync::shutdown;
use hass::wsapi::WsApi;
use hass::json::{WsMessage, EventType};
use tokio;
use tokio::sync::mpsc::Receiver;
use tracing_subscriber;
use haevlo::{self, ExitCode};

type AppError<'a> = (ExitCode, Option<(Error, &'a str)>);
type AppResult<'a> = Result<(), AppError<'a>>;

async fn recv_ctrl_events(rx_opt: &mut Option<Receiver<WsMessage>>) -> Option<EventType> {
    match rx_opt.as_mut() {
        None => None,
        Some(rx) => match rx.recv().await {
            Some(WsMessage::Event { data }) => {
                data.event.event_type
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

    let args = haevlo::CmdArgs::parse_args();
    tracing::debug!("commandline args: {:?}", args);

    let manager = shutdown::Manager::new();

    let api = match WsApi::new_unsecure(&args.host, args.port, &args.token, manager.subscribe()).await {
        Ok(u) => u,
        Err(e) => {
            return app_err(ExitCode::ConnectionError, e, "could not connect to HA WebSocket");
        }
    };

    let mut control_events = if args.use_events {
        match haevlo::register_control_events(&api).await {
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
    loop {
        tokio::select! {
            Some(ev) = recv_ctrl_events(&mut control_events), if args.use_events => match ev {
                EventType::HaevloStart => {
                    recording = true;
                    tracing::info!("haevlo_start event: started logging");
                },
                EventType::HaevloStop => {
                    recording = false;
                    tracing::info!("haevlo_stop event: stopped logging");
                    break;
                },
                _ => (),
            },
            Some(st) = state_events.recv() => {
                if !recording {
                    continue;
                }
                tracing::info!("state_changed event:\n{}", st);
            },
            else => break,
        }
    }

    Ok(())
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