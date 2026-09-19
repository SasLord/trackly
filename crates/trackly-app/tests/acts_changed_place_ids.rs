//! `ActDto::changed_place_ids` regression tests — Phase 40.1 exhaustive sweep
//! (place-tree invalidation completeness).
//!
//! Every server-side act mutation that moves a device's `place_id` must
//! surface the touched place(s) back to the caller so the client can
//! invalidate place-tree counters for every place actually left/arrived at,
//! not just the act's own single `place_id` header field, which cannot
//! represent per-device source places in a batch operation. Covers four of
//! the five mutating entry points: `create`, `update` (add + remove device)
//! and `do_return`. `update_return` shares the exact same code shape as
//! `update`'s add/remove loops (same `record_movement_if_applicable` plus
//! `touched_place_ids.push` pattern) and is not re-tested here; it is
//! covered by symmetry with `update`'s two cases plus
//! `acts_update_return.rs`'s existing behavioral coverage. `delete_soft`
//! (undo cascade) is covered separately below.
//!
//! Только вымышленные ФИО/названия мест — privacy gate (CLAUDE.md).

use std::sync::Arc;
use std::time::Duration;

use rusqlite::params;
use trackly_app::dto::act::ActNumberEditInput;
use trackly_app::dto::act::{
    ActCreateDto, ActItemNewDto, ActReturnDto, ActReturnItemDto, ActUpdateDto, ActUpdateItemDto,
};
use trackly_app::dto::number_template::NumberFieldInput;
use trackly_app::services::ActService;
use trackly_core::auth::Identity;
use trackly_core::domain::places::{PlaceKind, PlaceNew};
use trackly_core::ports::places::PlaceRepository;
use trackly_core::primitives::clock::Clock;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::db::writer_worker::WriterHandle;
use trackly_infra::error_conversions::map_rusqlite;
use trackly_infra::repos::SqlitePlaceRepository;
use trackly_infra::test_support::test_writer_and_readers;

fn make_acts_service() -> (ActService, tempfile::TempDir) {
    let (writer, readers, dir) = test_writer_and_readers();
    let clock: Arc<dyn Clock + Send + Sync> = Arc::new(SystemClock);
    let svc = ActService::new(writer, readers, clock);
    (svc, dir)
}

async fn create_place(writer: &Arc<WriterHandle>, name: &str) -> i64 {
    let name = name.to_string();
    writer
        .execute(move |conn| {
            let repo = SqlitePlaceRepository;
            let new_place = PlaceNew {
                parent_id: None,
                kind: PlaceKind::Building,
                name: name.clone(),
                level: None,
                is_storage: false,
                sort_order: None,
                notes: None,
            };
            repo.create(conn, &new_place, 1_700_000_000)
        })
        .await
        .expect("create place")
}

async fn seed_device(writer: &Arc<WriterHandle>, name: &str) -> i64 {
    let name = name.to_string();
    writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO devices \
                 (type_id, name, status_id, version, created_at_utc, updated_at_utc) \
                 VALUES (1, ?1, 1, 1, ?2, ?2)",
                params![name, 1_700_000_000_i64],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed device")
}

/// Directly sets a device's `place_id` (bypassing any service) — used to
/// give an "added" device a distinct pre-existing place before `update()`
/// moves it, so the test can assert both the old AND new place surface.
async fn set_device_place(writer: &Arc<WriterHandle>, device_id: i64, place_id: i64) {
    writer
        .execute(move |conn| {
            conn.execute(
                "UPDATE devices SET place_id = ?1 WHERE id = ?2",
                params![place_id, device_id],
            )
            .map_err(map_rusqlite)
        })
        .await
        .expect("set device place");
}

// ---------------------------------------------------------------------------
// 1. create() — new handover surfaces the act's own place.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_populates_changed_place_ids() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let place_a = create_place(&svc.writer, "Здание А").await;
        let device_id = seed_device(&svc.writer, "Ноутбук-1").await;

        let act = svc
            .create(
                &Identity::trusted_admin(),
                ActCreateDto {
                    number_input: NumberFieldInput {
                        value: "9001".into(),
                        template_id: None,
                        confirm_mismatch: false,
                        confirm_script_mix: false,
                    },
                    giver_name: "Иванов И.И.".into(),
                    receiver_name: "Петров П.П.".into(),
                    place_id: Some(place_a),
                    notes: None,
                    deadline_utc: None,
                    handover_date_utc: None,
                    items: vec![ActItemNewDto {
                        device_id,
                        device_ids: Vec::new(),
                        quantity: 1,
                    }],
                },
            )
            .await
            .expect("create handover")
            .expect_created("create handover");

        // Device started with no place (`seed_device` leaves place_id NULL),
        // so only the new place (place_a) is Some — the null "before" is
        // dropped by dedupe_place_ids.
        assert_eq!(
            act.changed_place_ids,
            vec![place_a],
            "create must surface the handover's own place as changed"
        );
    })
    .await
    .expect("create_populates_changed_place_ids budget");
}

// ---------------------------------------------------------------------------
// 2. update() — adding a device that already sat at a DIFFERENT place
//    surfaces BOTH the device's prior place and the act's place.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn update_add_device_surfaces_old_and_new_place() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let place_a = create_place(&svc.writer, "Здание А").await;
        let place_b = create_place(&svc.writer, "Здание Б").await;
        let device1 = seed_device(&svc.writer, "Ноутбук-1").await;
        let device2 = seed_device(&svc.writer, "Ноутбук-2").await;
        // device2 already sits at place_b (e.g. warehouse), BEFORE it gets
        // added to a handover act targeting place_a.
        set_device_place(&svc.writer, device2, place_b).await;

        let act = svc
            .create(
                &Identity::trusted_admin(),
                ActCreateDto {
                    number_input: NumberFieldInput {
                        value: "9002".into(),
                        template_id: None,
                        confirm_mismatch: false,
                        confirm_script_mix: false,
                    },
                    giver_name: "Иванов И.И.".into(),
                    receiver_name: "Петров П.П.".into(),
                    place_id: Some(place_a),
                    notes: None,
                    deadline_utc: None,
                    handover_date_utc: None,
                    items: vec![ActItemNewDto {
                        device_id: device1,
                        device_ids: Vec::new(),
                        quantity: 1,
                    }],
                },
            )
            .await
            .expect("create handover")
            .expect_created("create handover");

        let updated = svc
            .update(
                &Identity::trusted_admin(),
                ActUpdateDto {
                    id: act.id,
                    expected_version: act.version,
                    number_input: ActNumberEditInput {
                        value: act.number_raw.clone(),
                        confirm_script_mix: false,
                    },
                    giver_name: act.giver_name.clone(),
                    receiver_name: act.receiver_name.clone(),
                    place_id: Some(place_a),
                    notes: None,
                    deadline_utc: None,
                    handover_date_utc: None,
                    items: vec![
                        ActUpdateItemDto {
                            device_id: device1,
                            complectation_at_time: None,
                        },
                        ActUpdateItemDto {
                            device_id: device2,
                            complectation_at_time: None,
                        },
                    ],
                },
            )
            .await
            .expect("update add device2")
            .expect_created("update add device2");

        let mut got = updated.changed_place_ids.clone();
        got.sort_unstable();
        let mut want = vec![place_a, place_b];
        want.sort_unstable();
        assert_eq!(
            got, want,
            "adding a device must surface BOTH its prior place (place_b) and the act's place (place_a)"
        );
    })
    .await
    .expect("update_add_device_surfaces_old_and_new_place budget");
}

// ---------------------------------------------------------------------------
// 3. update() — removing a device restores it to its pre-handover place
//    (here: no place / NULL), surfacing the in-act place it left.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn update_remove_device_surfaces_reverted_place() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let place_a = create_place(&svc.writer, "Здание А").await;
        let device1 = seed_device(&svc.writer, "Ноутбук-1").await;
        let device2 = seed_device(&svc.writer, "Ноутбук-2").await;

        let act = svc
            .create(
                &Identity::trusted_admin(),
                ActCreateDto {
                    number_input: NumberFieldInput {
                        value: "9003".into(),
                        template_id: None,
                        confirm_mismatch: false,
                        confirm_script_mix: false,
                    },
                    giver_name: "Иванов И.И.".into(),
                    receiver_name: "Петров П.П.".into(),
                    place_id: Some(place_a),
                    notes: None,
                    deadline_utc: None,
                    handover_date_utc: None,
                    items: vec![
                        ActItemNewDto {
                            device_id: device1,
                            device_ids: Vec::new(),
                            quantity: 1,
                        },
                        ActItemNewDto {
                            device_id: device2,
                            device_ids: Vec::new(),
                            quantity: 1,
                        },
                    ],
                },
            )
            .await
            .expect("create handover with 2 devices")
            .expect_created("create handover with 2 devices");

        // Drop device2 from the item set — it must revert to its
        // pre-handover place (NULL, since seed_device leaves it unset).
        let updated = svc
            .update(
                &Identity::trusted_admin(),
                ActUpdateDto {
                    id: act.id,
                    expected_version: act.version,
                    number_input: ActNumberEditInput {
                        value: act.number_raw.clone(),
                        confirm_script_mix: false,
                    },
                    giver_name: act.giver_name.clone(),
                    receiver_name: act.receiver_name.clone(),
                    place_id: Some(place_a),
                    notes: None,
                    deadline_utc: None,
                    handover_date_utc: None,
                    items: vec![ActUpdateItemDto {
                        device_id: device1,
                        complectation_at_time: None,
                    }],
                },
            )
            .await
            .expect("update remove device2")
            .expect_created("update remove device2");

        assert_eq!(
            updated.changed_place_ids,
            vec![place_a],
            "removing a device must surface the in-act place it reverted FROM \
             (the reverted-TO place is NULL here and correctly dropped)"
        );
    })
    .await
    .expect("update_remove_device_surfaces_reverted_place budget");
}

// ---------------------------------------------------------------------------
// 4. do_return() — returning to a warehouse place surfaces both the
//    handover's place and the return's target place.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn do_return_surfaces_handover_and_return_place() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let place_a = create_place(&svc.writer, "Здание А").await;
        let place_warehouse = create_place(&svc.writer, "Склад").await;
        let device_id = seed_device(&svc.writer, "Ноутбук-1").await;

        let act = svc
            .create(
                &Identity::trusted_admin(),
                ActCreateDto {
                    number_input: NumberFieldInput {
                        value: "9004".into(),
                        template_id: None,
                        confirm_mismatch: false,
                        confirm_script_mix: false,
                    },
                    giver_name: "Иванов И.И.".into(),
                    receiver_name: "Петров П.П.".into(),
                    place_id: Some(place_a),
                    notes: None,
                    deadline_utc: None,
                    handover_date_utc: None,
                    items: vec![ActItemNewDto {
                        device_id,
                        device_ids: Vec::new(),
                        quantity: 1,
                    }],
                },
            )
            .await
            .expect("create handover")
            .expect_created("create handover");

        let act_item_id = act.items.first().expect("one item").id;

        let ret = svc
            .do_return(
                &Identity::trusted_admin(),
                act.id,
                ActReturnDto {
                    bulk_condition: None,
                    bulk_place_id: Some(place_warehouse),
                    apply_to_all: true,
                    items: vec![ActReturnItemDto {
                        act_item_id,
                        device_id,
                        device_ids: vec![device_id],
                        quantity: 1,
                        condition_override: None,
                        place_id_override: None,
                    }],
                    giver_name: None,
                    receiver_name: None,
                    handover_date_utc: None,
                },
            )
            .await
            .expect("do_return");

        let mut got = ret.changed_place_ids.clone();
        got.sort_unstable();
        let mut want = vec![place_a, place_warehouse];
        want.sort_unstable();
        assert_eq!(
            got, want,
            "return must surface BOTH the handover's place (left) and the warehouse (arrived)"
        );
    })
    .await
    .expect("do_return_surfaces_handover_and_return_place budget");
}

// ---------------------------------------------------------------------------
// 5. delete_soft() — undoing a handover surfaces the in-act place being
//    reverted away from.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn delete_soft_surfaces_undone_place() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let place_a = create_place(&svc.writer, "Здание А").await;
        let device_id = seed_device(&svc.writer, "Ноутбук-1").await;

        let act = svc
            .create(
                &Identity::trusted_admin(),
                ActCreateDto {
                    number_input: NumberFieldInput {
                        value: "9005".into(),
                        template_id: None,
                        confirm_mismatch: false,
                        confirm_script_mix: false,
                    },
                    giver_name: "Иванов И.И.".into(),
                    receiver_name: "Петров П.П.".into(),
                    place_id: Some(place_a),
                    notes: None,
                    deadline_utc: None,
                    handover_date_utc: None,
                    items: vec![ActItemNewDto {
                        device_id,
                        device_ids: Vec::new(),
                        quantity: 1,
                    }],
                },
            )
            .await
            .expect("create handover")
            .expect_created("create handover");

        let changed = svc
            .delete_soft(act.id, act.version)
            .await
            .expect("delete_soft");

        assert_eq!(
            changed,
            vec![place_a],
            "undoing the handover must surface place_a (the reverted-TO place is \
             NULL here — device started with no place — and correctly dropped)"
        );
    })
    .await
    .expect("delete_soft_surfaces_undone_place budget");
}
