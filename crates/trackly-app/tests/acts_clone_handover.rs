//! Phase 03.1-01 (G-7 / G-11 / G-12) — clone-on-handover integration tests.
//!
//! Verifies the V015 architectural shift:
//!   - V015 migration applies cleanly to `user_version = 15`.
//!   - `recompute_parent_archived` uses COUNT-based formula (count distinct
//!     device_id across return-acts vs handover act_items).
//!   - Plan 03.1-01 Task 1 scaffold (Tests 1-2). Tasks 3-4 expand with full
//!     clone lifecycle tests (8+ scenarios).
//!
//! All async tests wrapped in `tokio::time::timeout(30s)` (S-6).

use std::sync::Arc;
use std::time::Duration;

use rusqlite::params;
use trackly_app::dto::act::{ActCreateDto, ActItemNewDto, ActReturnDto, ActReturnItemDto};
use trackly_app::services::ActService;
use trackly_core::primitives::clock::Clock;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::error_conversions::map_rusqlite;
use trackly_infra::test_support::{test_db, test_writer_and_readers};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_acts_service() -> (ActService, tempfile::TempDir) {
    let (writer, readers, dir) = test_writer_and_readers();
    let clock: Arc<dyn Clock + Send + Sync> = Arc::new(SystemClock);
    let svc = ActService::new(writer, readers, clock);
    (svc, dir)
}

/// Seed a single source device, return its id.
async fn seed_device(
    writer: &Arc<trackly_infra::db::writer_worker::WriterHandle>,
    name: &str,
) -> i64 {
    let name = name.to_string();
    writer
        .execute(move |conn| {
            let tx = conn.transaction().map_err(map_rusqlite)?;
            tx.execute(
                "INSERT INTO devices \
                 (type_id, name, status_id, version, created_at_utc, updated_at_utc) \
                 VALUES (1, ?1, 1, 1, ?2, ?2)",
                params![name, 1_700_000_000_i64],
            )
            .map_err(map_rusqlite)?;
            let id = tx.last_insert_rowid();
            tx.commit().map_err(map_rusqlite)?;
            Ok(id)
        })
        .await
        .expect("seed device")
}

// ---------------------------------------------------------------------------
// Test 1: V015 migration applies → user_version = 15
// ---------------------------------------------------------------------------

#[test]
fn migration_v015_applies_clean() {
    let (conn, _guard) = test_db();
    let user_version: i64 = conn
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .expect("read user_version");
    assert_eq!(user_version, 15, "V015 must bring schema to version 15");

    // Verify new columns exist.
    let acts_columns: Vec<String> = {
        let mut stmt = conn.prepare("PRAGMA table_info(acts)").expect("prepare");
        stmt.query_map([], |r| r.get::<_, String>(1))
            .expect("query")
            .collect::<rusqlite::Result<Vec<_>>>()
            .expect("collect")
    };
    assert!(
        acts_columns.iter().any(|c| c == "handover_date_utc"),
        "acts.handover_date_utc must exist after V015, columns: {acts_columns:?}"
    );

    let act_items_columns: Vec<String> = {
        let mut stmt = conn
            .prepare("PRAGMA table_info(act_items)")
            .expect("prepare");
        stmt.query_map([], |r| r.get::<_, String>(1))
            .expect("query")
            .collect::<rusqlite::Result<Vec<_>>>()
            .expect("collect")
    };
    assert!(
        act_items_columns.iter().any(|c| c == "parent_act_item_id"),
        "act_items.parent_act_item_id must exist after V015, columns: {act_items_columns:?}"
    );
}

// ---------------------------------------------------------------------------
// Test 2: recompute_parent_archived uses COUNT-based formula (G-11)
//
// Setup: handover act with 3 act_items (3 distinct device_id, all status_id=2 «в работе»).
// Action 1: return-act with 2 act_items (2 of 3 device_id) → archived=0.
// Action 2: return-act with remaining 1 device_id → archived=1.
//
// This test exercises do_return() end-to-end on a 3-quantity-1 handover
// (no cloning needed yet — full clone tests come in Task 4).
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn recompute_parent_archived_count_based() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let d1 = seed_device(&svc.writer, "RP-1").await;
        let d2 = seed_device(&svc.writer, "RP-2").await;
        let d3 = seed_device(&svc.writer, "RP-3").await;

        let payload = ActCreateDto {
            number_override: None,
            giver_name: "Иванов И.И.".into(),
            receiver_name: "Петров П.П.".into(),
            location_id: None,
            notes: None,
            deadline_utc: None,
            items: vec![
                ActItemNewDto { device_id: d1, quantity: 1 },
                ActItemNewDto { device_id: d2, quantity: 1 },
                ActItemNewDto { device_id: d3, quantity: 1 },
            ],
        };
        let handover = svc.create(payload).await.expect("create handover");
        assert_eq!(handover.items.len(), 3);
        assert!(!handover.archived, "fresh handover must not be archived");

        // Step 1: return 2 of 3 devices → expect archived=0.
        let it0 = &handover.items[0];
        let it1 = &handover.items[1];
        let ret_payload = ActReturnDto {
            bulk_condition: Some("Хорошее".into()),
            bulk_location_id: None,
            bulk_location_name: None,
            apply_to_all: true,
            items: vec![
                ActReturnItemDto {
                    act_item_id: it0.id,
                    device_id: it0.device_id,
                device_ids: vec![it0.device_id],
                    quantity: 1,
                    condition_override: None,
                    location_id_override: None,
                    location_name_override: None,
                },
                ActReturnItemDto {
                    act_item_id: it1.id,
                    device_id: it1.device_id,
                device_ids: vec![it1.device_id],
                    quantity: 1,
                    condition_override: None,
                    location_id_override: None,
                    location_name_override: None,
                },
            ],
        };
        svc.do_return(handover.id, ret_payload)
            .await
            .expect("do_return partial");

        let parent = svc.get(handover.id).await.expect("get parent");
        assert!(
            !parent.archived,
            "after 2 of 3 returned, handover must remain active (archived=0)"
        );

        // Step 2: return remaining 1 device → archived=1.
        let it2 = &handover.items[2];
        let ret_payload_2 = ActReturnDto {
            bulk_condition: Some("Хорошее".into()),
            bulk_location_id: None,
            bulk_location_name: None,
            apply_to_all: true,
            items: vec![ActReturnItemDto {
                act_item_id: it2.id,
                device_id: it2.device_id,
                device_ids: vec![it2.device_id],
                quantity: 1,
                condition_override: None,
                location_id_override: None,
                location_name_override: None,
            }],
        };
        svc.do_return(handover.id, ret_payload_2)
            .await
            .expect("do_return remaining");

        let parent = svc.get(handover.id).await.expect("get parent");
        assert!(
            parent.archived,
            "after all 3 of 3 returned, handover must be archived (archived=1)"
        );
    })
    .await
    .expect("recompute_parent_archived_count_based budget");
}
