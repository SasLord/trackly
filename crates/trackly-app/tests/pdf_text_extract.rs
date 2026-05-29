//! Phase 3 plan 01 acceptance test #2 — Cyrillic glyphs survive the PDF.
//!
//! Renders the canonical fixture, then extracts text via `pdf-extract` and
//! asserts the marker substrings are present. If DejaVu Sans fails to cover
//! a Russian or yo character, `pdf-extract` will return mojibake or `?`
//! placeholders and these assertions will fire.

use trackly_app::pdf::docspec::DocSpec;
use trackly_app::pdf::PdfRenderer;

#[test]
fn fixture_pdf_contains_cyrillic_marker() {
    let json = include_str!("fixtures/act_42.json");
    let spec: DocSpec = serde_json::from_str(json).expect("fixture parse");
    let bytes = PdfRenderer::new().render_docspec(&spec).expect("render");

    let text = pdf_extract::extract_text_from_mem(&bytes).expect("extract text");

    assert!(
        text.contains("Сидоров-Петроградский"),
        "Cyrillic marker «Сидоров-Петроградский» missing from extracted text. \
         DejaVu Sans glyph coverage regression? Extracted text head: {:?}",
        text.chars().take(200).collect::<String>(),
    );
    assert!(
        text.contains("№42"),
        "Act number «№42» missing. Extracted text head: {:?}",
        text.chars().take(200).collect::<String>(),
    );
    assert!(
        text.contains("(ё)"),
        "yo character «(ё)» missing — encoding regression. \
         Extracted text head: {:?}",
        text.chars().take(200).collect::<String>(),
    );
}
