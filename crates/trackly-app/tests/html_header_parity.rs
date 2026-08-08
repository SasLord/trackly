//! Header-partial structural gate (Phase 34, Plan 02): the three shipped HTML
//! print templates (`act_handover.html`, `act_acceptance.html`, `report.html`)
//! must all pull in the shared header markup/CSS via
//! `{% include "_header.html" %}` (D-12) rather than duplicating it, and the
//! shared partial's `.orgName` node must never carry a hardcoded organization
//! name (DOC-05) — only Jinja expressions and markup.
//!
//! Reads the templates via `include_str!` (compile-time, relative to this
//! test file's own location), modeled on `html_page_parity.rs`'s style — no
//! tokio needed for Tests 1-2, this test only READS the templates, never
//! modifies them.
//!
//! Test 3 (Phase 34 Plan 03, DOC-04 render-gate) is the exception — it
//! exercises the real render pipeline (ActService + ReportService) to prove
//! the header fragment is byte-identical across all three document forms.

use std::sync::Arc;

use rusqlite::params;
use trackly_app::dto::act::{ActCreateDto, ActItemNewDto};
use trackly_app::dto::reports::{OrgPatch, ReportResponse, ReportRow};
use trackly_app::pdf::PdfRenderer;
use trackly_app::services::{ActService, OrgDbService, OrganizationService, ReportService, TemplateService};
use trackly_core::auth::Identity;
use trackly_core::primitives::clock::Clock;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::error_conversions::map_rusqlite;
use trackly_infra::test_support::test_writer_and_readers;
use trackly_infra::{AppConfig, Paths};

const ACT_HANDOVER_HTML: &str = include_str!("../templates/act_handover.html");
const ACT_ACCEPTANCE_HTML: &str = include_str!("../templates/act_acceptance.html");
const REPORT_HTML: &str = include_str!("../templates/report.html");
const HEADER_HTML: &str = include_str!("../templates/_header.html");

#[test]
fn all_three_templates_include_header_partial() {
    assert!(
        ACT_HANDOVER_HTML.contains("{% include \"_header.html\" %}"),
        "act_handover.html must include the shared _header.html partial"
    );
    assert!(
        ACT_ACCEPTANCE_HTML.contains("{% include \"_header.html\" %}"),
        "act_acceptance.html must include the shared _header.html partial"
    );
    assert!(
        REPORT_HTML.contains("{% include \"_header.html\" %}"),
        "report.html must include the shared _header.html partial"
    );
}

/// DOC-05, privacy-safe positive form: proves the `.orgName` node in the
/// shared header partial holds no hardcoded organization-name text, without
/// ever writing the real organization name into this test file. Extracts the
/// `<div class="orgName">...</div>` fragment (non-greedy — safe here because,
/// unlike `.header`, `.orgName`'s own children are only `{{ }}` / `{% %}` /
/// `<br />` / `(` / `)`, never a nested `<div>`), strips every Jinja
/// expression/statement span, then asserts the remainder contains no Unicode
/// letter character.
#[test]
fn header_partial_org_name_node_has_no_hardcoded_literal() {
    let org_name_re = regex::Regex::new(r#"(?s)<div class="orgName">.*?</div>"#)
        .expect("valid orgName extraction regex");
    let org_name_block = org_name_re
        .find(HEADER_HTML)
        .unwrap_or_else(|| panic!("no <div class=\"orgName\">...</div> block found in _header.html"))
        .as_str();

    let jinja_expr_re = regex::Regex::new(r"(?s)\{\{.*?\}\}").expect("valid Jinja expr regex");
    let jinja_stmt_re = regex::Regex::new(r"(?s)\{%.*?%\}").expect("valid Jinja stmt regex");

    let stripped = jinja_expr_re.replace_all(org_name_block, "");
    let stripped = jinja_stmt_re.replace_all(&stripped, "");

    let remainder: String = stripped
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .replace("<div", "")
        .replace("class=\"orgName\">", "")
        .replace("</div>", "")
        .replace("<br", "")
        .replace("/>", "")
        .replace('(', "")
        .replace(')', "");

    let has_letter = remainder.chars().any(|c| c.is_alphabetic());
    assert!(
        !has_letter,
        "_header.html's .orgName node must contain no hardcoded literal text \
         (only Jinja expressions/markup) — found leftover non-markup content: {remainder:?}"
    );
}

/// Extracts the substring between the literal `<!-- HEADER-START -->` /
/// `<!-- HEADER-END -->` markers (non-nested, plain string search — no regex
/// needed) that `_header.html` wraps its header block in.
fn extract_header_fragment(html: &str) -> &str {
    let start_marker = "<!-- HEADER-START -->";
    let end_marker = "<!-- HEADER-END -->";
    let start = html
        .find(start_marker)
        .unwrap_or_else(|| panic!("no {start_marker} marker found in rendered HTML"))
        + start_marker.len();
    let end = html[start..]
        .find(end_marker)
        .unwrap_or_else(|| panic!("no {end_marker} marker found in rendered HTML"));
    &html[start..start + end]
}

async fn seed_device(writer: &Arc<trackly_infra::db::writer_worker::WriterHandle>, name: &str) -> i64 {
    let name = name.to_string();
    writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO devices \
                 (type_id, name, status_id, version, created_at_utc, updated_at_utc) \
                 VALUES (1, ?1, 1, 1, ?2, ?2)",
                params![name, 1_700_000_000_i64],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed device")
}

/// Test 3 (Phase 34 Plan 03, DOC-04 render-gate): after saving org fields
/// including a non-empty `full_name` via the real `OrgDbService::save_fields`
/// path, rendering all three document types through the real pipeline
/// produces byte-identical `<!-- HEADER-START -->...<!-- HEADER-END -->`
/// header fragments — structurally proving DOC-04 (identical header across
/// all three forms) via a real render, not just a substring/include check.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn header_fragment_identical_across_all_three_forms() {
    let (writer, readers, dir) = test_writer_and_readers();
    let clock: Arc<dyn Clock + Send + Sync> = Arc::new(SystemClock);
    let paths = Arc::new(Paths::resolve_for_exe_dir(dir.path().to_path_buf()).expect("paths"));
    let organization = Arc::new(OrganizationService::new(paths.clone()));
    let templates = Arc::new(TemplateService::new(
        writer.clone(),
        readers.clone(),
        clock.clone(),
    ));
    templates.seed_defaults_on_startup().await.expect("seed");
    let pdf = Arc::new(PdfRenderer::new());
    let org_db = Arc::new(OrgDbService::new(
        writer.clone(),
        readers.clone(),
        clock.clone(),
        paths,
    ));

    let acts = ActService::new(writer.clone(), readers.clone(), clock.clone())
        .with_pdf_pipeline(templates.clone(), organization.clone(), pdf.clone())
        .with_org_db(org_db.clone());
    let reports = ReportService::new(
        writer.clone(),
        readers.clone(),
        clock.clone(),
        Arc::new(AppConfig::default()),
        pdf.clone(),
    )
    .with_organization(organization.clone());

    let caller = Identity::trusted_admin();
    org_db
        .save_fields(
            &caller,
            OrgPatch {
                org_name: "ООО Тест".to_string(),
                inn: "7712345678".to_string(),
                kpp: "771001001".to_string(),
                address: "г. Москва, ул. Тестовая, 1".to_string(),
                phone: "+7 495 000-00-01".to_string(),
                fax: "+7 495 000-00-02".to_string(),
                email: "info@test-org.ru".to_string(),
                okpo: "87654321".to_string(),
                ogrn: "1027700654321".to_string(),
                address_line2: "офис 1".to_string(),
                full_name: "Общество с ограниченной ответственностью\nООО «Тест»".to_string(),
            },
        )
        .await
        .expect("save_fields");

    let device_id = seed_device(&writer, "Тестовый принтер").await;

    let act_payload = ActCreateDto {
        number_override: None,
        giver_name: "Иванов И.И.".to_string(),
        receiver_name: "Петров П.П.".to_string(),
        location_id: None,
        location_name: None,
        notes: None,
        deadline_utc: None,
        handover_date_utc: None,
        items: vec![ActItemNewDto {
            device_id,
            device_ids: Vec::new(),
            quantity: 1,
        }],
    };
    let act = acts.create(act_payload).await.expect("create handover");

    let handover_html = acts.render_pdf(act.id).await.expect("render_pdf");
    let acceptance_html = acts
        .render_acceptance_pdf(
            device_id,
            "Иванов И.И.".to_string(),
            "Петров П.П.".to_string(),
            1_700_000_000,
        )
        .await
        .expect("render_acceptance_pdf");

    let (org_dto, logo_bytes, logo_mime) = org_db.get_for_pdf().await.expect("get_for_pdf");
    let rows = ReportResponse {
        rows: vec![ReportRow {
            id: 1,
            month_key: Some("2026-09".to_string()),
            number: Some("1".to_string()),
            sub_number: None,
            giver_name: Some("Иванов И.И.".to_string()),
            receiver_name: Some("Петров П.П.".to_string()),
            handover_date_utc: Some(1_700_000_000),
            location_name: Some("Офис 101".to_string()),
            act_type: Some("handover".to_string()),
            device_name: Some("Тестовый принтер".to_string()),
            quantity: Some(1),
            code: None,
            model_label: None,
            status_name: None,
        }],
        total: 1,
    };
    let columns = ["number", "device_name", "giver_name", "receiver_name"];
    let labels = ["Номер", "Устройство", "Сдал", "Принял"];
    let report_html = reports
        .export_pdf(
            &rows,
            "Тестовый отчёт",
            "Сентябрь 2026",
            &org_dto,
            logo_bytes,
            logo_mime,
            &columns,
            &labels,
        )
        .await
        .expect("export_pdf");

    let handover_fragment = extract_header_fragment(&handover_html);
    let acceptance_fragment = extract_header_fragment(&acceptance_html);
    let report_fragment = extract_header_fragment(&report_html);

    assert_eq!(
        handover_fragment, acceptance_fragment,
        "act_handover and act_acceptance header fragments must be byte-identical"
    );
    assert_eq!(
        acceptance_fragment, report_fragment,
        "act_acceptance and report header fragments must be byte-identical"
    );
}
