//! Интеграционные тесты `RequestService::delete()` / `RequestService::cancel()`
//! (GAP-12-07/A4, Plan 12-14).
//!
//! Покрывает:
//! - `delete()`: Admin/Manager soft-delete заявки в ЛЮБОМ статусе (включая
//!   "completed"), optimistic-lock mismatch, повторный delete (NotFound).
//! - `cancel()`: Employee-автор отменяет СОБСТВЕННУЮ "open" заявку; чужая
//!   заявка → Forbidden (BOLA); заявка в "in_progress" → Validation.

use std::sync::Arc;

use rusqlite::params;

use trackly_app::dto::request::{RequestCreateDto, RequestTransitionPayload};
use trackly_app::services::RequestService;
use trackly_core::auth::{Identity, Role};
use trackly_core::error::AppError;
use trackly_core::primitives::clock::Clock;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::db::writer_worker::WriterHandle;
use trackly_infra::test_support::test_app_ctx::test_writer_and_readers;

fn manager(user_id: i64) -> Identity {
    Identity {
        user_id: Some(user_id),
        role: Role::Manager,
    }
}

fn employee(user_id: i64) -> Identity {
    Identity {
        user_id: Some(user_id),
        role: Role::Employee,
    }
}

fn make_service() -> (RequestService, Arc<WriterHandle>, tempfile::TempDir) {
    let (writer, readers, dir) = test_writer_and_readers();
    let clock: Arc<dyn Clock + Send + Sync> = Arc::new(SystemClock);
    let (ws_tx, _rx) = tokio::sync::broadcast::channel(16);
    let svc = RequestService::new(writer.clone(), readers, clock, Arc::new(ws_tx));
    (svc, writer, dir)
}

/// Seed a real `users` row (FK target for `requested_by_user_id`) and return
/// its id. Mirrors `seed_pending_register`'s direct-SQL insert shape but for
/// an already-active employee, since these tests need real ownership.
async fn seed_user(writer: &WriterHandle, login: &str, full_name: &str, role: &str) -> i64 {
    let now = SystemClock.unix_seconds();
    let login = login.to_string();
    let full_name = full_name.to_string();
    let role = role.to_string();
    writer
        .execute(move |conn| {
            let tx = conn.transaction().map_err(|e| AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            tx.execute(
                "INSERT INTO users \
                 (login, full_name, password_hash, role, ad_user, is_active, \
                  created_at_utc, updated_at_utc, version) \
                 VALUES (?1, ?2, NULL, ?3, 0, 1, ?4, ?4, 1)",
                params![login, full_name, role, now],
            )
            .map_err(|e| AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            let user_id = tx.last_insert_rowid();
            tx.commit().map_err(|e| AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            Ok(user_id)
        })
        .await
        .expect("seed user")
}

// ---------------------------------------------------------------------------
// delete()
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delete_completed_request_succeeds() {
    let (svc, writer, _dir) = make_service();
    let employee_id = seed_user(&writer, "us400", "Employee Delete", "employee").await;
    let manager_id = seed_user(&writer, "us401", "Manager Delete", "manager").await;

    let created = svc
        .create(
            RequestCreateDto {
                request_type: "free_form".to_string(),
                printer_device_id: None,
                cartridge_model_id: None,
                category_id: None,
                description: Some("to be completed then deleted".to_string()),
            },
            &employee(employee_id),
        )
        .await
        .expect("create");

    // Drive to "completed": open -> in_progress -> completed (Admin/Manager).
    let accepted = svc
        .transition(
            RequestTransitionPayload::Accept {
                request_id: created.id,
                version: created.version,
                assigned_to_user_id: None,
            },
            &manager(manager_id),
        )
        .await
        .expect("accept");
    assert_eq!(accepted.status, "in_progress");

    let completed = svc
        .transition(
            RequestTransitionPayload::Complete {
                request_id: accepted.id,
                version: accepted.version,
                notes: None,
                linked_cartridge_id: None,
            },
            &manager(manager_id),
        )
        .await
        .expect("complete");
    assert_eq!(completed.status, "completed");

    svc.delete(completed.id, completed.version, &manager(manager_id))
        .await
        .expect("delete completed request should succeed");

    let readers = svc.readers.clone();
    let id = completed.id;
    let (deleted_at, version): (Option<i64>, i64) = tokio::task::spawn_blocking(move || {
        let conn = readers.acquire();
        conn.query_row(
            "SELECT deleted_at_utc, version FROM requests WHERE id = ?1",
            params![id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .expect("query request")
    })
    .await
    .expect("spawn_blocking");
    assert!(
        deleted_at.is_some(),
        "deleted_at_utc должен быть установлен"
    );
    assert_eq!(version, completed.version + 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delete_with_wrong_version_returns_optimistic_lock_mismatch() {
    let (svc, writer, _dir) = make_service();
    let employee_id = seed_user(&writer, "us402", "Employee OptLock", "employee").await;
    let manager_id = seed_user(&writer, "us403", "Manager OptLock", "manager").await;

    let created = svc
        .create(
            RequestCreateDto {
                request_type: "free_form".to_string(),
                printer_device_id: None,
                cartridge_model_id: None,
                category_id: None,
                description: Some("delete with stale version".to_string()),
            },
            &employee(employee_id),
        )
        .await
        .expect("create");

    let err = svc
        .delete(created.id, created.version + 99, &manager(manager_id))
        .await
        .expect_err("stale version must fail");
    assert!(
        matches!(err, AppError::OptimisticLockMismatch { .. }),
        "ожидали OptimisticLockMismatch, получили {err:?}"
    );
}

/// Second `delete()` on an already-deleted row: the row still physically
/// exists (soft-delete), so the disambiguation query (mirrors
/// `cartridge_service::delete()`'s established pattern — `SELECT version
/// FROM requests WHERE id=?` with no `deleted_at_utc` filter) finds it and
/// reports `OptimisticLockMismatch`, not `NotFound`. `NotFound` is reserved
/// for ids that never existed at all (see next test). This is consistent,
/// pre-existing project behavior — not specific to this plan.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delete_already_deleted_request_returns_optimistic_lock_mismatch() {
    let (svc, writer, _dir) = make_service();
    let employee_id = seed_user(&writer, "us404", "Employee Repeat", "employee").await;
    let manager_id = seed_user(&writer, "us405", "Manager Repeat", "manager").await;

    let created = svc
        .create(
            RequestCreateDto {
                request_type: "free_form".to_string(),
                printer_device_id: None,
                cartridge_model_id: None,
                category_id: None,
                description: Some("delete twice".to_string()),
            },
            &employee(employee_id),
        )
        .await
        .expect("create");

    svc.delete(created.id, created.version, &manager(manager_id))
        .await
        .expect("first delete succeeds");

    let err = svc
        .delete(created.id, created.version + 1, &manager(manager_id))
        .await
        .expect_err("second delete on an already-deleted row must fail");
    assert!(
        matches!(err, AppError::OptimisticLockMismatch { .. }),
        "ожидали OptimisticLockMismatch (row exists but deleted_at_utc IS NOT NULL), получили {err:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delete_nonexistent_request_returns_not_found() {
    let (svc, writer, _dir) = make_service();
    let manager_id = seed_user(&writer, "us411", "Manager NotFound", "manager").await;

    let err = svc
        .delete(999_999, 1, &manager(manager_id))
        .await
        .expect_err("deleting a nonexistent id must fail");
    assert!(
        matches!(err, AppError::NotFound { .. }),
        "ожидали NotFound, получили {err:?}"
    );
}

/// WR-04 (Phase 12 Round 2 review): an `ad_register` request owns the
/// reconciliation of a linked `users` row (Admin-only approve/reject). The
/// generic `delete()` (Admin|Manager, no user-row cleanup) must refuse to
/// soft-delete one — otherwise a Manager could orphan the linked user. We
/// insert the `ad_register` row directly because `create()` forbids that type.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delete_ad_register_request_is_refused() {
    let (svc, writer, _dir) = make_service();
    let employee_id = seed_user(&writer, "us412", "Pending AD User", "employee").await;
    let manager_id = seed_user(&writer, "us413", "Manager AdReg", "manager").await;

    // Seed an open `ad_register` request directly (the create() path allowlist
    // rejects `ad_register`, mirroring how AuthService writes these rows).
    let req_id = {
        let now = SystemClock.unix_seconds();
        writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(|e| AppError::Internal {
                    source_chain: format!("{e}"),
                })?;
                tx.execute(
                    "INSERT INTO requests \
                     (request_type, status, requested_by_user_id, ad_subtype, \
                      created_at_utc, updated_at_utc, version) \
                     VALUES ('ad_register', 'open', ?1, 'register', ?2, ?2, 1)",
                    params![employee_id, now],
                )
                .map_err(|e| AppError::Internal {
                    source_chain: format!("{e}"),
                })?;
                let id = tx.last_insert_rowid();
                tx.commit().map_err(|e| AppError::Internal {
                    source_chain: format!("{e}"),
                })?;
                Ok(id)
            })
            .await
            .expect("seed ad_register request")
    };

    let err = svc
        .delete(req_id, 1, &manager(manager_id))
        .await
        .expect_err("deleting an ad_register request must be refused");
    assert!(
        matches!(err, AppError::Validation { ref field, .. } if field == "request_type"),
        "ожидали Validation[request_type], получили {err:?}"
    );

    // The request must still be present (not soft-deleted) after the refusal.
    let readers = svc.readers.clone();
    let deleted_at: Option<i64> = tokio::task::spawn_blocking(move || {
        let conn = readers.acquire();
        conn.query_row(
            "SELECT deleted_at_utc FROM requests WHERE id = ?1",
            params![req_id],
            |r| r.get(0),
        )
        .expect("query request")
    })
    .await
    .expect("spawn_blocking");
    assert!(
        deleted_at.is_none(),
        "ad_register request must NOT have been soft-deleted"
    );
}

// ---------------------------------------------------------------------------
// cancel()
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancel_own_open_request_succeeds() {
    let (svc, writer, _dir) = make_service();
    let employee_id = seed_user(&writer, "us406", "Employee Cancel", "employee").await;

    let created = svc
        .create(
            RequestCreateDto {
                request_type: "free_form".to_string(),
                printer_device_id: None,
                cartridge_model_id: None,
                category_id: None,
                description: Some("self-cancel".to_string()),
            },
            &employee(employee_id),
        )
        .await
        .expect("create");

    let cancelled = svc
        .cancel(created.id, created.version, &employee(employee_id))
        .await
        .expect("cancel own open request should succeed");
    assert_eq!(cancelled.status, "cancelled");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancel_other_users_request_returns_forbidden() {
    let (svc, writer, _dir) = make_service();
    let owner_id = seed_user(&writer, "us407", "Owner", "employee").await;
    let other_id = seed_user(&writer, "us408", "Other Employee", "employee").await;

    let created = svc
        .create(
            RequestCreateDto {
                request_type: "free_form".to_string(),
                printer_device_id: None,
                cartridge_model_id: None,
                category_id: None,
                description: Some("not yours".to_string()),
            },
            &employee(owner_id),
        )
        .await
        .expect("create");

    let err = svc
        .cancel(created.id, created.version, &employee(other_id))
        .await
        .expect_err("cancelling another user's request must fail");
    assert!(
        matches!(err, AppError::Forbidden),
        "ожидали Forbidden (BOLA), получили {err:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancel_in_progress_request_returns_validation_error() {
    let (svc, writer, _dir) = make_service();
    let employee_id = seed_user(&writer, "us409", "Employee InProgress", "employee").await;
    let manager_id = seed_user(&writer, "us410", "Manager InProgress", "manager").await;

    let created = svc
        .create(
            RequestCreateDto {
                request_type: "free_form".to_string(),
                printer_device_id: None,
                cartridge_model_id: None,
                category_id: None,
                description: Some("already accepted".to_string()),
            },
            &employee(employee_id),
        )
        .await
        .expect("create");

    let accepted = svc
        .transition(
            RequestTransitionPayload::Accept {
                request_id: created.id,
                version: created.version,
                assigned_to_user_id: None,
            },
            &manager(manager_id),
        )
        .await
        .expect("accept");
    assert_eq!(accepted.status, "in_progress");

    let err = svc
        .cancel(accepted.id, accepted.version, &employee(employee_id))
        .await
        .expect_err("cancelling an in_progress request must fail");
    assert!(
        matches!(err, AppError::Validation { .. }),
        "ожидали Validation, получили {err:?}"
    );
}
