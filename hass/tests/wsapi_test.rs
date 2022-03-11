mod stubs;

use stubs::WsApiServer;
use hass::wsapi::WsApi;
use std::sync::Once;
use tracing_subscriber;

static INIT: Once = Once::new();

fn init_test() {
    INIT.call_once(|| {
        tracing_subscriber::fmt::init();
    });
}

#[test]
fn my_test() {
    init_test();
    let stub = WsApiServer::new(18123);
    let ws = WsApi::new_unsecure(stub.host(), stub.port(), "helloToken").unwrap();
    stub.stop();
    assert!(false);
}