//! Access control of the attachment service (Phase 41.3 Plan 05):
//! BOLA, role matrix, parent soft-delete follow, missing file.
//! Names are fictional.

use std::time::Duration;

use rusqlite::params;
use trackly_app::context::AppCtx;
use trackly_app::services::attachment_service::{Intent, ParentRef, ReadKind};
use trackly_core::auth::{Identity, Role};
use trackly_core::domain::attachments::ParentKind;
use trackly_core::error::AppError;
use trackly_infra::error_conversions::map_rusqlite;
use trackly_infra::repos::attachments_sqlite::{AttachmentNew, SqliteAttachmentRepository};

#[path = "entities_support/fixture.rs"]
#[allow(dead_code)]
mod fixture;

const NOW: i64 = 1_700_000_000;

fn hash(seed: u8) -> String {
    format!("{seed:02x}").repeat(32)
}

fn ident(id: i64, role: Role) -> Identity {
    Identity {
        user_id: Some(id),
        role,
    }
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

async fn seed_request(ctx: &AppCtx, author: i64, status: &str) -> i64 {
    let status = status.to_string();
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO requests (request_type, status, requested_by_user_id, \
                 created_at_utc, updated_at_utc) VALUES ('free_form', ?1, ?2, 1700000000, 1700000000)",
                params![status, author],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed request")
}

async fn seed_cartridge(ctx: &AppCtx) -> i64 {
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO cartridge_models (brand, model, created_at_utc, updated_at_utc) \
                 VALUES ('Brand', 'M-1', 1700000000, 1700000000)",
                [],
            )
            .map_err(map_rusqlite)?;
            let model = conn.last_insert_rowid();
            conn.execute(
                "INSERT INTO cartridges (code, model_id, created_at_utc, updated_at_utc) \
                 VALUES ('C-T00001', ?1, 1700000000, 1700000000)",
                params![model],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed cartridge")
}

async fn soft_delete(ctx: &AppCtx, table: &'static str, id: i64, at: Option<i64>) {
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                &format!("UPDATE {table} SET deleted_at_utc = ?1 WHERE id = ?2"),
                params![at, id],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("soft delete");
}

/// Row plus a blob on disk (content is arbitrary, the hash is a fake).
async fn seed_attachment(
    ctx: &AppCtx,
    kind: ParentKind,
    parent_id: i64,
    name: &str,
    seed: u8,
    uploader: Option<i64>,
) -> i64 {
    let h = hash(seed);
    let path = ctx.attachments.store().blob_path(&h).unwrap();
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, b"data").unwrap();
    let (name, h2) = (name.to_string(), h);
    ctx.writer
        .execute(move |conn| {
            SqliteAttachmentRepository.insert_on(
                conn,
                &AttachmentNew {
                    parent_kind: kind.as_str(),
                    parent_id,
                    original_name: &name,
                    mime: "application/pdf",
                    size_bytes: 4,
                    content_hash: &h2,
                    uploaded_by_user_id: uploader,
                    created_at_utc: NOW,
                },
            )
        })
        .await
        .expect("seed attachment")
}

fn p(kind: ParentKind, id: i64) -> ParentRef {
    ParentRef { kind, id }
}

macro_rules! timed {
    ($body:expr) => {
        tokio::time::timeout(Duration::from_secs(30), $body)
            .await
            .expect("timeout")
    };
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn attachments_access_roles() {
    timed!(async {
        let (ctx, _dir) = fixture::make_test_ctx().await;
        let svc = &ctx.attachments;
        let author = seed_user(&ctx, "ivanov", "Иванов И.И.", "employee").await;
        let other = seed_user(&ctx, "petrov", "Петров П.П.", "employee").await;
        let mgr = seed_user(&ctx, "mgr", "Сидоров С.С.", "manager").await;
        let device = fixture::seed_device(&ctx, 1, "ПК-1", "INV-1", None, None).await;
        let printer = fixture::seed_device(&ctx, 2, "Принтер-1", "INV-2", None, None).await;
        let cart = seed_cartridge(&ctx).await;
        let completed = seed_request(&ctx, author, "completed").await;
        let cancelled = seed_request(&ctx, author, "cancelled").await;
        let open = seed_request(&ctx, author, "open").await;
        for (i, (k, id)) in [
            (ParentKind::Request, completed),
            (ParentKind::Request, cancelled),
            (ParentKind::Request, open),
            (ParentKind::Device, device),
            (ParentKind::Device, printer),
            (ParentKind::Cartridge, cart),
        ]
        .into_iter()
        .enumerate()
        {
            seed_attachment(&ctx, k, id, "скан.pdf", i as u8 + 1, Some(author)).await;
        }

        let admin = Identity::trusted_admin();
        let manager = ident(mgr, Role::Manager);
        let emp = ident(author, Role::Employee);
        let stranger = ident(other, Role::Employee);

        for (k, id) in [
            (ParentKind::Request, open),
            (ParentKind::Device, device),
            (ParentKind::Device, printer),
            (ParentKind::Cartridge, cart),
        ] {
            for who in [&admin, &manager] {
                let l = svc.list(p(k, id), who).await.expect("staff list");
                assert_eq!(l.items.len(), 1);
                assert!(l.can_attach);
            }
        }
        // Employee-author: own request in any status (D-02).
        for id in [open, completed, cancelled] {
            let l = svc.list(p(ParentKind::Request, id), &emp).await.unwrap();
            assert_eq!(l.items.len(), 1);
            assert!(l.can_attach && !l.can_delete_any && !l.can_manage_deleted);
            assert!(!l.readonly);
            assert!(l.deleted.is_empty());
        }
        // Foreign request.
        let e = svc.list(p(ParentKind::Request, open), &stranger).await;
        assert!(matches!(e, Err(AppError::Forbidden)));
        // Devices, printers, cartridges: denied for Employee (D-27).
        for (k, id) in [
            (ParentKind::Device, device),
            (ParentKind::Device, printer),
            (ParentKind::Cartridge, cart),
        ] {
            for who in [&emp, &stranger] {
                let e = svc.list(p(k, id), who).await;
                assert!(matches!(e, Err(AppError::Forbidden)), "{k:?}");
                let e = svc.authorize(p(k, id), who, Intent::Attach).await;
                assert!(matches!(e, Err(AppError::Forbidden)));
            }
        }
        // Perms.
        let a = svc
            .authorize(p(ParentKind::Request, open), &admin, Intent::Read)
            .await
            .unwrap();
        assert!(a.can_manage_deleted && a.can_delete_any && a.can_attach);
        let m = svc
            .authorize(p(ParentKind::Request, open), &manager, Intent::Read)
            .await
            .unwrap();
        assert!(!m.can_manage_deleted && m.can_delete_any);
        assert!(matches!(
            svc.authorize(
                p(ParentKind::Request, open),
                &manager,
                Intent::ManageDeleted
            )
            .await,
            Err(AppError::Forbidden)
        ));
        assert!(matches!(
            svc.authorize(p(ParentKind::Request, open), &emp, Intent::DeleteAny)
                .await,
            Err(AppError::Forbidden)
        ));
        // DeleteOwn: only the uploader.
        assert!(svc
            .authorize(
                p(ParentKind::Request, open),
                &emp,
                Intent::DeleteOwn {
                    uploaded_by: Some(author)
                }
            )
            .await
            .is_ok());
        assert!(matches!(
            svc.authorize(
                p(ParentKind::Request, open),
                &emp,
                Intent::DeleteOwn {
                    uploaded_by: Some(mgr)
                }
            )
            .await,
            Err(AppError::Forbidden)
        ));
    });
}

#[test]
fn attachments_access_unknown_kind_rejected() {
    assert!(ParentKind::parse("message").is_none());
    assert!(ParentKind::parse("printer").is_none());
    assert!(ParentKind::parse("../etc").is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn attachments_access_parent_follow() {
    timed!(async {
        let (ctx, _dir) = fixture::make_test_ctx().await;
        let svc = &ctx.attachments;
        let author = seed_user(&ctx, "ivanov", "Иванов И.И.", "employee").await;
        let req = seed_request(&ctx, author, "open").await;
        let att =
            seed_attachment(&ctx, ParentKind::Request, req, "скан.pdf", 7, Some(author)).await;
        let admin = Identity::trusted_admin();
        let emp = ident(author, Role::Employee);

        soft_delete(&ctx, "requests", req, Some(NOW)).await;
        for who in [&admin, &emp] {
            let e = svc.list(p(ParentKind::Request, req), who).await;
            assert!(matches!(e, Err(AppError::NotFound { .. })));
            let e = svc.open_for_read(att, who, ReadKind::File).await;
            assert!(matches!(e, Err(AppError::NotFound { .. })));
        }
        soft_delete(&ctx, "requests", req, None).await;
        let l = svc.list(p(ParentKind::Request, req), &emp).await.unwrap();
        assert_eq!(l.items.len(), 1);
        assert!(svc.open_for_read(att, &emp, ReadKind::File).await.is_ok());
    });
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn attachments_access_bola_by_id() {
    timed!(async {
        let (ctx, _dir) = fixture::make_test_ctx().await;
        let svc = &ctx.attachments;
        let author = seed_user(&ctx, "ivanov", "Иванов И.И.", "employee").await;
        let other = seed_user(&ctx, "petrov", "Петров П.П.", "employee").await;
        let req = seed_request(&ctx, author, "open").await;
        let device = fixture::seed_device(&ctx, 1, "ПК-1", "INV-1", None, None).await;
        let on_req =
            seed_attachment(&ctx, ParentKind::Request, req, "а.pdf", 1, Some(author)).await;
        let on_dev = seed_attachment(&ctx, ParentKind::Device, device, "б.pdf", 2, None).await;
        let stranger = ident(other, Role::Employee);
        let emp = ident(author, Role::Employee);

        for id in [on_req, on_dev] {
            let e = svc.open_for_read(id, &stranger, ReadKind::File).await;
            assert!(matches!(e, Err(AppError::Forbidden)), "id {id}");
        }
        // The author reads own request file but not a device file.
        assert!(svc
            .open_for_read(on_req, &emp, ReadKind::File)
            .await
            .is_ok());
        assert!(matches!(
            svc.open_for_read(on_dev, &emp, ReadKind::File).await,
            Err(AppError::Forbidden)
        ));
        // Unknown id.
        assert!(matches!(
            svc.open_for_read(99_999, &Identity::trusted_admin(), ReadKind::File)
                .await,
            Err(AppError::NotFound { .. })
        ));
    });
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn attachments_access_missing_file() {
    timed!(async {
        let (ctx, _dir) = fixture::make_test_ctx().await;
        let svc = &ctx.attachments;
        let author = seed_user(&ctx, "ivanov", "Иванов И.И.", "employee").await;
        let req = seed_request(&ctx, author, "open").await;
        let att =
            seed_attachment(&ctx, ParentKind::Request, req, "скан.pdf", 9, Some(author)).await;
        let admin = Identity::trusted_admin();

        let l = svc.list(p(ParentKind::Request, req), &admin).await.unwrap();
        assert!(!l.items[0].missing);
        std::fs::remove_file(svc.store().blob_path(&hash(9)).unwrap()).unwrap();
        let l = svc.list(p(ParentKind::Request, req), &admin).await.unwrap();
        assert!(l.items[0].missing);
        let e = svc.open_for_read(att, &admin, ReadKind::File).await;
        assert!(matches!(
            e,
            Err(AppError::NotFound {
                entity: "attachment_file",
                ..
            })
        ));
        // Thumbnail for a non-photo row is NotFound too.
        let e = svc.open_for_read(att, &admin, ReadKind::Thumb).await;
        assert!(matches!(e, Err(AppError::NotFound { .. })));
    });
}
