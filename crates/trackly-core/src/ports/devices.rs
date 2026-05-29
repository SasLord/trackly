//! `DeviceRepository` port — repository trait for the Devices entity.
//!
//! Pattern: associated `type Conn` keeps rusqlite out of trackly-core.
//! The concrete type (`rusqlite::Connection`) is specified in the adapter
//! impl in `trackly-infra::repos::devices_sqlite`.
//!
//! Method signatures follow D-Repo-01 (CONTEXT.md Phase 2).
//! Implementations land in Plan 03 (CRUD) and Plan 04 (search/autocomplete/grouping).

use crate::domain::devices::{
    AutocompleteField, DeviceFilter, DeviceGroupRow, DeviceNew, DevicePatch, DeviceRow, Pagination,
};
use crate::error::AppError;

/// Repository port for devices. Implemented by `SqliteDeviceRepository` in trackly-infra.
///
/// `type Conn` is the connection type — kept generic so that trackly-core
/// does not take a hard dependency on rusqlite.
pub trait DeviceRepository {
    /// The connection type provided by the adapter (e.g. `rusqlite::Connection`).
    type Conn;

    /// Create a new device. Returns the new device's `id`.
    fn create(&self, conn: &mut Self::Conn, new: &DeviceNew, now_utc: i64)
        -> Result<i64, AppError>;

    /// Get a single device by ID. Returns `AppError::NotFound` if absent or soft-deleted.
    fn get(&self, conn: &Self::Conn, id: i64) -> Result<DeviceRow, AppError>;

    /// List devices with optional filter and pagination. Returns (rows, total_count).
    fn list(
        &self,
        conn: &Self::Conn,
        filter: &DeviceFilter,
        page: &Pagination,
    ) -> Result<(Vec<DeviceRow>, u64), AppError>;

    /// Apply a partial update with optimistic-lock check via `version`.
    fn update(
        &self,
        conn: &mut Self::Conn,
        id: i64,
        version: i64,
        patch: &DevicePatch,
        now_utc: i64,
    ) -> Result<DeviceRow, AppError>;

    /// Soft-delete a device (sets `deleted_at_utc`). Optimistic-lock via `version`.
    fn delete_soft(
        &self,
        conn: &mut Self::Conn,
        id: i64,
        version: i64,
        now_utc: i64,
    ) -> Result<(), AppError>;

    /// Full-text search using FTS5. Returns (matching rows, total count).
    fn search_fts(
        &self,
        conn: &Self::Conn,
        fts_query: &str,
        page: &Pagination,
    ) -> Result<(Vec<DeviceRow>, u64), AppError>;

    /// Per-field autocomplete: DISTINCT values of `field` matching `prefix`.
    /// `ctx_name`: if provided, restricts to devices with that `name` (D-AutocompleteEndpoint-01).
    /// `ctx_status_id`: if provided, restricts to devices with that `status_id`.
    /// Both filters are ANDed when both are present.
    /// `field` is a whitelisted enum — prevents SQL injection (T-02-04-02).
    fn autocomplete(
        &self,
        conn: &Self::Conn,
        field: AutocompleteField,
        prefix: &str,
        ctx_name: Option<&str>,
        ctx_status_id: Option<i64>,
    ) -> Result<Vec<String>, AppError>;

    /// List grouped non-unique devices (D-Group-01). Returns groups with repr + ids + count.
    fn list_grouped(
        &self,
        conn: &Self::Conn,
        filter: &DeviceFilter,
        page: &Pagination,
    ) -> Result<Vec<DeviceGroupRow>, AppError>;

    /// Count active (non-deleted) devices per status_id.
    /// Returns Vec<(status_id, count)>.
    fn count_by_status(&self, conn: &Self::Conn) -> Result<Vec<(i64, u64)>, AppError>;

    /// Fetch multiple devices by ID list (DEV-11 expand). Cap: 1000 IDs.
    fn list_by_ids(&self, conn: &Self::Conn, ids: &[i64]) -> Result<Vec<DeviceRow>, AppError>;
}
