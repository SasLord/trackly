//! Quick task 260820-vad: домен «Заявки» в разделе «Отчёты» — интеграционные
//! тесты (VAD-01..04).
//!
//! Покрывает: статус-фильтр по вкладкам, RU-перевод Тип/Статус на экране и в
//! CSV, пустую «Принтер / Локация» для заявок без принтера, per-tab счётчики
//! (`get_report_counts(domain="requests")`), RBAC-исключение `ad_register`
//! для роли Manager (REQ-06/T-09-11).
//!
//! Приватность (CLAUDE.md): все имена вымышленные («Иванов И.И.»), заявитель
//! в фикстуре — не реальный сотрудник.

use std::collections::HashMap;
use std::sync::Arc;

use rusqlite::params;
use tempfile::TempDir;
use tokio_util::sync::CancellationToken;

use trackly_app::context::AppCtx;
use trackly_app::dto::reports::{PeriodDto, ReportFilter};
use trackly_app::tauri_cmds::reports::{
    build_reports_export_csv, build_reports_get_report_counts, build_reports_list_requests_all,
    build_reports_list_requests_in_progress, build_reports_list_requests_open,
};
use trackly_core::auth::{Identity, Role};
use trackly_core::error::AppError;
use trackly_core::primitives::clock::Clock;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::db::writer_worker::WriterHandle;
use trackly_infra::test_support::test_app_ctx::test_writer_and_readers;

/// Minimal fully-wired `AppCtx` fixture — copied 1:1 from
/// `tests/reports_period_required.rs::minimal_ctx()` (same dependency shape:
/// `AppCtx.reports: Arc<ReportService>`).
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
    let reports = Arc::new(trackly_app::services::ReportService::new(
        writer.clone(),
        readers.clone(),
        clock.clone(),
        Arc::new(config.clone()),
        pdf.clone(),
    ));
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
    let ctx = AppCtx {
        writer,
        readers,
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

// ---------------------------------------------------------------------------
// Seed helpers (direct SQL, per request_lifecycle.rs::seed_user pattern)
// ---------------------------------------------------------------------------

async fn seed_user(writer: &WriterHandle, login: &str, full_name: &str) -> i64 {
    let now = SystemClock.unix_seconds();
    let login = login.to_string();
    let full_name = full_name.to_string();
    writer
        .execute(move |conn| {
            let tx = conn.transaction().map_err(|e| AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            tx.execute(
                "INSERT INTO users (login, full_name, role, ad_user, created_at_utc, updated_at_utc, version) \
                 VALUES (?1, ?2, 'employee', 0, ?3, ?3, 1)",
                params![login, full_name, now],
            )
            .map_err(|e| AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            let id = tx.last_insert_rowid();
            tx.commit().map_err(|e| AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            Ok(id)
        })
        .await
        .expect("seed user")
}

async fn seed_location(writer: &WriterHandle, name: &str) -> i64 {
    let now = SystemClock.unix_seconds();
    let name = name.to_string();
    writer
        .execute(move |conn| {
            let tx = conn.transaction().map_err(|e| AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            tx.execute(
                "INSERT INTO locations (name, created_at_utc, updated_at_utc, version) \
                 VALUES (?1, ?2, ?2, 1)",
                params![name, now],
            )
            .map_err(|e| AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            let id = tx.last_insert_rowid();
            tx.commit().map_err(|e| AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            Ok(id)
        })
        .await
        .expect("seed location")
}

async fn seed_printer_device(writer: &WriterHandle, name: &str, location_id: i64) -> i64 {
    let now = SystemClock.unix_seconds();
    let name = name.to_string();
    writer
        .execute(move |conn| {
            let tx = conn.transaction().map_err(|e| AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            tx.execute(
                "INSERT INTO devices (type_id, name, location_id, status_id, created_at_utc, updated_at_utc, version) \
                 VALUES (2, ?1, ?2, 2, ?3, ?3, 1)",
                params![name, location_id, now],
            )
            .map_err(|e| AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            let id = tx.last_insert_rowid();
            tx.commit().map_err(|e| AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            Ok(id)
        })
        .await
        .expect("seed printer device")
}

#[allow(clippy::too_many_arguments)]
async fn seed_request(
    writer: &WriterHandle,
    request_type: &str,
    status: &str,
    requested_by: i64,
    printer_device_id: Option<i64>,
    created_at_utc: i64,
) -> i64 {
    let request_type = request_type.to_string();
    let status = status.to_string();
    writer
        .execute(move |conn| {
            let tx = conn.transaction().map_err(|e| AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            tx.execute(
                "INSERT INTO requests \
                 (request_type, status, requested_by_user_id, printer_device_id, created_at_utc, updated_at_utc, version) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?5, 1)",
                params![request_type, status, requested_by, printer_device_id, created_at_utc],
            )
            .map_err(|e| AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            let id = tx.last_insert_rowid();
            tx.commit().map_err(|e| AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            Ok(id)
        })
        .await
        .expect("seed request")
}

/// Fixed timestamp inside June 2026 Europe/Moscow bounds
/// (`period_month_june_2026_moscow`: `1_780_261_200..=1_782_853_199`).
const FIXTURE_CREATED_AT_UTC: i64 = 1_780_300_000;

fn fixture_period() -> PeriodDto {
    PeriodDto {
        mode: "month".to_string(),
        year: Some(2026),
        month: Some(6),
        date_from: None,
        date_to: None,
    }
}

/// Common fixture: one requester, one location, one printer, and 4 requests
/// covering every status the «Все» tab must show (including `rejected`).
/// Returns `(ctx, dir, requester_id)`.
async fn seed_fixture() -> (AppCtx, TempDir, i64) {
    let (ctx, dir) = minimal_ctx();
    let requester_id = seed_user(&ctx.writer, "us501", "Иванов И.И.").await;
    let location_id = seed_location(&ctx.writer, "Склад тест").await;
    let printer_id = seed_printer_device(&ctx.writer, "Принтер HP LaserJet", location_id).await;

    seed_request(
        &ctx.writer,
        "cartridge_replace",
        "open",
        requester_id,
        Some(printer_id),
        FIXTURE_CREATED_AT_UTC,
    )
    .await;
    seed_request(
        &ctx.writer,
        "free_form",
        "in_progress",
        requester_id,
        None,
        FIXTURE_CREATED_AT_UTC,
    )
    .await;
    seed_request(
        &ctx.writer,
        "ad_register",
        "completed",
        requester_id,
        None,
        FIXTURE_CREATED_AT_UTC,
    )
    .await;
    seed_request(
        &ctx.writer,
        "cartridge_replace",
        "rejected",
        requester_id,
        Some(printer_id),
        FIXTURE_CREATED_AT_UTC,
    )
    .await;

    (ctx, dir, requester_id)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn report_requests_all_includes_every_status_translated_including_rejected() {
    let (ctx, _dir, _requester_id) = seed_fixture().await;
    let period = fixture_period();

    let response = build_reports_list_requests_all(
        &ctx,
        &Identity::trusted_admin(),
        ReportFilter::default(),
        period,
    )
    .await
    .expect("requests_all");

    assert_eq!(response.total, 4);
    let statuses: std::collections::HashSet<String> = response
        .rows
        .iter()
        .map(|r| r.status_name.clone().expect("status_name"))
        .collect();
    let expected: std::collections::HashSet<String> =
        ["Открыта", "В работе", "Выполнена", "Отклонена"]
            .into_iter()
            .map(String::from)
            .collect();
    assert_eq!(statuses, expected);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn report_requests_open_filters_by_status_and_translates_type() {
    let (ctx, _dir, _requester_id) = seed_fixture().await;
    let period = fixture_period();

    let response = build_reports_list_requests_open(
        &ctx,
        &Identity::trusted_admin(),
        ReportFilter::default(),
        period,
    )
    .await
    .expect("requests_open");

    assert_eq!(response.total, 1);
    let row = &response.rows[0];
    assert_eq!(row.status_name, Some("Открыта".to_string()));
    assert_eq!(row.request_type_label, Some("Замена картриджа".to_string()));
    assert_eq!(row.giver_name, Some("Иванов И.И.".to_string()));
    assert_eq!(
        row.location_name,
        Some("Принтер HP LaserJet, Склад тест".to_string())
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn report_requests_printer_location_blank_when_no_printer() {
    let (ctx, _dir, _requester_id) = seed_fixture().await;
    let period = fixture_period();

    let response = build_reports_list_requests_in_progress(
        &ctx,
        &Identity::trusted_admin(),
        ReportFilter::default(),
        period,
    )
    .await
    .expect("requests_in_progress");

    assert_eq!(response.total, 1);
    assert!(response.rows[0].location_name.is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn report_requests_csv_export_uses_translated_values_not_raw_enum_keys() {
    let (ctx, _dir, _requester_id) = seed_fixture().await;
    let period = fixture_period();

    let bytes = build_reports_export_csv(
        &ctx,
        &Identity::trusted_admin(),
        "requests_all".to_string(),
        ReportFilter::default(),
        Some(period),
    )
    .await
    .expect("csv export");

    let body = std::str::from_utf8(&bytes[3..]).expect("utf8 body after BOM");
    assert!(body.contains("Замена картриджа"), "body: {body}");
    assert!(body.contains("Открыта"), "body: {body}");
    assert!(body.contains("Иванов И.И."), "body: {body}");
    assert!(
        !body.contains("cartridge_replace"),
        "raw enum key must not leak into CSV cells: {body}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn report_requests_status_counts_match_tab_keys() {
    let (ctx, _dir, _requester_id) = seed_fixture().await;
    let period = fixture_period();

    let counts_dto = build_reports_get_report_counts(
        &ctx,
        &Identity::trusted_admin(),
        "requests".to_string(),
        ReportFilter::default(),
        period,
    )
    .await
    .expect("get_report_counts");

    let counts: HashMap<String, i64> = counts_dto
        .counts
        .into_iter()
        .map(|e| (e.key, e.count))
        .collect();

    assert_eq!(counts.get("all"), Some(&4));
    assert_eq!(counts.get("open"), Some(&1));
    assert_eq!(counts.get("in_progress"), Some(&1));
    assert_eq!(counts.get("completed"), Some(&1));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn report_requests_manager_role_excludes_ad_register_admin_sees_all() {
    let (ctx, _dir, requester_id) = seed_fixture().await;
    let period = fixture_period();

    let manager_identity = Identity {
        user_id: Some(requester_id),
        role: Role::Manager,
    };
    let manager_response = build_reports_list_requests_all(
        &ctx,
        &manager_identity,
        ReportFilter::default(),
        period.clone(),
    )
    .await
    .expect("requests_all as manager");

    assert_eq!(manager_response.total, 3);
    assert!(
        !manager_response
            .rows
            .iter()
            .any(|r| r.request_type_label == Some("Учётная запись AD".to_string())),
        "Manager must not see ad_register requests"
    );

    let admin_response = build_reports_list_requests_all(
        &ctx,
        &Identity::trusted_admin(),
        ReportFilter::default(),
        period,
    )
    .await
    .expect("requests_all as admin");

    assert_eq!(admin_response.total, 4);
    let ad_register_count = admin_response
        .rows
        .iter()
        .filter(|r| r.request_type_label == Some("Учётная запись AD".to_string()))
        .count();
    assert_eq!(ad_register_count, 1);
}
