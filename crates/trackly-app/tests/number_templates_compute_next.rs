//! `NumberTemplateService` context memory / occupied / warnings tests —
//! Plan 40.2-04 Task 3 (TDD).
//!
//! Covers the three `<behavior>` scenarios literally:
//!   1. `remember_context`/`get_context` round-trip, and deletion of the
//!      referenced template (via a real `delete()`, not a mock) clears the
//!      context's memory to `None` (D-17, `ON DELETE SET NULL`).
//!   2. `is_occupied` — `.trim()`ed, case-insensitive match, `exclude_id`
//!      self-exclusion (NUM-09).
//!   3. `detect_warnings` — script-mix warning, with and without a
//!      doppelganger paragraph (NUM-12), plus `check_mismatch` against a
//!      saved template's mask (NUM-11).

use std::sync::Arc;

use rusqlite::params;
use trackly_core::domain::number_templates::{NumberTemplateRow, TemplateType};
use trackly_core::primitives::clock::Clock;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::db::writer_worker::WriterHandle;
use trackly_infra::error_conversions::map_rusqlite;
use trackly_infra::test_support::test_writer_and_readers;

use trackly_app::dto::number_template::{NumberWarningKind, TemplateContextDto, TemplateTypeDto};
use trackly_app::services::NumberTemplateService;

fn make_service() -> (NumberTemplateService, tempfile::TempDir) {
    let (writer, readers, dir) = test_writer_and_readers();
    let clock = Arc::new(SystemClock);
    let svc = NumberTemplateService::new(writer, readers, clock);
    (svc, dir)
}

/// Seed a `devices` row (type_id=1 device / 2 printer) with an
/// `inventory_number` and return its id. Fictional data only (CLAUDE.md).
async fn seed_device(
    writer: &Arc<WriterHandle>,
    type_id: i64,
    name: &str,
    inventory_number: &str,
    model: Option<&str>,
) -> i64 {
    let name = name.to_string();
    let inventory_number = inventory_number.to_string();
    let model = model.map(|s| s.to_string());
    writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO devices \
                 (type_id, name, inventory_number, model, status_id, \
                  created_at_utc, updated_at_utc, version) \
                 VALUES (?1, ?2, ?3, ?4, 2, 1700000000, 1700000000, 1)",
                params![type_id, name, inventory_number, model],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed_device")
}

/// Seed a `cartridge_models` + `cartridges` row and return the cartridge id.
async fn seed_cartridge(
    writer: &Arc<WriterHandle>,
    kind_id: i64,
    brand: &str,
    model: &str,
    code: &str,
) -> i64 {
    let brand = brand.to_string();
    let model = model.to_string();
    let code = code.to_string();
    writer
        .execute(move |conn| {
            let tx = conn.transaction().map_err(map_rusqlite)?;
            tx.execute(
                "INSERT INTO cartridge_models \
                 (brand, model, kind_id, created_at_utc, updated_at_utc, version) \
                 VALUES (?1, ?2, ?3, 1700000000, 1700000000, 1)",
                params![brand, model, kind_id],
            )
            .map_err(map_rusqlite)?;
            let model_id = tx.last_insert_rowid();
            tx.execute(
                "INSERT INTO cartridges \
                 (code, model_id, status_id, created_at_utc, updated_at_utc, version) \
                 VALUES (?1, ?2, 1, 1700000000, 1700000000, 1)",
                params![code, model_id],
            )
            .map_err(map_rusqlite)?;
            let cart_id = tx.last_insert_rowid();
            tx.commit().map_err(map_rusqlite)?;
            Ok(cart_id)
        })
        .await
        .expect("seed_cartridge")
}

// ---------------------------------------------------------------------------
// 1. Context memory
// ---------------------------------------------------------------------------

#[tokio::test]
async fn remember_and_get_context_round_trip_then_clears_on_delete() {
    let (svc, _dir) = make_service();

    let template = svc
        .create(TemplateTypeDto::DeviceInventory, "ИНВ-[XXXXXX]".to_string())
        .await
        .expect("create template");

    // No context remembered yet — `device_create` has no seeded default
    // (unlike `act_create`/`cartridge_create`/`drum_create`, which V041
    // pre-wires to their seed templates).
    let before = svc
        .get_context(TemplateContextDto::DeviceCreate)
        .await
        .expect("get_context before");
    assert_eq!(before, None);

    svc.remember_context(TemplateContextDto::DeviceCreate, Some(template.id))
        .await
        .expect("remember_context");

    let after = svc
        .get_context(TemplateContextDto::DeviceCreate)
        .await
        .expect("get_context after remember");
    assert_eq!(after, Some(template.id));

    // Deleting the template the context points at must clear it to None
    // through the REAL delete() path (D-17, ON DELETE SET NULL) — not a
    // mocked repository call.
    svc.delete(template.id).await.expect("delete template");

    let after_delete = svc
        .get_context(TemplateContextDto::DeviceCreate)
        .await
        .expect("get_context after delete");
    assert_eq!(
        after_delete, None,
        "context memory must be cleared when its template is deleted"
    );
}

#[tokio::test]
async fn remember_context_none_clears_previous_memory() {
    let (svc, _dir) = make_service();

    let template = svc
        .create(TemplateTypeDto::CartridgeCode, "К-[XXXX]".to_string())
        .await
        .expect("create template");

    svc.remember_context(TemplateContextDto::CartridgeCreate, Some(template.id))
        .await
        .expect("remember Some");
    svc.remember_context(TemplateContextDto::CartridgeCreate, None)
        .await
        .expect("remember None (NUM-08: 'continue without template')");

    let got = svc
        .get_context(TemplateContextDto::CartridgeCreate)
        .await
        .expect("get_context");
    assert_eq!(got, None);
}

// ---------------------------------------------------------------------------
// 2. is_occupied
// ---------------------------------------------------------------------------

#[tokio::test]
async fn is_occupied_matches_after_trim_and_case_fold() {
    let (svc, _dir) = make_service();
    let device_id = seed_device(
        &svc.writer,
        1,
        "Ноутбук Иванова И.И.",
        "ОРГ-00-000001",
        Some("ThinkPad X1"),
    )
    .await;

    let found = svc
        .is_occupied(TemplateType::DeviceInventory, "орг-00-000001 ", None)
        .await
        .expect("is_occupied")
        .expect("must find the seeded device (trim + case-insensitive)");
    assert_eq!(found.kind, "device");
    assert_eq!(found.title, "Ноутбук Иванова И.И.");
    assert_eq!(found.subtitle.as_deref(), Some("ThinkPad X1"));

    // exclude_id excludes the match itself (editing own record).
    let excluded = svc
        .is_occupied(
            TemplateType::DeviceInventory,
            "ОРГ-00-000001",
            Some(device_id),
        )
        .await
        .expect("is_occupied with exclude_id");
    assert_eq!(excluded, None, "exclude_id must skip self-conflict");

    // A genuinely free value is not occupied.
    let free = svc
        .is_occupied(TemplateType::DeviceInventory, "ОРГ-00-999999", None)
        .await
        .expect("is_occupied free value");
    assert_eq!(free, None);
}

#[tokio::test]
async fn is_occupied_printer_kind_and_cartridge_kind() {
    let (svc, _dir) = make_service();

    seed_device(&svc.writer, 2, "Принтер бухгалтерии", "ПРИНТЕР-01", None).await;
    let printer_hit = svc
        .is_occupied(TemplateType::DeviceInventory, "принтер-01", None)
        .await
        .expect("is_occupied")
        .expect("must find seeded printer");
    assert_eq!(printer_hit.kind, "printer");

    seed_cartridge(&svc.writer, 1, "Pantum", "TL-5120X", "C-000017").await;
    let cartridge_hit = svc
        .is_occupied(TemplateType::CartridgeCode, "c-000017", None)
        .await
        .expect("is_occupied")
        .expect("must find seeded cartridge");
    assert_eq!(cartridge_hit.kind, "cartridge");
    assert_eq!(cartridge_hit.title, "Pantum TL-5120X");
    // BE-WR-09: the card carries the record's OWN stored number.
    assert_eq!(cartridge_hit.number, "C-000017");

    seed_cartridge(&svc.writer, 2, "Kyocera", "DK-1150", "D-000003").await;
    let drum_hit = svc
        .is_occupied(TemplateType::DrumCode, "d-000003", None)
        .await
        .expect("is_occupied")
        .expect("must find seeded drum");
    assert_eq!(drum_hit.kind, "drum");
}

// ---------------------------------------------------------------------------
// 3. detect_warnings (script mix + doppelganger) and check_mismatch
// ---------------------------------------------------------------------------

#[tokio::test]
async fn detect_warnings_script_mix_without_doppelganger() {
    let (svc, _dir) = make_service();

    // "OPГ" mixes Latin O/P with Cyrillic Г — script mix, no existing
    // Cyrillic-only doppelganger seeded.
    let warning = svc
        .detect_warnings(TemplateType::DeviceInventory, "OPГ-00-000001", None)
        .await
        .expect("detect_warnings")
        .expect("script mix must be detected");
    assert_eq!(warning.kind, NumberWarningKind::ScriptMix);
    assert!(warning
        .message
        .contains("смешаны русские и латинские буквы"));
    assert!(warning.doppelganger.is_none());
}

#[tokio::test]
async fn detect_warnings_script_mix_with_doppelganger() {
    let (svc, _dir) = make_service();

    // Pure-Cyrillic existing record.
    seed_device(
        &svc.writer,
        1,
        "Ноутбук Петрова П.П.",
        "ОРГ-00-000001",
        None,
    )
    .await;

    // Latin O and P look-alikes mixed into an otherwise identical number —
    // collapses to the same Latin skeleton as the seeded Cyrillic value.
    let warning = svc
        .detect_warnings(TemplateType::DeviceInventory, "OPГ-00-000001", None)
        .await
        .expect("detect_warnings")
        .expect("script mix + doppelganger must be detected");
    assert_eq!(warning.kind, NumberWarningKind::ScriptMix);
    let doppelganger = warning
        .doppelganger
        .expect("doppelganger must be found for a skeleton-identical existing record");
    assert_eq!(doppelganger.kind, "device");
    assert_eq!(doppelganger.title, "Ноутбук Петрова П.П.");
    assert!(warning.message.contains("выглядит так же"));
}

#[tokio::test]
async fn detect_warnings_no_script_mix_returns_none() {
    let (svc, _dir) = make_service();

    let result = svc
        .detect_warnings(TemplateType::DeviceInventory, "ОРГ-00-000001", None)
        .await
        .expect("detect_warnings");
    assert_eq!(
        result, None,
        "a pure-Cyrillic (or pure-Latin) candidate has no script-mix warning"
    );
}

#[test]
fn check_mismatch_true_when_candidate_does_not_fit_mask() {
    let template = NumberTemplateRow {
        id: 1,
        template_type: TemplateType::CartridgeCode,
        mask: "C-[XXXX]".to_string(),
        created_at_utc: 0,
        updated_at_utc: 0,
        version: 1,
    };
    let today = SystemClock.unix_seconds();

    assert!(
        NumberTemplateService::check_mismatch(&template, "7", today),
        "a bare number with no C- prefix must mismatch the mask"
    );
    assert!(
        !NumberTemplateService::check_mismatch(&template, "C-0007", today),
        "a value that fits prefix + fixed digit width must NOT mismatch"
    );
}

#[test]
fn check_mismatch_false_when_candidate_fits_unbounded_mask() {
    let template = NumberTemplateRow {
        id: 2,
        template_type: TemplateType::ActNumber,
        mask: "[X]".to_string(),
        created_at_utc: 0,
        updated_at_utc: 0,
        version: 1,
    };
    let today = SystemClock.unix_seconds();

    assert!(!NumberTemplateService::check_mismatch(
        &template, "42", today
    ));
    assert!(NumberTemplateService::check_mismatch(
        &template,
        "not-a-number",
        today
    ));
}

/// BE-CR-03: NUM-12 has two independent triggers. A single-alphabet
/// candidate that collapses to an existing number written with DIFFERENT
/// letters (Cyrillic «С-0005» next to the seeded Latin «C-0005») must warn,
/// even though there is no script mix at all.
#[tokio::test]
async fn detect_warnings_doppelganger_without_script_mix() {
    let (svc, _dir) = make_service();
    seed_cartridge(&svc.writer, 1, "Pantum", "TL-5120X", "C-0005").await;

    let warning = svc
        .detect_warnings(TemplateType::CartridgeCode, "С-0005", None)
        .await
        .expect("detect_warnings")
        .expect("a pure-Cyrillic visual duplicate of a Latin code must warn");
    assert_eq!(warning.kind, NumberWarningKind::ScriptMix);
    let doppelganger = warning.doppelganger.expect("doppelganger card");
    assert_eq!(doppelganger.kind, "cartridge");
    assert_eq!(
        doppelganger.number, "C-0005",
        "BE-WR-09: the card must carry the existing number, not the input"
    );
    assert!(
        warning.message.contains("«C-0005»"),
        "the message must quote the EXISTING number, got: {}",
        warning.message
    );
    assert!(!warning.message.contains("смешаны"));

    // An exact (case-insensitive) match is an occupied number, not a
    // doppelganger — detect_warnings stays silent for it.
    let exact = svc
        .detect_warnings(TemplateType::CartridgeCode, "c-0005", None)
        .await
        .expect("detect_warnings");
    assert_eq!(exact, None);
}

/// BE-WR-01: a template of another type is rejected by the context memory
/// and by the usage-context peek; the context keeps its previous value.
#[tokio::test]
async fn context_memory_and_peek_reject_template_of_another_type() {
    let (svc, _dir) = make_service();
    let device_tpl = svc
        .create(TemplateTypeDto::DeviceInventory, "ИНВ-[XXXXXX]".to_string())
        .await
        .expect("create device template");

    let before = svc
        .get_context(TemplateContextDto::ActCreate)
        .await
        .expect("get act context");
    let err = svc
        .remember_context(TemplateContextDto::ActCreate, Some(device_tpl.id))
        .await;
    assert!(
        matches!(&err, Err(trackly_core::error::AppError::Validation { field, .. }) if field == "template_id"),
        "got {err:?}"
    );
    assert_eq!(
        svc.get_context(TemplateContextDto::ActCreate)
            .await
            .expect("get act context"),
        before,
        "a rejected write must not change the remembered template"
    );

    let peek = svc
        .peek_next_for_context(device_tpl.id, TemplateContextDto::DrumCreate)
        .await;
    assert!(
        matches!(peek, Err(trackly_core::error::AppError::Validation { .. })),
        "got {peek:?}"
    );
    svc.peek_next_for_context(device_tpl.id, TemplateContextDto::PrinterCreate)
        .await
        .expect("same-type context is fine");
}
