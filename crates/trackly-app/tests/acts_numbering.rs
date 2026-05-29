//! ACT-14 acceptance: 50 concurrent `acts_create`-style writes through the
//! single-writer pattern produce 50 unique, monotonically increasing numbers.
//!
//! This test exercises the structural guarantee directly through
//! `WriterHandle::execute` + `increment_counter_in_tx` + `insert_act_in_tx`
//! — no `ActService` involved yet (service tests live in `acts_crud.rs`).
//!
//! Per S-6: wrapped in `tokio::time::timeout(30s)` to catch Linux-CI
//! deadlocks if the writer-thread starves.

use std::sync::Arc;
use std::time::Duration;

use rusqlite::params;
use trackly_core::domain::acts::{ActRow, ActType};
use trackly_core::error::AppError;
use trackly_infra::error_conversions::map_rusqlite;
use trackly_infra::repos::acts_sqlite::{increment_counter_in_tx, SqliteActRepository};
use trackly_infra::test_support::test_writer_and_readers;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_50_creates_unique_numbers() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (writer, readers, _dir) = test_writer_and_readers();

        // Seed 50 minimal device rows directly through the writer so each
        // act can reference its own device. (We use a single shared writer
        // — there is no contention with read paths via WAL.)
        writer
            .execute(|conn| {
                let now: i64 = 1_700_000_000;
                let tx = conn.transaction().map_err(map_rusqlite)?;
                for i in 0..50 {
                    tx.execute(
                        "INSERT INTO devices \
                         (type_id, name, status_id, version, created_at_utc, updated_at_utc) \
                         VALUES (1, ?1, 1, 1, ?2, ?2)",
                        params![format!("Device {i}"), now],
                    )
                    .map_err(map_rusqlite)?;
                }
                tx.commit().map_err(map_rusqlite)?;
                Ok(())
            })
            .await
            .expect("seed devices");

        // Spawn 50 concurrent jobs, each doing:
        //   BEGIN IMMEDIATE → increment counter → INSERT acts → COMMIT
        let repo = Arc::new(SqliteActRepository);
        let mut handles = Vec::with_capacity(50);
        for i in 0..50i64 {
            let w = writer.clone();
            let repo = repo.clone();
            handles.push(tokio::spawn(async move {
                w.execute(move |conn| {
                    let tx = conn.transaction().map_err(map_rusqlite)?;
                    let number = increment_counter_in_tx(&tx, "act_number")?;
                    let row = ActRow {
                        id: 0,
                        number,
                        sub_number: None,
                        parent_act_id: None,
                        act_type: ActType::Handover,
                        giver_name: format!("Giver {i}"),
                        receiver_name: format!("Receiver {i}"),
                        location_id: None,
                        location: None,
                        notes: None,
                        deadline_utc: None,
                        archived: false,
                        created_at_utc: 1_700_000_000,
                        updated_at_utc: 1_700_000_000,
                        deleted_at_utc: None,
                        version: 1,
                        parent_number: None,
                        sibling_return_count: None,
                    };
                    repo.insert_act_in_tx(&tx, &row)?;
                    tx.commit().map_err(map_rusqlite)?;
                    Ok::<i64, AppError>(number)
                })
                .await
            }));
        }

        let mut numbers: Vec<i64> = Vec::with_capacity(50);
        for h in handles {
            let n = h.await.expect("join").expect("writer.execute");
            numbers.push(n);
        }

        numbers.sort();
        let expected: Vec<i64> = (1..=50).collect();
        assert_eq!(
            numbers, expected,
            "expected exactly 1..=50 with no duplicates, got {numbers:?}"
        );

        // Sanity: COUNT(*) of acts in DB equals 50, all numbers unique.
        let readers_clone = readers.clone();
        let (total, distinct): (i64, i64) = tokio::task::spawn_blocking(move || {
            let conn = readers_clone.acquire();
            let total: i64 = conn
                .query_row("SELECT COUNT(*) FROM acts", [], |r| r.get(0))
                .expect("count total");
            let distinct: i64 = conn
                .query_row("SELECT COUNT(DISTINCT number) FROM acts", [], |r| r.get(0))
                .expect("count distinct");
            (total, distinct)
        })
        .await
        .expect("spawn_blocking sanity");
        assert_eq!(total, 50);
        assert_eq!(distinct, 50);
    })
    .await
    .expect("concurrent_50_creates_unique_numbers exceeded 30 s budget");
}
