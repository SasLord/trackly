//! Upload pipeline of the attachment service (Phase 41.3 Plan 07):
//! staging limit, sniffing, dedup, audit, event, concurrency.
//! Names are fictional.

use std::time::Duration;

use rusqlite::params;
use trackly_app::context::AppCtx;
use trackly_app::dto::printer::WsEvent;
use trackly_app::services::attachment_service::{ParentRef, StageError};
use trackly_core::auth::{Identity, Role};
use trackly_core::domain::attachments::{ParentKind, RejectCode};
use trackly_core::error::AppError;
use trackly_infra::error_conversions::map_rusqlite;

#[path = "entities_support/fixture.rs"]
#[allow(dead_code)]
mod fixture;

mod attachments_support;
use attachments_support as fx;

fn ident(id: i64, role: Role) -> Identity {
    Identity {
        user_id: Some(id),
        role,
    }
}

macro_rules! timed {
    ($body:expr) => {
        tokio::time::timeout(Duration::from_secs(120), $body)
            .await
            .expect("timeout")
    };
}

async fn seed_user(ctx: &AppCtx, login: &str, full: &str, role: &str) -> i64 {
    let (login, full, role) = (login.to_string(), full.to_string(), role.to_string());
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO users (login, full_name, role, created_at_utc, updated_at_utc) \
                 VALUES (?1, ?2, ?3, 1700000000, 1700000000)",
                params![login, full, role],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed user")
}

async fn seed_request(ctx: &AppCtx, author: i64) -> i64 {
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO requests (request_type, status, requested_by_user_id, \
                 created_at_utc, updated_at_utc) VALUES ('free_form', 'completed', ?1, 1700000000, 1700000000)",
                params![author],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed request")
}

async fn set_setting(ctx: &AppCtx, value: &str) {
    let v = value.to_string();
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO app_settings (key, value, created_at_utc, updated_at_utc) \
                 VALUES ('attachment_max_bytes', ?1, 1700000000, 1700000000) \
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![v],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("set setting");
}

fn req(id: i64) -> ParentRef {
    ParentRef {
        kind: ParentKind::Request,
        id,
    }
}

/// Stages `bytes` through the public pipeline and ingests them.
async fn upload(
    ctx: &AppCtx,
    who: &Identity,
    parent: ParentRef,
    name: &str,
    bytes: &[u8],
) -> Result<trackly_app::dto::attachment::AttachmentUploadDto, AppError> {
    let svc = &ctx.attachments;
    svc.authorize_attach(who, parent).await?;
    let limit = svc.effective_limit().await?;
    let mut staged = svc.begin_staging()?;
    if let Err(e) = staged.write_chunk(bytes, limit) {
        return match e {
            StageError::Reject(c) => Ok(trackly_app::dto::attachment::AttachmentUploadDto {
                attachment: None,
                reject_code: Some(c.as_code().to_string()),
            }),
            StageError::Internal(e) => Err(e),
        };
    }
    svc.ingest(who, parent, name, staged).await
}

fn count_files(dir: &std::path::Path) -> usize {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return 0;
    };
    rd.flatten()
        .map(|e| {
            let p = e.path();
            if p.is_dir() {
                count_files(&p)
            } else {
                1
            }
        })
        .sum()
}

async fn row_count(ctx: &AppCtx) -> i64 {
    ctx.writer
        .execute(|conn| {
            conn.query_row("SELECT COUNT(*) FROM attachments", [], |r| r.get(0))
                .map_err(map_rusqlite)
        })
        .await
        .expect("count")
}

fn blobs_dir(ctx: &AppCtx) -> std::path::PathBuf {
    ctx.attachments.store().root().to_path_buf()
}

fn tmp_dir(ctx: &AppCtx) -> std::path::PathBuf {
    blobs_dir(ctx).join(".tmp")
}

fn sha_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn attachments_store_dedup() {
    timed!(async {
        let (ctx, _dir) = fixture::make_test_ctx().await;
        let author = seed_user(&ctx, "ivanov", "Иванов И.И.", "employee").await;
        let r1 = seed_request(&ctx, author).await;
        let r2 = seed_request(&ctx, author).await;
        let admin = Identity::trusted_admin();
        let bytes = fx::tiny_pdf();
        let a = upload(&ctx, &admin, req(r1), "акт.pdf", &bytes)
            .await
            .unwrap();
        let b = upload(&ctx, &admin, req(r2), "копия.pdf", &bytes)
            .await
            .unwrap();
        let (a, b) = (a.attachment.expect("a"), b.attachment.expect("b"));
        assert_ne!(a.id, b.id);
        assert_eq!(a.original_name, "акт.pdf");
        assert_eq!(a.mime, "application/pdf");
        assert_eq!(a.size_bytes, bytes.len() as i64);
        let h = sha_hex(&bytes);
        let blob = ctx.attachments.store().blob_path(&h).unwrap();
        assert!(blob.is_file());
        assert_eq!(std::fs::read(&blob).unwrap(), bytes);
        assert_eq!(row_count(&ctx).await, 2);
        // one blob in the shards (thumbs/ and .tmp/ hold no blobs for a PDF)
        let shard = blob.parent().unwrap();
        assert_eq!(count_files(shard), 1);
        assert_eq!(count_files(&tmp_dir(&ctx)), 0);
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn attachments_limits() {
    timed!(async {
        let (ctx, _dir) = fixture::make_test_ctx().await;
        let author = seed_user(&ctx, "ivanov", "Иванов И.И.", "employee").await;
        let r = seed_request(&ctx, author).await;
        let admin = Identity::trusted_admin();

        set_setting(&ctx, "100").await;
        let mut big = fx::tiny_pdf();
        big.resize(101, b' ');
        let out = upload(&ctx, &admin, req(r), "a.pdf", &big).await.unwrap();
        assert_eq!(out.reject_code.as_deref(), Some("too_large"));
        assert_eq!(row_count(&ctx).await, 0);
        assert_eq!(count_files(&blobs_dir(&ctx)), 0);

        let out = upload(&ctx, &admin, req(r), "a.pdf", b"").await.unwrap();
        assert_eq!(out.reject_code.as_deref(), Some("empty"));

        // The ceiling beats an admin-set value of 101 MiB.
        set_setting(&ctx, &(101i64 * 1024 * 1024).to_string()).await;
        assert_eq!(
            ctx.attachments.effective_limit().await.unwrap(),
            100 * 1024 * 1024
        );
        assert_eq!(row_count(&ctx).await, 0);
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn attachments_sniff_wins_over_name() {
    timed!(async {
        let (ctx, _dir) = fixture::make_test_ctx().await;
        let author = seed_user(&ctx, "ivanov", "Иванов И.И.", "employee").await;
        let r = seed_request(&ctx, author).await;
        let admin = Identity::trusted_admin();
        let out = upload(&ctx, &admin, req(r), "скан.pdf", &fx::tiny_png())
            .await
            .unwrap();
        assert_eq!(out.attachment.expect("stored").mime, "image/png");
        let out = upload(&ctx, &admin, req(r), "x.pdf", &fx::pe_stub())
            .await
            .unwrap();
        assert_eq!(out.reject_code.as_deref(), Some("type_not_allowed"));
        assert_eq!(row_count(&ctx).await, 1);
    })
}

#[cfg(not(feature = "heic"))]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn attachments_heic_without_feature_rejected() {
    timed!(async {
        let (ctx, _dir) = fixture::make_test_ctx().await;
        let author = seed_user(&ctx, "ivanov", "Иванов И.И.", "employee").await;
        let r = seed_request(&ctx, author).await;
        let out = upload(
            &ctx,
            &Identity::trusted_admin(),
            req(r),
            "фото.heic",
            &fx::heic_stub(),
        )
        .await
        .unwrap();
        assert_eq!(out.reject_code.as_deref(), Some("heic_failed"));
        assert_eq!(row_count(&ctx).await, 0);
        assert_eq!(count_files(&tmp_dir(&ctx)), 0);
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn attachments_batch_isolated() {
    timed!(async {
        let (ctx, _dir) = fixture::make_test_ctx().await;
        let author = seed_user(&ctx, "ivanov", "Иванов И.И.", "employee").await;
        let r = seed_request(&ctx, author).await;
        let admin = Identity::trusted_admin();
        let files: Vec<(&str, Vec<u8>)> = vec![
            ("a.png", fx::tiny_png()),
            ("bad.exe", fx::pe_stub()),
            ("c.pdf", fx::tiny_pdf()),
        ];
        let mut ok = 0;
        let mut bad = 0;
        for (n, b) in &files {
            let out = upload(&ctx, &admin, req(r), n, b).await.unwrap();
            if out.attachment.is_some() {
                ok += 1;
            } else {
                assert_eq!(out.reject_code.as_deref(), Some("type_not_allowed"));
                bad += 1;
            }
        }
        assert_eq!((ok, bad), (2, 1));
        assert_eq!(row_count(&ctx).await, 2);
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn attachments_audit_and_event() {
    timed!(async {
        let (ctx, _dir) = fixture::make_test_ctx().await;
        let author = seed_user(&ctx, "ivanov", "Иванов И.И.", "employee").await;
        let r = seed_request(&ctx, author).await;
        let mut rx = ctx.ws_broadcast.subscribe();
        fixture::drain(&mut rx);

        // Rejected upload: no event.
        let emp = ident(author, Role::Employee);
        let out = upload(&ctx, &emp, req(r), "x.exe", &fx::pe_stub())
            .await
            .unwrap();
        assert!(out.reject_code.is_some());
        assert!(fixture::collect_events(&mut rx)
            .iter()
            .all(|e| !matches!(e, WsEvent::AttachmentsChanged { .. })));

        // Employee-author uploads to own (completed) request.
        let out = upload(&ctx, &emp, req(r), "Акт.pdf", &fx::tiny_pdf())
            .await
            .unwrap();
        let id = out.attachment.expect("stored").id;
        let events = fixture::collect_events(&mut rx);
        assert!(events.iter().any(|e| matches!(
            e,
            WsEvent::AttachmentsChanged { parent_kind, parent_id, requested_by_user_id, .. }
                if parent_kind == "request" && *parent_id == r && *requested_by_user_id == Some(author)
        )));

        let (etype, eid, payload): (String, i64, String) = ctx
            .writer
            .execute(|conn| {
                conn.query_row(
                    "SELECT entity_type, entity_id, payload_json FROM audit_log \
                     WHERE action = 'custom:attachment_added'",
                    [],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
                )
                .map_err(map_rusqlite)
            })
            .await
            .unwrap();
        assert_eq!((etype.as_str(), eid), ("request", r));
        assert!(payload.contains(&format!("\"attachment_id\":{id}")));
        assert!(payload.contains("Акт.pdf"));
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn attachments_forbidden_before_staging() {
    timed!(async {
        let (ctx, _dir) = fixture::make_test_ctx().await;
        let author = seed_user(&ctx, "ivanov", "Иванов И.И.", "employee").await;
        let other = seed_user(&ctx, "petrov", "Петров П.П.", "employee").await;
        let r = seed_request(&ctx, author).await;
        let stranger = ident(other, Role::Employee);
        let err = upload(&ctx, &stranger, req(r), "a.pdf", &fx::tiny_pdf())
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::Forbidden));
        assert_eq!(count_files(&tmp_dir(&ctx)), 0);
        assert_eq!(row_count(&ctx).await, 0);
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn attachments_concurrent_same_bytes() {
    timed!(async {
        let (ctx, _dir) = fixture::make_test_ctx().await;
        let author = seed_user(&ctx, "ivanov", "Иванов И.И.", "employee").await;
        let mut reqs = Vec::new();
        for _ in 0..8 {
            reqs.push(seed_request(&ctx, author).await);
        }
        let admin = Identity::trusted_admin();
        let bytes = fx::tiny_pdf();
        let u = |i: usize| upload(&ctx, &admin, req(reqs[i]), "a.pdf", &bytes);
        let results = tokio::join!(u(0), u(1), u(2), u(3), u(4), u(5), u(6), u(7));
        for r in [
            results.0, results.1, results.2, results.3, results.4, results.5, results.6, results.7,
        ] {
            assert!(r.unwrap().attachment.is_some());
        }
        assert_eq!(row_count(&ctx).await, 8);
        let h = sha_hex(&bytes);
        let blob = ctx.attachments.store().blob_path(&h).unwrap();
        assert_eq!(count_files(blob.parent().unwrap()), 1);
        assert_eq!(count_files(&tmp_dir(&ctx)), 0);
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn attachments_rejected_leave_tmp_empty() {
    timed!(async {
        let (ctx, _dir) = fixture::make_test_ctx().await;
        let author = seed_user(&ctx, "ivanov", "Иванов И.И.", "employee").await;
        let r = seed_request(&ctx, author).await;
        let admin = Identity::trusted_admin();
        let _ = upload(&ctx, &admin, req(r), "a.exe", &fx::pe_stub())
            .await
            .unwrap();
        let _ = upload(&ctx, &admin, req(r), "a.pdf", b"").await.unwrap();
        assert_eq!(count_files(&tmp_dir(&ctx)), 0);
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn attachments_sanitize_name_and_path() {
    timed!(async {
        let (ctx, _dir) = fixture::make_test_ctx().await;
        let author = seed_user(&ctx, "ivanov", "Иванов И.И.", "employee").await;
        let r = seed_request(&ctx, author).await;
        let admin = Identity::trusted_admin();
        let bytes = fx::tiny_pdf();
        let out = upload(&ctx, &admin, req(r), "..\\a/b\r\n\"c.pdf", &bytes)
            .await
            .unwrap();
        let a = out.attachment.expect("stored");
        assert_eq!(a.original_name, "bc.pdf");
        let blob = ctx.attachments.store().blob_path(&sha_hex(&bytes)).unwrap();
        assert!(!blob.to_string_lossy().contains("bc.pdf"));
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn attachments_bomb_png_no_thumbnail() {
    timed!(async {
        let (ctx, _dir) = fixture::make_test_ctx().await;
        let author = seed_user(&ctx, "ivanov", "Иванов И.И.", "employee").await;
        let r = seed_request(&ctx, author).await;
        let out = upload(
            &ctx,
            &Identity::trusted_admin(),
            req(r),
            "big.png",
            &fx::png_claiming(60_000, 60_000),
        )
        .await
        .unwrap();
        let a = out.attachment.expect("stored as a file");
        assert!(!a.has_thumbnail);
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn attachments_soft_deleted_parent_not_found() {
    timed!(async {
        let (ctx, _dir) = fixture::make_test_ctx().await;
        let author = seed_user(&ctx, "ivanov", "Иванов И.И.", "employee").await;
        let r = seed_request(&ctx, author).await;
        ctx.writer
            .execute(move |conn| {
                conn.execute(
                    "UPDATE requests SET deleted_at_utc = 1700000100 WHERE id = ?1",
                    params![r],
                )
                .map_err(map_rusqlite)?;
                Ok(())
            })
            .await
            .unwrap();
        let err = upload(
            &ctx,
            &Identity::trusted_admin(),
            req(r),
            "a.pdf",
            &fx::tiny_pdf(),
        )
        .await
        .unwrap_err();
        assert!(matches!(err, AppError::NotFound { .. }));
        assert_eq!(row_count(&ctx).await, 0);
    })
}

#[allow(dead_code)]
fn _uses(_: RejectCode) {}
