//! Integration tests for D-07 (Phase 22, ACT-03): compute-on-read
//! `ActDto.archived_at_utc` — «Дата архивации», `MAX(handover_date_utc)`
//! among a parent act's non-deleted `act_type='return'` children, populated
//! ONLY when `archived == true`. No stored column, no migration.
//!
//! Helper scaffolding copied verbatim from `acts_returns.rs`
//! (`make_acts_service`/`seed_devices`/`create_handover`).
//!
//! Каждый тест wrapped в `tokio::time::timeout(30s)` (S-6).

use std::sync::Arc;
use std::time::Duration;

use rusqlite::params;
use trackly_app::dto::act::{ActCreateDto, ActItemNewDto, ActReturnDto, ActReturnItemDto};
use trackly_app::services::ActService;
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
        .map(|i| format!("ArchivedAtTestDevice {i}"))
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
        place_id: None,
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
// Test 1: fully-returned parent → archived_at_utc == Some(return's own date)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn archived_at_utc_present_for_fully_returned_parent() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let device_ids = seed_devices(&svc.writer, 2).await;
        let handover = create_handover(&svc, &device_ids).await;

        // Return BOTH devices in a single call.
        let return_payload = ActReturnDto {
            bulk_condition: Some("Хорошее".into()),
            bulk_place_id: None,
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
                    place_id_override: None,
                })
                .collect(),
            giver_name: None,
            receiver_name: None,
            handover_date_utc: None,
        };
        let ret = svc
            .do_return(handover.id, return_payload)
            .await
            .expect("do_return full");

        let parent = svc.get(handover.id).await.expect("get parent");
        assert!(parent.archived, "fully-returned parent must be archived");
        // Compare against the RETURN act's own handover_date_utc, not a
        // hardcoded timestamp — this assertion holds regardless of whether
        // do_return's write-site still copies the parent's date or later
        // consumes the payload's own date (Plan 22-02), no forward
        // dependency on that plan.
        assert_eq!(
            parent.archived_at_utc,
            Some(ret.handover_date_utc),
            "archived_at_utc must equal MAX(handover_date_utc) over non-deleted return children"
        );
    })
    .await
    .expect("fully_returned budget");
}

// ---------------------------------------------------------------------------
// Test 2: partially-returned parent → archived_at_utc absent
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn archived_at_utc_absent_for_partially_returned_parent() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let device_ids = seed_devices(&svc.writer, 2).await;
        let handover = create_handover(&svc, &device_ids).await;

        // Return only 1 of 2 devices.
        let first_item = &handover.items[0];
        let return_payload = ActReturnDto {
            bulk_condition: Some("Хорошее".into()),
            bulk_place_id: None,
            apply_to_all: true,
            items: vec![ActReturnItemDto {
                act_item_id: first_item.id,
                device_id: first_item.device_id,
                device_ids: vec![first_item.device_id],
                quantity: 1,
                condition_override: None,
                place_id_override: None,
            }],
            giver_name: None,
            receiver_name: None,
            handover_date_utc: None,
        };
        svc.do_return(handover.id, return_payload)
            .await
            .expect("do_return partial");

        let parent = svc.get(handover.id).await.expect("get parent");
        assert!(
            !parent.archived,
            "partially-returned parent must not be archived"
        );
        assert!(
            parent.archived_at_utc.is_none(),
            "archived_at_utc must be None when archived == false"
        );
    })
    .await
    .expect("partially_returned budget");
}
