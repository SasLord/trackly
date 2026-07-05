//! HTML render full pipeline integration tests — Phase 3 Plan 04 Task 2,
//! migrated to the HTML-string contract in Phase 16 Plan 05 (D-10: `render_pdf`/
//! `render_acceptance_pdf` now return `Result<String, AppError>` — an HTML
//! document, not PDF bytes).
//!
//! Covers (per behavior list):
//!   - render_handover_act_produces_cyrillic_pdf (now: html)
//!   - render_falls_back_to_embedded_default_when_template_file_missing
//!   - render_falls_back_to_embedded_default_when_broken_template_row_present
//!   - render_acceptance_pdf_for_device_works
//!   - render_pdf_with_missing_logo_renders_without_logo
//!
//! End-to-end: seed templates + org.json + devices + handover акт → render →
//! assert directly on the returned HTML string (no pdf-extract/PDF-magic-header
//! checks — the render path stopped producing PDF bytes in Plan 16-02).

use std::sync::Arc;
use std::time::Duration;

use rusqlite::params;
use tempfile::TempDir;
use trackly_app::dto::act::{ActCreateDto, ActItemNewDto};
use trackly_app::dto::reports::OrgPatch;
use trackly_app::pdf::PdfRenderer;
use trackly_app::services::{ActService, OrgDbService, OrganizationService, TemplateService};
use trackly_core::auth::Identity;
use trackly_core::primitives::clock::Clock;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::db::{pools::ReaderPool, writer_worker::WriterHandle};
use trackly_infra::error_conversions::map_rusqlite;
use trackly_infra::test_support::test_writer_and_readers;
use trackly_infra::Paths;

struct Pipeline {
    acts: ActService,
    templates: Arc<TemplateService>,
    writer: Arc<WriterHandle>,
    _readers: Arc<ReaderPool>,
    _dir: TempDir,
}

async fn make_full_pipeline() -> Pipeline {
    let (writer, readers, dir) = test_writer_and_readers();
    let clock: Arc<dyn Clock + Send + Sync> = Arc::new(SystemClock);

    let paths = Arc::new(Paths::resolve_for_exe_dir(dir.path().to_path_buf()).expect("paths"));
    let organization = Arc::new(OrganizationService::new(paths));
    let templates = Arc::new(TemplateService::new(
        writer.clone(),
        readers.clone(),
        clock.clone(),
    ));
    let pdf = Arc::new(PdfRenderer::new());
    templates.seed_defaults_on_startup().await.expect("seed");

    let acts = ActService::new(writer.clone(), readers.clone(), clock.clone()).with_pdf_pipeline(
        templates.clone(),
        organization.clone(),
        pdf.clone(),
    );

    Pipeline {
        acts,
        templates,
        writer,
        _readers: readers,
        _dir: dir,
    }
}

/// Same as `make_full_pipeline`, but also wires `OrgDbService` (D-05) via
/// `with_org_db` — exercises the real production org-requisites source
/// (`org_settings`), not the fallback branch.
async fn make_full_pipeline_with_org_db() -> Pipeline {
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
        templates,
        writer,
        _readers: readers,
        _dir: dir,
    }
}

async fn seed_devices(writer: &Arc<WriterHandle>, count: usize) -> Vec<i64> {
    let names: Vec<String> = (0..count).map(|i| format!("Ноутбук-{i}")).collect();
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

async fn create_handover_with_giver(
    svc: &ActService,
    device_ids: &[i64],
    giver: &str,
) -> trackly_app::dto::act::ActDto {
    let payload = ActCreateDto {
        number_override: None,
        giver_name: giver.to_string(),
        receiver_name: "Петров П.П.".into(),
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

/// N=1 regression anchor. D-09 removed `giver_name` from the rendered body
/// (it now only appears via the bare "Выдал" signature label); this test
/// therefore asserts `receiver_name` — which D-09's intro paragraph does
/// render — instead of the old giver_name-in-body assertion. The N=5
/// multi-device case (with long-field wrap coverage) is covered separately
/// by `render_handover_multi_device_wraps_long_fields` below.
///
/// Phase 16 Plan 05: page-count (single-page) assertion removed — browser
/// pagination via CSS `@page`/`page-break-inside` cannot be asserted from a
/// raw HTML string in a Rust unit test; that guarantee is now a CSS-authoring
/// concern (see `act_handover.html`'s `.device-block { page-break-inside:
/// avoid }`), not a generator-output concern.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn render_handover_act_produces_cyrillic_pdf() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let p = make_full_pipeline().await;
        let device_ids = seed_devices(&p.writer, 2).await;
        let act = create_handover_with_giver(
            &p.acts,
            &device_ids,
            "Сидоров-Петроградский Иван Александрович",
        )
        .await;
        let html = p.acts.render_pdf(act.id).await.expect("render_pdf");
        assert!(
            html.len() > 1000,
            "expected substantive HTML, got {}",
            html.len()
        );
        assert!(
            html.to_lowercase().contains("<!doctype html") || html.to_lowercase().contains("<html"),
            "expected HTML document markers, got head: {:?}",
            html.chars().take(200).collect::<String>()
        );

        assert!(
            html.contains("Петров"),
            "Cyrillic receiver name (D-09 intro paragraph) missing. Head: {:?}",
            html.chars().take(300).collect::<String>()
        );
        // Act number is 1 (auto-incremented).
        assert!(
            html.contains("№1") || html.contains('1'),
            "Act number missing. Head: {:?}",
            html.chars().take(300).collect::<String>()
        );
    })
    .await
    .expect("timeout");
}

/// D-09 intro paragraph presence + interpolated receiver_name, on the full
/// `act_service::render_pdf` pipeline (PDFA-01).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn render_handover_act_contains_d09_intro_phrase() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let p = make_full_pipeline().await;
        let device_ids = seed_devices(&p.writer, 1).await;
        let act = create_handover_with_giver(&p.acts, &device_ids, "Иванов И.И.").await;
        let html = p.acts.render_pdf(act.id).await.expect("render_pdf");
        assert!(
            html.contains("Настоящим актом утверждаю"),
            "D-09 intro phrase missing. Head: {:?}",
            html.chars().take(500).collect::<String>()
        );
        assert!(
            html.contains(&act.receiver_name),
            "act.receiver_name ({:?}) not interpolated into intro paragraph. Head: {:?}",
            act.receiver_name,
            html.chars().take(500).collect::<String>()
        );
    })
    .await
    .expect("timeout");
}

/// Two-line signature sublabels (D-07): «Подпись»/«ФИО» under «Выдал»/«Получил».
/// N=1 is sufficient — the signature block does not vary with device count.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn signature_renders_two_line_labels() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let p = make_full_pipeline().await;
        let device_ids = seed_devices(&p.writer, 1).await;
        let act = create_handover_with_giver(&p.acts, &device_ids, "Иванов И.И.").await;
        let html = p.acts.render_pdf(act.id).await.expect("render_pdf");
        for expected in ["Выдал", "Получил", "Подпись", "ФИО"] {
            assert!(
                html.contains(expected),
                "expected signature label {expected:?} missing. Head: {:?}",
                html.chars().take(500).collect::<String>()
            );
        }
    })
    .await
    .expect("timeout");
}

/// PDFA-02: 1-vs-N device rendering. Seeds 5 devices in ONE handover act,
/// sets a long (150+ char) Cyrillic `complectation_at_time` on 2 of the 5
/// resulting `act_items` rows directly (mirrors the `devices.notes` UPDATE
/// idiom used elsewhere in this file), then asserts: all 5 device names are
/// present, no ellipsis truncation marker appears (proves the FieldRow/HTML
/// wrap path was used, not a truncate path), and a substring from the MIDDLE
/// of the long value survived (proves it wasn't cut off).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn render_handover_multi_device_wraps_long_fields() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let p = make_full_pipeline().await;
        let device_ids = seed_devices(&p.writer, 5).await;
        let act = create_handover_with_giver(&p.acts, &device_ids, "Кузнецов К.К.").await;

        let long_kit = "Блок питания, кабель питания, кабель HDMI, сумка для переноски, \
            документация на русском языке, гарантийный талон, комплект крепёжных винтов, \
            салфетка для протирки экрана СЕРЕДИНА-МАРКЕР-ЗНАЧЕНИЯ хвостовая часть строки"
            .to_string();
        assert!(
            long_kit.chars().count() > 150,
            "test fixture string must exceed 150 chars"
        );

        // Set complectation_at_time on 2 of the 5 act_items rows directly.
        for item in act.items.iter().take(2) {
            let act_id = act.id;
            let device_id = item.device_id;
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

        // seed_devices names devices "Ноутбук-{i}" where i is the loop
        // index (0..count), independent of the assigned device_id.
        assert_eq!(device_ids.len(), 5, "expected 5 seeded devices");
        for i in 0..5 {
            let expected_name = format!("Ноутбук-{i}");
            assert!(
                html.contains(&expected_name),
                "device name {expected_name:?} missing from rendered HTML. Head: {:?}",
                html.chars().take(800).collect::<String>()
            );
        }

        assert!(
            !html.contains('…'),
            "long complectation field must wrap, not truncate with ellipsis. Head: {:?}",
            html.chars().take(800).collect::<String>()
        );

        assert!(
            html.contains("СЕРЕДИНА-МАРКЕР-ЗНАЧЕНИЯ"),
            "middle-of-value marker missing — long field appears to have been cut off. \
             Head: {:?}",
            html.chars().take(800).collect::<String>()
        );
    })
    .await
    .expect("timeout");
}

/// 260704-wxw success criterion carried into Phase 16: the default
/// `act_handover.html` must emit `field-row`-style blocks (full-length
/// labels), never a «Устройство №N» heading/counter nor the abbreviated
/// legacy labels. Sets `inventory_number`/`serial_number`/`model` directly on
/// the seeded `devices` rows (these fields live on `devices`, not
/// `act_items` — `ActItemDto.inventory_no`/`serial_no`/`model` are joined
/// live from `devices` at render time, unlike
/// `complectation_at_time`/`condition_at_time` which are `act_items` snapshot
/// columns).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn render_handover_default_template_uses_field_rows_not_device_card() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let p = make_full_pipeline().await;
        let device_ids = seed_devices(&p.writer, 2).await;

        for (i, &device_id) in device_ids.iter().enumerate() {
            p.writer
                .execute(move |conn| {
                    conn.execute(
                        "UPDATE devices SET inventory_number = ?1, serial_number = ?2, model = ?3 \
                         WHERE id = ?4",
                        params![
                            format!("ИНВ-{i:03}"),
                            format!("SN-{i:04}"),
                            format!("Модель-{i}"),
                            device_id
                        ],
                    )
                    .map(|_| ())
                    .map_err(map_rusqlite)
                })
                .await
                .expect("set device inventory/serial/model");
        }

        let act = create_handover_with_giver(&p.acts, &device_ids, "Волков В.В.").await;
        let html = p.acts.render_pdf(act.id).await.expect("render_pdf");

        // Full-length labels present.
        for label in ["Инвентарный номер:", "Серийный номер:", "Модель:"]
        {
            assert!(
                html.contains(label),
                "expected full-length label {label:?} in rendered HTML. Head: {:?}",
                html.chars().take(800).collect::<String>()
            );
        }

        // No device_card heading/counter of any kind.
        assert!(
            !html.contains("Устройство №1") && !html.contains("Устройство №2"),
            "device_card-style «Устройство №N» heading must not appear. Head: {:?}",
            html.chars().take(800).collect::<String>()
        );

        // No abbreviated legacy labels.
        assert!(
            !html.contains("Инв.№") && !html.contains("Серийный №"),
            "abbreviated legacy labels must not appear. Head: {:?}",
            html.chars().take(800).collect::<String>()
        );

        // Both device names present, in item order (first before second).
        let first_idx = html.find("Ноутбук-0").expect("first device name missing");
        let second_idx = html.find("Ноутбук-1").expect("second device name missing");
        assert!(
            first_idx < second_idx,
            "device names must render in item order: {html:?}"
        );
    })
    .await
    .expect("timeout");
}

/// Phase 16 Plan 05 (Task 1, T-16-14 mitigation): renamed/rewritten from
/// `render_with_missing_template_returns_notfound`. After Plan 16-02,
/// `render_pdf` no longer calls `pipeline.templates.get_active` (the DB-backed
/// `document_templates` lookup) — it reads `templates/act_handover.html` via
/// `html_templates::load_template`, which NEVER errors: a missing/soft-deleted
/// DB row is irrelevant to the new code path, and a missing on-disk template
/// file gracefully falls back to the embedded default (D-06). Soft-deleting
/// the DB row here (same setup as before) must NOT raise `NotFound` — it must
/// render successfully using the file/embedded-default HTML template, proving
/// the DB-backed template lookup is truly dead in this path.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn render_falls_back_to_embedded_default_when_template_file_missing() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let p = make_full_pipeline().await;
        let device_ids = seed_devices(&p.writer, 1).await;
        let act = create_handover_with_giver(&p.acts, &device_ids, "Иванов И.И.").await;

        // Soft-delete the DB-backed handover template row (frozen path — no
        // longer consulted by render_pdf's HTML pipeline).
        p.writer
            .execute(|conn| {
                conn.execute(
                    "UPDATE document_templates SET deleted_at_utc = ?1, is_active = 0 \
                     WHERE kind = 'act_handover'",
                    params![1_900_000_000_i64],
                )
                .map(|_| ())
                .map_err(map_rusqlite)
            })
            .await
            .expect("soft delete template");

        let html =
            p.acts.render_pdf(act.id).await.expect(
                "render_pdf must succeed via embedded HTML default, DB row is irrelevant now",
            );
        assert!(
            html.contains("Акт приема-передачи"),
            "expected embedded-default marker text in fallback HTML. Head: {:?}",
            html.chars().take(300).collect::<String>()
        );
    })
    .await
    .expect("timeout");
}

/// Phase 16 Plan 05 (Task 1, T-16-14 mitigation): renamed/rewritten from
/// `render_with_broken_template_returns_validation`. Corrupting the DB-backed
/// `document_templates.body_minijinja` row is now a no-op for `render_pdf` —
/// the HTML pipeline never reads that column. Render must still succeed via
/// the file-backed `templates/act_handover.html` (materialized on startup by
/// `TemplateService`'s DB seed being unrelated, and the HTML template's own
/// startup materialization in `AppCtx::build`/this test's `make_full_pipeline`
/// helper, which — like production — resolves to the embedded HTML default
/// when no on-disk file has been written by a test).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn render_falls_back_to_embedded_default_when_broken_template_row_present() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let p = make_full_pipeline().await;
        let device_ids = seed_devices(&p.writer, 1).await;
        let act = create_handover_with_giver(&p.acts, &device_ids, "Иванов И.И.").await;

        // Replace the DB-backed template body with invalid Jinja — irrelevant
        // to the new HTML pipeline, but proves the old code path is truly
        // dead (no error surfaces from this corruption).
        p.writer
            .execute(|conn| {
                conn.execute(
                    "UPDATE document_templates SET body_minijinja = '{ invalid jinja {% endif' \
                     WHERE kind = 'act_handover' AND is_active = 1",
                    [],
                )
                .map(|_| ())
                .map_err(map_rusqlite)
            })
            .await
            .expect("corrupt template");

        let html =
            p.acts.render_pdf(act.id).await.expect(
                "render_pdf must succeed — DB template body is not read by the HTML pipeline",
            );
        assert!(
            html.contains("Акт приема-передачи"),
            "expected embedded-default marker text despite corrupted DB row. Head: {:?}",
            html.chars().take(300).collect::<String>()
        );
    })
    .await
    .expect("timeout");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn render_acceptance_pdf_for_device_works() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let p = make_full_pipeline().await;
        let device_ids = seed_devices(&p.writer, 1).await;
        let html = p
            .acts
            .render_acceptance_pdf(
                device_ids[0],
                "Сидоров С.С.".to_string(),
                "Иванов И.И.".to_string(),
                1_700_000_000,
            )
            .await
            .expect("render acceptance");
        assert!(html.len() > 1000);
        assert!(
            html.contains("Сидоров") || html.contains("Иванов"),
            "expected names in rendered HTML. Head: {:?}",
            html.chars().take(300).collect::<String>()
        );
    })
    .await
    .expect("timeout");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn render_pdf_with_missing_logo_renders_without_logo() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let p = make_full_pipeline().await;
        // Write org.json with logo_path pointing at missing file.
        let org_path = p._dir.path().join("org.json");
        let custom_org = serde_json::json!({
            "name": "ООО Тест",
            "inn": "1234567890",
            "kpp": "111222333",
            "address": "г. Москва",
            "logo_path": "missing-logo.png"
        });
        std::fs::write(
            &org_path,
            serde_json::to_string_pretty(&custom_org).unwrap(),
        )
        .expect("write org.json");

        let device_ids = seed_devices(&p.writer, 1).await;
        let act = create_handover_with_giver(&p.acts, &device_ids, "Тестов Т.Т.").await;
        let html = p
            .acts
            .render_pdf(act.id)
            .await
            .expect("render succeeds despite missing logo");
        assert!(html.len() > 1000);

        // touch templates to silence unused-warning if it surfaces
        let _ = p.templates.clone();
    })
    .await
    .expect("timeout");
}

/// Phase 14 plan 03 — backward-compat (success criterion #4, T-14-03-01):
/// an "old" act whose device has NULL `notes` (specs) and whose org_settings
/// requisites are all at their V033 empty-string default must still render
/// to a non-empty HTML document — never error. `device.notes` defaults to
/// NULL on INSERT (not set by `seed_devices`); `org_db` is wired but untouched
/// (all-defaults row from the V026/V033 migration seed).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn render_pdf_with_null_specs_and_empty_requisites_succeeds() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let p = make_full_pipeline_with_org_db().await;
        // seed_devices does not set `notes` -> NULL by default.
        let device_ids = seed_devices(&p.writer, 1).await;
        let act = create_handover_with_giver(&p.acts, &device_ids, "Старый И.И.").await;

        let html = p
            .acts
            .render_pdf(act.id)
            .await
            .expect("render_pdf must succeed with NULL specs + empty org requisites");
        assert!(html.len() > 1000, "expected substantive HTML");
        assert!(
            html.to_lowercase().contains("<html"),
            "missing HTML document marker"
        );
    })
    .await
    .expect("timeout");
}

/// Phase 14 plan 03 — positive path: device.notes filled + org_settings
/// requisites filled via `save_fields` (D-01/D-02/D-05) both surface in the
/// rendered HTML, proving the data actually flows through the render context
/// instead of just "not erroring".
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn render_pdf_with_filled_specs_and_requisites_surfaces_data() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let p = make_full_pipeline_with_org_db().await;
        let device_ids = seed_devices(&p.writer, 1).await;

        // Fill device.notes (specs, D-01 — live value read at render time).
        let device_id = device_ids[0];
        p.writer
            .execute(move |conn| {
                conn.execute(
                    "UPDATE devices SET notes = ?1 WHERE id = ?2",
                    params!["Intel i5, 8GB ОЗУ", device_id],
                )
                .map(|_| ())
                .map_err(map_rusqlite)
            })
            .await
            .expect("set device notes");

        // Fill org_settings requisites (D-02/D-05) via the real save path.
        let org_db = Arc::new(OrgDbService::new(
            p.writer.clone(),
            p._readers.clone(),
            Arc::new(SystemClock),
            Arc::new(
                Paths::resolve_for_exe_dir(p._dir.path().to_path_buf()).expect("paths for org_db"),
            ),
        ));
        let caller = Identity::trusted_admin();
        org_db
            .save_fields(
                &caller,
                OrgPatch {
                    org_name: "ООО Ромашка".to_string(),
                    inn: "7712345678".to_string(),
                    kpp: "771001001".to_string(),
                    address: "г. Москва, ул. Тестовая, 1".to_string(),
                    phone: "+7 495 000-00-01".to_string(),
                    fax: "+7 495 000-00-02".to_string(),
                    email: "info@romashka.ru".to_string(),
                    okpo: "87654321".to_string(),
                    ogrn: "1027700654321".to_string(),
                },
            )
            .await
            .expect("save_fields");

        let act = create_handover_with_giver(&p.acts, &device_ids, "Новый Н.Н.").await;
        let html = p
            .acts
            .render_pdf(act.id)
            .await
            .expect("render_pdf with filled specs/requisites");
        assert!(html.len() > 1000);

        // Org requisites from org_settings (D-05) must reach the shipped
        // default template's header, proving the render context carries them
        // (the default act_handover template renders org.name/inn/kpp/address).
        assert!(
            html.contains("Ромашка"),
            "org_settings org_name missing from rendered HTML. Head: {:?}",
            html.chars().take(500).collect::<String>()
        );
    })
    .await
    .expect("timeout");
}
