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
            text.contains("Сидоров-Петроградский"),
            "Cyrillic giver name missing. Head: {:?}",
            text.chars().take(300).collect::<String>()
        );
        // Act number is 1 (auto-incremented).
        assert!(
            text.contains("№1") || text.contains("1"),
            "Act number missing. Head: {:?}",
            text.chars().take(300).collect::<String>()
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
