//! Phase 17 Plan 04 — Task 1: dedicated HTML-render regression suite for
//! `ReportService::export_pdf` (Phase 17 Plan 01's HTML-print migration off
//! the legacy krilla document-spec pipeline). Mirrors `html_act_render.rs`'s
//! fixture/assertion style, adapted for `ReportService` instead of
//! `ActService`.
//!
//! Covers: 1-row render, multi-month grouping, empty report, org header, and
//! a negative "no krilla artifacts" assertion (output is HTML text, never
//! accidentally still PDF bytes encoded as a string).

use std::sync::Arc;

use trackly_app::dto::reports::{OrgSettingsDto, ReportResponse, ReportRow};
use trackly_app::pdf::PdfRenderer;
use trackly_app::services::{OrganizationService, ReportService};
use trackly_core::primitives::clock::Clock;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::test_support::test_writer_and_readers;
use trackly_infra::{AppConfig, Paths};

/// Build a `ReportService` wired with `with_organization` (mirrors
/// `html_act_render.rs::make_full_pipeline`, adapted for reports). Does NOT
/// call `materialize_defaults_on_startup` — relies on the embedded-default
/// fallback path (same as `html_act_render.rs`'s
/// `html_falls_back_to_embedded_default_when_file_absent` test), proving
/// `report.html`'s embedded default alone is sufficient for these assertions.
fn make_report_service() -> (ReportService, tempfile::TempDir) {
    let (writer, readers, dir) = test_writer_and_readers();
    let clock: Arc<dyn Clock + Send + Sync> = Arc::new(SystemClock);
    let paths = Arc::new(Paths::resolve_for_exe_dir(dir.path().to_path_buf()).expect("paths"));
    let organization = Arc::new(OrganizationService::new(paths));
    let pdf = Arc::new(PdfRenderer::new());
    let svc = ReportService::new(writer, readers, clock, Arc::new(AppConfig::default()), pdf)
        .with_organization(organization);
    (svc, dir)
}

fn make_row(
    month_key: &str,
    number: &str,
    device_name: &str,
    giver: &str,
    receiver: &str,
    location: &str,
) -> ReportRow {
    ReportRow {
        id: 1,
        month_key: Some(month_key.to_string()),
        number: Some(number.to_string()),
        sub_number: None,
        giver_name: Some(giver.to_string()),
        receiver_name: Some(receiver.to_string()),
        handover_date_utc: Some(1_780_000_000),
        location_name: Some(location.to_string()),
        act_type: Some("handover".to_string()),
        device_name: Some(device_name.to_string()),
        quantity: Some(1),
        code: None,
        model_label: None,
        status_name: None,
    }
}

fn empty_org() -> OrgSettingsDto {
    OrgSettingsDto {
        org_name: String::new(),
        inn: String::new(),
        kpp: String::new(),
        address: String::new(),
        has_logo: false,
        phone: String::new(),
        fax: String::new(),
        email: String::new(),
        okpo: String::new(),
        ogrn: String::new(),
    }
}

/// 1-row report: HTML contains the Russian month heading and every field
/// value of the single row.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn html_report_single_row_renders_columns_and_month() {
    let (svc, _dir) = make_report_service();
    let row = make_row(
        "2026-09",
        "42",
        "Принтер HP LaserJet",
        "Петров П.П.",
        "Сидоров С.С.",
        "Склад №1",
    );
    let rows = ReportResponse {
        rows: vec![row],
        total: 1,
    };
    let columns = [
        "number",
        "device_name",
        "giver_name",
        "receiver_name",
        "location_name",
    ];
    let labels = ["Номер", "Устройства", "Сдал", "Принял", "Локация"];

    let html = svc
        .export_pdf(
            &rows,
            "Тестовый отчёт",
            "Сентябрь 2026",
            &empty_org(),
            None,
            None,
            &columns,
            &labels,
        )
        .await
        .expect("export_pdf ok");

    assert!(
        html.contains("Сентябрь 2026"),
        "expected September 2026 month heading in HTML: {html}"
    );
    for expected in [
        "42",
        "Принтер HP LaserJet",
        "Петров П.П.",
        "Сидоров С.С.",
        "Склад №1",
    ] {
        assert!(
            html.contains(expected),
            "expected row value {expected:?} missing from rendered HTML: {html}"
        );
    }
}

/// Multi-month report: rows spanning 2 distinct month_key values render both
/// Russian month labels, each with its own row's values present.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn html_report_multi_month_groups_render_separately() {
    let (svc, _dir) = make_report_service();
    let rows = ReportResponse {
        rows: vec![
            make_row(
                "2026-08",
                "10",
                "Сканер Canon",
                "Иванов И.И.",
                "Петров П.П.",
                "Склад №2",
            ),
            make_row(
                "2026-09",
                "42",
                "Принтер HP LaserJet",
                "Петров П.П.",
                "Сидоров С.С.",
                "Склад №1",
            ),
        ],
        total: 2,
    };
    let columns = ["number", "device_name", "giver_name", "receiver_name"];
    let labels = ["Номер", "Устройства", "Сдал", "Принял"];

    let html = svc
        .export_pdf(
            &rows,
            "Тестовый отчёт",
            "Август-Сентябрь 2026",
            &empty_org(),
            None,
            None,
            &columns,
            &labels,
        )
        .await
        .expect("export_pdf ok");

    assert!(
        html.contains("Август 2026"),
        "expected August 2026 month heading in HTML: {html}"
    );
    assert!(
        html.contains("Сентябрь 2026"),
        "expected September 2026 month heading in HTML: {html}"
    );
    assert!(html.contains("Сканер Canon"));
    assert!(html.contains("Иванов И.И."));
    assert!(html.contains("Принтер HP LaserJet"));
    assert!(html.contains("Сидоров С.С."));
}

/// Empty report: rows vec is empty, HTML shows the no-data message.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn html_report_empty_response_shows_no_data_message() {
    let (svc, _dir) = make_report_service();
    let rows = ReportResponse {
        rows: vec![],
        total: 0,
    };
    let columns = ["device_name"];
    let labels = ["Устройства"];

    let html = svc
        .export_pdf(
            &rows,
            "Пустой отчёт",
            "Ноябрь 2026",
            &empty_org(),
            None,
            None,
            &columns,
            &labels,
        )
        .await
        .expect("export_pdf ok");

    assert!(
        html.contains("Нет данных за указанный период."),
        "expected empty-state message in HTML: {html}"
    );
}

/// Org header: a populated `OrgSettingsDto` (non-empty org_name/inn) renders
/// the org name string in the HTML output.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn html_report_org_header_present() {
    let (svc, _dir) = make_report_service();
    let rows = ReportResponse {
        rows: vec![make_row(
            "2026-09",
            "42",
            "Принтер",
            "Петров П.П.",
            "Сидоров С.С.",
            "Склад №1",
        )],
        total: 1,
    };
    let columns = ["device_name"];
    let labels = ["Устройства"];
    let mut org = empty_org();
    org.org_name = "ООО «Ромашка»".to_string();
    org.inn = "7701234567".to_string();

    let html = svc
        .export_pdf(
            &rows,
            "Отчёт",
            "Сентябрь 2026",
            &org,
            None,
            None,
            &columns,
            &labels,
        )
        .await
        .expect("export_pdf ok");

    assert!(
        html.contains("ООО «Ромашка»"),
        "expected org name in HTML header: {html}"
    );
}

/// Negative assertion: the returned string is HTML text, not accidentally
/// still PDF bytes encoded as a string (proves the krilla migration is
/// complete for every fixture shape exercised above).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn html_report_no_krilla_artifacts() {
    let (svc, _dir) = make_report_service();

    let non_empty = ReportResponse {
        rows: vec![make_row(
            "2026-09",
            "42",
            "Принтер",
            "Петров П.П.",
            "Сидоров С.С.",
            "Склад №1",
        )],
        total: 1,
    };
    let empty = ReportResponse {
        rows: vec![],
        total: 0,
    };
    let columns = ["device_name"];
    let labels = ["Устройства"];

    for rows in [&non_empty, &empty] {
        let html = svc
            .export_pdf(
                rows,
                "Отчёт",
                "Сентябрь 2026",
                &empty_org(),
                None,
                None,
                &columns,
                &labels,
            )
            .await
            .expect("export_pdf ok");

        assert!(
            !html.starts_with("%PDF"),
            "export_pdf output must be HTML text, not PDF bytes: {:?}",
            html.chars().take(80).collect::<String>()
        );
        assert!(
            html.contains("<html") || html.contains("<!DOCTYPE"),
            "export_pdf output must be well-formed HTML markup: {:?}",
            html.chars().take(80).collect::<String>()
        );
    }
}

/// Regression test for D-03/CR-01: the rendered header row must use the
/// Russian labels supplied via `column_labels`, never the raw snake_case
/// keys from `columns` — even though `columns` is still passed (and still
/// used to resolve cell values via `row_field`).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn html_report_header_uses_russian_labels_not_raw_keys() {
    let (svc, _dir) = make_report_service();
    let rows = ReportResponse {
        rows: vec![make_row(
            "2026-09",
            "42",
            "Принтер HP LaserJet",
            "Петров П.П.",
            "Сидоров С.С.",
            "Склад №1",
        )],
        total: 1,
    };
    let columns = [
        "number",
        "device_name",
        "giver_name",
        "receiver_name",
        "location_name",
    ];
    let labels = ["Номер", "Устройства", "Сдал", "Принял", "Локация"];

    let html = svc
        .export_pdf(
            &rows,
            "Тестовый отчёт",
            "Сентябрь 2026",
            &empty_org(),
            None,
            None,
            &columns,
            &labels,
        )
        .await
        .expect("export_pdf ok");

    assert!(
        html.contains("<th>Сдал</th>"),
        "expected Russian header label <th>Сдал</th> in HTML: {html}"
    );
    assert!(
        !html.contains("giver_name"),
        "raw snake_case key 'giver_name' must not leak into rendered header/output: {html}"
    );
}

/// Regression test for WR-05: a disallowed logo mime must drop the logo
/// entirely rather than being embedded (with any mime, spoofed or not) into
/// the `data:` URI.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn html_report_disallowed_logo_mime_drops_logo() {
    let (svc, _dir) = make_report_service();
    let rows = ReportResponse {
        rows: vec![make_row(
            "2026-09",
            "42",
            "Принтер",
            "Петров П.П.",
            "Сидоров С.С.",
            "Склад №1",
        )],
        total: 1,
    };
    let columns = ["device_name"];
    let labels = ["Устройства"];

    let html = svc
        .export_pdf(
            &rows,
            "Отчёт",
            "Сентябрь 2026",
            &empty_org(),
            Some(vec![1, 2, 3]),
            Some("text/html".to_string()),
            &columns,
            &labels,
        )
        .await
        .expect("export_pdf ok");

    assert!(
        !html.contains("<img src=\"data:"),
        "disallowed logo mime must fully drop the logo, not embed it: {html}"
    );
}
