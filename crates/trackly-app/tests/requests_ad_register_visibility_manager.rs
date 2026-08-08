//! Regression test: quick task 260808-np4 (unify ad_register visibility
//! predicate).
//!
//! Purpose: close the ONE coverage gap left after consolidating the REQ-06 /
//! T-09-11 "only Admin sees `ad_register` requests" rule into a single
//! `trackly_core::auth::excludes_ad_register` function and a single
//! `trackly_infra::repos::requests_sqlite::ad_register_predicate` /
//! `ad_register_exclude_clause` pair — `requests_ad_register.rs`'s
//! `ad_register_admin_only` / `ad_register_excluded_from_employee_counts`
//! only ever drove `RequestService::list`/`counts` through an Employee
//! Identity. This file exercises the same predicate through a Manager
//! Identity (never previously covered) via the service layer, with a
//! control assertion that a Manager's own non-ad_register request remains
//! visible/counted, and a companion Admin assertion proving the exclusion
//! is role-specific rather than a blanket "hide everything" bug.

use std::sync::Arc;

use rusqlite::params;

use trackly_app::dto::request::RequestCreateDto;
use trackly_app::services::RequestService;
use trackly_core::auth::{Identity, Role};
use trackly_core::domain::requests::{Pagination, RequestFilter};
use trackly_core::error::AppError;
use trackly_core::primitives::clock::Clock;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::db::writer_worker::WriterHandle;
use trackly_infra::test_support::test_app_ctx::test_writer_and_readers;

fn admin() -> Identity {
    // user_id = None mirrors Identity::trusted_admin() (D-Desktop-01
    // unlocked mode), matching requests_ad_register.rs's admin() helper.
    Identity {
        user_id: None,
        role: Role::Admin,
    }
}

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
/// its id. Mirrors `request_lifecycle.rs`'s `seed_user`.
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

/// Seed an open `ad_register` request directly (the `create()` path
/// allowlist rejects `ad_register`, mirroring how `AuthService` writes these
/// rows). Mirrors `request_lifecycle.rs`'s `seed_ad_register`.
async fn seed_ad_register(writer: &WriterHandle, requested_by: i64) -> i64 {
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
                params![requested_by, now],
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
}

// ---------------------------------------------------------------------------
// Manager-role visibility gap (quick-260808-np4)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn manager_cannot_see_ad_register_in_list_or_counts_admin_can() {
    let (svc, writer, _dir) = make_service();

    // 1. Seed a real employee user (owner) and an ad_register request for
    //    that owner.
    let owner_id = seed_user(&writer, "us500", "Смирнов Кирилл", "employee").await;
    let ad_register_id = seed_ad_register(&writer, owner_id).await;

    // 2. Create a real free_form request for the same owner — the control
    //    row that MUST remain visible to everyone.
    let free_form = svc
        .create(
            RequestCreateDto {
                request_type: "free_form".to_string(),
                printer_device_id: None,
                cartridge_model_id: None,
                category_id: None,
                description: Some("контрольная заявка".to_string()),
            },
            &employee(owner_id),
        )
        .await
        .expect("create free_form control request");

    // 3. Seed a real manager user.
    let manager_id = seed_user(&writer, "us501", "Логинова Вера", "manager").await;

    // 4. Manager: list() must exclude ad_register but include the control
    //    free_form request.
    let manager_list = svc
        .list(
            RequestFilter::default(),
            Pagination::default(),
            &manager(manager_id),
        )
        .await
        .expect("list as manager");
    assert!(
        manager_list
            .items
            .iter()
            .all(|r| r.request_type != "ad_register"),
        "manager list must not contain any ad_register requests"
    );
    assert!(
        manager_list.items.iter().any(|r| r.id == free_form.id),
        "manager list must still contain the control free_form request"
    );

    // 5. Manager: counts() must report exactly 1 (the free_form request
    //    only) — precise count, not just > 0, to catch a predicate that
    //    silently becomes a no-op for one bucket but not others.
    let manager_counts = svc
        .counts(&manager(manager_id))
        .await
        .expect("counts as manager");
    assert_eq!(
        manager_counts.all, 1,
        "manager counts.all must be exactly 1 (only the control free_form request)"
    );

    // 6. Admin: list()/counts() must see BOTH requests — proves the
    //    exclusion is role-specific, not a blanket "hide everything" bug.
    let admin_list = svc
        .list(RequestFilter::default(), Pagination::default(), &admin())
        .await
        .expect("list as admin");
    assert!(
        admin_list.items.iter().any(|r| r.id == ad_register_id),
        "admin list must contain the ad_register request"
    );
    assert!(
        admin_list.items.iter().any(|r| r.id == free_form.id),
        "admin list must contain the control free_form request"
    );

    let admin_counts = svc.counts(&admin()).await.expect("counts as admin");
    assert_eq!(
        admin_counts.all, 2,
        "admin counts.all must be exactly 2 (both requests)"
    );
}
