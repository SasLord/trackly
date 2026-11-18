//! Per-user chat notification mute (Phase 41.5 plan 22, D-G2, D-G3, D-06).
//! Names are fictional.

use std::time::Duration;

use trackly_app::dto::printer::WsEvent;
use trackly_core::auth::{Identity, Role};
use trackly_core::error::AppError;

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

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn chat_mute_allowed_roles_idempotent_and_closed_request() {
    timed!(async {
        let (ctx, _dir) = fixture::make_test_ctx().await;
        let author = seed_user(&ctx, "ivanov", "Иванов И.И.", "employee").await;
        let member = seed_user(&ctx, "sidorova", "Сидорова А.А.", "employee").await;
        let mgr = seed_user(&ctx, "petrov", "Петров П.П.", "manager").await;
        let adm = seed_user(&ctx, "admin1", "Админов А.А.", "admin").await;
        let r = seed_request(&ctx, author, "repair", "open").await;
        add_participant(&ctx, r, member).await;
        let r_closed = seed_request(&ctx, author, "repair", "completed").await;

        for (id, role) in [
            (author, Role::Employee),
            (member, Role::Employee),
            (mgr, Role::Manager),
            (adm, Role::Admin),
        ] {
            let who = ident(id, role);
            ctx.chat.set_muted(&who, r, true).await.unwrap();
            ctx.chat.set_muted(&who, r, true).await.unwrap(); // idempotent
            assert_eq!(
                ctx.chat
                    .unread_summary(&who)
                    .await
                    .unwrap()
                    .muted_request_ids,
                vec![r]
            );
            ctx.chat.set_muted(&who, r, false).await.unwrap();
            ctx.chat.set_muted(&who, r, false).await.unwrap();
            assert!(ctx
                .chat
                .unread_summary(&who)
                .await
                .unwrap()
                .muted_request_ids
                .is_empty());
        }
        let a = ident(author, Role::Employee);
        ctx.chat.set_muted(&a, r_closed, true).await.unwrap();
        assert_eq!(
            ctx.chat.unread_summary(&a).await.unwrap().muted_request_ids,
            vec![r_closed]
        );
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn chat_mute_forbidden_cases() {
    timed!(async {
        let (ctx, _dir) = fixture::make_test_ctx().await;
        let author = seed_user(&ctx, "ivanov", "Иванов И.И.", "employee").await;
        let stranger = seed_user(&ctx, "kuznetsov", "Кузнецов К.К.", "employee").await;
        let adm = seed_user(&ctx, "admin1", "Админов А.А.", "admin").await;
        let r = seed_request(&ctx, author, "repair", "open").await;
        let r_ad = seed_request(&ctx, author, "ad_register", "open").await;
        let mut rx = ctx.ws_broadcast.subscribe();
        fixture::drain(&mut rx);

        assert!(matches!(
            ctx.chat
                .set_muted(&ident(stranger, Role::Employee), r, true)
                .await,
            Err(AppError::Forbidden)
        ));
        for (id, role) in [(author, Role::Employee), (adm, Role::Admin)] {
            assert!(matches!(
                ctx.chat.set_muted(&ident(id, role), r_ad, true).await,
                Err(AppError::Forbidden)
            ));
        }
        assert!(matches!(
            ctx.chat
                .set_muted(&ident(author, Role::Employee), 9999, true)
                .await,
            Err(AppError::NotFound { .. })
        ));
        let no_user = Identity {
            user_id: None,
            role: Role::Admin,
        };
        assert!(matches!(
            ctx.chat.set_muted(&no_user, r, true).await,
            Err(AppError::Forbidden)
        ));
        assert!(fixture::collect_events(&mut rx)
            .iter()
            .all(|e| !matches!(e, WsEvent::ChatReadChanged { .. })));
        let s = ctx
            .chat
            .unread_summary(&ident(stranger, Role::Employee))
            .await
            .unwrap();
        assert!(s.muted_request_ids.is_empty());
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn chat_mute_isolated_and_counters_keep_growing() {
    timed!(async {
        let (ctx, _dir) = fixture::make_test_ctx().await;
        let author = seed_user(&ctx, "ivanov", "Иванов И.И.", "employee").await;
        let mgr = seed_user(&ctx, "petrov", "Петров П.П.", "manager").await;
        let r = seed_request(&ctx, author, "repair", "open").await;
        let a = ident(author, Role::Employee);
        let m = ident(mgr, Role::Manager);

        ctx.chat.set_muted(&a, r, true).await.unwrap();
        message(&ctx, r, Some(mgr), "Добрый день", 1_700_000_100, None).await;
        message(&ctx, r, Some(mgr), "Ещё раз", 1_700_000_110, None).await;

        // D-G2: the counter still grows while muted.
        let s = ctx.chat.unread_summary(&a).await.unwrap();
        assert_eq!(s.muted_request_ids, vec![r]);
        assert_eq!(s.requests_with_unread, 1);
        assert_eq!(s.items[0].request_id, r);
        assert_eq!(s.items[0].count, 2);
        // Isolation: the other user is unaffected.
        assert!(ctx
            .chat
            .unread_summary(&m)
            .await
            .unwrap()
            .muted_request_ids
            .is_empty());
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn chat_mute_ws_once_to_owner_after_commit() {
    timed!(async {
        let (ctx, _dir) = fixture::make_test_ctx().await;
        let author = seed_user(&ctx, "ivanov", "Иванов И.И.", "employee").await;
        let mgr = seed_user(&ctx, "petrov", "Петров П.П.", "manager").await;
        let r = seed_request(&ctx, author, "repair", "open").await;
        let a = ident(author, Role::Employee);
        let mut rx = ctx.ws_broadcast.subscribe();
        fixture::drain(&mut rx);

        ctx.chat.set_muted(&a, r, true).await.unwrap();
        let events = fixture::collect_events(&mut rx);
        let evs: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, WsEvent::ChatReadChanged { .. }))
            .collect();
        assert_eq!(evs.len(), 1);
        assert!(matches!(
            evs[0],
            WsEvent::ChatReadChanged { user_id } if *user_id == author
        ));
        assert!(evs[0].is_visible_to(&a));
        assert!(!evs[0].is_visible_to(&ident(mgr, Role::Manager)));
        assert!(!evs[0].is_visible_to(&Identity::trusted_admin()));
        // The row is already visible to a reader when the event arrives.
        assert_eq!(
            ctx.chat.unread_summary(&a).await.unwrap().muted_request_ids,
            vec![r]
        );
    })
}
