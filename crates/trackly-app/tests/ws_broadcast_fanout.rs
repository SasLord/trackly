//! Regression coverage for the ws_broadcast → Tauri bridge gap-closure fix
//! (09-ad live-verify: browser-originated mutations never reached the
//! desktop webview, because nothing forwarded `ctx.ws_broadcast` events to
//! `app.emit("trackly-event", ...)`).
//!
//! The fix wires a second `ctx.ws_broadcast.subscribe()` consumer in
//! `main.rs`'s `.setup(...)` closure, alongside the existing browser-WS
//! consumer in `http/ws.rs::ws_handler`. This is desktop-runtime behavior
//! that can't be exercised directly without a real Tauri `AppHandle`, but
//! the property the bridge depends on — that `tokio::sync::broadcast`
//! fans the SAME event out to *every* independent subscriber, not just the
//! first one — is fully testable here. This proves a second subscriber
//! (standing in for the Tauri bridge task) receives the identical `WsEvent`
//! a browser WS client (`ws_handler`'s `ctx.ws_broadcast.subscribe()`)
//! would receive, with no loss/duplication for either side.

use std::sync::Arc;

use trackly_app::dto::printer::WsEvent;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn broadcast_fans_out_identical_event_to_every_subscriber() {
    let (tx, mut browser_rx) = tokio::sync::broadcast::channel::<WsEvent>(128);
    let tx = Arc::new(tx);

    // A second subscriber, registered AFTER the first — mirrors the
    // ordering in AppCtx::build (ws_broadcast created once, shared via
    // Arc::clone) vs. main.rs's bridge subscribing inside `.setup(...)`
    // and http/ws.rs's `ws_handler` subscribing per-connection. Order of
    // `.subscribe()` calls must not matter — both get every event sent
    // after they subscribed.
    let mut desktop_rx = tx.subscribe();

    let event = WsEvent::RequestStatusChanged {
        request_id: 42,
        new_status: "completed".to_string(),
        requested_by_user_id: 7,
    };

    let sent = tx.send(event.clone());
    assert!(
        sent.is_ok(),
        "send should succeed with at least one subscriber"
    );
    assert_eq!(
        sent.unwrap(),
        2,
        "exactly 2 receivers (browser_rx + desktop_rx) should have been notified"
    );

    let browser_got = browser_rx
        .recv()
        .await
        .expect("browser subscriber must receive the event");
    let desktop_got = desktop_rx
        .recv()
        .await
        .expect("desktop-bridge-stand-in subscriber must receive the same event");

    // Both sides must observe an identical, unwrapped WsEvent — the bridge
    // re-emits the SAME serde-serialized WsEvent via `app.emit`, no
    // wrap/rename (ui/src/lib/api/ws.ts dispatches `e.payload` directly).
    match (&browser_got, &desktop_got) {
        (
            WsEvent::RequestStatusChanged {
                request_id: rid1,
                new_status: status1,
                requested_by_user_id: uid1,
            },
            WsEvent::RequestStatusChanged {
                request_id: rid2,
                new_status: status2,
                requested_by_user_id: uid2,
            },
        ) => {
            assert_eq!(rid1, rid2);
            assert_eq!(status1, status2);
            assert_eq!(uid1, uid2);
            assert_eq!(*rid1, 42);
            assert_eq!(status1, "completed");
            assert_eq!(*uid1, 7);
        }
        _ => panic!("expected RequestStatusChanged on both subscribers, got {browser_got:?} / {desktop_got:?}"),
    }
}
