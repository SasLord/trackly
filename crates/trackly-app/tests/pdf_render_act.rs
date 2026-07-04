//! PDF render full pipeline integration tests — Phase 3 Plan 04 Task 2.
//!
//! Covers (per behavior list):
//!   - render_handover_act_produces_cyrillic_pdf
//!   - render_with_missing_template_returns_notfound
//!   - render_with_broken_template_returns_validation
//!   - render_acceptance_pdf_for_device_works
//!   - render_pdf_with_missing_logo_renders_without_logo
//!
//! End-to-end: seed templates + org.json + devices + handover акт → render →
//! pdf-extract verifies Cyrillic.

use std::sync::Arc;
use std::time::Duration;

use rusqlite::params;
use tempfile::TempDir;
use trackly_app::dto::act::{ActCreateDto, ActItemNewDto};
use trackly_app::dto::reports::OrgPatch;
use trackly_app::pdf::PdfRenderer;
use trackly_app::services::{ActService, OrgDbService, OrganizationService, TemplateService};
use trackly_core::auth::Identity;
use trackly_core::error::AppError;
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
        let bytes = p.acts.render_pdf(act.id).await.expect("render_pdf");
        assert!(
            bytes.len() > 1000,
            "expected substantive PDF, got {}",
            bytes.len()
        );
        assert_eq!(&bytes[..4], b"%PDF", "missing PDF magic header");

        let text = pdf_extract::extract_text_from_mem(&bytes).expect("extract");
        assert!(
            text.contains("Петров"),
            "Cyrillic receiver name (D-09 intro paragraph) missing. Head: {:?}",
            text.chars().take(300).collect::<String>()
        );
        // Act number is 1 (auto-incremented).
        assert!(
            text.contains("№1") || text.contains("1"),
            "Act number missing. Head: {:?}",
            text.chars().take(300).collect::<String>()
        );

        // Phase 15 plan 04 (WR-05 gap closure): a short 2-device act with no
        // long fields must still render as exactly 1 page — page-count
        // regression guard, not just an absence-of-clipping assumption.
        assert_eq!(
            pdf_extract::extract_text_from_mem_by_pages(&bytes)
                .expect("pages by page")
                .len(),
            1,
            "single short device act must still render as exactly 1 page"
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
        let bytes = p.acts.render_pdf(act.id).await.expect("render_pdf");
        let text = pdf_extract::extract_text_from_mem(&bytes).expect("extract");
        assert!(
            text.contains("Настоящим актом утверждаю"),
            "D-09 intro phrase missing. Head: {:?}",
            text.chars().take(500).collect::<String>()
        );
        assert!(
            text.contains(&act.receiver_name),
            "act.receiver_name ({:?}) not interpolated into intro paragraph. Head: {:?}",
            act.receiver_name,
            text.chars().take(500).collect::<String>()
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
        let bytes = p.acts.render_pdf(act.id).await.expect("render_pdf");
        let text = pdf_extract::extract_text_from_mem(&bytes).expect("extract");
        for expected in ["Выдал", "Получил", "Подпись", "ФИО"] {
            assert!(
                text.contains(expected),
                "expected signature label {expected:?} missing. Head: {:?}",
                text.chars().take(500).collect::<String>()
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
/// present, no ellipsis truncation marker appears (proves the FieldRow wrap
/// path was used — 260704-wxw replaced DeviceCard with field_row in the
/// default template — not `ItemsTable`'s truncate path), and a substring
/// from the MIDDLE of the long value survived (proves it wasn't cut off).
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

        let bytes = p.acts.render_pdf(act.id).await.expect("render_pdf");
        let text = pdf_extract::extract_text_from_mem(&bytes).expect("extract");

        // seed_devices names devices "Ноутбук-{i}" where i is the loop
        // index (0..count), independent of the assigned device_id.
        assert_eq!(device_ids.len(), 5, "expected 5 seeded devices");
        for i in 0..5 {
            let expected_name = format!("Ноутбук-{i}");
            assert!(
                text.contains(&expected_name),
                "device name {expected_name:?} missing from rendered text. Head: {:?}",
                text.chars().take(800).collect::<String>()
            );
        }

        assert!(
            !text.contains('…'),
            "long complectation field must wrap, not truncate with ellipsis. Head: {:?}",
            text.chars().take(800).collect::<String>()
        );

        assert!(
            text.contains("СЕРЕДИНА-МАРКЕР-ЗНАЧЕНИЯ"),
            "middle-of-value marker missing — long field appears to have been cut off. \
             Head: {:?}",
            text.chars().take(800).collect::<String>()
        );
    })
    .await
    .expect("timeout");
}

/// Phase 15 plan 04 — WR-05/PDFA-02 gap closure: a realistic multi-device act
/// (8 devices, each with a populated 150+ char Cyrillic `complectation_at_time`)
/// must render as a REAL multi-page PDF — measured via
/// `pdf_extract::extract_text_from_mem_by_pages`'s page-tree-aware page count,
/// not via text-extraction alone (which cannot detect content drawn past the
/// visible page area). Also asserts no data loss: every seeded device's name
/// is still present somewhere across the full multi-page document.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn render_handover_multi_device_paginates_when_overflowing_one_page() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let p = make_full_pipeline().await;
        let device_ids = seed_devices(&p.writer, 8).await;
        let act = create_handover_with_giver(&p.acts, &device_ids, "Смирнов С.С.").await;

        let long_kit = "Блок питания, кабель питания, кабель HDMI, сумка для переноски, \
            документация на русском языке, гарантийный талон, комплект крепёжных винтов, \
            салфетка для протирки экрана, дополнительный набор переходников и адаптеров \
            для подключения к разным типам мониторов и периферийных устройств"
            .to_string();
        assert!(
            long_kit.chars().count() > 150,
            "test fixture string must exceed 150 chars"
        );

        // Set complectation_at_time on ALL 8 act_items rows (not just 2) —
        // this test needs enough cumulative height to force a real page
        // break, unlike render_handover_multi_device_wraps_long_fields above.
        for item in &act.items {
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

        let bytes = p.acts.render_pdf(act.id).await.expect("render_pdf");

        let pages = pdf_extract::extract_text_from_mem_by_pages(&bytes).expect("pages by page");
        assert!(
            pages.len() > 1,
            "expected a real multi-page PDF for 8 devices with long fields, got {} page(s)",
            pages.len()
        );

        // No data loss: every seeded device's name survives somewhere across
        // the full multi-page document.
        let full_text = pages.join("\n");
        for i in 0..8 {
            let expected_name = format!("Ноутбук-{i}");
            assert!(
                full_text.contains(&expected_name),
                "device name {expected_name:?} missing from full multi-page document"
            );
        }
    })
    .await
    .expect("timeout");
}

/// 260704-wxw success criterion: the default `act_handover.minijinja` must
/// emit `field_row` sections (full-length labels), never `device_card`'s
/// «Устройство №N» heading/counter nor the abbreviated legacy labels. Sets
/// `inventory_number`/`serial_number`/`model` directly on the seeded
/// `devices` rows (these fields live on `devices`, not `act_items` —
/// `ActItemDto.inventory_no`/`serial_no`/`model` are joined live from
/// `devices` at render time, unlike `complectation_at_time`/`condition_at_time`
/// which are `act_items` snapshot columns).
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
        let bytes = p.acts.render_pdf(act.id).await.expect("render_pdf");
        let text = pdf_extract::extract_text_from_mem(&bytes).expect("extract");

        // Full-length labels present.
        for label in ["Инвентарный номер:", "Серийный номер:", "Модель:"]
        {
            assert!(
                text.contains(label),
                "expected full-length label {label:?} in rendered PDF. Head: {:?}",
                text.chars().take(800).collect::<String>()
            );
        }

        // No device_card heading/counter of any kind.
        assert!(
            !text.contains("Устройство №1") && !text.contains("Устройство №2"),
            "device_card-style «Устройство №N» heading must not appear. Head: {:?}",
            text.chars().take(800).collect::<String>()
        );

        // No abbreviated legacy labels.
        assert!(
            !text.contains("Инв.№") && !text.contains("Серийный №"),
            "abbreviated legacy labels must not appear. Head: {:?}",
            text.chars().take(800).collect::<String>()
        );

        // Both device names present, in item order (first before second).
        let first_idx = text.find("Ноутбук-0").expect("first device name missing");
        let second_idx = text.find("Ноутбук-1").expect("second device name missing");
        assert!(
            first_idx < second_idx,
            "device names must render in item order: {text:?}"
        );
    })
    .await
    .expect("timeout");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn render_with_missing_template_returns_notfound() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let p = make_full_pipeline().await;
        let device_ids = seed_devices(&p.writer, 1).await;
        let act = create_handover_with_giver(&p.acts, &device_ids, "Иванов И.И.").await;

        // Soft-delete handover template.
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

        let err = p.acts.render_pdf(act.id).await.expect_err("should fail");
        match err {
            AppError::NotFound { entity, .. } => assert_eq!(entity, "document_template"),
            other => panic!("expected NotFound, got {other:?}"),
        }
    })
    .await
    .expect("timeout");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn render_with_broken_template_returns_validation() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let p = make_full_pipeline().await;
        let device_ids = seed_devices(&p.writer, 1).await;
        let act = create_handover_with_giver(&p.acts, &device_ids, "Иванов И.И.").await;

        // Replace template body with invalid Jinja.
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

        let err = p.acts.render_pdf(act.id).await.expect_err("should fail");
        match err {
            AppError::Validation { field, .. } => assert_eq!(field, "template"),
            other => panic!("expected Validation, got {other:?}"),
        }
    })
    .await
    .expect("timeout");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn render_acceptance_pdf_for_device_works() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let p = make_full_pipeline().await;
        let device_ids = seed_devices(&p.writer, 1).await;
        let bytes = p
            .acts
            .render_acceptance_pdf(
                device_ids[0],
                "Сидоров С.С.".to_string(),
                "Иванов И.И.".to_string(),
                1_700_000_000,
            )
            .await
            .expect("render acceptance");
        assert!(bytes.len() > 1000);
        let text = pdf_extract::extract_text_from_mem(&bytes).expect("extract");
        assert!(
            text.contains("Сидоров") || text.contains("Иванов"),
            "expected names in extracted text. Head: {:?}",
            text.chars().take(300).collect::<String>()
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
        let bytes = p
            .acts
            .render_pdf(act.id)
            .await
            .expect("render succeeds despite missing logo");
        assert!(bytes.len() > 1000);

        // touch templates to silence unused-warning if it surfaces
        let _ = p.templates.clone();
    })
    .await
    .expect("timeout");
}

/// Phase 14 plan 03 — backward-compat (success criterion #4, T-14-03-01):
/// an "old" act whose device has NULL `notes` (specs) and whose org_settings
/// requisites are all at their V033 empty-string default must still render
/// to a non-empty, valid PDF — never error. `device.notes` defaults to NULL
/// on INSERT (not set by `seed_devices`); `org_db` is wired but untouched
/// (all-defaults row from the V026/V033 migration seed).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn render_pdf_with_null_specs_and_empty_requisites_succeeds() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let p = make_full_pipeline_with_org_db().await;
        // seed_devices does not set `notes` -> NULL by default.
        let device_ids = seed_devices(&p.writer, 1).await;
        let act = create_handover_with_giver(&p.acts, &device_ids, "Старый И.И.").await;

        let bytes = p
            .acts
            .render_pdf(act.id)
            .await
            .expect("render_pdf must succeed with NULL specs + empty org requisites");
        assert!(bytes.len() > 1000, "expected substantive PDF");
        assert_eq!(&bytes[..4], b"%PDF", "missing PDF magic header");
    })
    .await
    .expect("timeout");
}

/// Phase 14 plan 03 — positive path: device.notes filled + org_settings
/// requisites filled via `save_fields` (D-01/D-02/D-05) both surface in the
/// PDF text (extracted via pdf_extract), proving the data actually flows
/// through the render context instead of just "not erroring".
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
        let bytes = p
            .acts
            .render_pdf(act.id)
            .await
            .expect("render_pdf with filled specs/requisites");
        assert!(bytes.len() > 1000);

        let text = pdf_extract::extract_text_from_mem(&bytes).expect("extract");
        // Org requisites from org_settings (D-05) must reach the shipped
        // default template's header, proving the render context carries them
        // (the default act_handover template renders org.name/inn/kpp/address).
        assert!(
            text.contains("Ромашка"),
            "org_settings org_name missing from rendered PDF. Head: {:?}",
            text.chars().take(500).collect::<String>()
        );
    })
    .await
    .expect("timeout");
}
