
use hass::{self};
use haevlo;
use tracing_subscriber;
use hass::wsapi::WsApi;
use hass::json::{WsMessage, EventType};

fn main() {
    // Initialize logging framework
    tracing_subscriber::fmt::init();

    let args = haevlo::CmdArgs::parse_args();
    tracing::debug!("commandline args: {:?}", args);
    
    let mut ws = match WsApi::new_unsecure(&args.host, args.port, &args.token) {
        Ok(ws) => ws,
        Err(e) => {
            return haevlo::exit_error(e, "connecting");
        }
    };

    if args.use_events {
        if let Err((e, when)) = haevlo::register_events(&mut ws) {
            return haevlo::exit_error(e, &when);
        }
    }

    loop {
        match ws.read_message() {
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
                ws.close();
                return haevlo::exit_error(e, "receiving messages from HA")
            }
        }
    }

    ws.close();
}
