use std::env;
use std::process;
use hass;
use piresence::CmdArgs;
use tracing_subscriber;
use hass::wsapi::WsApi;

fn main() {
    // Initialize logging framework
    tracing_subscriber::fmt::init();

    let args = CmdArgs::parse_args();
    tracing::trace!("args: {:?}", args);
    //hass::wsconnect(&args.host, args.port, &args.token);
    let ws = WsApi::new_unsecure(&args.host, args.port, &args.token).unwrap();

    /*
    let host = env::var("HA_HOST").expect("environment variable missing: HA_HOST");
    let port : u16 = env::var("HA_PORT")
        .unwrap_or("8123".into())
        //.expect("environment variable missing: HA_PORT")
        .parse()
        .expect("environment variable error: HA_PORT only accepts 16 bit unsigned integers");
    let token = env::var("HA_TOKEN")
        .unwrap_or("ABCDEFG".into());
    */
}
