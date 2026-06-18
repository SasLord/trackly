//! `ReaderPool` — простой LIFO-пул read-only connections.
//!
//! Размер фиксирован (`size`), LAN-scale: 8 readers покрывают типичную
//! concurrent-нагрузку. Если все заняты, `acquire()` **блокируется** на
//! `Condvar` до освобождения соединения (queue-on-exhaust) — НЕ паникует.
//!
//! Исторически (Phase 1) `acquire()` паниковал при пустом пуле, причём паника
//! происходила *под* удержанным `MutexGuard`, что отравляло (`poison`) Mutex и
//! навсегда убивало пул: каждый последующий `lock()` тоже паниковал. Раздел
//! «Картриджи» (CartridgesPage `loadAll()` → `Promise.all([list, counts,
//! lowStock])` + model_list/search) штатно порождает >4 одновременных чтений и
//! детерминированно ронял пул. Теперь acquire очередует, а все `lock()`
//! poison-устойчивы (`into_inner`), так что одна паника где-либо больше не
//! убивает пул необратимо.
//!
//! Использует `std::sync::Mutex` + `std::sync::Condvar` (НЕ `tokio::sync`) —
//! `Connection` синхронный, а `acquire()` всегда вызывается внутри
//! `spawn_blocking`, поэтому парковка blocking-потока безопасна для рантайма.
//!
//! Каждое соединение открыто с `SQLITE_OPEN_READ_ONLY | SQLITE_OPEN_NO_MUTEX`:
//! - `READ_ONLY` — никаких случайных писем через reader-пути (FOUND-02 invariant).
//! - `NO_MUTEX` — единственный handle на connection, синхронизация — наш Mutex.

use rusqlite::{Connection, OpenFlags};
use std::path::Path;
use std::sync::{Condvar, Mutex};
use trackly_core::error::AppError;

use crate::db::pragmas::apply_reader_pragmas;
use crate::error_conversions::map_rusqlite;

/// Пул read-only SQLite соединений. Каждое — с применёнными reader pragmas.
pub struct ReaderPool {
    conns: Mutex<Vec<Connection>>,
    /// Сигнализируется на drop'е `ReaderHandle`; пробуждает acquirers,
    /// ожидающих свободное соединение при исчерпанном пуле.
    available: Condvar,
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
            available: Condvar::new(),
            size,
        })
    }

    /// Получить соединение из пула. Возвращает RAII-guard; на drop'е
    /// connection возвращается в пул и будит одного ожидающего acquirer'а.
    ///
    /// **Блокирует** вызывающий поток, если пул пуст, до освобождения
    /// соединения (queue-on-exhaust через `Condvar`). НЕ паникует на
    /// исчерпании. `lock()` poison-устойчив: паника в другом потоке не
    /// убивает пул необратимо.
    ///
    /// Контракт: вызывается только внутри `tokio::task::spawn_blocking`,
    /// поэтому блокировка потока безопасна (не стопорит async-рантайм).
    pub fn acquire(&self) -> ReaderHandle<'_> {
        let mut conns = self
            .conns
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        loop {
            if let Some(conn) = conns.pop() {
                return ReaderHandle {
                    pool: self,
                    conn: Some(conn),
                };
            }
            // Пул исчерпан — паркуемся до notify_one() из Drop'а ReaderHandle.
            conns = self
                .available
                .wait(conns)
                .unwrap_or_else(|poisoned| poisoned.into_inner());
        }
    }

    /// Размер пула (число соединений, открытых на старте).
    pub fn size(&self) -> usize {
        self.size
    }
}

impl Drop for ReaderPool {
    /// Закрываем все reader-connections под process-global close-guard'ом.
    ///
    /// Без сериализации одновременный teardown нескольких WAL-пулов из разных
    /// потоков детерминированно вис в `sqlite3_close → unixEnterMutex`
    /// (lock-order инверсия в unix VFS SQLite 3.45.3). См.
    /// [`crate::db::close_serializer`]. В проде пул живёт от старта до
    /// shutdown'а (закрытие однократно, без contention); в тестах множество
    /// `#[tokio::test]` роняют свои ctx параллельно — guard их выстраивает в
    /// очередь.
    fn drop(&mut self) {
        let _close = crate::db::close_serializer::close_guard();
        let mut conns = self
            .conns
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        // `clear()` дропает (= sqlite3_close) каждое соединение прямо здесь,
        // пока guard удержан. drop(conns) ниже релизит наш Mutex, не connections.
        conns.clear();
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
            // Poison-устойчиво: даже если кто-то паниковал под локом, возвращаем
            // соединение в пул (into_inner), чтобы пул не «усыхал».
            let mut conns = self
                .pool
                .conns
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            conns.push(c);
            drop(conns);
            // Будим одного ожидающего acquirer'а (если есть).
            self.pool.available.notify_one();
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

    #[test]
    fn concurrent_acquirers_exceeding_size_queue_instead_of_panicking() {
        // Regression for the UAT ReaderPool panic: CartridgesPage loadAll fired
        // more concurrent reads than the pool size. Old behaviour: pop()-on-empty
        // panicked *under the held lock*, poisoning the Mutex and killing the pool
        // for the whole process. New behaviour: acquire() blocks until a handle is
        // dropped, so N >> size acquirers all complete and the pool stays healthy.
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;
        use std::thread;
        use std::time::Duration;

        let dir = seed_db();
        let path = dir.path().join("reader-pool-test.db");
        let pool = Arc::new(ReaderPool::new(&path, 2).expect("new pool"));
        let done = Arc::new(AtomicUsize::new(0));

        // 12 threads contend for a pool of 2 — 10 must queue at some point.
        let handles: Vec<_> = (0..12)
            .map(|_| {
                let p = pool.clone();
                let d = done.clone();
                thread::spawn(move || {
                    let g = p.acquire();
                    let v: String = g
                        .query_row("SELECT v FROM t WHERE id = 1", [], |r| r.get(0))
                        .expect("query");
                    assert_eq!(v, "seed");
                    // Hold briefly to force genuine contention / queueing.
                    thread::sleep(Duration::from_millis(5));
                    d.fetch_add(1, Ordering::SeqCst);
                })
            })
            .collect();
        for h in handles {
            h.join().expect("acquirer thread must not panic");
        }
        assert_eq!(done.load(Ordering::SeqCst), 12);

        // Pool is still usable afterwards (all connections returned, no poison).
        let g = pool.acquire();
        let v: String = g
            .query_row("SELECT v FROM t WHERE id = 1", [], |r| r.get(0))
            .expect("pool still healthy after contention");
        assert_eq!(v, "seed");
    }
}
