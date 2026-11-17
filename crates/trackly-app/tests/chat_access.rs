//! Chat access (Phase 41.5, D-05/D-06): one check for feed, mark_read and the
//! unread summary. Names are fictional.

use std::time::Duration;

use trackly_core::auth::{Identity, Role};
use trackly_core::error::AppError;

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

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn chat_access_roles_matrix() {
    timed!(async {
        let (ctx, _dir) = fixture::make_test_ctx().await;
        let author = seed_user(&ctx, "ivanov", "Иванов И.И.", "employee").await;
        let member = seed_user(&ctx, "sidorova", "Сидорова А.А.", "employee").await;
        let mgr = seed_user(&ctx, "petrov", "Петров П.П.", "manager").await;
        let stranger = seed_user(&ctx, "kuznetsov", "Кузнецов К.К.", "employee").await;
        let r = seed_request(&ctx, author, "repair", "open").await;
        add_participant(&ctx, r, member).await;

        let allowed = [
            ident(author, Role::Employee),
            ident(member, Role::Employee),
            ident(mgr, Role::Manager),
            Identity::trusted_admin(),
        ];
        for who in &allowed {
            ctx.chat.feed(who, r).await.expect("feed allowed");
        }
        let denied = ident(stranger, Role::Employee);
        assert!(matches!(
            ctx.chat.feed(&denied, r).await,
            Err(AppError::Forbidden)
        ));
        assert!(matches!(
            ctx.chat.mark_read(&denied, r).await,
            Err(AppError::Forbidden)
        ));
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn chat_access_ad_register_has_no_chat() {
    timed!(async {
        let (ctx, _dir) = fixture::make_test_ctx().await;
        let author = seed_user(&ctx, "ivanov", "Иванов И.И.", "employee").await;
        let mgr = seed_user(&ctx, "petrov", "Петров П.П.", "manager").await;
        let r = seed_request(&ctx, author, "ad_register", "open").await;
        for who in [
            ident(author, Role::Employee),
            ident(mgr, Role::Manager),
            Identity::trusted_admin(),
        ] {
            assert!(matches!(
                ctx.chat.feed(&who, r).await,
                Err(AppError::Forbidden)
            ));
            assert!(matches!(
                ctx.chat.mark_read(&who, r).await,
                Err(AppError::Forbidden)
            ));
        }
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn chat_access_missing_deleted_and_no_identity() {
    timed!(async {
        let (ctx, _dir) = fixture::make_test_ctx().await;
        let author = seed_user(&ctx, "ivanov", "Иванов И.И.", "employee").await;
        let r = seed_request(&ctx, author, "free_form", "open").await;
        let admin = Identity::trusted_admin();

        assert!(matches!(
            ctx.chat.feed(&admin, 99_999).await,
            Err(AppError::NotFound { .. })
        ));
        // Employee without a user id gets nothing.
        let ghost = Identity {
            user_id: None,
            role: Role::Employee,
        };
        assert!(matches!(
            ctx.chat.feed(&ghost, r).await,
            Err(AppError::Forbidden)
        ));
        assert!(matches!(
            ctx.chat.mark_read(&ghost, r).await,
            Err(AppError::Forbidden)
        ));

        soft_delete_request(&ctx, r).await;
        assert!(matches!(
            ctx.chat.feed(&admin, r).await,
            Err(AppError::NotFound { .. })
        ));
        assert!(matches!(
            ctx.chat.feed(&ident(author, Role::Employee), r).await,
            Err(AppError::NotFound { .. })
        ));
    })
}
