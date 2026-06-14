//! `RequestRepository` port — repository trait for the Requests entity.
//!
//! Pattern: associated `type Conn` keeps rusqlite out of trackly-core.
//! Write methods (create, transition) are NOT in this trait — they live as
//! `*_in_tx` helpers on `SqliteRequestRepository` in trackly-infra.

use crate::domain::requests::{Pagination, RequestCounts, RequestFilter, RequestRow};
use crate::error::AppError;

/// Repository port for requests. Implemented by `SqliteRequestRepository` in trackly-infra.
pub trait RequestRepository {
    /// The connection type provided by the adapter (e.g. `rusqlite::Connection`).
    type Conn;

    /// Fetch a single request by ID (with joined display columns).
    /// Returns `AppError::NotFound` if absent or soft-deleted.
    fn get(&self, conn: &Self::Conn, id: i64) -> Result<RequestRow, AppError>;

    /// Paginated list of requests matching `filter`. Returns `(rows, total)`.
    fn list(
        &self,
        conn: &Self::Conn,
        filter: &RequestFilter,
        page: &Pagination,
    ) -> Result<(Vec<RequestRow>, u64), AppError>;

    /// Aggregate counts for the status switch-bar.
    fn counts(&self, conn: &Self::Conn) -> Result<RequestCounts, AppError>;
}
