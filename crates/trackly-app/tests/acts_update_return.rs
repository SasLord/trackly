//! Acts update_return integration tests — Phase 22 Plan 02 (ACT-03).
//!
//! `ActService::update_return()` implements the full delta-reconciliation
//! flow for editing an existing **return** act: un-return (D-09.1),
//! add-outstanding (D-09.3), retained condition/location edit (D-09.2),
//! D-10 (empty item set rejected), D-11 (device-drift conflict guard, both
//! the re-issuance and manual-relocation paths), archived flag flips in
//! both directions, optimistic-lock CAS, and D-12 giver/receiver persistence
//! on the edit path.
//!
//! Helper scaffolding mirrors `acts_update.rs` (Phase 19) — same
//! `make_acts_service`/`seed_devices_with_state`/`seed_location`/
//! `create_handover_with_location`/`read_device_snap` shapes, plus two new
//! return-specific helpers: `do_return_for` and `update_return_dto_from`.
//!
//! Каждый тест wrapped в `tokio::time::timeout(30s)` (S-6).

use std::sync::Arc;
use std::time::Duration;

use rusqlite::params;
use trackly_app::dto::act::{
    ActCreateDto, ActDto, ActItemNewDto, ActReturnDto, ActReturnItemDto, ActUpdateReturnDto,
};
use trackly_app::dto::device::DevicePatch;
use trackly_app::services::{ActService, DeviceService};
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

async fn seed_devices_with_state(
    writer: &Arc<trackly_infra::db::writer_worker::WriterHandle>,
    count: usize,
    loc_a: i64,
    condition: &str,
) -> Vec<i64> {
    let names: Vec<String> = (0..count)
        .map(|i| format!("UpdateReturnTestDevice {i}"))
        .collect();
    let condition = condition.to_string();
    writer
        .execute(move |conn| {
            let tx = conn.transaction().map_err(map_rusqlite)?;
            let mut out = Vec::with_capacity(names.len());
            for name in &names {
                tx.execute(
                    "INSERT INTO devices \
                     (type_id, name, status_id, place_id, condition, version, \
                      created_at_utc, updated_at_utc) \
                     VALUES (1, ?1, 1, ?2, ?3, 1, ?4, ?4)",
                    params![name, loc_a, condition, 1_700_000_000_i64],
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

async fn seed_location(
    writer: &Arc<trackly_infra::db::writer_worker::WriterHandle>,
    name: &str,
) -> i64 {
    let name = name.to_string();
    writer
        .execute(move |conn| {
            let tx = conn.transaction().map_err(map_rusqlite)?;
            tx.execute(
                "INSERT INTO places (kind, name, created_at_utc, updated_at_utc) \
                 VALUES ('room', ?1, ?2, ?2)",
                params![name, 1_700_000_000_i64],
            )
            .map_err(map_rusqlite)?;
            let id = tx.last_insert_rowid();
            tx.commit().map_err(map_rusqlite)?;
            Ok(id)
        })
        .await
        .expect("seed loc")
}

async fn create_handover_with_location(
    svc: &ActService,
    device_ids: &[i64],
    place_id: i64,
) -> ActDto {
    svc.create(
        &Identity::trusted_admin(),
        ActCreateDto {
            number_override: None,
            giver_name: "Иванов И.И.".into(),
            receiver_name: "Петров П.П.".into(),
            place_id: Some(place_id),
            notes: None,
            deadline_utc: None,
            handover_date_utc: None,
            items: device_ids
                .iter()
                .map(|&id| ActItemNewDto {
                    device_id: id,
                    device_ids: Vec::new(),
                    quantity: 1,
                })
                .collect(),
        },
    )
    .await
    .expect("create handover")
}

#[derive(Debug, PartialEq)]
struct DeviceSnap {
    status_id: i64,
    place_id: Option<i64>,
    condition: Option<String>,
}

async fn read_device_snap(svc: &ActService, device_id: i64) -> DeviceSnap {
    let readers = svc.readers.clone();
    tokio::task::spawn_blocking(move || {
        let conn = readers.acquire();
        conn.query_row(
            "SELECT status_id, place_id, condition FROM devices WHERE id = ?1",
            params![device_id],
            |r| {
                Ok(DeviceSnap {
                    status_id: r.get(0)?,
                    place_id: r.get(1)?,
                    condition: r.get(2)?,
                })
            },
        )
        .expect("read device")
    })
    .await
    .expect("spawn_blocking")
}

/// Perform an initial `do_return` for the given device_ids off `handover`,
/// applying a single bulk condition/location to all of them. Returns the
/// fresh return `ActDto`.
async fn do_return_for(
    svc: &ActService,
    handover: &ActDto,
    device_ids: &[i64],
    condition: &str,
    place_id: i64,
) -> ActDto {
    let items: Vec<ActReturnItemDto> = device_ids
        .iter()
        .map(|&did| {
            let it = handover
                .items
                .iter()
                .find(|i| i.device_id == did)
                .expect("device_id must be a handover item");
            ActReturnItemDto {
                act_item_id: it.id,
                device_id: did,
                device_ids: vec![did],
                quantity: 1,
                condition_override: None,
                place_id_override: None,
            }
        })
        .collect();
    svc.do_return(
        handover.id,
        ActReturnDto {
            bulk_condition: Some(condition.into()),
            bulk_place_id: Some(place_id),
            apply_to_all: true,
            giver_name: None,
            receiver_name: None,
            handover_date_utc: None,
            items,
        },
    )
    .await
    .expect("do_return")
}

/// Build an `ActUpdateReturnDto` from an existing return `ActDto`, applying
/// a single bulk condition/location to `device_ids` (the full replacement
/// set for this edit).
fn update_return_dto_from(
    ret: &ActDto,
    device_ids: &[i64],
    condition: &str,
    place_id: i64,
) -> ActUpdateReturnDto {
    ActUpdateReturnDto {
        id: ret.id,
        expected_version: ret.version,
        giver_name: ret.giver_name.clone(),
        receiver_name: ret.receiver_name.clone(),
        place_id: ret.place_id,
        notes: None,
        deadline_utc: None,
        handover_date_utc: ret.handover_date_utc,
        bulk_condition: Some(condition.into()),
        bulk_place_id: Some(place_id),
        apply_to_all: true,
        items: device_ids
            .iter()
            .map(|&did| ActReturnItemDto {
                act_item_id: 0,
                device_id: did,
                device_ids: vec![did],
                quantity: 1,
                condition_override: None,
                place_id_override: None,
            })
            .collect(),
    }
}

async fn act_items_count(svc: &ActService, act_id: i64, device_id: i64) -> i64 {
    let readers = svc.readers.clone();
    tokio::task::spawn_blocking(move || {
        let conn = readers.acquire();
        conn.query_row(
            "SELECT COUNT(*) FROM act_items WHERE act_id = ?1 AND device_id = ?2",
            params![act_id, device_id],
            |r| r.get(0),
        )
        .expect("count act_items")
    })
    .await
    .expect("spawn_blocking")
}

// ---------------------------------------------------------------------------
// Test 1: retained_edit_changes_device_condition_location
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn retained_edit_changes_device_condition_location() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let loc_a = seed_location(&svc.writer, "Склад-A").await;
        let loc_b = seed_location(&svc.writer, "Склад-B").await;
        let loc_c = seed_location(&svc.writer, "Кабинет-C").await;
        let device_ids = seed_devices_with_state(&svc.writer, 1, loc_a, "Новое").await;
        let handover = create_handover_with_location(&svc, &device_ids, loc_a).await;

        let ret = do_return_for(&svc, &handover, &device_ids, "Хорошее", loc_b).await;

        let update = update_return_dto_from(&ret, &device_ids, "Б/У", loc_c);
        let updated = svc
            .update_return(update)
            .await
            .expect("update_return retained edit");
        assert_eq!(updated.items.len(), 1);

        let post = read_device_snap(&svc, device_ids[0]).await;
        assert_eq!(post.condition.as_deref(), Some("Б/У"), "condition updated");
        assert_eq!(post.place_id, Some(loc_c), "location updated");

        let readers = svc.readers.clone();
        let act_id = ret.id;
        let dev_id = device_ids[0];
        let stored_condition: Option<String> = tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            conn.query_row(
                "SELECT condition_at_time FROM act_items WHERE act_id = ?1 AND device_id = ?2",
                params![act_id, dev_id],
                |r| r.get(0),
            )
            .expect("read act_items")
        })
        .await
        .expect("spawn_blocking");
        assert_eq!(
            stored_condition.as_deref(),
            Some("Б/У"),
            "act_items.condition_at_time reflects the edit"
        );
    })
    .await
    .expect("retained_edit budget");
}

// ---------------------------------------------------------------------------
// Test 2: un_return_restores_prior_state
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn un_return_restores_prior_state() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let loc_a = seed_location(&svc.writer, "Склад-A").await;
        let loc_b = seed_location(&svc.writer, "Склад-B").await;
        let device_ids = seed_devices_with_state(&svc.writer, 2, loc_a, "Новое").await;
        let handover = create_handover_with_location(&svc, &device_ids, loc_a).await;

        let ret = do_return_for(&svc, &handover, &device_ids, "Хорошее", loc_b).await;
        let removed_id = device_ids[0];
        let kept_id = device_ids[1];

        // Remove device 0 from the return (keep only device 1).
        let update = update_return_dto_from(&ret, &[kept_id], "Хорошее", loc_b);
        let updated = svc
            .update_return(update)
            .await
            .expect("update_return un-return");
        assert_eq!(updated.items.len(), 1, "return now has 1 item");
        assert!(!updated.items.iter().any(|it| it.device_id == removed_id));

        // Removed device restored to its pre-return state (в_работе/loc_a/Новое
        // — the state it had immediately before this return's own do_return).
        let post = read_device_snap(&svc, removed_id).await;
        assert_eq!(post.status_id, 2, "restored to в_работе");
        assert_eq!(
            post.place_id,
            Some(loc_a),
            "restored to pre-return location"
        );
        assert_eq!(post.condition.as_deref(), Some("Новое"));

        // Kept device unaffected.
        let kept_post = read_device_snap(&svc, kept_id).await;
        assert_eq!(kept_post.status_id, 1, "kept device still на_складе");

        // act_items row for removed device is gone.
        assert_eq!(
            act_items_count(&svc, ret.id, removed_id).await,
            0,
            "act_items row for removed device is gone"
        );
    })
    .await
    .expect("un_return_restores_prior_state budget");
}

// ---------------------------------------------------------------------------
// Test 3: add_outstanding_device_to_return
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn add_outstanding_device_to_return() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let loc_a = seed_location(&svc.writer, "Склад-A").await;
        let loc_b = seed_location(&svc.writer, "Склад-B").await;
        let device_ids = seed_devices_with_state(&svc.writer, 2, loc_a, "Новое").await;
        let handover = create_handover_with_location(&svc, &device_ids, loc_a).await;

        // Return only device 0.
        let ret = do_return_for(&svc, &handover, &[device_ids[0]], "Хорошее", loc_b).await;

        let extra_id = device_ids[1];
        let pre_extra = read_device_snap(&svc, extra_id).await;
        assert_eq!(pre_extra.status_id, 2, "extra device starts в_работе");

        // Add the still-outstanding device (device 1) to the SAME return.
        let update = update_return_dto_from(&ret, &device_ids, "Хорошее", loc_b);
        let updated = svc
            .update_return(update)
            .await
            .expect("update_return add outstanding");
        assert_eq!(updated.items.len(), 2, "return now has 2 items");
        assert!(updated.items.iter().any(|it| it.device_id == extra_id));

        let post_extra = read_device_snap(&svc, extra_id).await;
        assert_eq!(post_extra.status_id, 1, "extra device now на_складе");
        assert_eq!(
            post_extra.place_id,
            Some(loc_b),
            "extra device at bulk location"
        );
        assert_eq!(post_extra.condition.as_deref(), Some("Хорошее"));
    })
    .await
    .expect("add_outstanding_device_to_return budget");
}

// ---------------------------------------------------------------------------
// Test 4: reject_empty_item_set (D-10)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn reject_empty_item_set() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let loc_a = seed_location(&svc.writer, "Склад-A").await;
        let device_ids = seed_devices_with_state(&svc.writer, 1, loc_a, "Новое").await;
        let handover = create_handover_with_location(&svc, &device_ids, loc_a).await;
        let ret = do_return_for(&svc, &handover, &device_ids, "Хорошее", loc_a).await;

        let mut update = update_return_dto_from(&ret, &device_ids, "Хорошее", loc_a);
        update.items = Vec::new();

        let err = svc
            .update_return(update)
            .await
            .expect_err("empty item set must be rejected");
        match err {
            AppError::Validation { field, .. } => assert_eq!(field, "items"),
            other => panic!("expected Validation, got {other:?}"),
        }

        // No mutation happened.
        let act_after = svc.get(ret.id).await.expect("re-fetch return");
        assert_eq!(act_after.version, ret.version, "version unchanged");
        assert_eq!(act_after.items.len(), 1, "items unchanged");
    })
    .await
    .expect("reject_empty_item_set budget");
}

// ---------------------------------------------------------------------------
// Test 5: reject_un_return_after_reissue (D-11, re-issuance path)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn reject_un_return_after_reissue() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let loc_a = seed_location(&svc.writer, "Склад-A").await;
        let loc_b = seed_location(&svc.writer, "Склад-B").await;
        let device_ids = seed_devices_with_state(&svc.writer, 2, loc_a, "Новое").await;
        let handover = create_handover_with_location(&svc, &device_ids, loc_a).await;

        // Return BOTH devices in one call.
        let ret = do_return_for(&svc, &handover, &device_ids, "Хорошее", loc_b).await;
        let reissued_id = device_ids[0];
        let other_id = device_ids[1];

        // Re-issue device 0 via a brand-new handover act (на_складе → в_работе).
        create_handover_with_location(&svc, &[reissued_id], loc_a).await;
        let drifted = read_device_snap(&svc, reissued_id).await;
        assert_eq!(drifted.status_id, 2, "reissued device now в_работе");

        // Attempt to un-return device 0 from the ORIGINAL return (items
        // non-empty — still contains device 1 — so D-10 does not trip).
        let update = update_return_dto_from(&ret, &[other_id], "Хорошее", loc_b);
        let err = svc
            .update_return(update)
            .await
            .expect_err("un-return after reissue must be rejected");
        match err {
            AppError::Conflict { .. } => {}
            other => panic!("expected Conflict, got {other:?}"),
        }

        // No mutation applied.
        let act_after = svc.get(ret.id).await.expect("re-fetch return");
        assert_eq!(act_after.version, ret.version, "version unchanged");
        assert_eq!(act_after.items.len(), 2, "items unchanged");
        let post = read_device_snap(&svc, reissued_id).await;
        assert_eq!(
            post.status_id, 2,
            "reissued device still в_работе (untouched)"
        );
    })
    .await
    .expect("reject_un_return_after_reissue budget");
}

// ---------------------------------------------------------------------------
// Test 6: reject_edit_after_manual_device_relocation (D-11, manual-relocation path)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn reject_edit_after_manual_device_relocation() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let loc_a = seed_location(&svc.writer, "Склад-A").await;
        let loc_b = seed_location(&svc.writer, "Склад-B").await;
        let loc_c = seed_location(&svc.writer, "Кабинет-C").await;
        let device_ids = seed_devices_with_state(&svc.writer, 1, loc_a, "Новое").await;
        let handover = create_handover_with_location(&svc, &device_ids, loc_a).await;
        let dev_id = device_ids[0];

        let ret = do_return_for(&svc, &handover, &device_ids, "Хорошее", loc_b).await;

        // Manual device-page edit: location only (status stays на_складе).
        let device_svc = DeviceService::new(
            svc.writer.clone(),
            svc.readers.clone(),
            Arc::new(SystemClock),
        );
        let dev_before = read_device_snap(&svc, dev_id).await;
        let readers_v = svc.readers.clone();
        let dev_version: i64 = tokio::task::spawn_blocking(move || {
            let conn = readers_v.acquire();
            conn.query_row(
                "SELECT version FROM devices WHERE id = ?1",
                params![dev_id],
                |r| r.get(0),
            )
            .expect("read device version")
        })
        .await
        .expect("spawn_blocking");
        device_svc
            .update(
                &Identity::trusted_admin(),
                dev_id,
                dev_version, // do_return already bumped this past the seed value
                DevicePatch {
                    type_id: None,
                    name: None,
                    inventory_no: None,
                    serial_no: None,
                    model: None,
                    specs: None,
                    kit: None,
                    state: None,
                    place_id: Some(Some(loc_c)),
                    status_id: None,
                },
            )
            .await
            .expect("manual device relocation");
        let dev_after = read_device_snap(&svc, dev_id).await;
        assert_eq!(
            dev_after.status_id, dev_before.status_id,
            "status unchanged (на_складе)"
        );
        assert_eq!(dev_after.place_id, Some(loc_c), "location manually changed");

        // update_return attempts to edit condition (apply_to_all=false
        // isolates the intent per-row) with a location override supplied
        // (WR-01 requires it when apply_to_all=false — see
        // `reject_update_return_missing_override_when_apply_to_all_false`
        // for that check in isolation). The override here does not need to
        // match the device's drifted location: D-11 still fires because the
        // 3-field snapshot compare catches the drifted location regardless
        // of what value this edit's own location override carries.
        let mut update = update_return_dto_from(&ret, &device_ids, "Б/У", loc_b);
        update.apply_to_all = false;
        update.bulk_condition = None;
        update.bulk_place_id = None;
        update.items = vec![ActReturnItemDto {
            act_item_id: 0,
            device_id: dev_id,
            device_ids: vec![dev_id],
            quantity: 1,
            condition_override: Some("Б/У".into()),
            place_id_override: Some(loc_b),
        }];

        let err = svc
            .update_return(update)
            .await
            .expect_err("edit after manual relocation must be rejected");
        match err {
            AppError::Conflict { .. } => {}
            other => panic!("expected Conflict, got {other:?}"),
        }
    })
    .await
    .expect("reject_edit_after_manual_device_relocation budget");
}

// ---------------------------------------------------------------------------
// Test 7: allow_edit_when_device_untouched
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn allow_edit_when_device_untouched() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let loc_a = seed_location(&svc.writer, "Склад-A").await;
        let loc_b = seed_location(&svc.writer, "Склад-B").await;
        let device_ids = seed_devices_with_state(&svc.writer, 1, loc_a, "Новое").await;
        let handover = create_handover_with_location(&svc, &device_ids, loc_a).await;

        let ret = do_return_for(&svc, &handover, &device_ids, "Хорошее", loc_b).await;

        // Re-submit the SAME condition/location — no drift since the return.
        let update = update_return_dto_from(&ret, &device_ids, "Хорошее", loc_b);
        let updated = svc
            .update_return(update)
            .await
            .expect("no-op resubmit on an untouched device must succeed");
        assert_eq!(updated.items.len(), 1);

        let post = read_device_snap(&svc, device_ids[0]).await;
        assert_eq!(post.condition.as_deref(), Some("Хорошее"));
        assert_eq!(post.place_id, Some(loc_b));
    })
    .await
    .expect("allow_edit_when_device_untouched budget");
}

// ---------------------------------------------------------------------------
// Test 8: add_last_device_archives_parent
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn add_last_device_archives_parent() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let loc_a = seed_location(&svc.writer, "Склад-A").await;
        let loc_b = seed_location(&svc.writer, "Склад-B").await;
        let device_ids = seed_devices_with_state(&svc.writer, 2, loc_a, "Новое").await;
        let handover = create_handover_with_location(&svc, &device_ids, loc_a).await;

        // Return only device 0 — parent not archived (1 of 2 returned).
        let ret = do_return_for(&svc, &handover, &[device_ids[0]], "Хорошее", loc_b).await;
        let parent_before = svc.get(handover.id).await.expect("get parent");
        assert!(!parent_before.archived, "not archived before edit (1 of 2)");

        // Add device 1 to the SAME return.
        let update = update_return_dto_from(&ret, &device_ids, "Хорошее", loc_b);
        svc.update_return(update)
            .await
            .expect("update_return add last device");

        let parent_after = svc.get(handover.id).await.expect("get parent");
        assert!(
            parent_after.archived,
            "parent must archive once the last outstanding device is added to this return"
        );
    })
    .await
    .expect("add_last_device_archives_parent budget");
}

// ---------------------------------------------------------------------------
// Test 9: un_return_unarchives_parent
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn un_return_unarchives_parent() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let loc_a = seed_location(&svc.writer, "Склад-A").await;
        let loc_b = seed_location(&svc.writer, "Склад-B").await;
        let device_ids = seed_devices_with_state(&svc.writer, 2, loc_a, "Новое").await;
        let handover = create_handover_with_location(&svc, &device_ids, loc_a).await;

        // Return BOTH devices — parent fully archived.
        let ret = do_return_for(&svc, &handover, &device_ids, "Хорошее", loc_b).await;
        let parent_before = svc.get(handover.id).await.expect("get parent");
        assert!(parent_before.archived, "fully returned → archived");

        // Un-return device 0 (keep device 1 in the return — non-empty items).
        let update = update_return_dto_from(&ret, &[device_ids[1]], "Хорошее", loc_b);
        svc.update_return(update)
            .await
            .expect("update_return un-return one device");

        let parent_after = svc.get(handover.id).await.expect("get parent");
        assert!(
            !parent_after.archived,
            "parent must unarchive once a device is un-returned"
        );
    })
    .await
    .expect("un_return_unarchives_parent budget");
}

// ---------------------------------------------------------------------------
// Test 10: version_mismatch_returns_conflict (CAS)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn version_mismatch_returns_conflict() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let loc_a = seed_location(&svc.writer, "Склад-A").await;
        let device_ids = seed_devices_with_state(&svc.writer, 1, loc_a, "Новое").await;
        let handover = create_handover_with_location(&svc, &device_ids, loc_a).await;
        let ret = do_return_for(&svc, &handover, &device_ids, "Хорошее", loc_a).await;

        let mut update = update_return_dto_from(&ret, &device_ids, "Б/У", loc_a);
        update.expected_version = ret.version - 1;

        let err = svc
            .update_return(update)
            .await
            .expect_err("stale version must fail");
        match err {
            AppError::OptimisticLockMismatch {
                entity,
                id,
                expected,
                actual,
            } => {
                assert_eq!(entity, "act");
                assert_eq!(id, ret.id);
                assert_eq!(expected, ret.version - 1);
                assert_eq!(actual, ret.version);
            }
            other => panic!("expected OptimisticLockMismatch, got {other:?}"),
        }

        let act_after = svc.get(ret.id).await.expect("re-fetch return");
        assert_eq!(act_after.version, ret.version, "version unchanged");
        let post = read_device_snap(&svc, device_ids[0]).await;
        assert_eq!(
            post.condition.as_deref(),
            Some("Хорошее"),
            "device unchanged"
        );
    })
    .await
    .expect("version_mismatch_returns_conflict budget");
}

// ---------------------------------------------------------------------------
// Test 11: edit_persists_giver_receiver (D-12, edit-path confirmation)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn edit_persists_giver_receiver() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let loc_a = seed_location(&svc.writer, "Склад-A").await;
        let device_ids = seed_devices_with_state(&svc.writer, 1, loc_a, "Новое").await;
        let handover = create_handover_with_location(&svc, &device_ids, loc_a).await;
        let ret = do_return_for(&svc, &handover, &device_ids, "Хорошее", loc_a).await;

        let mut update = update_return_dto_from(&ret, &device_ids, "Хорошее", loc_a);
        update.giver_name = "Новый Возвращающий".into();
        update.receiver_name = "Новый Принимающий".into();

        let updated = svc
            .update_return(update)
            .await
            .expect("update_return giver/receiver edit");
        assert_eq!(updated.giver_name, "Новый Возвращающий");
        assert_eq!(updated.receiver_name, "Новый Принимающий");

        let act_after = svc.get(ret.id).await.expect("re-fetch return");
        assert_eq!(act_after.giver_name, "Новый Возвращающий");
        assert_eq!(act_after.receiver_name, "Новый Принимающий");
    })
    .await
    .expect("edit_persists_giver_receiver budget");
}

// ---------------------------------------------------------------------------
// Test 12: retained_edit_condition_only_preserves_location (CR-01)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn retained_edit_condition_only_preserves_location() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let loc_a = seed_location(&svc.writer, "Склад-A").await;
        let loc_b = seed_location(&svc.writer, "Склад-B").await;
        let device_ids = seed_devices_with_state(&svc.writer, 1, loc_a, "Новое").await;
        let handover = create_handover_with_location(&svc, &device_ids, loc_a).await;
        let device_id = device_ids[0];

        let ret = do_return_for(&svc, &handover, &device_ids, "Хорошее", loc_b).await;

        // Condition-only edit: apply_to_all stays true, but the bulk
        // location is left empty (CR-01 repro — none of the earlier tests
        // exercise this because `update_return_dto_from` always supplies a
        // bulk location).
        let mut update = update_return_dto_from(&ret, &device_ids, "Б/У", loc_b);
        update.bulk_place_id = None;

        svc.update_return(update)
            .await
            .expect("update_return condition-only edit");

        let post = read_device_snap(&svc, device_id).await;
        assert_eq!(
            post.place_id,
            Some(loc_b),
            "location must be preserved (not NULLed) when only condition changes"
        );
        assert_eq!(
            post.condition.as_deref(),
            Some("Б/У"),
            "condition DID change"
        );
    })
    .await
    .expect("retained_edit_condition_only_preserves_location budget");
}

// ---------------------------------------------------------------------------
// Test 13: add_outstanding_device_without_bulk_location_preserves_current_location (CR-01)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn add_outstanding_device_without_bulk_location_preserves_current_location() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let loc_a = seed_location(&svc.writer, "Склад-A").await;
        let loc_b = seed_location(&svc.writer, "Склад-B").await;
        let device_ids = seed_devices_with_state(&svc.writer, 2, loc_a, "Новое").await;
        let handover = create_handover_with_location(&svc, &device_ids, loc_a).await;

        // Return only device 0 — device 1 stays outstanding at loc_a.
        let ret = do_return_for(&svc, &handover, &[device_ids[0]], "Хорошее", loc_b).await;
        let extra_id = device_ids[1];

        // Add device 1 to the same return with NO bulk/per-row location.
        let mut update = update_return_dto_from(&ret, &device_ids, "Хорошее", loc_b);
        update.bulk_place_id = None;

        svc.update_return(update)
            .await
            .expect("update_return add outstanding without bulk location");

        let post_extra = read_device_snap(&svc, extra_id).await;
        assert_eq!(
            post_extra.place_id,
            Some(loc_a),
            "device 1's ORIGINAL pre-add location must be preserved, not NULLed"
        );
    })
    .await
    .expect("add_outstanding_device_without_bulk_location_preserves_current_location budget");
}

// ---------------------------------------------------------------------------
// Test 14: un_return_after_retained_edit_restores_original_pre_return_state (CR-02)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn un_return_after_retained_edit_restores_original_pre_return_state() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let loc_a = seed_location(&svc.writer, "Склад-A").await;
        let loc_b = seed_location(&svc.writer, "Склад-B").await;
        let loc_c = seed_location(&svc.writer, "Кабинет-C").await;
        let device_ids = seed_devices_with_state(&svc.writer, 2, loc_a, "Новое").await;
        let handover = create_handover_with_location(&svc, &device_ids, loc_a).await;
        let dev1 = device_ids[0];
        let dev2 = device_ids[1];

        // 1. do_return both devices: в_работе/loc_a/Новое -> на_складе/loc_b/Хорошее.
        let ret = do_return_for(&svc, &handover, &device_ids, "Хорошее", loc_b).await;

        // 2. Edit dev1's condition+location within the same return (a THIRD
        // location, distinct from both pre-return and post-return states) —
        // writes dev1's custom:return_item_edit audit row (step 11).
        let update = update_return_dto_from(&ret, &device_ids, "Б/У", loc_c);
        let updated = svc
            .update_return(update)
            .await
            .expect("update_return retained edit on dev1+dev2");

        // 3. Un-return dev1 (remove it, keep dev2 so D-10's non-empty guard
        // doesn't trip).
        let update2 = update_return_dto_from(&updated, &[dev2], "Хорошее", loc_b);
        svc.update_return(update2)
            .await
            .expect("update_return un-return dev1 after retained edit");

        // dev1 must restore to its TRUE pre-return state (в_работе, loc_a,
        // "Новое") — NOT the intermediate post-return/pre-edit state
        // (на_складе, loc_c, "Б/У") that the buggy code would restore.
        let post_dev1 = read_device_snap(&svc, dev1).await;
        assert_eq!(
            post_dev1.status_id, 2,
            "restored to в_работе (true pre-return state)"
        );
        assert_eq!(
            post_dev1.place_id,
            Some(loc_a),
            "restored to original pre-return location, not loc_b or loc_c"
        );
        assert_eq!(
            post_dev1.condition.as_deref(),
            Some("Новое"),
            "restored to original pre-return condition, not Хорошее or Б/У"
        );
    })
    .await
    .expect("un_return_after_retained_edit_restores_original_pre_return_state budget");
}

// ---------------------------------------------------------------------------
// Test 15: reject_update_return_duplicate_device_id_across_items (WR-01)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn reject_update_return_duplicate_device_id_across_items() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let loc_a = seed_location(&svc.writer, "Склад-A").await;
        let device_ids = seed_devices_with_state(&svc.writer, 1, loc_a, "Новое").await;
        let handover = create_handover_with_location(&svc, &device_ids, loc_a).await;
        let ret = do_return_for(&svc, &handover, &device_ids, "Хорошее", loc_a).await;

        let mut update = update_return_dto_from(&ret, &device_ids, "Хорошее", loc_a);
        // Duplicate the same device_id in a second item — must be rejected
        // server-side, not silently collapsed via last-write-wins.
        let dup = update.items[0].clone();
        update.items.push(dup);

        let err = svc
            .update_return(update)
            .await
            .expect_err("duplicate device_id across items must be rejected");
        match err {
            AppError::Validation { field, .. } => {
                assert!(field.contains("device_ids"), "field={field}")
            }
            other => panic!("expected Validation, got {other:?}"),
        }
    })
    .await
    .expect("reject_update_return_duplicate_device_id_across_items budget");
}

// ---------------------------------------------------------------------------
// Test 16: reject_update_return_missing_override_when_apply_to_all_false (WR-01)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn reject_update_return_missing_override_when_apply_to_all_false() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let loc_a = seed_location(&svc.writer, "Склад-A").await;
        let device_ids = seed_devices_with_state(&svc.writer, 1, loc_a, "Новое").await;
        let handover = create_handover_with_location(&svc, &device_ids, loc_a).await;
        let ret = do_return_for(&svc, &handover, &device_ids, "Хорошее", loc_a).await;

        let mut update = update_return_dto_from(&ret, &device_ids, "Хорошее", loc_a);
        update.apply_to_all = false;
        // Leave the item's own condition_override/location_*_override at
        // their helper-default None — this must be rejected server-side.

        let err = svc
            .update_return(update)
            .await
            .expect_err("missing per-item override with apply_to_all=false must be rejected");
        match err {
            AppError::Validation { .. } => {}
            other => panic!("expected Validation, got {other:?}"),
        }
    })
    .await
    .expect("reject_update_return_missing_override_when_apply_to_all_false budget");
}

// ---------------------------------------------------------------------------
// Test 17: reject_add_when_device_already_returned_elsewhere_under_parent (WR-03)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn reject_add_when_device_already_returned_elsewhere_under_parent() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let loc_a = seed_location(&svc.writer, "Склад-A").await;
        let loc_b = seed_location(&svc.writer, "Склад-B").await;
        let device_ids = seed_devices_with_state(&svc.writer, 2, loc_a, "Новое").await;
        let dev_x = device_ids[0];
        let dev_y = device_ids[1];
        let p = create_handover_with_location(&svc, &device_ids, loc_a).await;

        // r1: return dev_x only — 1 of 2 returned, parent p NOT archived.
        let _r1 = do_return_for(&svc, &p, &[dev_x], "Хорошее", loc_b).await;

        // Re-issue dev_x via a SECOND, UNRELATED handover under a DIFFERENT
        // parent — dev_x becomes в_работе again; r1's act_items row for
        // dev_x is untouched.
        create_handover_with_location(&svc, &[dev_x], loc_a).await;
        let reissued = read_device_snap(&svc, dev_x).await;
        assert_eq!(reissued.status_id, 2, "dev_x re-issued, в_работе again");

        // r2: a SIBLING return under the SAME parent p, for dev_y.
        let r2 = do_return_for(&svc, &p, &[dev_y], "Хорошее", loc_b).await;

        // Attempt to add dev_x to r2 — passes the existence check (belongs
        // to p's act_items) and the status guard (в_работе, due to the
        // re-issue), but the WR-03 bound must reject it:
        // handover_qty=1, already_returned=1 (from r1), per_device_qty=1
        // -> 1+1>1.
        let update = update_return_dto_from(&r2, &[dev_y, dev_x], "Хорошее", loc_b);
        let err = svc
            .update_return(update)
            .await
            .expect_err("adding a device already covered by a sibling return must be rejected");
        match err {
            AppError::Validation { .. } => {}
            other => panic!("expected Validation, got {other:?}"),
        }

        // No partial mutation leaked — r2 still has only 1 item.
        let r2_after = svc.get(r2.id).await.expect("re-fetch r2");
        assert_eq!(r2_after.items.len(), 1, "r2 unchanged after rejected edit");
    })
    .await
    .expect("reject_add_when_device_already_returned_elsewhere_under_parent budget");
}

// ---------------------------------------------------------------------------
// Test 18: update_return_null_parent_act_id_returns_error_not_panic (WR-02)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn update_return_null_parent_act_id_returns_error_not_panic() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let loc_a = seed_location(&svc.writer, "Склад-A").await;
        let device_ids = seed_devices_with_state(&svc.writer, 1, loc_a, "Новое").await;
        let handover = create_handover_with_location(&svc, &device_ids, loc_a).await;
        let ret = do_return_for(&svc, &handover, &device_ids, "Хорошее", loc_a).await;

        // Corrupt the return row directly via SQL — simulates data
        // corruption / a bad import / a future migration bug (mirrors the
        // direct-SQL pattern `seed_devices_with_state`/`seed_location`
        // already use in this file).
        let ret_id = ret.id;
        svc.writer
            .execute(move |conn| {
                conn.execute(
                    "UPDATE acts SET parent_act_id = NULL WHERE id = ?1",
                    params![ret_id],
                )
                .map_err(map_rusqlite)?;
                Ok(())
            })
            .await
            .expect("corrupt parent_act_id");

        let update = update_return_dto_from(&ret, &device_ids, "Б/У", loc_a);
        let err = svc
            .update_return(update)
            .await
            .expect_err("NULL parent_act_id must return an error, not panic the writer task");
        assert!(
            matches!(err, AppError::Internal { .. }),
            "expected Internal, got {err:?}"
        );
    })
    .await
    .expect("update_return_null_parent_act_id_returns_error_not_panic budget");
}
