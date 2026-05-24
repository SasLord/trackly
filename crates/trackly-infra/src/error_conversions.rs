//! Конверсии I/O-ошибок (`rusqlite::Error`, `refinery::Error`, tokio mpsc/oneshot)
//! в [`AppError`].
//!
//! Эти конверсии живут в `trackly-infra`, а не в `trackly-core`, потому что
//! `trackly-core` запрещено зависеть от `rusqlite`/`refinery`/`tokio`
//! (`crates/trackly-core/tests/no_io_deps.rs`).
//!
//! **Почему free-функции, а не `impl From<…> for AppError`?**
//! Rust orphan rule: для `impl From<rusqlite::Error> for AppError` нужно владеть
//! либо трейтом (`From` — std), либо типом (`AppError` — в `trackly-core`).
//! `trackly-infra` не владеет ни тем, ни другим, поэтому `impl From` тут
//! невозможен. Free-функции (`map_rusqlite`, `map_refinery`, `map_send_timeout`,
//! `map_oneshot_recv`) — канонический workaround. Callsites используют их
//! через `.map_err(map_rusqlite)?`.
//!
//! Mapping policy:
//! - SQLITE_BUSY / SQLITE_LOCKED → `AppError::WriteQueueBusy` (defensive —
//!   single-writer по дизайну предотвращает busy).
//! - UNIQUE / CHECK / FK violation → `AppError::Conflict { reason }`.
//! - Всё остальное `rusqlite::Error` → `AppError::Internal`.
//! - `refinery::Error` → `AppError::Internal` (миграции — конфигурация, не runtime).
//! - `mpsc::error::SendTimeoutError<T>` → `AppError::WriteQueueBusy` (и
//!   `Timeout`, и `Closed` — оба «писатель недоступен»).
//! - `oneshot::error::RecvError` → `AppError::Internal { ... "writer dropped reply" }`.

use rusqlite::ErrorCode;
use tokio::sync::{mpsc, oneshot};
use trackly_core::error::AppError;

/// Маппинг [`rusqlite::Error`] → [`AppError`].
pub fn map_rusqlite(err: rusqlite::Error) -> AppError {
    if let rusqlite::Error::SqliteFailure(code, msg) = &err {
        let reason = msg
            .clone()
            .unwrap_or_else(|| format!("sqlite error code {code:?}"));
        return match code.code {
            ErrorCode::DatabaseBusy | ErrorCode::DatabaseLocked => AppError::WriteQueueBusy,
            ErrorCode::ConstraintViolation => AppError::Conflict { reason },
            _ => AppError::Internal {
                source_chain: format!("rusqlite: {err}"),
            },
        };
    }
    AppError::Internal {
        source_chain: format!("rusqlite: {err}"),
    }
}

/// Маппинг [`refinery::Error`] → [`AppError::Internal`].
pub fn map_refinery(err: refinery::Error) -> AppError {
    AppError::Internal {
        source_chain: format!("refinery: {err}"),
    }
}

/// Маппинг `mpsc::error::SendTimeoutError<T>` → [`AppError::WriteQueueBusy`].
///
/// Маппим и `Timeout`, и `Closed` в один и тот же `WriteQueueBusy`: с точки
/// зрения caller'а оба означают «писатель недоступен сейчас, попробуй позже».
/// Подробности (timeout vs panic) попадают в trace-log внутри
/// `WriterHandle::execute`.
pub fn map_send_timeout<T>(_err: mpsc::error::SendTimeoutError<T>) -> AppError {
    AppError::WriteQueueBusy
}

/// Маппинг `oneshot::error::RecvError` → [`AppError::Internal`].
///
/// Получено, когда worker уронил reply channel (паника внутри замыкания,
/// преждевременный shutdown). Это не должно происходить в норме; если
/// случилось — это либо bug в worker'е, либо graceful shutdown во время
/// активного запроса.
pub fn map_oneshot_recv(_err: oneshot::error::RecvError) -> AppError {
    AppError::Internal {
        source_chain: "writer worker dropped reply channel".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rusqlite_query_returned_no_rows_maps_to_internal() {
        let err = rusqlite::Error::QueryReturnedNoRows;
        let app = map_rusqlite(err);
        match app {
            AppError::Internal { source_chain } => {
                assert!(
                    source_chain.contains("rusqlite"),
                    "source_chain should mention rusqlite, got: {source_chain}"
                );
            }
            other => panic!("expected Internal, got {other:?}"),
        }
    }

    #[test]
    fn rusqlite_busy_maps_to_write_queue_busy() {
        let err = rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error {
                code: ErrorCode::DatabaseBusy,
                extended_code: 5, // SQLITE_BUSY
            },
            Some("database is locked".to_string()),
        );
        let app = map_rusqlite(err);
        assert!(matches!(app, AppError::WriteQueueBusy), "got {app:?}");
    }

    #[test]
    fn rusqlite_constraint_violation_maps_to_conflict() {
        let err = rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error {
                code: ErrorCode::ConstraintViolation,
                extended_code: 1555, // SQLITE_CONSTRAINT_PRIMARYKEY
            },
            Some("UNIQUE constraint failed: devices.serial".to_string()),
        );
        let app = map_rusqlite(err);
        match app {
            AppError::Conflict { reason } => {
                assert!(reason.contains("UNIQUE"), "got {reason}");
            }
            other => panic!("expected Conflict, got {other:?}"),
        }
    }

    #[test]
    fn oneshot_recv_error_maps_to_internal() {
        let (tx, rx) = oneshot::channel::<()>();
        drop(tx);
        let err = futures_or_block_on(rx);
        let app = map_oneshot_recv(err);
        match app {
            AppError::Internal { source_chain } => {
                assert!(source_chain.contains("writer worker"), "got {source_chain}");
            }
            other => panic!("expected Internal, got {other:?}"),
        }
    }

    #[test]
    fn send_timeout_maps_to_write_queue_busy() {
        // Construct a SendTimeoutError::Timeout(payload) manually.
        let timeout_err: mpsc::error::SendTimeoutError<i32> =
            mpsc::error::SendTimeoutError::Timeout(42);
        let app = map_send_timeout(timeout_err);
        assert!(matches!(app, AppError::WriteQueueBusy));

        let closed_err: mpsc::error::SendTimeoutError<i32> =
            mpsc::error::SendTimeoutError::Closed(7);
        let app = map_send_timeout(closed_err);
        assert!(matches!(app, AppError::WriteQueueBusy));
    }

    /// Helper: блокирующий `await` без полноценного tokio-runtime
    /// (для теста oneshot::RecvError достаточно).
    fn futures_or_block_on(rx: oneshot::Receiver<()>) -> oneshot::error::RecvError {
        // tokio current-thread runtime — самый дешёвый способ.
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("build current-thread runtime");
        rt.block_on(async move { rx.await.expect_err("sender dropped") })
    }
}
