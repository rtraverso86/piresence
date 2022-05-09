
use hass::{self};
use hass::sync::shutdown;
use hass::wsapi::WsApi;
use hass::json::{WsMessage, EventType};
use tokio;
use tracing_subscriber;


#[tokio::main]
async fn main() {
    // Initialize logging framework
    tracing_subscriber::fmt::init();

    let args = haevlo::CmdArgs::parse_args();
    tracing::debug!("commandline args: {:?}", args);

    let manager = shutdown::Manager::new();
    let api = haevlo::build_wsapi(args, manager.shutdown())
    let mut ws = match WsApi::new_unsecure(&args.host, args.port, &args.token, manager.subscribe()).await {
        Ok(ws) => ws,
        Err(e) => {
            return haevlo::exit_error(e, "connecting");
        }
    };

    let mut recording = !args.use_events;
    loop {
        tokio::select! {
            Some(ev) = registered_events(), if args.use_events => match ev {
                EventType::HaevloStart => {
                    recording = true;
                },
                EventType::HaevloStop => {
                    recording = false;
                },
                _ => (),
            },
            else => {
                break;
            }
        }
    }

    if args.use_events {
        if let Err((e, when)) = haevlo::register_control_events(&mut ws).await {
            return haevlo::exit_error(e, &when);
        }
    }

    loop {
        match ws.read_message().await {
            Ok(msg) => {
                match msg {
                    WsMessage::Event { data } => {
                        tracing::debug!("received event: {:?}", data);
                        if data.event.event_type.unwrap_or_default() == EventType::HaevloStop {
                            break;
                        }
                    },
                    _ => ()
                };
            },
            Err(e) => {
                ws.close().await;
                return haevlo::exit_error(e, "receiving messages from HA")
            }
        }
    }

}
