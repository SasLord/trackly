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
