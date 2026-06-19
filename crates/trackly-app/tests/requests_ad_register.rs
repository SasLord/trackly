//! Интеграционные тесты `RequestService::approve_ad_register` / reject
//! branching для заявок `ad_register` (Phase 9 Plan 03 — USR-09/USR-11/
//! SET-10/REQ-06).
//!
//! Покрывает:
//! - admin-only видимость `ad_register` в `list()` (T-09-11).
//! - approve с выбранной ролью / ролью по умолчанию (D-REG-02).
//! - reject: pending discard / auto-accept soft-delete / restore-reject
//!   (D-REG-03, T-09-14).

use std::sync::Arc;

use rusqlite::params;

use trackly_app::dto::request::{ApproveAdRegisterDto, RequestTransitionPayload};
use trackly_app::services::RequestService;
use trackly_core::auth::{Identity, Role};
use trackly_core::domain::requests::{Pagination, RequestFilter};
use trackly_core::error::AppError;
use trackly_core::primitives::clock::Clock;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::db::writer_worker::WriterHandle;
use trackly_infra::test_support::test_app_ctx::test_writer_and_readers;

fn admin() -> Identity {
    // user_id = None mirrors Identity::trusted_admin() (D-Desktop-01 unlocked
    // mode) — audit_log.user_id is nullable and FK-checked when Some, so
    // tests that don't seed a real admin user row must use None here.
    Identity {
        user_id: None,
        role: Role::Admin,
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

/// Seed a "pending" ad_register scenario: inactive AD user row + open
/// ad_register/register request referencing it. Mirrors
/// `AuthService::create_pending_registration`'s write shape.
async fn seed_pending_register(writer: &WriterHandle, login: &str, full_name: &str) -> (i64, i64) {
    let now = SystemClock.unix_seconds();
    let login = login.to_string();
    let full_name = full_name.to_string();
    writer
        .execute(move |conn| {
            let tx = conn.transaction().map_err(|e| AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            tx.execute(
                "INSERT INTO users \
                 (login, full_name, password_hash, role, ad_user, is_active, \
                  created_at_utc, updated_at_utc, version) \
                 VALUES (?1, ?2, NULL, 'employee', 1, 0, ?3, ?3, 1)",
                params![login, full_name, now],
            )
            .map_err(|e| AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            let user_id = tx.last_insert_rowid();
            tx.execute(
                "INSERT INTO requests \
                 (request_type, status, requested_by_user_id, description, ad_subtype, \
                  created_at_utc, updated_at_utc, version) \
                 VALUES ('ad_register', 'open', ?1, ?2, 'register', ?3, ?3, 1)",
                params![user_id, full_name, now],
            )
            .map_err(|e| AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            let request_id = tx.last_insert_rowid();
            tx.commit().map_err(|e| AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            Ok((user_id, request_id))
        })
        .await
        .expect("seed pending register")
}

/// Seed an "auto-accept" ad_register scenario: ACTIVE AD user row + open
/// ad_register/register request referencing it. Mirrors
/// `AuthService::auto_register_ad_user`'s write shape.
async fn seed_auto_accept_register(
    writer: &WriterHandle,
    login: &str,
    full_name: &str,
) -> (i64, i64) {
    let now = SystemClock.unix_seconds();
    let login = login.to_string();
    let full_name = full_name.to_string();
    writer
        .execute(move |conn| {
            let tx = conn.transaction().map_err(|e| AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            tx.execute(
                "INSERT INTO users \
                 (login, full_name, password_hash, role, ad_user, is_active, \
                  created_at_utc, updated_at_utc, version) \
                 VALUES (?1, ?2, NULL, 'employee', 1, 1, ?3, ?3, 1)",
                params![login, full_name, now],
            )
            .map_err(|e| AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            let user_id = tx.last_insert_rowid();
            tx.execute(
                "INSERT INTO requests \
                 (request_type, status, requested_by_user_id, description, ad_subtype, \
                  created_at_utc, updated_at_utc, version) \
                 VALUES ('ad_register', 'open', ?1, ?2, 'register', ?3, ?3, 1)",
                params![user_id, full_name, now],
            )
            .map_err(|e| AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            let request_id = tx.last_insert_rowid();
            tx.commit().map_err(|e| AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            Ok((user_id, request_id))
        })
        .await
        .expect("seed auto-accept register")
}

/// Seed a "restore" ad_register scenario: existing BLOCKED user row
/// (is_active=0, deleted_at_utc=NULL) + open ad_register/restore request.
/// Mirrors `AuthService::create_restore_request`'s write shape.
async fn seed_restore_request(writer: &WriterHandle, login: &str, full_name: &str) -> (i64, i64) {
    let now = SystemClock.unix_seconds();
    let login = login.to_string();
    let full_name = full_name.to_string();
    writer
        .execute(move |conn| {
            let tx = conn.transaction().map_err(|e| AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            tx.execute(
                "INSERT INTO users \
                 (login, full_name, password_hash, role, ad_user, is_active, \
                  created_at_utc, updated_at_utc, version) \
                 VALUES (?1, ?2, NULL, 'employee', 1, 0, ?3, ?3, 1)",
                params![login, full_name, now],
            )
            .map_err(|e| AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            let user_id = tx.last_insert_rowid();
            tx.execute(
                "INSERT INTO requests \
                 (request_type, status, requested_by_user_id, description, ad_subtype, \
                  created_at_utc, updated_at_utc, version) \
                 VALUES ('ad_register', 'open', ?1, ?2, 'restore', ?3, ?3, 1)",
                params![user_id, full_name, now],
            )
            .map_err(|e| AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            let request_id = tx.last_insert_rowid();
            tx.commit().map_err(|e| AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            Ok((user_id, request_id))
        })
        .await
        .expect("seed restore request")
}

// ---------------------------------------------------------------------------
// Test 1: admin-only visibility (T-09-11 / REQ-06)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ad_register_admin_only() {
    let (svc, writer, _dir) = make_service();
    let (user_id, _request_id) = seed_pending_register(&writer, "us300", "Сидоров Пётр").await;

    let as_employee = svc
        .list(
            RequestFilter::default(),
            Pagination::default(),
            &employee(user_id),
        )
        .await
        .expect("list as employee");
    assert!(
        as_employee
            .items
            .iter()
            .all(|r| r.request_type != "ad_register"),
        "non-admin list должен НЕ содержать ad_register заявок"
    );

    let as_admin = svc
        .list(RequestFilter::default(), Pagination::default(), &admin())
        .await
        .expect("list as admin");
    assert!(
        as_admin
            .items
            .iter()
            .any(|r| r.request_type == "ad_register"),
        "admin list должен содержать ad_register заявки"
    );
}

// ---------------------------------------------------------------------------
// Test 2: approve with explicit role (D-REG-02)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn approve_creates_user_with_selected_role() {
    let (svc, writer, _dir) = make_service();
    let (user_id, request_id) = seed_pending_register(&writer, "us301", "Кузнецов Олег").await;

    let dto = svc
        .approve_ad_register(
            ApproveAdRegisterDto {
                request_id,
                version: 1,
                role: Some("manager".to_string()),
            },
            &admin(),
        )
        .await
        .expect("approve");
    assert_eq!(dto.status, "completed");

    let readers = svc.readers.clone();
    let (is_active, role): (i64, String) = tokio::task::spawn_blocking(move || {
        let conn = readers.acquire();
        conn.query_row(
            "SELECT is_active, role FROM users WHERE id = ?1",
            params![user_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .expect("query user")
    })
    .await
    .expect("spawn_blocking");
    assert_eq!(is_active, 1, "approve должен активировать пользователя");
    assert_eq!(role, "manager");
}

// ---------------------------------------------------------------------------
// Test 3: approve without explicit role defaults to employee (D-REG-02)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn approve_default_role_employee() {
    let (svc, writer, _dir) = make_service();
    let (user_id, request_id) = seed_pending_register(&writer, "us302", "Васильева Мария").await;

    svc.approve_ad_register(
        ApproveAdRegisterDto {
            request_id,
            version: 1,
            role: None,
        },
        &admin(),
    )
    .await
    .expect("approve without role");

    let readers = svc.readers.clone();
    let role: String = tokio::task::spawn_blocking(move || {
        let conn = readers.acquire();
        conn.query_row(
            "SELECT role FROM users WHERE id = ?1",
            params![user_id],
            |r| r.get(0),
        )
        .expect("query user")
    })
    .await
    .expect("spawn_blocking");
    assert_eq!(role, "employee", "default role должна быть employee");
}

// ---------------------------------------------------------------------------
// Test 4: reject pending discards (user stays inactive, no access)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reject_pending_discards() {
    let (svc, writer, _dir) = make_service();
    let (user_id, request_id) = seed_pending_register(&writer, "us303", "Никитин Игорь").await;

    let dto = svc
        .transition(
            RequestTransitionPayload::Reject {
                request_id,
                version: 1,
                notes: Some("отказ".to_string()),
            },
            &admin(),
        )
        .await
        .expect("reject pending");
    assert_eq!(dto.status, "rejected");

    let readers = svc.readers.clone();
    let (is_active, deleted_at): (i64, Option<i64>) = tokio::task::spawn_blocking(move || {
        let conn = readers.acquire();
        conn.query_row(
            "SELECT is_active, deleted_at_utc FROM users WHERE id = ?1",
            params![user_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .expect("query user")
    })
    .await
    .expect("spawn_blocking");
    assert_eq!(
        is_active, 0,
        "pending reject: пользователь остаётся неактивным"
    );
    assert_eq!(
        deleted_at, None,
        "pending reject: пользователь НЕ должен быть soft-deleted (он никогда не был активен)"
    );
}

// ---------------------------------------------------------------------------
// Test 5: reject auto-accept soft-deletes the already-active user
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reject_auto_accept_softdeletes_user() {
    let (svc, writer, _dir) = make_service();
    let (user_id, request_id) =
        seed_auto_accept_register(&writer, "us304", "Орлова Светлана").await;

    let dto = svc
        .transition(
            RequestTransitionPayload::Reject {
                request_id,
                version: 1,
                notes: None,
            },
            &admin(),
        )
        .await
        .expect("reject auto-accept");
    assert_eq!(dto.status, "rejected");

    let readers = svc.readers.clone();
    let (is_active, deleted_at): (i64, Option<i64>) = tokio::task::spawn_blocking(move || {
        let conn = readers.acquire();
        conn.query_row(
            "SELECT is_active, deleted_at_utc FROM users WHERE id = ?1",
            params![user_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .expect("query user")
    })
    .await
    .expect("spawn_blocking");
    assert_eq!(
        is_active, 0,
        "auto-accept reject должен деактивировать пользователя"
    );
    assert!(
        deleted_at.is_some(),
        "auto-accept reject должен soft-delete пользователя (T-09-14)"
    );
}

// ---------------------------------------------------------------------------
// Test 6: approve restore revives the blocked/deleted user
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn approve_restore_revives_user() {
    let (svc, writer, _dir) = make_service();
    let (user_id, request_id) = seed_restore_request(&writer, "us305", "Тихонов Артём").await;

    // Mark the seeded user as soft-deleted too, to exercise the "blocked
    // AND soft-deleted" combined case (D-REG-03 treats both the same way).
    let writer2 = writer.clone();
    let now = SystemClock.unix_seconds();
    writer2
        .execute(move |conn| {
            conn.execute(
                "UPDATE users SET deleted_at_utc = ?1 WHERE id = ?2",
                params![now, user_id],
            )
            .map_err(|e| AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            Ok(())
        })
        .await
        .expect("mark soft-deleted");

    let dto = svc
        .approve_ad_register(
            ApproveAdRegisterDto {
                request_id,
                version: 1,
                role: Some("admin".to_string()),
            },
            &admin(),
        )
        .await
        .expect("approve restore");
    assert_eq!(dto.status, "completed");

    let readers = svc.readers.clone();
    let (is_active, deleted_at, role): (i64, Option<i64>, String) =
        tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            conn.query_row(
                "SELECT is_active, deleted_at_utc, role FROM users WHERE id = ?1",
                params![user_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .expect("query user")
        })
        .await
        .expect("spawn_blocking");
    assert_eq!(
        is_active, 1,
        "restore approve должен реактивировать пользователя"
    );
    assert_eq!(deleted_at, None, "restore approve должен снять soft-delete");
    assert_eq!(role, "admin");
}
