use clap::Parser;

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
}

impl CmdArgs {
    pub fn parse_args() -> CmdArgs {
        CmdArgs::parse()
    }
}