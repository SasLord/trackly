//! Phase 3.1 Plan 05 — G-8a column overflow integration test.
//!
//! Verifies что `render_table_section` truncate-ит длинный текст с ellipsis
//! '…' и НЕ allows overlap'у с соседней колонкой. Use pdf_extract для read
//! текст из rendered PDF и assert на присутствие truncate-marker и
//! separation between cells.

use trackly_app::pdf::docspec::{DocSpec, HeaderBlock, Section};
use trackly_app::pdf::renderer::{truncate_to_width, PdfRenderer};

const LONG_NAME: &str = "Тестовое устройство с очень длинным русским названием которое не помещается в одну колонку и должно быть truncate-нуто";

fn spec_with_long_name() -> DocSpec {
    DocSpec {
        title: "Тест overflow".into(),
        header: HeaderBlock {
            org_name: "ООО Тест".into(),
            org_inn: "1234567890".into(),
            org_kpp: "123456789".into(),
            org_address: "Москва".into(),
            logo_path: None,
            logo_bytes: None,
            logo_mime: None,
            act_label: "Акт №1".into(),
            date_label: "31 мая 2026".into(),
        },
        sections: vec![Section::ItemsTable {
            columns: vec!["Устр-во".into(), "Инв.№".into(), "Кол-во".into()],
            rows: vec![vec![LONG_NAME.into(), "INV-001".into(), "1".into()]],
        }],
    }
}

#[test]
fn truncate_to_width_byte_identical_for_short_text() {
    // B-3 invariant: short strings (≤ max_chars) must pass through unchanged.
    // Это direct unit test инварианта — pdf_determinism тест дополнительно
    // гарантирует это на full PDF level.
    let result = truncate_to_width("INV-001", 10.0, 200.0);
    assert_eq!(result, "INV-001", "short text must be byte-identical (B-3)");

    let result2 = truncate_to_width("1", 10.0, 200.0);
    assert_eq!(result2, "1");

    // Cyrillic short string.
    let result3 = truncate_to_width("Иванов", 10.0, 200.0);
    assert_eq!(result3, "Иванов");
}

#[test]
fn truncate_to_width_adds_ellipsis_for_long_text() {
    // 100 chars длиной с font 10pt avg=5pt → требует width=500pt. В 50pt
    // помещается 10 chars → ellipsis должен появиться, и result.chars()=10.
    let long = "X".repeat(100);
    let result = truncate_to_width(&long, 10.0, 50.0);
    assert!(result.ends_with('…'), "truncated text must end with ellipsis");
    assert_eq!(result.chars().count(), 10, "result must fit max_chars=10");
}

#[test]
fn long_name_truncated_does_not_overlap_inv_no() {
    let spec = spec_with_long_name();
    let renderer = PdfRenderer::new();
    let pdf_bytes = renderer.render_docspec(&spec).expect("render");
    let text = pdf_extract::extract_text_from_mem(&pdf_bytes).expect("extract text");

    // INV-001 must appear as separate cell — pdf_extract returns text
    // ordered by approximate render position, so INV-001 должен встретиться
    // отдельной токенизируемой строкой.
    assert!(
        text.contains("INV-001"),
        "INV-001 cell must appear in extracted text, got: {text:?}"
    );

    // The cell with long name must show ellipsis (the truncate marker).
    // Этот assert защищает: (а) что truncate-path был вызван,
    // (б) что ellipsis correctly embedded в PDF.
    assert!(
        text.contains('…'),
        "long cell must be truncated with '…', got: {text:?}"
    );

    // Full long name NOT present (truncate fired).
    assert!(
        !text.contains(LONG_NAME),
        "long name should be truncated, not fully present"
    );
}
