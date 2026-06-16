// audit_log.action for cartridge install = 'custom:install' (verified in CartridgeTransitionOp::audit_action)
//
//! Period bounds UTC math test scaffold — Phase 7 Plan 01 (RED).
//!
//! Covers RPT-07: period selection math — "month" / "year" / "range" modes must
//! convert to correct UTC Unix-second bounds accounting for Europe/Moscow UTC+3.
//!
//! e.g. month "2026-06" in Moscow time starts at 2026-06-01T00:00:00+03:00 =
//!   2026-05-31T21:00:00Z  (Unix 1748725200).
//!
//! Implemented in plan 03 (period_to_utc_bounds helper function).

use std::time::Duration;

/// Verify that PeriodDto mode="month" resolves to correct UTC bounds for Moscow TZ.
///
/// RED: period_to_utc_bounds function does not exist yet.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn period_month_to_utc_bounds_moscow() {
    tokio::time::timeout(Duration::from_secs(30), async {
        todo!("implement in plan 03")
    })
    .await
    .expect("period_month_to_utc_bounds_moscow budget")
}

/// Verify that PeriodDto mode="year" resolves to correct full-year UTC bounds.
///
/// RED: period_to_utc_bounds function does not exist yet.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn period_year_to_utc_bounds() {
    tokio::time::timeout(Duration::from_secs(30), async {
        todo!("implement in plan 03")
    })
    .await
    .expect("period_year_to_utc_bounds budget")
}

/// Verify that PeriodDto mode="range" with ISO date strings resolves to correct bounds.
///
/// RED: period_to_utc_bounds function does not exist yet.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn period_range_to_utc_bounds() {
    tokio::time::timeout(Duration::from_secs(30), async {
        todo!("implement in plan 03")
    })
    .await
    .expect("period_range_to_utc_bounds budget")
}
