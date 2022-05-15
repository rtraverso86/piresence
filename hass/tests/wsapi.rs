mod commons;

use commons::*;
use hass::WsApi;
use hass::WsMessage;
use hass::error as herror;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn auth_failed() {
    let manager = hast_start(HAEVLO_000_BASE.0).await;
    let wsapi = WsApi::new_unsecure(WS_HOST, WS_PORT, &format!("{}_", WS_TOKEN), manager.subscribe()).await;
    manager.shutdown().await;
    match wsapi {
        Err(herror::Error::Authentication(_)) => (),
        o => panic!("unexpected result: {:?}", o)
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn auth_success() {
    let manager = hast_start(HAEVLO_000_BASE.0).await;
    let wsapi = hast_connect(&manager).await;
    manager.shutdown().await;
    match wsapi {
        Ok(_) => (),
        Err(e) => panic!("unexpected error: {}", e)
    }
}


#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn receive_events_any() {
    let manager = hast_start(HAEVLO_000_BASE.0).await;
    let wsapi = hast_connect(&manager).await.unwrap();

    let mut rx = wsapi.subscribe_event(None).await.unwrap();
    let mut count = 0;
    while let Some(msg) = rx.recv().await {
        count += 1;
        assert!(msg.id().is_some());
        if count == HAEVLO_000_BASE.1 {
            let msg_id = msg.id().unwrap();
            let msg = wsapi.unsubscribe(msg_id).await.unwrap();
            assert!(matches!(msg, WsMessage::Result { success: true, ..}));
            break;
        }
    }
    if let Some(msg) = rx.recv().await {
        panic!("should have returned None, but instead got: {:?}", msg);
    }

    manager.shutdown().await;
}