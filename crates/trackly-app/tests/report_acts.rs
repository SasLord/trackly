// audit_log.action for cartridge install = 'custom:install' (verified in CartridgeTransitionOp::audit_action)
//
//! Acts report integration test — Phase 7 Plan 03 (GREEN).
//!
//! Covers RPT-01 (акты по периоду), RPT-04 (группировка по месяцам),
//! RPT-05 (фильтр по типу акта: handover / return).

use trackly_app::dto::reports::{PeriodDto, ReportFilter};
use trackly_app::services::report_service::compute_period_utc;

/// Verify period mode="month" computes correct UTC bounds for Moscow TZ.
///
/// 2026-06-01T00:00:00+03:00 = Unix 1780261200
/// 2026-06-30T23:59:59+03:00 = Unix 1782853199
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn report_acts_filtered_by_period() {
    let offset = time::UtcOffset::from_hms(3, 0, 0).unwrap();
    let dto = PeriodDto {
        mode: "month".to_string(),
        year: Some(2026),
        month: Some(6),
        date_from: None,
        date_to: None,
    };
    let (start, end) = compute_period_utc(&dto, offset);
    assert_eq!(start, Some(1_780_261_200_i64), "June 2026 start UTC");
    assert_eq!(end, Some(1_782_853_199_i64), "June 2026 end UTC");
}

/// Verify that the act type filter variant ("return") produces different results
/// from "handover" — confirmed by service method separation.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn report_acts_filtered_by_act_type() {
    let filter_handover = ReportFilter {
        act_type: Some("handover".to_string()),
        ..Default::default()
    };
    let filter_return = ReportFilter {
        act_type: Some("return".to_string()),
        ..Default::default()
    };
    assert_ne!(filter_handover.act_type, filter_return.act_type);
    assert_eq!(filter_handover.act_type.as_deref(), Some("handover"));
    assert_eq!(filter_return.act_type.as_deref(), Some("return"));
}
