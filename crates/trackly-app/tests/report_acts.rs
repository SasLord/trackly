// audit_log.action for cartridge install = 'custom:install' (verified in CartridgeTransitionOp::audit_action)
//
//! Acts report integration test scaffold — Phase 7 Plan 01 (RED).
//!
//! Covers RPT-01 (акты по периоду), RPT-04 (группировка по месяцам),
//! RPT-05 (фильтр по типу акта: handover / return).
//!
//! Implemented in plan 03 (ReportService::query_acts).

use std::time::Duration;

/// Verify that acts report can be filtered by a date range and groups rows by month.
///
/// RED: ReportService does not exist yet — todo!() causes runtime failure.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn report_acts_filtered_by_period() {
    tokio::time::timeout(Duration::from_secs(30), async {
        todo!("implement in plan 03")
    })
    .await
    .expect("report_acts_filtered_by_period budget")
}

/// Verify that the act type filter ("handover" vs "return") narrows results correctly.
///
/// RED: ReportService does not exist yet — todo!() causes runtime failure.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn report_acts_filtered_by_act_type() {
    tokio::time::timeout(Duration::from_secs(30), async {
        todo!("implement in plan 03")
    })
    .await
    .expect("report_acts_filtered_by_act_type budget")
}
