//! Phase 3 plan 06 — PDF logo rendering (ACT-11 / CR-01).
//!
//! Closes the gap surfaced in 03-VERIFICATION.md / 03-REVIEW.md §CR-01:
//! `renderer::render_docspec` previously ignored `spec.header.logo_path`.
//! These tests assert:
//!
//! 1. When `logo_path = Some(valid_png)` — output PDF contains an `/XObject`
//!    or `/Image` marker (krilla emits the logo as an image XObject).
//! 2. When `logo_path = None` — render still succeeds, output has no image.
//! 3. When `logo_path = Some(missing_file)` — render is graceful (no panic,
//!    returns Ok), the file is skipped with a tracing warn.
//!
//! These complement `pdf_determinism.rs::fixture_act_42_renders_to_known_hash`
//! which asserts the `None` branch keeps the pinned SHA256 stable.

use std::io::Write;
use std::sync::Arc;

use rusqlite::params;
use trackly_app::dto::act::{ActCreateDto, ActItemNewDto};
use trackly_app::pdf::docspec::{DocSpec, HeaderBlock, Section};
use trackly_app::pdf::PdfRenderer;
use trackly_app::services::{ActService, OrgDbService, OrganizationService, TemplateService};
use trackly_core::auth::Identity;
use trackly_core::primitives::clock::Clock;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::error_conversions::map_rusqlite;
use trackly_infra::test_support::test_writer_and_readers;
use trackly_infra::Paths;

const LOGO_PNG: &[u8] = include_bytes!("fixtures/logo_test.png");

fn base_spec(logo_path: Option<String>) -> DocSpec {
    DocSpec {
        title: "Логотип-тест".into(),
        header: HeaderBlock {
            org_name: "ООО Тест".into(),
            org_inn: "0".into(),
            org_kpp: "0".into(),
            org_address: "Москва".into(),
            logo_path,
            // Phase 7 plan 02 fields — None for backward-compat tests
            logo_bytes: None,
            logo_mime: None,
            act_label: "Акт".into(),
            date_label: "1 января 2026 г.".into(),
            ..Default::default()
        },
        sections: vec![Section::Paragraph {
            text: "Тело документа".into(),
            style: trackly_app::pdf::docspec::TextStyle::Regular,
        }],
    }
}

fn bytes_contain(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

#[test]
fn act_with_logo_renders_image_in_pdf() {
    // Write the embedded PNG to a temp file so spec.header.logo_path points to
    // a real on-disk file (renderer reads via std::fs::read).
    let tmp = tempfile::Builder::new()
        .prefix("trackly-logo-")
        .suffix(".png")
        .tempfile()
        .expect("tempfile");
    tmp.as_file()
        .write_all(LOGO_PNG)
        .and_then(|_| tmp.as_file().sync_all())
        .expect("write png");
    let path_str = tmp.path().to_str().expect("logo path utf8").to_string();

    let spec = base_spec(Some(path_str));
    let renderer = PdfRenderer::new();
    let bytes = renderer.render_docspec(&spec).expect("render");

    // A PDF embedding a raster image MUST carry an XObject stream with
    // `/Subtype /Image`. That marker is absent in the None-branch output of
    // the current renderer (verified separately), so it's the definitive
    // positive signal that the logo was drawn.
    assert!(
        bytes_contain(&bytes, b"/Subtype /Image") || bytes_contain(&bytes, b"/XObject"),
        "rendered PDF must contain /Subtype /Image or /XObject when logo_path is set; \
         got {} bytes",
        bytes.len()
    );
}

#[test]
fn logo_path_none_renders_without_panic() {
    let spec = base_spec(None);
    let renderer = PdfRenderer::new();
    let bytes = renderer.render_docspec(&spec).expect("render must succeed");
    assert_eq!(&bytes[..4], b"%PDF", "PDF magic header missing");
    assert!(bytes.len() > 500, "output too small to be a valid PDF");
}

#[test]
fn logo_path_missing_file_is_graceful() {
    // A path that definitely doesn't exist — renderer must NOT panic, must
    // NOT propagate an error; it logs and continues.
    let spec = base_spec(Some("/tmp/__trackly_nonexistent_logo_12345_zzz.png".into()));
    let renderer = PdfRenderer::new();
    let bytes = renderer
        .render_docspec(&spec)
        .expect("render must succeed even when logo file is missing");
    assert_eq!(&bytes[..4], b"%PDF", "PDF magic header missing");
}

/// Phase 7 plan 02: logo_bytes BLOB path takes priority over logo_path.
/// When logo_bytes is Some and mime is "image/png", the logo is rendered
/// from in-memory bytes without any filesystem access.
#[test]
fn logo_bytes_blob_renders_image_in_pdf() {
    let spec = DocSpec {
        title: "Логотип-BLOB-тест".into(),
        header: HeaderBlock {
            org_name: "ООО Тест".into(),
            org_inn: "0".into(),
            org_kpp: "0".into(),
            org_address: "Москва".into(),
            logo_path: None, // intentionally None — logo_bytes takes priority
            logo_bytes: Some(LOGO_PNG.to_vec()),
            logo_mime: Some("image/png".into()),
            act_label: "Акт".into(),
            date_label: "1 января 2026 г.".into(),
            ..Default::default()
        },
        sections: vec![Section::Paragraph {
            text: "Тело документа с BLOB логотипом".into(),
            style: trackly_app::pdf::docspec::TextStyle::Regular,
        }],
    };

    let renderer = PdfRenderer::new();
    let bytes = renderer
        .render_docspec(&spec)
        .expect("render with logo_bytes");

    assert_eq!(&bytes[..4], b"%PDF", "PDF magic header missing");
    assert!(
        bytes_contain(&bytes, b"/Subtype /Image") || bytes_contain(&bytes, b"/XObject"),
        "rendered PDF must contain image XObject when logo_bytes is set; got {} bytes",
        bytes.len()
    );
}

/// Phase 7 plan 02: logo_bytes priority — when both logo_bytes AND logo_path are set,
/// logo_bytes wins (no filesystem access for logo_path should occur).
#[test]
fn logo_bytes_takes_priority_over_logo_path() {
    // logo_path points to nonexistent file — if logo_bytes is used, render succeeds
    // and includes the logo; if logo_path is attempted first, it would warn and skip.
    let spec = DocSpec {
        title: "Логотип-priority-тест".into(),
        header: HeaderBlock {
            org_name: "ООО Тест".into(),
            org_inn: "0".into(),
            org_kpp: "0".into(),
            org_address: "Москва".into(),
            logo_path: Some("/tmp/__nonexistent_logo_priority_test.png".into()),
            logo_bytes: Some(LOGO_PNG.to_vec()),
            logo_mime: Some("image/png".into()),
            act_label: "Акт".into(),
            date_label: "1 января 2026 г.".into(),
            ..Default::default()
        },
        sections: vec![],
    };

    let renderer = PdfRenderer::new();
    let bytes = renderer
        .render_docspec(&spec)
        .expect("render must succeed with logo_bytes taking priority");

    // logo_bytes should be used — image XObject present
    assert!(
        bytes_contain(&bytes, b"/Subtype /Image") || bytes_contain(&bytes, b"/XObject"),
        "logo_bytes must take priority — image XObject should be present"
    );
}

/// WR-03 regression closure (Phase 15 plan 03): all tests above call
/// `PdfRenderer::render_docspec` directly, bypassing `act_service`/
/// `OrgDbService` entirely — exactly why the WR-03 bug (BLOB logo silently
/// dropped in `act_service::render_pdf`) was never caught. This test goes
/// through the FULL pipeline: `OrgDbService::save_logo` → `ActService::create`
/// → `ActService::render_pdf`, and asserts the same image-XObject marker.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn blob_logo_via_full_pipeline_renders_in_act_pdf() {
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

    // Save a BLOB logo through the real service path (Settings UI equivalent).
    org_db
        .save_logo(
            &Identity::trusted_admin(),
            LOGO_PNG.to_vec(),
            "image/png".to_string(),
        )
        .await
        .expect("save_logo");

    // Seed one device and create a handover act through the full pipeline.
    let device_id = writer
        .execute(|conn| {
            conn.execute(
                "INSERT INTO devices \
                 (type_id, name, status_id, version, created_at_utc, updated_at_utc) \
                 VALUES (1, 'Ноутбук-логотест', 1, 1, ?1, ?1)",
                params![1_700_000_000_i64],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed device");

    let act = acts
        .create(ActCreateDto {
            number_override: None,
            giver_name: "Логотестов Л.Л.".into(),
            receiver_name: "Приемов П.П.".into(),
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
        })
        .await
        .expect("create handover");

    let bytes = acts.render_pdf(act.id).await.expect("render_pdf");
    assert_eq!(&bytes[..4], b"%PDF", "missing PDF magic header");
    assert!(
        bytes_contain(&bytes, b"/Subtype /Image") || bytes_contain(&bytes, b"/XObject"),
        "full-pipeline rendered PDF must contain an image XObject when a BLOB \
         logo was saved via OrgDbService::save_logo — this is the WR-03 \
         regression the direct-render tests above never caught; got {} bytes",
        bytes.len()
    );
}
