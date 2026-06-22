//! SQLite adapter for `RequestRepository` + tx-helper methods used by the
//! service layer to compose multi-step write paths inside a single transaction.
//!
//! All SQL is parameterised through `rusqlite::params![...]`. No user input is
//! ever concatenated into query strings — SQL injection is structurally impossible.

use rusqlite::{params, Connection, Transaction};
use trackly_core::domain::printers::RequestTransitionOp;
use trackly_core::domain::requests::{
    Pagination, RequestCounts, RequestFilter, RequestNew, RequestRow,
};
use trackly_core::error::AppError;
use trackly_core::ports::requests::RequestRepository;

use crate::error_conversions::map_rusqlite;

/// SQLite-backed request repository adapter (zero-sized).
#[derive(Debug, Default, Clone)]
pub struct SqliteRequestRepository;

/// SELECT with the column order expected by `map_row_request`.
///
/// Joins:
///   - `users u` for display_name (requester_name).
///   - `devices d` for printer name (for cartridge_replace requests).
///   - `locations dl` for the printer's location name (D-05, Phase 12).
///   - `request_categories rc` for category display name (D-CAT-01, free_form requests).
///
/// `category_name` (idx 18) and `printer_location` (idx 19) are appended
/// LAST, in append order — never insert mid-list, it would shift every
/// subsequent `row.get(n)` in `map_row_request`.
const SELECT_REQUESTS: &str = "
    SELECT r.id, r.request_type, r.status,
           r.requested_by_user_id, r.assigned_to_user_id,
           r.printer_device_id, r.cartridge_model_id,
           r.category_id, r.completed_cartridge_id,
           r.description, r.resolution_notes,
           u.full_name AS requester_name,
           d.name AS printer_name,
           r.created_at_utc, r.updated_at_utc, r.deleted_at_utc, r.version,
           r.ad_subtype,
           rc.name AS category_name,
           dl.name AS printer_location
      FROM requests r
      LEFT JOIN users u ON u.id = r.requested_by_user_id
      LEFT JOIN devices d ON d.id = r.printer_device_id
      LEFT JOIN locations dl ON dl.id = d.location_id
      LEFT JOIN request_categories rc ON rc.id = r.category_id
";
// Note: users table uses `full_name` column (V002), not `display_name`.
// The SELECT alias `requester_name` maps to `RequestRow.requester_name`.

/// Maps a `SELECT_REQUESTS` row into `RequestRow`.
fn map_row_request(row: &rusqlite::Row<'_>) -> rusqlite::Result<RequestRow> {
    Ok(RequestRow {
        id: row.get(0)?,
        request_type: row.get(1)?,
        status: row.get(2)?,
        requested_by_user_id: row.get(3)?,
        assigned_to_user_id: row.get(4)?,
        printer_device_id: row.get(5)?,
        cartridge_model_id: row.get(6)?,
        category_id: row.get(7)?,
        completed_cartridge_id: row.get(8)?,
        description: row.get(9)?,
        resolution_notes: row.get(10)?,
        requester_name: row.get(11)?,
        printer_name: row.get(12)?,
        created_at_utc: row.get(13)?,
        updated_at_utc: row.get(14)?,
        deleted_at_utc: row.get(15)?,
        version: row.get(16)?,
        ad_subtype: row.get(17)?,
        category_name: row.get(18)?,
        printer_location: row.get(19)?,
    })
}

impl SqliteRequestRepository {
    // -----------------------------------------------------------------------
    // Tx-helpers (NOT in trait — orchestrated by RequestService)
    // -----------------------------------------------------------------------

    /// INSERT a new request row inside a transaction.
    /// Returns the new request `id`.
    pub fn insert_in_tx(
        &self,
        tx: &Transaction<'_>,
        new: &RequestNew,
        now_utc: i64,
    ) -> Result<i64, AppError> {
        tx.execute(
            "INSERT INTO requests \
             (request_type, status, requested_by_user_id, printer_device_id, \
              cartridge_model_id, category_id, description, ad_subtype, \
              created_at_utc, updated_at_utc, version) \
             VALUES (?1, 'open', ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8, 1)",
            params![
                new.request_type,
                new.requested_by_user_id,
                new.printer_device_id,
                new.cartridge_model_id,
                new.category_id,
                new.description,
                new.ad_subtype,
                now_utc,
            ],
        )
        .map_err(map_rusqlite)?;
        Ok(tx.last_insert_rowid())
    }

    /// Apply a lifecycle transition inside a transaction.
    ///
    /// Steps:
    ///   1. Fetch current row for optimistic lock + status validation.
    ///   2. Validate the op is allowed from the current status (domain rule).
    ///   3. UPDATE requests (status, resolution_notes, assigned_to, version).
    #[allow(clippy::too_many_arguments)]
    pub fn transition_in_tx(
        &self,
        tx: &Transaction<'_>,
        request_id: i64,
        version: i64,
        op: &RequestTransitionOp,
        assigned_to: Option<i64>,
        linked_cartridge_id: Option<i64>,
        now_utc: i64,
    ) -> Result<(), AppError> {
        // Fetch current row to check optimistic lock + current status.
        let current = self.fetch_in_tx(tx, request_id)?;

        // Optimistic lock check.
        if current.version != version {
            return Err(AppError::OptimisticLockMismatch {
                entity: "request",
                id: request_id,
                expected: version,
                actual: current.version,
            });
        }

        // Domain rule: validate transition from current status.
        op.validate_from_status(&current.status)?;

        let new_status = op.target_status();
        let notes = match op {
            RequestTransitionOp::Reject { notes } => notes.as_deref(),
            RequestTransitionOp::Complete { notes, .. } => notes.as_deref(),
            RequestTransitionOp::Accept => None,
        };

        let affected = tx
            .execute(
                "UPDATE requests \
                 SET status = ?1, resolution_notes = COALESCE(?2, resolution_notes), \
                     assigned_to_user_id = COALESCE(?3, assigned_to_user_id), \
                     completed_cartridge_id = COALESCE(?4, completed_cartridge_id), \
                     updated_at_utc = ?5, version = version + 1 \
                 WHERE id = ?6 AND version = ?7 AND deleted_at_utc IS NULL",
                params![
                    new_status,
                    notes,
                    assigned_to,
                    linked_cartridge_id,
                    now_utc,
                    request_id,
                    version,
                ],
            )
            .map_err(map_rusqlite)?;

        if affected == 0 {
            // WR-03: `fetch_in_tx` above already validated existence
            // (deleted_at_utc IS NULL) AND `current.version == version`, and the
            // UPDATE's WHERE clause uses that same version. Inside this single
            // transaction the only way the UPDATE can touch 0 rows after the
            // fetch succeeded is that the row was concurrently soft-deleted
            // (deleted_at_utc became non-NULL). It is NOT a version mismatch —
            // reporting `actual: current.version + 1` would fabricate a
            // non-existent concurrent edit and send a debugger chasing a ghost.
            return Err(AppError::NotFound {
                entity: "request",
                id: request_id,
            });
        }

        Ok(())
    }

    /// Fetch a request row inside an open transaction.
    pub fn fetch_in_tx(&self, tx: &Transaction<'_>, id: i64) -> Result<RequestRow, AppError> {
        tx.query_row(
            &format!("{SELECT_REQUESTS} WHERE r.id = ?1 AND r.deleted_at_utc IS NULL"),
            params![id],
            map_row_request,
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => AppError::NotFound {
                entity: "request",
                id,
            },
            other => map_rusqlite(other),
        })
    }
}

// ---------------------------------------------------------------------------
// RequestRepository trait impl
// ---------------------------------------------------------------------------

impl RequestRepository for SqliteRequestRepository {
    type Conn = Connection;

    fn get(&self, conn: &Self::Conn, id: i64) -> Result<RequestRow, AppError> {
        conn.query_row(
            &format!("{SELECT_REQUESTS} WHERE r.id = ?1 AND r.deleted_at_utc IS NULL"),
            params![id],
            map_row_request,
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => AppError::NotFound {
                entity: "request",
                id,
            },
            other => map_rusqlite(other),
        })
    }

    fn list(
        &self,
        conn: &Self::Conn,
        filter: &RequestFilter,
        page: &Pagination,
        exclude_ad_register: bool,
    ) -> Result<(Vec<RequestRow>, u64), AppError> {
        let limit = page.limit.min(200) as i64;
        let offset = page.offset as i64;
        // REQ-06 / T-09-11: non-admin callers never see ad_register rows —
        // enforced here at the SQL level, not row-hidden client-side.
        let exclude_ad_register_i64: i64 = if exclude_ad_register { 1 } else { 0 };

        let total: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM requests r \
                 WHERE r.deleted_at_utc IS NULL \
                   AND (?1 IS NULL OR r.status = ?1) \
                   AND (?2 IS NULL OR r.request_type = ?2) \
                   AND (?3 IS NULL OR r.assigned_to_user_id = ?3) \
                   AND (?4 IS NULL OR r.requested_by_user_id = ?4) \
                   AND (?5 = 0 OR r.request_type != 'ad_register')",
                params![
                    filter.status.as_deref(),
                    filter.request_type.as_deref(),
                    filter.assigned_to_user_id,
                    filter.requested_by_user_id,
                    exclude_ad_register_i64,
                ],
                |r| r.get(0),
            )
            .map_err(map_rusqlite)?;

        let mut stmt = conn
            .prepare(&format!(
                "{SELECT_REQUESTS} \
                 WHERE r.deleted_at_utc IS NULL \
                   AND (?1 IS NULL OR r.status = ?1) \
                   AND (?2 IS NULL OR r.request_type = ?2) \
                   AND (?3 IS NULL OR r.assigned_to_user_id = ?3) \
                   AND (?4 IS NULL OR r.requested_by_user_id = ?4) \
                   AND (?5 = 0 OR r.request_type != 'ad_register') \
                 ORDER BY r.created_at_utc DESC, r.id DESC \
                 LIMIT ?6 OFFSET ?7"
            ))
            .map_err(map_rusqlite)?;

        let rows = stmt
            .query_map(
                params![
                    filter.status.as_deref(),
                    filter.request_type.as_deref(),
                    filter.assigned_to_user_id,
                    filter.requested_by_user_id,
                    exclude_ad_register_i64,
                    limit,
                    offset,
                ],
                map_row_request,
            )
            .map_err(map_rusqlite)?;

        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(map_rusqlite)?);
        }
        Ok((out, total as u64))
    }

    fn counts(
        &self,
        conn: &Self::Conn,
        requested_by_user_id: Option<i64>,
    ) -> Result<RequestCounts, AppError> {
        let all: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM requests \
                 WHERE deleted_at_utc IS NULL \
                   AND (?1 IS NULL OR requested_by_user_id = ?1)",
                params![requested_by_user_id],
                |r| r.get(0),
            )
            .map_err(map_rusqlite)?;

        let open: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM requests \
                 WHERE status = 'open' AND deleted_at_utc IS NULL \
                   AND (?1 IS NULL OR requested_by_user_id = ?1)",
                params![requested_by_user_id],
                |r| r.get(0),
            )
            .map_err(map_rusqlite)?;

        let in_progress: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM requests \
                 WHERE status = 'in_progress' AND deleted_at_utc IS NULL \
                   AND (?1 IS NULL OR requested_by_user_id = ?1)",
                params![requested_by_user_id],
                |r| r.get(0),
            )
            .map_err(map_rusqlite)?;

        let completed: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM requests \
                 WHERE status = 'completed' AND deleted_at_utc IS NULL \
                   AND (?1 IS NULL OR requested_by_user_id = ?1)",
                params![requested_by_user_id],
                |r| r.get(0),
            )
            .map_err(map_rusqlite)?;

        let rejected: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM requests \
                 WHERE status = 'rejected' AND deleted_at_utc IS NULL \
                   AND (?1 IS NULL OR requested_by_user_id = ?1)",
                params![requested_by_user_id],
                |r| r.get(0),
            )
            .map_err(map_rusqlite)?;

        Ok(RequestCounts {
            all,
            open,
            in_progress,
            completed,
            rejected,
        })
    }
}

/// One row of request history — audit_log joined with the actor's name.
///
/// `actor_name` comes from a LEFT JOIN on `users` so a deleted/system actor
/// yields `None` rather than failing the query. `notes` is carried in
/// `payload_json` (e.g. `{"notes": "..."}`) for reject/complete transitions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestHistoryRow {
    pub id: i64,
    pub action: String,
    pub actor_name: Option<String>,
    pub payload_json: Option<String>,
    pub created_at_utc: i64,
}

impl SqliteRequestRepository {
    /// Request history from audit_log (REQ-07).
    ///
    /// Returns audit entries for `entity_type = 'request'` and the given
    /// `request_id`, excluding trivial read-ops, ordered newest-first.
    /// Joins `users` to surface the actor's display name.
    pub fn get_history(
        &self,
        conn: &Connection,
        request_id: i64,
    ) -> Result<Vec<RequestHistoryRow>, AppError> {
        let mut stmt = conn
            .prepare(
                "SELECT a.id, a.action, u.full_name, a.payload_json, a.created_at_utc \
                   FROM audit_log a \
                   LEFT JOIN users u ON u.id = a.user_id \
                  WHERE a.entity_type = 'request' \
                    AND a.entity_id = ?1 \
                    AND a.action NOT IN ('list', 'get') \
                  ORDER BY a.created_at_utc DESC, a.id DESC",
            )
            .map_err(map_rusqlite)?;

        let rows = stmt
            .query_map(params![request_id], |r| {
                Ok(RequestHistoryRow {
                    id: r.get(0)?,
                    action: r.get(1)?,
                    actor_name: r.get(2)?,
                    payload_json: r.get(3)?,
                    created_at_utc: r.get(4)?,
                })
            })
            .map_err(map_rusqlite)?;

        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(map_rusqlite)?);
        }
        Ok(out)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migrations;
    use crate::db::pragmas::apply_writer_pragmas;
    use tempfile::TempDir;

    fn fresh_conn() -> (Connection, TempDir) {
        let dir = TempDir::new().expect("tempdir");
        let path = dir.path().join("request-repo-test.db");
        let mut conn = Connection::open(&path).expect("open");
        apply_writer_pragmas(&conn).expect("writer pragmas");
        migrations::run(&mut conn).expect("migrations");
        (conn, dir)
    }

    /// Seed a user row, return its id.
    fn seed_user(conn: &mut Connection, full_name: &str) -> i64 {
        let now = 1_700_000_000_i64;
        conn.execute(
            "INSERT INTO users (login, full_name, password_hash, role, created_at_utc, updated_at_utc, version) \
             VALUES (?1, ?2, 'hash', 'employee', ?3, ?3, 1)",
            params![format!("u{}", conn.last_insert_rowid() + 1), full_name, now],
        )
        .expect("insert user");
        conn.last_insert_rowid()
    }

    #[test]
    fn test_request_repo_create() {
        let (mut conn, _g) = fresh_conn();
        let user_id = seed_user(&mut conn, "Иванов Иван");
        let repo = SqliteRequestRepository;
        let now = 1_700_000_000_i64;

        let new = RequestNew {
            request_type: "free_form".to_string(),
            requested_by_user_id: user_id,
            printer_device_id: None,
            cartridge_model_id: None,
            category_id: None,
            description: Some("Нужна помощь".to_string()),
            ad_subtype: None,
        };

        let request_id = {
            let tx = conn.transaction().expect("tx");
            let id = repo.insert_in_tx(&tx, &new, now).expect("insert");
            tx.commit().expect("commit");
            id
        };

        let row = repo.get(&conn, request_id).expect("get");
        assert_eq!(row.request_type, "free_form");
        assert_eq!(row.status, "open");
        assert_eq!(row.requested_by_user_id, user_id);
        assert_eq!(row.description.as_deref(), Some("Нужна помощь"));
        // D-CAT-01: no category set → category_name is None.
        assert_eq!(row.category_name, None);

        // D-CAT-01: a request seeded with category_id = Some(3) ("Программное
        // обеспечение", V024 seed order) must read back with the joined name.
        let with_category = RequestNew {
            request_type: "free_form".to_string(),
            requested_by_user_id: user_id,
            printer_device_id: None,
            cartridge_model_id: None,
            category_id: Some(3),
            description: Some("Нужно ПО".to_string()),
            ad_subtype: None,
        };
        let with_category_id = {
            let tx = conn.transaction().expect("tx");
            let id = repo.insert_in_tx(&tx, &with_category, now).expect("insert");
            tx.commit().expect("commit");
            id
        };
        let category_row = repo.get(&conn, with_category_id).expect("get");
        assert_eq!(
            category_row.category_name,
            Some("Программное обеспечение".to_string())
        );

        // list returns both rows
        let (rows, total) = repo
            .list(
                &conn,
                &RequestFilter::default(),
                &Pagination::default(),
                false,
            )
            .expect("list");
        assert_eq!(total, 2);
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().any(|r| r.id == request_id));
        assert!(rows.iter().any(|r| r.id == with_category_id));
    }

    /// D-CAT-01: the categories-list query must expose `{ id, name }`, not a
    /// bare list of names — every row must carry a nonzero id and a non-empty
    /// name (V024 seeds 4 RU category names).
    #[test]
    fn test_request_categories_list_has_id_and_name() {
        let (conn, _g) = fresh_conn();
        let mut stmt = conn
            .prepare("SELECT id, name FROM request_categories ORDER BY name")
            .expect("prepare");
        let rows: Vec<(i64, String)> = stmt
            .query_map([], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            })
            .expect("query")
            .filter_map(|r| r.ok())
            .collect();

        assert!(!rows.is_empty(), "request_categories must be seeded");
        for (id, name) in &rows {
            assert!(*id > 0, "category id must be nonzero, got {id}");
            assert!(!name.is_empty(), "category name must not be empty");
        }
    }

    #[test]
    fn test_request_transition_lifecycle() {
        let (mut conn, _g) = fresh_conn();
        let user_id = seed_user(&mut conn, "Петров Пётр");
        let repo = SqliteRequestRepository;
        let now = 1_700_000_000_i64;

        let new = RequestNew {
            request_type: "free_form".to_string(),
            requested_by_user_id: user_id,
            printer_device_id: None,
            cartridge_model_id: None,
            category_id: None,
            description: None,
            ad_subtype: None,
        };

        let request_id = {
            let tx = conn.transaction().expect("tx");
            let id = repo.insert_in_tx(&tx, &new, now).expect("insert");
            tx.commit().expect("commit");
            id
        };

        // Accept: open → in_progress
        {
            let tx = conn.transaction().expect("tx");
            repo.transition_in_tx(
                &tx,
                request_id,
                1,
                &RequestTransitionOp::Accept,
                None,
                None,
                now + 1,
            )
            .expect("accept");
            tx.commit().expect("commit");
        }

        let row = repo.get(&conn, request_id).expect("get after accept");
        assert_eq!(row.status, "in_progress");

        // Complete from in_progress (version=2 after accept)
        {
            let tx = conn.transaction().expect("tx");
            repo.transition_in_tx(
                &tx,
                request_id,
                2, // version after accept
                &RequestTransitionOp::Complete {
                    notes: None,
                    linked_cartridge_id: None,
                },
                None,
                None,
                now + 2,
            )
            .expect("complete from in_progress should succeed");
            tx.commit().expect("commit complete");
        }

        let row2 = repo.get(&conn, request_id).expect("get after complete");
        assert_eq!(row2.status, "completed");
    }

    #[test]
    fn test_request_wrong_transition_returns_validation_error() {
        let (mut conn, _g) = fresh_conn();
        let user_id = seed_user(&mut conn, "Сидоров");
        let repo = SqliteRequestRepository;
        let now = 1_700_000_000_i64;

        let new = RequestNew {
            request_type: "free_form".to_string(),
            requested_by_user_id: user_id,
            printer_device_id: None,
            cartridge_model_id: None,
            category_id: None,
            description: None,
            ad_subtype: None,
        };

        let request_id = {
            let tx = conn.transaction().expect("tx");
            let id = repo.insert_in_tx(&tx, &new, now).expect("insert");
            tx.commit().expect("commit");
            id
        };

        // Try Complete from open → should be AppError::Validation
        let tx = conn.transaction().expect("tx");
        let err = repo
            .transition_in_tx(
                &tx,
                request_id,
                1,
                &RequestTransitionOp::Complete {
                    notes: None,
                    linked_cartridge_id: None,
                },
                None,
                None,
                now + 1,
            )
            .expect_err("complete from open should fail");
        assert!(
            matches!(err, AppError::Validation { .. }),
            "expected Validation error, got: {err:?}"
        );
    }

    #[test]
    fn test_request_get_history_returns_create_entry() {
        let (mut conn, _g) = fresh_conn();
        let user_id = seed_user(&mut conn, "Козлов");
        let repo = SqliteRequestRepository;
        let now = 1_700_000_000_i64;

        let new = RequestNew {
            request_type: "free_form".to_string(),
            requested_by_user_id: user_id,
            printer_device_id: None,
            cartridge_model_id: None,
            category_id: None,
            description: Some("Тест истории".to_string()),
            ad_subtype: None,
        };

        let request_id = {
            let tx = conn.transaction().expect("tx");
            let id = repo.insert_in_tx(&tx, &new, now).expect("insert");
            // Manually insert an audit log entry (mirrors RequestService::create).
            tx.execute(
                "INSERT INTO audit_log \
                 (entity_type, entity_id, action, user_id, before_json, after_json, \
                  payload_json, created_at_utc) \
                 VALUES ('request', ?1, 'create', ?2, NULL, NULL, NULL, ?3)",
                params![id, user_id, now],
            )
            .expect("audit insert");
            tx.commit().expect("commit");
            id
        };

        let history = repo.get_history(&conn, request_id).expect("get_history");
        assert!(!history.is_empty(), "history should have ≥1 entry");
        assert_eq!(history[0].action, "create");
        // LEFT JOIN users surfaces the actor's display name (REQ-07).
        assert_eq!(history[0].actor_name.as_deref(), Some("Козлов"));
        assert_eq!(history[0].created_at_utc, now);
        // create rows carry no notes payload.
        assert!(history[0].payload_json.is_none());
    }

    #[test]
    fn test_request_get_history_carries_notes_payload() {
        let (mut conn, _g) = fresh_conn();
        let user_id = seed_user(&mut conn, "Орлова");
        let repo = SqliteRequestRepository;
        let now = 1_700_000_000_i64;

        let new = RequestNew {
            request_type: "free_form".to_string(),
            requested_by_user_id: user_id,
            printer_device_id: None,
            cartridge_model_id: None,
            category_id: None,
            description: Some("Тест notes".to_string()),
            ad_subtype: None,
        };

        let request_id = {
            let tx = conn.transaction().expect("tx");
            let id = repo.insert_in_tx(&tx, &new, now).expect("insert");
            // Reject transition stores its reason in payload_json (mirrors
            // RequestService::transition) so History can show it (REQ-07).
            tx.execute(
                "INSERT INTO audit_log \
                 (entity_type, entity_id, action, user_id, before_json, after_json, \
                  payload_json, created_at_utc) \
                 VALUES ('request', ?1, 'reject', ?2, NULL, NULL, ?3, ?4)",
                params![id, user_id, r#"{"notes":"нет картриджа"}"#, now + 10],
            )
            .expect("audit insert");
            tx.commit().expect("commit");
            id
        };

        let history = repo.get_history(&conn, request_id).expect("get_history");
        // Newest-first: reject row is first.
        assert_eq!(history[0].action, "reject");
        assert_eq!(
            history[0].payload_json.as_deref(),
            Some(r#"{"notes":"нет картриджа"}"#)
        );
    }
}
