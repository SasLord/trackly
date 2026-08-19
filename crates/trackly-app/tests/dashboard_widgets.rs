// audit_log.action for cartridge install = 'custom:install' (verified in CartridgeTransitionOp::audit_action)
//
//! Dashboard widget counts integration test — Phase 7 Plan 03 (GREEN).
//!
//! Covers DASH-01..05:
//!   - DASH-01: devices total + by-status breakdown
//!   - DASH-02: cartridges by-status breakdown + low-stock count
//!   - DASH-03: consumption chart (ConsumptionPoint list)
//!   - DASH-04: request counts open / in_progress / completed
//!   - DASH-05: printer online / offline / problematic

use std::sync::Arc;
use std::time::Duration;

use rusqlite::params;

use trackly_app::services::dashboard_service::DashboardService;
use trackly_core::auth::{Identity, Role};
use trackly_core::error::AppError;
use trackly_core::primitives::clock::Clock;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::db::{pools::ReaderPool, writer_worker::WriterHandle};
use trackly_infra::AppConfig;

/// Build an in-memory test DB and return (writer, readers) for DashboardService.
fn build_test_db() -> (Arc<WriterHandle>, Arc<ReaderPool>) {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let db_path = tmp.path().to_path_buf();

    // Leak the temp file handle so it isn't deleted before the pool closes.
    std::mem::forget(tmp);

    let mut conn = rusqlite::Connection::open(&db_path).unwrap();
    trackly_infra::db::pragmas::apply_writer_pragmas(&conn).unwrap();
    trackly_infra::db::migrations::run(&mut conn).unwrap();

    let writer = Arc::new(WriterHandle::spawn(conn));
    let readers = Arc::new(ReaderPool::new(&db_path, 2).unwrap());
    (writer, readers)
}

/// Verify that DashboardWidgetDto is populated with correct aggregate counts
/// on an empty (but fully-migrated) database.
///
/// On an empty DB: all counts should be 0 and vecs empty.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn dashboard_widget_counts_match_db_state() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (writer, readers) = build_test_db();
        let clock =
            Arc::new(SystemClock) as Arc<dyn trackly_core::primitives::clock::Clock + Send + Sync>;
        let config = Arc::new(AppConfig::default());

        let svc = DashboardService::new(writer, readers, clock, config);
        let dto = svc
            .get_all_widgets(&Identity::trusted_admin(), None)
            .await
            .unwrap();

        // On empty DB: devices_total should be 0.
        assert_eq!(dto.devices_total, 0, "empty DB: devices_total = 0");
        // Cartridge counts: empty.
        assert_eq!(dto.cartridge_by_status.len(), 0, "no cartridges");
        // Low-stock: 0 models.
        assert_eq!(dto.low_stock_count, 0, "no low-stock models");
        // Requests: all 0.
        assert_eq!(dto.request_counts_open, 0);
        assert_eq!(dto.request_counts_in_progress, 0);
        assert_eq!(dto.request_counts_completed, 0);
        // Printers: 0 total.
        assert_eq!(dto.printer_online, 0);
        assert_eq!(dto.printer_offline, 0);
        assert_eq!(dto.printer_problematic, 0);
    })
    .await
    .expect("dashboard_widget_counts_match_db_state budget")
}

/// Verify that low_stock_count and low_stock_models reflect cartridge stock state.
///
/// On an empty DB, low_stock_count = 0 (no models at all).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn dashboard_low_stock_reflects_cartridge_state() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (writer, readers) = build_test_db();
        let clock =
            Arc::new(SystemClock) as Arc<dyn trackly_core::primitives::clock::Clock + Send + Sync>;
        let config = Arc::new(AppConfig::default());

        let svc = DashboardService::new(writer, readers, clock, config);
        let dto = svc
            .get_all_widgets(&Identity::trusted_admin(), None)
            .await
            .unwrap();

        // Empty DB has no cartridge models, so low_stock_count = 0.
        assert_eq!(
            dto.low_stock_count, 0,
            "no cartridge models → low_stock_count = 0"
        );
        assert!(
            dto.low_stock_models.is_empty(),
            "no cartridge models → low_stock_models empty"
        );

        // Consumption chart on empty DB should return empty vec.
        let chart = svc.get_consumption_chart(3).await.unwrap();
        assert!(chart.is_empty(), "empty DB → no consumption chart data");
    })
    .await
    .expect("dashboard_low_stock_reflects_cartridge_state budget")
}

/// Seed one employee user with a single request row of the given
/// `request_type`. Mirrors `seed_pending_register`'s INSERT shape from
/// `requests_ad_register.rs` (cannot import it directly — separate
/// integration test binary). Returns `(user_id, request_id)`.
async fn seed_employee_with_request(
    writer: &WriterHandle,
    login: &str,
    full_name: &str,
    request_type: &str,
) -> (i64, i64) {
    let now = SystemClock.unix_seconds();
    let login = login.to_string();
    let full_name = full_name.to_string();
    let request_type = request_type.to_string();
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
                 VALUES (?1, 'open', ?2, ?3, NULL, ?4, ?4, 1)",
                params![request_type, user_id, full_name, now],
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
        .expect("seed employee with request")
}

/// Regression test for the third (and last) of three independently-written
/// request-counting code paths that leaked the invisible, auto-created
/// `ad_register` row into an employee-visible count.
///
/// Test A: an employee whose ONLY request is `ad_register` must see zero
/// counts from the dashboard widget — matching the empty list they see on
/// the requests page.
///
/// Test B (control): the SAME employee's real (`free_form`) request is
/// still counted normally — proving the exclusion is scoped to
/// `ad_register`, not a blanket suppression of the employee's own requests.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn dashboard_employee_widget_excludes_ad_register() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (writer, readers) = build_test_db();
        let clock =
            Arc::new(SystemClock) as Arc<dyn trackly_core::primitives::clock::Clock + Send + Sync>;
        let config = Arc::new(AppConfig::default());

        let svc = DashboardService::new(writer.clone(), readers, clock, config);

        // Test A: employee's ONLY request is the auto-created ad_register row.
        let (user_id, _request_id) =
            seed_employee_with_request(&writer, "us400", "Employee AD", "ad_register").await;
        let employee_identity = Identity {
            user_id: Some(user_id),
            role: Role::Employee,
        };

        let dto = svc
            .get_all_widgets(&employee_identity, None)
            .await
            .expect("get_all_widgets for employee (ad_register only)");
        assert_eq!(
            dto.request_counts_open, 0,
            "ad_register-only employee must see request_counts_open = 0"
        );
        assert_eq!(
            dto.request_counts_in_progress, 0,
            "ad_register-only employee must see request_counts_in_progress = 0"
        );
        assert_eq!(
            dto.request_counts_completed, 0,
            "ad_register-only employee must see request_counts_completed = 0"
        );

        // Test B (control): a REAL request for the same employee is still counted.
        writer
            .execute(move |conn| {
                let now = SystemClock.unix_seconds();
                conn.execute(
                    "INSERT INTO requests \
                     (request_type, status, requested_by_user_id, description, ad_subtype, \
                      created_at_utc, updated_at_utc, version) \
                     VALUES ('free_form', 'open', ?1, 'Нужен новый монитор', NULL, ?2, ?2, 1)",
                    params![user_id, now],
                )
                .map_err(|e| AppError::Internal {
                    source_chain: format!("{e}"),
                })?;
                Ok(())
            })
            .await
            .expect("seed control free_form request");

        let dto = svc
            .get_all_widgets(&employee_identity, None)
            .await
            .expect("get_all_widgets for employee (ad_register + free_form)");
        assert_eq!(
            dto.request_counts_open, 1,
            "employee's real free_form request must still be counted"
        );
    })
    .await
    .expect("dashboard_employee_widget_excludes_ad_register budget")
}

/// Seed one cartridge model, one compatibility row, and one full-stock
/// cartridge via raw SQL — mirrors `seed_employee_with_request`'s
/// `writer.execute(move |conn| { let tx = ...; })` shape. Returns model_id.
/// Fictional brand/model/printer names only (privacy constraint).
async fn seed_model_with_compat_and_stock(
    writer: &WriterHandle,
    brand: &str,
    model: &str,
    printer_name: &str,
    full_stock_count: i64,
) -> i64 {
    let now = SystemClock.unix_seconds();
    let brand = brand.to_string();
    let model = model.to_string();
    let printer_name = printer_name.to_string();
    writer
        .execute(move |conn| {
            let tx = conn.transaction().map_err(|e| AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            tx.execute(
                "INSERT INTO cartridge_models \
                 (brand, model, kind_id, created_at_utc, updated_at_utc, version) \
                 VALUES (?1, ?2, 1, ?3, ?3, 1)",
                params![brand, model, now],
            )
            .map_err(|e| AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            let model_id = tx.last_insert_rowid();
            tx.execute(
                "INSERT INTO cartridge_model_compatibility (cartridge_model_id, printer_name) \
                 VALUES (?1, ?2)",
                params![model_id, printer_name],
            )
            .map_err(|e| AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            for i in 0..full_stock_count {
                tx.execute(
                    "INSERT INTO cartridges \
                     (code, model_id, status_id, state_id, created_at_utc, updated_at_utc, version) \
                     VALUES (?1, ?2, 1, 1, ?3, ?3, 1)",
                    params![format!("C-{model_id}-{i}"), model_id, now],
                )
                .map_err(|e| AppError::Internal {
                    source_chain: format!("{e}"),
                })?;
            }
            tx.commit().map_err(|e| AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            Ok(model_id)
        })
        .await
        .expect("seed model with compat and stock")
}

/// Default basis (no app_settings.low_stock_basis row) must group by
/// `cartridge_model_compatibility.printer_name` in the dashboard widget too
/// — this is the direct proof that the repo's `low_stock()` and this
/// service's independent SQL copy do not diverge (quick task 260819-wq5).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn dashboard_low_stock_printer_model_default_matches_repo_grouping() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (writer, readers) = build_test_db();
        let clock =
            Arc::new(SystemClock) as Arc<dyn trackly_core::primitives::clock::Clock + Send + Sync>;
        let config = Arc::new(AppConfig::default());

        seed_model_with_compat_and_stock(&writer, "Fabrikam", "F-777", "Fabrikam LaserJet 200", 1)
            .await;

        let svc = DashboardService::new(writer, readers, clock, config);
        let dto = svc
            .get_all_widgets(&Identity::trusted_admin(), None)
            .await
            .expect("get_all_widgets");

        assert_eq!(dto.low_stock_count, 1);
        assert!(
            dto.low_stock_models
                .iter()
                .any(|m| m.to_lowercase().contains("fabrikam laserjet 200")),
            "default basis must group by printer name, got {:?}",
            dto.low_stock_models
        );
    })
    .await
    .expect("dashboard_low_stock_printer_model_default_matches_repo_grouping budget")
}

/// Explicit `cartridge_model` basis regression-locks the legacy per-model
/// grouping in the dashboard's independent SQL copy (quick task 260819-wq5).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn dashboard_low_stock_cartridge_model_basis_matches_legacy_grouping() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (writer, readers) = build_test_db();
        let clock =
            Arc::new(SystemClock) as Arc<dyn trackly_core::primitives::clock::Clock + Send + Sync>;
        let config = Arc::new(AppConfig::default());

        seed_model_with_compat_and_stock(&writer, "Fabrikam", "F-777", "Fabrikam LaserJet 200", 1)
            .await;
        writer
            .execute(|conn| {
                conn.execute(
                    "INSERT INTO app_settings (key, value, created_at_utc, updated_at_utc) \
                     VALUES ('low_stock_basis', 'cartridge_model', 0, 0)",
                    [],
                )
                .map(|_| ())
                .map_err(|e| AppError::Internal {
                    source_chain: format!("{e}"),
                })
            })
            .await
            .expect("seed cartridge_model basis");

        let svc = DashboardService::new(writer, readers, clock, config);
        let dto = svc
            .get_all_widgets(&Identity::trusted_admin(), None)
            .await
            .expect("get_all_widgets");

        assert_eq!(dto.low_stock_count, 1);
        assert!(
            dto.low_stock_models
                .iter()
                .any(|m| m.to_lowercase().contains("fabrikam f-777")),
            "cartridge_model basis must group by '{{brand}} {{model}}' label, got {:?}",
            dto.low_stock_models
        );
    })
    .await
    .expect("dashboard_low_stock_cartridge_model_basis_matches_legacy_grouping budget")
}
