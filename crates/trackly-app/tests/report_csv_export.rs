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
            from_place_path: None,
            from_place_path_short: None,
            actor_name: None,
            reason: None,
            entity_type_label: None,
            is_deleted: None,
            batch_id: None,
            batch_role: None,
            batch_size: None,
            batch_shown: None,
            batch_label: None,
            repair_device_label: None,
            repair_fault: None,
            repair_service_place: None,
            repair_sent_at: None,
            repair_returned_at: None,
            repair_works: None,
            repair_cost_kopecks: None,
            repair_has_scan: None,
            repair_request_id: None,
        }],
    };
    let columns = &[
        "number",
        "giver_name",
        "receiver_name",
        "place_path",
        "device_name",
    ];
    // WARNING-4 (audit v1.4, 2026-09-17, D-19): header row is written from
    // column_labels, not from the raw column keys — mirrors column_labels_for's
    // Russian labels for the same keys elsewhere in the report pipeline.
    let column_labels = &["Номер", "Сдал", "Принял", "Место", "Устройство"];
    let bytes = svc
        .export_csv(&response, columns, column_labels)
        .await
        .unwrap();

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

    // Header row must contain the Russian label 'Номер' (D-19 — labels, not raw keys).
    assert!(
        body.contains("Номер"),
        "header row must contain the Russian label 'Номер' (D-19 — labels, not raw keys)"
    );
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
            from_place_path: None,
            from_place_path_short: None,
            actor_name: None,
            reason: None,
            entity_type_label: None,
            is_deleted: None,
            batch_id: None,
            batch_role: None,
            batch_size: None,
            batch_shown: None,
            batch_label: None,
            repair_device_label: None,
            repair_fault: None,
            repair_service_place: None,
            repair_sent_at: None,
            repair_returned_at: None,
            repair_works: None,
            repair_cost_kopecks: None,
            repair_has_scan: None,
            repair_request_id: None,
        }],
    };
    let columns = &["device_name", "model_label"];
    // This test only checks cell-value escaping, not the header row — labels
    // are functionally irrelevant here, so they simply repeat the keys.
    let column_labels = columns;
    let bytes = svc
        .export_csv(&response, columns, column_labels)
        .await
        .unwrap();
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

// ---------------------------------------------------------------------------
// quick-261121-hef (INT-06): CSV «Перемещений» несёт колонку «Пакет» — имя
// группы в каждой строке пакета, пусто у обычного перемещения. Сквозной путь
// через `build_reports_export_csv` (колонки — из `columns_for`).
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn csv_export_movements_has_batch_column_for_group_transfer() {
    use trackly_app::context::AppCtx;
    use trackly_app::dto::reports::{PeriodDto, ReportFilter};
    use trackly_app::tauri_cmds::reports::build_reports_export_csv;
    use trackly_core::auth::Identity;
    use trackly_infra::error_conversions::map_rusqlite;

    let dir = tempfile::TempDir::new().expect("tempdir");
    let paths = trackly_infra::Paths::resolve_for_exe_dir(dir.path().to_path_buf()).expect("paths");
    let config = trackly_infra::AppConfig::default();
    let (_nb, guard) = tracing_appender::non_blocking(std::io::sink());
    let ctx = AppCtx::build(paths, config, guard).await.expect("ctx");

    ctx.writer
        .execute(|conn| {
            conn.execute_batch(
                "INSERT INTO places (id, kind, name, parent_id, is_storage, created_at_utc, \
                   updated_at_utc, version) \
                 VALUES (901, 'room', 'Склад А', NULL, 0, 1700000000, 1700000000, 1), \
                        (902, 'room', 'Склад Б', NULL, 0, 1700000000, 1700000000, 1); \
                 INSERT INTO devices (id, type_id, name, inventory_number, place_id, status_id, \
                   created_at_utc, updated_at_utc, version) \
                 VALUES (801, 1, 'Монитор одиночный', 'INV-H-1', 901, 1, 1700000000, 1700000000, 1), \
                        (802, 1, 'Системный блок стенда', 'INV-H-2', 902, 1, 1700000000, 1700000000, 1), \
                        (803, 1, 'Монитор стенда', 'INV-H-3', 902, 1, 1700000000, 1700000000, 1); \
                 INSERT INTO place_movements (entity_type, entity_id, from_place_id, \
                   from_place_path, to_place_id, to_place_path, source, created_at_utc, \
                   batch_id, entity_label) \
                 VALUES ('device', 801, 901, 'Склад А', 902, 'Склад Б', 'manual', 1800000000, NULL, NULL), \
                        ('group', 1, 901, 'Склад А', 902, 'Склад Б', 'manual', 1800000100, 'hef-b', 'Стенд А'), \
                        ('device', 802, 901, 'Склад А', 902, 'Склад Б', 'manual', 1800000101, 'hef-b', 'Стенд А'), \
                        ('device', 803, 901, 'Склад А', 902, 'Склад Б', 'manual', 1800000102, 'hef-b', 'Стенд А');",
            )
            .map_err(map_rusqlite)
        })
        .await
        .expect("seed");

    let bytes = build_reports_export_csv(
        &ctx,
        &Identity::trusted_admin(),
        "movements".to_string(),
        ReportFilter::default(),
        Some(PeriodDto {
            mode: "range".to_string(),
            year: None,
            month: None,
            date_from: Some("2000-01-01".to_string()),
            date_to: Some("2099-12-31".to_string()),
        }),
    )
    .await
    .expect("csv");
    let text = String::from_utf8(bytes).expect("utf8");
    let mut lines = text
        .trim_start_matches('\u{FEFF}')
        .lines()
        .filter(|l| !l.is_empty());
    let header: Vec<&str> = lines.next().unwrap().split(';').collect();
    let batch_idx = header
        .iter()
        .position(|h| *h == "Пакет")
        .unwrap_or_else(|| panic!("нет колонки «Пакет»: {header:?}"));
    assert_eq!(batch_idx, header.len() - 1, "«Пакет» — последняя колонка");

    let rows: Vec<Vec<&str>> = lines.map(|l| l.split(';').collect()).collect();
    assert_eq!(rows.len(), 4);
    let cell = |needle: &str| -> &str {
        rows.iter()
            .find(|r| r.iter().any(|c| c.contains(needle)))
            .unwrap_or_else(|| panic!("нет строки {needle}"))[batch_idx]
    };
    assert_eq!(cell("Монитор одиночный"), "", "обычное перемещение");
    for name in ["Системный блок стенда", "Монитор стенда"] {
        assert_eq!(
            cell(name),
            "Перенос группы «Стенд А» (2 устройства)",
            "{name}"
        );
    }
    let batch_cells = rows
        .iter()
        .filter(|r| r[batch_idx].contains("Стенд А"))
        .count();
    assert_eq!(batch_cells, 3, "строка группы тоже несёт ячейку пакета");
}
