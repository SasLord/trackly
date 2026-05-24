//! `ReaderPool` — простой LIFO-пул read-only connections.
//!
//! Размер фиксирован (`size`), LAN-scale: 4 readers >> типичная concurrent
//! нагрузка 20 пользователей. Если все заняты, `acquire()` *панично* выходит
//! (Phase 2+ может swap'нуть на `deadpool` который очередует acquirers).
//!
//! Использует `std::sync::Mutex` (НЕ `tokio::sync::Mutex`) — `Connection`
//! синхронный, `acquire` — не async.
//!
//! Каждое соединение открыто с `SQLITE_OPEN_READ_ONLY | SQLITE_OPEN_NO_MUTEX`:
//! - `READ_ONLY` — никаких случайных писем через reader-пути (FOUND-02 invariant).
//! - `NO_MUTEX` — единственный handle на connection, синхронизация — наш Mutex.

use rusqlite::{Connection, OpenFlags};
use std::path::Path;
use std::sync::Mutex;
use trackly_core::error::AppError;

use crate::db::pragmas::apply_reader_pragmas;
use crate::error_conversions::map_rusqlite;

/// Пул read-only SQLite соединений. Каждое — с применёнными reader pragmas.
pub struct ReaderPool {
    conns: Mutex<Vec<Connection>>,
    size: usize,
}

impl ReaderPool {
    /// Открыть `size` read-only connections к `db_path` и применить
    /// reader pragmas на каждое. Возвращает `AppError`, если хоть одно
    /// соединение не открылось или pragma не применилась.
    pub fn new(db_path: &Path, size: usize) -> Result<Self, AppError> {
        assert!(size > 0, "ReaderPool size must be > 0");
        let mut conns = Vec::with_capacity(size);
        for _ in 0..size {
            let conn = Connection::open_with_flags(
                db_path,
                OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
            )
            .map_err(map_rusqlite)?;
            apply_reader_pragmas(&conn)?;
            conns.push(conn);
        }
        Ok(Self {
            conns: Mutex::new(conns),
            size,
        })
    }

    /// Получить соединение из пула. Возвращает RAII-guard; на drop'е
    /// connection возвращается в пул.
    ///
    /// **Паникует**, если пул пуст. Phase 1 принимает это (LAN-scale,
    /// 4 readers >> 20 users типичной concurrency); Phase 2+ может
    /// заменить на `deadpool` для queue-on-exhaust.
    pub fn acquire(&self) -> ReaderHandle<'_> {
        let conn = self
            .conns
            .lock()
            .expect("ReaderPool mutex poisoned")
            .pop()
            .expect("ReaderPool exhausted — bump size or audit long-running reads");
        ReaderHandle {
            pool: self,
            conn: Some(conn),
        }
    }

    /// Размер пула (число соединений, открытых на старте).
    pub fn size(&self) -> usize {
        self.size
    }
}

/// RAII-guard над одолженным reader-connection. На drop'е возвращает
/// connection в пул.
pub struct ReaderHandle<'a> {
    pool: &'a ReaderPool,
    conn: Option<Connection>,
}

impl std::ops::Deref for ReaderHandle<'_> {
    type Target = Connection;
    fn deref(&self) -> &Connection {
        self.conn
            .as_ref()
            .expect("ReaderHandle: connection already returned to pool")
    }
}

impl Drop for ReaderHandle<'_> {
    fn drop(&mut self) {
        if let Some(c) = self.conn.take() {
            // Если mutex poisoned, мы уже в plate-glass-shattering pizdets
            // mode: разумнее тихо потерять connection (просто закроется на
            // drop'е), чем second-order panic в Drop.
            if let Ok(mut conns) = self.pool.conns.lock() {
                conns.push(c);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::pragmas::apply_writer_pragmas;
    use rusqlite::Connection as RConn;
    use tempfile::TempDir;

    fn seed_db() -> TempDir {
        let dir = TempDir::new().expect("tempdir");
        let path = dir.path().join("reader-pool-test.db");
        let conn = RConn::open(&path).expect("open writer");
        apply_writer_pragmas(&conn).expect("writer pragmas");
        conn.execute(
            "CREATE TABLE t (id INTEGER PRIMARY KEY AUTOINCREMENT, v TEXT NOT NULL)",
            [],
        )
        .expect("create table");
        conn.execute("INSERT INTO t (v) VALUES ('seed')", [])
            .expect("insert seed");
        drop(conn);
        dir
    }

    #[test]
    fn new_opens_size_connections_and_acquire_works() {
        let dir = seed_db();
        let path = dir.path().join("reader-pool-test.db");
        let pool = ReaderPool::new(&path, 4).expect("new pool");
        assert_eq!(pool.size(), 4);

        // Acquire one and query.
        let guard = pool.acquire();
        let v: String = guard
            .query_row("SELECT v FROM t WHERE id = 1", [], |r| r.get(0))
            .expect("query");
        assert_eq!(v, "seed");
        drop(guard);
    }

    #[test]
    fn acquire_drops_return_connection_to_pool() {
        let dir = seed_db();
        let path = dir.path().join("reader-pool-test.db");
        let pool = ReaderPool::new(&path, 1).expect("new pool");

        // Acquire, drop, acquire again — должен сработать.
        {
            let _g = pool.acquire();
        }
        {
            let _g = pool.acquire();
        }
    }

    #[test]
    fn reader_pool_uses_read_only_flag() {
        // Confirm reader connection rejects writes (query_only=ON pragma).
        let dir = seed_db();
        let path = dir.path().join("reader-pool-test.db");
        let pool = ReaderPool::new(&path, 1).expect("new pool");
        let guard = pool.acquire();
        let err = guard
            .execute("INSERT INTO t (v) VALUES ('attempt')", [])
            .expect_err("write through reader should fail");
        // Не строгая проверка кода: достаточно того, что вернулась ошибка.
        let msg = format!("{err}");
        assert!(
            msg.contains("readonly")
                || msg.contains("read-only")
                || msg.contains("read only")
                || msg.contains("attempt to write")
                || msg.contains("query_only"),
            "expected read-only error, got: {msg}"
        );
    }

    #[test]
    fn acquire_concurrent_four_readers_all_succeed() {
        use std::sync::Arc;
        use std::thread;

        let dir = seed_db();
        let path = dir.path().join("reader-pool-test.db");
        let pool = Arc::new(ReaderPool::new(&path, 4).expect("new pool"));

        let handles: Vec<_> = (0..4)
            .map(|_| {
                let p = pool.clone();
                thread::spawn(move || {
                    let g = p.acquire();
                    let v: String = g
                        .query_row("SELECT v FROM t WHERE id = 1", [], |r| r.get(0))
                        .expect("query");
                    assert_eq!(v, "seed");
                })
            })
            .collect();
        for h in handles {
            h.join().expect("thread");
        }
    }
}
