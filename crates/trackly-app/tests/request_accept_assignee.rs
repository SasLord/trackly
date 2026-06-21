//! Regression test for the request-accept FK bug (quick task 260621-r8x).
//!
//! Bug: the UI sent `assignedToUserId: identity.id` on accept. In unlocked
//! desktop mode `identity.id` is the sentinel `0` ("Рабочий стол"), which has
//! no `users` row, so the `requests.assigned_to_user_id → users(id)` FK failed
//! ("conflict: FOREIGN KEY constraint failed"). Reject worked because it sends
//! no assignee.
//!
//! Fix: `RequestService::transition` Accept now resolves the assignee
//! server-side from `caller.user_id` (None for trusted-desktop → COALESCE keeps
//! the existing value), ignoring the client-supplied value entirely.
//!
//! This test reproduces the exact failing path: a trusted-admin caller
//! (`user_id: None`) accepting a request while the client forges
//! `assigned_to_user_id: Some(0)`. Pre-fix this returned a FK error; post-fix
//! it succeeds and the request moves to `in_progress`.

use std::sync::Arc;
use std::time::Duration;

use rusqlite::params;
use trackly_app::dto::request::{RequestCreateDto, RequestTransitionPayload};
use trackly_app::services::RequestService;
use trackly_core::auth::{Identity, Role};
use trackly_core::primitives::clock::Clock;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::db::{pools::ReaderPool, writer_worker::WriterHandle};

/// Fresh migrated temp DB → (writer, readers). Mirrors dashboard_widgets.rs.
fn build_test_db() -> (Arc<WriterHandle>, Arc<ReaderPool>) {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let db_path = tmp.path().to_path_buf();
    std::mem::forget(tmp); // keep the file alive for the pool lifetime

    let mut conn = rusqlite::Connection::open(&db_path).unwrap();
    trackly_infra::db::pragmas::apply_writer_pragmas(&conn).unwrap();
    trackly_infra::db::migrations::run(&mut conn).unwrap();

    let writer = Arc::new(WriterHandle::spawn(conn));
    let readers = Arc::new(ReaderPool::new(&db_path, 2).unwrap());
    (writer, readers)
}

fn make_request_service(writer: Arc<WriterHandle>, readers: Arc<ReaderPool>) -> RequestService {
    let clock: Arc<dyn Clock + Send + Sync> = Arc::new(SystemClock);
    let (ws_tx, _rx) = tokio::sync::broadcast::channel(16);
    RequestService::new(writer, readers, clock, Arc::new(ws_tx))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn accept_resolves_assignee_from_caller_not_client_id() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (writer, readers) = build_test_db();

        // Seed one active employee (becomes users.id = 1) so the request's
        // requested_by_user_id FK is satisfied at creation time.
        let now = SystemClock.unix_seconds();
        let requester_id: i64 = writer
            .execute(move |conn| {
                conn.execute(
                    "INSERT INTO users \
                     (login, full_name, password_hash, role, ad_user, is_active, \
                      created_at_utc, updated_at_utc, version) \
                     VALUES ('emp1', 'Сотрудник Один', 'x', 'employee', 0, 1, ?1, ?1, 1)",
                    params![now],
                )
                .unwrap();
                Ok(conn.last_insert_rowid())
            })
            .await
            .expect("seed employee");

        let svc = make_request_service(writer, readers);

        // Create a free-form request as the employee (valid requested_by FK).
        let employee = Identity {
            user_id: Some(requester_id),
            role: Role::Employee,
        };
        let created = svc
            .create(
                RequestCreateDto {
                    request_type: "free_form".to_string(),
                    printer_device_id: None,
                    cartridge_model_id: None,
                    category_id: None,
                    description: Some("Глючит офис".to_string()),
                },
                &employee,
            )
            .await
            .expect("create request");
        assert_eq!(created.status, "open");

        // Accept as trusted-admin (user_id: None) while the client forges a
        // bogus assignee id 0 — the exact unlocked-desktop failure path.
        let trusted_admin = Identity {
            user_id: None,
            role: Role::Admin,
        };
        let accepted = svc
            .transition(
                RequestTransitionPayload::Accept {
                    request_id: created.id,
                    version: created.version,
                    assigned_to_user_id: Some(0), // bogus client value — must be ignored
                },
                &trusted_admin,
            )
            .await
            .expect("accept must NOT fail with a FOREIGN KEY constraint error");

        assert_eq!(
            accepted.status, "in_progress",
            "accepted request moves to in_progress"
        );
        // Server ignored the forged id 0 and used caller.user_id (None) →
        // COALESCE kept the pre-existing NULL assignee. No invalid FK written.
        assert_eq!(
            accepted.assigned_to_user_id, None,
            "trusted-admin accept leaves assignee NULL (caller.user_id was None), never id 0"
        );
    })
    .await
    .expect("accept_resolves_assignee_from_caller_not_client_id exceeded budget");
}
