// audit_log.action for cartridge install = 'custom:install' (verified in CartridgeTransitionOp::audit_action)
//
//! Cartridge consumption report integration test scaffold — Phase 7 Plan 01 (RED).
//!
//! Covers RPT-06 (cartridges report — consumption grouped by month).
//! SQL query groups audit_log entries WHERE action = 'custom:install'
//! by strftime('%Y-%m', datetime(created_at_utc, 'unixepoch')) and cartridge model.
//!
//! Implemented in plan 03 (ReportService::query_cartridge_consumption).

use std::time::Duration;

/// Verify that cartridge consumption events are grouped correctly by month.
///
/// Each row in the result corresponds to one model in one month.
/// action = 'custom:install' is the canonical audit_log action for a cartridge install.
///
/// RED: ReportService does not exist yet — todo!() causes runtime failure.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn report_cartridges_consumption_grouped_by_month() {
    tokio::time::timeout(Duration::from_secs(30), async {
        todo!("implement in plan 03")
    })
    .await
    .expect("report_cartridges_consumption_grouped_by_month budget")
}

/// Verify that color filter narrows consumption report to matching models only.
///
/// RED: ReportService does not exist yet — todo!() causes runtime failure.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn report_cartridges_filtered_by_color() {
    tokio::time::timeout(Duration::from_secs(30), async {
        todo!("implement in plan 03")
    })
    .await
    .expect("report_cartridges_filtered_by_color budget")
}
