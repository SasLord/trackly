//! Single-writer worker (D-WriterChannel-01).
//!
//! SQLite WAL пропускает MNOГО конкурентных читателей и ОДНОГО писателя.
//! Если несколько threads/tasks пытаются писать одновременно (даже через
//! отдельные `Connection`'ы), второй и далее получают `SQLITE_BUSY`. Этот
//! модуль предотвращает busy *структурно*: все писать-запросы маршалятся
//! через bounded mpsc-канал к единственному worker'у, владеющему write-conn.
//!
//! Контракт:
//! - mpsc capacity = 256 (256 «in-flight» writes — буфера хватит на ~125 мс
//!   пиков при 2000 wps; больше — caller получит `WriteQueueBusy`).
//! - `send_timeout = 5s` — если worker сильно отстаёт, отдадим клиенту
//!   `WriteQueueBusy` вместо вечного ожидания.
//! - Worker живёт в `tokio::task::spawn_blocking` (refinery + rusqlite — синхронные).
//! - Worker не ловит panic'и из замыканий: если closure уронит worker, новые
//!   `execute` получат `Internal { source_chain: "writer worker dropped reply channel" }`.
//!   В Phase 7 можно обернуть в `catch_unwind` для авто-restart, пока — accept.

use rusqlite::Connection;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
use trackly_core::error::AppError;

use crate::error_conversions::{map_oneshot_recv, map_send_timeout};

/// Канал-замыкание. Каждое замыкание получает `&mut Connection` и сообщает
/// результат через capturing `oneshot::Sender<R>`.
type BoxedJob = Box<dyn FnOnce(&mut Connection) + Send + 'static>;

/// Дефолтная capacity для mpsc-канала писателя.
pub const DEFAULT_WRITER_CAPACITY: usize = 256;

/// Дефолтный timeout для `send_timeout`.
pub const DEFAULT_SEND_TIMEOUT: Duration = Duration::from_secs(5);

/// Клиентский handle для писатель-канала. Cloneable (внутри `Arc` через `mpsc::Sender`
/// который сам клонируем).
#[derive(Clone)]
pub struct WriterHandle {
    tx: mpsc::Sender<BoxedJob>,
    send_timeout: Duration,
}

impl WriterHandle {
    /// Запустить worker с дефолтной capacity (256). Worker fire-and-forget;
    /// graceful shutdown через drop последнего `WriterHandle` (rx закроется,
    /// worker выйдет из loop, connection drop'нется).
    pub fn spawn(conn: Connection) -> Self {
        Self::spawn_with_capacity(conn, DEFAULT_WRITER_CAPACITY)
    }

    /// Запустить worker с заданной capacity. Полезно в тестах для
    /// форсирования backpressure.
    pub fn spawn_with_capacity(mut conn: Connection, capacity: usize) -> Self {
        assert!(capacity > 0, "writer capacity must be > 0");
        let (tx, mut rx) = mpsc::channel::<BoxedJob>(capacity);
        tokio::task::spawn_blocking(move || {
            while let Some(job) = rx.blocking_recv() {
                job(&mut conn);
            }
            // rx closed -> все WriterHandle drop'нуты -> graceful shutdown.
        });
        Self {
            tx,
            send_timeout: DEFAULT_SEND_TIMEOUT,
        }
    }

    /// Test-only: переопределить `send_timeout` на handle (не влияет на capacity).
    #[doc(hidden)]
    pub fn with_send_timeout(mut self, timeout: Duration) -> Self {
        self.send_timeout = timeout;
        self
    }

    /// Отправить замыкание в worker и `.await` результат.
    ///
    /// Возвращает `AppError::WriteQueueBusy`, если очередь занята дольше
    /// `send_timeout` (5s по дефолту) или receiver закрылся (worker умер).
    /// Возвращает `AppError::Internal`, если worker уронил reply-канал
    /// (паника внутри closure).
    #[must_use = "WriterHandle::execute returns a Result; ignoring it loses errors"]
    pub async fn execute<F, R>(&self, op: F) -> Result<R, AppError>
    where
        F: FnOnce(&mut Connection) -> Result<R, AppError> + Send + 'static,
        R: Send + 'static,
    {
        let (reply_tx, reply_rx) = oneshot::channel();
        let job: BoxedJob = Box::new(move |conn| {
            // Если caller отменил ожидание (timeout сработал ранее), reply_tx.send
            // вернёт Err — это нормально, просто игнорируем.
            let _ = reply_tx.send(op(conn));
        });
        self.tx
            .send_timeout(job, self.send_timeout)
            .await
            .map_err(map_send_timeout)?;
        reply_rx.await.map_err(map_oneshot_recv)?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::pragmas::apply_writer_pragmas;
    use tempfile::TempDir;

    fn fresh_conn() -> (Connection, TempDir) {
        let dir = TempDir::new().expect("tempdir");
        let path = dir.path().join("writer-worker-test.db");
        let conn = Connection::open(&path).expect("open");
        apply_writer_pragmas(&conn).expect("writer pragmas");
        conn.execute(
            "CREATE TABLE IF NOT EXISTS k (id INTEGER PRIMARY KEY AUTOINCREMENT, v TEXT NOT NULL)",
            [],
        )
        .expect("create table");
        (conn, dir)
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn ten_sequential_executes_complete_in_order() {
        let (conn, _guard) = fresh_conn();
        let writer = WriterHandle::spawn(conn);
        for i in 0..10 {
            writer
                .execute(move |c| {
                    c.execute("INSERT INTO k (v) VALUES (?1)", [format!("job-{i}")])
                        .map(|_| ())
                        .map_err(crate::error_conversions::map_rusqlite)
                })
                .await
                .expect("execute");
        }
        let count = writer
            .execute(|c| {
                c.query_row("SELECT COUNT(*) FROM k", [], |r| r.get::<_, i64>(0))
                    .map_err(crate::error_conversions::map_rusqlite)
            })
            .await
            .expect("count");
        assert_eq!(count, 10);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn backpressure_returns_write_queue_busy_when_channel_saturated() {
        // capacity=1, send_timeout=100ms — занимаем worker одним долгим job'ом,
        // затем заполняем очередь, следующий вызов получит WriteQueueBusy.
        let (conn, _guard) = fresh_conn();
        let writer = WriterHandle::spawn_with_capacity(conn, 1)
            .with_send_timeout(Duration::from_millis(100));

        // Job-1: блокирует worker на 500ms (sleep внутри spawn_blocking).
        let w = writer.clone();
        let slow = tokio::spawn(async move {
            w.execute(|_c| {
                std::thread::sleep(Duration::from_millis(500));
                Ok::<(), AppError>(())
            })
            .await
        });

        // Дать slow-task время попасть в worker и начать sleep.
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Job-2: ляжет в очередь (capacity=1 — место есть).
        let w2 = writer.clone();
        let queued = tokio::spawn(async move { w2.execute(|_c| Ok::<(), AppError>(())).await });

        // Дать time, чтобы job-2 успел сесть в канал.
        tokio::time::sleep(Duration::from_millis(20)).await;

        // Job-3: канал полон (1 in worker + 1 in queue, capacity=1),
        // send_timeout=100ms сработает.
        let result = writer.execute(|_c| Ok::<(), AppError>(())).await;
        assert!(
            matches!(result, Err(AppError::WriteQueueBusy)),
            "expected WriteQueueBusy, got {result:?}"
        );

        // Cleanup: дождёмся завершения первых двух job'ов чтобы они не
        // упали в abort на drop runtime.
        let _ = slow.await.expect("slow joined");
        let _ = queued.await.expect("queued joined");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn worker_dies_on_panic_then_next_execute_returns_internal() {
        let (conn, _guard) = fresh_conn();
        let writer = WriterHandle::spawn(conn);
        // Замыкание паникует внутри worker'а — worker thread обвалится.
        let panicker = writer.clone();
        let h = tokio::spawn(async move {
            panicker
                .execute(|_c| -> Result<(), AppError> {
                    panic!("intentional panic for test");
                })
                .await
        });
        // Worker уронил reply channel, caller получит Internal.
        let res = h.await.expect("join");
        match res {
            Err(AppError::Internal { source_chain }) => {
                assert!(source_chain.contains("writer worker"), "got {source_chain}");
            }
            other => panic!("expected Internal, got {other:?}"),
        }
    }
}
