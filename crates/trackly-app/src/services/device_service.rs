//! `DeviceService` — application service for the Devices entity.
//!
//! Owns:
//! - `writer`       — single-writer handle for all DB mutations
//! - `readers`      — reader pool for all DB reads
//! - `clock`        — UTC timestamp source
//! - `repo`         — SQLite adapter (Arc so the service is cheaply Clone-able)
//! - `csv_sessions` — in-memory import session store (preview→commit TTL store)
//!
//! Plan 01 scaffold: struct + Clone + constructor only.
//! CRUD methods land in Plan 03; search/autocomplete/grouping in Plan 04;
//! CSV import/export in Plan 05.

use std::sync::Arc;

use trackly_core::primitives::clock::Clock;
use trackly_infra::db::{pools::ReaderPool, writer_worker::WriterHandle};
use trackly_infra::repos::SqliteDeviceRepository;

use crate::csv::session_store::ImportSessionStore;

/// Application service for device management.
///
/// `Arc`-wrapped fields make `Clone` O(1) — used by Tauri State and axum State.
///
/// Fields are `pub(crate)` for use by method impls in Plans 03-05.
/// `allow(dead_code)` suppresses scaffold warnings until those plans land.
#[allow(dead_code)]
#[derive(Clone)]
pub struct DeviceService {
    pub(crate) writer: Arc<WriterHandle>,
    pub(crate) readers: Arc<ReaderPool>,
    pub(crate) clock: Arc<dyn Clock + Send + Sync>,
    pub(crate) repo: Arc<SqliteDeviceRepository>,
    pub(crate) csv_sessions: Arc<ImportSessionStore>,
}

impl DeviceService {
    /// Construct a new `DeviceService`.
    ///
    /// Called from `AppCtx::build` after reader pool initialization.
    pub fn new(
        writer: Arc<WriterHandle>,
        readers: Arc<ReaderPool>,
        clock: Arc<dyn Clock + Send + Sync>,
    ) -> Self {
        Self {
            writer,
            readers,
            clock,
            repo: Arc::new(SqliteDeviceRepository),
            csv_sessions: Arc::new(ImportSessionStore::new()),
        }
    }
}
