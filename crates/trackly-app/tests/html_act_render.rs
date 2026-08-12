//! Phase 16 Plan 05 — Task 2: dedicated HTML-generation test suite covering
//! D-14 items 1-4 (block/field presence + logo, 1-vs-N devices, fallback-vs-
//! file, offline/no-CDN) for both acts (`act_handover` / `act_acceptance`),
//! rendered through the full `ActService::render_pdf`/`render_acceptance_pdf`
//! pipeline (Phase 16 Plan 02's HTML rewiring).

use std::sync::Arc;

use rusqlite::params;
use trackly_app::dto::act::{ActCreateDto, ActItemNewDto, ActReturnDto, ActReturnItemDto};
use trackly_app::dto::reports::OrgPatch;
use trackly_app::pdf::PdfRenderer;
use trackly_app::services::act_service::format_ru_date;
use trackly_app::services::{ActService, OrgDbService, OrganizationService, TemplateService};
use trackly_core::auth::Identity;
use trackly_core::primitives::clock::Clock;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::db::{pools::ReaderPool, writer_worker::WriterHandle};
use trackly_infra::error_conversions::map_rusqlite;
use trackly_infra::test_support::test_writer_and_readers;
use trackly_infra::Paths;

const LOGO_PNG: &[u8] = include_bytes!("fixtures/logo_test.png");
const LOGO_SVG_WITH_SCRIPT: &[u8] = include_bytes!("fixtures/logo_test_with_script.svg");

/// Phase 36 Plan 03: compile-time read of the shipped template, used only by
/// the CSS-structural gate below (`html_handover_appendix_css_declares_forced_break_and_keep_together`).
/// Per D-01/D-03 in this plan (frozen-template constraint), this file never
/// modifies `act_handover.html` — it only reads it, exactly like
/// `html_page_parity.rs`'s existing `include_str!` pattern.
const ACT_HANDOVER_HTML: &str = include_str!("../templates/act_handover.html");

struct Pipeline {
    acts: ActService,
    writer: Arc<WriterHandle>,
    _readers: Arc<ReaderPool>,
    _dir: tempfile::TempDir,
}

async fn make_full_pipeline() -> Pipeline {
    let (writer, readers, dir) = test_writer_and_readers();
    let clock: Arc<dyn Clock + Send + Sync> = Arc::new(SystemClock);

    let paths = Arc::new(Paths::resolve_for_exe_dir(dir.path().to_path_buf()).expect("paths"));
    let organization = Arc::new(OrganizationService::new(paths.clone()));
    let templates = Arc::new(TemplateService::new(
        writer.clone(),
        readers.clone(),
        clock.clone(),
    ));
    let pdf = Arc::new(PdfRenderer::new());
    templates.seed_defaults_on_startup().await.expect("seed");

    let org_db = Arc::new(OrgDbService::new(
        writer.clone(),
        readers.clone(),
        clock.clone(),
        paths,
    ));

    let acts = ActService::new(writer.clone(), readers.clone(), clock.clone())
        .with_pdf_pipeline(templates.clone(), organization.clone(), pdf.clone())
        .with_org_db(org_db.clone());

    Pipeline {
        acts,
        writer,
        _readers: readers,
        _dir: dir,
    }
}

async fn seed_device(writer: &Arc<WriterHandle>, name: &str) -> i64 {
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

async fn seed_devices(writer: &Arc<WriterHandle>, count: usize) -> Vec<i64> {
    let names: Vec<String> = (0..count).map(|i| format!("HTML-Ноутбук-{i}")).collect();
    writer
        .execute(move |conn| {
            let tx = conn.transaction().map_err(map_rusqlite)?;
            let mut out = Vec::with_capacity(names.len());
            for name in &names {
                tx.execute(
                    "INSERT INTO devices \
                     (type_id, name, status_id, version, created_at_utc, updated_at_utc) \
                     VALUES (1, ?1, 1, 1, ?2, ?2)",
                    params![name, 1_700_000_000_i64],
                )
                .map_err(map_rusqlite)?;
                out.push(tx.last_insert_rowid());
            }
            tx.commit().map_err(map_rusqlite)?;
            Ok(out)
        })
        .await
        .expect("seed devices")
}

async fn create_handover(
    svc: &ActService,
    device_ids: &[i64],
    giver: &str,
    receiver: &str,
) -> trackly_app::dto::act::ActDto {
    let payload = ActCreateDto {
        number_override: None,
        giver_name: giver.to_string(),
        receiver_name: receiver.to_string(),
        location_id: None,
        location_name: None,
        notes: None,
        deadline_utc: None,
        handover_date_utc: None,
        items: device_ids
            .iter()
            .map(|&id| ActItemNewDto {
                device_id: id,
                device_ids: Vec::new(),
                quantity: 1,
            })
            .collect(),
    };
    svc.create(payload).await.expect("create handover")
}

async fn create_handover_with_handover_date(
    svc: &ActService,
    device_ids: &[i64],
    giver: &str,
    receiver: &str,
    handover_date_utc: i64,
) -> trackly_app::dto::act::ActDto {
    let payload = ActCreateDto {
        number_override: None,
        giver_name: giver.to_string(),
        receiver_name: receiver.to_string(),
        location_id: None,
        location_name: None,
        notes: None,
        deadline_utc: None,
        handover_date_utc: Some(handover_date_utc),
        items: device_ids
            .iter()
            .map(|&id| ActItemNewDto {
                device_id: id,
                device_ids: Vec::new(),
                quantity: 1,
            })
            .collect(),
    };
    svc.create(payload).await.expect("create handover")
}

async fn create_handover_with_deadline(
    svc: &ActService,
    device_ids: &[i64],
    giver: &str,
    receiver: &str,
    deadline_utc: Option<i64>,
) -> trackly_app::dto::act::ActDto {
    let payload = ActCreateDto {
        number_override: None,
        giver_name: giver.to_string(),
        receiver_name: receiver.to_string(),
        location_id: None,
        location_name: None,
        notes: None,
        deadline_utc,
        handover_date_utc: None,
        items: device_ids
            .iter()
            .map(|&id| ActItemNewDto {
                device_id: id,
                device_ids: Vec::new(),
                quantity: 1,
            })
            .collect(),
    };
    svc.create(payload).await.expect("create handover")
}

/// Returns the substring of `html` strictly between the first
/// `<ol class="...">` opening tag and the following `</ol>`, panicking if
/// either marker is missing. Phase 36 (D-07) replaced the plain `<ul>`
/// plural summary with a numbered `<ol class="device-summary">` whose
/// numbers align 1:1 with the appendix table's № column.
fn extract_first_ol(html: &str) -> &str {
    let tag_start = html
        .find("<ol class=")
        .expect("rendered HTML must contain an <ol class=\"...\"> tag");
    let start = html[tag_start..]
        .find('>')
        .map(|i| tag_start + i + 1)
        .expect("the <ol> opening tag must be closed with '>'");
    let end = html[start..]
        .find("</ol>")
        .map(|i| start + i)
        .expect("the <ol> must be closed");
    &html[start..end]
}

/// Returns the substring of `css` strictly between the first `{` and the
/// following `}` after `selector` — i.e. the declaration body of the first
/// CSS rule whose selector text is `selector`. Panics with an informative
/// message if either the selector or its braces are missing. Used only by
/// the read-only CSS-structural gate below.
fn extract_css_rule_body<'a>(css: &'a str, selector: &str) -> &'a str {
    let sel_start = css
        .find(selector)
        .unwrap_or_else(|| panic!("selector {selector:?} not found in template"));
    let brace_start = css[sel_start..]
        .find('{')
        .map(|i| sel_start + i + 1)
        .unwrap_or_else(|| panic!("selector {selector:?} has no opening brace"));
    let brace_end = css[brace_start..]
        .find('}')
        .map(|i| brace_start + i)
        .unwrap_or_else(|| panic!("selector {selector:?} has no closing brace"));
    &css[brace_start..brace_end]
}

/// DOC-09 / D-02 (N = 1 branch): a single-device handover must render the
/// Word-sample's singular wording «было получено устройство: ⟨name⟩» and must
/// NOT emit the plural summary line/list, which only applies to N > 1. Asserts
/// the exact `.field-row` markup (not a loose substring) so a regression that
/// moves the phrase out of its field-row — or renders the plural branch
/// alongside it — fails here.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn html_handover_single_device_renders_singular_intro_not_plural_summary() {
    let p = make_full_pipeline().await;
    let device_id = seed_device(&p.writer, "HTML-Единственный-Ноутбук").await;
    let act = create_handover(&p.acts, &[device_id], "Иванов И.И.", "Получилов П.П.").await;

    let html = p.acts.render_pdf(act.id).await.expect("render_pdf");

    assert!(
        html.contains(
            "<div class=\"field-row\">было получено устройство: HTML-Единственный-Ноутбук</div>"
        ),
        "N=1 must render the singular field-row «было получено устройство: ⟨name⟩» (D-01/D-02). \
         Body: {:?}",
        html.chars().take(2000).collect::<String>()
    );
    assert_eq!(
        html.matches("было получено устройство:").count(),
        1,
        "N=1 must render the singular label exactly once"
    );
    assert!(
        !html.contains("были получены устройства"),
        "N=1 must NOT render the plural summary line (D-02 — that branch is for N > 1 only). \
         Body: {:?}",
        html.chars().take(2000).collect::<String>()
    );
    assert!(
        !html.contains("<ol class="),
        "N=1 must NOT render the plural device-name <ol> summary list (D-02/D-07). Body: {:?}",
        html.chars().take(2000).collect::<String>()
    );
    assert!(
        !html.contains("<div class=\"appendix\">"),
        "N=1 must NOT render the appendix section at all (DOC-10 SC#1, D-08). Body: {:?}",
        html.chars().take(2000).collect::<String>()
    );
    assert!(
        !html.contains("<table class=\"appendix-table\">"),
        "N=1 must NOT render the appendix table element (the CSS class definition is static \
         and always present in <style>, but the element itself must not appear — DOC-10 SC#1, \
         D-08). Body: {:?}",
        html.chars().take(2000).collect::<String>()
    );
}

/// DOC-09 / D-02 + D-07/D-08 (N > 1 branch, rewritten Phase 36): a
/// multi-device handover must render the plural summary line «были получены
/// устройства:» followed by a numbered `<ol class="device-summary">` naming
/// EVERY device — numbers align 1:1 with the appendix table's № column
/// (D-07). Per D-08 (supersedes Phase 35's D-02a at N > 1), `.device-block`
/// no longer renders AT ALL when N > 1 — the per-device attribution D-02a
/// used to provide via the singular label is now carried entirely by the
/// appendix table's row-per-device shape, verified by the dedicated
/// appendix-structural tests below.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn html_handover_multi_device_renders_plural_summary_listing_every_name() {
    let p = make_full_pipeline().await;
    let device_ids = seed_devices(&p.writer, 3).await;
    let act = create_handover(&p.acts, &device_ids, "Многоустройствов М.М.", "Петров П.П.").await;

    let html = p.acts.render_pdf(act.id).await.expect("render_pdf");

    assert!(
        html.contains("<div class=\"field-row\">были получены устройства:</div>"),
        "N>1 must render the plural summary field-row (D-02). Body: {:?}",
        html.chars().take(2000).collect::<String>()
    );

    // The summary <ol> must name every device exactly once, in order — the
    // plural line must not swallow the per-device names.
    let ol = extract_first_ol(&html);
    for i in 0..3 {
        let expected = format!("<li>HTML-Ноутбук-{i}</li>");
        assert!(
            ol.contains(&expected),
            "plural summary list must contain {expected:?}. List: {ol:?}"
        );
    }
    assert_eq!(
        ol.matches("<li>").count(),
        3,
        "plural summary list must have exactly one <li> per device. List: {ol:?}"
    );

    // D-08: .device-block is GONE ENTIRELY at N > 1 — there is no
    // per-device block on the first sheet at all anymore.
    assert!(
        !html.contains("<div class=\"device-block\">"),
        ".device-block must NOT render at all when act.items | length > 1 (D-08). Body: {:?}",
        html.chars().take(2000).collect::<String>()
    );
    assert!(
        !html.contains("было получено устройство:"),
        "the singular per-device label must not leak onto the first sheet at N>1 (D-08 \
         supersedes Phase 35 D-02a for N>1). Body: {:?}",
        html.chars().take(2000).collect::<String>()
    );
}

/// DOC-11 (appendix structure, D-01): the appendix table must have exactly
/// one `<tbody class="device-group ...">` per device — the row-group that
/// now carries the per-device attribution D-08 removed from the first
/// sheet's `.device-block`.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn html_handover_appendix_table_has_one_row_group_per_device() {
    let p = make_full_pipeline().await;
    let device_ids = seed_devices(&p.writer, 3).await;
    let act = create_handover(&p.acts, &device_ids, "Групповов Г.Г.", "Петров П.П.").await;

    let html = p.acts.render_pdf(act.id).await.expect("render_pdf");

    assert_eq!(
        html.matches("<tbody class=\"device-group").count(),
        3,
        "expected exactly one tbody.device-group per device (N=3, DOC-11). Body: {:?}",
        html.chars().take(2000).collect::<String>()
    );
}

/// DOC-11 / D-07: the numbered summary `<ol>` on the first sheet and the
/// appendix table's № column must agree — both in order (item i's name
/// appears as the (i+1)-th `<li>`) and in the printed number (the (i+1)-th
/// `tbody.device-group`'s first `<td>` equals i+1). This is the exact
/// cross-check D-07 was chosen for: a torn-off appendix sheet can still be
/// matched to the right device via its number.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn html_handover_appendix_ol_numbering_matches_table_number_column() {
    let p = make_full_pipeline().await;
    let device_ids = seed_devices(&p.writer, 3).await;
    let act = create_handover(&p.acts, &device_ids, "Нумератов Н.Н.", "Петров П.П.").await;

    let html = p.acts.render_pdf(act.id).await.expect("render_pdf");

    let ol = extract_first_ol(&html);
    let lis: Vec<&str> = ol
        .split("<li>")
        .skip(1)
        .map(|s| {
            s.split("</li>")
                .next()
                .expect("each <li> segment must close with </li>")
        })
        .collect();
    assert_eq!(lis.len(), 3, "expected 3 <li> entries. List: {ol:?}");

    let groups: Vec<&str> = html.split("<tbody class=\"device-group").skip(1).collect();
    assert_eq!(
        groups.len(),
        3,
        "expected 3 tbody.device-group entries. Body: {:?}",
        html.chars().take(2000).collect::<String>()
    );

    for i in 0..3 {
        let expected_name = format!("HTML-Ноутбук-{i}");
        assert_eq!(
            lis[i], expected_name,
            "li #{} must match act.items order (D-07). List: {:?}",
            i + 1,
            ol
        );

        // First <td> in this device group is the № column; it must equal
        // i+1, matching the <li>'s ordinal position (D-07).
        let group = groups[i];
        let td_start = group
            .find("<td>")
            .map(|p| p + "<td>".len())
            .unwrap_or_else(|| panic!("group {i} missing № <td> open tag. Group: {group:?}"));
        let td_end = group[td_start..]
            .find("</td>")
            .map(|p| td_start + p)
            .unwrap_or_else(|| panic!("group {i} missing № <td> close tag. Group: {group:?}"));
        let number_cell = &group[td_start..td_end];
        assert_eq!(
            number_cell,
            (i + 1).to_string(),
            "tbody.device-group #{} № column must equal {} (D-07). Group head: {:?}",
            i + 1,
            i + 1,
            group.chars().take(200).collect::<String>()
        );
    }
}

/// DOC-11 / D-03: the appendix table's Кол-во column prints a dash for the
/// (default) quantity=1 case and the numeric value once quantity > 1.
/// `ActService::create`'s legacy clone-on-handover path always inserts
/// `act_items.quantity = 1` per row (act_service.rs:411) — exercising the
/// `> 1` branch requires a direct DB UPDATE, the same pattern already used
/// elsewhere in this file/suite for `complectation_at_time`.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn html_handover_appendix_quantity_column_dash_at_one_value_at_more() {
    let p = make_full_pipeline().await;
    let device_ids = seed_devices(&p.writer, 2).await;
    let act = create_handover(&p.acts, &device_ids, "Количествов К.К.", "Петров П.П.").await;

    {
        let act_id = act.id;
        let device_id = act.items[1].device_id;
        p.writer
            .execute(move |conn| {
                conn.execute(
                    "UPDATE act_items SET quantity = 3 WHERE act_id = ?1 AND device_id = ?2",
                    params![act_id, device_id],
                )
                .map(|_| ())
                .map_err(map_rusqlite)
            })
            .await
            .expect("set quantity=3 on second item");
    }

    let html = p.acts.render_pdf(act.id).await.expect("render_pdf");

    let extract_quantity_cell = |device_name: &str| -> String {
        let name_marker = format!("<td>{device_name}</td>");
        let after_name = html
            .find(&name_marker)
            .map(|i| i + name_marker.len())
            .unwrap_or_else(|| panic!("appendix table row for {device_name:?} not found"));
        let rest = &html[after_name..];
        let td_start = rest
            .find("<td>")
            .map(|i| i + "<td>".len())
            .expect("Кол-во <td> open tag");
        let td_end = rest[td_start..]
            .find("</td>")
            .map(|i| td_start + i)
            .expect("Кол-во <td> close tag");
        rest[td_start..td_end].to_string()
    };

    assert_eq!(
        extract_quantity_cell("HTML-Ноутбук-0"),
        "—",
        "quantity=1 (default) must print a dash in the Кол-во column (D-03)"
    );
    assert_eq!(
        extract_quantity_cell("HTML-Ноутбук-1"),
        "3",
        "quantity=3 must print the numeric value in the Кол-во column (D-03)"
    );
}

/// DOC-11 / D-15 + D-16 (regression gate for Nyquist audit, cheap replacement
/// for a geometric print-layout check per the plan's `<interfaces>` note):
/// the shipped template must declare `break-before: page` on `.appendix`
/// (D-16 — the appendix always starts on a fresh sheet) and
/// `break-inside: avoid` on `.appendix-table tbody.device-group` (D-15 — a
/// device's two rows never split across a page). Read-only (`include_str!`),
/// mirrors `html_page_parity.rs`'s pattern exactly — never modifies the
/// template.
#[test]
fn html_handover_appendix_css_declares_forced_break_and_keep_together() {
    let appendix_rule = extract_css_rule_body(ACT_HANDOVER_HTML, ".appendix {");
    assert!(
        appendix_rule.contains("break-before: page"),
        ".appendix rule must declare break-before: page (D-16). Rule: {appendix_rule:?}"
    );

    let device_group_rule =
        extract_css_rule_body(ACT_HANDOVER_HTML, ".appendix-table tbody.device-group {");
    assert!(
        device_group_rule.contains("break-inside: avoid"),
        ".appendix-table tbody.device-group rule must declare break-inside: avoid (D-15). \
         Rule: {device_group_rule:?}"
    );
}

/// DOC-07 / D-03 + D-12 (empty branch): with no `deadline_utc`, the «Сроком
/// до:» row must STILL be rendered (D-12 — unconditional, the pre-Phase-35
/// template hid the whole row) and must carry the `.value-blank` span, i.e.
/// the handwriting underline. Asserts the exact rendered row markup, so
/// re-wrapping the row in an `{% if %}` — or dropping the blank span — fails.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn html_handover_without_deadline_renders_row_with_blank_underline() {
    let p = make_full_pipeline().await;
    let device_id = seed_device(&p.writer, "HTML-БезСрока-Ноутбук").await;
    let act =
        create_handover_with_deadline(&p.acts, &[device_id], "Иванов И.И.", "Получилов П.П.", None)
            .await;
    assert!(
        act.deadline_utc.is_none(),
        "fixture invariant: this act must have no deadline"
    );

    let html = p.acts.render_pdf(act.id).await.expect("render_pdf");

    assert!(
        html.contains(
            "<div class=\"field-row\">Сроком до: <span class=\"value-blank\"></span></div>"
        ),
        "an empty deadline must still render the «Сроком до:» row with a blank underline span \
         (D-03/D-12). Body: {:?}",
        html.chars().take(2000).collect::<String>()
    );
}

/// DOC-07 / D-03 + D-12 (filled branch): with a `deadline_utc`, the same
/// unconditional row must show the Russian-formatted date produced by
/// `act_service`'s `deadline_human` (`format_ru_date`) and must NOT emit the
/// `.value-blank` handwriting underline — the underline is reserved for the
/// empty case (D-10: exactly two legitimate underlines remain).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn html_handover_with_deadline_renders_ru_date_without_blank_underline() {
    let p = make_full_pipeline().await;
    let device_id = seed_device(&p.writer, "HTML-СоСроком-Ноутбук").await;

    // 2023-11-14 UTC — deliberately far from "now" (the act's own date, which
    // SystemClock sets at create time), so the assertion cannot pass by
    // accidentally matching the act date in the subtitle.
    let deadline_utc: i64 = 1_700_000_000;
    let act = create_handover_with_deadline(
        &p.acts,
        &[device_id],
        "Иванов И.И.",
        "Получилов П.П.",
        Some(deadline_utc),
    )
    .await;
    assert_eq!(
        act.deadline_utc,
        Some(deadline_utc),
        "fixture invariant: the deadline must have been persisted"
    );
    assert_ne!(
        format_ru_date(deadline_utc),
        format_ru_date(act.handover_date_utc),
        "fixture invariant: the deadline date must differ from the act date"
    );

    let html = p.acts.render_pdf(act.id).await.expect("render_pdf");

    let expected_row = format!(
        "<div class=\"field-row\">Сроком до: {}</div>",
        format_ru_date(deadline_utc)
    );
    assert!(
        html.contains(&expected_row),
        "a filled deadline must render as the RU-formatted `deadline_human` on the «Сроком до:» \
         row — expected {expected_row:?}. Body: {:?}",
        html.chars().take(2000).collect::<String>()
    );
    assert!(
        !html.contains("<span class=\"value-blank\"></span>"),
        "a filled deadline must NOT emit the blank handwriting underline (D-03/D-10). Body: {:?}",
        html.chars().take(2000).collect::<String>()
    );
}

/// D-14 item 1: both acts' HTML output contains all required
/// blocks/fields, plus the logo `data:` URI.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn html_handover_contains_required_blocks_and_logo() {
    let p = make_full_pipeline().await;

    // Save a BLOB logo via the real production path (OrgDbService::save_logo).
    let org_db = Arc::new(OrgDbService::new(
        p.writer.clone(),
        p._readers.clone(),
        Arc::new(SystemClock),
        Arc::new(Paths::resolve_for_exe_dir(p._dir.path().to_path_buf()).expect("paths")),
    ));
    org_db
        .save_logo(
            &Identity::trusted_admin(),
            LOGO_PNG.to_vec(),
            "image/png".to_string(),
        )
        .await
        .expect("save_logo");

    let device_id = seed_device(&p.writer, "HTML-Логотест-Ноутбук").await;
    let act = create_handover(&p.acts, &[device_id], "Выдалов В.В.", "Получилов П.П.").await;

    let html = p.acts.render_pdf(act.id).await.expect("render_pdf");

    for expected in ["Акт приема-передачи", "Выдал:", "Получил:", "Подпись"] {
        assert!(
            html.contains(expected),
            "expected block/label {expected:?} missing from handover HTML. Head: {:?}",
            html.chars().take(500).collect::<String>()
        );
    }
    // Phase 35 D-06: giver_name is now printed in the signature block (the
    // separate "ФИО" sublabel was removed, D-07 — the printed name replaces
    // it).
    assert!(
        html.contains("Выдалов В.В."),
        "expected printed giver_name in signature block. Head: {:?}",
        html.chars().take(500).collect::<String>()
    );
    // Act number present.
    assert!(
        html.contains(&act.number_raw.to_string()),
        "act number missing from rendered HTML"
    );
    // Logo present as a data: URI.
    assert!(
        html.contains("data:image/png;base64,"),
        "logo must appear as a data: URI in HTML output. Head: {:?}",
        html.chars().take(500).collect::<String>()
    );
}

/// D-14 item 1: acceptance act's HTML output contains required blocks +
/// giver/receiver names.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn html_acceptance_contains_required_blocks() {
    let p = make_full_pipeline().await;
    let device_id = seed_device(&p.writer, "HTML-Приемка-Ноутбук").await;

    let html = p
        .acts
        .render_acceptance_pdf(
            device_id,
            "Отдалов О.О.".to_string(),
            "Принялов П.П.".to_string(),
            1_700_000_000,
        )
        .await
        .expect("render_acceptance_pdf");

    assert!(
        html.contains("Документ приёма устройства"),
        "expected acceptance document title. Head: {:?}",
        html.chars().take(500).collect::<String>()
    );
    assert!(
        html.contains("Отдалов О.О."),
        "giver name missing from rendered HTML"
    );
    assert!(
        html.contains("Принялов П.П."),
        "receiver name missing from rendered HTML"
    );
    // Phase 35 D-09: the duplicate "Кто передал"/"Кто принял" table rows
    // were removed — giver/receiver names now appear only in the signature
    // block.
    assert!(
        !html.contains("Кто передал"),
        "duplicate 'Кто передал' table row should have been removed"
    );
}

/// PRN-01/ORG-02 (Plan 20-05, Task 1): `render_acceptance_pdf` must be at
/// full org-requisite parity with `render_pdf` — every field populated via
/// the production write path (`OrgDbService::save_fields`), including the
/// new `address_line2` (ORG-02), must appear in BOTH rendered HTML strings.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn html_acceptance_full_org_parity_with_handover() {
    let p = make_full_pipeline().await;

    let org_db = Arc::new(OrgDbService::new(
        p.writer.clone(),
        p._readers.clone(),
        Arc::new(SystemClock),
        Arc::new(Paths::resolve_for_exe_dir(p._dir.path().to_path_buf()).expect("paths")),
    ));
    org_db
        .save_fields(
            &Identity::trusted_admin(),
            OrgPatch {
                org_name: "ООО Паритет".into(),
                inn: "7712345678".into(),
                kpp: "771201001".into(),
                address: "г. Москва, ул. Тестовая, д. 1".into(),
                phone: "+7 495 000-00-00".into(),
                fax: "+7 495 000-00-01".into(),
                email: "info@paritet.test".into(),
                okpo: "12345678".into(),
                ogrn: "1027700000000".into(),
                address_line2: "офис 305, корпус 2".into(),
                full_name: String::new(),
            },
        )
        .await
        .expect("save_fields");

    let device_id = seed_device(&p.writer, "HTML-Паритет-Ноутбук").await;
    let handover_act =
        create_handover(&p.acts, &[device_id], "Выдалов В.В.", "Получилов П.П.").await;

    let handover_html = p
        .acts
        .render_pdf(handover_act.id)
        .await
        .expect("render_pdf");
    let acceptance_html = p
        .acts
        .render_acceptance_pdf(
            device_id,
            "Отдалов О.О.".to_string(),
            "Принялов П.П.".to_string(),
            1_700_000_000,
        )
        .await
        .expect("render_acceptance_pdf");

    let expected = [
        "7712345678",         // inn
        "+7 495 000-00-00",   // phone
        "+7 495 000-00-01",   // fax
        "info@paritet.test",  // email
        "12345678",           // okpo
        "1027700000000",      // ogrn
        "офис 305, корпус 2", // address_line2
    ];
    for value in expected {
        assert!(
            handover_html.contains(value),
            "expected {value:?} in handover (render_pdf) HTML. Head: {:?}",
            handover_html.chars().take(500).collect::<String>()
        );
        assert!(
            acceptance_html.contains(value),
            "expected {value:?} in acceptance (render_acceptance_pdf) HTML — \
             PRN-01 parity failure. Head: {:?}",
            acceptance_html.chars().take(500).collect::<String>()
        );
    }
}

/// ORG-01/D-09 (Plan 20-05, Task 2): an SVG logo containing an embedded
/// `<script>` tag must be embedded EXCLUSIVELY as a `data:` URI inside
/// `<img>` — the raw `<script>` must never appear as literal, executable
/// markup in the rendered document DOM. Locks the core security invariant
/// with a concrete adversarial payload rather than a one-time review.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn html_svg_logo_with_script_embeds_img_only_no_inline_script() {
    let p = make_full_pipeline().await;

    let org_db = Arc::new(OrgDbService::new(
        p.writer.clone(),
        p._readers.clone(),
        Arc::new(SystemClock),
        Arc::new(Paths::resolve_for_exe_dir(p._dir.path().to_path_buf()).expect("paths")),
    ));
    org_db
        .save_logo(
            &Identity::trusted_admin(),
            LOGO_SVG_WITH_SCRIPT.to_vec(),
            "image/svg+xml".to_string(),
        )
        .await
        .expect("save_logo");

    let device_id = seed_device(&p.writer, "HTML-XSS-Ноутбук").await;
    let act = create_handover(&p.acts, &[device_id], "Выдалов В.В.", "Получилов П.П.").await;

    let html = p.acts.render_pdf(act.id).await.expect("render_pdf");

    // (a) The raw <script> tag must never appear inline in the rendered DOM.
    assert!(
        !html.contains("<script>"),
        "SVG-embedded <script> must NOT appear as literal markup in the \
         rendered document (ORG-01/D-09 XSS invariant). Head: {:?}",
        html.chars().take(500).collect::<String>()
    );
    // (b) Sanity check: the logo DID embed as a data: URI, so (a) is not
    // vacuously true because the logo silently failed to embed.
    assert!(
        html.contains("data:image/svg+xml;base64,"),
        "SVG logo must embed as a data: URI (proves the <script> absence \
         assertion is non-vacuous). Head: {:?}",
        html.chars().take(500).collect::<String>()
    );
    // (c) The logo is embedded exclusively via <img src="data:...">, not as
    // inline <svg>/raw markup elsewhere in the document.
    assert!(
        html.contains("<img src=\"data:image/svg+xml;base64,"),
        "SVG logo must be embedded exclusively via <img src=\"data:...\">. \
         Head: {:?}",
        html.chars().take(500).collect::<String>()
    );
}

/// WR-01 regression gate: the read-side logo mime allowlist must be enforced
/// on BOTH act render paths, not only on `report_service::export_pdf`.
///
/// Phase 34 made all three printed forms share one `| safe` sink
/// (`<img src="{{ org.logo_data_uri | safe }}">` in `_header.html`), so an
/// unvalidated `logo_mime` read out of the mutable `org_settings` column
/// would be interpolated straight into an HTML attribute. `save_logo`'s
/// write-side allowlist is bypassed here deliberately (direct UPDATE) to
/// simulate a mutated/legacy DB row — the read side must fail closed on its
/// own.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn html_acts_drop_logo_when_stored_mime_is_disallowed() {
    let p = make_full_pipeline().await;

    let org_db = Arc::new(OrgDbService::new(
        p.writer.clone(),
        p._readers.clone(),
        Arc::new(SystemClock),
        Arc::new(Paths::resolve_for_exe_dir(p._dir.path().to_path_buf()).expect("paths")),
    ));
    org_db
        .save_logo(
            &Identity::trusted_admin(),
            LOGO_PNG.to_vec(),
            "image/png".to_string(),
        )
        .await
        .expect("save_logo");

    // Attribute-breaking mime, as could reach the column via a hand-edited /
    // migrated DB. Never writable through `save_logo`.
    let hostile_mime = "image/png\" onerror=\"alert(1)";
    let hostile_mime_owned = hostile_mime.to_string();
    p.writer
        .execute(move |conn| {
            conn.execute(
                "UPDATE org_settings SET logo_mime = ?1 WHERE id = 1",
                params![hostile_mime_owned],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("force disallowed logo_mime");

    let device_id = seed_device(&p.writer, "HTML-BadMime-Ноутбук").await;
    let act = create_handover(&p.acts, &[device_id], "Выдалов В.В.", "Получилов П.П.").await;

    let handover_html = p.acts.render_pdf(act.id).await.expect("render_pdf");
    let acceptance_html = p
        .acts
        .render_acceptance_pdf(
            device_id,
            "Отдалов О.О.".to_string(),
            "Принялов П.П.".to_string(),
            1_700_000_000,
        )
        .await
        .expect("render_acceptance_pdf");

    for (form, html) in [
        ("act_handover", &handover_html),
        ("act_acceptance", &acceptance_html),
    ] {
        assert!(
            !html.contains("data:image/png"),
            "{form}: logo must be dropped entirely when the stored mime is not \
             on the allowlist. Head: {:?}",
            html.chars().take(500).collect::<String>()
        );
        assert!(
            !html.contains("onerror"),
            "{form}: a disallowed mime must never reach the `| safe` src \
             attribute. Head: {:?}",
            html.chars().take(500).collect::<String>()
        );
    }
}

/// D-14 item 2: 1-vs-N devices — a multi-device handover renders every
/// device's identity AND a long field value in full, no truncation.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn html_handover_multi_device_all_items_present_no_truncation() {
    let p = make_full_pipeline().await;
    let device_ids = seed_devices(&p.writer, 3).await;
    let act = create_handover(&p.acts, &device_ids, "Многоустройствов М.М.", "Петров П.П.").await;

    let long_kit = "Блок питания, кабель питания, кабель HDMI, сумка для переноски, \
        документация на русском языке, гарантийный талон, комплект крепёжных винтов, \
        салфетка для протирки экрана СЕРЕДИНА-МАРКЕР-HTML хвостовая часть строки"
        .to_string();
    assert!(
        long_kit.chars().count() > 150,
        "test fixture string must exceed 150 chars"
    );

    let first_item = act.items.first().expect("at least one item");
    {
        let act_id = act.id;
        let device_id = first_item.device_id;
        let value = long_kit.clone();
        p.writer
            .execute(move |conn| {
                conn.execute(
                    "UPDATE act_items SET complectation_at_time = ?1 \
                     WHERE act_id = ?2 AND device_id = ?3",
                    params![value, act_id, device_id],
                )
                .map(|_| ())
                .map_err(map_rusqlite)
            })
            .await
            .expect("set complectation_at_time");
    }

    let html = p.acts.render_pdf(act.id).await.expect("render_pdf");

    for i in 0..3 {
        let expected_name = format!("HTML-Ноутбук-{i}");
        assert!(
            html.contains(&expected_name),
            "device name {expected_name:?} missing from rendered HTML. Head: {:?}",
            html.chars().take(800).collect::<String>()
        );
    }

    assert!(
        !html.contains('…'),
        "long field must not be truncated with ellipsis in HTML output"
    );
    assert!(
        html.contains("СЕРЕДИНА-МАРКЕР-HTML"),
        "middle-of-value marker missing — long field appears to have been cut off"
    );
}

/// D-14 item 3: fallback-to-embedded-default when the templates directory
/// has no on-disk `act_handover.html` file.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn html_falls_back_to_embedded_default_when_file_absent() {
    let p = make_full_pipeline().await;
    let device_id = seed_device(&p.writer, "HTML-Fallback-Ноутбук").await;
    let act = create_handover(&p.acts, &[device_id], "Иванов И.И.", "Петров П.П.").await;

    // make_full_pipeline's TempDir never had materialize_defaults_on_startup
    // called against it (that's an html_templates-module concern wired at
    // AppCtx::build in production, not by ActService itself) — so
    // `templates/act_handover.html` does not exist on disk. render_pdf must
    // still succeed via the embedded default.
    let html = p
        .acts
        .render_pdf(act.id)
        .await
        .expect("render_pdf must succeed via embedded default when file absent");
    assert!(
        html.contains("Акт приема-передачи"),
        "expected embedded-default marker content. Head: {:?}",
        html.chars().take(300).collect::<String>()
    );
}

/// D-14 item 3: when `templates/act_handover.html` IS present on disk, it is
/// used instead of the embedded default — and editing the file changes the
/// rendered output.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn html_uses_file_when_present_and_edit_changes_output() {
    let p = make_full_pipeline().await;
    let device_id = seed_device(&p.writer, "HTML-FileOverride-Ноутбук").await;
    let act = create_handover(&p.acts, &[device_id], "Иванов И.И.", "Петров П.П.").await;

    let paths = Paths::resolve_for_exe_dir(p._dir.path().to_path_buf()).expect("paths");
    let templates_dir = paths.templates_dir();
    std::fs::create_dir_all(templates_dir).expect("create templates dir");

    // Write a minimal but real MiniJinja HTML template using only variables
    // the real ctx provides, with a unique sentinel marker.
    let sentinel = "<!-- CUSTOM-TEST-MARKER-html-uses-file -->";
    let custom_template = format!(
        "<!DOCTYPE html>\n{sentinel}\n<html><body>\n\
         <div>{{{{ act.receiver_name }}}}</div>\n\
         {{%- for item in act.items %}}<div>{{{{ item.name }}}}</div>{{%- endfor %}}\n\
         </body></html>\n"
    );
    std::fs::write(templates_dir.join("act_handover.html"), &custom_template)
        .expect("write custom template");

    let html = p
        .acts
        .render_pdf(act.id)
        .await
        .expect("render_pdf with custom on-disk template");

    assert!(
        html.contains(sentinel),
        "expected sentinel marker proving the on-disk file was used, not the embedded default. \
         Got: {:?}",
        html.chars().take(300).collect::<String>()
    );
    assert!(
        html.contains("Петров П.П."),
        "custom template's ctx interpolation must still work"
    );
}

/// D-14 item 4: rendered HTML for both acts contains no `http(s)://`
/// references in the markup — self-contained/offline guarantee. The only
/// external-looking scheme permitted is `data:`, which never contains the
/// substring `http`.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn html_is_offline_safe_no_external_links() {
    let p = make_full_pipeline().await;

    let org_db = Arc::new(OrgDbService::new(
        p.writer.clone(),
        p._readers.clone(),
        Arc::new(SystemClock),
        Arc::new(Paths::resolve_for_exe_dir(p._dir.path().to_path_buf()).expect("paths")),
    ));
    org_db
        .save_logo(
            &Identity::trusted_admin(),
            LOGO_PNG.to_vec(),
            "image/png".to_string(),
        )
        .await
        .expect("save_logo");

    let device_id = seed_device(&p.writer, "HTML-Offline-Ноутбук").await;
    let act = create_handover(&p.acts, &[device_id], "Иванов И.И.", "Петров П.П.").await;

    let handover_html = p.acts.render_pdf(act.id).await.expect("render_pdf");
    assert!(
        !handover_html.to_lowercase().contains("http://")
            && !handover_html.to_lowercase().contains("https://"),
        "handover HTML must not contain any http(s):// reference (offline/no-CDN guarantee)"
    );
    // Sanity: the offline-safety assertion above didn't just pass vacuously —
    // confirm the logo (a data: URI, which legitimately contains no http(s))
    // is indeed present.
    assert!(
        handover_html.contains("data:image/png;base64,"),
        "expected data: URI logo to be present (proves the absence check above wasn't vacuous)"
    );

    let acceptance_html = p
        .acts
        .render_acceptance_pdf(
            device_id,
            "Отдалов О.О.".to_string(),
            "Принялов П.П.".to_string(),
            1_700_000_000,
        )
        .await
        .expect("render_acceptance_pdf");
    assert!(
        !acceptance_html.to_lowercase().contains("http://")
            && !acceptance_html.to_lowercase().contains("https://"),
        "acceptance HTML must not contain any http(s):// reference (offline/no-CDN guarantee)"
    );
}

/// ACT-01 (Phase 19 Plan 01): `render_pdf`'s `act.date`/`act.date_human` must
/// derive from `handover_date_utc` (the user-entered «Когда отдали» date),
/// not `created_at_utc` (the row-insertion timestamp, which `SystemClock`
/// sets to "now" at create time). Uses a fixture where the two values are
/// deliberately far apart so the assertion cannot pass vacuously.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn html_render_pdf_act_date_uses_handover_date_not_created_at() {
    let p = make_full_pipeline().await;
    let device_id = seed_device(&p.writer, "HTML-DateSource-Ноутбук").await;

    // 2020-09-13 UTC — far from "now" (act creation time via SystemClock).
    let handover_date_utc: i64 = 1_600_000_000;
    let act = create_handover_with_handover_date(
        &p.acts,
        &[device_id],
        "Даталов Д.Д.",
        "Приемов П.П.",
        handover_date_utc,
    )
    .await;

    assert_ne!(
        act.handover_date_utc, act.created_at_utc,
        "fixture invariant broken: handover_date_utc must differ from created_at_utc"
    );

    let html = p.acts.render_pdf(act.id).await.expect("render_pdf");

    // The template (act_handover.html) renders `act.date_human` (RU format)
    // in the subtitle, not the raw ISO `act.date` — assert on what is
    // actually rendered.
    let expected_ru = format_ru_date(act.handover_date_utc);
    let wrong_ru = format_ru_date(act.created_at_utc);

    assert!(
        html.contains(&expected_ru),
        "expected handover_date_utc-derived RU date {expected_ru:?} in rendered HTML. \
         Head: {:?}",
        html.chars().take(500).collect::<String>()
    );
    assert!(
        !html.contains(&wrong_ru),
        "created_at_utc-derived RU date {wrong_ru:?} must NOT appear in rendered HTML \
         (it would prove the date source regressed back to created_at_utc)"
    );
}

/// ACT-01: the parent block on a return-act's rendered HTML must also
/// reflect `handover_date_utc` (of the parent handover act), not
/// `created_at_utc`.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn html_render_pdf_parent_block_date_uses_handover_date_not_created_at() {
    let p = make_full_pipeline().await;
    let device_id = seed_device(&p.writer, "HTML-DateSource-Parent-Ноутбук").await;

    let handover_date_utc: i64 = 1_600_000_000;
    let handover = create_handover_with_handover_date(
        &p.acts,
        &[device_id],
        "Даталов Д.Д.",
        "Приемов П.П.",
        handover_date_utc,
    )
    .await;

    assert_ne!(
        handover.handover_date_utc, handover.created_at_utc,
        "fixture invariant broken: handover_date_utc must differ from created_at_utc"
    );

    // Phase 22 (D-05): a return's `handover_date_utc` is now its OWN field
    // (no longer inherited from the parent) — pass an explicit value here,
    // distinct from both the parent's `handover_date_utc` and `created_at_utc`,
    // so this fixture stays deterministic and independent of `do_return`'s
    // back-compat `now()` fallback (used only when this field is omitted).
    let return_date_utc: i64 = 1_650_000_000;
    assert_ne!(
        return_date_utc, handover_date_utc,
        "fixture invariant: return's own date must differ from the parent's"
    );

    let first_item = handover.items.first().expect("at least one item");
    let return_act = p
        .acts
        .do_return(
            handover.id,
            ActReturnDto {
                bulk_condition: Some("Хорошее".into()),
                bulk_location_id: None,
                bulk_location_name: None,
                apply_to_all: true,
                giver_name: None,
                receiver_name: None,
                handover_date_utc: Some(return_date_utc),
                items: vec![ActReturnItemDto {
                    act_item_id: first_item.id,
                    device_id: first_item.device_id,
                    device_ids: vec![first_item.device_id],
                    quantity: 1,
                    condition_override: None,
                    location_id_override: None,
                    location_name_override: None,
                }],
            },
        )
        .await
        .expect("do_return");

    let html = p
        .acts
        .render_pdf(return_act.id)
        .await
        .expect("render_pdf return act");

    // Template renders `act.parent.date_human` (RU format), not the raw
    // ISO `act.parent.date` — assert on what is actually rendered.
    let expected_ru = format_ru_date(handover.handover_date_utc);
    let wrong_ru = format_ru_date(handover.created_at_utc);

    assert!(
        html.contains(&expected_ru),
        "expected parent's handover_date_utc-derived RU date {expected_ru:?} in rendered \
         return-act HTML. Head: {:?}",
        html.chars().take(500).collect::<String>()
    );
    assert!(
        !html.contains(&wrong_ru),
        "parent's created_at_utc-derived RU date {wrong_ru:?} must NOT appear in rendered \
         HTML (it would prove the parent-block date source regressed back to created_at_utc)"
    );
}
