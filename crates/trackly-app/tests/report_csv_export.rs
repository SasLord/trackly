// audit_log.action for cartridge install = 'custom:install' (verified in CartridgeTransitionOp::audit_action)
//
//! Report CSV export integration test — Phase 7 Plan 03 (GREEN).
//!
//! Covers RPT-03: CSV export must:
//!   - Start with UTF-8 BOM (EF BB BF) for Excel compatibility
//!   - Use semicolon (;) as delimiter (consistent with existing CSV export)
//!   - Guard against formula injection (cells starting with =, +, -, @)

use trackly_app::dto::reports::{ReportResponse, ReportRow};

/// Build a minimal ReportService CSV export using the public export_csv method
/// and verify the UTF-8 BOM prefix and semicolon delimiter.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn csv_export_has_utf8_bom_and_semicolon() {
    use std::sync::Arc;
    use trackly_app::pdf::PdfRenderer;
    use trackly_app::services::report_service::ReportService;
    use trackly_infra::clock_impl::SystemClock;
    use trackly_infra::{
        db::{pools::ReaderPool, writer_worker::WriterHandle},
        AppConfig,
    };

    // Build an in-memory SQLite DB for the service.
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let db_path = tmp.path().to_path_buf();

    // Open writer and run migrations.
    let mut conn = rusqlite::Connection::open(&db_path).unwrap();
    trackly_infra::db::pragmas::apply_writer_pragmas(&conn).unwrap();
    trackly_infra::db::migrations::run(&mut conn).unwrap();

    let writer = Arc::new(WriterHandle::spawn(conn));
    let readers = Arc::new(ReaderPool::new(&db_path, 2).unwrap());
    let clock =
        Arc::new(SystemClock) as Arc<dyn trackly_core::primitives::clock::Clock + Send + Sync>;
    let config = Arc::new(AppConfig::default());
    let pdf = Arc::new(PdfRenderer::new());

    let svc = ReportService::new(writer, readers, clock, config, pdf);

    // Create a sample ReportResponse with one row that has normal values.
    let response = ReportResponse {
        total: 1,
        rows: vec![ReportRow {
            id: 1,
            month_key: Some("2026-06".to_string()),
            number: Some("42".to_string()),
            sub_number: None,
            giver_name: Some("Иванов И.И.".to_string()),
            receiver_name: Some("Петров П.П.".to_string()),
            handover_date_utc: Some(1_748_800_000),
            place_path: Some("Склад 1".to_string()),
            place_path_short: Some("Склад 1".to_string()),
            act_type: Some("handover".to_string()),
            device_name: Some("Принтер HP".to_string()),
            quantity: Some(1),
            code: None,
            model_label: None,
            status_name: None,
            request_type_label: None,
        }],
    };
    let columns = &[
        "number",
        "giver_name",
        "receiver_name",
        "place_path",
        "device_name",
    ];
    let bytes = svc.export_csv(&response, columns).await.unwrap();

    // UTF-8 BOM must be the first 3 bytes: EF BB BF.
    assert!(
        bytes.starts_with(&[0xEF, 0xBB, 0xBF]),
        "CSV must start with UTF-8 BOM (EF BB BF)"
    );

    // Content after BOM must contain semicolons as delimiter.
    let body = std::str::from_utf8(&bytes[3..]).unwrap();
    assert!(
        body.contains(';'),
        "CSV must use semicolon delimiter, got: {body:?}"
    );

    // Header row must contain column names.
    assert!(body.contains("number"), "header row must contain 'number'");
}

/// Verify that cells starting with '=' are escaped to prevent formula injection.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn csv_export_guards_formula_injection() {
    use std::sync::Arc;
    use trackly_app::pdf::PdfRenderer;
    use trackly_app::services::report_service::ReportService;
    use trackly_infra::clock_impl::SystemClock;
    use trackly_infra::{
        db::{pools::ReaderPool, writer_worker::WriterHandle},
        AppConfig,
    };

    let tmp = tempfile::NamedTempFile::new().unwrap();
    let db_path = tmp.path().to_path_buf();
    let mut conn = rusqlite::Connection::open(&db_path).unwrap();
    trackly_infra::db::pragmas::apply_writer_pragmas(&conn).unwrap();
    trackly_infra::db::migrations::run(&mut conn).unwrap();

    let writer = Arc::new(WriterHandle::spawn(conn));
    let readers = Arc::new(ReaderPool::new(&db_path, 2).unwrap());
    let clock =
        Arc::new(SystemClock) as Arc<dyn trackly_core::primitives::clock::Clock + Send + Sync>;
    let config = Arc::new(AppConfig::default());
    let pdf = Arc::new(PdfRenderer::new());
    let svc = ReportService::new(writer, readers, clock, config, pdf);

    // Row with formula-injection payload in device_name.
    let response = ReportResponse {
        total: 1,
        rows: vec![ReportRow {
            id: 1,
            month_key: None,
            number: None,
            sub_number: None,
            giver_name: None,
            receiver_name: None,
            handover_date_utc: None,
            place_path: None,
            place_path_short: None,
            act_type: None,
            device_name: Some("=SUM(A1:A10)".to_string()),
            quantity: None,
            code: None,
            model_label: Some("+payload".to_string()),
            status_name: None,
            request_type_label: None,
        }],
    };
    let columns = &["device_name", "model_label"];
    let bytes = svc.export_csv(&response, columns).await.unwrap();
    let body = std::str::from_utf8(&bytes[3..]).unwrap(); // skip BOM

    // The formula injection payload must be escaped with a leading single-quote.
    assert!(
        body.contains("'=SUM(A1:A10)"),
        "formula starting with '=' must be escaped; got: {body:?}"
    );
    assert!(
        body.contains("'+payload"),
        "cell starting with '+' must be escaped; got: {body:?}"
    );
}
