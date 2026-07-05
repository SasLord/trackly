//! Phase 3 plan 01 acceptance test #1 — PDF determinism.
//!
//! Two assertions:
//! 1. `fixture_act_42_renders_to_known_hash` — the canonical fixture renders
//!    to a SHA256 hash that is committed alongside the fixture itself. If the
//!    hash drifts, either the renderer changed deliberately (regenerate the
//!    .sha256 file) OR an unintended non-deterministic source crept in.
//! 2. `rendering_twice_yields_identical_bytes` — defends against
//!    same-machine, same-process non-determinism (race conditions, font
//!    subset ordering, etc.).
//!
//! The fixture deliberately contains «Сидоров-Петроградский Иван
//! Александрович (ё) №42» so the Cyrillic-glyphs test (`pdf_text_extract.rs`)
//! can verify the extracted text under the same PDF.

use sha2::{Digest, Sha256};
use trackly_app::pdf::docspec::DocSpec;
use trackly_app::pdf::PdfRenderer;

// D-13 (Phase 16): frozen krilla path, ignored by default — run explicitly
// with `cargo test -- --ignored` to verify byte-determinism after a
// deliberate renderer change.
#[test]
#[ignore]
fn fixture_act_42_renders_to_known_hash() {
    let json = include_str!("fixtures/act_42.json");
    let spec: DocSpec = serde_json::from_str(json).expect("fixture parse");
    let renderer = PdfRenderer::new();
    let bytes = renderer.render_docspec(&spec).expect("render");

    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let actual = format!("{:x}", hasher.finalize());

    let expected = include_str!("fixtures/act_42.sha256").trim();
    assert_eq!(
        actual, expected,
        "PDF hash drift detected. If the change is intentional, update \
         crates/trackly-app/tests/fixtures/act_42.sha256 to {actual}."
    );
}

// D-13 (Phase 16): frozen krilla path, ignored by default — run explicitly
// with `cargo test -- --ignored` to verify byte-determinism after a
// deliberate renderer change.
#[test]
#[ignore]
fn rendering_twice_yields_identical_bytes() {
    let json = include_str!("fixtures/act_42.json");
    let spec: DocSpec = serde_json::from_str(json).expect("fixture parse");
    let renderer = PdfRenderer::new();
    let a = renderer.render_docspec(&spec).expect("render a");
    let b = renderer.render_docspec(&spec).expect("render b");
    assert_eq!(
        a.len(),
        b.len(),
        "PDF size differs between two consecutive renders ({} vs {})",
        a.len(),
        b.len(),
    );
    if a != b {
        let diffs: Vec<usize> = a
            .iter()
            .zip(b.iter())
            .enumerate()
            .filter_map(|(i, (x, y))| if x != y { Some(i) } else { None })
            .take(12)
            .collect();
        panic!("non-deterministic PDF output. First diff offsets: {diffs:?}");
    }
}
