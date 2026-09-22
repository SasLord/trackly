//! Phase 40.2 Plan 08 (NUM-09/D-05/D-08) — device/printer inventory-number
//! integration tests.
//!
//! Devices/printers get uniqueness on `inventory_number` for the FIRST TIME
//! in this plan (it was always free text, no constraint at all). Coverage
//! per this plan's own `<behavior>`:
//!   1. `create()`/`bulk_create(count=1)`/`create_single_with_number_check()`
//!      all `.trim()` explicitly (D-08, not `normalize_str`) — leading/
//!      trailing spaces are stripped, case preserved.
//!   2. A second LIVE device/printer with the same number (any case/
//!      whitespace) is rejected as occupied (NUM-09).
//!   3. `update()` to a number occupied by ANOTHER live device is rejected;
//!      updating a device to its OWN current number is never a
//!      self-conflict (`exclude_id`).
//!   4. `update()` (D-05) never checks template mismatch (`DevicePatch`'s
//!      `number_input: Option<DeviceNumberEditInput>` structurally has no
//!      `template_id` field) but DOES check script-mix, same as
//!      `create_single_with_number_check()`.
//!   5. Empty inventory number is never checked (NUM-09 — the one number
//!      field in this whole phase that stays optional, D-16).
//!   6. `create_single_with_number_check()` remembers the used template
//!      only for its own NUM-08 context; `update()` never calls
//!      `remember_context` at all.
//!
//! Фиктивные названия/номера — приватность (CLAUDE.md).

use std::sync::Arc;
use std::time::Duration;

use trackly_app::dto::device::{DeviceNew, DeviceNumberEditInput, DevicePatch, DeviceSaveOutcome};
use trackly_app::dto::number_template::{
    NumberFieldInput, NumberWarningKind, TemplateContextDto, TemplateTypeDto,
};
use trackly_app::services::{DeviceService, NumberTemplateService};
use trackly_core::auth::Identity;
use trackly_core::primitives::clock::Clock;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::test_support::test_writer_and_readers;

/// `DeviceService::number_templates` is `pub(crate)` — this test is an
/// external integration crate, so it stands up its own
/// `NumberTemplateService` against the SAME writer/readers (mirrors
/// `cartridges_numbering.rs::number_templates_for`'s precedent).
fn number_templates_for(svc: &DeviceService) -> NumberTemplateService {
    NumberTemplateService::new(
        svc.writer.clone(),
        svc.readers.clone(),
        Arc::new(SystemClock),
    )
}

fn make_service() -> (DeviceService, tempfile::TempDir) {
    let (writer, readers, dir) = test_writer_and_readers();
    let clock: Arc<dyn Clock + Send + Sync> = Arc::new(SystemClock);
    let svc = DeviceService::new(writer, readers, clock);
    (svc, dir)
}

fn admin_caller() -> Identity {
    Identity::trusted_admin()
}

fn minimal_new(name: &str, inventory_no: Option<&str>) -> DeviceNew {
    DeviceNew {
        type_id: 1,
        name: name.to_string(),
        inventory_no: inventory_no.map(str::to_string),
        serial_no: None,
        model: None,
        specs: None,
        kit: None,
        state: None,
        place_id: None,
        status_id: 1,
    }
}

fn number_input(value: &str) -> NumberFieldInput {
    NumberFieldInput {
        value: value.to_string(),
        template_id: None,
        confirm_mismatch: false,
        confirm_script_mix: false,
    }
}

// ---------------------------------------------------------------------------
// create() trims explicitly (D-08)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_trims_inventory_number() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let dto = svc
            .create(minimal_new("Ноутбук Lenovo", Some(" ОРГ-00-000001 ")))
            .await
            .expect("create")
            .expect_created("create");
        assert_eq!(
            dto.inventory_no.as_deref(),
            Some("ОРГ-00-000001"),
            "create() must .trim() the number (D-08), not store surrounding whitespace"
        );
    })
    .await
    .expect("create_trims_inventory_number exceeded 30 s budget");
}

// ---------------------------------------------------------------------------
// bulk_create(count=1) trims + rejects occupied
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn bulk_create_single_trims_and_rejects_occupied() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let dtos = svc
            .bulk_create(minimal_new("Принтер HP", Some(" ОРГ-00-000002 ")), 1)
            .await
            .expect("bulk_create count=1");
        assert_eq!(
            dtos[0].inventory_no.as_deref(),
            Some("ОРГ-00-000002"),
            "bulk_create(count=1) must trim the number too"
        );

        // Different case + surrounding whitespace, same number.
        let err = svc
            .bulk_create(minimal_new("Принтер HP 2", Some(" орг-00-000002 ")), 1)
            .await
            .expect_err("занятый номер должен быть отклонён даже через bulk_create(count=1)");
        match err {
            trackly_core::error::AppError::Conflict { reason } => {
                assert!(
                    reason.contains("уже занят"),
                    "reason must mention 'уже занят': {reason}"
                );
            }
            other => panic!("ожидали Conflict, получили {other:?}"),
        }
    })
    .await
    .expect("bulk_create_single_trims_and_rejects_occupied exceeded 30 s budget");
}

// ---------------------------------------------------------------------------
// create() rejects a case-differing duplicate (NUM-09)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_rejects_case_insensitive_duplicate() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        svc.create(minimal_new("Устройство 1", Some("ОРГ-00-000003")))
            .await
            .expect("create first")
            .expect_created("create first");

        let err = svc
            .create(minimal_new("Устройство 2", Some("орг-00-000003")))
            .await
            .expect_err("должен быть отклонён как занятый (регистр не важен, NUM-09)");
        match err {
            trackly_core::error::AppError::Conflict { .. } => {}
            other => panic!("ожидали Conflict, получили {other:?}"),
        }
    })
    .await
    .expect("create_rejects_case_insensitive_duplicate exceeded 30 s budget");
}

// ---------------------------------------------------------------------------
// create() with empty number is never checked (D-16)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_with_empty_number_never_checked() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        svc.create(minimal_new("Устройство А", None))
            .await
            .expect("create first with no number")
            .expect_created("create first with no number");
        // A second device with NO number must also succeed — empty is
        // never part of the occupied space (SPEC NUM-09).
        let dto2 = svc
            .create(minimal_new("Устройство Б", None))
            .await
            .expect("create second with no number")
            .expect_created("create second with no number");
        assert!(dto2.inventory_no.is_none());
    })
    .await
    .expect("create_with_empty_number_never_checked exceeded 30 s budget");
}

// ---------------------------------------------------------------------------
// update(): occupied-by-another-device rejected, self-number is not a conflict
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn update_rejects_number_occupied_by_another_device_but_not_self() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let d1 = svc
            .create(minimal_new("Устройство 1", Some("ОРГ-00-000010")))
            .await
            .expect("create 1")
            .expect_created("create 1");
        let d2 = svc
            .create(minimal_new("Устройство 2", Some("ОРГ-00-000011")))
            .await
            .expect("create 2")
            .expect_created("create 2");

        // d2 tries to rename to d1's number -> Conflict.
        let err = svc
            .update(
                &admin_caller(),
                d2.id,
                d2.version,
                DevicePatch {
                    number_input: Some(DeviceNumberEditInput {
                        value: "ОРГ-00-000010".to_string(),
                        confirm_script_mix: false,
                    }),
                    ..Default::default()
                },
            )
            .await
            .expect_err("переименование в занятый номер должно быть отклонено");
        match err {
            trackly_core::error::AppError::Conflict { .. } => {}
            other => panic!("ожидали Conflict, получили {other:?}"),
        }

        // d1 "renaming" to its OWN current number must succeed (self is
        // excluded from the occupied check).
        let updated = svc
            .update(
                &admin_caller(),
                d1.id,
                d1.version,
                DevicePatch {
                    number_input: Some(DeviceNumberEditInput {
                        value: "ОРГ-00-000010".to_string(),
                        confirm_script_mix: false,
                    }),
                    ..Default::default()
                },
            )
            .await
            .expect("update to own current number must succeed")
            .expect_created("update to own current number must succeed");
        assert_eq!(updated.inventory_no.as_deref(), Some("ОРГ-00-000010"));
    })
    .await
    .expect("update_rejects_number_occupied_by_another_device_but_not_self exceeded 30 s budget");
}

// ---------------------------------------------------------------------------
// update() unrelated field edit never re-opens the number chain
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn update_without_number_input_skips_number_chain_entirely() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let dto = svc
            .create(minimal_new("Устройство", Some("ОРГ-00-000020")))
            .await
            .expect("create")
            .expect_created("create");

        // Editing name only, `number_input: None` -> must succeed and leave
        // the number untouched, even though a "duplicate" of it now exists
        // elsewhere in a hypothetical world — the chain never even runs.
        let updated = svc
            .update(
                &admin_caller(),
                dto.id,
                dto.version,
                DevicePatch {
                    name: Some("Устройство (переименовано)".to_string()),
                    ..Default::default()
                },
            )
            .await
            .expect("update name only")
            .expect_created("update name only");
        assert_eq!(updated.inventory_no.as_deref(), Some("ОРГ-00-000020"));
        assert_eq!(updated.name, "Устройство (переименовано)");
    })
    .await
    .expect("update_without_number_input_skips_number_chain_entirely exceeded 30 s budget");
}

// ---------------------------------------------------------------------------
// update() script-mix warning (NUM-12) -> NeedsConfirmation
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn update_script_mix_number_needs_confirmation_then_succeeds() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let dto = svc
            .create(minimal_new("Устройство", Some("ОРГ-00-000030")))
            .await
            .expect("create")
            .expect_created("create");

        // Mixed Cyrillic/Latin look-alikes: Latin "O" + Latin "P" among
        // Cyrillic letters.
        let mixed = "OPГ-00-000031";
        let outcome = svc
            .update(
                &admin_caller(),
                dto.id,
                dto.version,
                DevicePatch {
                    number_input: Some(DeviceNumberEditInput {
                        value: mixed.to_string(),
                        confirm_script_mix: false,
                    }),
                    ..Default::default()
                },
            )
            .await
            .expect("update with script-mix number");
        match outcome {
            DeviceSaveOutcome::NeedsConfirmation(warning) => {
                assert!(
                    warning.message.contains("смешаны"),
                    "warning message must mention script-mix: {}",
                    warning.message
                );
            }
            DeviceSaveOutcome::Created(_) => {
                panic!("ожидали NeedsConfirmation для номера со смешанными буквами")
            }
        }

        // Re-fetch: number must be UNCHANGED (the warning blocked the save).
        let unchanged = svc.get(dto.id).await.expect("get after warning");
        assert_eq!(unchanged.inventory_no.as_deref(), Some("ОРГ-00-000030"));

        // Confirming proceeds.
        let confirmed = svc
            .update(
                &admin_caller(),
                dto.id,
                unchanged.version,
                DevicePatch {
                    number_input: Some(DeviceNumberEditInput {
                        value: mixed.to_string(),
                        confirm_script_mix: true,
                    }),
                    ..Default::default()
                },
            )
            .await
            .expect("update with confirmed script-mix number")
            .expect_created("update with confirmed script-mix number");
        assert_eq!(confirmed.inventory_no.as_deref(), Some(mixed));
    })
    .await
    .expect("update_script_mix_number_needs_confirmation_then_succeeds exceeded 30 s budget");
}

// ---------------------------------------------------------------------------
// create_single_with_number_check(): occupied / script-mix / remember_context
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_single_with_number_check_full_chain() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let number_templates = number_templates_for(&svc);

        // No seeded default for device_inventory (unlike act/cartridge/drum,
        // Plan 01) — NUM-08 context memory starts at None.
        assert_eq!(
            number_templates
                .get_context(TemplateContextDto::DeviceCreate)
                .await
                .expect("get_context before any create"),
            None
        );

        // Baseline occupant.
        svc.create(minimal_new("Занято", Some("ОРГ-00-000040")))
            .await
            .expect("seed occupant")
            .expect_created("seed occupant");

        // 1. Occupied -> Err(Conflict).
        let err = svc
            .create_single_with_number_check(
                minimal_new("Новое устройство", None),
                number_input("ОРГ-00-000040"),
                TemplateContextDto::DeviceCreate,
            )
            .await
            .expect_err("занятый номер должен быть отклонён");
        match err {
            trackly_core::error::AppError::Conflict { .. } => {}
            other => panic!("ожидали Conflict, получили {other:?}"),
        }

        // 2. Script-mix -> NeedsConfirmation, no device created.
        let mixed = "OPГ-00-000041";
        let outcome = svc
            .create_single_with_number_check(
                minimal_new("Новое устройство", None),
                number_input(mixed),
                TemplateContextDto::DeviceCreate,
            )
            .await
            .expect("create_single_with_number_check script-mix");
        let DeviceSaveOutcome::NeedsConfirmation(_) = outcome else {
            panic!("ожидали NeedsConfirmation для номера со смешанными буквами");
        };

        // 3. Confirmed script-mix WITH a template selected -> Created.
        // Fix 40.2-13 (NUM-11): `mixed` ("OPГ-00-000041") does not actually
        // match "ОРГ-[XXXXXX]"'s mask (literal prefix "ОРГ-" + 6 digits, no
        // "00-" in the middle, and check_mismatch is a literal/byte
        // comparison — it does NOT normalise Latin/Cyrillic homoglyphs), so
        // `create_single_with_number_check()` now ALSO needs
        // `confirm_mismatch: true` to get past the mismatch gate (D-01
        // order: occupied -> mismatch -> script-mix) before it can even
        // reach the script-mix confirmation this step is meant to exercise.
        // A confirmed mismatch resets NUM-08 memory to "без шаблона"
        // (proven in detail by
        // `create_single_with_number_check_mismatch_needs_confirmation_then_succeeds`)
        // regardless of which `template_id` was supplied — this step's own
        // `remember_context` assertion below reflects that.
        let template_id = number_templates
            .create(TemplateTypeDto::DeviceInventory, "ОРГ-[XXXXXX]".to_string())
            .await
            .expect("create device_inventory template")
            .id;
        let created = svc
            .create_single_with_number_check(
                minimal_new("Новое устройство", None),
                NumberFieldInput {
                    value: mixed.to_string(),
                    template_id: Some(template_id),
                    confirm_mismatch: true,
                    confirm_script_mix: true,
                },
                TemplateContextDto::DeviceCreate,
            )
            .await
            .expect("create_single_with_number_check confirmed")
            .expect_created("create_single_with_number_check confirmed");
        assert_eq!(created.inventory_no.as_deref(), Some(mixed));

        assert_eq!(
            number_templates
                .get_context(TemplateContextDto::DeviceCreate)
                .await
                .expect("get_context after create"),
            None,
            "NUM-08/NUM-11: a confirmed mismatch resets the context to «без шаблона», \
             even though a template_id was supplied"
        );
    })
    .await
    .expect("create_single_with_number_check_full_chain exceeded 30 s budget");
}

// ---------------------------------------------------------------------------
// T-40.2-18 (Tampering — race on one inventory number): two concurrent
// create_single_with_number_check() calls for the SAME number — the async
// pre-check alone has a small TOCTOU window, `idx_devices_inventory_number_live`
// (V044, Task 1) is the final DB-level backstop guaranteeing exactly one
// success. Mirrors `acts_numbering.rs::concurrent_creates_same_number_exactly_one_succeeds`.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_creates_same_number_exactly_one_succeeds() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let svc = Arc::new(svc);

        let svc1 = svc.clone();
        let h1 = tokio::spawn(async move {
            svc1.create_single_with_number_check(
                minimal_new("Гонка 1", None),
                number_input("ОРГ-00-000077"),
                TemplateContextDto::DeviceCreate,
            )
            .await
        });
        let svc2 = svc.clone();
        let h2 = tokio::spawn(async move {
            svc2.create_single_with_number_check(
                minimal_new("Гонка 2", None),
                number_input("ОРГ-00-000077"),
                TemplateContextDto::DeviceCreate,
            )
            .await
        });

        let r1 = h1.await.expect("join1");
        let r2 = h2.await.expect("join2");

        let ok_count = [&r1, &r2]
            .iter()
            .filter(|r| matches!(r, Ok(DeviceSaveOutcome::Created(_))))
            .count();
        let conflict_count = [&r1, &r2]
            .iter()
            .filter(|r| matches!(r, Err(trackly_core::error::AppError::Conflict { .. })))
            .count();
        assert_eq!(
            ok_count, 1,
            "exactly one of two concurrent same-number creates must succeed, got r1={r1:?} r2={r2:?}"
        );
        assert_eq!(
            conflict_count, 1,
            "the other must fail with Conflict (either pre-check or DB unique index), \
             got r1={r1:?} r2={r2:?}"
        );

        let readers = svc.readers.clone();
        let count: i64 = tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            conn.query_row(
                "SELECT COUNT(*) FROM devices WHERE inventory_number = 'ОРГ-00-000077' \
                 AND deleted_at_utc IS NULL",
                [],
                |r| r.get(0),
            )
        })
        .await
        .expect("spawn_blocking")
        .expect("count devices");
        assert_eq!(count, 1, "exactly one live device with the contested number");
    })
    .await
    .expect("concurrent_creates_same_number_exactly_one_succeeds exceeded 30 s budget");
}

// ---------------------------------------------------------------------------
// Fix 40.2-13 (NUM-11): create_single_with_number_check() DOES check template
// mismatch, contrary to Plan 08's original (wrong) doc-comment. SPEC NUM-11's
// own acceptance example (template ОРГ-00-[XXXXXX], value ИНВ-7) applies
// verbatim to devices/printers — this is exactly that scenario.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_single_with_number_check_mismatch_needs_confirmation_then_succeeds() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let number_templates = number_templates_for(&svc);

        let template_id = number_templates
            .create(TemplateTypeDto::DeviceInventory, "ОРГ-00-[XXXXXX]".to_string())
            .await
            .expect("create device_inventory template")
            .id;

        // SPEC NUM-11 acceptance example: active `ОРГ-00-[XXXXXX]`, value
        // `ИНВ-7` -> mismatch popup.
        let outcome = svc
            .create_single_with_number_check(
                minimal_new("Новое устройство", None),
                NumberFieldInput {
                    value: "ИНВ-7".to_string(),
                    template_id: Some(template_id),
                    confirm_mismatch: false,
                    confirm_script_mix: false,
                },
                TemplateContextDto::DeviceCreate,
            )
            .await
            .expect("create_single_with_number_check mismatch");
        match outcome {
            DeviceSaveOutcome::NeedsConfirmation(warning) => {
                assert_eq!(warning.kind, NumberWarningKind::Mismatch);
                assert!(
                    warning.message.contains("ИНВ-7") && warning.message.contains("ОРГ-00-[XXXXXX]"),
                    "message must name the candidate and the mask verbatim: {}",
                    warning.message
                );
            }
            DeviceSaveOutcome::Created(_) => {
                panic!("ожидали NeedsConfirmation(mismatch) для «ИНВ-7» при активном шаблоне")
            }
        }

        // Nothing was created by the mismatch preview.
        let listed = svc
            .list(
                trackly_app::dto::device::DeviceFilter::default(),
                trackly_app::dto::device::Pagination {
                    offset: 0,
                    limit: 200,
                },
            )
            .await
            .expect("list after mismatch preview");
        assert!(
            !listed.items.iter().any(|d| d.inventory_no.as_deref() == Some("ИНВ-7")),
            "mismatch preview must NOT persist a device"
        );

        // «Продолжить» -> confirm_mismatch=true -> saves ИНВ-7 as-is, and per
        // SPEC NUM-11 п.8 the context is remembered as "без шаблона" (None),
        // so the next такой попап открывается с пустым полем.
        let created = svc
            .create_single_with_number_check(
                minimal_new("Новое устройство", None),
                NumberFieldInput {
                    value: "ИНВ-7".to_string(),
                    template_id: Some(template_id),
                    confirm_mismatch: true,
                    confirm_script_mix: false,
                },
                TemplateContextDto::DeviceCreate,
            )
            .await
            .expect("create_single_with_number_check confirmed mismatch")
            .expect_created("create_single_with_number_check confirmed mismatch");
        assert_eq!(created.inventory_no.as_deref(), Some("ИНВ-7"));

        assert_eq!(
            number_templates
                .get_context(TemplateContextDto::DeviceCreate)
                .await
                .expect("get_context after confirmed mismatch"),
            None,
            "NUM-08/NUM-11: a confirmed mismatch must reset the context to «без шаблона»"
        );
    })
    .await
    .expect(
        "create_single_with_number_check_mismatch_needs_confirmation_then_succeeds exceeded 30 s budget",
    );
}

// ---------------------------------------------------------------------------
// device_create/printer_create NUM-08 contexts stay isolated even though
// both read the same physical TemplateType::DeviceInventory space — a
// confirmed mismatch in ONE context must not touch the other's memory.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_single_with_number_check_printer_context_isolated_from_device_context() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let number_templates = number_templates_for(&svc);

        let template_id = number_templates
            .create(TemplateTypeDto::DeviceInventory, "ОРГ-00-[XXXXXX]".to_string())
            .await
            .expect("create device_inventory template")
            .id;

        // Establish device_create's remembered template first (matching
        // value -> no mismatch -> template IS remembered).
        svc.create_single_with_number_check(
            minimal_new("Устройство по шаблону", None),
            NumberFieldInput {
                value: "ОРГ-00-000001".to_string(),
                template_id: Some(template_id),
                confirm_mismatch: false,
                confirm_script_mix: false,
            },
            TemplateContextDto::DeviceCreate,
        )
        .await
        .expect("device_create seed")
        .expect_created("device_create seed");
        assert_eq!(
            number_templates
                .get_context(TemplateContextDto::DeviceCreate)
                .await
                .expect("get_context device_create after seed"),
            Some(template_id)
        );

        // Now a PRINTER create with a confirmed mismatch must reset ONLY
        // printer_create's memory, leaving device_create's remembered
        // template untouched.
        svc.create_single_with_number_check(
            minimal_new("Принтер вручную", None),
            NumberFieldInput {
                value: "ИНВ-9".to_string(),
                template_id: Some(template_id),
                confirm_mismatch: true,
                confirm_script_mix: false,
            },
            TemplateContextDto::PrinterCreate,
        )
        .await
        .expect("printer_create confirmed mismatch")
        .expect_created("printer_create confirmed mismatch");

        assert_eq!(
            number_templates
                .get_context(TemplateContextDto::PrinterCreate)
                .await
                .expect("get_context printer_create after confirmed mismatch"),
            None,
            "printer_create's own context must reset to «без шаблона»"
        );
        assert_eq!(
            number_templates
                .get_context(TemplateContextDto::DeviceCreate)
                .await
                .expect("get_context device_create must stay untouched"),
            Some(template_id),
            "device_create's remembered template must be UNAFFECTED by printer_create's mismatch"
        );
    })
    .await
    .expect(
        "create_single_with_number_check_printer_context_isolated_from_device_context exceeded 30 s budget",
    );
}

// ---------------------------------------------------------------------------
// D-01 order: occupied beats mismatch (a candidate that is BOTH occupied AND
// mismatched must surface as Conflict, never as NeedsConfirmation(mismatch)).
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_single_with_number_check_occupied_beats_mismatch() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let number_templates = number_templates_for(&svc);

        let template_id = number_templates
            .create(
                TemplateTypeDto::DeviceInventory,
                "ОРГ-00-[XXXXXX]".to_string(),
            )
            .await
            .expect("create device_inventory template")
            .id;

        // Baseline occupant with a number that would ALSO mismatch the
        // template above (does not matter for occupied — value is arbitrary).
        svc.create(minimal_new("Занято", Some("ИНВ-7")))
            .await
            .expect("seed occupant")
            .expect_created("seed occupant");

        let err = svc
            .create_single_with_number_check(
                minimal_new("Новое устройство", None),
                NumberFieldInput {
                    value: "ИНВ-7".to_string(),
                    template_id: Some(template_id),
                    confirm_mismatch: false,
                    confirm_script_mix: false,
                },
                TemplateContextDto::DeviceCreate,
            )
            .await
            .expect_err("occupied must win over mismatch — D-01 order");
        match err {
            trackly_core::error::AppError::Conflict { .. } => {}
            other => panic!("ожидали Conflict (occupied beats mismatch), получили {other:?}"),
        }
    })
    .await
    .expect("create_single_with_number_check_occupied_beats_mismatch exceeded 30 s budget");
}

// ---------------------------------------------------------------------------
// D-01 order: mismatch is checked BEFORE script-mix — a candidate that is
// simultaneously non-matching AND script-mixed must surface as
// NeedsConfirmation(Mismatch), never NeedsConfirmation(ScriptMix).
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_single_with_number_check_mismatch_beats_script_mix() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let number_templates = number_templates_for(&svc);

        let template_id = number_templates
            .create(
                TemplateTypeDto::DeviceInventory,
                "ОРГ-00-[XXXXXX]".to_string(),
            )
            .await
            .expect("create device_inventory template")
            .id;

        // "OPГ-7": Latin O/P mixed with Cyrillic Г (script-mix-eligible) AND
        // does not match the "ОРГ-00-XXXXXX" mask (mismatch-eligible).
        let mixed_and_mismatched = "OPГ-7";
        let outcome = svc
            .create_single_with_number_check(
                minimal_new("Новое устройство", None),
                NumberFieldInput {
                    value: mixed_and_mismatched.to_string(),
                    template_id: Some(template_id),
                    confirm_mismatch: false,
                    confirm_script_mix: false,
                },
                TemplateContextDto::DeviceCreate,
            )
            .await
            .expect("create_single_with_number_check mismatch+script-mix candidate");
        match outcome {
            DeviceSaveOutcome::NeedsConfirmation(warning) => {
                assert_eq!(
                    warning.kind,
                    NumberWarningKind::Mismatch,
                    "mismatch must be surfaced BEFORE script-mix (D-01 order), got: {:?}",
                    warning.kind
                );
            }
            DeviceSaveOutcome::Created(_) => {
                panic!("ожидали NeedsConfirmation для несоответствующего смешанного номера")
            }
        }
    })
    .await
    .expect("create_single_with_number_check_mismatch_beats_script_mix exceeded 30 s budget");
}

// ---------------------------------------------------------------------------
// BE-WR-10: inventory number shape — bounded length, no control characters.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_rejects_overlong_or_control_char_inventory_number() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        for bad in ["Я".repeat(65), "ИНВ\t000001".to_string()] {
            let err = svc.create(minimal_new("Устройство", Some(&bad))).await;
            assert!(
                matches!(
                    &err,
                    Err(trackly_core::error::AppError::Validation { field, .. })
                        if field == "inventory_no"
                ),
                "inventory number {bad:?} must be a validation error, got {err:?}"
            );
        }
    })
    .await
    .expect("budget");
}

/// BE-WR-01: the device create path must not write another section's
/// context memory, nor accept a template of another type.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_single_rejects_foreign_context_and_foreign_template() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let err = svc
            .create_single_with_number_check(
                minimal_new("Устройство", Some("ИНВ-1")),
                number_input("ИНВ-1"),
                TemplateContextDto::ActCreate,
            )
            .await;
        assert!(
            matches!(&err, Err(trackly_core::error::AppError::Validation { field, .. }) if field == "context"),
            "got {err:?}"
        );

        let nts = number_templates_for(&svc);
        let drum_tpl = nts
            .create(TemplateTypeDto::DrumCode, "ФБ-[XXX]".to_string())
            .await
            .expect("drum template");
        let mut input = number_input("ФБ-001");
        input.template_id = Some(drum_tpl.id);
        let err = svc
            .create_single_with_number_check(
                minimal_new("Устройство", None),
                input,
                TemplateContextDto::DeviceCreate,
            )
            .await;
        assert!(
            matches!(&err, Err(trackly_core::error::AppError::Validation { field, .. }) if field == "template_id"),
            "got {err:?}"
        );
    })
    .await
    .expect("budget");
}
