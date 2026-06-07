//! `CartridgeRepository` port — repository trait for the Cartridges entity.
//!
//! Pattern: associated `type Conn` keeps rusqlite out of trackly-core.
//! The concrete type (`rusqlite::Connection`) is specified in the adapter
//! impl in `trackly_infra::repos::cartridges_sqlite`.
//!
//! Write methods that participate in larger transactions (create, transition,
//! delete with audit) are NOT part of this trait — they live as `*_in_tx`
//! helpers on `SqliteCartridgeRepository` and are orchestrated by the service
//! layer (`CartridgeService`) inside a single `WriterHandle::execute` closure.
//!
//! This trait covers the read-only port surface plus the simple soft-delete
//! and counter peek operations.

use crate::domain::cartridges::{CartridgeCounts, CartridgeFilter, CartridgeRow, Pagination};
use crate::error::AppError;

/// Repository port for cartridges. Implemented by `SqliteCartridgeRepository` in trackly-infra.
pub trait CartridgeRepository {
    /// The connection type provided by the adapter (e.g. `rusqlite::Connection`).
    type Conn;

    /// Fetch a single cartridge by ID (including JOIN'ed model and status names).
    /// Returns `AppError::NotFound` if absent or soft-deleted.
    fn get(&self, conn: &Self::Conn, id: i64) -> Result<CartridgeRow, AppError>;

    /// Paginated list of cartridges matching `filter`. Returns `(rows, total)`.
    fn list(
        &self,
        conn: &Self::Conn,
        filter: &CartridgeFilter,
        page: &Pagination,
    ) -> Result<(Vec<CartridgeRow>, u64), AppError>;

    /// Aggregate counts for the status switch-bar (Все/На складе/В работе/На заправке/Списано).
    fn counts(&self, conn: &Self::Conn) -> Result<CartridgeCounts, AppError>;

    /// Read-only peek at the next auto-code sequence value — `current_value + 1` of
    /// `counters.cartridge_seq`. Does NOT increment. Used for UI preview only.
    fn peek_next_code(&self, conn: &Self::Conn) -> Result<i64, AppError>;

    /// Soft-delete a cartridge with optimistic-lock via `version`.
    /// Sets `deleted_at_utc` and increments `version`. The service layer
    /// wraps this with audit-log inserts.
    fn delete_soft(
        &self,
        conn: &mut Self::Conn,
        id: i64,
        version: i64,
        now_utc: i64,
    ) -> Result<(), AppError>;
}
