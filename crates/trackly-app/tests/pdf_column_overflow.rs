//! Phase 3.1 Plan 05 — G-8a column overflow integration test.
//!
//! Verifies что `render_table_section` truncate-ит длинный текст с ellipsis
//! '…' и НЕ allows overlap'у с соседней колонкой. Use pdf_extract для read
//! текст из rendered PDF и assert на присутствие truncate-marker и
//! separation between cells.

use std::sync::Arc;

use rusqlite::params;
use trackly_app::dto::act::{ActCreateDto, ActItemNewDto};
use trackly_app::pdf::docspec::{DocSpec, HeaderBlock, Section};
use trackly_app::pdf::renderer::{truncate_to_width, PdfRenderer};
use trackly_app::services::{ActService, OrganizationService, TemplateService};
use trackly_core::primitives::clock::Clock;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::error_conversions::map_rusqlite;
use trackly_infra::test_support::test_writer_and_readers;
use trackly_infra::Paths;

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
            ..Default::default()
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
    assert!(
        result.ends_with('…'),
        "truncated text must end with ellipsis"
    );
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

/// Full-pipeline contrast to `long_name_truncated_does_not_overlap_inv_no`
/// above: that test proves the frozen krilla `ItemsTable` path still
/// truncates its own (old, compact-table) columns with an ellipsis via direct
/// `PdfRenderer::render_docspec` calls. This test proves the INVERSE for the
/// active HTML pipeline (Phase 16): a long `complectation_at_time` value
/// rendered through the real `act_handover.html` template via the full
/// `act_service::render_pdf` pipeline must wrap (via CSS at print time), never
/// truncate with '…' at generation time. Both code paths coexist: the frozen
/// krilla `ItemsTable` truncation is untouched; the active HTML field-row
/// blocks never truncate — word-wrap is left entirely to the browser's CSS.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn device_card_long_field_wraps_instead_of_truncating() {
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

    let device_id = writer
        .execute(|conn| {
            conn.execute(
                "INSERT INTO devices \
                 (type_id, name, status_id, version, created_at_utc, updated_at_utc) \
                 VALUES (1, 'Ноутбук-overflow-тест', 1, 1, ?1, ?1)",
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
            giver_name: "Тестов Т.Т.".into(),
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

    let long_kit = "Блок питания, кабель питания, кабель HDMI, сумка для переноски, \
        документация на русском языке, гарантийный талон, комплект крепёжных винтов, \
        салфетка для протирки экрана СЕРЕДИНА-МАРКЕР-ЗНАЧЕНИЯ хвостовая часть длинной строки"
        .to_string();
    assert!(
        long_kit.chars().count() > 150,
        "test fixture string must exceed 150 chars"
    );

    {
        let act_id = act.id;
        let value = long_kit.clone();
        writer
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

    let html = acts.render_pdf(act.id).await.expect("render_pdf");

    assert!(
        !html.contains('…'),
        "long field must wrap, not truncate, in HTML output. Head: {:?}",
        html.chars().take(800).collect::<String>()
    );
    assert!(
        html.contains("СЕРЕДИНА-МАРКЕР-ЗНАЧЕНИЯ"),
        "middle-of-value marker missing — long field appears truncated. Head: {:?}",
        html.chars().take(800).collect::<String>()
    );
}
