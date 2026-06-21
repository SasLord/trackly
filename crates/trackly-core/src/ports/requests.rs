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
    ///
    /// `exclude_ad_register` (REQ-06, T-09-11): when `true`, rows with
    /// `request_type = 'ad_register'` are excluded at the SQL level —
    /// enforced by the service for non-admin callers, never row-hidden
    /// client-side.
    fn list(
        &self,
        conn: &Self::Conn,
        filter: &RequestFilter,
        page: &Pagination,
        exclude_ad_register: bool,
    ) -> Result<(Vec<RequestRow>, u64), AppError>;

    /// Aggregate counts for the status switch-bar.
    ///
    /// `requested_by_user_id` (D-REQ-01): when `Some(id)`, counts are scoped
    /// to requests owned by that user — the Employee-scoped path. `None`
    /// means unrestricted (Admin/Manager).
    fn counts(
        &self,
        conn: &Self::Conn,
        requested_by_user_id: Option<i64>,
    ) -> Result<RequestCounts, AppError>;
}
