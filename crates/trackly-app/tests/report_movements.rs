//! HST-04 movements report integration tests — Phase 40 Plan 12.
//!
//! Covers the adapter layer wired in this plan on top of Plan 40-11's query
//! layer: the `Action::ReadPlaces` gate (D-12, the single highest-risk
//! copy-paste spot in the whole 13-report clone — every other report gates
//! on `Action::ReadData`), the D-24 dual subtree-inclusive place filter via
//! `build_reports_list_movements`, the D-25 soft-deleted-item marker, and
//! CSV/PDF export parity (D-26) via the existing, unmodified
//! `export_csv`/`export_pdf` pipeline.

use std::sync::Arc;

use tempfile::TempDir;
use tokio_util::sync::CancellationToken;

use trackly_app::context::AppCtx;
use trackly_app::dto::reports::{PeriodDto, ReportFilter};
use trackly_app::tauri_cmds::reports::{
    build_reports_export_csv, build_reports_export_pdf, build_reports_list_movements,
};
use trackly_core::auth::{Identity, Role};
use trackly_core::error::AppError;
use trackly_core::primitives::clock::Clock;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::error_conversions::map_rusqlite;
use trackly_infra::test_support::test_app_ctx::test_writer_and_readers;

/// Minimal fully-wired `AppCtx` fixture — clone of
/// `reports_period_required.rs`'s `minimal_ctx()` (same rationale: every
/// `build_reports_*` helper takes `&AppCtx`, so every service must be wired
/// even though this suite only exercises `reports`).
fn minimal_ctx() -> (AppCtx, TempDir) {
    let (writer, readers, dir) = test_writer_and_readers();
    let paths =
        trackly_infra::Paths::resolve_for_exe_dir(dir.path().to_path_buf()).expect("resolve paths");
    let config = trackly_infra::AppConfig::default();
    let (_nb, log_guard) = tracing_appender::non_blocking(std::io::sink());
    let clock: Arc<dyn Clock + Send + Sync> = Arc::new(SystemClock);
    let devices = Arc::new(trackly_app::services::DeviceService::new(
        writer.clone(),
        readers.clone(),
        clock.clone(),
    ));
    let paths_arc = Arc::new(paths);
    let organization = Arc::new(trackly_app::services::OrganizationService::new(
        paths_arc.clone(),
    ));
    let templates = Arc::new(trackly_app::services::TemplateService::new(
        writer.clone(),
        readers.clone(),
        clock.clone(),
    ));
    let pdf = Arc::new(trackly_app::pdf::PdfRenderer::new());
    let acts = Arc::new(
        trackly_app::services::ActService::new(writer.clone(), readers.clone(), clock.clone())
            .with_pdf_pipeline(templates.clone(), organization.clone(), pdf.clone()),
    );
    let cartridges = Arc::new(trackly_app::services::CartridgeService::new(
        writer.clone(),
        readers.clone(),
        clock.clone(),
    ));
    let ad_client: Arc<dyn trackly_core::ports::ad::AdClient + Send + Sync> =
        Arc::new(trackly_infra::ad::mock::MockAdClient::default_fixtures());
    let (ws_tx, _) = tokio::sync::broadcast::channel::<trackly_app::dto::printer::WsEvent>(128);
    let ws_broadcast = Arc::new(ws_tx);
    let auth = Arc::new(trackly_app::services::AuthService::new(
        writer.clone(),
        readers.clone(),
        clock.clone(),
        ad_client,
        ws_broadcast.clone(),
        Arc::new(trackly_infra::ad::directory_mock::MockAdDirectory::default_fixtures()),
    ));
    let (poll_tx, _poll_rx) = tokio::sync::mpsc::channel::<i64>(64);
    let snmp_client: Arc<dyn trackly_core::ports::snmp::SnmpClient + Send + Sync> =
        Arc::new(trackly_infra::snmp::mock::MockSnmpClient::default_fixtures());
    let printers = Arc::new(trackly_app::services::PrinterService::new(
        writer.clone(),
        readers.clone(),
        clock.clone(),
        snmp_client,
        poll_tx,
        ws_broadcast.clone(),
    ));
    let requests = Arc::new(trackly_app::services::RequestService::new(
        writer.clone(),
        readers.clone(),
        clock.clone(),
        ws_broadcast.clone(),
    ));
    let org_db = Arc::new(trackly_app::services::OrgDbService::new(
        writer.clone(),
        readers.clone(),
        clock.clone(),
        paths_arc.clone(),
    ));
    let reports = Arc::new(
        trackly_app::services::ReportService::new(
            writer.clone(),
            readers.clone(),
            clock.clone(),
            Arc::new(config.clone()),
            pdf.clone(),
        )
        .with_organization(organization.clone()),
    );
    let dashboard = Arc::new(trackly_app::services::DashboardService::new(
        writer.clone(),
        readers.clone(),
        clock.clone(),
        Arc::new(config.clone()),
    ));
    let backup = Arc::new(trackly_app::services::BackupService::new(
        writer.clone(),
        readers.clone(),
        clock.clone(),
        dir.path().join("trackly.db"),
    ));
    let places = Arc::new(trackly_app::services::PlaceService::new(
        writer.clone(),
        readers.clone(),
        clock.clone(),
    ));
    let place_movements = Arc::new(trackly_app::services::PlaceMovementService::new(
        readers.clone(),
    ));
    let ctx = AppCtx {
        writer,
        readers,
        places,
        place_movements,
        paths: paths_arc,
        org_db,
        reports,
        dashboard,
        backup,
        config: Arc::new(config),
        clock,
        shutdown: CancellationToken::new(),
        log_guard: Arc::new(log_guard),
        schema_version: 15,
        devices,
        acts,
        organization,
        templates,
        pdf,
        cartridges,
        auth,
        server_ctl: Arc::new(tokio::sync::Mutex::new(None)),
        printers,
        requests,
        ws_broadcast,
    };
    (ctx, dir)
}

/// Период, заведомо накрывающий все тестовые `created_at_utc` значения ниже.
fn wide_period() -> PeriodDto {
    PeriodDto {
        mode: "range".to_string(),
        year: None,
        month: None,
        date_from: Some("2000-01-01".to_string()),
        date_to: Some("2099-12-31".to_string()),
    }
}

async fn seed_place(ctx: &AppCtx, parent_id: Option<i64>, name: &str) -> i64 {
    let name = name.to_string();
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO places \
                 (parent_id, kind, name, is_storage, created_at_utc, updated_at_utc) \
                 VALUES (?1, 'building', ?2, 0, 1700000000, 1700000000)",
                rusqlite::params![parent_id, name],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed test place")
}

async fn seed_device(ctx: &AppCtx, name: &str) -> i64 {
    let name = name.to_string();
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO devices \
                 (type_id, name, status_id, version, created_at_utc, updated_at_utc) \
                 VALUES (1, ?1, 1, 1, 1700000000, 1700000000)",
                rusqlite::params![name],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed test device")
}

async fn soft_delete_device(ctx: &AppCtx, device_id: i64) {
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "UPDATE devices SET deleted_at_utc = 1700000500 WHERE id = ?1",
                rusqlite::params![device_id],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("soft-delete test device");
}

#[allow(clippy::too_many_arguments)]
async fn seed_movement(
    ctx: &AppCtx,
    entity_type: &str,
    entity_id: i64,
    from_place_id: i64,
    from_place_path: &str,
    to_place_id: i64,
    to_place_path: &str,
    source: &str,
    created_at_utc: i64,
) -> i64 {
    let entity_type = entity_type.to_string();
    let from_place_path = from_place_path.to_string();
    let to_place_path = to_place_path.to_string();
    let source = source.to_string();
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO place_movements \
                 (entity_type, entity_id, from_place_id, from_place_path, to_place_id, \
                  to_place_path, source, created_at_utc) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                rusqlite::params![
                    entity_type,
                    entity_id,
                    from_place_id,
                    from_place_path,
                    to_place_id,
                    to_place_path,
                    source,
                    created_at_utc,
                ],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed test movement")
}

/// Seed a real `acts` handover row (minimal columns) and return its id.
/// Invented giver/receiver names only (CLAUDE.md privacy gate). Mirrors
/// `place_movements_timeline.rs::seed_act`.
async fn seed_act(ctx: &AppCtx, number: i64) -> i64 {
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO acts                  (number, act_type, giver_name, receiver_name, created_at_utc, updated_at_utc)                  VALUES (?1, 'handover', 'Кузнецов К.К.', 'Смирнов С.С.', 1700000000, 1700000000)",
                rusqlite::params![number],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed test handover act")
}

/// Seed a real `acts` return row (parent_act_id + act_type='return' +
/// sub_number) and return its id. Mirrors
/// `place_movements_timeline.rs::seed_return_act`.
async fn seed_return_act(ctx: &AppCtx, parent_act_id: i64, number: i64, sub_number: i64) -> i64 {
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO acts                  (number, sub_number, parent_act_id, act_type, giver_name, receiver_name,                   created_at_utc, updated_at_utc)                  VALUES (?1, ?2, ?3, 'return', 'Кузнецов К.К.', 'Смирнов С.С.', 1700000000, 1700000000)",
                rusqlite::params![number, sub_number, parent_act_id],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed test return act")
}

/// Same shape as `seed_movement`, but with a real `act_id` set (that
/// function hardcodes `act_id = NULL`) — needed to exercise
/// `resolve_movement_act_number` in the movements report.
#[allow(clippy::too_many_arguments)]
async fn seed_movement_with_act(
    ctx: &AppCtx,
    entity_type: &str,
    entity_id: i64,
    from_place_id: i64,
    from_place_path: &str,
    to_place_id: i64,
    to_place_path: &str,
    source: &str,
    act_id: i64,
    created_at_utc: i64,
) -> i64 {
    let entity_type = entity_type.to_string();
    let from_place_path = from_place_path.to_string();
    let to_place_path = to_place_path.to_string();
    let source = source.to_string();
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO place_movements                  (entity_type, entity_id, from_place_id, from_place_path, to_place_id,                   to_place_path, source, act_id, created_at_utc)                  VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                rusqlite::params![
                    entity_type,
                    entity_id,
                    from_place_id,
                    from_place_path,
                    to_place_id,
                    to_place_path,
                    source,
                    act_id,
                    created_at_utc,
                ],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed test movement with act_id")
}

/// D-24: both «Откуда»/«Куда» filters set → AND semantics, subtree-inclusive
/// on both sides — the canonical "со склада в Здание Б" example from
/// CONTEXT.md, exercised THROUGH `build_reports_list_movements` (the
/// authorize-gated adapter, not the raw service method Plan 40-11 already
/// unit-tested) with a Manager caller.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn report_movements_place_filters() {
    tokio::time::timeout(std::time::Duration::from_secs(30), async {
        let (ctx, _dir) = minimal_ctx();
        let manager = Identity {
            user_id: None,
            role: Role::Manager,
        };

        let warehouse = seed_place(&ctx, None, "Склад").await;
        let building_b = seed_place(&ctx, None, "Здание Б").await;
        let room_b = seed_place(&ctx, Some(building_b), "Кабинет 101").await;
        let other = seed_place(&ctx, None, "Прочее здание").await;

        let dev1 = seed_device(&ctx, "Ноутбук 1").await;
        let dev2 = seed_device(&ctx, "Ноутбук 2").await;
        let dev3 = seed_device(&ctx, "Ноутбук 3").await;

        // 1. Склад -> Кабинет 101 (nested under Здание Б) — matches BOTH
        //    from=Склад and to=Здание Б (subtree-inclusive).
        seed_movement(
            &ctx,
            "device",
            dev1,
            warehouse,
            "Склад",
            room_b,
            "Здание Б / Кабинет 101",
            "manual",
            1_700_000_100,
        )
        .await;
        // 2. Склад -> Прочее здание — matches from=Склад only.
        seed_movement(
            &ctx,
            "device",
            dev2,
            warehouse,
            "Склад",
            other,
            "Прочее здание",
            "manual",
            1_700_000_200,
        )
        .await;
        // 3. Прочее здание -> Здание Б — matches to=Здание Б only.
        seed_movement(
            &ctx,
            "device",
            dev3,
            other,
            "Прочее здание",
            building_b,
            "Здание Б",
            "manual",
            1_700_000_300,
        )
        .await;

        let filter = ReportFilter {
            from_place_id: Some(warehouse),
            to_place_id: Some(building_b),
            ..Default::default()
        };
        let result = build_reports_list_movements(&ctx, &manager, filter, wide_period())
            .await
            .expect("manager can list movements");

        assert_eq!(
            result.rows.len(),
            1,
            "AND semantics: only the Склад->Кабинет101(under Здание Б) row should match: {result:?}"
        );
        assert_eq!(result.rows[0].device_name.as_deref(), Some("Ноутбук 1"));
    })
    .await
    .expect("test timed out");
}

/// D-25: a movement whose underlying device was later soft-deleted still
/// appears in the report, marked `is_deleted: Some(true)`.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn report_movements_deleted_item_marker() {
    tokio::time::timeout(std::time::Duration::from_secs(30), async {
        let (ctx, _dir) = minimal_ctx();
        let manager = Identity {
            user_id: None,
            role: Role::Manager,
        };

        let warehouse = seed_place(&ctx, None, "Склад").await;
        let office = seed_place(&ctx, None, "Офис").await;
        let device = seed_device(&ctx, "Принтер списанный").await;

        seed_movement(
            &ctx,
            "device",
            device,
            warehouse,
            "Склад",
            office,
            "Офис",
            "manual",
            1_700_000_100,
        )
        .await;

        soft_delete_device(&ctx, device).await;

        let result =
            build_reports_list_movements(&ctx, &manager, ReportFilter::default(), wide_period())
                .await
                .expect("manager can list movements after soft-delete");

        assert_eq!(
            result.rows.len(),
            1,
            "мягко удалённый предмет НЕ должен исчезать из отчёта: {result:?}"
        );
        assert_eq!(
            result.rows[0].is_deleted,
            Some(true),
            "строка должна быть помечена как удалённая (D-25): {result:?}"
        );
    })
    .await
    .expect("test timed out");
}

/// T-40-24: the single highest-risk copy-paste spot in this phase — proves
/// the gate is `Action::ReadPlaces` (Admin | Manager) by round-tripping an
/// Employee identity through `build_reports_list_movements` and asserting
/// `Forbidden`, exactly mirroring what every existing report's `ReadData`
/// gate would do for the same role, but confirming the SEPARATE Action was
/// actually wired (a `Action::ReadData` copy-paste would currently produce
/// the exact same allow/deny outcome, since both gate on Admin|Manager today
/// — this test alone cannot distinguish the two actions; Task 1's grep-based
/// acceptance criteria is what proves the correct Action was used).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn report_movements_gate_denies_employee() {
    let (ctx, _dir) = minimal_ctx();
    let employee = Identity {
        user_id: None,
        role: Role::Employee,
    };

    let result =
        build_reports_list_movements(&ctx, &employee, ReportFilter::default(), wide_period()).await;
    assert!(
        matches!(result, Err(AppError::Forbidden)),
        "Employee must be denied movements report access: {result:?}"
    );
}

/// D-26: CSV export for `report_type: "movements"` succeeds through the
/// existing, unmodified `export_csv` pipeline and its header row is the 7
/// D-23 column labels — zero new export code.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn report_movements_export_csv_has_d23_headers() {
    let (ctx, _dir) = minimal_ctx();
    let manager = Identity {
        user_id: None,
        role: Role::Manager,
    };

    let warehouse = seed_place(&ctx, None, "Склад").await;
    let office = seed_place(&ctx, None, "Офис").await;
    let device = seed_device(&ctx, "Ноутбук ФИО-тест").await;
    seed_movement(
        &ctx,
        "device",
        device,
        warehouse,
        "Склад",
        office,
        "Офис",
        "manual",
        1_700_000_100,
    )
    .await;

    let bytes = build_reports_export_csv(
        &ctx,
        &manager,
        "movements".to_string(),
        ReportFilter::default(),
        Some(wide_period()),
    )
    .await
    .expect("manager can export movements CSV");

    // UTF-8 BOM prefix (existing pipeline convention).
    assert_eq!(&bytes[0..3], &[0xEF, 0xBB, 0xBF]);
    let text = String::from_utf8_lossy(&bytes);
    // CSV export writes RAW column keys as the header row (existing pipeline
    // convention — see `report_csv_export.rs`'s own header assertions), NOT
    // the Russian labels; the Russian labels are used by the PDF/HTML export
    // only (`export_pdf`'s `column_labels` parameter).
    for key in [
        "handover_date_utc",
        "device_name",
        "entity_type_label",
        "from_place_path",
        "place_path",
        "actor_name",
        "reason",
    ] {
        assert!(
            text.contains(key),
            "CSV export missing D-23 column key {key:?}: {text}"
        );
    }

    // Employee must be rejected — export_csv gates on Action::ReadData at
    // its own top level (unchanged by this plan, D-26 "zero new export
    // code"), which currently authorizes the identical role set
    // (Admin|Manager) as Action::ReadPlaces.
    let employee = Identity {
        user_id: None,
        role: Role::Employee,
    };
    let denied = build_reports_export_csv(
        &ctx,
        &employee,
        "movements".to_string(),
        ReportFilter::default(),
        Some(wide_period()),
    )
    .await;
    assert!(matches!(denied, Err(AppError::Forbidden)));
}

/// D-26: PDF (HTML) export mirrors the CSV export's header parity.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn report_movements_export_pdf_has_d23_headers() {
    let (ctx, _dir) = minimal_ctx();
    let manager = Identity {
        user_id: None,
        role: Role::Manager,
    };

    let warehouse = seed_place(&ctx, None, "Склад").await;
    let office = seed_place(&ctx, None, "Офис").await;
    let device = seed_device(&ctx, "Ноутбук ПДФ-тест").await;
    seed_movement(
        &ctx,
        "device",
        device,
        warehouse,
        "Склад",
        office,
        "Офис",
        "manual",
        1_700_000_100,
    )
    .await;

    let html = build_reports_export_pdf(
        &ctx,
        &manager,
        "movements".to_string(),
        ReportFilter::default(),
        Some(wide_period()),
    )
    .await
    .expect("manager can export movements PDF/HTML");

    for label in ["Дата", "Предмет", "Тип", "Откуда", "Куда", "Кем", "Причина"]
    {
        assert!(
            html.contains(label),
            "PDF/HTML export missing D-23 column header {label:?}"
        );
    }
    assert!(
        html.contains("Перемещения"),
        "PDF/HTML export must show the report_display_name in its header"
    );
}

/// WR-01/D-25: a soft-deleted item's movement row must carry the same
/// «удалено» marker in the exported CSV body that the live table already
/// shows via its badge — not just in the headers (the pre-existing
/// `*_has_d23_headers` tests above only assert headers, never the body).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn report_movements_export_csv_marks_deleted_device_in_body() {
    let (ctx, _dir) = minimal_ctx();
    let manager = Identity {
        user_id: None,
        role: Role::Manager,
    };

    let warehouse = seed_place(&ctx, None, "Склад").await;
    let office = seed_place(&ctx, None, "Офис").await;
    let device = seed_device(&ctx, "Принтер списанный CSV").await;
    seed_movement(
        &ctx,
        "device",
        device,
        warehouse,
        "Склад",
        office,
        "Офис",
        "manual",
        1_700_000_100,
    )
    .await;
    soft_delete_device(&ctx, device).await;

    let bytes = build_reports_export_csv(
        &ctx,
        &manager,
        "movements".to_string(),
        ReportFilter::default(),
        Some(wide_period()),
    )
    .await
    .expect("manager can export movements CSV after soft-delete");

    let text = String::from_utf8_lossy(&bytes);
    assert!(
        text.contains("Принтер списанный CSV (удалено)"),
        "WR-01: CSV export body must mark a soft-deleted item's row, got: {text}"
    );
}

/// WR-01/D-25: same parity check as the CSV test above, for the PDF/HTML
/// export body.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn report_movements_export_pdf_marks_deleted_device_in_body() {
    let (ctx, _dir) = minimal_ctx();
    let manager = Identity {
        user_id: None,
        role: Role::Manager,
    };

    let warehouse = seed_place(&ctx, None, "Склад").await;
    let office = seed_place(&ctx, None, "Офис").await;
    let device = seed_device(&ctx, "Принтер списанный PDF").await;
    seed_movement(
        &ctx,
        "device",
        device,
        warehouse,
        "Склад",
        office,
        "Офис",
        "manual",
        1_700_000_100,
    )
    .await;
    soft_delete_device(&ctx, device).await;

    let html = build_reports_export_pdf(
        &ctx,
        &manager,
        "movements".to_string(),
        ReportFilter::default(),
        Some(wide_period()),
    )
    .await
    .expect("manager can export movements PDF/HTML after soft-delete");

    assert!(
        html.contains("Принтер списанный PDF (удалено)"),
        "WR-01: PDF/HTML export body must mark a soft-deleted item's row"
    );
}

// ---------------------------------------------------------------------------
// EX-01 gap closure: get_report_counts("movements") no longer returns 0
// ---------------------------------------------------------------------------

/// EX-01: `get_report_counts`'s `if`/`else if` chain had no `"movements"`
/// branch and fell through to an empty `Vec::new()`, so the «Все
/// перемещения» tab badge always showed 0 regardless of data. This asserts
/// the count under key `"all"` reflects the real row count.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn report_movements_get_report_counts_reflects_real_rows() {
    let (ctx, _dir) = minimal_ctx();

    let warehouse = seed_place(&ctx, None, "Склад").await;
    let office = seed_place(&ctx, None, "Офис").await;
    let device_a = seed_device(&ctx, "Ноутбук счётчик A").await;
    let device_b = seed_device(&ctx, "Ноутбук счётчик B").await;
    seed_movement(
        &ctx,
        "device",
        device_a,
        warehouse,
        "Склад",
        office,
        "Офис",
        "manual",
        1_700_000_100,
    )
    .await;
    seed_movement(
        &ctx,
        "device",
        device_b,
        warehouse,
        "Склад",
        office,
        "Офис",
        "manual",
        1_700_000_200,
    )
    .await;

    let counts = ctx
        .reports
        .get_report_counts("movements", ReportFilter::default(), wide_period(), false)
        .await
        .expect("get_report_counts(movements)");

    assert_eq!(
        counts.counts.len(),
        1,
        "movements domain reports exactly one tab: \"all\""
    );
    assert_eq!(counts.counts[0].key, "all");
    assert_eq!(
        counts.counts[0].count, 2,
        "EX-01: count must reflect the real number of movement rows, not 0"
    );
}

/// EX-01 correctness constraint: the count must apply the SAME filters as
/// the list (D-24's dual subtree-inclusive place filters), or the badge
/// would lie. Seeds two movements at different places, filters to just one
/// via `to_place_id`, and asserts the count matches the FILTERED list length
/// (1), not the total row count (2).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn report_movements_get_report_counts_respects_place_filter() {
    let (ctx, _dir) = minimal_ctx();

    let warehouse = seed_place(&ctx, None, "Склад-Ф").await;
    let office = seed_place(&ctx, None, "Офис-Ф").await;
    let other_office = seed_place(&ctx, None, "Другой офис-Ф").await;
    let device_a = seed_device(&ctx, "Ноутбук фильтр A").await;
    let device_b = seed_device(&ctx, "Ноутбук фильтр B").await;
    seed_movement(
        &ctx,
        "device",
        device_a,
        warehouse,
        "Склад-Ф",
        office,
        "Офис-Ф",
        "manual",
        1_700_000_300,
    )
    .await;
    seed_movement(
        &ctx,
        "device",
        device_b,
        warehouse,
        "Склад-Ф",
        other_office,
        "Другой офис-Ф",
        "manual",
        1_700_000_400,
    )
    .await;

    let filter = ReportFilter {
        to_place_id: Some(office),
        ..ReportFilter::default()
    };

    let manager = Identity {
        user_id: None,
        role: Role::Manager,
    };
    let listed = build_reports_list_movements(&ctx, &manager, filter.clone(), wide_period())
        .await
        .expect("list_movements with to_place_id filter");
    assert_eq!(
        listed.rows.len(),
        1,
        "sanity: filter narrows the list to 1 row"
    );

    let counts = ctx
        .reports
        .get_report_counts("movements", filter, wide_period(), false)
        .await
        .expect("get_report_counts(movements) with the same filter");

    assert_eq!(
        counts.counts[0].count, 1,
        "EX-01: the count must apply the SAME to_place_id filter as the list, not the unfiltered total"
    );
}

// ---------------------------------------------------------------------------
// report_movements_return_act_shows_canonical_number (Phase 40-29, WR-10)
// ---------------------------------------------------------------------------

/// WR-10 gap closure: a movement caused by a RETURN act must show the same
/// canonical number the timeline already shows ("20в" for a solo return),
/// not the bare parent handover number ("20", indistinguishable from the
/// handover itself). Before this plan, `query_movements_inner` selected
/// `a.number AS act_number` directly off `acts` and never ran it through
/// `format_act_number` — this test fails on that code (asserts the reason
/// contains the exact suffixed string, not just a substring of the bare
/// number).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn report_movements_return_act_shows_canonical_number() {
    let (ctx, _dir) = minimal_ctx();
    let manager = Identity {
        user_id: None,
        role: Role::Manager,
    };

    let place_a = seed_place(&ctx, None, "Здание А").await;
    let place_b = seed_place(&ctx, None, "Здание Б").await;
    let device = seed_device(&ctx, "Ноутбук возврат-тест").await;

    let handover_id = seed_act(&ctx, 20).await;
    // Solo return (sibling_return_count == 1) — canonical display drops the
    // sub-number suffix, "20в".
    let return_id = seed_return_act(&ctx, handover_id, 20, 1).await;

    seed_movement_with_act(
        &ctx,
        "device",
        device,
        place_b,
        "Здание Б",
        place_a,
        "Здание А",
        "act",
        return_id,
        1_700_000_100,
    )
    .await;

    let result =
        build_reports_list_movements(&ctx, &manager, ReportFilter::default(), wide_period())
            .await
            .expect("manager can list movements");

    assert_eq!(result.rows.len(), 1, "sanity: exactly one movement row");
    let reason = result.rows[0]
        .reason
        .as_deref()
        .expect("reason must be present for an act-caused movement");
    assert!(
        reason.contains("№20в"),
        "WR-10: report must show the canonical return number \"20в\", got reason: {reason:?}"
    );
    assert!(
        !reason.contains("№20 ") && !reason.ends_with("№20"),
        "WR-10: report must NOT show the bare parent handover number, got reason: {reason:?}"
    );
}
