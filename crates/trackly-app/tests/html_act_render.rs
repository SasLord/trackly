//! Phase 16 Plan 05 — Task 2: dedicated HTML-generation test suite covering
//! D-14 items 1-4 (block/field presence + logo, 1-vs-N devices, fallback-vs-
//! file, offline/no-CDN) for both acts (`act_handover` / `act_acceptance`),
//! rendered through the full `ActService::render_pdf`/`render_acceptance_pdf`
//! pipeline (Phase 16 Plan 02's HTML rewiring).

use std::sync::Arc;

use rusqlite::params;
use trackly_app::dto::act::{
    ActCreateDto, ActItemNewDto, ActReturnDto, ActReturnItemDto,
};
use trackly_app::pdf::PdfRenderer;
use trackly_app::services::act_service::{format_iso_date, format_ru_date};
use trackly_app::services::{ActService, OrgDbService, OrganizationService, TemplateService};
use trackly_core::auth::Identity;
use trackly_core::primitives::clock::Clock;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::db::{pools::ReaderPool, writer_worker::WriterHandle};
use trackly_infra::error_conversions::map_rusqlite;
use trackly_infra::test_support::test_writer_and_readers;
use trackly_infra::Paths;

const LOGO_PNG: &[u8] = include_bytes!("fixtures/logo_test.png");

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

    for expected in ["Акт приема-передачи", "Выдал", "Получил", "Подпись", "ФИО"]
    {
        assert!(
            html.contains(expected),
            "expected block/label {expected:?} missing from handover HTML. Head: {:?}",
            html.chars().take(500).collect::<String>()
        );
    }
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

    let expected_iso = format_iso_date(act.handover_date_utc);
    let expected_ru = format_ru_date(act.handover_date_utc);
    let wrong_iso = format_iso_date(act.created_at_utc);

    assert!(
        html.contains(&expected_iso),
        "expected handover_date_utc-derived ISO date {expected_iso:?} in rendered HTML. \
         Head: {:?}",
        html.chars().take(500).collect::<String>()
    );
    assert!(
        html.contains(&expected_ru),
        "expected handover_date_utc-derived RU date {expected_ru:?} in rendered HTML. \
         Head: {:?}",
        html.chars().take(500).collect::<String>()
    );
    assert!(
        !html.contains(&wrong_iso),
        "created_at_utc-derived ISO date {wrong_iso:?} must NOT appear in rendered HTML \
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

    let expected_iso = format_iso_date(handover.handover_date_utc);
    let wrong_iso = format_iso_date(handover.created_at_utc);

    assert!(
        html.contains(&expected_iso),
        "expected parent's handover_date_utc-derived ISO date {expected_iso:?} in rendered \
         return-act HTML. Head: {:?}",
        html.chars().take(500).collect::<String>()
    );
    assert!(
        !html.contains(&wrong_iso),
        "parent's created_at_utc-derived ISO date {wrong_iso:?} must NOT appear in rendered \
         HTML (it would prove the parent-block date source regressed back to created_at_utc)"
    );
}
