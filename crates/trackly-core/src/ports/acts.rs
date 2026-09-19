//! `ActRepository` port — repository trait for the Acts entity.
//!
//! Pattern: associated `type Conn` keeps rusqlite out of trackly-core.
//! The concrete type (`rusqlite::Connection`) is specified in the adapter
//! impl in `trackly_infra::repos::acts_sqlite`.
//!
//! Write methods that participate in larger transactions (create, return,
//! delete with undo) are NOT part of this trait — they live as `*_in_tx`
//! helpers on `SqliteActRepository` and are orchestrated by the service
//! layer (`ActService`) inside a single `WriterHandle::execute` closure.
//!
//! This trait covers the read-only port surface plus the simple soft-delete
//! operation. Phase 40.2 (NUM-13): the old `peek_next_number`
//! counter-peek method was removed — act numbers are now free TEXT values
//! resolved via `NumberTemplateService`, not a `counters` row.

use crate::domain::acts::{ActCounts, ActFilter, ActRow, Pagination};
use crate::error::AppError;

/// Repository port for acts. Implemented by `SqliteActRepository` in trackly-infra.
pub trait ActRepository {
    /// The connection type provided by the adapter (e.g. `rusqlite::Connection`).
    type Conn;

    /// Fetch a single act by ID, including parent_number and sibling_return_count
    /// for the display-rule. Returns `AppError::NotFound` if absent or soft-deleted.
    fn get(&self, conn: &Self::Conn, id: i64) -> Result<ActRow, AppError>;

    /// Paginated list of acts matching `filter`. Returns `(rows, total)`.
    fn list(
        &self,
        conn: &Self::Conn,
        filter: &ActFilter,
        page: &Pagination,
    ) -> Result<(Vec<ActRow>, u64), AppError>;

    /// Soft-delete an act with optimistic-lock via `version`.
    /// Hard-deletes the associated `act_items` rows in the same transaction
    /// (FK CASCADE does not fire on soft-delete). The service layer wraps
    /// this with audit-log inserts; this trait method just touches `acts`
    /// and `act_items`.
    fn delete_soft(
        &self,
        conn: &mut Self::Conn,
        id: i64,
        version: i64,
        now_utc: i64,
    ) -> Result<(), AppError>;

    /// Counts for the switch-bar tabs (Акты / Возвраты / Архив).
    fn counts(&self, conn: &Self::Conn) -> Result<ActCounts, AppError>;
}
