use hass::sync::shutdown::Manager;
use hass::hast::server::{HastConfig, Hast};

const WS_PORT : u16 = 8123;
const WS_TOKEN : &str = "letmein";

async fn new(scenario: &str) -> Manager {
    let manager = Manager::new();
    let yaml_dir = format!("{}/resources/", env!("CARGO_MANIFEST_DIR"));
    let cfg = HastConfig::new_with_scenario(
        WS_PORT,
        WS_TOKEN.to_owned(),
        yaml_dir,
        Some(scenario.to_owned())
    );

    let hast = Hast::new(cfg, manager.subscribe());
    let mut startup_notifier = hast.startup_notifier();

    tokio::spawn(async move {
        if let Err(e) = hast.run().await {
            println!("hast quit with error: {}", e);
        }
    });

    let _ = startup_notifier.changed().await;

    manager
}


#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn test() {
    let manager = new("000-base.yaml").await;
    
    manager.shutdown().await;
}


#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]

async fn test2() {
    let manager = new("000-base.yaml").await;
    
    manager.shutdown().await;
}


#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]

async fn test3() {
    let manager = new("000-base.yaml").await;
    
    manager.shutdown().await;
}