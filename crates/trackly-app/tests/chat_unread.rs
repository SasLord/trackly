//! Chat read marks and the unread summary (Phase 41.5, D-17..D-20).
//! Names are fictional.

use std::time::Duration;

use trackly_app::dto::chat::UnreadSummaryDto;
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

fn count_of(s: &UnreadSummaryDto, request_id: i64) -> i32 {
    s.items
        .iter()
        .find(|i| i.request_id == request_id)
        .map_or(0, |i| i.count)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn chat_unread_counts_and_mark_read() {
    timed!(async {
        let (ctx, _dir) = fixture::make_test_ctx().await;
        let author = seed_user(&ctx, "ivanov", "Иванов И.И.", "employee").await;
        let mgr = seed_user(&ctx, "petrov", "Петров П.П.", "manager").await;
        let r = seed_request(&ctx, author, "repair", "open").await;
        let a = ident(author, Role::Employee);
        let m = ident(mgr, Role::Manager);

        message(&ctx, r, Some(mgr), "Добрый день", 1_700_000_100, None).await;
        assert_eq!(count_of(&ctx.chat.unread_summary(&a).await.unwrap(), r), 1);
        // Own message does not count for its author.
        assert_eq!(count_of(&ctx.chat.unread_summary(&m).await.unwrap(), r), 0);

        // A foreign action counts, an own one does not.
        audit(&ctx, r, "accept", Some(mgr), None, 1_700_000_110).await;
        audit(&ctx, r, "update", Some(author), None, 1_700_000_120).await;
        assert_eq!(count_of(&ctx.chat.unread_summary(&a).await.unwrap(), r), 2);
        assert_eq!(count_of(&ctx.chat.unread_summary(&m).await.unwrap(), r), 1);

        ctx.chat.mark_read(&a, r).await.unwrap();
        ctx.chat.mark_read(&a, r).await.unwrap(); // idempotent
        let s = ctx.chat.unread_summary(&a).await.unwrap();
        assert_eq!(count_of(&s, r), 0);
        assert_eq!(s.requests_with_unread, 0);
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn chat_unread_scope_by_role() {
    timed!(async {
        let (ctx, _dir) = fixture::make_test_ctx().await;
        let author = seed_user(&ctx, "ivanov", "Иванов И.И.", "employee").await;
        let member = seed_user(&ctx, "sidorova", "Сидорова А.А.", "employee").await;
        let stranger = seed_user(&ctx, "kuznetsov", "Кузнецов К.К.", "employee").await;
        let mgr = seed_user(&ctx, "petrov", "Петров П.П.", "manager").await;
        let r1 = seed_request(&ctx, author, "repair", "open").await;
        add_participant(&ctx, r1, member).await;
        let r2 = seed_request(&ctx, stranger, "free_form", "open").await;
        let r_ad = seed_request(&ctx, author, "ad_register", "open").await;
        let r_del = seed_request(&ctx, author, "repair", "open").await;
        for r in [r1, r1, r2, r_ad, r_del] {
            message(&ctx, r, Some(mgr), "текст", 1_700_000_100, None).await;
        }
        soft_delete_request(&ctx, r_del).await;

        let author_s = ctx
            .chat
            .unread_summary(&ident(author, Role::Employee))
            .await
            .unwrap();
        assert_eq!(count_of(&author_s, r1), 2);
        assert_eq!(count_of(&author_s, r2), 0);
        assert_eq!(count_of(&author_s, r_ad), 0);
        assert_eq!(count_of(&author_s, r_del), 0);
        assert_eq!(author_s.requests_with_unread, 1, "requests, not messages");

        let member_s = ctx
            .chat
            .unread_summary(&ident(member, Role::Employee))
            .await
            .unwrap();
        assert_eq!(count_of(&member_s, r1), 2);

        // Staff (non-author manager reader here is the author of the messages,
        // so read as another manager).
        let mgr2 = seed_user(&ctx, "orlov", "Орлов О.О.", "manager").await;
        let staff = ctx
            .chat
            .unread_summary(&ident(mgr2, Role::Manager))
            .await
            .unwrap();
        assert_eq!(count_of(&staff, r1), 2);
        assert_eq!(count_of(&staff, r2), 1);
        assert_eq!(count_of(&staff, r_ad), 0);
        assert_eq!(count_of(&staff, r_del), 0);
        assert_eq!(staff.requests_with_unread, 2);
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn chat_unread_message_file_not_double_counted() {
    timed!(async {
        let (ctx, _dir) = fixture::make_test_ctx().await;
        let author = seed_user(&ctx, "ivanov", "Иванов И.И.", "employee").await;
        let mgr = seed_user(&ctx, "petrov", "Петров П.П.", "manager").await;
        let r = seed_request(&ctx, author, "repair", "open").await;
        let a = ident(author, Role::Employee);

        let mid = message(&ctx, r, Some(mgr), "Фото", 1_700_000_100, None).await;
        audit(
            &ctx,
            r,
            "custom:attachment_added",
            Some(mgr),
            Some(&format!(
                "{{\"attachment_id\":1,\"name\":\"фото.png\",\"message_id\":{mid}}}"
            )),
            1_700_000_100,
        )
        .await;
        assert_eq!(count_of(&ctx.chat.unread_summary(&a).await.unwrap(), r), 1);

        // A file added through the "Информация" tab is one more event.
        let up = upload(
            &ctx,
            &ident(mgr, Role::Manager),
            ParentRef {
                kind: ParentKind::Request,
                id: r,
            },
            "скан.png",
            &attachments_support::tiny_png(),
        )
        .await
        .unwrap();
        assert!(up.attachment.is_some());
        assert_eq!(count_of(&ctx.chat.unread_summary(&a).await.unwrap(), r), 2);
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn chat_unread_baseline_watermark() {
    timed!(async {
        let (ctx, _dir) = fixture::make_test_ctx().await;
        let author = seed_user(&ctx, "ivanov", "Иванов И.И.", "employee").await;
        let mgr = seed_user(&ctx, "petrov", "Петров П.П.", "manager").await;
        let r = seed_request(&ctx, author, "repair", "open").await;
        let a = ident(author, Role::Employee);

        let old = audit(&ctx, r, "accept", Some(mgr), None, 1_700_000_100).await;
        ctx.writer
            .execute(move |conn| {
                conn.execute(
                    "UPDATE app_settings SET value = ?1 WHERE key = 'chat_unread_baseline_audit_id'",
                    rusqlite::params![old.to_string()],
                )
                .map_err(map_rusqlite)?;
                Ok(())
            })
            .await
            .unwrap();
        assert_eq!(
            count_of(&ctx.chat.unread_summary(&a).await.unwrap(), r),
            0,
            "events up to the watermark are not unread"
        );
        audit(&ctx, r, "reject", Some(mgr), None, 1_700_000_200).await;
        assert_eq!(count_of(&ctx.chat.unread_summary(&a).await.unwrap(), r), 1);
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn chat_unread_mark_read_ws_after_commit_to_owner() {
    timed!(async {
        let (ctx, _dir) = fixture::make_test_ctx().await;
        let author = seed_user(&ctx, "ivanov", "Иванов И.И.", "employee").await;
        let mgr = seed_user(&ctx, "petrov", "Петров П.П.", "manager").await;
        let r = seed_request(&ctx, author, "repair", "open").await;
        message(&ctx, r, Some(mgr), "Привет", 1_700_000_100, None).await;
        let a = ident(author, Role::Employee);
        let mut rx = ctx.ws_broadcast.subscribe();
        fixture::drain(&mut rx);

        ctx.chat.mark_read(&a, r).await.unwrap();
        let events = fixture::collect_events(&mut rx);
        let ev = events
            .iter()
            .find(|e| matches!(e, WsEvent::ChatReadChanged { .. }))
            .expect("ChatReadChanged sent");
        assert!(ev.is_visible_to(&a));
        assert!(!ev.is_visible_to(&ident(mgr, Role::Manager)));
        assert!(!ev.is_visible_to(&Identity::trusted_admin()));
        // The mark is already stored when the event arrives.
        assert_eq!(count_of(&ctx.chat.unread_summary(&a).await.unwrap(), r), 0);
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn chat_unread_mark_read_forbidden_cases_and_no_identity() {
    timed!(async {
        let (ctx, _dir) = fixture::make_test_ctx().await;
        let author = seed_user(&ctx, "ivanov", "Иванов И.И.", "employee").await;
        let stranger = seed_user(&ctx, "kuznetsov", "Кузнецов К.К.", "employee").await;
        let r = seed_request(&ctx, author, "repair", "open").await;
        let r_ad = seed_request(&ctx, author, "ad_register", "open").await;
        let mut rx = ctx.ws_broadcast.subscribe();
        fixture::drain(&mut rx);

        assert!(matches!(
            ctx.chat
                .mark_read(&ident(stranger, Role::Employee), r)
                .await,
            Err(AppError::Forbidden)
        ));
        assert!(matches!(
            ctx.chat
                .mark_read(&ident(author, Role::Employee), r_ad)
                .await,
            Err(AppError::Forbidden)
        ));
        assert!(fixture::collect_events(&mut rx)
            .iter()
            .all(|e| !matches!(e, WsEvent::ChatReadChanged { .. })));

        // Trusted desktop admin: empty summary, no mark.
        let admin = Identity::trusted_admin();
        let s = ctx.chat.unread_summary(&admin).await.unwrap();
        assert!(s.items.is_empty());
        assert_eq!(s.requests_with_unread, 0);
        assert!(matches!(
            ctx.chat.mark_read(&admin, r).await,
            Err(AppError::Forbidden)
        ));
    })
}
