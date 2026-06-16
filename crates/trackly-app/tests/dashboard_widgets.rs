// audit_log.action for cartridge install = 'custom:install' (verified in CartridgeTransitionOp::audit_action)
//
//! Dashboard widget counts integration test scaffold — Phase 7 Plan 01 (RED).
//!
//! Covers DASH-01..05:
//!   - DASH-01: devices total + by-status breakdown
//!   - DASH-02: cartridges by-status breakdown + low-stock count
//!   - DASH-03: consumption chart (ConsumptionPoint list) — see also report_cartridges.rs
//!   - DASH-04: request counts open / in_progress / completed
//!   - DASH-05: printer online / offline / problematic
//!
//! Implemented in plan 02 (DashboardService::get_widget_data).

use std::time::Duration;

/// Verify that DashboardWidgetDto is populated with correct aggregate counts.
///
/// RED: DashboardService does not exist yet — todo!() causes runtime failure.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn dashboard_widget_counts_match_db_state() {
    tokio::time::timeout(Duration::from_secs(30), async {
        todo!("implement in plan 02")
    })
    .await
    .expect("dashboard_widget_counts_match_db_state budget")
}

/// Verify that low_stock_count and low_stock_models reflect cartridge stock state.
///
/// RED: DashboardService does not exist yet — todo!() causes runtime failure.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn dashboard_low_stock_reflects_cartridge_state() {
    tokio::time::timeout(Duration::from_secs(30), async {
        todo!("implement in plan 02")
    })
    .await
    .expect("dashboard_low_stock_reflects_cartridge_state budget")
}
