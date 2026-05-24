//! Phase 1 success criterion #2: 50 concurrent writes, no SQLITE_BUSY, no WriteQueueBusy.
//!
//! Спускаем по 25 «Tauri-style» + 25 «axum-style» writer.execute() задач
//! одновременно через один `WriterHandle`. Single-writer pattern должен
//! предотвратить SQLite-busy *структурно* — все вставки реально сериализованы
//! внутри одного worker'а, поэтому SQLITE_BUSY невозможен.

use std::sync::Arc;

use trackly_core::error::AppError;
use trackly_infra::error_conversions::map_rusqlite;
use trackly_infra::test_support::test_writer_and_readers;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn fifty_concurrent_writes_complete_without_sqlite_busy() {
    let (writer, readers, _guard) = test_writer_and_readers();

    // Pre-create test sink.
    writer
        .execute(|c| {
            c.execute(
                "CREATE TABLE concurrent_test (\
                   id INTEGER PRIMARY KEY AUTOINCREMENT, \
                   payload TEXT NOT NULL\
                 )",
                [],
            )
            .map(|_| ())
            .map_err(map_rusqlite)
        })
        .await
        .expect("create concurrent_test table");

    // Spawn 50 concurrent writes: 25 tauri-style + 25 axum-style payload labels.
    let mut handles = Vec::with_capacity(50);
    for i in 0..50 {
        let w: Arc<_> = writer.clone();
        let label = if i < 25 { "tauri" } else { "axum" };
        let payload = format!("{label}:job-{i}");
        handles.push(tokio::spawn(async move {
            w.execute(move |c| {
                c.execute(
                    "INSERT INTO concurrent_test (payload) VALUES (?1)",
                    [payload],
                )
                .map(|_| ())
                .map_err(map_rusqlite)
            })
            .await
        }));
    }

    let mut errors: Vec<AppError> = Vec::new();
    for (i, h) in handles.into_iter().enumerate() {
        let result = h.await.expect("join task");
        if let Err(e) = result {
            errors.push(e);
            eprintln!("task {i} failed");
        }
    }

    assert!(
        errors.is_empty(),
        "expected 0 errors across 50 concurrent writes, got {}: first = {:?}",
        errors.len(),
        errors.first()
    );

    // Verify count via reader pool. tokio::task::spawn_blocking because
    // ReaderPool::acquire + query are sync.
    let readers_clone = readers.clone();
    let count: i64 = tokio::task::spawn_blocking(move || {
        let conn = readers_clone.acquire();
        conn.query_row("SELECT COUNT(*) FROM concurrent_test", [], |r| {
            r.get::<_, i64>(0)
        })
        .expect("count query")
    })
    .await
    .expect("blocking task");
    assert_eq!(count, 50, "expected 50 rows after 50 writes, got {count}");

    // Sanity: ensure both labels are present.
    let readers_clone = readers.clone();
    let (tauri_count, axum_count) = tokio::task::spawn_blocking(move || {
        let conn = readers_clone.acquire();
        let t: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM concurrent_test WHERE payload LIKE 'tauri:%'",
                [],
                |r| r.get(0),
            )
            .expect("count tauri");
        let a: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM concurrent_test WHERE payload LIKE 'axum:%'",
                [],
                |r| r.get(0),
            )
            .expect("count axum");
        (t, a)
    })
    .await
    .expect("blocking task");
    assert_eq!(tauri_count, 25, "expected 25 tauri-labelled rows");
    assert_eq!(axum_count, 25, "expected 25 axum-labelled rows");
}
