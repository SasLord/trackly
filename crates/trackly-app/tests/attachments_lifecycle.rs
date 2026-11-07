//! Soft delete, restore, purge, history of attachments (Phase 41.3 Plan 09).
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

#[allow(dead_code)]
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

async fn row_count(ctx: &AppCtx) -> i64 {
    ctx.writer
        .execute(|conn| {
            conn.query_row("SELECT COUNT(*) FROM attachments", [], |r| r.get(0))
                .map_err(map_rusqlite)
        })
        .await
        .expect("count")
}

async fn audit_count(ctx: &AppCtx, action: &str) -> i64 {
    let a = action.to_string();
    ctx.writer
        .execute(move |conn| {
            conn.query_row(
                "SELECT COUNT(*) FROM audit_log WHERE action = ?1",
                params![a],
                |r| r.get(0),
            )
            .map_err(map_rusqlite)
        })
        .await
        .expect("audit count")
}

fn sha_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn att_events(rx: &mut tokio::sync::broadcast::Receiver<WsEvent>) -> usize {
    fixture::collect_events(rx)
        .iter()
        .filter(|e| matches!(e, WsEvent::AttachmentsChanged { .. }))
        .count()
}

async fn up(ctx: &AppCtx, who: &Identity, parent: ParentRef, name: &str, bytes: &[u8]) -> i64 {
    upload(ctx, who, parent, name, bytes)
        .await
        .unwrap()
        .attachment
        .expect("stored")
        .id
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn lifecycle_soft_delete() {
    timed!(async {
        let (ctx, _dir) = fixture::make_test_ctx().await;
        let a = seed_user(&ctx, "ivanov", "Иванов И.И.", "employee").await;
        let b = seed_user(&ctx, "petrov", "Петров П.П.", "employee").await;
        let m = seed_user(&ctx, "sidorov", "Сидоров С.С.", "manager").await;
        let adm = seed_user(&ctx, "admin1", "Админов А.А.", "admin").await;
        let r = seed_request(&ctx, a).await;
        let ea = ident(a, Role::Employee);
        let eb = ident(b, Role::Employee);
        let mgr = ident(m, Role::Manager);
        let admin = ident(adm, Role::Admin);
        let pdf = fx::tiny_pdf();
        let id = up(&ctx, &ea, req(r), "акт.pdf", &pdf).await;

        // Another employee cannot delete.
        assert!(matches!(
            ctx.attachments.delete(&eb, id).await,
            Err(AppError::Forbidden)
        ));
        // Uploader deletes own.
        ctx.attachments.delete(&ea, id).await.unwrap();
        let blob = ctx.attachments.store().blob_path(&sha_hex(&pdf)).unwrap();
        assert!(blob.is_file(), "soft delete never touches the file");
        let l = ctx.attachments.list(req(r), &ea).await.unwrap();
        assert!(l.items.is_empty() && l.deleted.is_empty());
        let l = ctx.attachments.list(req(r), &mgr).await.unwrap();
        assert!(l.items.is_empty() && l.deleted.is_empty());
        let l = ctx.attachments.list(req(r), &admin).await.unwrap();
        assert_eq!(l.deleted.len(), 1);
        assert_eq!(l.deleted[0].deleted_by_name.as_deref(), Some("Иванов И.И."));
        assert!(l.deleted[0].deleted_at_utc.is_some());

        // Manager deletes any; employee-uploader who lost the parent is Forbidden.
        let id2 = up(&ctx, &ea, req(r), "второй.pdf", &fx::tiny_png()).await;
        ctx.writer
            .execute(move |conn| {
                conn.execute(
                    "UPDATE requests SET requested_by_user_id = ?1 WHERE id = ?2",
                    params![b, r],
                )
                .map_err(map_rusqlite)?;
                Ok(())
            })
            .await
            .unwrap();
        assert!(matches!(
            ctx.attachments.delete(&ea, id2).await,
            Err(AppError::Forbidden)
        ));
        ctx.attachments.delete(&mgr, id2).await.unwrap();
        assert_eq!(row_count(&ctx).await, 2);
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn lifecycle_idempotent_delete_no_second_audit_or_event() {
    timed!(async {
        let (ctx, _dir) = fixture::make_test_ctx().await;
        let a = seed_user(&ctx, "ivanov", "Иванов И.И.", "employee").await;
        let r = seed_request(&ctx, a).await;
        let admin = Identity::trusted_admin();
        let id = up(&ctx, &admin, req(r), "a.pdf", &fx::tiny_pdf()).await;
        let mut rx = ctx.ws_broadcast.subscribe();
        fixture::drain(&mut rx);
        ctx.attachments.delete(&admin, id).await.unwrap();
        assert_eq!(att_events(&mut rx), 1);
        ctx.attachments.delete(&admin, id).await.unwrap();
        assert_eq!(att_events(&mut rx), 0);
        assert_eq!(audit_count(&ctx, "custom:attachment_deleted").await, 1);
        // restore twice: one audit row, one event
        ctx.attachments.restore(&admin, id).await.unwrap();
        assert_eq!(att_events(&mut rx), 1);
        ctx.attachments.restore(&admin, id).await.unwrap();
        assert_eq!(att_events(&mut rx), 0);
        assert_eq!(audit_count(&ctx, "custom:attachment_restored").await, 1);
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn lifecycle_restore_admin_only_and_position() {
    timed!(async {
        let (ctx, _dir) = fixture::make_test_ctx().await;
        let a = seed_user(&ctx, "ivanov", "Иванов И.И.", "employee").await;
        let m = seed_user(&ctx, "sidorov", "Сидоров С.С.", "manager").await;
        let r = seed_request(&ctx, a).await;
        let admin = Identity::trusted_admin();
        let mgr = ident(m, Role::Manager);
        let ea = ident(a, Role::Employee);
        let i1 = up(&ctx, &admin, req(r), "1.pdf", &fx::tiny_pdf()).await;
        let i2 = up(&ctx, &admin, req(r), "2.png", &fx::tiny_png()).await;
        ctx.attachments.delete(&mgr, i1).await.unwrap();
        assert!(matches!(
            ctx.attachments.restore(&mgr, i1).await,
            Err(AppError::Forbidden)
        ));
        assert!(matches!(
            ctx.attachments.restore(&ea, i1).await,
            Err(AppError::Forbidden)
        ));
        ctx.attachments.restore(&admin, i1).await.unwrap();
        let l = ctx.attachments.list(req(r), &admin).await.unwrap();
        let ids: Vec<i64> = l.items.iter().map(|x| x.id).collect();
        assert_eq!(ids, vec![i1, i2], "back at the original position");
        assert!(l.deleted.is_empty());
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn lifecycle_purge_refcount() {
    timed!(async {
        let (ctx, _dir) = fixture::make_test_ctx().await;
        let a = seed_user(&ctx, "ivanov", "Иванов И.И.", "employee").await;
        let m = seed_user(&ctx, "sidorov", "Сидоров С.С.", "manager").await;
        let r1 = seed_request(&ctx, a).await;
        let r2 = seed_request(&ctx, a).await;
        let admin = Identity::trusted_admin();
        let mgr = ident(m, Role::Manager);
        let png = fx::png_sized(600, 400);
        let i1 = up(&ctx, &admin, req(r1), "a.png", &png).await;
        let i2 = up(&ctx, &admin, req(r2), "b.png", &png).await;
        let store = ctx.attachments.store().clone();
        let h = sha_hex(&png);
        let blob = store.blob_path(&h).unwrap();
        let thumb = store.thumb_path(&h).unwrap();
        assert!(blob.is_file() && thumb.is_file());

        // Not yet soft-deleted: Validation. Manager: Forbidden.
        assert!(matches!(
            ctx.attachments.purge(&admin, i1).await,
            Err(AppError::Validation { .. })
        ));
        ctx.attachments.delete(&admin, i1).await.unwrap();
        assert!(matches!(
            ctx.attachments.purge(&mgr, i1).await,
            Err(AppError::Forbidden)
        ));
        // The other row is live: blob stays.
        ctx.attachments.purge(&admin, i1).await.unwrap();
        assert!(blob.is_file() && thumb.is_file());
        assert_eq!(row_count(&ctx).await, 1);
        // The last (soft-deleted) reference goes: blob and thumbnail removed.
        ctx.attachments.delete(&admin, i2).await.unwrap();
        ctx.attachments.purge(&admin, i2).await.unwrap();
        assert!(!blob.exists() && !thumb.exists());
        assert_eq!(row_count(&ctx).await, 0);
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn lifecycle_purge_keeps_blob_for_soft_deleted_twin() {
    timed!(async {
        let (ctx, _dir) = fixture::make_test_ctx().await;
        let a = seed_user(&ctx, "ivanov", "Иванов И.И.", "employee").await;
        let r1 = seed_request(&ctx, a).await;
        let r2 = seed_request(&ctx, a).await;
        let admin = Identity::trusted_admin();
        let pdf = fx::tiny_pdf();
        let i1 = up(&ctx, &admin, req(r1), "a.pdf", &pdf).await;
        let i2 = up(&ctx, &admin, req(r2), "b.pdf", &pdf).await;
        ctx.attachments.delete(&admin, i1).await.unwrap();
        ctx.attachments.delete(&admin, i2).await.unwrap();
        let blob = ctx.attachments.store().blob_path(&sha_hex(&pdf)).unwrap();
        ctx.attachments.purge(&admin, i1).await.unwrap();
        assert!(blob.is_file(), "soft-deleted twin still references it");
        ctx.attachments.purge(&admin, i2).await.unwrap();
        assert!(!blob.exists());
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn lifecycle_race_upload_vs_purge() {
    timed!(async {
        let (ctx, _dir) = fixture::make_test_ctx().await;
        let a = seed_user(&ctx, "ivanov", "Иванов И.И.", "employee").await;
        let admin = Identity::trusted_admin();
        for round in 0..6 {
            let r1 = seed_request(&ctx, a).await;
            let r2 = seed_request(&ctx, a).await;
            let png = fx::png_sized(300 + round, 200);
            let i1 = up(&ctx, &admin, req(r1), "a.png", &png).await;
            ctx.attachments.delete(&admin, i1).await.unwrap();
            let (up_res, purge_res) = tokio::join!(
                upload(&ctx, &admin, req(r2), "b.png", &png),
                ctx.attachments.purge(&admin, i1)
            );
            purge_res.unwrap();
            let out = up_res.unwrap().attachment.expect("stored");
            let blob = ctx.attachments.store().blob_path(&sha_hex(&png)).unwrap();
            assert!(
                blob.is_file(),
                "round {round}: a live row must never lose its blob"
            );
            // A thumbnail lost to the purge is accepted: no error, flag false.
            let l = ctx.attachments.list(req(r2), &admin).await.unwrap();
            let item = l.items.iter().find(|x| x.id == out.id).expect("row");
            assert!(!item.missing);
        }
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn lifecycle_audit_history_and_events() {
    timed!(async {
        let (ctx, _dir) = fixture::make_test_ctx().await;
        let a = seed_user(&ctx, "ivanov", "Иванов И.И.", "employee").await;
        let b = seed_user(&ctx, "petrov", "Петров П.П.", "employee").await;
        let r = seed_request(&ctx, a).await;
        let admin = Identity::trusted_admin();
        let ea = ident(a, Role::Employee);
        let eb = ident(b, Role::Employee);
        let mut rx = ctx.ws_broadcast.subscribe();
        let live = up(&ctx, &admin, req(r), "живой.pdf", &fx::tiny_pdf()).await;
        let gone = up(&ctx, &admin, req(r), "удалён.png", &fx::tiny_png()).await;
        let purged = up(&ctx, &admin, req(r), "стёрт.gif", &fx::tiny_gif()).await;
        fixture::drain(&mut rx);
        ctx.attachments.delete(&admin, gone).await.unwrap();
        assert_eq!(att_events(&mut rx), 1);
        ctx.attachments.delete(&admin, purged).await.unwrap();
        ctx.attachments.purge(&admin, purged).await.unwrap();
        assert_eq!(att_events(&mut rx), 2);
        ctx.attachments.delete(&admin, live).await.unwrap();
        ctx.attachments.restore(&admin, live).await.unwrap();
        assert_eq!(att_events(&mut rx), 2);

        for act in ["deleted", "restored", "purged"] {
            assert!(audit_count(&ctx, &format!("custom:attachment_{act}")).await >= 1);
        }
        let h = ctx.attachments.history(&ea, req(r)).await.unwrap();
        let find = |id: i64, action: &str| {
            h.iter()
                .find(|e| e.attachment_id == id && e.action == action)
                .map(|e| (e.state.clone(), e.name.clone()))
        };
        assert_eq!(find(live, "added").unwrap().0, "active");
        assert_eq!(find(live, "restored").unwrap().0, "active");
        assert_eq!(
            find(gone, "deleted").unwrap(),
            ("deleted".into(), "удалён.png".into())
        );
        assert_eq!(find(purged, "purged").unwrap().0, "purged");
        assert_eq!(find(purged, "added").unwrap().0, "purged");
        // Another employee sees no history of someone else's request.
        assert!(matches!(
            ctx.attachments.history(&eb, req(r)).await,
            Err(AppError::Forbidden)
        ));
    })
}
