//! ACT-01 regression: `list()`/`search()` must sort acts by
//! `handover_date_utc` (the user-entered «Когда отдали» date), not by
//! `created_at_utc` (the row-insertion timestamp).
//!
//! Phase 19 Plan 01 fixes the READ side of ACT-01 — `ActDto` now carries
//! `handover_date_utc`, and both SQL `ORDER BY` call sites in
//! `acts_sqlite.rs` (`list()` / `search_acts()`) were switched from
//! `created_at_utc` to `handover_date_utc`. This test seeds two acts whose
//! creation order is the REVERSE of their `handover_date_utc` order and
//! proves `list()` returns them ordered by `handover_date_utc DESC`.

use std::sync::Arc;
use std::time::Duration;

use trackly_app::dto::act::{
    ActCreateDto, ActDto, ActFilter, ActItemNewDto, ActReturnDto, ActReturnItemDto,
    Pagination as ActPagination,
};
use trackly_app::services::ActService;
use trackly_core::primitives::clock::Clock;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::db::writer_worker::WriterHandle;
use trackly_infra::error_conversions::map_rusqlite;
use trackly_infra::test_support::test_writer_and_readers;
use rusqlite::params;

fn make_acts_service() -> (ActService, tempfile::TempDir) {
    let (writer, readers, dir) = test_writer_and_readers();
    let clock: Arc<dyn Clock + Send + Sync> = Arc::new(SystemClock);
    let svc = ActService::new(writer, readers, clock);
    (svc, dir)
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

async fn create_handover_with_date(
    svc: &ActService,
    device_id: i64,
    giver: &str,
    handover_date_utc: i64,
) -> ActDto {
    svc.create(ActCreateDto {
        number_override: None,
        giver_name: giver.to_string(),
        receiver_name: "Получатель Т.Т.".into(),
        location_id: None,
        location_name: None,
        notes: None,
        deadline_utc: None,
        handover_date_utc: Some(handover_date_utc),
        items: vec![ActItemNewDto {
            device_id,
            device_ids: Vec::new(),
            quantity: 1,
        }],
    })
    .await
    .expect("create handover")
}

/// Act A is created FIRST but with a LATER handover_date_utc than Act B,
/// created SECOND with an EARLIER handover_date_utc. `list()` sorted DESC by
/// `handover_date_utc` must return A before B — the reverse of insertion
/// (id) order, which is what a `created_at_utc`-based sort would produce.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn list_sorts_by_handover_date_not_creation_order() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();

        let device_a = seed_device(&svc.writer, "DateSourceTestDevice-A").await;
        let device_b = seed_device(&svc.writer, "DateSourceTestDevice-B").await;

        let t = 1_700_000_000_i64;
        // A: created first (lower id), handover_date_utc = t + 1 day (LATEST).
        let act_a = create_handover_with_date(&svc, device_a, "А. Первый", t + 86_400).await;
        // B: created second (higher id), handover_date_utc = t (EARLIEST).
        let act_b = create_handover_with_date(&svc, device_b, "Б. Второй", t).await;

        assert!(
            act_b.id > act_a.id,
            "fixture invariant broken: B must be created after A (higher id)"
        );
        assert!(
            act_a.handover_date_utc > act_b.handover_date_utc,
            "fixture invariant broken: A's handover_date_utc must be later than B's"
        );

        let resp = svc
            .list(ActFilter::default(), ActPagination::default())
            .await
            .expect("list");

        let ids: Vec<i64> = resp
            .items
            .iter()
            .filter(|a| a.id == act_a.id || a.id == act_b.id)
            .map(|a| a.id)
            .collect();

        assert_eq!(
            ids,
            vec![act_a.id, act_b.id],
            "list() must sort DESC by handover_date_utc — act A (created first, later \
             handover_date_utc) must come before act B (created second, earlier \
             handover_date_utc). A created_at_utc-based sort would return them in the \
             opposite (creation/id) order."
        );
    })
    .await
    .expect("test timed out");
}

/// Same invariant via `search()` with an empty query (delegates to `list()`
/// internally, but exercises the public entry point the UI actually calls
/// for the acts list view's default/no-search state).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn search_with_empty_query_sorts_by_handover_date_not_creation_order() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();

        let device_a = seed_device(&svc.writer, "DateSourceTestDevice-C").await;
        let device_b = seed_device(&svc.writer, "DateSourceTestDevice-D").await;

        let t = 1_700_000_000_i64;
        let act_a = create_handover_with_date(&svc, device_a, "В. Третий", t + 86_400).await;
        let act_b = create_handover_with_date(&svc, device_b, "Г. Четвёртый", t).await;

        let resp = svc
            .search(String::new(), ActFilter::default(), ActPagination::default())
            .await
            .expect("search");

        let ids: Vec<i64> = resp
            .items
            .iter()
            .filter(|a| a.id == act_a.id || a.id == act_b.id)
            .map(|a| a.id)
            .collect();

        assert_eq!(
            ids,
            vec![act_a.id, act_b.id],
            "search() with empty query must also sort DESC by handover_date_utc"
        );
    })
    .await
    .expect("test timed out");
}

/// Phase 22 Plan 02 (ACT-03, D-05/D-08): `do_return` must persist the
/// payload's OWN `handover_date_utc` («Дата возврата»), never the parent's —
/// a deliberate semantic break from the pre-Phase-22 inheritance model.
///
/// Case 1: explicit `handover_date_utc` in the payload persists exactly.
/// Case 2: `handover_date_utc: None` falls back to `now` (NOT the parent's
/// date — this is the specific regression this test guards against, since
/// the pre-fix code copied `parent.handover_date_utc` verbatim).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn do_return_persists_own_date() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let device_a = seed_device(&svc.writer, "DateSourceTestDevice-ReturnA").await;
        let device_b = seed_device(&svc.writer, "DateSourceTestDevice-ReturnB").await;

        let t = 1_700_000_000_i64;
        let handover = svc
            .create(ActCreateDto {
                number_override: None,
                giver_name: "Иванов И.И.".into(),
                receiver_name: "Получатель Т.Т.".into(),
                location_id: None,
                location_name: None,
                notes: None,
                deadline_utc: None,
                handover_date_utc: Some(t),
                items: vec![
                    ActItemNewDto {
                        device_id: device_a,
                        device_ids: Vec::new(),
                        quantity: 1,
                    },
                    ActItemNewDto {
                        device_id: device_b,
                        device_ids: Vec::new(),
                        quantity: 1,
                    },
                ],
            })
            .await
            .expect("create handover with 2 devices");

        let item_a = handover
            .items
            .iter()
            .find(|it| it.device_id == device_a)
            .expect("item for device A");
        let item_b = handover
            .items
            .iter()
            .find(|it| it.device_id == device_b)
            .expect("item for device B");

        // Case 1: explicit handover_date_utc persists exactly, not parent's.
        let explicit_date = t + 999_999;
        let ret_a = svc
            .do_return(
                handover.id,
                ActReturnDto {
                    bulk_condition: Some("Хорошее".into()),
                    bulk_location_id: None,
                    bulk_location_name: None,
                    apply_to_all: true,
                    giver_name: None,
                    receiver_name: None,
                    handover_date_utc: Some(explicit_date),
                    items: vec![ActReturnItemDto {
                        act_item_id: item_a.id,
                        device_id: item_a.device_id,
                        device_ids: vec![item_a.device_id],
                        quantity: 1,
                        condition_override: None,
                        location_id_override: None,
                        location_name_override: None,
                    }],
                },
            )
            .await
            .expect("do_return A with explicit date");
        assert_eq!(
            ret_a.handover_date_utc, explicit_date,
            "return must persist its OWN submitted date, not the parent's"
        );
        assert_ne!(
            ret_a.handover_date_utc, handover.handover_date_utc,
            "return's date must differ from the parent's (proves no inheritance)"
        );

        // Case 2: handover_date_utc: None → falls back to `now`, NOT the
        // parent's date (the specific regression this test guards).
        let clock = SystemClock;
        let before_call = clock.unix_seconds();
        let ret_b = svc
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
                        act_item_id: item_b.id,
                        device_id: item_b.device_id,
                        device_ids: vec![item_b.device_id],
                        quantity: 1,
                        condition_override: None,
                        location_id_override: None,
                        location_name_override: None,
                    }],
                },
            )
            .await
            .expect("do_return B without date (back-compat fallback)");
        assert_ne!(
            ret_b.handover_date_utc, handover.handover_date_utc,
            "fallback must be `now`, NOT the parent's handover_date_utc \
             (pre-fix behavior — the regression this test guards)"
        );
        assert!(
            ret_b.handover_date_utc >= before_call,
            "fallback date should be ~`now`, got {} (before_call={})",
            ret_b.handover_date_utc,
            before_call
        );
    })
    .await
    .expect("do_return_persists_own_date budget");
}
