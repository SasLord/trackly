//! Phase 40.2 Plan 07 (NUM-13/NUM-09) — cartridge/drum numbering integration
//! tests.
//!
//! REWRITTEN (not just extended) for this plan: the old tests here
//! (`concurrent_50_unique_codes`, `collision_retry_does_not_lose_counter`)
//! exercised the now-fully-retired `cartridge_seq`/`drum_seq` server-side
//! auto-generation mechanism directly — nothing to adapt them to, the
//! behavior they proved no longer exists in any form (matches Plan 06's
//! precedent for acts). Coverage now goes entirely through
//! `CartridgeService::create`, per this plan's own `<behavior>`:
//!   1. Explicit code, no template → created as-is (NUM-13).
//!   2. Empty code → hard validation error, no auto-generate fallback.
//!   3. Occupied check is LIVE-only (NUM-09 "в") — a live duplicate is
//!      rejected, but a soft-deleted cartridge's code can be reused.
//!   4. NUM-13 continuity smoke-test: a DB with live `C-0001..C-0042` (old
//!      `{seq:04}`-format codes, seeded directly via SQL) reports "C-0043"
//!      as the next number for the seeded "Картриджи" template — because
//!      the seeded mask `C-[XXXX]` has the SAME fixed digit-width (4) as
//!      the old format (D-16's continuity guarantee), not because of any
//!      special-cased logic in this plan.
//!   5. Cartridges (kind_id=1) and drums (kind_id=2) share ONE code space
//!      (NUM-09 "в" — both read the physical `cartridges.code` column) but
//!      have SEPARATE "last used template" context memory (NUM-08).
//!   6. T-40.2-16 (Tampering — race on one cartridge code): two concurrent
//!      `create()` calls for the SAME code — `idx_cartridges_code_live`
//!      (Plan 07 Task 1) guarantees exactly one success (mirrors acts'
//!      `concurrent_creates_same_number_exactly_one_succeeds`, Plan 06).
//!
//! Каждый тест обёрнут в `tokio::time::timeout` (S-6).

use std::sync::Arc;
use std::time::Duration;

use trackly_app::dto::cartridge::{CartridgeCreateDto, CartridgeModelCreateDto};
use trackly_app::dto::number_template::{NumberFieldInput, TemplateContextDto, TemplateTypeDto};
use trackly_app::services::{CartridgeService, NumberTemplateService};
use trackly_core::error::AppError;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::test_support::test_writer_and_readers;

fn make_cartridge_service() -> (CartridgeService, tempfile::TempDir) {
    let (writer, readers, dir) = test_writer_and_readers();
    let clock = Arc::new(SystemClock);
    let svc = CartridgeService::new(writer, readers, clock);
    (svc, dir)
}

fn number_input(value: &str) -> NumberFieldInput {
    NumberFieldInput {
        value: value.to_string(),
        template_id: None,
        confirm_mismatch: false,
        confirm_script_mix: false,
    }
}

async fn seed_model(svc: &CartridgeService, kind_id: i64) -> i64 {
    svc.model_create(CartridgeModelCreateDto {
        brand: "Pantum".into(),
        model: if kind_id == 2 {
            "DL-420X".into()
        } else {
            "TL-5120X".into()
        },
        kind_id,
        color: if kind_id == 2 {
            None
        } else {
            Some("Чёрный".into())
        },
        notes: None,
        compatibility: vec![],
    })
    .await
    .expect("seed model")
    .id
}

/// `CartridgeService::number_templates` is `pub(crate)` — this test is an
/// external integration crate, so it stands up its own
/// `NumberTemplateService` against the SAME writer/readers (mirrors
/// `acts_crud.rs::peek_next_number_frees_on_delete_of_last_act`'s precedent).
fn number_templates_for(svc: &CartridgeService) -> NumberTemplateService {
    NumberTemplateService::new(
        svc.writer.clone(),
        svc.readers.clone(),
        Arc::new(SystemClock),
    )
}

async fn seeded_template_id(svc: &CartridgeService, template_type: TemplateTypeDto) -> i64 {
    let number_templates = number_templates_for(svc);
    let seeded = number_templates
        .list(Some(template_type))
        .await
        .expect("list seeded templates");
    seeded.first().expect("V041 seeds one template per type").id
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_with_explicit_code_succeeds() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_cartridge_service();
        let model_id = seed_model(&svc, 1).await;

        let dto = svc
            .create(CartridgeCreateDto {
                model_id,
                number_input: number_input("BARCODE-42"),
                state_id: Some(1),
                place_id: None,
                notes: None,
            })
            .await
            .expect("create")
            .expect_created("create");

        assert_eq!(dto.code, "BARCODE-42");
        assert_eq!(dto.model_id, model_id);
        assert_eq!(dto.status_id, 1); // На складе
    })
    .await
    .expect("create_with_explicit_code_succeeds budget")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_with_empty_code_returns_validation() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_cartridge_service();
        let model_id = seed_model(&svc, 1).await;

        for candidate in ["", "   "] {
            let err = svc
                .create(CartridgeCreateDto {
                    model_id,
                    number_input: number_input(candidate),
                    state_id: None,
                    place_id: None,
                    notes: None,
                })
                .await
                .expect_err("empty/blank code must be rejected");
            match err {
                AppError::Validation { field, .. } => assert_eq!(field, "number"),
                other => panic!("expected Validation, got {other:?}"),
            }
        }
    })
    .await
    .expect("create_with_empty_code_returns_validation budget")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn live_duplicate_rejected_but_soft_deleted_code_reusable() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_cartridge_service();
        let model_id = seed_model(&svc, 1).await;

        let first = svc
            .create(CartridgeCreateDto {
                model_id,
                number_input: number_input("C-DUP-01"),
                state_id: None,
                place_id: None,
                notes: None,
            })
            .await
            .expect("create first")
            .expect_created("create first");

        // Live duplicate (case/whitespace-insensitive, NUM-09) → Conflict.
        let err = svc
            .create(CartridgeCreateDto {
                model_id,
                number_input: number_input("  c-dup-01  "),
                state_id: None,
                place_id: None,
                notes: None,
            })
            .await
            .expect_err("live duplicate must be rejected");
        assert!(matches!(err, AppError::Conflict { .. }), "got {err:?}");

        // Soft-delete the first cartridge — its code is now free (NUM-09 "в").
        svc.delete(first.id, first.version).await.expect("delete");

        let reused = svc
            .create(CartridgeCreateDto {
                model_id,
                number_input: number_input("C-DUP-01"),
                state_id: None,
                place_id: None,
                notes: None,
            })
            .await
            .expect("reuse after soft-delete must succeed")
            .expect_created("reuse after soft-delete must succeed");
        assert_eq!(reused.code, "C-DUP-01");
    })
    .await
    .expect("live_duplicate_rejected_but_soft_deleted_code_reusable budget")
}

/// NUM-13 continuity (D-16): existing `{seq:04}`-format codes survive
/// unchanged into the new template-driven world because the seeded mask's
/// digit-width (4) matches the old format's width exactly.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn continuity_next_after_seeded_max_is_0043() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_cartridge_service();
        let model_id = seed_model(&svc, 1).await;
        let now = 1_700_000_000_i64;

        // Seed C-0001..C-0042 directly (old {seq:04} format), all live.
        svc.writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(|e| AppError::Internal {
                    source_chain: format!("{e}"),
                })?;
                for n in 1..=42 {
                    let code = format!("C-{n:04}");
                    tx.execute(
                        "INSERT INTO cartridges \
                         (code, model_id, status_id, created_at_utc, updated_at_utc, version) \
                         VALUES (?1, ?2, 1, ?3, ?3, 1)",
                        rusqlite::params![code, model_id, now],
                    )
                    .map_err(|e| AppError::Internal {
                        source_chain: format!("{e}"),
                    })?;
                }
                tx.commit().map_err(|e| AppError::Internal {
                    source_chain: format!("{e}"),
                })?;
                Ok(())
            })
            .await
            .expect("seed C-0001..C-0042");

        let template_id = seeded_template_id(&svc, TemplateTypeDto::CartridgeCode).await;
        let number_templates = number_templates_for(&svc);
        let next = number_templates
            .peek_next(template_id)
            .await
            .expect("peek_next");

        assert_eq!(
            next.rendered, "C-0043",
            "next code after live C-0001..C-0042 must be C-0043 (D-16 continuity)"
        );
    })
    .await
    .expect("continuity_next_after_seeded_max_is_0043 budget")
}

/// NUM-09 "в": cartridges (kind_id=1) and drums (kind_id=2) share ONE
/// physical code space (a code used by a cartridge cannot be reused by a
/// drum) but have SEPARATE "last used template" context memory (NUM-08).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn cartridge_and_drum_share_code_space_but_not_context_memory() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_cartridge_service();
        let cartridge_model_id = seed_model(&svc, 1).await;
        let drum_model_id = seed_model(&svc, 2).await;

        let cartridge_template_id = seeded_template_id(&svc, TemplateTypeDto::CartridgeCode).await;
        let drum_template_id = seeded_template_id(&svc, TemplateTypeDto::DrumCode).await;

        // Create a cartridge with a templated code, remembering the template
        // for the cartridge_create context.
        svc.create(CartridgeCreateDto {
            model_id: cartridge_model_id,
            number_input: NumberFieldInput {
                value: "C-0100".into(),
                template_id: Some(cartridge_template_id),
                confirm_mismatch: false,
                confirm_script_mix: false,
            },
            state_id: None,
            place_id: None,
            notes: None,
        })
        .await
        .expect("create cartridge")
        .expect_created("create cartridge");

        // A DRUM cannot reuse the cartridge's code — one shared space.
        let err = svc
            .create(CartridgeCreateDto {
                model_id: drum_model_id,
                number_input: number_input("C-0100"),
                state_id: None,
                place_id: None,
                notes: None,
            })
            .await
            .expect_err("drum must not be able to reuse a live cartridge's code");
        assert!(matches!(err, AppError::Conflict { .. }), "got {err:?}");

        // Now create the drum with its OWN code + its OWN template.
        svc.create(CartridgeCreateDto {
            model_id: drum_model_id,
            number_input: NumberFieldInput {
                value: "D-0100".into(),
                template_id: Some(drum_template_id),
                confirm_mismatch: false,
                confirm_script_mix: false,
            },
            state_id: None,
            place_id: None,
            notes: None,
        })
        .await
        .expect("create drum")
        .expect_created("create drum");

        // Context memory (NUM-08) is separate per kind despite the shared space.
        let number_templates = number_templates_for(&svc);
        let cartridge_ctx = number_templates
            .get_context(TemplateContextDto::CartridgeCreate)
            .await
            .expect("get cartridge_create context");
        let drum_ctx = number_templates
            .get_context(TemplateContextDto::DrumCreate)
            .await
            .expect("get drum_create context");
        assert_eq!(cartridge_ctx, Some(cartridge_template_id));
        assert_eq!(drum_ctx, Some(drum_template_id));
    })
    .await
    .expect("cartridge_and_drum_share_code_space_but_not_context_memory budget")
}

/// T-40.2-16: two concurrent `create()` calls for the SAME code — exactly
/// one succeeds, backed by `idx_cartridges_code_live` (Plan 07 Task 1) as
/// the final race guard, not just the async pre-check.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_creates_same_code_exactly_one_succeeds() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_cartridge_service();
        let model_id = seed_model(&svc, 1).await;
        let svc = Arc::new(svc);

        let svc1 = svc.clone();
        let h1 = tokio::spawn(async move {
            svc1.create(CartridgeCreateDto {
                model_id,
                number_input: number_input("RACE-01"),
                state_id: None,
                place_id: None,
                notes: None,
            })
            .await
        });
        let svc2 = svc.clone();
        let h2 = tokio::spawn(async move {
            svc2.create(CartridgeCreateDto {
                model_id,
                number_input: number_input("RACE-01"),
                state_id: None,
                place_id: None,
                notes: None,
            })
            .await
        });

        let r1 = h1.await.expect("join1");
        let r2 = h2.await.expect("join2");

        let ok_count = [&r1, &r2].iter().filter(|r| r.is_ok()).count();
        let conflict_count = [&r1, &r2]
            .iter()
            .filter(|r| matches!(r, Err(AppError::Conflict { .. })))
            .count();
        assert_eq!(
            ok_count, 1,
            "exactly one of two concurrent same-code creates must succeed, got r1={r1:?} r2={r2:?}"
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
                "SELECT COUNT(*) FROM cartridges WHERE code = 'RACE-01' AND deleted_at_utc IS NULL",
                [],
                |r| r.get(0),
            )
            .expect("count")
        })
        .await
        .expect("spawn_blocking");
        assert_eq!(count, 1, "exactly one live cartridge with code 'RACE-01'");
    })
    .await
    .expect("concurrent_creates_same_code_exactly_one_succeeds budget")
}
