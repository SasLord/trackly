//! Acts returns integration tests — Plan 03 vertical slice + Plan 06 gap closure.
//!
//! Covers (per VALIDATION + plan behavior list):
//!   - partial_return_keeps_handover_active
//!   - full_return_archives_handover
//!   - second_partial_return_assigns_sub_number_2_and_promotes_suffix
//!   - bulk_apply_with_per_row_override
//!   - return_when_apply_to_all_false_requires_per_row_values
//!   - return_concurrent_two_returns_correct_sub_numbers
//!   - return_does_not_increment_act_counter
//!   - return_with_apply_to_all_false_and_full_per_row_succeeds
//!
//! Plan 06 (ACT-13 / CR-02..04) gap-closure additions:
//!   - return_twice_same_device_rejected (CR-02 status guard)
//!   - return_with_duplicate_act_item_id_rejected (CR-03 dedup)
//!   - return_with_duplicate_device_id_rejected (CR-03 dedup)
//!   - return_quantity_exceeds_handover_rejected (CR-04 quantity bound)
//!
//! Каждый тест wrapped в `tokio::time::timeout(30s)` (S-6).

use std::sync::Arc;
use std::time::Duration;

use rusqlite::params;
use trackly_app::dto::act::{ActCreateDto, ActItemNewDto, ActReturnDto, ActReturnItemDto};
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

async fn seed_devices(
    writer: &Arc<trackly_infra::db::writer_worker::WriterHandle>,
    count: usize,
) -> Vec<i64> {
    let names: Vec<String> = (0..count)
        .map(|i| format!("ReturnTestDevice {i}"))
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

async fn create_handover(svc: &ActService, device_ids: &[i64]) -> trackly_app::dto::act::ActDto {
    let payload = ActCreateDto {
        number_override: None,
        giver_name: "Иванов И.И.".into(),
        receiver_name: "Петров П.П.".into(),
        location_id: None,
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
    };
    svc.create(payload).await.expect("create handover")
}

// ---------------------------------------------------------------------------
// Test 1: partial return keeps handover active
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn partial_return_keeps_handover_active() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let device_ids = seed_devices(&svc.writer, 3).await;
        let handover = create_handover(&svc, &device_ids).await;
        assert_eq!(handover.items.len(), 3);
        assert!(!handover.archived);

        // Возврат только 1 из 3 устройств.
        let first_item = &handover.items[0];
        let return_payload = ActReturnDto {
            bulk_condition: Some("Хорошее".into()),
            bulk_location_id: None,
            bulk_location_name: None,
            apply_to_all: true,
            giver_name: None,
            receiver_name: None,
            handover_date_utc: None,
            items: vec![ActReturnItemDto {
                act_item_id: first_item.id,
                device_id: first_item.device_id,
                device_ids: vec![first_item.device_id],
                quantity: 1,
                condition_override: None,
                location_id_override: None,
                location_name_override: None,
            }],
        };
        let ret = svc
            .do_return(handover.id, return_payload)
            .await
            .expect("do_return");
        assert_eq!(ret.act_type, "return");
        assert_eq!(ret.sub_number, Some(1));
        assert_eq!(ret.parent_act_id, Some(handover.id));
        // Display rule: «42в» — единственный возврат.
        assert!(
            ret.number.ends_with('в'),
            "single-return display must drop sub-suffix, got: {}",
            ret.number
        );

        // Parent остался активным.
        let parent = svc.get(handover.id).await.expect("get parent");
        assert!(!parent.archived);

        let counts = svc.counts().await.expect("counts");
        assert_eq!(counts.handover_active, 1);
        assert_eq!(counts.returns, 1);
        assert_eq!(counts.archived, 0);
    })
    .await
    .expect("partial_return budget");
}

// ---------------------------------------------------------------------------
// Test 2: full return archives handover
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn full_return_archives_handover() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let device_ids = seed_devices(&svc.writer, 2).await;
        let handover = create_handover(&svc, &device_ids).await;

        let return_payload = ActReturnDto {
            bulk_condition: Some("Хорошее".into()),
            bulk_location_id: None,
            bulk_location_name: None,
            apply_to_all: true,
            giver_name: None,
            receiver_name: None,
            handover_date_utc: None,
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
        };
        svc.do_return(handover.id, return_payload)
            .await
            .expect("do_return full");

        // Parent теперь archived.
        let parent = svc.get(handover.id).await.expect("get parent");
        assert!(parent.archived, "handover must auto-archive at 100% return");

        let counts = svc.counts().await.expect("counts");
        assert_eq!(counts.handover_active, 0);
        assert_eq!(counts.returns, 1);
        assert_eq!(counts.archived, 1);
    })
    .await
    .expect("full_return budget");
}

// ---------------------------------------------------------------------------
// Test 3: second partial return → sub_number=2 + retroactive suffix promotion
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn second_partial_return_assigns_sub_number_2_and_promotes_suffix() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let device_ids = seed_devices(&svc.writer, 3).await;
        let handover = create_handover(&svc, &device_ids).await;

        // Return №1 — одна позиция.
        let it0 = &handover.items[0];
        let ret1 = svc
            .do_return(
                handover.id,
                ActReturnDto {
                    bulk_condition: Some("Хорошее".into()),
                    bulk_location_id: None,
                    bulk_location_name: None,
                    apply_to_all: true,
                    giver_name: None,
                    receiver_name: None,
                    handover_date_utc: None,
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
            .expect("ret 1");
        assert_eq!(ret1.sub_number, Some(1));

        // Return №2 — две оставшиеся позиции.
        let ret2 = svc
            .do_return(
                handover.id,
                ActReturnDto {
                    bulk_condition: Some("Хорошее".into()),
                    bulk_location_id: None,
                    bulk_location_name: None,
                    apply_to_all: true,
                    giver_name: None,
                    receiver_name: None,
                    handover_date_utc: None,
                    items: handover.items[1..]
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
            .expect("ret 2");
        assert_eq!(ret2.sub_number, Some(2));

        // После второго возврата: sibling_count=2 → display «42в1»/«42в2».
        let ret1_refreshed = svc.get(ret1.id).await.expect("get ret1");
        let ret2_refreshed = svc.get(ret2.id).await.expect("get ret2");
        assert!(
            ret1_refreshed.number.ends_with("в1"),
            "ret1 promotes to '...в1', got: {}",
            ret1_refreshed.number
        );
        assert!(
            ret2_refreshed.number.ends_with("в2"),
            "ret2 should be '...в2', got: {}",
            ret2_refreshed.number
        );

        // Полный возврат → handover archived.
        let parent = svc.get(handover.id).await.expect("get parent");
        assert!(parent.archived);
    })
    .await
    .expect("second_partial budget");
}

// ---------------------------------------------------------------------------
// Test 4: bulk_apply_with_per_row_override (per-row побеждает condition;
// per-row None для location → bulk_location wins)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn bulk_apply_with_per_row_override() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let device_ids = seed_devices(&svc.writer, 2).await;
        let handover = create_handover(&svc, &device_ids).await;

        // Seed location_id = 5 («Куда вернуть»).
        let bulk_loc_id: i64 = svc
            .writer
            .execute(|conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;
                tx.execute(
                    "INSERT INTO locations (name, created_at_utc, updated_at_utc) \
                     VALUES ('Склад-А', ?1, ?1)",
                    params![1_700_000_000_i64],
                )
                .map_err(map_rusqlite)?;
                let id = tx.last_insert_rowid();
                tx.commit().map_err(map_rusqlite)?;
                Ok(id)
            })
            .await
            .expect("seed loc");

        let payload = ActReturnDto {
            bulk_condition: Some("Хорошее".into()),
            bulk_location_id: Some(bulk_loc_id),
            bulk_location_name: None,
            apply_to_all: true,
            giver_name: None,
            receiver_name: None,
            handover_date_utc: None,
            items: vec![
                ActReturnItemDto {
                    act_item_id: handover.items[0].id,
                    device_id: handover.items[0].device_id,
                    device_ids: vec![handover.items[0].device_id],
                    quantity: 1,
                    condition_override: None,
                    location_id_override: None,
                    location_name_override: None,
                },
                ActReturnItemDto {
                    act_item_id: handover.items[1].id,
                    device_id: handover.items[1].device_id,
                    device_ids: vec![handover.items[1].device_id],
                    quantity: 1,
                    condition_override: Some("Б/У".into()),
                    location_id_override: None,
                    location_name_override: None,
                },
            ],
        };
        svc.do_return(handover.id, payload)
            .await
            .expect("do_return");

        // Devices: A → bulk condition («Хорошее»); B → override («Б/У»). Оба
        // → bulk location.
        let device_a = device_ids[0];
        let device_b = device_ids[1];
        let readers = svc.readers.clone();
        let (a_cond, a_loc, b_cond, b_loc): (
            Option<String>,
            Option<i64>,
            Option<String>,
            Option<i64>,
        ) = tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            let (ac, al): (Option<String>, Option<i64>) = conn
                .query_row(
                    "SELECT condition, location_id FROM devices WHERE id = ?1",
                    params![device_a],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .expect("query A");
            let (bc, bl): (Option<String>, Option<i64>) = conn
                .query_row(
                    "SELECT condition, location_id FROM devices WHERE id = ?1",
                    params![device_b],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .expect("query B");
            (ac, al, bc, bl)
        })
        .await
        .expect("spawn_blocking");

        assert_eq!(a_cond.as_deref(), Some("Хорошее"));
        assert_eq!(a_loc, Some(bulk_loc_id));
        assert_eq!(b_cond.as_deref(), Some("Б/У"));
        assert_eq!(b_loc, Some(bulk_loc_id));
    })
    .await
    .expect("bulk_apply budget");
}

// ---------------------------------------------------------------------------
// Test 5: apply_to_all=false без per-row → Validation
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn return_when_apply_to_all_false_requires_per_row_values() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let device_ids = seed_devices(&svc.writer, 1).await;
        let handover = create_handover(&svc, &device_ids).await;

        let payload = ActReturnDto {
            bulk_condition: None,
            bulk_location_id: None,
            bulk_location_name: None,
            apply_to_all: false,
            giver_name: None,
            receiver_name: None,
            handover_date_utc: None,
            items: vec![ActReturnItemDto {
                act_item_id: handover.items[0].id,
                device_id: handover.items[0].device_id,
                device_ids: vec![handover.items[0].device_id],
                quantity: 1,
                condition_override: None, // ← missing
                location_id_override: None,
                location_name_override: None,
            }],
        };
        let err = svc
            .do_return(handover.id, payload)
            .await
            .expect_err("must fail with Validation");
        match err {
            AppError::Validation { field, .. } => {
                assert!(
                    field.contains("condition_override") || field.contains("location_id_override"),
                    "field should mention per-row override, got: {field}"
                );
            }
            other => panic!("expected Validation, got {other:?}"),
        }
    })
    .await
    .expect("validation budget");
}

// ---------------------------------------------------------------------------
// Test 6: concurrent two returns get sub_number 1 + 2 (single-writer guarantee)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn return_concurrent_two_returns_correct_sub_numbers() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let device_ids = seed_devices(&svc.writer, 4).await;
        let handover = create_handover(&svc, &device_ids).await;

        let svc1 = svc.clone();
        let svc2 = svc.clone();
        let act_id = handover.id;
        let it0 = handover.items[0].clone();
        let it1 = handover.items[1].clone();

        let h1 = tokio::spawn(async move {
            svc1.do_return(
                act_id,
                ActReturnDto {
                    bulk_condition: Some("Хорошее".into()),
                    bulk_location_id: None,
                    bulk_location_name: None,
                    apply_to_all: true,
                    giver_name: None,
                    receiver_name: None,
                    handover_date_utc: None,
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
        });
        let h2 = tokio::spawn(async move {
            svc2.do_return(
                act_id,
                ActReturnDto {
                    bulk_condition: Some("Хорошее".into()),
                    bulk_location_id: None,
                    bulk_location_name: None,
                    apply_to_all: true,
                    giver_name: None,
                    receiver_name: None,
                    handover_date_utc: None,
                    items: vec![ActReturnItemDto {
                        act_item_id: it1.id,
                        device_id: it1.device_id,
                        device_ids: vec![it1.device_id],
                        quantity: 1,
                        condition_override: None,
                        location_id_override: None,
                        location_name_override: None,
                    }],
                },
            )
            .await
        });

        let r1 = h1.await.expect("join1").expect("ret1");
        let r2 = h2.await.expect("join2").expect("ret2");

        let mut subs = vec![r1.sub_number.expect("sub1"), r2.sub_number.expect("sub2")];
        subs.sort();
        assert_eq!(subs, vec![1, 2], "two concurrent returns get 1 + 2");
    })
    .await
    .expect("concurrent budget");
}

// ---------------------------------------------------------------------------
// Test 7 (W-7): return does NOT increment act_number counter
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn return_does_not_increment_act_counter() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let device_ids = seed_devices(&svc.writer, 2).await;
        let handover = create_handover(&svc, &device_ids).await;

        let readers_before = svc.readers.clone();
        let before: i64 = tokio::task::spawn_blocking(move || {
            let conn = readers_before.acquire();
            conn.query_row(
                "SELECT current_value FROM counters WHERE name='act_number'",
                [],
                |r| r.get(0),
            )
            .expect("counter before")
        })
        .await
        .expect("spawn before");

        svc.do_return(
            handover.id,
            ActReturnDto {
                bulk_condition: Some("Хорошее".into()),
                bulk_location_id: None,
                bulk_location_name: None,
                apply_to_all: true,
                giver_name: None,
                receiver_name: None,
                handover_date_utc: None,
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

        let readers_after = svc.readers.clone();
        let after: i64 = tokio::task::spawn_blocking(move || {
            let conn = readers_after.acquire();
            conn.query_row(
                "SELECT current_value FROM counters WHERE name='act_number'",
                [],
                |r| r.get(0),
            )
            .expect("counter after")
        })
        .await
        .expect("spawn after");
        assert_eq!(
            before, after,
            "act_number counter MUST NOT increment on return"
        );
    })
    .await
    .expect("counter budget");
}

// ---------------------------------------------------------------------------
// Test 8 (W-8): apply_to_all=false с full per-row override → succeeds
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn return_with_apply_to_all_false_and_full_per_row_succeeds() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let device_ids = seed_devices(&svc.writer, 1).await;
        let handover = create_handover(&svc, &device_ids).await;

        // Seed location_id.
        let loc_id: i64 = svc
            .writer
            .execute(|conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;
                tx.execute(
                    "INSERT INTO locations (name, created_at_utc, updated_at_utc) \
                     VALUES ('Склад-Б', ?1, ?1)",
                    params![1_700_000_000_i64],
                )
                .map_err(map_rusqlite)?;
                let id = tx.last_insert_rowid();
                tx.commit().map_err(map_rusqlite)?;
                Ok(id)
            })
            .await
            .expect("seed loc");

        svc.do_return(
            handover.id,
            ActReturnDto {
                bulk_condition: None,
                bulk_location_id: None,
                bulk_location_name: None,
                apply_to_all: false,
                giver_name: None,
                receiver_name: None,
                handover_date_utc: None,
                items: vec![ActReturnItemDto {
                    act_item_id: handover.items[0].id,
                    device_id: handover.items[0].device_id,
                    device_ids: vec![handover.items[0].device_id],
                    quantity: 1,
                    condition_override: Some("Хорошее".into()),
                    location_id_override: Some(loc_id),
                    location_name_override: None,
                }],
            },
        )
        .await
        .expect("do_return full per-row");

        let dev_id = device_ids[0];
        let readers = svc.readers.clone();
        let (cond, loc): (Option<String>, Option<i64>) = tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            conn.query_row(
                "SELECT condition, location_id FROM devices WHERE id = ?1",
                params![dev_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .expect("query device")
        })
        .await
        .expect("spawn_blocking");
        assert_eq!(cond.as_deref(), Some("Хорошее"));
        assert_eq!(loc, Some(loc_id));
    })
    .await
    .expect("per-row budget");
}

// ---------------------------------------------------------------------------
// Plan 06 gap closure (CR-02..04 / ACT-13)
// ---------------------------------------------------------------------------

// Test 9 (CR-02): двойной возврат тех же device_id отклоняется status-guard'ом.
// Handover держит 2 устройства, первое возвращается, затем повторный return
// первого должен сорваться с Conflict «уже не в работе» (handover ещё активен,
// так что check на `parent.archived` не сработает первым).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn return_twice_same_device_rejected() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let device_ids = seed_devices(&svc.writer, 2).await;
        let handover = create_handover(&svc, &device_ids).await;
        let item = handover.items[0].clone();

        let payload = || ActReturnDto {
            bulk_condition: Some("Хорошее".into()),
            bulk_location_id: None,
            bulk_location_name: None,
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
                location_id_override: None,
                location_name_override: None,
            }],
        };

        // 1st return of device A — succeeds; handover still has device B in
        // work, so parent.archived stays false.
        svc.do_return(handover.id, payload())
            .await
            .expect("first return");

        // 2nd return on device A — device is now «на_складе», status guard
        // must reject with Conflict («уже не в работе»). The parent-archived
        // check up the stack does NOT trip because device B is still in work.
        let err = svc
            .do_return(handover.id, payload())
            .await
            .expect_err("second return must fail");
        match err {
            AppError::Conflict { reason } => {
                assert!(
                    reason.contains("уже не в работе"),
                    "Conflict reason must mention «уже не в работе», got: {reason}"
                );
                assert!(
                    reason.contains(&format!("id={}", item.device_id)),
                    "Conflict reason must include device id, got: {reason}"
                );
            }
            other => panic!("expected Conflict, got {other:?}"),
        }

        // State invariant: no second return-act was inserted (count = 1).
        let counts = svc.counts().await.expect("counts");
        assert_eq!(counts.returns, 1, "second return must not be persisted");
    })
    .await
    .expect("return_twice budget");
}

// Test 10 (CR-03): duplicate act_item_id внутри одного payload отклоняется.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn return_with_duplicate_act_item_id_rejected() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let device_ids = seed_devices(&svc.writer, 1).await;
        let handover = create_handover(&svc, &device_ids).await;
        let item = handover.items[0].clone();

        let dup_item = ActReturnItemDto {
            act_item_id: item.id,
            device_id: item.device_id,
            device_ids: vec![item.device_id],
            quantity: 1,
            condition_override: None,
            location_id_override: None,
            location_name_override: None,
        };
        let payload = ActReturnDto {
            bulk_condition: Some("Хорошее".into()),
            bulk_location_id: None,
            bulk_location_name: None,
            apply_to_all: true,
            giver_name: None,
            receiver_name: None,
            handover_date_utc: None,
            items: vec![dup_item.clone(), dup_item],
        };
        let err = svc
            .do_return(handover.id, payload)
            .await
            .expect_err("dup act_item_id must fail");
        match err {
            AppError::Validation { field, message } => {
                assert!(
                    field.contains("act_item_id"),
                    "field must mention act_item_id, got: {field}"
                );
                assert!(
                    message.contains("продублирован"),
                    "message must say 'продублирован', got: {message}"
                );
            }
            other => panic!("expected Validation, got {other:?}"),
        }

        // No return-act persisted.
        let counts = svc.counts().await.expect("counts");
        assert_eq!(counts.returns, 0);
    })
    .await
    .expect("dup act_item_id budget");
}

// Test 11 (CR-03): duplicate device_id (разные act_item_id) внутри payload отклоняется.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn return_with_duplicate_device_id_rejected() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let device_ids = seed_devices(&svc.writer, 1).await;
        let handover = create_handover(&svc, &device_ids).await;
        let item = handover.items[0].clone();

        // Two payload entries with distinct act_item_id values (one real,
        // one fake) but the same device_id — HashSet dedup must trip before
        // any SQL existence check runs.
        let payload = ActReturnDto {
            bulk_condition: Some("Хорошее".into()),
            bulk_location_id: None,
            bulk_location_name: None,
            apply_to_all: true,
            giver_name: None,
            receiver_name: None,
            handover_date_utc: None,
            items: vec![
                ActReturnItemDto {
                    act_item_id: item.id,
                    device_id: item.device_id,
                    device_ids: vec![item.device_id],
                    quantity: 1,
                    condition_override: None,
                    location_id_override: None,
                    location_name_override: None,
                },
                ActReturnItemDto {
                    act_item_id: 999_999,
                    device_id: item.device_id,
                    device_ids: vec![item.device_id],
                    quantity: 1,
                    condition_override: None,
                    location_id_override: None,
                    location_name_override: None,
                },
            ],
        };
        let err = svc
            .do_return(handover.id, payload)
            .await
            .expect_err("dup device_id must fail");
        match err {
            AppError::Validation { field, message } => {
                assert!(
                    field.contains("device_id"),
                    "field must mention device_id, got: {field}"
                );
                assert!(
                    message.contains("продублирован"),
                    "message must say 'продублирован', got: {message}"
                );
            }
            other => panic!("expected Validation, got {other:?}"),
        }

        let counts = svc.counts().await.expect("counts");
        assert_eq!(counts.returns, 0);
    })
    .await
    .expect("dup device_id budget");
}

// Test 12 (CR-04): quantity > handover_quantity отклоняется.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn return_quantity_exceeds_handover_rejected() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let device_ids = seed_devices(&svc.writer, 1).await;
        let handover = create_handover(&svc, &device_ids).await;
        let item = handover.items[0].clone();
        // create_handover seeds quantity=1 — return 100 must fail.

        // G-12 legacy path: device_ids empty → backend применяет quantity-bound
        // guard на single device_id из item. В canonical G-12 модели
        // (`device_ids` non-empty) quantity игнорируется backend-ом и double-return
        // ловится `return_twice_same_device_rejected` через status-guard.
        let payload = ActReturnDto {
            bulk_condition: Some("Хорошее".into()),
            bulk_location_id: None,
            bulk_location_name: None,
            apply_to_all: true,
            giver_name: None,
            receiver_name: None,
            handover_date_utc: None,
            items: vec![ActReturnItemDto {
                act_item_id: item.id,
                device_id: item.device_id,
                device_ids: Vec::new(),
                quantity: 100,
                condition_override: None,
                location_id_override: None,
                location_name_override: None,
            }],
        };
        let err = svc
            .do_return(handover.id, payload)
            .await
            .expect_err("quantity overflow must fail");
        match err {
            AppError::Validation { field, message } => {
                assert!(
                    field.contains("items") || field.contains("quantity"),
                    "field should mention items/quantity, got: {field}"
                );
                assert!(
                    message.contains("превышает выданное"),
                    "message must say 'превышает выданное', got: {message}"
                );
            }
            other => panic!("expected Validation, got {other:?}"),
        }

        // Roll back: no return-act persisted, device still «в_работе».
        let counts = svc.counts().await.expect("counts");
        assert_eq!(counts.returns, 0);

        let dev_id = device_ids[0];
        let readers = svc.readers.clone();
        let status_id: i64 = tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            conn.query_row(
                "SELECT status_id FROM devices WHERE id = ?1",
                params![dev_id],
                |r| r.get(0),
            )
            .expect("query status")
        })
        .await
        .expect("spawn_blocking");
        // status_id 2 is «В работе» (V001 seed; create_handover transitions
        // the seeded id=1 «На складе» → id=2 «В работе»). After the failed
        // return the device must still be in_work — full rollback on Err.
        assert_eq!(
            status_id, 2,
            "device must remain «в_работе» after failed return"
        );
    })
    .await
    .expect("quantity_overflow budget");
}
