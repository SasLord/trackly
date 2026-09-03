//! `test_writer_and_readers` — canonical fixture для интеграционных тестов
//! `crates/trackly-app/tests/concurrent_writes.rs` (и аналогичных).
//!
//! Создаёт tempfile DB с применёнными writer pragmas + всеми миграциями
//! (V001..V012), затем переоткрывает: writer-conn → `WriterHandle::spawn`,
//! плюс `ReaderPool::new(_, 4)`. Возвращает `(writer, readers, tempdir_guard)`
//! — caller обязан держать `tempdir_guard` живым до конца теста.
//!
//! Должна вызываться внутри tokio-runtime (`#[tokio::test(flavor = "multi_thread")]`),
//! потому что `WriterHandle::spawn` использует `tokio::task::spawn_blocking`.

use std::path::PathBuf;
use std::sync::Arc;

use rusqlite::Connection;
use tempfile::TempDir;

use crate::db::{migrations, pools::ReaderPool, pragmas, writer_worker::WriterHandle};

/// Полностью wired (writer + readers) fixture поверх свежего tempfile DB с
/// applied migrations.
///
/// Возвращает:
/// - `Arc<WriterHandle>` — cloneable, отправляйте writes через `.execute(closure)`.
/// - `Arc<ReaderPool>` — pool из 4 read-only connections.
/// - `TempDir` — guard, держите живым до конца теста (иначе DB файл удалится).
///
/// Каждый вызов открывает СВЕЖИЙ DB (свой tempdir) — никаких пересечений
/// между тестами.
pub fn test_writer_and_readers() -> (Arc<WriterHandle>, Arc<ReaderPool>, TempDir) {
    test_writer_and_readers_sized(4)
}

/// Same fixture as [`test_writer_and_readers`], but with a caller-chosen
/// `ReaderPool` size instead of the fixed default of 4 (Phase 40 gap-closure
/// CR-01) — needed by regression tests that must exercise a pool with EXACTLY
/// one connection slot (e.g. `get_timeline_does_not_deadlock_with_single_reader_slot`)
/// to prove a read path never takes a second connection mid-flight.
pub fn test_writer_and_readers_sized(
    pool_size: usize,
) -> (Arc<WriterHandle>, Arc<ReaderPool>, TempDir) {
    let dir = TempDir::new().expect("create tempdir for test app ctx");
    let db_path: PathBuf = dir.path().join("test.db");

    // Open writer conn, apply pragmas, run migrations.
    let mut writer_conn = Connection::open(&db_path).expect("open writer conn");
    pragmas::apply_writer_pragmas(&writer_conn).expect("apply writer pragmas");
    migrations::run(&mut writer_conn).expect("run migrations");

    // Spawn writer worker (takes ownership of the conn).
    let writer = Arc::new(WriterHandle::spawn(writer_conn));

    // Open reader pool on the same file (size = 4 per Plan 04 D-WriterChannel-01,
    // unless the caller asked for a different size).
    let readers = Arc::new(ReaderPool::new(&db_path, pool_size).expect("new reader pool"));

    (writer, readers, dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use trackly_core::error::AppError;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn writer_and_readers_share_the_same_db_file() {
        let (writer, readers, _guard) = test_writer_and_readers();

        // Write through writer.
        writer
            .execute(|c| {
                c.execute("INSERT INTO device_types (name) VALUES ('TestType')", [])
                    .map(|_| ())
                    .map_err(crate::error_conversions::map_rusqlite)
            })
            .await
            .expect("write");

        // Read through reader pool.
        let guard = readers.acquire();
        let count: i64 = guard
            .query_row(
                "SELECT COUNT(*) FROM device_types WHERE name = 'TestType'",
                [],
                |r| r.get(0),
            )
            .expect("count");
        assert_eq!(count, 1);
        drop(guard);

        // Quiet AppError import.
        let _: Result<(), AppError> = Ok(());
    }
}
