//! Desktop bridge visibility gate (Phase 41.5 security audit, T-41.5-06).
//! The desktop webview must not receive other users' read/mute markers.
//! Names are fictional.

use std::time::Duration;

use trackly_app::desktop_event_bridge::desktop_event_visible;
use trackly_app::dto::printer::WsEvent;

#[path = "entities_support/fixture.rs"]
#[allow(dead_code)]
mod fixture;

mod attachments_support;
mod chat_support;
use chat_support::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn desktop_bridge_hides_foreign_read_markers() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let (ctx, _dir) = fixture::make_test_ctx().await;
        let other = seed_user(&ctx, "ivanov", "Иванов И.И.", "employee").await;
        let r = seed_request(&ctx, other, "repair", "open").await;
        let desktop = ctx.auth.system_admin_identity().await.unwrap();
        let desktop_id = desktop.user_id.expect("system admin has a user id");
        assert_ne!(desktop_id, other);

        // Another user's marker: dropped (even though the desktop is Admin).
        assert!(
            !desktop_event_visible(&ctx, &WsEvent::ChatReadChanged { user_id: other }).await,
            "foreign ChatReadChanged must not reach the desktop webview"
        );
        // The desktop operator's own marker: forwarded.
        assert!(
            desktop_event_visible(
                &ctx,
                &WsEvent::ChatReadChanged {
                    user_id: desktop_id
                }
            )
            .await
        );
        // A chat message in a request the desktop admin is not part of:
        // forwarded (Admin sees every chat message).
        assert!(
            desktop_event_visible(
                &ctx,
                &WsEvent::ChatMessage {
                    request_id: r,
                    message_id: 1,
                    actor_user_id: Some(other),
                    actor_name: Some("Иванов И.И.".to_string()),
                    requested_by_user_id: Some(other),
                    participant_user_ids: vec![other],
                },
            )
            .await
        );
    })
    .await
    .expect("timeout");
}

/// The gate must actually be wired: in `main.rs` the only `trackly-event`
/// emit of the bridge sits behind `desktop_event_visible`.
#[test]
fn desktop_bridge_emit_is_gated() {
    let src = include_str!("../src/main.rs");
    let emits: Vec<_> = src.match_indices("emit(\"trackly-event\"").collect();
    assert_eq!(emits.len(), 1, "exactly one trackly-event emit in main.rs");
    let emit_at = emits[0].0;
    let gate_at = src
        .rfind("desktop_event_visible(")
        .expect("bridge calls desktop_event_visible");
    assert!(gate_at < emit_at, "visibility gate must precede the emit");
    let between = &src[gate_at..emit_at];
    assert!(
        between.contains("if ") || between.contains(".await\n"),
        "emit must be inside the gate's branch"
    );
    assert!(
        !between.contains("};") && !between.contains("loop"),
        "emit must follow the gate directly"
    );
}
