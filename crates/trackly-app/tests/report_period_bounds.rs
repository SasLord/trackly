// audit_log.action for cartridge install = 'custom:install' (verified in CartridgeTransitionOp::audit_action)
//
//! Period bounds UTC math test — Phase 7 Plan 03 (GREEN).
//!
//! Covers RPT-07: period selection math — "month" / "year" / "range" modes must
//! convert to correct UTC Unix-second bounds accounting for Europe/Moscow UTC+3.
//!
//! Moscow UTC+3 values verified with Python:
//!   datetime(2026, 6, 1, 0, 0, 0, tzinfo=timezone(timedelta(hours=3))).timestamp()
//!   => 1780261200
//!   datetime(2026, 6, 30, 23, 59, 59, tzinfo=timezone(timedelta(hours=3))).timestamp()
//!   => 1782853199

use trackly_app::dto::reports::PeriodDto;
use trackly_app::services::report_service::compute_period_utc;

fn moscow() -> time::UtcOffset {
    time::UtcOffset::from_hms(3, 0, 0).unwrap()
}

/// Verify that PeriodDto mode="month" resolves to correct UTC bounds for Moscow TZ.
///
/// 2026-06-01T00:00:00+03:00 = Unix 1780261200
/// 2026-06-30T23:59:59+03:00 = Unix 1782853199
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn period_month_to_utc_bounds_moscow() {
    let dto = PeriodDto {
        mode: "month".to_string(),
        year: Some(2026),
        month: Some(6),
        date_from: None,
        date_to: None,
    };
    let (start, end) = compute_period_utc(&dto, moscow());
    assert_eq!(
        start,
        Some(1_780_261_200_i64),
        "June 2026 Moscow start should be 1780261200"
    );
    assert_eq!(
        end,
        Some(1_782_853_199_i64),
        "June 2026 Moscow end should be 1782853199"
    );
}

/// Verify that PeriodDto mode="year" resolves to correct full-year UTC bounds.
///
/// 2026-01-01T00:00:00+03:00 = Unix 1767214800
/// 2026-12-31T23:59:59+03:00 = Unix 1798750799
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn period_year_to_utc_bounds() {
    let dto = PeriodDto {
        mode: "year".to_string(),
        year: Some(2026),
        month: None,
        date_from: None,
        date_to: None,
    };
    let (start, end) = compute_period_utc(&dto, moscow());
    assert_eq!(start, Some(1_767_214_800_i64), "2026 year Moscow start");
    assert_eq!(end, Some(1_798_750_799_i64), "2026 year Moscow end");
}

/// Verify that PeriodDto mode="range" with ISO date strings resolves to correct bounds.
///
/// 2026-06-01 00:00:00+03:00 = Unix 1780261200
/// 2026-06-30 23:59:59+03:00 = Unix 1782853199
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn period_range_to_utc_bounds() {
    let dto = PeriodDto {
        mode: "range".to_string(),
        year: None,
        month: None,
        date_from: Some("2026-06-01".to_string()),
        date_to: Some("2026-06-30".to_string()),
    };
    let (start, end) = compute_period_utc(&dto, moscow());
    assert_eq!(start, Some(1_780_261_200_i64), "range start");
    assert_eq!(end, Some(1_782_853_199_i64), "range end");
}
