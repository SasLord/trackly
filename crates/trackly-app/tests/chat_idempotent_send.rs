//! Идемпотентная отправка сообщения чата заявки (Phase 43, D-11).
//! Имена вымышленные.

use std::time::Duration;

use rusqlite::params;
use trackly_app::dto::printer::WsEvent;
use trackly_core::auth::Role;
use trackly_core::error::AppError;
use trackly_infra::error_conversions::map_rusqlite;

#[path = "entities_support/fixture.rs"]
#[allow(dead_code)]
mod fixture;

mod chat_support;
use chat_support::*;

macro_rules! timed {
    ($body:expr) => {
        tokio::time::timeout(Duration::from_secs(60), $body)
            .await
            .expect("timeout")
    };
}

fn chat_events(evs: &[WsEvent]) -> usize {
    evs.iter()
        .filter(|e| matches!(e, WsEvent::ChatMessage { .. }))
        .count()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn idempotent_send_replays_without_duplicate_or_event() {
    timed!(async {
        let (ctx, _dir) = fixture::make_test_ctx().await;
        let author = seed_user(&ctx, "ivanov", "Иванов И.И.", "employee").await;
        let who = ident(author, Role::Employee);
        let r = seed_request(&ctx, author, "repair", "open").await;
        let mut rx = ctx.ws_broadcast.subscribe();
        fixture::drain(&mut rx);

        assert_eq!(count(&ctx, "SELECT COUNT(*) FROM chat_messages").await, 0);
        let first = ctx
            .chat
            .send(&who, r, "Привет", &[], Some("cm-1"))
            .await
            .unwrap();
        assert_eq!(count(&ctx, "SELECT COUNT(*) FROM chat_messages").await, 1);
        let second = ctx
            .chat
            .send(&who, r, "другой текст", &[], Some("cm-1"))
            .await
            .unwrap();
        assert_eq!(count(&ctx, "SELECT COUNT(*) FROM chat_messages").await, 1);
        let (a, b) = (first.message.unwrap(), second.message.unwrap());
        assert_eq!(a.id, b.id);
        assert_eq!(
            b.text, "Привет",
            "id задаёт идентичность, текст повтора игнорируется"
        );
        assert!(second.reject_code.is_none());
        let evs = fixture::collect_events(&mut rx);
        assert_eq!(chat_events(&evs), 1, "{evs:?}");
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn replay_after_request_closed_returns_message_new_id_rejected() {
    timed!(async {
        let (ctx, _dir) = fixture::make_test_ctx().await;
        let author = seed_user(&ctx, "ivanov", "Иванов И.И.", "employee").await;
        let who = ident(author, Role::Employee);
        let r = seed_request(&ctx, author, "repair", "open").await;
        let first = ctx
            .chat
            .send(&who, r, "Привет", &[], Some("cm-1"))
            .await
            .unwrap();
        ctx.writer
            .execute(move |conn| {
                conn.execute(
                    "UPDATE requests SET status = 'completed' WHERE id = ?1",
                    params![r],
                )
                .map_err(map_rusqlite)?;
                Ok(())
            })
            .await
            .unwrap();
        let replay = ctx
            .chat
            .send(&who, r, "Привет", &[], Some("cm-1"))
            .await
            .unwrap();
        assert!(replay.reject_code.is_none(), "replay не reject Closed");
        assert_eq!(replay.message.unwrap().id, first.message.unwrap().id);
        // Фикстура различается: новый id в закрытой заявке — Closed.
        let fresh = ctx
            .chat
            .send(&who, r, "Привет", &[], Some("cm-2"))
            .await
            .unwrap();
        assert!(fresh.message.is_none());
        assert_eq!(fresh.reject_code.as_deref(), Some("closed"));
        assert_eq!(count(&ctx, "SELECT COUNT(*) FROM chat_messages").await, 1);
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn key_includes_author_and_request() {
    timed!(async {
        let (ctx, _dir) = fixture::make_test_ctx().await;
        let a = seed_user(&ctx, "ivanov", "Иванов И.И.", "employee").await;
        let b = seed_user(&ctx, "petrov", "Петров П.П.", "admin").await;
        let r1 = seed_request(&ctx, a, "repair", "open").await;
        let r2 = seed_request(&ctx, a, "repair", "open").await;
        let m1 = ctx
            .chat
            .send(&ident(a, Role::Employee), r1, "раз", &[], Some("same"))
            .await
            .unwrap()
            .message
            .unwrap();
        // Другой автор, та же заявка.
        let m2 = ctx
            .chat
            .send(&ident(b, Role::Admin), r1, "два", &[], Some("same"))
            .await
            .unwrap()
            .message
            .unwrap();
        // Тот же автор, другая заявка.
        let m3 = ctx
            .chat
            .send(&ident(a, Role::Employee), r2, "три", &[], Some("same"))
            .await
            .unwrap()
            .message
            .unwrap();
        assert!(m1.id != m2.id && m1.id != m3.id && m2.id != m3.id);
        assert_eq!(count(&ctx, "SELECT COUNT(*) FROM chat_messages").await, 3);
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn invalid_client_id_is_validation_and_no_row() {
    timed!(async {
        let (ctx, _dir) = fixture::make_test_ctx().await;
        let a = seed_user(&ctx, "ivanov", "Иванов И.И.", "employee").await;
        let who = ident(a, Role::Employee);
        let r = seed_request(&ctx, a, "repair", "open").await;
        for bad in [&"x".repeat(65)[..], "a b"] {
            let err = ctx
                .chat
                .send(&who, r, "текст", &[], Some(bad))
                .await
                .unwrap_err();
            assert!(matches!(err, AppError::Validation { .. }), "{err:?}");
        }
        assert_eq!(count(&ctx, "SELECT COUNT(*) FROM chat_messages").await, 0);
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn send_without_client_id_unchanged() {
    timed!(async {
        let (ctx, _dir) = fixture::make_test_ctx().await;
        let a = seed_user(&ctx, "ivanov", "Иванов И.И.", "employee").await;
        let who = ident(a, Role::Employee);
        let r = seed_request(&ctx, a, "repair", "open").await;
        for _ in 0..2 {
            ctx.chat.send(&who, r, "текст", &[], None).await.unwrap();
        }
        assert_eq!(count(&ctx, "SELECT COUNT(*) FROM chat_messages").await, 2);
    })
}

/// WR-04: replay отдаёт то же сообщение с clientMsgId автора.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn replay_returns_client_msg_id() {
    timed!(async {
        let (ctx, _dir) = fixture::make_test_ctx().await;
        let author = seed_user(&ctx, "ivanov", "Иванов И.И.", "employee").await;
        let who = ident(author, Role::Employee);
        let r = seed_request(&ctx, author, "repair", "open").await;
        let first = ctx
            .chat
            .send(&who, r, "Привет", &[], Some("m-1"))
            .await
            .unwrap();
        let second = ctx
            .chat
            .send(&who, r, "Привет", &[], Some("m-1"))
            .await
            .unwrap();
        assert_eq!(first.message.unwrap().client_msg_id.as_deref(), Some("m-1"));
        assert_eq!(
            second.message.unwrap().client_msg_id.as_deref(),
            Some("m-1")
        );
    })
}
