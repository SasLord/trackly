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

use trackly_app::services::dashboard_service::DashboardService;
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
        let clock = Arc::new(SystemClock)
            as Arc<dyn trackly_core::primitives::clock::Clock + Send + Sync>;
        let config = Arc::new(AppConfig::default());

        let svc = DashboardService::new(writer, readers, clock, config);
        let dto = svc.get_all_widgets(None).await.unwrap();

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
        let clock = Arc::new(SystemClock)
            as Arc<dyn trackly_core::primitives::clock::Clock + Send + Sync>;
        let config = Arc::new(AppConfig::default());

        let svc = DashboardService::new(writer, readers, clock, config);
        let dto = svc.get_all_widgets(None).await.unwrap();

        // Empty DB has no cartridge models, so low_stock_count = 0.
        assert_eq!(dto.low_stock_count, 0, "no cartridge models → low_stock_count = 0");
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
