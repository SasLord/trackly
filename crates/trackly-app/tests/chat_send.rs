//! Chat send (Phase 41.5, D-05/D-07/D-08/D-14/D-16/D-23): atomic message with
//! files, reject codes, forged draft ids, WS after commit. Names are fictional.

use std::time::Duration;

use rusqlite::params;
use trackly_app::context::AppCtx;
use trackly_app::dto::chat::ChatItemDto;
use trackly_app::dto::printer::WsEvent;
use trackly_app::services::attachment_service::ParentRef;
use trackly_core::auth::{Identity, Role};
use trackly_core::domain::attachments::ParentKind;
use trackly_core::error::AppError;
use trackly_infra::error_conversions::map_rusqlite;

#[path = "entities_support/fixture.rs"]
#[allow(dead_code)]
mod fixture;

mod attachments_support;
mod chat_support;
use chat_support::*;

macro_rules! timed {
    ($body:expr) => {
        tokio::time::timeout(Duration::from_secs(60), $body)
            .await
            .expect("timeout")
    };
}

fn draft(r: i64) -> ParentRef {
    ParentRef {
        kind: ParentKind::MessageDraft,
        id: r,
    }
}

async fn draft_file(ctx: &AppCtx, who: &Identity, r: i64, name: &str) -> i64 {
    upload(ctx, who, draft(r), name, &attachments_support::tiny_png())
        .await
        .expect("upload")
        .attachment
        .expect("attachment")
        .id
}

async fn set_status(ctx: &AppCtx, r: i64, status: &'static str) {
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "UPDATE requests SET status = ?1 WHERE id = ?2",
                params![status, r],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("status");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn chat_send_in_open_statuses() {
    timed!(async {
        let (ctx, _dir) = fixture::make_test_ctx().await;
        let author = seed_user(&ctx, "ivanov", "Иванов И.И.", "employee").await;
        let who = ident(author, Role::Employee);
        for st in ["open", "in_progress", "in_repair"] {
            let r = seed_request(&ctx, author, "repair", st).await;
            let out = ctx.chat.send(&who, r, "  Привет  ", &[]).await.unwrap();
            let m = out.message.expect("message");
            assert!(out.reject_code.is_none());
            assert_eq!(m.text, "Привет", "trimmed");
            assert!(m.is_own);
            let feed = ctx.chat.feed(&who, r).await.unwrap();
            let found = feed
                .items
                .iter()
                .any(|i| matches!(i, ChatItemDto::Message(x) if x.id == m.id && x.is_own));
            assert!(found, "{st}: in feed");
        }
        assert_eq!(
            count(
                &ctx,
                "SELECT COUNT(*) FROM chat_messages WHERE author_user_id IS NOT NULL"
            )
            .await,
            3
        );
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn chat_send_closed_statuses_and_race() {
    timed!(async {
        let (ctx, _dir) = fixture::make_test_ctx().await;
        let author = seed_user(&ctx, "ivanov", "Иванов И.И.", "employee").await;
        let who = ident(author, Role::Employee);
        for st in ["completed", "rejected", "cancelled"] {
            let r = seed_request(&ctx, author, "repair", st).await;
            let out = ctx.chat.send(&who, r, "текст", &[]).await.unwrap();
            assert_eq!(out.reject_code.as_deref(), Some("closed"), "{st}");
            assert!(out.message.is_none());
        }
        assert_eq!(count(&ctx, "SELECT COUNT(*) FROM chat_messages").await, 0);

        // Closed AFTER the draft was uploaded and BEFORE send.
        let r = seed_request(&ctx, author, "repair", "open").await;
        let id = draft_file(&ctx, &who, r, "фото.png").await;
        set_status(&ctx, r, "completed").await;
        let out = ctx.chat.send(&who, r, "", &[id]).await.unwrap();
        assert_eq!(out.reject_code.as_deref(), Some("closed"));
        assert_eq!(count(&ctx, "SELECT COUNT(*) FROM chat_messages").await, 0);
        assert_eq!(
            count(
                &ctx,
                "SELECT COUNT(*) FROM attachments WHERE parent_kind = 'message_draft'"
            )
            .await,
            0,
            "drafts of a closed request are cleaned"
        );
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn chat_send_text_limits() {
    timed!(async {
        let (ctx, _dir) = fixture::make_test_ctx().await;
        let author = seed_user(&ctx, "ivanov", "Иванов И.И.", "employee").await;
        let who = ident(author, Role::Employee);
        let r = seed_request(&ctx, author, "free_form", "open").await;
        let ok = "я".repeat(4000);
        assert!(ctx
            .chat
            .send(&who, r, &ok, &[])
            .await
            .unwrap()
            .message
            .is_some());
        let long = "я".repeat(4001);
        let out = ctx.chat.send(&who, r, &long, &[]).await.unwrap();
        assert_eq!(out.reject_code.as_deref(), Some("too_long"));
        let out = ctx.chat.send(&who, r, "   \n ", &[]).await.unwrap();
        assert_eq!(out.reject_code.as_deref(), Some("empty"));
        let id = draft_file(&ctx, &who, r, "фото.png").await;
        let out = ctx.chat.send(&who, r, "", &[id]).await.unwrap();
        let m = out.message.expect("files only");
        assert_eq!(m.text, "");
        assert_eq!(m.attachments.len(), 1);
        assert_eq!(count(&ctx, "SELECT COUNT(*) FROM chat_messages").await, 2);
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn chat_send_no_chat_and_forbidden() {
    timed!(async {
        let (ctx, _dir) = fixture::make_test_ctx().await;
        let author = seed_user(&ctx, "ivanov", "Иванов И.И.", "employee").await;
        let stranger = seed_user(&ctx, "sidorov", "Сидоров С.С.", "employee").await;
        let admin = seed_user(&ctx, "petrov", "Петров П.П.", "admin").await;
        let ad = seed_request(&ctx, author, "ad_register", "open").await;
        let out = ctx
            .chat
            .send(&ident(admin, Role::Admin), ad, "текст", &[])
            .await
            .unwrap();
        assert_eq!(out.reject_code.as_deref(), Some("no_chat"));

        let r = seed_request(&ctx, author, "repair", "open").await;
        let err = ctx
            .chat
            .send(&ident(stranger, Role::Employee), r, "текст", &[])
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::Forbidden));
        let anon = Identity {
            user_id: None,
            role: Role::Employee,
        };
        let err = ctx.chat.send(&anon, r, "текст", &[]).await.unwrap_err();
        assert!(matches!(err, AppError::Forbidden));
        assert_eq!(count(&ctx, "SELECT COUNT(*) FROM chat_messages").await, 0);
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn chat_send_files_promoted_with_audit() {
    timed!(async {
        let (ctx, _dir) = fixture::make_test_ctx().await;
        let author = seed_user(&ctx, "ivanov", "Иванов И.И.", "employee").await;
        let who = ident(author, Role::Employee);
        let r = seed_request(&ctx, author, "repair", "open").await;
        let a = draft_file(&ctx, &who, r, "один.png").await;
        let b = draft_file(&ctx, &who, r, "два.png").await;
        let out = ctx
            .chat
            .send(&who, r, "Смотрите", &[a, b, a])
            .await
            .unwrap();
        let m = out.message.expect("message");
        assert_eq!(m.attachments.len(), 2, "ids deduplicated");
        assert!(m.attachments.iter().all(|x| x.from_message));
        assert_eq!(
            count(
                &ctx,
                "SELECT COUNT(*) FROM attachments WHERE parent_kind = 'request' \
                 AND message_id IS NOT NULL"
            )
            .await,
            2
        );
        assert_eq!(
            count(
                &ctx,
                "SELECT COUNT(*) FROM audit_log WHERE action = 'custom:attachment_added' \
                 AND payload_json LIKE '%\"message_id\"%'"
            )
            .await,
            2
        );
        let list = ctx
            .attachments
            .list(
                ParentRef {
                    kind: ParentKind::Request,
                    id: r,
                },
                &who,
            )
            .await
            .unwrap();
        assert_eq!(list.items.len(), 2, "visible in the request attachments");
        let feed = ctx.chat.feed(&who, r).await.unwrap();
        assert!(
            feed.items
                .iter()
                .all(|i| !matches!(i, ChatItemDto::AttachmentEvent(_))),
            "no attachment events for message files"
        );
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn chat_send_forged_draft_ids() {
    timed!(async {
        let (ctx, _dir) = fixture::make_test_ctx().await;
        let author = seed_user(&ctx, "ivanov", "Иванов И.И.", "employee").await;
        let other = seed_user(&ctx, "sidorov", "Сидоров С.С.", "manager").await;
        let who = ident(author, Role::Employee);
        let r = seed_request(&ctx, author, "repair", "open").await;
        let r2 = seed_request(&ctx, author, "repair", "open").await;
        let mine = draft_file(&ctx, &who, r, "мой.png").await;
        let foreign = draft_file(&ctx, &ident(other, Role::Manager), r, "чужой.png").await;
        let other_req = draft_file(&ctx, &who, r2, "другая.png").await;

        for bad in [foreign, other_req, 987_654] {
            let err = ctx
                .chat
                .send(&who, r, "текст", &[mine, bad])
                .await
                .unwrap_err();
            assert!(matches!(err, AppError::Forbidden), "{bad}");
        }
        assert_eq!(count(&ctx, "SELECT COUNT(*) FROM chat_messages").await, 0);
        assert_eq!(
            count(
                &ctx,
                "SELECT COUNT(*) FROM attachments WHERE parent_kind = 'message_draft'"
            )
            .await,
            3,
            "valid drafts stay drafts"
        );
        // A reused id: promoted once, then forged.
        ctx.chat.send(&who, r, "первый", &[mine]).await.unwrap();
        let err = ctx.chat.send(&who, r, "второй", &[mine]).await.unwrap_err();
        assert!(matches!(err, AppError::Forbidden), "reused id");
        assert_eq!(count(&ctx, "SELECT COUNT(*) FROM chat_messages").await, 1);
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn chat_send_ws_after_commit_without_text() {
    timed!(async {
        let (ctx, _dir) = fixture::make_test_ctx().await;
        let author = seed_user(&ctx, "ivanov", "Иванов И.И.", "employee").await;
        let who = ident(author, Role::Employee);
        let r = seed_request(&ctx, author, "repair", "open").await;
        let closed = seed_request(&ctx, author, "repair", "completed").await;
        let mut rx = ctx.ws_broadcast.subscribe();
        fixture::drain(&mut rx);

        // Reject: no events.
        ctx.chat
            .send(&who, closed, "секретный текст", &[])
            .await
            .unwrap();
        assert!(fixture::collect_events(&mut rx).is_empty());

        let id = draft_file(&ctx, &who, r, "фото.png").await;
        fixture::drain(&mut rx);
        let out = ctx
            .chat
            .send(&who, r, "секретный текст", &[id])
            .await
            .unwrap();
        let mid = out.message.unwrap().id;
        let events = fixture::collect_events(&mut rx);
        let chat = events
            .iter()
            .find_map(|e| match e {
                WsEvent::ChatMessage {
                    request_id,
                    message_id,
                    actor_user_id,
                    actor_name,
                    requested_by_user_id,
                    ..
                } => Some((
                    *request_id,
                    *message_id,
                    *actor_user_id,
                    actor_name.clone(),
                    *requested_by_user_id,
                )),
                _ => None,
            })
            .expect("ChatMessage event");
        assert_eq!(
            chat,
            (
                r,
                mid,
                Some(author),
                Some("Иванов И.И.".to_string()),
                Some(author)
            )
        );
        assert!(events.iter().any(|e| matches!(
            e,
            WsEvent::AttachmentsChanged { parent_kind, parent_id, .. }
                if parent_kind == "request" && *parent_id == r
        )));
        for e in &events {
            let json = serde_json::to_string(e).unwrap();
            assert!(!json.contains("секретный"), "text must not leak: {json}");
        }
    })
}
