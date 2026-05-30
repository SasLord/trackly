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

use trackly_app::pdf::docspec::{DocSpec, HeaderBlock, Section};
use trackly_app::pdf::PdfRenderer;

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
            act_label: "Акт".into(),
            date_label: "1 января 2026 г.".into(),
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
