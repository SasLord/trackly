// audit_log.action for cartridge install = 'custom:install' (verified in CartridgeTransitionOp::audit_action)
//
//! Report CSV export integration test scaffold — Phase 7 Plan 01 (RED).
//!
//! Covers RPT-03: CSV export must:
//!   - Start with UTF-8 BOM (EF BB BF) for Excel compatibility
//!   - Use semicolon (;) as delimiter (consistent with existing CSV export in plan 05)
//!   - Use Russian column headers
//!   - Guard against formula injection (cells starting with =, +, -, @)
//!
//! Implemented in plan 04 (ReportService::export_csv or export_acts_csv / export_cartridges_csv).

use std::time::Duration;

/// Verify that exported CSV starts with UTF-8 BOM and uses semicolon delimiter.
///
/// RED: export_csv function does not exist yet.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn csv_export_has_utf8_bom_and_semicolon() {
    tokio::time::timeout(Duration::from_secs(30), async {
        todo!("implement in plan 04")
    })
    .await
    .expect("csv_export_has_utf8_bom_and_semicolon budget")
}

/// Verify that cells starting with '=' are escaped to prevent formula injection.
///
/// RED: export_csv function does not exist yet.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn csv_export_guards_formula_injection() {
    tokio::time::timeout(Duration::from_secs(30), async {
        todo!("implement in plan 04")
    })
    .await
    .expect("csv_export_guards_formula_injection budget")
}
