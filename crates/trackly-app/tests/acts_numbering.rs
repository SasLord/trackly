//! Phase 40.2 Plan 06 (NUM-13/NUM-14) — act numbering integration tests.
//!
//! REWRITTEN (not just extended) for this plan: the old ACT-14 concurrency
//! test exercised `counters.act_number`/`increment_counter_in_tx` directly
//! (raw repo calls, no `ActService` involved) — that table and mechanism no
//! longer exist for acts (V041 dropped `counters`; V042 made `acts.number`
//! free TEXT). Coverage now goes entirely through `ActService::create`/
//! `update`, per this plan's own `<action>` instructions:
//!   1. Create with an explicit, non-numeric templated text number.
//!   2. Occupied check extends to a LIVE RETURN's DISPLAYED number (D-06) —
//!      both on `create()` and on `update()` (the literal review scenario:
//!      create #42, return it — displays "42в" — then a DIFFERENT act
//!      cannot be created OR renamed to "42в").
//!   3. Empty number (no template) is a hard validation error (NUM-13).
//!   4. T-40.2-14 (Tampering — race on one act number): two concurrent
//!      `create()` calls for the SAME number — the recreated
//!      `idx_acts_number_sub_unique` (V042) guarantees exactly one success.
//!
//! Каждый тест обёрнут в `tokio::time::timeout(30s)` (S-6).

use std::sync::Arc;
use std::time::Duration;

use rusqlite::params;
use trackly_app::dto::act::{
    ActCreateDto, ActItemNewDto, ActNumberEditInput, ActReturnDto, ActReturnItemDto, ActUpdateDto,
    ActUpdateItemDto,
};
use trackly_app::dto::number_template::NumberFieldInput;
use trackly_app::services::ActService;
use trackly_core::auth::Identity;
use trackly_core::error::AppError;
use trackly_core::primitives::clock::Clock;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::error_conversions::map_rusqlite;
use trackly_infra::test_support::test_writer_and_readers;

fn make_acts_service() -> (ActService, tempfile::TempDir) {
    let (writer, readers, dir) = test_writer_and_readers();
    let clock: Arc<dyn Clock + Send + Sync> = Arc::new(SystemClock);
    let svc = ActService::new(writer, readers, clock);
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

async fn seed_devices(
    writer: &Arc<trackly_infra::db::writer_worker::WriterHandle>,
    count: usize,
) -> Vec<i64> {
    let names: Vec<String> = (0..count)
        .map(|i| format!("NumberingTestDevice {i}"))
        .collect();
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

fn create_payload(number: &str, device_id: i64) -> ActCreateDto {
    ActCreateDto {
        number_input: number_input(number),
        giver_name: "Иванов И.И.".into(),
        receiver_name: "Петров П.П.".into(),
        place_id: None,
        notes: None,
        deadline_utc: None,
        handover_date_utc: None,
        items: vec![ActItemNewDto {
            device_id,
            device_ids: Vec::new(),
            quantity: 1,
        }],
    }
}

// ---------------------------------------------------------------------------
// Test 1: explicit free-text (templated, non-numeric) number succeeds.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_with_templated_text_number_succeeds() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let ids = seed_devices(&svc.writer, 1).await;

        let dto = svc
            .create(
                &Identity::trusted_admin(),
                create_payload("2026/09-1", ids[0]),
            )
            .await
            .expect("create")
            .expect_created("create");

        assert_eq!(dto.number, "2026/09-1");
        assert_eq!(dto.number_raw, "2026/09-1");
    })
    .await
    .expect("budget");
}

// ---------------------------------------------------------------------------
// Test 2 (D-06): occupied check on CREATE extends to a live return's
// DISPLAYED number — the literal review scenario.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_rejects_number_matching_live_return_display() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let ids = seed_devices(&svc.writer, 2).await;

        // Create act #42, then fully return it — a solo return displays
        // "42в" (D-Numbering-01: sibling_return_count == 1 drops the
        // sub_number suffix).
        let handover = svc
            .create(&Identity::trusted_admin(), create_payload("42", ids[0]))
            .await
            .expect("create #42")
            .expect_created("create #42");
        let item = handover.items.first().expect("one item");
        svc.do_return(
            &Identity::trusted_admin(),
            handover.id,
            ActReturnDto {
                bulk_condition: Some("Хорошее".into()),
                bulk_place_id: None,
                apply_to_all: true,
                giver_name: None,
                receiver_name: None,
                handover_date_utc: None,
                items: vec![ActReturnItemDto {
                    act_item_id: item.id,
                    device_id: item.device_id,
                    device_ids: vec![item.device_id],
                    quantity: 1,
                    condition_override: None,
                    place_id_override: None,
                }],
            },
        )
        .await
        .expect("do_return #42");

        // A brand-new act attempting to use "42в" (the return's DISPLAYED
        // number — never stored in a column) must be rejected as occupied,
        // even though no live act's raw `number` column literally equals
        // "42в".
        let err = svc
            .create(&Identity::trusted_admin(), create_payload("42в", ids[1]))
            .await
            .expect_err("must be rejected as occupied");
        match err {
            AppError::Conflict { .. } => {}
            other => panic!("expected Conflict, got {other:?}"),
        }
    })
    .await
    .expect("budget");
}

// ---------------------------------------------------------------------------
// Test 3 (D-06 on UPDATE, Task 4's literal review scenario): renaming a
// DIFFERENT act to a live return's displayed number is also rejected.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn update_rejects_rename_to_number_matching_live_return_display() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let ids = seed_devices(&svc.writer, 3).await;

        // Act #42, fully returned → its solo return displays "42в".
        let handover = svc
            .create(&Identity::trusted_admin(), create_payload("42", ids[0]))
            .await
            .expect("create #42")
            .expect_created("create #42");
        let item = handover.items.first().expect("one item");
        svc.do_return(
            &Identity::trusted_admin(),
            handover.id,
            ActReturnDto {
                bulk_condition: Some("Хорошее".into()),
                bulk_place_id: None,
                apply_to_all: true,
                giver_name: None,
                receiver_name: None,
                handover_date_utc: None,
                items: vec![ActReturnItemDto {
                    act_item_id: item.id,
                    device_id: item.device_id,
                    device_ids: vec![item.device_id],
                    quantity: 1,
                    condition_override: None,
                    place_id_override: None,
                }],
            },
        )
        .await
        .expect("do_return #42");

        // A SEPARATE act (#99) attempts to rename itself to "42в".
        let other = svc
            .create(&Identity::trusted_admin(), create_payload("99", ids[1]))
            .await
            .expect("create #99")
            .expect_created("create #99");

        let update = ActUpdateDto {
            id: other.id,
            expected_version: other.version,
            number_input: ActNumberEditInput {
                value: "42в".to_string(),
                confirm_script_mix: false,
            },
            giver_name: other.giver_name.clone(),
            receiver_name: other.receiver_name.clone(),
            place_id: other.place_id,
            notes: other.notes.clone(),
            deadline_utc: other.deadline_utc,
            handover_date_utc: None,
            items: vec![ActUpdateItemDto {
                device_id: ids[1],
                complectation_at_time: None,
            }],
        };
        let err = svc
            .update(&Identity::trusted_admin(), update)
            .await
            .expect_err("rename to a live return's displayed number must be rejected");
        match err {
            AppError::Conflict { .. } => {}
            other => panic!("expected Conflict, got {other:?}"),
        }
    })
    .await
    .expect("budget");
}

// ---------------------------------------------------------------------------
// Test 4 (NUM-13): empty number without a template is a hard validation
// error — there is no more server-side auto-generate to fall back on.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_with_empty_number_returns_validation() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let ids = seed_devices(&svc.writer, 1).await;

        // Both a bare empty string and a whitespace-only string must fail
        // identically (D-08: explicit `.trim()`).
        for candidate in ["", "   "] {
            let err = svc
                .create(
                    &Identity::trusted_admin(),
                    create_payload(candidate, ids[0]),
                )
                .await
                .expect_err("empty number must be rejected");
            match err {
                AppError::Validation { field, .. } => assert_eq!(field, "number"),
                other => panic!("expected Validation, got {other:?}"),
            }
        }
    })
    .await
    .expect("budget");
}

// ---------------------------------------------------------------------------
// Test 5 (T-40.2-14, Tampering — race on one act number): two concurrent
// `create()` calls for the SAME number — exactly one succeeds. The
// recreated `idx_acts_number_sub_unique` (V042) is the final backstop
// behind the async pre-check's small TOCTOU window (module doc-comment on
// `act_service.rs`).
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_creates_same_number_exactly_one_succeeds() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let ids = seed_devices(&svc.writer, 2).await;
        let svc = Arc::new(svc);

        let svc1 = svc.clone();
        let device1 = ids[0];
        let h1 = tokio::spawn(async move {
            svc1.create(&Identity::trusted_admin(), create_payload("77", device1))
                .await
        });
        let svc2 = svc.clone();
        let device2 = ids[1];
        let h2 = tokio::spawn(async move {
            svc2.create(&Identity::trusted_admin(), create_payload("77", device2))
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
            "exactly one of two concurrent same-number creates must succeed, got r1={r1:?} r2={r2:?}"
        );
        assert_eq!(
            conflict_count, 1,
            "the other must fail with Conflict (either pre-check or DB unique index), \
             got r1={r1:?} r2={r2:?}"
        );

        // Sanity: exactly one live act with number "77" in the DB.
        let readers = svc.readers.clone();
        let count: i64 = tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            conn.query_row(
                "SELECT COUNT(*) FROM acts WHERE number = '77' AND deleted_at_utc IS NULL",
                [],
                |r| r.get(0),
            )
            .expect("count")
        })
        .await
        .expect("spawn_blocking");
        assert_eq!(count, 1, "exactly one live act with number '77'");
    })
    .await
    .expect("concurrent_creates_same_number_exactly_one_succeeds budget");
}

// ---------------------------------------------------------------------------
// BE-WR-10: act number shape — bounded length, no control characters.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_rejects_overlong_or_control_char_number() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let ids = seed_devices(&svc.writer, 1).await;

        for bad in ["1".repeat(65), "42\n43".to_string()] {
            let err = svc
                .create(&Identity::trusted_admin(), create_payload(&bad, ids[0]))
                .await;
            assert!(
                matches!(&err, Err(AppError::Validation { field, .. }) if field == "number"),
                "number {bad:?} must be a validation error, got {err:?}"
            );
        }
    })
    .await
    .expect("budget");
}
