use hass::WsApi;
use hass::sync::shutdown::Manager;
use hass::hast::server::{HastConfig, Hast};

pub const WS_HOST: &str = "127.0.0.1";
pub const WS_PORT: u16 = 8123;
pub const WS_TOKEN: &str = "letmein";
pub const WS_YAML_DIR: &str = "tests/resources";

/// Home assistant event log resource info: `(name, event_count)`
pub const HAEVLO_000_BASE: (&str, u32) = ("000-base.yaml", 8);


/// Starts a new Hast mock server with default configuration [WS_PORT]
/// and [WS_TOKEN], looking for the given `scenario` in [WS_YAML_DIR].
///
/// The method returns as soon as the service is up & ready for clients
/// to connect to.
///
/// Tests using it should rely on `#[serial_test::serial]` to avoid
/// clashes on the port binding.
pub async fn hast_start(scenario: &str) -> Manager {
    let manager = Manager::new();
    let yaml_dir = format!("{}/{}/", env!("CARGO_MANIFEST_DIR"), WS_YAML_DIR);
    let cfg = HastConfig::new_with_scenario(
        WS_PORT,
        WS_TOKEN.to_owned(),
        yaml_dir,
        Some(scenario.to_owned())
    );

    let hast = Hast::new(cfg, manager.subscribe());
    let mut startup_notifier = hast.startup_notifier();

    tokio::spawn(async move {
        println!("hast starting");
        if let Err(e) = hast.run().await {
            println!("hast quit with error: {}", e);
        }
    });

    let _ = startup_notifier.changed().await;

    manager
}

pub async fn hast_connect(m: &Manager) -> hass::error::Result<WsApi> {
    WsApi::new_unsecure(WS_HOST, WS_PORT, WS_TOKEN, m.subscribe()).await
}
