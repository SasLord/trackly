//! `RusqliteSessionStore` — кастомная реализация `tower_sessions::SessionStore`
//! поверх единого писателя + пула читателей (D-Session-01).
//!
//! Таблица `sessions` (V010):
//!   id BLOB PRIMARY KEY — session ID как little-endian i128 bytes
//!   data BLOB            — MessagePack (rmp_serde) сериализованный Record
//!   expiry_date INTEGER  — unix timestamp (секунды) истечения сессии
//!
//! Сессии переживают рестарт приложения (D-Session-01) — они в SQLite, не in-memory.
//!
//! **Потокобезопасность:** Все операции bridge через `writer.execute()` (writes)
//! или `spawn_blocking` + `readers.acquire()` (reads) — как в DeviceService.

use std::sync::Arc;

use async_trait::async_trait;
use rusqlite::OptionalExtension;
use time::OffsetDateTime;
use tower_sessions::session::{Id, Record};
use tower_sessions::session_store;
use tower_sessions::SessionStore;

use trackly_core::error::AppError;
use trackly_infra::db::{pools::ReaderPool, writer_worker::WriterHandle};
use trackly_infra::error_conversions::map_rusqlite;

/// Хранилище сессий на основе SQLite (V010 таблица `sessions`).
///
/// Impl `SessionStore` для tower-sessions 0.15.
#[derive(Clone)]
pub struct RusqliteSessionStore {
    pub(crate) writer: Arc<WriterHandle>,
    pub(crate) readers: Arc<ReaderPool>,
}

// Manual Debug impl: WriterHandle and ReaderPool don't impl Debug.
impl std::fmt::Debug for RusqliteSessionStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RusqliteSessionStore").finish_non_exhaustive()
    }
}

impl RusqliteSessionStore {
    /// Создать новый `RusqliteSessionStore`.
    pub fn new(writer: Arc<WriterHandle>, readers: Arc<ReaderPool>) -> Self {
        Self { writer, readers }
    }

    /// Удалить все истёкшие сессии из БД.
    ///
    /// Вызывается один раз при старте сервера (T-05-08 mitigation, Pitfall 5).
    /// Не является фоновой задачей — просто разовая очистка.
    pub async fn background_cleanup(&self) -> Result<(), AppError> {
        self.writer
            .execute(|conn| {
                conn.execute(
                    "DELETE FROM sessions WHERE expiry_date < unixepoch()",
                    [],
                )
                .map_err(map_rusqlite)?;
                Ok(())
            })
            .await
    }
}

#[async_trait]
impl SessionStore for RusqliteSessionStore {
    /// Создать новую сессию.
    ///
    /// Использует INSERT OR IGNORE (T-05-04: коллизии ID обрабатываются через
    /// tower-sessions collision-safe Id generation — 128-bit OsRng).
    async fn create(&self, record: &mut Record) -> session_store::Result<()> {
        let id_bytes = record.id.0.to_le_bytes().to_vec();
        let data = rmp_serde::to_vec(record)
            .map_err(|e| session_store::Error::Encode(e.to_string()))?;
        let expiry_ts = record.expiry_date.unix_timestamp();

        self.writer
            .execute(move |conn| {
                conn.execute(
                    "INSERT OR IGNORE INTO sessions (id, data, expiry_date) VALUES (?1, ?2, ?3)",
                    rusqlite::params![id_bytes, data, expiry_ts],
                )
                .map_err(|e| AppError::Internal {
                    source_chain: format!("session create: {e}"),
                })?;
                Ok(())
            })
            .await
            .map_err(|e| session_store::Error::Backend(e.to_string()))
    }

    /// Сохранить (update) существующую сессию (INSERT OR REPLACE).
    async fn save(&self, record: &Record) -> session_store::Result<()> {
        let id_bytes = record.id.0.to_le_bytes().to_vec();
        let data = rmp_serde::to_vec(record)
            .map_err(|e| session_store::Error::Encode(e.to_string()))?;
        let expiry_ts = record.expiry_date.unix_timestamp();

        self.writer
            .execute(move |conn| {
                conn.execute(
                    "INSERT OR REPLACE INTO sessions (id, data, expiry_date) VALUES (?1, ?2, ?3)",
                    rusqlite::params![id_bytes, data, expiry_ts],
                )
                .map_err(|e| AppError::Internal {
                    source_chain: format!("session save: {e}"),
                })?;
                Ok(())
            })
            .await
            .map_err(|e| session_store::Error::Backend(e.to_string()))
    }

    /// Загрузить сессию по ID.
    ///
    /// Возвращает `None` если сессия не найдена или истекла.
    async fn load(&self, session_id: &Id) -> session_store::Result<Option<Record>> {
        let id_bytes = session_id.0.to_le_bytes().to_vec();
        let now = OffsetDateTime::now_utc().unix_timestamp();
        let readers = self.readers.clone();

        let data_opt: Option<Vec<u8>> = tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            conn.query_row(
                "SELECT data FROM sessions WHERE id = ?1 AND expiry_date > ?2",
                rusqlite::params![id_bytes, now],
                |row| row.get::<_, Vec<u8>>(0),
            )
            .optional()
            .map_err(map_rusqlite)
        })
        .await
        .map_err(|e| session_store::Error::Backend(format!("spawn_blocking: {e}")))?
        .map_err(|e| session_store::Error::Backend(e.to_string()))?;

        match data_opt {
            None => Ok(None),
            Some(bytes) => {
                match rmp_serde::from_slice::<Record>(&bytes) {
                    Ok(record) => Ok(Some(record)),
                    // WR-05: a corrupt / version-skewed session row must not 500
                    // every request carrying that cookie. Treat it as "no session":
                    // log, best-effort delete the row, and return Ok(None) so the
                    // client is simply re-authenticated.
                    Err(e) => {
                        tracing::warn!(
                            "session decode failed, dropping session row: {e}"
                        );
                        let _ = self.delete(session_id).await;
                        Ok(None)
                    }
                }
            }
        }
    }

    /// Удалить сессию по ID (logout / flush).
    async fn delete(&self, session_id: &Id) -> session_store::Result<()> {
        let id_bytes = session_id.0.to_le_bytes().to_vec();

        self.writer
            .execute(move |conn| {
                conn.execute(
                    "DELETE FROM sessions WHERE id = ?1",
                    rusqlite::params![id_bytes],
                )
                .map_err(|e| AppError::Internal {
                    source_chain: format!("session delete: {e}"),
                })?;
                Ok(())
            })
            .await
            .map_err(|e| session_store::Error::Backend(e.to_string()))
    }
}
