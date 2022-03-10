mod stubs;

use stubs::WsApiServer;
use hass::wsapi::WsApi;

#[test]
fn my_test() {
    let stub = WsApiServer::new(18123);
    let ws = WsApi::new_unsecure(stub.host(), stub.port(), "helloToken").unwrap();
    stub.stop();
}