//! Chat drafts cleanup (Phase 41.5, D-14): discard, stale purge, shared blobs.
//! Names are fictional.

use std::time::Duration;

use rusqlite::params;
use trackly_app::context::AppCtx;
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

async fn draft_file(ctx: &AppCtx, who: &Identity, r: i64, name: &str, bytes: &[u8]) -> i64 {
    upload(ctx, who, draft(r), name, bytes)
        .await
        .expect("upload")
        .attachment
        .expect("attachment")
        .id
}

async fn rows_with_id(ctx: &AppCtx, id: i64) -> i64 {
    ctx.writer
        .execute(move |conn| {
            conn.query_row(
                "SELECT COUNT(*) FROM attachments WHERE id = ?1",
                params![id],
                |r| r.get(0),
            )
            .map_err(map_rusqlite)
        })
        .await
        .expect("count")
}

async fn hash_of(ctx: &AppCtx, id: i64) -> String {
    ctx.writer
        .execute(move |conn| {
            conn.query_row(
                "SELECT content_hash FROM attachments WHERE id = ?1",
                params![id],
                |r| r.get(0),
            )
            .map_err(map_rusqlite)
        })
        .await
        .expect("hash")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn chat_drafts_discard_scoped_to_caller_and_request() {
    timed!(async {
        let (ctx, _dir) = fixture::make_test_ctx().await;
        let author = seed_user(&ctx, "ivanov", "Иванов И.И.", "employee").await;
        let mgr = seed_user(&ctx, "petrov", "Петров П.П.", "manager").await;
        let who = ident(author, Role::Employee);
        let r = seed_request(&ctx, author, "repair", "open").await;
        let r2 = seed_request(&ctx, author, "repair", "open").await;
        let png = attachments_support::tiny_png();
        let mine = draft_file(&ctx, &who, r, "мой.png", &png).await;
        draft_file(&ctx, &ident(mgr, Role::Manager), r, "чужой.png", &png).await;
        draft_file(&ctx, &who, r2, "другая.png", &png).await;
        let before = count(&ctx, "SELECT COUNT(*) FROM audit_log").await;
        let mut rx = ctx.ws_broadcast.subscribe();
        fixture::drain(&mut rx);

        ctx.chat.discard_drafts(&who, r).await.unwrap();
        assert_eq!(
            count(
                &ctx,
                "SELECT COUNT(*) FROM attachments WHERE parent_kind = 'message_draft'"
            )
            .await,
            2
        );
        assert_eq!(rows_with_id(&ctx, mine).await, 0);
        // Idempotent.
        ctx.chat.discard_drafts(&who, r).await.unwrap();
        assert_eq!(count(&ctx, "SELECT COUNT(*) FROM audit_log").await, before);
        assert!(fixture::collect_events(&mut rx).is_empty());
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn chat_drafts_blob_kept_while_referenced() {
    timed!(async {
        let (ctx, _dir) = fixture::make_test_ctx().await;
        let author = seed_user(&ctx, "ivanov", "Иванов И.И.", "employee").await;
        let who = ident(author, Role::Employee);
        let r = seed_request(&ctx, author, "repair", "open").await;
        let r2 = seed_request(&ctx, author, "repair", "open").await;
        let png = attachments_support::tiny_png();
        let d1 = draft_file(&ctx, &who, r, "один.png", &png).await;
        let hash = hash_of(&ctx, d1).await;
        // Same bytes on a different parent keep the blob alive.
        let d2 = draft_file(&ctx, &who, r2, "два.png", &png).await;
        assert_eq!(hash_of(&ctx, d2).await, hash);
        let store = ctx.attachments.store().clone();
        assert!(store.blob_exists(&hash));

        ctx.chat.discard_drafts(&who, r).await.unwrap();
        assert!(store.blob_exists(&hash), "still referenced");
        ctx.chat.discard_drafts(&who, r2).await.unwrap();
        assert!(!store.blob_exists(&hash), "last reference gone");
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn chat_drafts_discard_access_rules() {
    timed!(async {
        let (ctx, _dir) = fixture::make_test_ctx().await;
        let author = seed_user(&ctx, "ivanov", "Иванов И.И.", "employee").await;
        let stranger = seed_user(&ctx, "sidorov", "Сидоров С.С.", "employee").await;
        let who = ident(author, Role::Employee);
        let r = seed_request(&ctx, author, "repair", "open").await;
        draft_file(&ctx, &who, r, "мой.png", &attachments_support::tiny_png()).await;
        let err = ctx
            .chat
            .discard_drafts(&ident(stranger, Role::Employee), r)
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::Forbidden));
        // A closed request can still be cleaned up by a participant.
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
        ctx.chat.discard_drafts(&who, r).await.unwrap();
        assert_eq!(
            count(
                &ctx,
                "SELECT COUNT(*) FROM attachments WHERE parent_kind = 'message_draft'"
            )
            .await,
            0
        );
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn chat_drafts_purge_stale_only_old_drafts() {
    timed!(async {
        let (ctx, _dir) = fixture::make_test_ctx().await;
        let author = seed_user(&ctx, "ivanov", "Иванов И.И.", "employee").await;
        let who = ident(author, Role::Employee);
        let r = seed_request(&ctx, author, "repair", "open").await;
        let png = attachments_support::tiny_png();
        let old = draft_file(&ctx, &who, r, "старый.png", &png).await;
        let fresh = draft_file(
            &ctx,
            &who,
            r,
            "свежий.png",
            &attachments_support::tiny_jpeg(),
        )
        .await;
        let sent = draft_file(&ctx, &who, r, "ушёл.png", &attachments_support::tiny_gif()).await;
        ctx.chat.send(&who, r, "текст", &[sent]).await.unwrap();
        // Age the first draft and the promoted file far into the past.
        ctx.writer
            .execute(move |conn| {
                conn.execute(
                    "UPDATE attachments SET created_at_utc = 1 WHERE id IN (?1, ?2)",
                    params![old, sent],
                )
                .map_err(map_rusqlite)?;
                Ok(())
            })
            .await
            .unwrap();

        let n = ctx.chat.purge_stale_drafts(24 * 3600).await.unwrap();
        assert_eq!(n, 1);
        for (id, want) in [(old, 0), (fresh, 1), (sent, 1)] {
            assert_eq!(rows_with_id(&ctx, id).await, want, "id {id}");
        }
    })
}

/// Nyquist 41.5 G2 (D-14): очистка устаревших черновиков действительно
/// запускается при старте (`AppCtx::build`), а не только существует как метод.
/// Ожидание — опрос с крайним сроком, без фиксированных пауз.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn chat_drafts_purged_on_startup() {
    timed!(async {
        let (ctx, dir) = fixture::make_test_ctx().await;
        let author = seed_user(&ctx, "ivanov", "Иванов И.И.", "employee").await;
        let who = ident(author, Role::Employee);
        let r = seed_request(&ctx, author, "repair", "open").await;
        let old = draft_file(
            &ctx,
            &who,
            r,
            "старый.png",
            &attachments_support::tiny_png(),
        )
        .await;
        let fresh = draft_file(
            &ctx,
            &who,
            r,
            "свежий.jpg",
            &attachments_support::tiny_jpeg(),
        )
        .await;
        ctx.writer
            .execute(move |conn| {
                conn.execute(
                    "UPDATE attachments SET created_at_utc = 1 WHERE id = ?1",
                    params![old],
                )
                .map_err(map_rusqlite)?;
                Ok(())
            })
            .await
            .unwrap();
        drop(ctx);

        // Second start on the same portable directory.
        let paths = trackly_infra::Paths::resolve_for_exe_dir(dir.path().to_path_buf())
            .expect("resolve paths");
        let config = trackly_infra::AppConfig::default();
        let (_nb, guard) = tracing_appender::non_blocking(std::io::sink());
        let ctx = AppCtx::build(paths, config, guard)
            .await
            .expect("second build");

        let deadline = tokio::time::Instant::now() + Duration::from_secs(20);
        loop {
            if rows_with_id(&ctx, old).await == 0 {
                break;
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "stale draft {old} was not purged after start"
            );
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        assert_eq!(rows_with_id(&ctx, fresh).await, 1, "fresh draft must stay");
    })
}
