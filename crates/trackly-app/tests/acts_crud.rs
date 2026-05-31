//! Acts CRUD integration tests — Plan 02 vertical slice.
//!
//! Covers: create handover happy path, override numbering audit, conflict on
//! reuse, rollback on invalid device, switch-bar counts, quantity persistence.
//!
//! Each test wrapped in `tokio::time::timeout(30s)` per S-6.

use std::sync::Arc;
use std::time::Duration;

use rusqlite::params;
use trackly_app::dto::act::{ActCreateDto, ActFilter, ActItemNewDto, Pagination};
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

/// Seed `count` minimal devices (status=1 = «На складе», type=1 = Устройство)
/// directly through the writer and return their IDs in insertion order.
async fn seed_devices(
    writer: &Arc<trackly_infra::db::writer_worker::WriterHandle>,
    count: usize,
) -> Vec<i64> {
    let names: Vec<String> = (0..count).map(|i| format!("TestDevice {i}")).collect();
    let ids = writer
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
        .expect("seed devices");
    ids
}

// ---------------------------------------------------------------------------
// Test 1: create_handover_happy
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_handover_happy() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let device_ids = seed_devices(&svc.writer, 3).await;

        let payload = ActCreateDto {
            number_override: None,
            giver_name: "Иванов И.И.".into(),
            receiver_name: "Петров П.П.".into(),
            location_id: None,
            notes: None,
            deadline_utc: None,
            items: device_ids
                .iter()
                .map(|&id| ActItemNewDto {
                    device_id: id,
                    quantity: 1,
                })
                .collect(),
        };
        let dto = svc.create(payload).await.expect("create");
        assert_eq!(dto.number, "1");
        assert_eq!(dto.number_raw, 1);
        assert_eq!(dto.act_type, "handover");
        assert_eq!(dto.items.len(), 3);

        // Devices switched to «В работе» (resolved via V014 code column).
        let readers = svc.readers.clone();
        let device_ids2 = device_ids.clone();
        let in_work_count: i64 = tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            let placeholders: String = device_ids2
                .iter()
                .enumerate()
                .map(|(i, _)| format!("?{}", i + 1))
                .collect::<Vec<_>>()
                .join(",");
            let sql = format!(
                "SELECT COUNT(*) FROM devices d \
                 JOIN device_statuses s ON s.id = d.status_id \
                 WHERE d.id IN ({placeholders}) AND s.code = 'в_работе'"
            );
            use rusqlite::types::ToSql;
            let params: Vec<&dyn ToSql> = device_ids2.iter().map(|id| id as &dyn ToSql).collect();
            conn.query_row(&sql, params.as_slice(), |r| r.get::<_, i64>(0))
                .expect("count")
        })
        .await
        .expect("spawn_blocking");
        assert_eq!(in_work_count, 3);

        // audit_log: 3 device-update rows + 1 act-create row.
        let readers2 = svc.readers.clone();
        let act_id = dto.id;
        let (device_audits, act_audits): (i64, i64) = tokio::task::spawn_blocking(move || {
            let conn = readers2.acquire();
            let d: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM audit_log \
                         WHERE entity_type='device' AND action='update' \
                           AND json_extract(payload_json, '$.act_id') = ?1",
                    params![act_id],
                    |r| r.get(0),
                )
                .expect("count device audits");
            let a: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM audit_log \
                         WHERE entity_type='act' AND entity_id=?1 AND action='create'",
                    params![act_id],
                    |r| r.get(0),
                )
                .expect("count act audits");
            (d, a)
        })
        .await
        .expect("spawn_blocking audits");
        assert_eq!(device_audits, 3);
        assert_eq!(act_audits, 1);
    })
    .await
    .expect("create_handover_happy budget");
}

// ---------------------------------------------------------------------------
// Test 2: override increments only audit, NOT counter
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_with_override_audits_and_increments_only_audit() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let ids = seed_devices(&svc.writer, 1).await;

        let payload = ActCreateDto {
            number_override: Some(99),
            giver_name: "А".into(),
            receiver_name: "Б".into(),
            location_id: None,
            notes: None,
            deadline_utc: None,
            items: vec![ActItemNewDto {
                device_id: ids[0],
                quantity: 1,
            }],
        };
        let dto = svc.create(payload).await.expect("create override");
        assert_eq!(dto.number_raw, 99);
        assert_eq!(dto.number, "99");

        // counter NOT incremented (still 0)
        let readers = svc.readers.clone();
        let act_id = dto.id;
        let (counter, override_count, payload_json): (i64, i64, String) =
            tokio::task::spawn_blocking(move || {
                let conn = readers.acquire();
                let counter: i64 = conn
                    .query_row(
                        "SELECT current_value FROM counters WHERE name='act_number'",
                        [],
                        |r| r.get(0),
                    )
                    .expect("counter");
                let oc: i64 = conn
                    .query_row(
                        "SELECT COUNT(*) FROM audit_log \
                         WHERE entity_type='act' AND entity_id=?1 \
                           AND action='custom:act_number_override'",
                        params![act_id],
                        |r| r.get(0),
                    )
                    .expect("override count");
                let pj: String = conn
                    .query_row(
                        "SELECT payload_json FROM audit_log \
                         WHERE entity_type='act' AND entity_id=?1 \
                           AND action='custom:act_number_override'",
                        params![act_id],
                        |r| r.get(0),
                    )
                    .expect("override payload");
                (counter, oc, pj)
            })
            .await
            .expect("spawn_blocking");
        assert_eq!(counter, 0, "counter must NOT increment on override");
        assert_eq!(override_count, 1);
        assert!(
            payload_json.contains("\"requested\":99"),
            "override payload should record requested number, got: {payload_json}"
        );
        assert!(
            payload_json.contains("\"next_auto_would_be\":1"),
            "override payload should record next auto, got: {payload_json}"
        );
    })
    .await
    .expect("override budget");
}

// ---------------------------------------------------------------------------
// Test 3: override number that already exists → Conflict
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn override_number_already_exists_returns_conflict() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let ids = seed_devices(&svc.writer, 2).await;

        // First create with auto numbering → number=1.
        let p1 = ActCreateDto {
            number_override: None,
            giver_name: "A".into(),
            receiver_name: "B".into(),
            location_id: None,
            notes: None,
            deadline_utc: None,
            items: vec![ActItemNewDto {
                device_id: ids[0],
                quantity: 1,
            }],
        };
        svc.create(p1).await.expect("first");

        let p2 = ActCreateDto {
            number_override: Some(1),
            giver_name: "C".into(),
            receiver_name: "D".into(),
            location_id: None,
            notes: None,
            deadline_utc: None,
            items: vec![ActItemNewDto {
                device_id: ids[1],
                quantity: 1,
            }],
        };
        let err = svc.create(p2).await.expect_err("conflict expected");
        match err {
            AppError::Conflict { reason } => {
                assert!(
                    reason.contains("№1") || reason.contains("1"),
                    "conflict reason should mention number 1, got: {reason}"
                );
            }
            other => panic!("expected Conflict, got {other:?}"),
        }
    })
    .await
    .expect("conflict budget");
}

// ---------------------------------------------------------------------------
// Test 4: rollback on invalid device id (counter stays put, no orphans)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn rollback_on_invalid_device_id() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();

        let payload = ActCreateDto {
            number_override: None,
            giver_name: "А".into(),
            receiver_name: "Б".into(),
            location_id: None,
            notes: None,
            deadline_utc: None,
            items: vec![ActItemNewDto {
                device_id: 9999,
                quantity: 1,
            }],
        };
        let err = svc.create(payload).await.expect_err("must fail");
        match err {
            AppError::NotFound { entity, id } => {
                assert_eq!(entity, "device");
                assert_eq!(id, 9999);
            }
            other => panic!("expected NotFound, got {other:?}"),
        }

        let readers = svc.readers.clone();
        let (counter, acts_total, items_total, audits_total): (i64, i64, i64, i64) =
            tokio::task::spawn_blocking(move || {
                let conn = readers.acquire();
                let counter: i64 = conn
                    .query_row(
                        "SELECT current_value FROM counters WHERE name='act_number'",
                        [],
                        |r| r.get(0),
                    )
                    .expect("counter");
                let a: i64 = conn
                    .query_row("SELECT COUNT(*) FROM acts", [], |r| r.get(0))
                    .expect("acts");
                let i: i64 = conn
                    .query_row("SELECT COUNT(*) FROM act_items", [], |r| r.get(0))
                    .expect("act_items");
                let au: i64 = conn
                    .query_row("SELECT COUNT(*) FROM audit_log", [], |r| r.get(0))
                    .expect("audit_log");
                (counter, a, i, au)
            })
            .await
            .expect("spawn_blocking");
        assert_eq!(counter, 0, "counter must roll back");
        assert_eq!(acts_total, 0, "no act rows after rollback");
        assert_eq!(items_total, 0, "no act_items after rollback");
        assert_eq!(audits_total, 0, "no audit_log rows after rollback");
    })
    .await
    .expect("rollback budget");
}

// ---------------------------------------------------------------------------
// Test 5: counts match switch-bar contract
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn counts_match_switch_bar() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let ids = seed_devices(&svc.writer, 3).await;
        for &id in &ids {
            let p = ActCreateDto {
                number_override: None,
                giver_name: "A".into(),
                receiver_name: "B".into(),
                location_id: None,
                notes: None,
                deadline_utc: None,
                items: vec![ActItemNewDto {
                    device_id: id,
                    quantity: 1,
                }],
            };
            svc.create(p).await.expect("create");
        }
        let counts = svc.counts().await.expect("counts");
        assert_eq!(counts.handover_active, 3);
        assert_eq!(counts.returns, 0);
        assert_eq!(counts.archived, 0);
    })
    .await
    .expect("counts budget");
}

// ---------------------------------------------------------------------------
// Test 6: handover qty>1 splits в N device-rows (G-12 clone-on-handover)
// ---------------------------------------------------------------------------
// PRE-V015 интент: «quantity column denorm column на act_items сохраняет N».
// POST-V015 G-12: qty=3 порождает 3 разных act_items (по одному device_id
// каждый, quantity=1), 2 из них клоны (parent_act_item_id != NULL).

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn handover_with_quantity_persists() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let ids = seed_devices(&svc.writer, 1).await;
        let source_id = ids[0];
        let payload = ActCreateDto {
            number_override: None,
            giver_name: "А".into(),
            receiver_name: "Б".into(),
            location_id: None,
            notes: None,
            deadline_utc: None,
            items: vec![ActItemNewDto {
                device_id: source_id,
                quantity: 3,
            }],
        };
        let dto = svc.create(payload).await.expect("create");
        // G-12: 3 act_items (1 original + 2 clones), все с quantity=1.
        assert_eq!(dto.items.len(), 3);
        for it in &dto.items {
            assert_eq!(it.quantity, 1);
        }
        // Все 3 device_id различны.
        let mut dids: Vec<i64> = dto.items.iter().map(|i| i.device_id).collect();
        dids.sort();
        dids.dedup();
        assert_eq!(dids.len(), 3);

        let readers = svc.readers.clone();
        let act_id = dto.id;
        let (rows, clones) = tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            let rows: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM act_items WHERE act_id=?1",
                    params![act_id],
                    |r| r.get(0),
                )
                .expect("count rows");
            let clones: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM act_items WHERE act_id=?1 AND parent_act_item_id IS NOT NULL",
                    params![act_id],
                    |r| r.get(0),
                )
                .expect("count clones");
            (rows, clones)
        })
        .await
        .expect("spawn_blocking");
        assert_eq!(rows, 3, "3 act_items per G-12 clone-on-handover");
        assert_eq!(clones, 2, "2 of 3 act_items имеют parent_act_item_id (clones)");
    })
    .await
    .expect("quantity budget");
}

// ---------------------------------------------------------------------------
// Test 7: validation rejects empty name / no items
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_validates_required_fields() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let ids = seed_devices(&svc.writer, 1).await;

        // Empty giver_name
        let p1 = ActCreateDto {
            number_override: None,
            giver_name: "".into(),
            receiver_name: "B".into(),
            location_id: None,
            notes: None,
            deadline_utc: None,
            items: vec![ActItemNewDto {
                device_id: ids[0],
                quantity: 1,
            }],
        };
        let err = svc.create(p1).await.expect_err("empty giver");
        match err {
            AppError::Validation { field, .. } => assert_eq!(field, "giver_name"),
            other => panic!("expected Validation, got {other:?}"),
        }

        // Empty items list
        let p2 = ActCreateDto {
            number_override: None,
            giver_name: "A".into(),
            receiver_name: "B".into(),
            location_id: None,
            notes: None,
            deadline_utc: None,
            items: vec![],
        };
        let err = svc.create(p2).await.expect_err("empty items");
        match err {
            AppError::Validation { field, .. } => assert_eq!(field, "items"),
            other => panic!("expected Validation, got {other:?}"),
        }
    })
    .await
    .expect("validation budget");
}

// ---------------------------------------------------------------------------
// Test 8: list returns created handover with items
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn list_returns_handover_with_items() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let ids = seed_devices(&svc.writer, 2).await;
        let payload = ActCreateDto {
            number_override: None,
            giver_name: "A".into(),
            receiver_name: "B".into(),
            location_id: None,
            notes: None,
            deadline_utc: None,
            items: ids
                .iter()
                .map(|&id| ActItemNewDto {
                    device_id: id,
                    quantity: 1,
                })
                .collect(),
        };
        svc.create(payload).await.expect("create");

        let filter = ActFilter {
            act_type: Some("handover".into()),
            archived: Some(false),
            search: None,
            include_deleted: false,
        };
        let resp = svc.list(filter, Pagination::default()).await.expect("list");
        assert_eq!(resp.total, 1);
        assert_eq!(resp.items.len(), 1);
        assert_eq!(resp.items[0].items.len(), 2);
    })
    .await
    .expect("list budget");
}
