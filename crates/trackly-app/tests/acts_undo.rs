//! Acts undo integration tests — Plan 03 (D-Undo-01).
//!
//! Coverage:
//!   1. delete_handover_restores_devices_to_pre_handover (ACT-06)
//!   2. delete_handover_with_partial_return_cascades_undo
//!   3. delete_return_restores_to_handover_state_unarchives_parent (ACT-10)
//!   4. delete_act_optimistic_lock_mismatch
//!   5. delete_act_audits_undo_entries

use std::sync::Arc;
use std::time::Duration;

use rusqlite::params;
use trackly_app::dto::act::{ActCreateDto, ActDto, ActItemNewDto, ActReturnDto, ActReturnItemDto};
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

/// Seed devices with explicit fields: status=1 (на_складе), location_id=loc_a,
/// condition='Новое'. Returns IDs.
async fn seed_devices_with_state(
    writer: &Arc<trackly_infra::db::writer_worker::WriterHandle>,
    count: usize,
    loc_a: i64,
    condition: &str,
) -> Vec<i64> {
    let names: Vec<String> = (0..count).map(|i| format!("UndoTestDevice {i}")).collect();
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
        notes: None,
        deadline_utc: None,
        items: device_ids
            .iter()
            .map(|&id| ActItemNewDto {
                device_id: id,
                quantity: 1,
            })
            .collect(),
    })
    .await
    .expect("create handover")
}

#[derive(Debug)]
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

// ---------------------------------------------------------------------------
// Test 1: delete_handover_restores_devices_to_pre_handover (ACT-06)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn delete_handover_restores_devices_to_pre_handover() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let loc_a = seed_location(&svc.writer, "Склад-A").await;
        let loc_b = seed_location(&svc.writer, "Кабинет-B").await;
        let device_ids = seed_devices_with_state(&svc.writer, 3, loc_a, "Новое").await;

        // Pre-handover snapshots.
        let mut pre = Vec::new();
        for &id in &device_ids {
            pre.push(read_device_snap(&svc, id).await);
        }
        for snap in &pre {
            assert_eq!(snap.status_id, 1, "pre: status=1 (на_складе)");
            assert_eq!(snap.location_id, Some(loc_a));
            assert_eq!(snap.condition.as_deref(), Some("Новое"));
        }

        // Create handover → devices в работе / location_b.
        let handover = create_handover_with_location(&svc, &device_ids, loc_b).await;
        for &id in &device_ids {
            let snap = read_device_snap(&svc, id).await;
            assert_eq!(snap.status_id, 2, "after handover: 'в_работе'");
            assert_eq!(snap.location_id, Some(loc_b));
        }

        // Delete handover → undo each device.
        svc.delete_soft(handover.id, handover.version)
            .await
            .expect("delete handover");

        for &id in &device_ids {
            let snap = read_device_snap(&svc, id).await;
            assert_eq!(snap.status_id, 1, "undo: status restored to 1 (на_складе)");
            assert_eq!(snap.location_id, Some(loc_a), "undo: location restored");
            assert_eq!(
                snap.condition.as_deref(),
                Some("Новое"),
                "undo: condition restored"
            );
        }

        let counts = svc.counts().await.expect("counts");
        assert_eq!(counts.handover_active, 0);
    })
    .await
    .expect("delete_handover budget");
}

// ---------------------------------------------------------------------------
// Test 2: cascade — delete handover with active return
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn delete_handover_with_partial_return_cascades_undo() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let loc_a = seed_location(&svc.writer, "Склад-A").await;
        let loc_b = seed_location(&svc.writer, "Кабинет-B").await;
        let device_ids = seed_devices_with_state(&svc.writer, 3, loc_a, "Новое").await;

        let handover = create_handover_with_location(&svc, &device_ids, loc_b).await;

        // Partial return: возвращаем одно устройство в condition="Б/У".
        svc.do_return(
            handover.id,
            ActReturnDto {
                bulk_condition: Some("Б/У".into()),
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
        .expect("partial return");

        // Reload act (version обновилась после recompute_parent_archived).
        let handover_after = svc.get(handover.id).await.expect("get parent");

        // Delete handover → cascade undo all returns + handover.
        svc.delete_soft(handover.id, handover_after.version)
            .await
            .expect("delete handover with cascade");

        // All 3 devices back to pre-handover.
        for &id in &device_ids {
            let snap = read_device_snap(&svc, id).await;
            assert_eq!(snap.status_id, 1, "all devices on warehouse");
            assert_eq!(snap.location_id, Some(loc_a));
            assert_eq!(snap.condition.as_deref(), Some("Новое"));
        }

        let counts = svc.counts().await.expect("counts");
        assert_eq!(counts.handover_active, 0);
        assert_eq!(counts.returns, 0, "cascaded return also soft-deleted");
        assert_eq!(counts.archived, 0);
    })
    .await
    .expect("cascade budget");
}

// ---------------------------------------------------------------------------
// Test 3: delete return → un-archive parent + restore to handover state
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn delete_return_restores_to_handover_state_unarchives_parent() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let loc_a = seed_location(&svc.writer, "Склад-A").await;
        let loc_b = seed_location(&svc.writer, "Кабинет-B").await;
        let device_ids = seed_devices_with_state(&svc.writer, 2, loc_a, "Новое").await;

        let handover = create_handover_with_location(&svc, &device_ids, loc_b).await;

        // Full return → handover archived.
        let return_dto = svc
            .do_return(
                handover.id,
                ActReturnDto {
                    bulk_condition: Some("Хорошее".into()),
                    bulk_location_id: Some(loc_a),
                    bulk_location_name: None,
                    apply_to_all: true,
                    items: handover
                        .items
                        .iter()
                        .map(|it| ActReturnItemDto {
                            act_item_id: it.id,
                            device_id: it.device_id,
                device_ids: vec![it.device_id],
                            quantity: 1,
                            condition_override: None,
                            location_id_override: None,
                            location_name_override: None,
                        })
                        .collect(),
                },
            )
            .await
            .expect("do_return full");

        let parent_after_return = svc.get(handover.id).await.expect("get parent");
        assert!(parent_after_return.archived, "full return → archived=true");

        // Delete return → restore devices to handover-state (в работе/loc_b) + un-archive parent.
        svc.delete_soft(return_dto.id, return_dto.version)
            .await
            .expect("delete return");

        for &id in &device_ids {
            let snap = read_device_snap(&svc, id).await;
            assert_eq!(snap.status_id, 2, "back to 'в_работе' (handover state)");
            assert_eq!(snap.location_id, Some(loc_b));
        }

        let parent_after_delete = svc.get(handover.id).await.expect("get parent");
        assert!(
            !parent_after_delete.archived,
            "un-archive after delete return"
        );

        let counts = svc.counts().await.expect("counts");
        assert_eq!(counts.handover_active, 1);
        assert_eq!(counts.returns, 0);
        assert_eq!(counts.archived, 0);
    })
    .await
    .expect("delete return budget");
}

// ---------------------------------------------------------------------------
// Test 4: optimistic-lock mismatch
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn delete_act_optimistic_lock_mismatch() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let loc_a = seed_location(&svc.writer, "Склад").await;
        let device_ids = seed_devices_with_state(&svc.writer, 1, loc_a, "Новое").await;
        let handover = create_handover_with_location(&svc, &device_ids, loc_a).await;

        let wrong_version = handover.version + 99;
        let err = svc
            .delete_soft(handover.id, wrong_version)
            .await
            .expect_err("should fail");
        match err {
            AppError::OptimisticLockMismatch {
                entity,
                id,
                expected,
                actual,
            } => {
                assert_eq!(entity, "act");
                assert_eq!(id, handover.id);
                assert_eq!(expected, wrong_version);
                assert_eq!(actual, handover.version);
            }
            other => panic!("expected OptimisticLockMismatch, got {other:?}"),
        }
    })
    .await
    .expect("optimistic lock budget");
}

// ---------------------------------------------------------------------------
// Test 5: audit_log captures undo entries
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn delete_act_audits_undo_entries() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let loc_a = seed_location(&svc.writer, "Склад").await;
        let device_ids = seed_devices_with_state(&svc.writer, 2, loc_a, "Новое").await;
        let handover = create_handover_with_location(&svc, &device_ids, loc_a).await;

        svc.delete_soft(handover.id, handover.version)
            .await
            .expect("delete");

        let readers = svc.readers.clone();
        let act_id = handover.id;
        let (undo_count, delete_count): (i64, i64) = tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            let undo: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM audit_log \
                     WHERE entity_type = 'device' AND action = 'custom:undo' \
                       AND json_extract(payload_json, '$.undo_of_act_id') = ?1",
                    params![act_id],
                    |r| r.get(0),
                )
                .expect("count undo");
            let del: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM audit_log \
                     WHERE entity_type = 'act' AND entity_id = ?1 AND action = 'delete'",
                    params![act_id],
                    |r| r.get(0),
                )
                .expect("count del");
            (undo, del)
        })
        .await
        .expect("spawn");
        assert_eq!(undo_count, 2, "one custom:undo per device");
        assert_eq!(delete_count, 1, "one delete row for act");
    })
    .await
    .expect("audits budget");
}
