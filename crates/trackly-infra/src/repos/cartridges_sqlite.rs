//! SQLite adapter for `CartridgeRepository` + tx-helper methods used by the
//! service layer to compose multi-step write paths (create, transition, delete)
//! inside a single transaction.
//!
//! All SQL is parameterised through `rusqlite::params![...]`. No user input is
//! ever concatenated into query strings — SQL injection is structurally impossible.
//!
//! The `*_in_tx` helpers expect the caller to own a `rusqlite::Transaction`
//! (started via `conn.transaction()` inside a `WriterHandle::execute` closure —
//! see D-WriterChannel-01 and D-Counter-Acts-01).

use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde_json::json;
use trackly_core::domain::cartridges::{
    CartridgeCounts, CartridgeFilter, CartridgeModelNew, CartridgeModelRow, CartridgeRow,
    CartridgeTransitionOp, LowStockItem, Pagination,
};
use trackly_core::error::AppError;
use trackly_core::ports::cartridges::CartridgeRepository;

use crate::error_conversions::map_rusqlite;
use crate::repos::acts_sqlite::increment_counter_in_tx;
use crate::repos::audit_log_sqlite::{AuditEntry, SqliteAuditLogRepository};

/// SQLite-backed cartridge repository adapter (zero-sized).
#[derive(Debug, Default, Clone)]
pub struct SqliteCartridgeRepository;

/// A row from `audit_log` for the cartridge history view (D-History-01).
#[derive(Debug, Clone)]
pub struct AuditEntryRow {
    /// Primary key of the audit_log row (stable unique key for UI list keying).
    pub id: i64,
    pub entity_type: String,
    pub entity_id: i64,
    pub action: String,
    pub user_id: Option<i64>,
    pub before_json: Option<String>,
    pub after_json: Option<String>,
    pub payload_json: Option<String>,
    pub created_at_utc: i64,
}

/// SELECT with the column order expected by `map_row`.
///
/// Joins:
///   - `cartridge_models m` for brand, model name and kind_id.
///   - `cartridge_statuses cs` for human-readable status name.
///   - `cartridge_states cst` for human-readable state name.
const SELECT_CARTRIDGES: &str = "
    SELECT c.id, c.code, c.model_id,
           m.brand AS model_brand, m.model AS model_name, m.kind_id AS model_kind_id,
           c.status_id, cs.name AS status_name,
           c.state_id, cst.name AS state_name,
           c.location, c.holder_name, c.notes,
           c.created_at_utc, c.updated_at_utc, c.deleted_at_utc, c.version
      FROM cartridges c
      LEFT JOIN cartridge_models m ON m.id = c.model_id
      LEFT JOIN cartridge_statuses cs ON cs.id = c.status_id
      LEFT JOIN cartridge_states cst ON cst.id = c.state_id
";

/// Maps a row from `SELECT_CARTRIDGES` into `CartridgeRow`.
fn map_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<CartridgeRow> {
    Ok(CartridgeRow {
        id: row.get(0)?,
        code: row.get(1)?,
        model_id: row.get(2)?,
        model_brand: row.get(3)?,
        model_name: row.get(4)?,
        model_kind_id: row.get(5)?,
        status_id: row.get(6)?,
        status_name: row.get(7)?,
        state_id: row.get(8)?,
        state_name: row.get(9)?,
        location: row.get(10)?,
        holder_name: row.get(11)?,
        notes: row.get(12)?,
        created_at_utc: row.get(13)?,
        updated_at_utc: row.get(14)?,
        deleted_at_utc: row.get(15)?,
        version: row.get(16)?,
    })
}

impl SqliteCartridgeRepository {
    // -----------------------------------------------------------------------
    // Tx-helpers used by CartridgeService
    // -----------------------------------------------------------------------

    /// Look up the kind_id (1=Картридж, 2=Фотобарабан) of a cartridge model
    /// inside a transaction. Used by the create path to pick the C-/D- code
    /// prefix. Returns `NotFound` if the model is missing/soft-deleted.
    pub fn model_kind_in_tx(tx: &Transaction<'_>, model_id: i64) -> Result<i64, AppError> {
        tx.query_row(
            "SELECT kind_id FROM cartridge_models WHERE id = ?1 AND deleted_at_utc IS NULL",
            params![model_id],
            |r| r.get::<_, i64>(0),
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => AppError::NotFound {
                entity: "cartridge_model",
                id: model_id,
            },
            other => map_rusqlite(other),
        })
    }

    /// Assign a code to a new cartridge/drum inside a transaction.
    ///
    /// - `code_override = Some(s)`: validate UNIQUE; return `(s, false)` or
    ///   `AppError::Conflict` on collision (D-Code-Override-01).
    /// - `code_override = None`: increment the kind-specific counter
    ///   (`cartridge_seq`→`C-NNNNNN` / `drum_seq`→`D-NNNNNN`) in a retry loop
    ///   until a unique code is found (D-Code-01). The counter is never lost.
    ///
    /// Returns `(code, was_auto)`.
    pub fn assign_code_in_tx(
        tx: &Transaction<'_>,
        code_override: Option<&str>,
        kind_id: i64,
        _now_utc: i64,
    ) -> Result<(String, bool), AppError> {
        if let Some(custom) = code_override {
            let exists: bool = tx
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM cartridges WHERE code = ?1 LIMIT 1)",
                    params![custom],
                    |r| r.get(0),
                )
                .map_err(map_rusqlite)?;
            if exists {
                return Err(AppError::Conflict {
                    reason: format!("Картридж с кодом «{}» уже существует", custom),
                });
            }
            return Ok((custom.to_owned(), false));
        }

        // Префикс и счётчик зависят от вида расходника: фотобарабаны (kind 2) →
        // D-NNNNNN из drum_seq; картриджи (kind 1) → C-NNNNNN из cartridge_seq.
        let (counter_name, prefix) = if kind_id == 2 {
            ("drum_seq", 'D')
        } else {
            ("cartridge_seq", 'C')
        };

        // Auto-code: increment counter + retry loop (counter never lost on collision).
        loop {
            let seq = increment_counter_in_tx(tx, counter_name)?;
            let candidate = format!("{prefix}-{seq:06}");
            let exists: bool = tx
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM cartridges WHERE code = ?1 LIMIT 1)",
                    params![&candidate],
                    |r| r.get(0),
                )
                .map_err(map_rusqlite)?;
            if !exists {
                return Ok((candidate, true));
            }
            // On collision — increment counter again, the slot is not lost.
        }
    }

    /// INSERT a new cartridge row inside a transaction.
    ///
    /// Performs location round-trip: if `location` is non-empty, inserts
    /// `INSERT OR IGNORE INTO locations` to maintain the shared locations
    /// autocomplete (D-Op-Location-01).
    #[allow(clippy::too_many_arguments)]
    pub fn insert_cartridge_in_tx(
        &self,
        tx: &Transaction<'_>,
        code: &str,
        model_id: i64,
        status_id: i64,
        state_id: Option<i64>,
        location: Option<&str>,
        holder_name: Option<&str>,
        notes: Option<&str>,
        now_utc: i64,
    ) -> Result<i64, AppError> {
        // Location round-trip — keep shared autocomplete in sync.
        Self::upsert_location_in_tx(tx, location, now_utc)?;

        tx.execute(
            "INSERT INTO cartridges \
             (code, model_id, status_id, state_id, location, holder_name, notes, \
              created_at_utc, updated_at_utc, version) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8, 1)",
            params![
                code,
                model_id,
                status_id,
                state_id,
                location,
                holder_name,
                notes,
                now_utc,
            ],
        )
        .map_err(map_rusqlite)?;
        Ok(tx.last_insert_rowid())
    }

    /// INSERT a new cartridge model row inside a transaction.
    pub fn insert_model_in_tx(
        &self,
        tx: &Transaction<'_>,
        new: &CartridgeModelNew,
        now_utc: i64,
    ) -> Result<i64, AppError> {
        tx.execute(
            "INSERT INTO cartridge_models \
             (brand, model, kind_id, color, notes, created_at_utc, updated_at_utc, version) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6, 1)",
            params![
                new.brand,
                new.model,
                new.kind_id,
                new.color,
                new.notes,
                now_utc,
            ],
        )
        .map_err(map_rusqlite)?;
        Ok(tx.last_insert_rowid())
    }

    /// UPDATE an existing cartridge model row inside a transaction.
    #[allow(clippy::too_many_arguments)]
    pub fn update_model_in_tx(
        &self,
        tx: &Transaction<'_>,
        id: i64,
        version: i64,
        brand: &str,
        model: &str,
        kind_id: i64,
        color: Option<&str>,
        notes: Option<&str>,
        now_utc: i64,
    ) -> Result<(), AppError> {
        let affected = tx
            .execute(
                "UPDATE cartridge_models SET brand=?1, model=?2, kind_id=?3, color=?4, notes=?5, \
                 updated_at_utc=?6, version=version+1 \
                 WHERE id=?7 AND version=?8 AND deleted_at_utc IS NULL",
                params![brand, model, kind_id, color, notes, now_utc, id, version],
            )
            .map_err(map_rusqlite)?;

        if affected == 0 {
            let actual: Option<i64> = tx
                .query_row(
                    "SELECT version FROM cartridge_models WHERE id = ?1",
                    params![id],
                    |r| r.get(0),
                )
                .optional()
                .map_err(map_rusqlite)?;
            return match actual {
                None => Err(AppError::NotFound {
                    entity: "cartridge_model",
                    id,
                }),
                Some(actual) => Err(AppError::OptimisticLockMismatch {
                    entity: "cartridge_model",
                    id,
                    expected: version,
                    actual,
                }),
            };
        }
        Ok(())
    }

    /// Upsert compatibility pairs for a cartridge model inside a transaction.
    ///
    /// Deletes all existing pairs for `model_id`, then inserts the provided
    /// `pairs` (Бренд, Модель). Empty `pairs` → effectively clears compatibility.
    pub fn upsert_compatibility_in_tx(
        &self,
        tx: &Transaction<'_>,
        model_id: i64,
        pairs: &[(String, String)],
    ) -> Result<(), AppError> {
        tx.execute(
            "DELETE FROM cartridge_model_compatibility WHERE cartridge_model_id = ?1",
            params![model_id],
        )
        .map_err(map_rusqlite)?;

        for (brand, model) in pairs {
            tx.execute(
                "INSERT INTO cartridge_model_compatibility \
                 (cartridge_model_id, printer_brand, printer_model) VALUES (?1, ?2, ?3)",
                params![model_id, brand, model],
            )
            .map_err(map_rusqlite)?;
        }
        Ok(())
    }

    /// Fetch compatibility pairs for a cartridge model (read-only).
    pub fn get_compatibility(
        &self,
        conn: &Connection,
        model_id: i64,
    ) -> Result<Vec<(String, String)>, AppError> {
        let mut stmt = conn
            .prepare(
                "SELECT printer_brand, printer_model \
                 FROM cartridge_model_compatibility \
                 WHERE cartridge_model_id = ?1 \
                 ORDER BY printer_brand ASC, printer_model ASC",
            )
            .map_err(map_rusqlite)?;
        let rows = stmt
            .query_map(params![model_id], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
            })
            .map_err(map_rusqlite)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(map_rusqlite)?);
        }
        Ok(out)
    }

    /// Apply a lifecycle transition inside a transaction.
    ///
    /// Steps:
    ///   1. Fetch current row for optimistic lock + status validation.
    ///   2. Validate the op is allowed from the current status (domain rule).
    ///   3. UPDATE cartridges (status_id, state_id, location, holder_name, version).
    ///   4. Location round-trip (INSERT OR IGNORE INTO locations).
    ///   5. Insert audit_log row with before/after snapshots + payload.
    pub fn transition_in_tx(
        &self,
        tx: &Transaction<'_>,
        cartridge_id: i64,
        version: i64,
        op: &CartridgeTransitionOp,
        now_utc: i64,
    ) -> Result<(), AppError> {
        // 1. Fetch current row (also validates it exists).
        let current = self.fetch_in_tx(tx, cartridge_id)?;

        // 2. Optimistic lock check.
        if current.version != version {
            return Err(AppError::OptimisticLockMismatch {
                entity: "cartridge",
                id: cartridge_id,
                expected: version,
                actual: current.version,
            });
        }

        // 3. Domain rule: validate the transition is allowed for current status.
        op.validate_from_status(current.status_id)?;

        // 3b. Kind-specific rules для фотобарабанов (kind 2): нет заправки;
        // отработанный (state 6) нельзя устанавливать — только списать.
        if current.model_kind_id == Some(2) {
            match op {
                CartridgeTransitionOp::ToRefill { .. }
                | CartridgeTransitionOp::FromRefill { .. } => {
                    return Err(AppError::Validation {
                        field: "op".to_string(),
                        message: "Фотобарабан нельзя отправлять на заправку".to_string(),
                    });
                }
                CartridgeTransitionOp::Install { .. } if current.state_id == Some(6) => {
                    return Err(AppError::Validation {
                        field: "op".to_string(),
                        message: "Отработанный фотобарабан нельзя установить — только списать"
                            .to_string(),
                    });
                }
                _ => {}
            }
        }

        // 4. Calculate new field values.
        let new_status_id = op.target_status_id();
        let (new_state_id, new_location, new_holder_name) = match op {
            CartridgeTransitionOp::Install {
                location,
                given_to_name,
                ..
            } => (
                current.state_id,
                Some(location.as_str()),
                Some(given_to_name.as_str()),
            ),
            CartridgeTransitionOp::ReturnToStock {
                state_id, location, ..
            } => (Some(*state_id), Some(location.as_str()), None),
            CartridgeTransitionOp::ToRefill {
                location,
                given_to_name,
                ..
            } => (
                current.state_id,
                Some(location.as_str()),
                Some(given_to_name.as_str()),
            ),
            CartridgeTransitionOp::FromRefill {
                state_id, location, ..
            } => (Some(*state_id), Some(location.as_str()), None),
            CartridgeTransitionOp::WriteOff { .. } => {
                (current.state_id, current.location.as_deref(), None)
            }
        };

        // 5. UPDATE cartridges (optimistic lock on version).
        let affected = tx
            .execute(
                "UPDATE cartridges SET status_id=?1, state_id=?2, location=?3, holder_name=?4, \
                 updated_at_utc=?5, version=version+1 \
                 WHERE id=?6 AND version=?7",
                params![
                    new_status_id,
                    new_state_id,
                    new_location,
                    new_holder_name,
                    now_utc,
                    cartridge_id,
                    version,
                ],
            )
            .map_err(map_rusqlite)?;

        if affected == 0 {
            // Race: something changed between our fetch and our update.
            return Err(AppError::OptimisticLockMismatch {
                entity: "cartridge",
                id: cartridge_id,
                expected: version,
                actual: current.version + 1,
            });
        }

        // 6. Location round-trip.
        Self::upsert_location_in_tx(tx, new_location, now_utc)?;

        // 7. Build payload_json for audit (D-History-01).
        let payload_json = Self::op_payload_json(op);

        // 8. Before snapshot (for history display).
        let before_json = serde_json::to_string(&json!({
            "status_id": current.status_id,
            "status_name": current.status_name,
            "state_id": current.state_id,
            "state_name": current.state_name,
            "location": current.location,
            "holder_name": current.holder_name,
        }))
        .map_err(|e| AppError::Internal {
            source_chain: format!("before_json serialize: {e}"),
        })?;

        // 9. Audit log insert.
        let audit_repo = SqliteAuditLogRepository;
        audit_repo.insert(
            tx,
            AuditEntry {
                entity_type: "cartridge",
                entity_id: cartridge_id,
                action: op.audit_action(),
                user_id: None, // Phase 4: always NULL (RBAC is Phase 5)
                before_json: Some(before_json),
                after_json: None,
                payload_json: Some(payload_json),
                created_at_utc: now_utc,
            },
        )?;

        Ok(())
    }

    /// Build the payload_json string for a lifecycle operation.
    fn op_payload_json(op: &CartridgeTransitionOp) -> String {
        let value = match op {
            CartridgeTransitionOp::Install {
                date_utc,
                given_by_name,
                given_to_name,
                location,
            } => json!({
                "op": "install",
                "date_utc": date_utc,
                "given_by_name": given_by_name,
                "given_to_name": given_to_name,
                "location": location,
            }),
            CartridgeTransitionOp::ReturnToStock {
                state_id,
                location,
                notes,
            } => json!({
                "op": "return_to_stock",
                "state_id": state_id,
                "location": location,
                "notes": notes,
            }),
            CartridgeTransitionOp::ToRefill {
                date_utc,
                given_by_name,
                given_to_name,
                location,
            } => json!({
                "op": "to_refill",
                "date_utc": date_utc,
                "given_by_name": given_by_name,
                "given_to_name": given_to_name,
                "location": location,
            }),
            CartridgeTransitionOp::FromRefill {
                state_id,
                location,
                notes,
            } => json!({
                "op": "from_refill",
                "state_id": state_id,
                "location": location,
                "notes": notes,
            }),
            CartridgeTransitionOp::WriteOff { date_utc, notes } => json!({
                "op": "write_off",
                "date_utc": date_utc,
                "notes": notes,
            }),
        };
        value.to_string()
    }

    /// Fetch a cartridge row inside an open transaction.
    /// Used to capture the before-snapshot and do optimistic lock validation.
    pub fn fetch_in_tx(&self, tx: &Transaction<'_>, id: i64) -> Result<CartridgeRow, AppError> {
        tx.query_row(
            &format!("{SELECT_CARTRIDGES} WHERE c.id = ?1"),
            params![id],
            map_row,
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => AppError::NotFound {
                entity: "cartridge",
                id,
            },
            other => map_rusqlite(other),
        })
    }

    /// Perform location round-trip: INSERT OR IGNORE INTO locations.
    ///
    /// Only inserts if location is Some and non-empty. This keeps the shared
    /// `locations` autocomplete in sync with freeform text entered in
    /// cartridge forms (D-Op-Location-01).
    fn upsert_location_in_tx(
        tx: &Transaction<'_>,
        location: Option<&str>,
        now_utc: i64,
    ) -> Result<(), AppError> {
        if let Some(loc) = location.filter(|s| !s.is_empty()) {
            tx.execute(
                "INSERT OR IGNORE INTO locations (name, created_at_utc, updated_at_utc, version) \
                 VALUES (?1, ?2, ?2, 1)",
                params![loc, now_utc],
            )
            .map_err(map_rusqlite)?;
        }
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Public read-only helpers (called from service layer via ReaderPool)
    // -----------------------------------------------------------------------

    /// FTS5 + LIKE search over cartridges (CART-11, D-Search-01).
    ///
    /// UNION CTE: `fts_hits` (FTS5 MATCH on cartridges_fts) UNION `like_hits`
    /// (LIKE on code, location, holder_name + model brand/model via JOIN).
    ///
    /// Security: FTS MATCH parameter is passed via `params![]` — not concatenated.
    /// Double-quotes in `query` are escaped before MATCH to avoid FTS syntax errors.
    pub fn search(
        &self,
        conn: &Connection,
        query: &str,
        filter: &CartridgeFilter,
    ) -> Result<Vec<CartridgeRow>, AppError> {
        // Guard: FTS5 MATCH on a phrase with no alphanumeric tokens (e.g. "---",
        // a lone double-quote, or punctuation-only input) can return SQLITE_ERROR
        // on some unicode61 builds. When the query has no alphanumeric chars,
        // skip the fts_hits CTE and fall back to LIKE-only (WR-01).
        let has_token = query.chars().any(|c| c.is_alphanumeric());

        let like_query = format!("%{}%", query);

        let sql = if has_token {
            // Escape double-quotes in FTS query to avoid FTS5 syntax errors (T-04-02-01).
            let fts_query_escaped = query.replace('"', "\"\"");
            // Store in a way that outlives the if-arm.
            format!(
                "WITH fts_hits AS ( \
                   SELECT f.rowid AS id FROM cartridges_fts f \
                   WHERE cartridges_fts MATCH '\"{}\"*' \
                 ), \
                 like_hits AS ( \
                   SELECT c.id FROM cartridges c \
                   LEFT JOIN cartridge_models m ON m.id = c.model_id \
                   WHERE c.code LIKE ?1 \
                      OR c.location LIKE ?1 \
                      OR c.holder_name LIKE ?1 \
                      OR m.brand LIKE ?1 \
                      OR m.model LIKE ?1 \
                 ) \
                 {SELECT_CARTRIDGES} \
                 WHERE c.id IN (SELECT id FROM fts_hits UNION SELECT id FROM like_hits) \
                   AND c.deleted_at_utc IS NULL \
                   AND (?2 IS NULL OR c.status_id = ?2) \
                   AND (?3 IS NULL OR m.kind_id = ?3) \
                   AND (?4 IS NULL OR c.model_id = ?4) \
                 ORDER BY c.created_at_utc DESC, c.id DESC \
                 LIMIT 200",
                fts_query_escaped
            )
        } else {
            format!(
                "WITH like_hits AS ( \
                   SELECT c.id FROM cartridges c \
                   LEFT JOIN cartridge_models m ON m.id = c.model_id \
                   WHERE c.code LIKE ?1 \
                      OR c.location LIKE ?1 \
                      OR c.holder_name LIKE ?1 \
                      OR m.brand LIKE ?1 \
                      OR m.model LIKE ?1 \
                 ) \
                 {SELECT_CARTRIDGES} \
                 WHERE c.id IN (SELECT id FROM like_hits) \
                   AND c.deleted_at_utc IS NULL \
                   AND (?2 IS NULL OR c.status_id = ?2) \
                   AND (?3 IS NULL OR m.kind_id = ?3) \
                   AND (?4 IS NULL OR c.model_id = ?4) \
                 ORDER BY c.created_at_utc DESC, c.id DESC \
                 LIMIT 200"
            )
        };

        let mut stmt = conn.prepare(&sql).map_err(map_rusqlite)?;
        let rows = stmt
            .query_map(
                params![
                    like_query,
                    filter.status_id,
                    filter.kind_id,
                    filter.model_id,
                ],
                map_row,
            )
            .map_err(map_rusqlite)?;

        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(map_rusqlite)?);
        }
        Ok(out)
    }

    /// Low-stock query (CART-12, D-LowStock-02).
    ///
    /// Returns models where `count(in_stock AND full) < threshold`.
    /// Threshold is read from `app_settings.low_stock_threshold` (default 2).
    ///
    /// WR-06: `CAST(value AS INTEGER)` in SQLite silently converts non-numeric
    /// strings to 0, bypassing the `unwrap_or(2)` fallback. Instead, read the
    /// raw string value and parse it in Rust with an explicit > 0 guard so a
    /// malformed setting always falls back to the intended default of 2.
    pub fn low_stock(&self, conn: &Connection) -> Result<Vec<LowStockItem>, AppError> {
        let threshold: i64 = conn
            .query_row(
                "SELECT value FROM app_settings WHERE key = 'low_stock_threshold'",
                [],
                |r| r.get::<_, String>(0),
            )
            .ok()
            .and_then(|s| s.trim().parse::<i64>().ok())
            .filter(|&t| t > 0)
            .unwrap_or(2);

        let sql = "SELECT m.id, m.brand, m.model, COUNT(c.id) AS cnt \
                   FROM cartridge_models m \
                   LEFT JOIN cartridges c ON c.model_id = m.id \
                     AND c.status_id = 1 \
                     AND c.state_id = 1 \
                     AND c.deleted_at_utc IS NULL \
                   WHERE m.deleted_at_utc IS NULL \
                   GROUP BY m.id \
                   HAVING cnt < ?1 \
                   ORDER BY cnt ASC, m.brand ASC, m.model ASC";

        let mut stmt = conn.prepare(sql).map_err(map_rusqlite)?;
        let rows = stmt
            .query_map(params![threshold], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, i64>(3)?,
                ))
            })
            .map_err(map_rusqlite)?;

        let mut out = Vec::new();
        for row in rows {
            let (model_id, brand, model, count) = row.map_err(map_rusqlite)?;
            out.push(LowStockItem {
                model_id,
                brand,
                model,
                count,
                threshold,
            });
        }
        Ok(out)
    }

    /// Cartridge history from audit_log (D-History-01, CART-10).
    ///
    /// Returns audit entries for `entity_type = 'cartridge'` and the given
    /// `cartridge_id`, excluding trivial read-ops, ordered newest-first.
    pub fn get_history(
        &self,
        conn: &Connection,
        cartridge_id: i64,
    ) -> Result<Vec<AuditEntryRow>, AppError> {
        let mut stmt = conn
            .prepare(
                "SELECT id, entity_type, entity_id, action, user_id, \
                        before_json, after_json, payload_json, created_at_utc \
                   FROM audit_log \
                  WHERE entity_type = 'cartridge' \
                    AND entity_id = ?1 \
                    AND action NOT IN ('list', 'get') \
                  ORDER BY created_at_utc DESC, id DESC",
            )
            .map_err(map_rusqlite)?;

        let rows = stmt
            .query_map(params![cartridge_id], |r| {
                Ok(AuditEntryRow {
                    id: r.get(0)?,
                    entity_type: r.get(1)?,
                    entity_id: r.get(2)?,
                    action: r.get(3)?,
                    user_id: r.get(4)?,
                    before_json: r.get(5)?,
                    after_json: r.get(6)?,
                    payload_json: r.get(7)?,
                    created_at_utc: r.get(8)?,
                })
            })
            .map_err(map_rusqlite)?;

        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(map_rusqlite)?);
        }
        Ok(out)
    }

    // -----------------------------------------------------------------------
    // Model read helpers (called from service via ReaderPool)
    // -----------------------------------------------------------------------

    /// Fetch a single cartridge model by ID.
    pub fn get_model(&self, conn: &Connection, id: i64) -> Result<CartridgeModelRow, AppError> {
        conn.query_row(
            "SELECT id, brand, model, kind_id, color, notes, \
                    created_at_utc, updated_at_utc, deleted_at_utc, version \
               FROM cartridge_models \
              WHERE id = ?1 AND deleted_at_utc IS NULL",
            params![id],
            map_model_row,
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => AppError::NotFound {
                entity: "cartridge_model",
                id,
            },
            other => map_rusqlite(other),
        })
    }

    /// List all non-deleted cartridge models.
    pub fn list_models(&self, conn: &Connection) -> Result<Vec<CartridgeModelRow>, AppError> {
        let mut stmt = conn
            .prepare(
                "SELECT id, brand, model, kind_id, color, notes, \
                        created_at_utc, updated_at_utc, deleted_at_utc, version \
                   FROM cartridge_models \
                  WHERE deleted_at_utc IS NULL \
                  ORDER BY brand ASC, model ASC",
            )
            .map_err(map_rusqlite)?;
        let rows = stmt.query_map([], map_model_row).map_err(map_rusqlite)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(map_rusqlite)?);
        }
        Ok(out)
    }

    /// Count live (non-deleted) cartridge instances grouped by model id.
    /// Returns a map `model_id -> count`; models with zero instances are absent.
    pub fn count_instances_by_model(
        &self,
        conn: &Connection,
    ) -> Result<std::collections::HashMap<i64, i64>, AppError> {
        let mut stmt = conn
            .prepare(
                "SELECT model_id, COUNT(*) AS cnt \
                   FROM cartridges \
                  WHERE deleted_at_utc IS NULL \
                  GROUP BY model_id",
            )
            .map_err(map_rusqlite)?;
        let rows = stmt
            .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)))
            .map_err(map_rusqlite)?;
        let mut map = std::collections::HashMap::new();
        for row in rows {
            let (model_id, cnt) = row.map_err(map_rusqlite)?;
            map.insert(model_id, cnt);
        }
        Ok(map)
    }

    /// Soft-delete a cartridge model inside a transaction.
    ///
    /// Guards: returns `AppError::Conflict` if there are live (non-deleted)
    /// cartridge instances referencing this model (D-Conflict-Delete-Models-01).
    pub fn soft_delete_model_in_tx(
        &self,
        tx: &Transaction<'_>,
        id: i64,
        version: i64,
        now_utc: i64,
    ) -> Result<(), AppError> {
        // Guard: live cartridges referencing this model?
        let live_count: i64 = tx
            .query_row(
                "SELECT COUNT(*) FROM cartridges \
                  WHERE model_id = ?1 AND deleted_at_utc IS NULL",
                params![id],
                |r| r.get(0),
            )
            .map_err(map_rusqlite)?;

        if live_count > 0 {
            return Err(AppError::Conflict {
                reason: format!(
                    "Нельзя удалить модель: она используется {} картриджами",
                    live_count
                ),
            });
        }

        let affected = tx
            .execute(
                "UPDATE cartridge_models \
                 SET deleted_at_utc=?1, updated_at_utc=?1, version=version+1 \
                 WHERE id=?2 AND version=?3 AND deleted_at_utc IS NULL",
                params![now_utc, id, version],
            )
            .map_err(map_rusqlite)?;

        if affected == 0 {
            let actual: Option<i64> = tx
                .query_row(
                    "SELECT version FROM cartridge_models WHERE id = ?1",
                    params![id],
                    |r| r.get(0),
                )
                .optional()
                .map_err(map_rusqlite)?;
            return match actual {
                None => Err(AppError::NotFound {
                    entity: "cartridge_model",
                    id,
                }),
                Some(actual) => Err(AppError::OptimisticLockMismatch {
                    entity: "cartridge_model",
                    id,
                    expected: version,
                    actual,
                }),
            };
        }
        Ok(())
    }
}

/// Maps a cartridge_models row into `CartridgeModelRow`.
fn map_model_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<CartridgeModelRow> {
    Ok(CartridgeModelRow {
        id: row.get(0)?,
        brand: row.get(1)?,
        model: row.get(2)?,
        kind_id: row.get(3)?,
        color: row.get(4)?,
        notes: row.get(5)?,
        created_at_utc: row.get(6)?,
        updated_at_utc: row.get(7)?,
        deleted_at_utc: row.get(8)?,
        version: row.get(9)?,
    })
}

// ---------------------------------------------------------------------------
// CartridgeRepository trait impl
// ---------------------------------------------------------------------------

impl CartridgeRepository for SqliteCartridgeRepository {
    type Conn = Connection;

    fn get(&self, conn: &Self::Conn, id: i64) -> Result<CartridgeRow, AppError> {
        conn.query_row(
            &format!("{SELECT_CARTRIDGES} WHERE c.id = ?1 AND c.deleted_at_utc IS NULL"),
            params![id],
            map_row,
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => AppError::NotFound {
                entity: "cartridge",
                id,
            },
            other => map_rusqlite(other),
        })
    }

    fn list(
        &self,
        conn: &Self::Conn,
        filter: &CartridgeFilter,
        page: &Pagination,
    ) -> Result<(Vec<CartridgeRow>, u64), AppError> {
        let limit = page.limit.min(200) as i64;
        let offset = page.offset as i64;
        let include_deleted = filter.include_deleted;

        let installable_only = filter.installable_only as i64;

        // COUNT(*)
        let total: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM cartridges c \
                 LEFT JOIN cartridge_models m ON m.id = c.model_id \
                 WHERE (?1 = 1 OR c.deleted_at_utc IS NULL) \
                   AND (?2 IS NULL OR c.status_id = ?2) \
                   AND (?3 IS NULL OR m.kind_id = ?3) \
                   AND (?4 IS NULL OR c.model_id = ?4) \
                   AND (?5 = 0 OR (c.status_id = 1 AND (\
                         (m.kind_id = 1 AND c.state_id IN (1, 2)) \
                      OR (m.kind_id = 2 AND c.state_id IN (4, 5)) \
                   ))) \
                   AND (?6 IS NULL \
                        OR NOT EXISTS (SELECT 1 FROM printer_cartridge_models pcm WHERE pcm.device_id = ?6) \
                        OR c.model_id IN (SELECT cartridge_model_id FROM printer_cartridge_models WHERE device_id = ?6))",
                params![
                    include_deleted as i64,
                    filter.status_id,
                    filter.kind_id,
                    filter.model_id,
                    installable_only,
                    filter.compatible_with_printer_device_id,
                ],
                |r| r.get(0),
            )
            .map_err(map_rusqlite)?;

        let mut stmt = conn
            .prepare(&format!(
                "{SELECT_CARTRIDGES} \
                 WHERE (?1 = 1 OR c.deleted_at_utc IS NULL) \
                   AND (?2 IS NULL OR c.status_id = ?2) \
                   AND (?3 IS NULL OR m.kind_id = ?3) \
                   AND (?4 IS NULL OR c.model_id = ?4) \
                   AND (?5 = 0 OR (c.status_id = 1 AND (\
                         (m.kind_id = 1 AND c.state_id IN (1, 2)) \
                      OR (m.kind_id = 2 AND c.state_id IN (4, 5)) \
                   ))) \
                   AND (?6 IS NULL \
                        OR NOT EXISTS (SELECT 1 FROM printer_cartridge_models pcm WHERE pcm.device_id = ?6) \
                        OR c.model_id IN (SELECT cartridge_model_id FROM printer_cartridge_models WHERE device_id = ?6)) \
                 ORDER BY c.created_at_utc DESC, c.id DESC \
                 LIMIT ?7 OFFSET ?8"
            ))
            .map_err(map_rusqlite)?;

        let rows = stmt
            .query_map(
                params![
                    include_deleted as i64,
                    filter.status_id,
                    filter.kind_id,
                    filter.model_id,
                    installable_only,
                    filter.compatible_with_printer_device_id,
                    limit,
                    offset,
                ],
                map_row,
            )
            .map_err(map_rusqlite)?;

        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(map_rusqlite)?);
        }
        Ok((out, total as u64))
    }

    fn counts(&self, conn: &Self::Conn) -> Result<CartridgeCounts, AppError> {
        let all: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM cartridges WHERE deleted_at_utc IS NULL",
                [],
                |r| r.get(0),
            )
            .map_err(map_rusqlite)?;
        let in_stock: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM cartridges WHERE status_id = 1 AND deleted_at_utc IS NULL",
                [],
                |r| r.get(0),
            )
            .map_err(map_rusqlite)?;
        let in_use: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM cartridges WHERE status_id = 2 AND deleted_at_utc IS NULL",
                [],
                |r| r.get(0),
            )
            .map_err(map_rusqlite)?;
        let at_refill: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM cartridges WHERE status_id = 3 AND deleted_at_utc IS NULL",
                [],
                |r| r.get(0),
            )
            .map_err(map_rusqlite)?;
        let written_off: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM cartridges WHERE status_id = 4 AND deleted_at_utc IS NULL",
                [],
                |r| r.get(0),
            )
            .map_err(map_rusqlite)?;
        Ok(CartridgeCounts {
            all,
            in_stock,
            in_use,
            at_refill,
            written_off,
        })
    }

    fn peek_next_code(&self, conn: &Self::Conn) -> Result<i64, AppError> {
        conn.query_row(
            "SELECT current_value + 1 FROM counters WHERE name = 'cartridge_seq'",
            [],
            |r| r.get::<_, i64>(0),
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => AppError::Internal {
                source_chain: "counter 'cartridge_seq' not seeded".to_string(),
            },
            other => map_rusqlite(other),
        })
    }

    fn delete_soft(
        &self,
        conn: &mut Self::Conn,
        id: i64,
        version: i64,
        now_utc: i64,
    ) -> Result<(), AppError> {
        let tx = conn.transaction().map_err(map_rusqlite)?;

        let affected = tx
            .execute(
                "UPDATE cartridges SET deleted_at_utc=?1, updated_at_utc=?1, version=version+1 \
                 WHERE id=?2 AND version=?3 AND deleted_at_utc IS NULL",
                params![now_utc, id, version],
            )
            .map_err(map_rusqlite)?;

        if affected == 0 {
            let actual: Option<i64> = tx
                .query_row(
                    "SELECT version FROM cartridges WHERE id = ?1",
                    params![id],
                    |r| r.get(0),
                )
                .optional()
                .map_err(map_rusqlite)?;
            return match actual {
                None => Err(AppError::NotFound {
                    entity: "cartridge",
                    id,
                }),
                Some(actual) => Err(AppError::OptimisticLockMismatch {
                    entity: "cartridge",
                    id,
                    expected: version,
                    actual,
                }),
            };
        }

        tx.commit().map_err(map_rusqlite)?;
        Ok(())
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
        let path = dir.path().join("cart-repo-test.db");
        let mut conn = Connection::open(&path).expect("open");
        apply_writer_pragmas(&conn).expect("writer pragmas");
        migrations::run(&mut conn).expect("migrations");
        (conn, dir)
    }

    fn seed_model(conn: &mut Connection, brand: &str, model: &str) -> i64 {
        let tx = conn.transaction().expect("tx");
        let now = 1_700_000_000_i64;
        let repo = SqliteCartridgeRepository;
        let id = repo
            .insert_model_in_tx(
                &tx,
                &CartridgeModelNew {
                    brand: brand.into(),
                    model: model.into(),
                    kind_id: 1,
                    color: Some("Чёрный".into()),
                    notes: None,
                },
                now,
            )
            .expect("insert model");
        tx.commit().expect("commit");
        id
    }

    #[test]
    fn assign_code_auto_increments() {
        let (mut conn, _g) = fresh_conn();
        let tx = conn.transaction().expect("tx");
        let now = 1_700_000_000_i64;
        let (code1, was_auto) =
            SqliteCartridgeRepository::assign_code_in_tx(&tx, None, 1, now).expect("code1");
        assert!(was_auto);
        assert_eq!(code1, "C-000001");
        tx.commit().expect("commit");

        let tx2 = conn.transaction().expect("tx2");
        let (code2, _) =
            SqliteCartridgeRepository::assign_code_in_tx(&tx2, None, 1, now).expect("code2");
        assert_eq!(code2, "C-000002");
        tx2.commit().expect("commit");
    }

    #[test]
    fn assign_code_drum_uses_d_prefix_and_separate_counter() {
        // UAT round 3 №4: фотобарабаны (kind 2) получают код D-NNNNNN из
        // отдельного счётчика drum_seq, не конфликтуя с C-NNNNNN картриджей.
        let (mut conn, _g) = fresh_conn();
        let now = 1_700_000_000_i64;

        let tx = conn.transaction().expect("tx");
        let (c_code, _) =
            SqliteCartridgeRepository::assign_code_in_tx(&tx, None, 1, now).expect("cartridge");
        let (d_code, _) =
            SqliteCartridgeRepository::assign_code_in_tx(&tx, None, 2, now).expect("drum");
        let (d_code2, _) =
            SqliteCartridgeRepository::assign_code_in_tx(&tx, None, 2, now).expect("drum2");
        tx.commit().expect("commit");

        assert_eq!(c_code, "C-000001");
        assert_eq!(d_code, "D-000001");
        assert_eq!(d_code2, "D-000002");
    }

    #[test]
    fn assign_code_custom_roundtrip() {
        let (mut conn, _g) = fresh_conn();
        let tx = conn.transaction().expect("tx");
        let now = 1_700_000_000_i64;
        let (code, was_auto) =
            SqliteCartridgeRepository::assign_code_in_tx(&tx, Some("BARCODE-42"), 1, now)
                .expect("custom code");
        assert!(!was_auto);
        assert_eq!(code, "BARCODE-42");
        tx.commit().expect("commit");
    }

    #[test]
    fn insert_and_get_cartridge() {
        let (mut conn, _g) = fresh_conn();
        let model_id = seed_model(&mut conn, "Pantum", "TL-5120X");
        let repo = SqliteCartridgeRepository;
        let now = 1_700_000_000_i64;

        let id = {
            let tx = conn.transaction().expect("tx");
            let (code, _) =
                SqliteCartridgeRepository::assign_code_in_tx(&tx, None, 1, now).expect("code");
            let id = repo
                .insert_cartridge_in_tx(
                    &tx,
                    &code,
                    model_id,
                    1,
                    Some(1),
                    Some("Склад"),
                    None,
                    None,
                    now,
                )
                .expect("insert");
            tx.commit().expect("commit");
            id
        };

        let row = repo.get(&conn, id).expect("get");
        assert_eq!(row.model_brand.as_deref(), Some("Pantum"));
        assert_eq!(row.model_name.as_deref(), Some("TL-5120X"));
        assert_eq!(row.status_id, 1);
        assert_eq!(row.state_id, Some(1));
        assert_eq!(row.location.as_deref(), Some("Склад"));
    }

    #[test]
    fn counts_correct() {
        let (mut conn, _g) = fresh_conn();
        let model_id = seed_model(&mut conn, "Cactus", "TL-5120P");
        let repo = SqliteCartridgeRepository;
        let now = 1_700_000_000_i64;

        // Insert one in_stock cartridge.
        {
            let tx = conn.transaction().expect("tx");
            let (code, _) =
                SqliteCartridgeRepository::assign_code_in_tx(&tx, None, 1, now).expect("code");
            repo.insert_cartridge_in_tx(&tx, &code, model_id, 1, None, None, None, None, now)
                .expect("insert");
            tx.commit().expect("commit");
        }

        let counts = repo.counts(&conn).expect("counts");
        assert_eq!(counts.all, 1);
        assert_eq!(counts.in_stock, 1);
        assert_eq!(counts.in_use, 0);
    }

    #[test]
    fn count_instances_by_model_groups_live_cartridges() {
        // UAT round 2 №4: модели показывали «0 шт.» — счётчик экземпляров не
        // вычислялся. Здесь проверяем, что count группирует только живые
        // (не soft-deleted) картриджи по model_id.
        let (mut conn, _g) = fresh_conn();
        let model_a = seed_model(&mut conn, "Pantum", "TL-5120X");
        let model_b = seed_model(&mut conn, "HP", "W1106A");
        let repo = SqliteCartridgeRepository;
        let now = 1_700_000_000_i64;

        // 2 картриджа модели A (один потом soft-delete) + 1 модели B.
        let mut a_ids = Vec::new();
        for _ in 0..2 {
            let tx = conn.transaction().expect("tx");
            let (code, _) =
                SqliteCartridgeRepository::assign_code_in_tx(&tx, None, 1, now).expect("code");
            let id = repo
                .insert_cartridge_in_tx(&tx, &code, model_a, 1, None, None, None, None, now)
                .expect("insert");
            tx.commit().expect("commit");
            a_ids.push(id);
        }
        {
            let tx = conn.transaction().expect("tx");
            let (code, _) =
                SqliteCartridgeRepository::assign_code_in_tx(&tx, None, 1, now).expect("code");
            repo.insert_cartridge_in_tx(&tx, &code, model_b, 1, None, None, None, None, now)
                .expect("insert");
            tx.commit().expect("commit");
        }

        let map = repo
            .count_instances_by_model(&conn)
            .expect("count_instances_by_model");
        assert_eq!(map.get(&model_a).copied().unwrap_or(0), 2);
        assert_eq!(map.get(&model_b).copied().unwrap_or(0), 1);

        // Soft-delete one cartridge of model A → count drops to 1, не в нуль.
        conn.execute(
            "UPDATE cartridges SET deleted_at_utc = ?1 WHERE id = ?2",
            params![now, a_ids[0]],
        )
        .expect("soft delete");
        let map2 = repo
            .count_instances_by_model(&conn)
            .expect("count after delete");
        assert_eq!(map2.get(&model_a).copied().unwrap_or(0), 1);
    }

    #[test]
    fn transition_install_changes_status() {
        let (mut conn, _g) = fresh_conn();
        let model_id = seed_model(&mut conn, "Pantum", "TL-5120X");
        let repo = SqliteCartridgeRepository;
        let now = 1_700_000_000_i64;

        let cart_id = {
            let tx = conn.transaction().expect("tx");
            let (code, _) =
                SqliteCartridgeRepository::assign_code_in_tx(&tx, None, 1, now).expect("code");
            let id = repo
                .insert_cartridge_in_tx(
                    &tx,
                    &code,
                    model_id,
                    1,
                    Some(1),
                    Some("Склад"),
                    None,
                    None,
                    now,
                )
                .expect("insert");
            tx.commit().expect("commit");
            id
        };

        let op = CartridgeTransitionOp::Install {
            date_utc: now,
            given_by_name: "Иванов".into(),
            given_to_name: "Петров".into(),
            location: "Каб. 305".into(),
        };

        {
            let tx = conn.transaction().expect("tx");
            repo.transition_in_tx(&tx, cart_id, 1, &op, now)
                .expect("transition");
            tx.commit().expect("commit");
        }

        let row = repo.get(&conn, cart_id).expect("get after transition");
        assert_eq!(row.status_id, 2); // В работе
        assert_eq!(row.holder_name.as_deref(), Some("Петров"));
        assert_eq!(row.location.as_deref(), Some("Каб. 305"));
    }

    #[test]
    fn transition_wrong_status_returns_validation_error() {
        let (mut conn, _g) = fresh_conn();
        let model_id = seed_model(&mut conn, "Pantum", "TL-5120X");
        let repo = SqliteCartridgeRepository;
        let now = 1_700_000_000_i64;

        let cart_id = {
            let tx = conn.transaction().expect("tx");
            let (code, _) =
                SqliteCartridgeRepository::assign_code_in_tx(&tx, None, 1, now).expect("code");
            let id = repo
                .insert_cartridge_in_tx(&tx, &code, model_id, 1, Some(1), None, None, None, now)
                .expect("insert");
            tx.commit().expect("commit");
            id
        };

        // ReturnToStock requires status_id=2 (В работе); current is 1 (На складе)
        let op = CartridgeTransitionOp::ReturnToStock {
            state_id: 3,
            location: "Склад".into(),
            notes: None,
        };

        let tx = conn.transaction().expect("tx");
        let err = repo
            .transition_in_tx(&tx, cart_id, 1, &op, now)
            .expect_err("should fail");
        assert!(matches!(err, AppError::Validation { .. }), "got {err:?}");
    }

    #[test]
    fn low_stock_returns_models_below_threshold() {
        let (mut conn, _g) = fresh_conn();
        let model_id = seed_model(&mut conn, "Cactus", "TL-5120P");
        let repo = SqliteCartridgeRepository;
        let now = 1_700_000_000_i64;

        // Insert 1 in-stock + full cartridge (threshold default is 2, so 1 < 2)
        {
            let tx = conn.transaction().expect("tx");
            let (code, _) =
                SqliteCartridgeRepository::assign_code_in_tx(&tx, None, 1, now).expect("code");
            repo.insert_cartridge_in_tx(&tx, &code, model_id, 1, Some(1), None, None, None, now)
                .expect("insert");
            tx.commit().expect("commit");
        }

        let items = repo.low_stock(&conn).expect("low_stock");
        assert_eq!(items.len(), 1, "one model below threshold");
        assert_eq!(items[0].model_id, model_id);
        assert_eq!(items[0].count, 1);
        assert_eq!(items[0].threshold, 2);
    }

    #[test]
    fn params_are_parameterized_not_concatenated() {
        // Verify that the search function accepts SQL-injection-like input without panic.
        let (conn, _g) = fresh_conn();
        let repo = SqliteCartridgeRepository;
        let filter = CartridgeFilter::default();
        // Would break if input was concatenated into SQL string.
        let result = repo.search(&conn, "' OR '1'='1", &filter);
        assert!(result.is_ok(), "search should not panic on injection input");
    }

    #[test]
    fn search_punctuation_only_query_returns_ok() {
        // WR-01: a query with no alphanumeric tokens (e.g. "---") must not
        // produce an FTS5 syntax error — the LIKE-only fallback path is used.
        let (conn, _g) = fresh_conn();
        let repo = SqliteCartridgeRepository;
        let filter = CartridgeFilter::default();
        for q in &["---", "...", "\"", "   ", "!!"] {
            let result = repo.search(&conn, q, &filter);
            assert!(
                result.is_ok(),
                "search should return Ok for punctuation-only query {:?}, got: {:?}",
                q,
                result
            );
        }
    }
}
