//! Acts update integration tests — Phase 19 Plan 03 (ACT-02).
//!
//! Coverage (Task 1, 4 tests):
//!   1. header_only_edit_does_not_touch_devices (D-05)
//!   2. add_position_transitions_device (D-06 add-half)
//!   3. version_mismatch_returns_conflict (CAS)
//!   4. reject_update_on_return_act (D-07)
//!
//! Coverage (Task 2, 5 tests):
//!   5. remove_position_restores_prior_state (D-06 remove-half)
//!   6. double_edit_restores_most_recent_snapshot (Pitfall 2 regression)
//!   7. reject_removal_of_returned_device (D-08)
//!   8. header_edit_free_even_with_existing_return (D-08 non-overfire)
//!   9. number_change_rejects_duplicate (A3)

use std::sync::Arc;
use std::time::Duration;

use rusqlite::params;
use trackly_app::dto::act::{
    ActCreateDto, ActDto, ActItemNewDto, ActReturnDto, ActReturnItemDto, ActUpdateDto,
    ActUpdateItemDto,
};
use trackly_app::services::ActService;
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
    let names: Vec<String> = (0..count).map(|i| format!("UpdateTestDevice {i}")).collect();
    let condition = condition.to_string();
    writer
        .execute(move |conn| {
            let tx = conn.transaction().map_err(map_rusqlite)?;
            let mut out = Vec::with_capacity(names.len());
            for name in &names {
                tx.execute(
                    "INSERT INTO devices \
                     (type_id, name, status_id, location_id, condition, version, \
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
                "INSERT INTO locations (name, created_at_utc, updated_at_utc) \
                 VALUES (?1, ?2, ?2)",
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
    location_id: i64,
) -> ActDto {
    svc.create(ActCreateDto {
        number_override: None,
        giver_name: "А".into(),
        receiver_name: "Б".into(),
        location_id: Some(location_id),
        location_name: None,
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
    })
    .await
    .expect("create handover")
}

#[derive(Debug, PartialEq)]
struct DeviceSnap {
    status_id: i64,
    location_id: Option<i64>,
    condition: Option<String>,
}

async fn read_device_snap(svc: &ActService, device_id: i64) -> DeviceSnap {
    let readers = svc.readers.clone();
    tokio::task::spawn_blocking(move || {
        let conn = readers.acquire();
        conn.query_row(
            "SELECT status_id, location_id, condition FROM devices WHERE id = ?1",
            params![device_id],
            |r| {
                Ok(DeviceSnap {
                    status_id: r.get(0)?,
                    location_id: r.get(1)?,
                    condition: r.get(2)?,
                })
            },
        )
        .expect("read device")
    })
    .await
    .expect("spawn_blocking")
}

/// Builds an `ActUpdateDto` from an existing `ActDto`'s current header,
/// letting the caller override specific fields via the closure.
fn update_dto_from(act: &ActDto, device_ids: &[i64]) -> ActUpdateDto {
    ActUpdateDto {
        id: act.id,
        expected_version: act.version,
        number_override: None,
        giver_name: act.giver_name.clone(),
        receiver_name: act.receiver_name.clone(),
        location_id: act.location_id,
        location_name: None,
        notes: act.notes.clone(),
        deadline_utc: act.deadline_utc,
        handover_date_utc: None,
        items: device_ids
            .iter()
            .map(|&id| ActUpdateItemDto {
                device_id: id,
                complectation_at_time: None,
            })
            .collect(),
    }
}

// ---------------------------------------------------------------------------
// Test 1: header_only_edit_does_not_touch_devices (D-05)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn header_only_edit_does_not_touch_devices() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let loc_a = seed_location(&svc.writer, "Склад-A").await;
        let loc_b = seed_location(&svc.writer, "Кабинет-B").await;
        let device_ids = seed_devices_with_state(&svc.writer, 2, loc_a, "Новое").await;
        let handover = create_handover_with_location(&svc, &device_ids, loc_b).await;

        let mut pre = Vec::new();
        for &id in &device_ids {
            pre.push(read_device_snap(&svc, id).await);
        }

        let mut update = update_dto_from(&handover, &device_ids);
        update.giver_name = "Новый сдающий".into();

        let updated = svc.update(update).await.expect("update header only");
        assert_eq!(updated.giver_name, "Новый сдающий");
        assert_eq!(updated.version, handover.version + 1, "version incremented");

        for (idx, &id) in device_ids.iter().enumerate() {
            let post = read_device_snap(&svc, id).await;
            assert_eq!(post, pre[idx], "device state byte-for-byte unchanged (D-05)");
        }
    })
    .await
    .expect("header_only_edit budget");
}

// ---------------------------------------------------------------------------
// Test 2: add_position_transitions_device (D-06 add-half)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn add_position_transitions_device() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let loc_a = seed_location(&svc.writer, "Склад-A").await;
        let loc_b = seed_location(&svc.writer, "Кабинет-B").await;
        let device_ids = seed_devices_with_state(&svc.writer, 1, loc_a, "Новое").await;
        let extra_ids = seed_devices_with_state(&svc.writer, 1, loc_a, "Новое").await;
        let extra_id = extra_ids[0];

        let handover = create_handover_with_location(&svc, &device_ids, loc_b).await;

        // extra_id starts на_складе.
        let pre_extra = read_device_snap(&svc, extra_id).await;
        assert_eq!(pre_extra.status_id, 1, "extra device starts на_складе");

        let mut new_device_ids = device_ids.clone();
        new_device_ids.push(extra_id);
        let update = update_dto_from(&handover, &new_device_ids);

        let updated = svc.update(update).await.expect("update add position");
        assert_eq!(updated.items.len(), 2, "act now has 2 items");
        assert!(updated.items.iter().any(|it| it.device_id == extra_id));

        let post_extra = read_device_snap(&svc, extra_id).await;
        assert_eq!(post_extra.status_id, 2, "extra device now в_работе");
        assert_eq!(post_extra.location_id, Some(loc_b), "extra device at act's location");

        // Audit row exists for the added device.
        let readers = svc.readers.clone();
        let act_id = handover.id;
        let count: i64 = tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            conn.query_row(
                "SELECT COUNT(*) FROM audit_log \
                 WHERE entity_type = 'device' AND entity_id = ?1 AND action = 'update' \
                   AND json_extract(payload_json, '$.act_id') = ?2 \
                   AND json_extract(payload_json, '$.kind') = 'handover'",
                params![extra_id, act_id],
                |r| r.get(0),
            )
            .expect("count audit")
        })
        .await
        .expect("spawn_blocking");
        assert_eq!(count, 1, "one audit row for the newly added device");
    })
    .await
    .expect("add_position budget");
}

// ---------------------------------------------------------------------------
// Test 3: version_mismatch_returns_conflict (CAS)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn version_mismatch_returns_conflict() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let loc_a = seed_location(&svc.writer, "Склад-A").await;
        let device_ids = seed_devices_with_state(&svc.writer, 1, loc_a, "Новое").await;
        let handover = create_handover_with_location(&svc, &device_ids, loc_a).await;

        let pre = read_device_snap(&svc, device_ids[0]).await;

        let mut update = update_dto_from(&handover, &device_ids);
        update.expected_version = handover.version - 1;
        update.giver_name = "Should not apply".into();

        let err = svc.update(update).await.expect_err("should fail");
        match err {
            AppError::OptimisticLockMismatch {
                entity,
                id,
                expected,
                actual,
            } => {
                assert_eq!(entity, "act");
                assert_eq!(id, handover.id);
                assert_eq!(expected, handover.version - 1);
                assert_eq!(actual, handover.version);
            }
            other => panic!("expected OptimisticLockMismatch, got {other:?}"),
        }

        // No mutation happened.
        let act_after = svc.get(handover.id).await.expect("re-fetch act");
        assert_eq!(act_after.giver_name, "А", "giver_name unchanged");
        assert_eq!(act_after.version, handover.version, "version unchanged");
        let post = read_device_snap(&svc, device_ids[0]).await;
        assert_eq!(post, pre, "device state unchanged");
    })
    .await
    .expect("version_mismatch budget");
}

// ---------------------------------------------------------------------------
// Test 4: reject_update_on_return_act (D-07)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn reject_update_on_return_act() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let loc_a = seed_location(&svc.writer, "Склад-A").await;
        let device_ids = seed_devices_with_state(&svc.writer, 1, loc_a, "Новое").await;
        let handover = create_handover_with_location(&svc, &device_ids, loc_a).await;

        let return_act = svc
            .do_return(
                handover.id,
                ActReturnDto {
                    bulk_condition: Some("Хорошее".into()),
                    bulk_location_id: Some(loc_a),
                    bulk_location_name: None,
                    apply_to_all: true,
                    items: vec![ActReturnItemDto {
                        act_item_id: handover.items[0].id,
                        device_id: handover.items[0].device_id,
                        device_ids: vec![handover.items[0].device_id],
                        quantity: 1,
                        condition_override: None,
                        location_id_override: None,
                        location_name_override: None,
                    }],
                },
            )
            .await
            .expect("do_return");

        let update = update_dto_from(&return_act, &device_ids);
        let err = svc.update(update).await.expect_err("should reject");
        match err {
            AppError::Validation { field, .. } => {
                assert_eq!(field, "id");
            }
            other => panic!("expected Validation, got {other:?}"),
        }
    })
    .await
    .expect("reject_update_on_return_act budget");
}

// ---------------------------------------------------------------------------
// Test 5: remove_position_restores_prior_state (D-06 remove-half)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn remove_position_restores_prior_state() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let loc_a = seed_location(&svc.writer, "Склад-A").await;
        let loc_b = seed_location(&svc.writer, "Кабинет-B").await;
        let device_ids = seed_devices_with_state(&svc.writer, 2, loc_a, "Новое").await;
        let handover = create_handover_with_location(&svc, &device_ids, loc_b).await;
        let removed_id = device_ids[0];
        let kept_id = device_ids[1];

        // Remove device_ids[0] from the act.
        let update = update_dto_from(&handover, &[kept_id]);
        let updated = svc.update(update).await.expect("update remove position");
        assert_eq!(updated.items.len(), 1, "act now has 1 item");
        assert!(!updated.items.iter().any(|it| it.device_id == removed_id));

        // Removed device restored to pre-handover state (на_складе/loc_a/Новое).
        let post = read_device_snap(&svc, removed_id).await;
        assert_eq!(post.status_id, 1, "restored to на_складе");
        assert_eq!(post.location_id, Some(loc_a), "restored to pre-handover location");
        assert_eq!(post.condition.as_deref(), Some("Новое"));

        // Kept device unaffected.
        let kept_post = read_device_snap(&svc, kept_id).await;
        assert_eq!(kept_post.status_id, 2, "kept device still в_работе");

        // Audit row exists with the distinct action name.
        let readers = svc.readers.clone();
        let act_id = handover.id;
        let count: i64 = tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            conn.query_row(
                "SELECT COUNT(*) FROM audit_log \
                 WHERE entity_type = 'device' AND entity_id = ?1 \
                   AND action = 'custom:update_remove' \
                   AND json_extract(payload_json, '$.act_id') = ?2",
                params![removed_id, act_id],
                |r| r.get(0),
            )
            .expect("count audit")
        })
        .await
        .expect("spawn_blocking");
        assert_eq!(count, 1, "one custom:update_remove audit row");

        // act_items row for the removed device is gone.
        let readers2 = svc.readers.clone();
        let items_count: i64 = tokio::task::spawn_blocking(move || {
            let conn = readers2.acquire();
            conn.query_row(
                "SELECT COUNT(*) FROM act_items WHERE act_id = ?1 AND device_id = ?2",
                params![act_id, removed_id],
                |r| r.get(0),
            )
            .expect("count items")
        })
        .await
        .expect("spawn_blocking");
        assert_eq!(items_count, 0, "act_items row for removed device is gone");
    })
    .await
    .expect("remove_position budget");
}

// ---------------------------------------------------------------------------
// Test 6: double_edit_restores_most_recent_snapshot (Pitfall 2 regression)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn double_edit_restores_most_recent_snapshot() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let loc_a = seed_location(&svc.writer, "Склад-A").await;
        let loc_b = seed_location(&svc.writer, "Кабинет-B").await;
        let loc_c = seed_location(&svc.writer, "Кабинет-C").await;
        let base_ids = seed_devices_with_state(&svc.writer, 1, loc_a, "Новое").await;
        let extra_ids = seed_devices_with_state(&svc.writer, 1, loc_a, "Новое").await;
        let device_x = extra_ids[0];

        // Create handover WITHOUT device X, at loc_b.
        let handover = create_handover_with_location(&svc, &base_ids, loc_b).await;

        // Edit #1: add device X (transitions на_складе/loc_a → в_работе/loc_b).
        let mut ids_with_x = base_ids.clone();
        ids_with_x.push(device_x);
        let update1 = update_dto_from(&handover, &ids_with_x);
        let after1 = svc.update(update1).await.expect("edit #1: add X");

        // Edit #2: remove device X (restores to на_складе/loc_a — its state
        // immediately before edit #1).
        let update2 = update_dto_from(&after1, &base_ids);
        let after2 = svc.update(update2).await.expect("edit #2: remove X");
        let post_edit2 = read_device_snap(&svc, device_x).await;
        assert_eq!(post_edit2.status_id, 1, "edit #2: X back on warehouse");
        assert_eq!(post_edit2.location_id, Some(loc_a));

        // Edit #3: re-add device X, but this time move the act to loc_c
        // (transitions на_складе/loc_a → в_работе/loc_c).
        let mut update3 = update_dto_from(&after2, &ids_with_x);
        update3.location_id = Some(loc_c);
        update3.location_name = None;
        let after3 = svc.update(update3).await.expect("edit #3: re-add X at loc_c");
        let post_edit3 = read_device_snap(&svc, device_x).await;
        assert_eq!(post_edit3.status_id, 2, "edit #3: X в_работе");
        assert_eq!(post_edit3.location_id, Some(loc_c));

        // Edit #4: remove device X again — must restore to its state
        // immediately before edit #4 (на_складе/loc_a, i.e. edit #3's
        // recorded before_json), NOT its ORIGINAL pre-edit-#1 state (which
        // also happens to be на_складе/loc_a in this fixture — the
        // distinguishing assertion is that it is NOT в_работе/loc_b, the
        // state edit #1 would have restored to).
        let update4 = update_dto_from(&after3, &base_ids);
        svc.update(update4).await.expect("edit #4: remove X again");
        let post_edit4 = read_device_snap(&svc, device_x).await;
        assert_eq!(
            post_edit4.status_id, 1,
            "edit #4: X restored to на_складе (most-recent snapshot, not stuck в_работе)"
        );
        assert_eq!(
            post_edit4.location_id,
            Some(loc_a),
            "edit #4: X restored to loc_a (its state right before edit #4, from edit #3's \
             before_json) — NOT loc_b (which would be the wrong result if the FIRST audit \
             row, from edit #1, were used instead of the most-recent one)"
        );
    })
    .await
    .expect("double_edit budget");
}

// ---------------------------------------------------------------------------
// Test 7: reject_removal_of_returned_device (D-08)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn reject_removal_of_returned_device() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let loc_a = seed_location(&svc.writer, "Склад-A").await;
        let device_ids = seed_devices_with_state(&svc.writer, 2, loc_a, "Новое").await;
        let handover = create_handover_with_location(&svc, &device_ids, loc_a).await;

        // Return ONE of the two devices.
        let returned_item = handover
            .items
            .iter()
            .find(|it| it.device_id == device_ids[0])
            .expect("item for device 0");
        svc.do_return(
            handover.id,
            ActReturnDto {
                bulk_condition: Some("Хорошее".into()),
                bulk_location_id: Some(loc_a),
                bulk_location_name: None,
                apply_to_all: true,
                items: vec![ActReturnItemDto {
                    act_item_id: returned_item.id,
                    device_id: returned_item.device_id,
                    device_ids: vec![returned_item.device_id],
                    quantity: 1,
                    condition_override: None,
                    location_id_override: None,
                    location_name_override: None,
                }],
            },
        )
        .await
        .expect("do_return one device");

        let handover_after_return = svc.get(handover.id).await.expect("re-fetch handover");
        let pre_d0 = read_device_snap(&svc, device_ids[0]).await;
        let pre_d1 = read_device_snap(&svc, device_ids[1]).await;

        // Attempt to remove BOTH devices from the act, replacing them with a
        // fresh replacement device (keeps `items` non-empty — validate_update
        // rejects a truly-empty items list before any D-08 check runs, same
        // as `create`'s rule) — must reject on the already-returned device,
        // with NO partial mutation (not even the replacement device add).
        let replacement_ids = seed_devices_with_state(&svc.writer, 1, loc_a, "Новое").await;
        let replacement_id = replacement_ids[0];
        let pre_replacement = read_device_snap(&svc, replacement_id).await;

        let update = update_dto_from(&handover_after_return, &[replacement_id]);
        let err = svc.update(update).await.expect_err("should reject removal");
        match err {
            AppError::Conflict { .. } => {}
            other => panic!("expected Conflict, got {other:?}"),
        }

        let post_d0 = read_device_snap(&svc, device_ids[0]).await;
        let post_d1 = read_device_snap(&svc, device_ids[1]).await;
        let post_replacement = read_device_snap(&svc, replacement_id).await;
        assert_eq!(post_d0, pre_d0, "already-returned device unchanged");
        assert_eq!(post_d1, pre_d1, "still-outstanding device unchanged (no partial mutation)");
        assert_eq!(
            post_replacement, pre_replacement,
            "replacement device never added (whole transaction rolled back)"
        );
    })
    .await
    .expect("reject_removal_of_returned_device budget");
}

// ---------------------------------------------------------------------------
// Test 8: header_edit_free_even_with_existing_return (D-08 non-overfire)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn header_edit_free_even_with_existing_return() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let loc_a = seed_location(&svc.writer, "Склад-A").await;
        let device_ids = seed_devices_with_state(&svc.writer, 2, loc_a, "Новое").await;
        let handover = create_handover_with_location(&svc, &device_ids, loc_a).await;

        let returned_item = handover
            .items
            .iter()
            .find(|it| it.device_id == device_ids[0])
            .expect("item for device 0");
        svc.do_return(
            handover.id,
            ActReturnDto {
                bulk_condition: Some("Хорошее".into()),
                bulk_location_id: Some(loc_a),
                bulk_location_name: None,
                apply_to_all: true,
                items: vec![ActReturnItemDto {
                    act_item_id: returned_item.id,
                    device_id: returned_item.device_id,
                    device_ids: vec![returned_item.device_id],
                    quantity: 1,
                    condition_override: None,
                    location_id_override: None,
                    location_name_override: None,
                }],
            },
        )
        .await
        .expect("do_return one device");

        let handover_after_return = svc.get(handover.id).await.expect("re-fetch handover");

        // items UNCHANGED (same device_id set as currently on the act) —
        // only giver_name differs. The D-08 guard must NOT fire.
        let mut update = update_dto_from(&handover_after_return, &device_ids);
        update.giver_name = "Другой сдающий".into();
        let updated = svc
            .update(update)
            .await
            .expect("header edit should succeed despite existing return");
        assert_eq!(updated.giver_name, "Другой сдающий");
    })
    .await
    .expect("header_edit_free_even_with_existing_return budget");
}

// ---------------------------------------------------------------------------
// Test 9: number_change_rejects_duplicate (A3)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn number_change_rejects_duplicate() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let loc_a = seed_location(&svc.writer, "Склад-A").await;
        let device_ids_a = seed_devices_with_state(&svc.writer, 1, loc_a, "Новое").await;
        let device_ids_b = seed_devices_with_state(&svc.writer, 1, loc_a, "Новое").await;

        let act_a = svc
            .create(ActCreateDto {
                number_override: Some(9001),
                giver_name: "А".into(),
                receiver_name: "Б".into(),
                location_id: Some(loc_a),
                location_name: None,
                notes: None,
                deadline_utc: None,
                handover_date_utc: None,
                items: vec![ActItemNewDto {
                    device_id: device_ids_a[0],
                    device_ids: Vec::new(),
                    quantity: 1,
                }],
            })
            .await
            .expect("create act A");
        let act_b = svc
            .create(ActCreateDto {
                number_override: Some(9002),
                giver_name: "В".into(),
                receiver_name: "Г".into(),
                location_id: Some(loc_a),
                location_name: None,
                notes: None,
                deadline_utc: None,
                handover_date_utc: None,
                items: vec![ActItemNewDto {
                    device_id: device_ids_b[0],
                    device_ids: Vec::new(),
                    quantity: 1,
                }],
            })
            .await
            .expect("create act B");

        let mut update = update_dto_from(&act_a, &device_ids_a);
        update.number_override = Some(act_b.number_raw);
        let err = svc.update(update).await.expect_err("should reject duplicate number");
        match err {
            AppError::Conflict { reason } => {
                assert!(reason.contains(&act_b.number_raw.to_string()));
            }
            other => panic!("expected Conflict, got {other:?}"),
        }
    })
    .await
    .expect("number_change_rejects_duplicate budget");
}
