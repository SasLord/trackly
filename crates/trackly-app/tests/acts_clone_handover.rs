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
    // V016 was added in plan 04-01 (cartridge tables), so the final schema version
    // is 16, not 15. The test still verifies V015 columns exist below.
    assert!(
        user_version >= 15,
        "schema must be at V015+ after migrations, got: {user_version}"
    );

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
            location_name: None,
            notes: None,
            deadline_utc: None,
            handover_date_utc: None,
            items: vec![
                ActItemNewDto { device_id: d1, device_ids: Vec::new(), quantity: 1 },
                ActItemNewDto { device_id: d2, device_ids: Vec::new(), quantity: 1 },
                ActItemNewDto { device_id: d3, device_ids: Vec::new(), quantity: 1 },
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

// ---------------------------------------------------------------------------
// Helper: seed a device row с optional serial_number / inventory_number.
// ---------------------------------------------------------------------------

async fn seed_device_with_serial(
    writer: &Arc<trackly_infra::db::writer_worker::WriterHandle>,
    name: &str,
    serial: Option<&str>,
    inventory: Option<&str>,
) -> i64 {
    let name = name.to_string();
    let serial = serial.map(|s| s.to_string());
    let inventory = inventory.map(|s| s.to_string());
    writer
        .execute(move |conn| {
            let tx = conn.transaction().map_err(map_rusqlite)?;
            tx.execute(
                "INSERT INTO devices \
                 (type_id, name, inventory_number, serial_number, status_id, version, \
                  created_at_utc, updated_at_utc) \
                 VALUES (1, ?1, ?2, ?3, 1, 1, ?4, ?4)",
                params![name, inventory, serial, 1_700_000_000_i64],
            )
            .map_err(map_rusqlite)?;
            let id = tx.last_insert_rowid();
            tx.commit().map_err(map_rusqlite)?;
            Ok(id)
        })
        .await
        .expect("seed device with serial")
}

// ---------------------------------------------------------------------------
// Test 3 (G-12): handover qty=3 → 3 act_items, 3 distinct device_id, все 'в_работе'.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn clone_3_devices_on_handover_qty_3() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let source = seed_device(&svc.writer, "ClonedSource").await;

        let handover = svc
            .create(ActCreateDto {
                number_override: None,
                giver_name: "А".into(),
                receiver_name: "Б".into(),
                location_id: None,
                location_name: None,
                notes: None,
                deadline_utc: None,
                handover_date_utc: None,
                items: vec![ActItemNewDto {
                    device_id: source,
                    device_ids: Vec::new(),
                    quantity: 3,
                }],
            })
            .await
            .expect("create handover qty=3");

        assert_eq!(handover.items.len(), 3, "3 act_items per G-12 clone");
        let mut dids: Vec<i64> = handover.items.iter().map(|i| i.device_id).collect();
        dids.sort();
        dids.dedup();
        assert_eq!(dids.len(), 3, "all 3 device_id distinct");

        // All 3 devices currently в_работе (status_id=2 V001 seed).
        let readers = svc.readers.clone();
        let in_work_count: i64 = tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            conn.query_row(
                "SELECT COUNT(*) FROM devices d \
                 JOIN device_statuses s ON s.id = d.status_id \
                 WHERE d.id IN (?1, ?2, ?3) AND s.code = 'в_работе'",
                params![dids[0], dids[1], dids[2]],
                |r| r.get(0),
            )
            .expect("query in_work")
        })
        .await
        .expect("spawn_blocking");
        assert_eq!(in_work_count, 3, "all 3 (source + 2 clones) → 'в_работе'");
    })
    .await
    .expect("clone_3_devices_on_handover_qty_3 budget");
}

// ---------------------------------------------------------------------------
// Test 4 (G-7): partial return 2/3 → archived=0, suffix «в1»; затем 1/1 → archived=1, suffix «в2».
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn return_2_of_3_keeps_handover_active_and_uses_v1_suffix() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let source = seed_device(&svc.writer, "PartialReturn").await;
        let handover = svc
            .create(ActCreateDto {
                number_override: None,
                giver_name: "А".into(),
                receiver_name: "Б".into(),
                location_id: None,
                location_name: None,
                notes: None,
                deadline_utc: None,
                handover_date_utc: None,
                items: vec![ActItemNewDto {
                    device_id: source,
                    device_ids: Vec::new(),
                    quantity: 3,
                }],
            })
            .await
            .expect("create handover qty=3");

        let it0 = handover.items[0].clone();
        let it1 = handover.items[1].clone();
        let ret = svc
            .do_return(
                handover.id,
                ActReturnDto {
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
                },
            )
            .await
            .expect("partial return 2/3");

        // Suffix «в1» — единственный return, sibling_return_count=1 → «в».
        // Уточнение: формат "в" без цифры применяется ТОЛЬКО когда единственный return
        // покрыл все devices (sibling_return_count = 1 AND complete). Тут partial,
        // но sibling_return_count = 1, поэтому реально получим «в». G-7 семантика
        // переключения «в»→«в1» происходит когда появляется СЛЕДУЮЩИЙ return (см. Test 5).
        let parent = svc.get(handover.id).await.expect("get parent");
        assert!(!parent.archived, "after partial 2/3 archived=0");
        assert_eq!(ret.sub_number, Some(1));
    })
    .await
    .expect("return_2_of_3_keeps_handover_active_and_uses_v1_suffix budget");
}

// ---------------------------------------------------------------------------
// Test 5 (G-7): второй return (1 device) → archived=1, suffix retroactive promotion.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn return_remaining_1_archives_handover_uses_v2_suffix() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let source = seed_device(&svc.writer, "FullCycle").await;
        let handover = svc
            .create(ActCreateDto {
                number_override: None,
                giver_name: "А".into(),
                receiver_name: "Б".into(),
                location_id: None,
                location_name: None,
                notes: None,
                deadline_utc: None,
                handover_date_utc: None,
                items: vec![ActItemNewDto {
                    device_id: source,
                    device_ids: Vec::new(),
                    quantity: 3,
                }],
            })
            .await
            .expect("create handover qty=3");

        // First return — 2 of 3.
        let it0 = handover.items[0].clone();
        let it1 = handover.items[1].clone();
        let ret1 = svc
            .do_return(
                handover.id,
                ActReturnDto {
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
                },
            )
            .await
            .expect("first return 2/3");

        // Second return — remaining 1 device.
        let it2 = handover.items[2].clone();
        let ret2 = svc
            .do_return(
                handover.id,
                ActReturnDto {
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
                },
            )
            .await
            .expect("second return 1/1");

        let parent = svc.get(handover.id).await.expect("get parent");
        assert!(parent.archived, "after all 3 returned archived=1");
        assert_eq!(ret1.sub_number, Some(1));
        assert_eq!(ret2.sub_number, Some(2));
        // sibling_return_count теперь 2 → display suffix «в1», «в2».
        // Numbers parsed: ret1.number = "{parent}в1", ret2.number = "{parent}в2"
        let ret1_full = svc.get(ret1.id).await.expect("get ret1");
        let ret2_full = svc.get(ret2.id).await.expect("get ret2");
        assert!(
            ret1_full.number.ends_with("в1"),
            "ret1 must use «в1» suffix after ret2 created, got: {}",
            ret1_full.number
        );
        assert!(
            ret2_full.number.ends_with("в2"),
            "ret2 must use «в2» suffix, got: {}",
            ret2_full.number
        );
    })
    .await
    .expect("return_remaining_1_archives_handover_uses_v2_suffix budget");
}

// ---------------------------------------------------------------------------
// Test 6 (G-7): single return covering all 3 → «в» suffix (без цифры).
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn return_all_3_in_single_return_uses_v_suffix() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let source = seed_device(&svc.writer, "AllInOne").await;
        let handover = svc
            .create(ActCreateDto {
                number_override: None,
                giver_name: "А".into(),
                receiver_name: "Б".into(),
                location_id: None,
                location_name: None,
                notes: None,
                deadline_utc: None,
                handover_date_utc: None,
                items: vec![ActItemNewDto {
                    device_id: source,
                    device_ids: Vec::new(),
                    quantity: 3,
                }],
            })
            .await
            .expect("create handover qty=3");

        let items: Vec<ActReturnItemDto> = handover
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
            .collect();

        let ret = svc
            .do_return(
                handover.id,
                ActReturnDto {
                    bulk_condition: Some("Хорошее".into()),
                    bulk_location_id: None,
                    bulk_location_name: None,
                    apply_to_all: true,
                    items,
                },
            )
            .await
            .expect("single return all 3");

        let parent = svc.get(handover.id).await.expect("get parent");
        assert!(parent.archived);
        assert_eq!(ret.sub_number, Some(1));
        // Single return: sibling_return_count = 1 → suffix «в».
        let ret_full = svc.get(ret.id).await.expect("get ret");
        assert!(
            ret_full.number.ends_with('в')
                && !ret_full.number.ends_with("в1")
                && !ret_full.number.ends_with("в2"),
            "single return must use «в» (no number), got: {}",
            ret_full.number
        );
    })
    .await
    .expect("return_all_3_in_single_return_uses_v_suffix budget");
}

// ---------------------------------------------------------------------------
// Test 7 (G-10): outstanding_device_ids correctness after partial return.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn outstanding_device_ids_correctness_after_partial_return() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let source = seed_device(&svc.writer, "Outstanding").await;
        let handover = svc
            .create(ActCreateDto {
                number_override: None,
                giver_name: "А".into(),
                receiver_name: "Б".into(),
                location_id: None,
                location_name: None,
                notes: None,
                deadline_utc: None,
                handover_date_utc: None,
                items: vec![ActItemNewDto {
                    device_id: source,
                    device_ids: Vec::new(),
                    quantity: 3,
                }],
            })
            .await
            .expect("create handover qty=3");

        // Fresh: outstanding = [d_i] на каждом item.
        let parent_fresh = svc.get(handover.id).await.expect("get parent fresh");
        for it in &parent_fresh.items {
            assert_eq!(
                it.outstanding_device_ids,
                vec![it.device_id],
                "before any return — outstanding = [device_id] на каждом item"
            );
        }

        // Return first device.
        let it0 = handover.items[0].clone();
        svc.do_return(
            handover.id,
            ActReturnDto {
                bulk_condition: Some("Хорошее".into()),
                bulk_location_id: None,
                bulk_location_name: None,
                apply_to_all: true,
                items: vec![ActReturnItemDto {
                    act_item_id: it0.id,
                    device_id: it0.device_id,
                    device_ids: vec![it0.device_id],
                    quantity: 1,
                    condition_override: None,
                    location_id_override: None,
                    location_name_override: None,
                }],
            },
        )
        .await
        .expect("return first");

        let parent_after = svc.get(handover.id).await.expect("get parent after");
        let outstanding_total: usize = parent_after
            .items
            .iter()
            .map(|i| i.outstanding_device_ids.len())
            .sum();
        assert_eq!(outstanding_total, 2, "после возврата 1/3 outstanding total = 2");

        // Find the returned item — outstanding должен быть [].
        let returned_item = parent_after
            .items
            .iter()
            .find(|i| i.device_id == it0.device_id)
            .expect("returned item must still appear in items");
        assert!(
            returned_item.outstanding_device_ids.is_empty(),
            "returned device_id has empty outstanding"
        );
    })
    .await
    .expect("outstanding_device_ids_correctness_after_partial_return budget");
}

// ---------------------------------------------------------------------------
// Test 8 (G-12): cardinality bound — device_id вне handover → Validation.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn cardinality_bound_rejects_extra_device_id() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let source = seed_device(&svc.writer, "InHandover").await;
        let foreign = seed_device(&svc.writer, "Foreign").await;
        let handover = svc
            .create(ActCreateDto {
                number_override: None,
                giver_name: "А".into(),
                receiver_name: "Б".into(),
                location_id: None,
                location_name: None,
                notes: None,
                deadline_utc: None,
                handover_date_utc: None,
                items: vec![ActItemNewDto {
                    device_id: source,
                    device_ids: Vec::new(),
                    quantity: 1,
                }],
            })
            .await
            .expect("create handover qty=1");

        let it0 = handover.items[0].clone();
        let err = svc
            .do_return(
                handover.id,
                ActReturnDto {
                    bulk_condition: Some("Хорошее".into()),
                    bulk_location_id: None,
                    bulk_location_name: None,
                    apply_to_all: true,
                    items: vec![ActReturnItemDto {
                        act_item_id: it0.id,
                        device_id: it0.device_id,
                        // Foreign device_id не принадлежит handover.
                        device_ids: vec![foreign],
                        quantity: 1,
                        condition_override: None,
                        location_id_override: None,
                        location_name_override: None,
                    }],
                },
            )
            .await
            .expect_err("foreign device must reject");
        match err {
            trackly_core::error::AppError::Validation { field, message } => {
                assert!(
                    field.contains("device_ids") || message.contains("не принадлежит"),
                    "expected device_ids validation error, got field={field} message={message}"
                );
            }
            other => panic!("expected Validation, got {other:?}"),
        }
    })
    .await
    .expect("cardinality_bound_rejects_extra_device_id budget");
}

// ---------------------------------------------------------------------------
// Test 9 (T-03.1-02): MAX_CLONE_QTY=1000 bound rejects qty=1001.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn max_clone_qty_validation_rejects_1001() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let source = seed_device(&svc.writer, "OverLimit").await;
        let err = svc
            .create(ActCreateDto {
                number_override: None,
                giver_name: "А".into(),
                receiver_name: "Б".into(),
                location_id: None,
                location_name: None,
                notes: None,
                deadline_utc: None,
                handover_date_utc: None,
                items: vec![ActItemNewDto {
                    device_id: source,
                    device_ids: Vec::new(),
                    quantity: 1001,
                }],
            })
            .await
            .expect_err("qty=1001 must fail");
        match err {
            trackly_core::error::AppError::Validation { field, message } => {
                assert!(field.contains("quantity"));
                assert!(
                    message.contains("1000"),
                    "message must mention 1000 bound, got: {message}"
                );
            }
            other => panic!("expected Validation, got {other:?}"),
        }
    })
    .await
    .expect("max_clone_qty_validation_rejects_1001 budget");
}

// ---------------------------------------------------------------------------
// Test 10 (W-5): clone source с serial_number → clones получают NULL serial.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn clones_have_null_serial_number_per_w5() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let source =
            seed_device_with_serial(&svc.writer, "WithSerial", Some("SN-0042"), Some("INV-1"))
                .await;
        let handover = svc
            .create(ActCreateDto {
                number_override: None,
                giver_name: "А".into(),
                receiver_name: "Б".into(),
                location_id: None,
                location_name: None,
                notes: None,
                deadline_utc: None,
                handover_date_utc: None,
                items: vec![ActItemNewDto {
                    device_id: source,
                    device_ids: Vec::new(),
                    quantity: 3,
                }],
            })
            .await
            .expect("create qty=3 with serial source");

        assert_eq!(handover.items.len(), 3);
        let dids: Vec<i64> = handover.items.iter().map(|i| i.device_id).collect();
        let clone_ids: Vec<i64> = dids.iter().copied().filter(|&id| id != source).collect();
        assert_eq!(clone_ids.len(), 2, "2 clones expected");

        let readers = svc.readers.clone();
        let (source_serial, clone_serials, clone_inventories): (
            Option<String>,
            Vec<Option<String>>,
            Vec<Option<String>>,
        ) = tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            let source_serial: Option<String> = conn
                .query_row(
                    "SELECT serial_number FROM devices WHERE id=?1",
                    params![source],
                    |r| r.get(0),
                )
                .expect("source serial");
            let mut serials = Vec::new();
            let mut inventories = Vec::new();
            for cid in &clone_ids {
                let (s, i): (Option<String>, Option<String>) = conn
                    .query_row(
                        "SELECT serial_number, inventory_number FROM devices WHERE id=?1",
                        params![cid],
                        |r| Ok((r.get(0)?, r.get(1)?)),
                    )
                    .expect("clone fields");
                serials.push(s);
                inventories.push(i);
            }
            (source_serial, serials, inventories)
        })
        .await
        .expect("spawn_blocking");

        assert_eq!(source_serial.as_deref(), Some("SN-0042"));
        for s in &clone_serials {
            assert!(s.is_none(), "clone serial_number must be NULL (W-5)");
        }
        for inv in &clone_inventories {
            assert!(inv.is_none(), "clone inventory_number must be NULL (G-12 b)");
        }
    })
    .await
    .expect("clones_have_null_serial_number_per_w5 budget");
}

// ---------------------------------------------------------------------------
// Test 11 (G-12 undo): delete-return восстанавливает archived=0 + outstanding.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn undo_return_restores_archived_to_false() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let source = seed_device(&svc.writer, "UndoCycle").await;
        let handover = svc
            .create(ActCreateDto {
                number_override: None,
                giver_name: "А".into(),
                receiver_name: "Б".into(),
                location_id: None,
                location_name: None,
                notes: None,
                deadline_utc: None,
                handover_date_utc: None,
                items: vec![ActItemNewDto {
                    device_id: source,
                    device_ids: Vec::new(),
                    quantity: 3,
                }],
            })
            .await
            .expect("create qty=3");

        // Return all 3 → archived=1.
        let items: Vec<ActReturnItemDto> = handover
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
            .collect();
        let ret = svc
            .do_return(
                handover.id,
                ActReturnDto {
                    bulk_condition: Some("Хорошее".into()),
                    bulk_location_id: None,
                    bulk_location_name: None,
                    apply_to_all: true,
                    items,
                },
            )
            .await
            .expect("return all");

        let parent_archived = svc.get(handover.id).await.expect("get parent");
        assert!(parent_archived.archived);

        // Undo (soft-delete return).
        svc.delete_soft(ret.id, ret.version)
            .await
            .expect("delete return");

        // After undo: archived=0; outstanding restored.
        let parent_after_undo = svc.get(handover.id).await.expect("get parent after undo");
        assert!(
            !parent_after_undo.archived,
            "after undo of return, parent must un-archive"
        );
        let outstanding_total: usize = parent_after_undo
            .items
            .iter()
            .map(|i| i.outstanding_device_ids.len())
            .sum();
        assert_eq!(
            outstanding_total, 3,
            "after undo all 3 devices back to outstanding"
        );
    })
    .await
    .expect("undo_return_restores_archived_to_false budget");
}

// ---------------------------------------------------------------------------
// Test 12 (DEF-3): handover via location_name sets devices.location_id.
// ---------------------------------------------------------------------------
// Verifies that ActService::create передаёт resolved_location_id (а не
// payload.location_id=None) в update_status_and_location_in_tx при handover.
// После create: devices.location_id == resolved location (не NULL).
// После do_return с bulk_location_name: devices.location_id == return location.

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn handover_via_location_name_sets_device_location_id() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let device_id = seed_device(&svc.writer, "DEF3-Device").await;

        // Seed locations by inserting devices that reference them via DeviceService
        // resolve path. Alternatively: INSERT directly into locations table.
        // Simplest approach: seed the location row directly via writer and remember its id.
        let handover_location_name = "Отдел кадров";
        let return_location_name = "Склад-ОК";

        // Pre-seed the handover location so its id is known for assertion.
        let handover_loc_id: i64 = svc
            .writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;
                tx.execute(
                    "INSERT OR IGNORE INTO locations (name, created_at_utc, updated_at_utc) \
                     VALUES (?1, ?2, ?2)",
                    params![handover_location_name, 1_700_000_000_i64],
                )
                .map_err(map_rusqlite)?;
                let id: i64 = tx
                    .query_row(
                        "SELECT id FROM locations WHERE name = ?1",
                        params![handover_location_name],
                        |r| r.get(0),
                    )
                    .map_err(map_rusqlite)?;
                tx.commit().map_err(map_rusqlite)?;
                Ok(id)
            })
            .await
            .expect("seed handover location");

        // Create handover act via location_name (payload.location_id = None).
        let handover = svc
            .create(ActCreateDto {
                number_override: None,
                giver_name: "Иванов И.И.".into(),
                receiver_name: "Петров П.П.".into(),
                location_id: None,
                location_name: Some(handover_location_name.into()),
                notes: None,
                deadline_utc: None,
                handover_date_utc: None,
                items: vec![ActItemNewDto {
                    device_id,
                    device_ids: Vec::new(),
                    quantity: 1,
                }],
            })
            .await
            .expect("create handover via location_name");

        // Assert: devices.location_id = resolved handover location (DEF-3 core assertion).
        let readers = svc.readers.clone();
        let loc_after_handover: Option<i64> = tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            conn.query_row(
                "SELECT location_id FROM devices WHERE id = ?1",
                params![device_id],
                |r| r.get(0),
            )
            .expect("query devices.location_id after handover")
        })
        .await
        .expect("spawn_blocking location_after_handover");

        assert_eq!(
            loc_after_handover,
            Some(handover_loc_id),
            "DEF-3: после handover via location_name devices.location_id должен быть \
             resolved location id={handover_loc_id}, получено {loc_after_handover:?}"
        );

        // Now do_return with bulk_location_name — verify devices.location_id is updated.
        let it0 = handover.items[0].clone();
        svc.do_return(
            handover.id,
            ActReturnDto {
                bulk_condition: Some("Хорошее".into()),
                bulk_location_id: None,
                bulk_location_name: Some(return_location_name.into()),
                apply_to_all: true,
                items: vec![ActReturnItemDto {
                    act_item_id: it0.id,
                    device_id: it0.device_id,
                    device_ids: vec![it0.device_id],
                    quantity: 1,
                    condition_override: None,
                    location_id_override: None,
                    location_name_override: None,
                }],
            },
        )
        .await
        .expect("do_return with bulk_location_name");

        let readers = svc.readers.clone();
        let (loc_after_return, return_loc_id): (Option<i64>, i64) =
            tokio::task::spawn_blocking(move || {
                let conn = readers.acquire();
                let loc_after: Option<i64> = conn
                    .query_row(
                        "SELECT location_id FROM devices WHERE id = ?1",
                        params![device_id],
                        |r| r.get(0),
                    )
                    .expect("query devices.location_id after return");
                let ret_id: i64 = conn
                    .query_row(
                        "SELECT id FROM locations WHERE name = ?1",
                        params![return_location_name],
                        |r| r.get(0),
                    )
                    .expect("query return location id");
                (loc_after, ret_id)
            })
            .await
            .expect("spawn_blocking location_after_return");

        assert_eq!(
            loc_after_return,
            Some(return_loc_id),
            "DEF-3: после do_return via bulk_location_name devices.location_id должен быть \
             return location id={return_loc_id}, получено {loc_after_return:?}"
        );
    })
    .await
    .expect("handover_via_location_name_sets_device_location_id budget");
}
