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
    CartridgeTransitionOp, CompatibleModelAggregate, LowStockBasis, LowStockItem, Pagination,
};
use trackly_core::domain::places::{shorten_place_path, PathDisplayVariant};
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
///   - `place_effective_variant pev` adds `pev.effective_variant` as column
///     index 18 (Phase 39.1 Plan 04). `map_row` NEVER reads index 18 — it is
///     read only by `map_row_with_short_path`, used exclusively by `list()`
///     (D-19-equivalent: `get`/`search_fts` keep using bare `map_row` and
///     always yield `place_path_short: None`).
const SELECT_CARTRIDGES: &str = "
    SELECT c.id, c.code, c.model_id,
           m.brand AS model_brand, m.model AS model_name, m.kind_id AS model_kind_id,
           c.status_id, cs.name AS status_name,
           c.state_id, cst.name AS state_name,
           c.place_id, pfp.full_path, c.holder_name, c.notes,
           c.created_at_utc, c.updated_at_utc, c.deleted_at_utc, c.version,
           pev.effective_variant AS place_variant
      FROM cartridges c
      LEFT JOIN cartridge_models m ON m.id = c.model_id
      LEFT JOIN cartridge_statuses cs ON cs.id = c.status_id
      LEFT JOIN cartridge_states cst ON cst.id = c.state_id
      LEFT JOIN place_full_paths pfp ON pfp.place_id = c.place_id
      LEFT JOIN place_effective_variant pev ON pev.place_id = c.place_id
";

/// Maps a row from `SELECT_CARTRIDGES` into `CartridgeRow`. `place_path_short`
/// is always `None` here — it is computed separately by
/// `map_row_with_short_path` (mirrors `devices_sqlite.rs::from_row`).
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
        place_id: row.get(10)?,
        full_path: row.get(11)?,
        place_path_short: None,
        holder_name: row.get(12)?,
        notes: row.get(13)?,
        created_at_utc: row.get(14)?,
        updated_at_utc: row.get(15)?,
        deleted_at_utc: row.get(16)?,
        version: row.get(17)?,
    })
}

/// Reads org-wide `place_path_sep_ends`/`place_path_sep_last_two` from
/// `app_settings` once per query call (not per row) — mirrors
/// `devices_sqlite.rs::read_path_display_separators` (same guarded-read shape
/// as `low_stock` above). Falls back to the V039-seeded defaults
/// (`" // "`/`" / "`) if the row is somehow missing.
fn read_path_display_separators(conn: &Connection) -> (String, String) {
    let read_sep = |key: &str, default: &str| -> String {
        conn.query_row(
            "SELECT value FROM app_settings WHERE key = ?1",
            [key],
            |r| r.get::<_, String>(0),
        )
        .ok()
        .unwrap_or_else(|| default.to_string())
    };
    (
        read_sep("place_path_sep_ends", " // "),
        read_sep("place_path_sep_last_two", " / "),
    )
}

/// Wraps `map_row`, additionally reading `place_effective_variant.effective_variant`
/// (column index 18, present only when the query joins `place_effective_variant` —
/// see `SELECT_CARTRIDGES`) and computing `place_path_short` via `shorten_place_path`.
/// Used ONLY by `list()` (PLC-08); a cartridge with `place_id IS NULL` naturally
/// yields `effective_variant: None` (LEFT JOIN, no match) and thus
/// `place_path_short: None`, mirroring `full_path`'s existing behavior.
fn map_row_with_short_path<'a>(
    sep_ends: &'a str,
    sep_last_two: &'a str,
) -> impl Fn(&rusqlite::Row<'_>) -> rusqlite::Result<CartridgeRow> + 'a {
    move |row| {
        let mut cartridge = map_row(row)?;
        let effective_variant: Option<String> = row.get(18)?;
        if let (Some(full), Some(token)) = (cartridge.full_path.as_deref(), effective_variant) {
            if let Ok(variant) = PathDisplayVariant::from_str(&token) {
                cartridge.place_path_short =
                    Some(shorten_place_path(full, variant, sep_ends, sep_last_two));
            }
        }
        Ok(cartridge)
    }
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
    ///   (`cartridge_seq`→`C-NNNN` / `drum_seq`→`D-NNNN`) in a retry loop
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
        // D-NNNN из drum_seq; картриджи (kind 1) → C-NNNN из cartridge_seq.
        let (counter_name, prefix) = if kind_id == 2 {
            ("drum_seq", 'D')
        } else {
            ("cartridge_seq", 'C')
        };

        // Auto-code: increment counter + retry loop (counter never lost on collision).
        loop {
            let seq = increment_counter_in_tx(tx, counter_name)?;
            let candidate = format!("{prefix}-{seq:04}");
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
    /// `place_id` is written directly — the caller (`CartridgeService`) has
    /// already validated it against the `places` tree (D-13); no implicit
    /// auto-create round-trip.
    #[allow(clippy::too_many_arguments)]
    pub fn insert_cartridge_in_tx(
        &self,
        tx: &Transaction<'_>,
        code: &str,
        model_id: i64,
        status_id: i64,
        state_id: Option<i64>,
        place_id: Option<i64>,
        holder_name: Option<&str>,
        notes: Option<&str>,
        now_utc: i64,
    ) -> Result<i64, AppError> {
        tx.execute(
            "INSERT INTO cartridges \
             (code, model_id, status_id, state_id, place_id, holder_name, notes, \
              created_at_utc, updated_at_utc, version) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8, 1)",
            params![
                code,
                model_id,
                status_id,
                state_id,
                place_id,
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

    /// Upsert compatibility printer names for a cartridge model inside a
    /// transaction.
    ///
    /// Deletes all existing rows for `model_id`, then inserts the provided
    /// `names` (free-text printer names, e.g. "Pantum BM5100ADN"). Empty
    /// `names` → effectively clears compatibility (D-05 pass-through then
    /// applies to the cartridge-selection filter). Values are stored exactly
    /// as given — TRIM/case-insensitive normalisation is applied at
    /// comparison time in `list()`/`compatible_model_aggregates`, not at
    /// write time (D-02/D-03/D-04).
    pub fn upsert_compatibility_in_tx(
        &self,
        tx: &Transaction<'_>,
        model_id: i64,
        names: &[String],
    ) -> Result<(), AppError> {
        tx.execute(
            "DELETE FROM cartridge_model_compatibility WHERE cartridge_model_id = ?1",
            params![model_id],
        )
        .map_err(map_rusqlite)?;

        for name in names {
            tx.execute(
                "INSERT INTO cartridge_model_compatibility \
                 (cartridge_model_id, printer_name) VALUES (?1, ?2)",
                params![model_id, name],
            )
            .map_err(map_rusqlite)?;
        }
        Ok(())
    }

    /// Fetch compatibility printer names for a cartridge model (read-only).
    pub fn get_compatibility(
        &self,
        conn: &Connection,
        model_id: i64,
    ) -> Result<Vec<String>, AppError> {
        let mut stmt = conn
            .prepare(
                "SELECT printer_name \
                 FROM cartridge_model_compatibility \
                 WHERE cartridge_model_id = ?1 \
                 ORDER BY printer_name ASC",
            )
            .map_err(map_rusqlite)?;
        let rows = stmt
            .query_map(params![model_id], |r| r.get::<_, String>(0))
            .map_err(map_rusqlite)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(map_rusqlite)?);
        }
        Ok(out)
    }

    /// Aggregate counts (by status) for every cartridge model compatible
    /// with `printer_device_id`, matching `cartridge_model_compatibility
    /// .printer_name` against `devices.name` (case-insensitive, TRIM'd).
    ///
    /// NOTE — does NOT apply the D-05 pass-through used by `list()`'s
    /// `compatible_with_printer_device_id` filter: a model with zero
    /// compatibility rows matching this printer's name is simply absent
    /// from the result (R4/D-07), so the printer card can render "Нет
    /// совместимых моделей картриджей." when appropriate. Pass-through is
    /// scoped strictly to the cartridge-selection filter in `list()`.
    ///
    /// NOTE (WR-03) — the `in_stock`/`at_refill`/`in_use` sums are RAW status
    /// counts (status_id 1/3/2), NOT installable counts: `state_id` is
    /// intentionally ignored so the figures match the "На складе/На заправке/
    /// В работе" UI labels. A drum (kind_id=2) with status=1 state=6
    /// (Отработанный) is counted in `in_stock` even though it cannot be
    /// installed. See `CompatibleModelAggregate`'s doc for rationale.
    pub fn compatible_model_aggregates(
        &self,
        conn: &Connection,
        printer_device_id: i64,
    ) -> Result<Vec<CompatibleModelAggregate>, AppError> {
        // WR-01/IN-01: the device-name lookup lives entirely inside the
        // EXISTS subquery (no outer JOIN on devices). The device join is
        // scoped to live printers only (type_id = 2 = Принтер,
        // deleted_at_utc IS NULL) so a non-printer or soft-deleted device id
        // whose name happens to collide with a compatibility entry can never
        // produce a false-positive match. This mirrors `list()`'s subquery
        // shape and `suggest_compat_printer`'s suggestion source.
        let sql = "
            SELECT m.id, m.brand, m.model,
                   COALESCE(SUM(CASE WHEN c.status_id = 1 THEN 1 ELSE 0 END), 0) AS in_stock,
                   COALESCE(SUM(CASE WHEN c.status_id = 3 THEN 1 ELSE 0 END), 0) AS at_refill,
                   COALESCE(SUM(CASE WHEN c.status_id = 2 THEN 1 ELSE 0 END), 0) AS in_use
              FROM cartridge_models m
              LEFT JOIN cartridges c
                     ON c.model_id = m.id AND c.deleted_at_utc IS NULL
             WHERE m.deleted_at_utc IS NULL
               AND EXISTS (
                     SELECT 1 FROM cartridge_model_compatibility cmc
                       JOIN devices d ON d.id = ?1
                                     AND d.type_id = 2
                                     AND d.deleted_at_utc IS NULL
                      WHERE cmc.cartridge_model_id = m.id
                        AND LOWER(TRIM(cmc.printer_name)) = LOWER(TRIM(d.name))
                   )
             GROUP BY m.id, m.brand, m.model
             ORDER BY m.brand ASC, m.model ASC
        ";

        let mut stmt = conn.prepare(sql).map_err(map_rusqlite)?;
        let rows = stmt
            .query_map(params![printer_device_id], |r| {
                Ok(CompatibleModelAggregate {
                    model_id: r.get(0)?,
                    brand: r.get(1)?,
                    model: r.get(2)?,
                    in_stock: r.get(3)?,
                    at_refill: r.get(4)?,
                    in_use: r.get(5)?,
                })
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
    ///   3. UPDATE cartridges (status_id, state_id, place_id, holder_name, version).
    ///   4. Insert audit_log row with before/after snapshots + payload.
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
        let (new_state_id, new_place_id, new_holder_name) = match op {
            CartridgeTransitionOp::Install {
                place_id,
                given_to_name,
                ..
            } => (current.state_id, *place_id, Some(given_to_name.as_str())),
            CartridgeTransitionOp::ReturnToStock {
                state_id, place_id, ..
            } => (Some(*state_id), *place_id, None),
            CartridgeTransitionOp::ToRefill {
                place_id,
                given_to_name,
                ..
            } => (current.state_id, *place_id, Some(given_to_name.as_str())),
            CartridgeTransitionOp::FromRefill {
                state_id, place_id, ..
            } => (Some(*state_id), *place_id, None),
            CartridgeTransitionOp::WriteOff { .. } => (current.state_id, current.place_id, None),
        };

        // 5. UPDATE cartridges (optimistic lock on version). current_printer_device_id
        // (D-19) is folded into the SAME UPDATE/SET clause so the optimistic-lock
        // WHERE stays in a single place: Install sets it to the target printer
        // (or NULL for the legacy cartridge-centric D-08 path with no printer);
        // every other op (ReturnToStock/ToRefill/FromRefill/WriteOff) clears it to
        // NULL — a cartridge leaving "В работе" must not keep a stale printer link.
        let install_printer_device_id = match op {
            CartridgeTransitionOp::Install {
                printer_device_id, ..
            } => *printer_device_id,
            _ => None,
        };

        let affected = tx
            .execute(
                "UPDATE cartridges SET status_id=?1, state_id=?2, place_id=?3, \
                 holder_name=?4, current_printer_device_id=?5, \
                 updated_at_utc=?6, version=version+1 \
                 WHERE id=?7 AND version=?8",
                params![
                    new_status_id,
                    new_state_id,
                    new_place_id,
                    new_holder_name,
                    install_printer_device_id,
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

        // 5b. D-16/D-17: if installing into a printer that already has another
        // cartridge "В работе", auto-return that previous cartridge to stock in
        // the SAME transaction (DISC-06) — records an INVERTED actor from the
        // new install's given_by/given_to (GAP-12-12, see below). printer_device_id=None
        // (D-08 legacy cartridge-centric entry) performs no lookup — no regression.
        if let CartridgeTransitionOp::Install {
            printer_device_id: Some(pid),
            previous_cartridge_state_id,
            previous_cartridge_place_id,
            given_by_name: install_given_by_name,
            given_to_name: install_given_to_name,
            ..
        } = op
        {
            let resolved_place_id = *previous_cartridge_place_id;

            let previous: Option<(i64, i64)> = tx
                .query_row(
                    "SELECT id, version FROM cartridges \
                     WHERE current_printer_device_id = ?1 AND status_id = 2 \
                       AND id != ?2 AND deleted_at_utc IS NULL \
                     LIMIT 1",
                    params![pid, cartridge_id],
                    |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)),
                )
                .optional()
                .map_err(map_rusqlite)?;

            if let Some((prev_id, prev_version)) = previous {
                // Snapshot before mutating, for the auto-return's audit before_json.
                let prev_current = self.fetch_in_tx(tx, prev_id)?;

                // R7 (13-SPEC.md): kind-aware default — фотобарабан (kind_id=2)
                // возвращается в state 5 «Изношенный», обычный картридж
                // (kind_id=1) — в state 3 «На заправке» (D-10/D-11/D-12), если
                // previous_cartridge_state_id не передан явно.
                let resolved_state_id = previous_cartridge_state_id.unwrap_or_else(|| {
                    if prev_current.model_kind_id == Some(2) {
                        5
                    } else {
                        3
                    }
                });

                let prev_affected = tx
                    .execute(
                        "UPDATE cartridges SET status_id=1, state_id=?1, place_id=?2, \
                         holder_name=NULL, current_printer_device_id=NULL, \
                         updated_at_utc=?3, version=version+1 \
                         WHERE id=?4 AND version=?5",
                        params![
                            resolved_state_id,
                            resolved_place_id,
                            now_utc,
                            prev_id,
                            prev_version
                        ],
                    )
                    .map_err(map_rusqlite)?;

                if prev_affected == 0 {
                    return Err(AppError::OptimisticLockMismatch {
                        entity: "cartridge",
                        id: prev_id,
                        expected: prev_version,
                        actual: prev_version + 1,
                    });
                }

                let prev_before_json = serde_json::to_string(&json!({
                    "status_id": prev_current.status_id,
                    "status_name": prev_current.status_name,
                    "state_id": prev_current.state_id,
                    "state_name": prev_current.state_name,
                    "place_id": prev_current.place_id,
                    "holder_name": prev_current.holder_name,
                }))
                .map_err(|e| AppError::Internal {
                    source_chain: format!("auto-return before_json serialize: {e}"),
                })?;

                let auto_return_op = CartridgeTransitionOp::ReturnToStock {
                    state_id: resolved_state_id,
                    place_id: resolved_place_id,
                    notes: None,
                };
                // GAP-12-12: record an INVERTED actor in the auto-return's own
                // payload_json — the new install's given_to_name (the
                // recipient of the new cartridge) is the one who *hands back*
                // the previous cartridge, and the new install's given_by_name
                // (the issuer/warehouse) is the one who *receives* it. Keys
                // are exactly given_by_name/given_to_name so the existing
                // history UI (CartridgeDetail.svelte parsePayload) renders
                // them without changes. ReturnToStock has no actor fields by
                // design — op_payload_json() (used for direct, user-initiated
                // returns) is intentionally left untouched.
                let prev_payload_json = json!({
                    "op": "return_to_stock",
                    "state_id": resolved_state_id,
                    "place_id": resolved_place_id,
                    "notes": null,
                    "given_by_name": install_given_to_name,
                    "given_to_name": install_given_by_name,
                })
                .to_string();

                let audit_repo = SqliteAuditLogRepository;
                audit_repo.insert(
                    tx,
                    AuditEntry {
                        entity_type: "cartridge",
                        entity_id: prev_id,
                        action: auto_return_op.audit_action(),
                        user_id: None,
                        before_json: Some(prev_before_json),
                        after_json: None,
                        payload_json: Some(prev_payload_json),
                        created_at_utc: now_utc,
                    },
                )?;
            }
        }

        // 6. Build payload_json for audit (D-History-01).
        let payload_json = Self::op_payload_json(op);

        // 7. Before snapshot (for history display).
        let before_json = serde_json::to_string(&json!({
            "status_id": current.status_id,
            "status_name": current.status_name,
            "state_id": current.state_id,
            "state_name": current.state_name,
            "place_id": current.place_id,
            "holder_name": current.holder_name,
        }))
        .map_err(|e| AppError::Internal {
            source_chain: format!("before_json serialize: {e}"),
        })?;

        // 8. Audit log insert.
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
                place_id,
                ..
            } => json!({
                "op": "install",
                "date_utc": date_utc,
                "given_by_name": given_by_name,
                "given_to_name": given_to_name,
                "place_id": place_id,
            }),
            CartridgeTransitionOp::ReturnToStock {
                state_id,
                place_id,
                notes,
            } => json!({
                "op": "return_to_stock",
                "state_id": state_id,
                "place_id": place_id,
                "notes": notes,
            }),
            CartridgeTransitionOp::ToRefill {
                date_utc,
                given_by_name,
                given_to_name,
                place_id,
            } => json!({
                "op": "to_refill",
                "date_utc": date_utc,
                "given_by_name": given_by_name,
                "given_to_name": given_to_name,
                "place_id": place_id,
            }),
            CartridgeTransitionOp::FromRefill {
                state_id,
                place_id,
                notes,
            } => json!({
                "op": "from_refill",
                "state_id": state_id,
                "place_id": place_id,
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

    // -----------------------------------------------------------------------
    // Public read-only helpers (called from service layer via ReaderPool)
    // -----------------------------------------------------------------------

    /// FTS5 + LIKE + place-path search over cartridges (CART-11, D-Search-01,
    /// D-29/PLC-05).
    ///
    /// UNION CTE: `fts_hits` (FTS5 MATCH on cartridges_fts) UNION `like_hits`
    /// (LIKE on code, holder_name + model brand/model via JOIN) UNION
    /// `place_hits` (cartridges whose `place_id` is one of a Rust-computed
    /// candidate set — see below).
    ///
    /// D-29/PLC-05: place-path substring matching is computed in Rust —
    /// `place_full_paths` is fetched once and compared with `.to_lowercase()`
    /// (RESEARCH Common Pitfall 2: Cyrillic substring matching must never go
    /// through SQL LIKE/GLOB). Read live on every call: a place rename/move
    /// is reflected on the very next `search` call, no reindex step.
    ///
    /// Security: FTS MATCH parameter is passed via `params![]` — not concatenated.
    /// Double-quotes in `query` are escaped before MATCH to avoid FTS syntax errors.
    /// Resolved `place_id`s are bound as parameterized placeholders, never
    /// interpolated as query text.
    pub fn search(
        &self,
        conn: &Connection,
        query: &str,
        filter: &CartridgeFilter,
    ) -> Result<Vec<CartridgeRow>, AppError> {
        use rusqlite::types::ToSql;

        // Guard: FTS5 MATCH on a phrase with no alphanumeric tokens (e.g. "---",
        // a lone double-quote, or punctuation-only input) can return SQLITE_ERROR
        // on some unicode61 builds. When the query has no alphanumeric chars,
        // skip the fts_hits CTE and fall back to LIKE-only (WR-01).
        let has_token = query.chars().any(|c| c.is_alphanumeric());

        let like_query = format!("%{}%", query);

        // Compute the place-path candidate set BEFORE building either branch's
        // SQL — empty query means no place candidates (mirrors like_query's
        // empty-input handling).
        let query_lower = query.trim().to_lowercase();
        let place_ids: Vec<i64> = if query_lower.is_empty() {
            Vec::new()
        } else {
            let mut stmt = conn
                .prepare("SELECT place_id, full_path FROM place_full_paths")
                .map_err(map_rusqlite)?;
            let rows = stmt
                .query_map([], |r| {
                    let id: i64 = r.get(0)?;
                    let path: String = r.get(1)?;
                    Ok((id, path))
                })
                .map_err(map_rusqlite)?;
            let mut ids = Vec::new();
            for row in rows {
                let (id, path) = row.map_err(map_rusqlite)?;
                if path.to_lowercase().contains(&query_lower) {
                    ids.push(id);
                }
            }
            ids
        };
        let has_place = !place_ids.is_empty();

        // place_hits placeholders start after the 4 fixed params (?1 like_query,
        // ?2 status_id, ?3 kind_id, ?4 model_id).
        let place_placeholders: Vec<String> = place_ids
            .iter()
            .enumerate()
            .map(|(i, _)| format!("?{}", 5 + i))
            .collect();
        let place_hits_cte = if has_place {
            format!(
                ", place_hits AS (SELECT c.id FROM cartridges c WHERE c.place_id IN ({}))",
                place_placeholders.join(",")
            )
        } else {
            String::new()
        };
        let place_union = if has_place {
            " UNION SELECT id FROM place_hits"
        } else {
            ""
        };

        let sql = if has_token {
            // Escape double-quotes in FTS query to avoid FTS5 syntax errors (T-04-02-01).
            let fts_query_escaped = query.replace('"', "\"\"");
            format!(
                "WITH fts_hits AS ( \
                   SELECT f.rowid AS id FROM cartridges_fts f \
                   WHERE cartridges_fts MATCH '\"{fts_query_escaped}\"*' \
                 ), \
                 like_hits AS ( \
                   SELECT c.id FROM cartridges c \
                   LEFT JOIN cartridge_models m ON m.id = c.model_id \
                   WHERE c.code LIKE ?1 \
                      OR c.holder_name LIKE ?1 \
                      OR m.brand LIKE ?1 \
                      OR m.model LIKE ?1 \
                 ){place_hits_cte} \
                 {SELECT_CARTRIDGES} \
                 WHERE c.id IN (SELECT id FROM fts_hits UNION SELECT id FROM like_hits{place_union}) \
                   AND c.deleted_at_utc IS NULL \
                   AND (?2 IS NULL OR c.status_id = ?2) \
                   AND (?3 IS NULL OR m.kind_id = ?3) \
                   AND (?4 IS NULL OR c.model_id = ?4) \
                 ORDER BY c.created_at_utc DESC, c.id DESC \
                 LIMIT 200"
            )
        } else {
            format!(
                "WITH like_hits AS ( \
                   SELECT c.id FROM cartridges c \
                   LEFT JOIN cartridge_models m ON m.id = c.model_id \
                   WHERE c.code LIKE ?1 \
                      OR c.holder_name LIKE ?1 \
                      OR m.brand LIKE ?1 \
                      OR m.model LIKE ?1 \
                 ){place_hits_cte} \
                 {SELECT_CARTRIDGES} \
                 WHERE c.id IN (SELECT id FROM like_hits{place_union}) \
                   AND c.deleted_at_utc IS NULL \
                   AND (?2 IS NULL OR c.status_id = ?2) \
                   AND (?3 IS NULL OR m.kind_id = ?3) \
                   AND (?4 IS NULL OR c.model_id = ?4) \
                 ORDER BY c.created_at_utc DESC, c.id DESC \
                 LIMIT 200"
            )
        };

        let mut bind_params: Vec<Box<dyn ToSql>> = vec![
            Box::new(like_query),
            Box::new(filter.status_id),
            Box::new(filter.kind_id),
            Box::new(filter.model_id),
        ];
        for id in &place_ids {
            bind_params.push(Box::new(*id));
        }
        let param_refs: Vec<&dyn ToSql> = bind_params.iter().map(|b| b.as_ref()).collect();

        let mut stmt = conn.prepare(&sql).map_err(map_rusqlite)?;
        let rows = stmt
            .query_map(param_refs.as_slice(), map_row)
            .map_err(map_rusqlite)?;

        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(map_rusqlite)?);
        }
        Ok(out)
    }

    /// Low-stock query (CART-12, D-LowStock-02, quick task 260819-wq5).
    ///
    /// Threshold is read from `app_settings.low_stock_threshold` (default 2).
    /// Basis (grouping key) is read from `app_settings.low_stock_basis`
    /// (default `LowStockBasis::PrinterModel` — see [`LowStockBasis`]):
    ///
    /// - `CartridgeModel`: legacy behavior — group by `cartridge_models.id`,
    ///   `count(in_stock AND full) < threshold`.
    /// - `PrinterModel`: group by printer name sourced strictly from
    ///   `cartridge_model_compatibility.printer_name` (normalized via
    ///   `LOWER(TRIM(...))`), summing in-stock+full cartridges across every
    ///   cartridge model compatible with that printer name. Models with no
    ///   compatibility rows never appear in any group (no D-05 pass-through
    ///   here — pass-through is scoped strictly to `list()`'s cartridge-
    ///   selection filter). A printer name with zero matching stock is still
    ///   included (0 < threshold is a real supply gap).
    ///
    /// WR-06: `CAST(value AS INTEGER)` in SQLite silently converts non-numeric
    /// strings to 0, bypassing the `unwrap_or(2)` fallback. Instead, read the
    /// raw string value and parse it in Rust with an explicit > 0 guard so a
    /// malformed setting always falls back to the intended default of 2. The
    /// same guarded-read shape is mirrored for `basis` below.
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

        let basis: LowStockBasis = conn
            .query_row(
                "SELECT value FROM app_settings WHERE key = 'low_stock_basis'",
                [],
                |r| r.get::<_, String>(0),
            )
            .ok()
            .and_then(|s| LowStockBasis::parse(s.trim()))
            .unwrap_or(LowStockBasis::DEFAULT);

        match basis {
            LowStockBasis::CartridgeModel => {
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
                        basis: LowStockBasis::CartridgeModel,
                        model_id: Some(model_id),
                        brand: Some(brand.clone()),
                        model: Some(model.clone()),
                        label: format!("{brand} {model}"),
                        count,
                        threshold,
                    });
                }
                Ok(out)
            }
            LowStockBasis::PrinterModel => {
                // Anti-fan-out via correlated EXISTS subquery — mirrors the
                // pattern in `compatible_model_aggregates` above: a direct
                // JOIN through `cartridge_model_compatibility` would multiply-
                // count cartridges whenever multiple compatibility rows
                // reference the same (model, printer_name) pair. This counts
                // each cartridge exactly once regardless of how many
                // compatibility rows exist.
                let sql = "
                    SELECT display_name, cnt
                      FROM (
                        SELECT pg.display_name AS display_name,
                               (
                                 SELECT COUNT(*)
                                   FROM cartridges c
                                   JOIN cartridge_models m ON m.id = c.model_id AND m.deleted_at_utc IS NULL
                                  WHERE c.status_id = 1 AND c.state_id = 1 AND c.deleted_at_utc IS NULL
                                    AND EXISTS (
                                          SELECT 1 FROM cartridge_model_compatibility cmc2
                                           WHERE cmc2.cartridge_model_id = m.id
                                             AND LOWER(TRIM(cmc2.printer_name)) = pg.norm_name
                                        )
                               ) AS cnt
                          FROM (
                                SELECT LOWER(TRIM(printer_name)) AS norm_name,
                                       MIN(printer_name) AS display_name
                                  FROM cartridge_model_compatibility
                                 GROUP BY LOWER(TRIM(printer_name))
                               ) pg
                      )
                     WHERE cnt < ?1
                     ORDER BY cnt ASC, display_name ASC
                ";

                let mut stmt = conn.prepare(sql).map_err(map_rusqlite)?;
                let rows = stmt
                    .query_map(params![threshold], |r| {
                        Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
                    })
                    .map_err(map_rusqlite)?;

                let mut out = Vec::new();
                for row in rows {
                    let (display_name, count) = row.map_err(map_rusqlite)?;
                    out.push(LowStockItem {
                        basis: LowStockBasis::PrinterModel,
                        model_id: None,
                        brand: None,
                        model: None,
                        label: display_name,
                        count,
                        threshold,
                    });
                }
                Ok(out)
            }
        }
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
                        OR NOT EXISTS (SELECT 1 FROM cartridge_model_compatibility cmc WHERE cmc.cartridge_model_id = c.model_id) \
                        OR EXISTS (SELECT 1 FROM cartridge_model_compatibility cmc \
                                   JOIN devices d ON d.id = ?6 AND d.type_id = 2 AND d.deleted_at_utc IS NULL \
                                   WHERE cmc.cartridge_model_id = c.model_id \
                                     AND LOWER(TRIM(cmc.printer_name)) = LOWER(TRIM(d.name))))",
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
                        OR NOT EXISTS (SELECT 1 FROM cartridge_model_compatibility cmc WHERE cmc.cartridge_model_id = c.model_id) \
                        OR EXISTS (SELECT 1 FROM cartridge_model_compatibility cmc \
                                   JOIN devices d ON d.id = ?6 AND d.type_id = 2 AND d.deleted_at_utc IS NULL \
                                   WHERE cmc.cartridge_model_id = c.model_id \
                                     AND LOWER(TRIM(cmc.printer_name)) = LOWER(TRIM(d.name)))) \
                 ORDER BY c.created_at_utc DESC, c.id DESC \
                 LIMIT ?7 OFFSET ?8"
            ))
            .map_err(map_rusqlite)?;

        let (sep_ends, sep_last_two) = read_path_display_separators(conn);
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
                map_row_with_short_path(&sep_ends, &sep_last_two),
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

    /// Сид места (places), для FK-валидного `cartridges.place_id` в тестах,
    /// которые действительно проверяют значение места (D-13).
    fn seed_place(conn: &mut Connection, name: &str) -> i64 {
        let now = 1_700_000_000_i64;
        conn.execute(
            "INSERT INTO places (kind, name, is_storage, created_at_utc, updated_at_utc, version) \
             VALUES ('room', ?1, 0, ?2, ?2, 1)",
            params![name, now],
        )
        .expect("insert place");
        conn.last_insert_rowid()
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

    /// Сид устройства (type_id=2 — Принтер) — возвращает device_id, на
    /// которое cartridges.current_printer_device_id может ссылаться (FK).
    fn seed_device(conn: &mut Connection) -> i64 {
        let now = 1_700_000_000_i64;
        conn.execute(
            "INSERT INTO devices (type_id, name, status_id, created_at_utc, updated_at_utc, version) \
             VALUES (2, 'Test Printer', 1, ?1, ?1, 1)",
            params![now],
        )
        .expect("insert device");
        conn.last_insert_rowid()
    }

    /// Сид модели фотобарабана (kind_id=2) — для R7 kind-aware auto-return тестов.
    fn seed_drum_model(conn: &mut Connection, brand: &str, model: &str) -> i64 {
        let tx = conn.transaction().expect("tx");
        let now = 1_700_000_000_i64;
        let repo = SqliteCartridgeRepository;
        let id = repo
            .insert_model_in_tx(
                &tx,
                &CartridgeModelNew {
                    brand: brand.into(),
                    model: model.into(),
                    kind_id: 2,
                    color: None,
                    notes: None,
                },
                now,
            )
            .expect("insert drum model");
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
        assert_eq!(code1, "C-0001");
        tx.commit().expect("commit");

        let tx2 = conn.transaction().expect("tx2");
        let (code2, _) =
            SqliteCartridgeRepository::assign_code_in_tx(&tx2, None, 1, now).expect("code2");
        assert_eq!(code2, "C-0002");
        tx2.commit().expect("commit");
    }

    #[test]
    fn assign_code_drum_uses_d_prefix_and_separate_counter() {
        // UAT round 3 №4: фотобарабаны (kind 2) получают код D-NNNN из
        // отдельного счётчика drum_seq, не конфликтуя с C-NNNN картриджей.
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

        assert_eq!(c_code, "C-0001");
        assert_eq!(d_code, "D-0001");
        assert_eq!(d_code2, "D-0002");
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
        let place_id = seed_place(&mut conn, "Склад");
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
                    Some(place_id),
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
        assert_eq!(row.place_id, Some(place_id));
        assert_eq!(row.full_path.as_deref(), Some("Склад"));
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
        let warehouse_place_id = seed_place(&mut conn, "Склад");
        let office_place_id = seed_place(&mut conn, "Каб. 305");
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
                    Some(warehouse_place_id),
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
            place_id: Some(office_place_id),
            printer_device_id: None,
            previous_cartridge_state_id: None,
            previous_cartridge_place_id: None,
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
        assert_eq!(row.place_id, Some(office_place_id));
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
            place_id: None,
            notes: None,
        };

        let tx = conn.transaction().expect("tx");
        let err = repo
            .transition_in_tx(&tx, cart_id, 1, &op, now)
            .expect_err("should fail");
        assert!(matches!(err, AppError::Validation { .. }), "got {err:?}");
    }

    #[test]
    fn auto_return_uses_kind_aware_default_state_for_drum() {
        // R7 regression: устанавливают фотобарабан (kind_id=2) на принтер,
        // где уже "В работе" другой фотобарабан, без явного
        // previous_cartridge_state_id → авто-возвращённый предыдущий
        // фотобарабан должен получить state_id=5 («Изношенный»), НЕ 3.
        let (mut conn, _g) = fresh_conn();
        let drum_model = seed_drum_model(&mut conn, "Pantum", "DL-5120");
        let printer_device_id = seed_device(&mut conn);
        let repo = SqliteCartridgeRepository;
        let now = 1_700_000_000_i64;

        // Первый фотобарабан — установлен "В работе" на printer_device_id.
        let prev_id = {
            let tx = conn.transaction().expect("tx");
            let (code, _) =
                SqliteCartridgeRepository::assign_code_in_tx(&tx, None, 2, now).expect("code");
            let id = repo
                .insert_cartridge_in_tx(&tx, &code, drum_model, 1, Some(4), None, None, None, now)
                .expect("insert prev drum");
            tx.commit().expect("commit");
            id
        };
        {
            let install_prev = CartridgeTransitionOp::Install {
                date_utc: now,
                given_by_name: "Иванов".into(),
                given_to_name: "Петров".into(),
                place_id: None,
                printer_device_id: Some(printer_device_id),
                previous_cartridge_state_id: None,
                previous_cartridge_place_id: None,
            };
            let tx = conn.transaction().expect("tx");
            repo.transition_in_tx(&tx, prev_id, 1, &install_prev, now)
                .expect("install prev drum");
            tx.commit().expect("commit");
        }

        // Второй фотобарабан — устанавливается на тот же принтер, без
        // явного previous_cartridge_state_id → должен авто-вернуть первый.
        let new_id = {
            let tx = conn.transaction().expect("tx");
            let (code, _) =
                SqliteCartridgeRepository::assign_code_in_tx(&tx, None, 2, now).expect("code2");
            let id = repo
                .insert_cartridge_in_tx(&tx, &code, drum_model, 1, Some(4), None, None, None, now)
                .expect("insert new drum");
            tx.commit().expect("commit");
            id
        };
        {
            let install_new = CartridgeTransitionOp::Install {
                date_utc: now,
                given_by_name: "Сидоров".into(),
                given_to_name: "Кузнецов".into(),
                place_id: None,
                printer_device_id: Some(printer_device_id),
                previous_cartridge_state_id: None,
                previous_cartridge_place_id: None,
            };
            let tx = conn.transaction().expect("tx");
            repo.transition_in_tx(&tx, new_id, 1, &install_new, now)
                .expect("install new drum");
            tx.commit().expect("commit");
        }

        let prev_row = repo
            .get(&conn, prev_id)
            .expect("get prev after auto-return");
        assert_eq!(prev_row.status_id, 1); // На складе (auto-returned)
        assert_eq!(prev_row.state_id, Some(5)); // Изношенный — NOT 3
    }

    #[test]
    fn auto_return_keeps_state_3_default_for_regular_cartridge() {
        // Non-regression: обычный картридж (kind_id=1) продолжает
        // авто-возвращаться в state_id=3 («На заправке») по умолчанию.
        let (mut conn, _g) = fresh_conn();
        let model_id = seed_model(&mut conn, "Pantum", "TL-5120X");
        let printer_device_id = seed_device(&mut conn);
        let repo = SqliteCartridgeRepository;
        let now = 1_700_000_000_i64;

        let prev_id = {
            let tx = conn.transaction().expect("tx");
            let (code, _) =
                SqliteCartridgeRepository::assign_code_in_tx(&tx, None, 1, now).expect("code");
            let id = repo
                .insert_cartridge_in_tx(&tx, &code, model_id, 1, Some(1), None, None, None, now)
                .expect("insert prev cartridge");
            tx.commit().expect("commit");
            id
        };
        {
            let install_prev = CartridgeTransitionOp::Install {
                date_utc: now,
                given_by_name: "Иванов".into(),
                given_to_name: "Петров".into(),
                place_id: None,
                printer_device_id: Some(printer_device_id),
                previous_cartridge_state_id: None,
                previous_cartridge_place_id: None,
            };
            let tx = conn.transaction().expect("tx");
            repo.transition_in_tx(&tx, prev_id, 1, &install_prev, now)
                .expect("install prev cartridge");
            tx.commit().expect("commit");
        }

        let new_id = {
            let tx = conn.transaction().expect("tx");
            let (code, _) =
                SqliteCartridgeRepository::assign_code_in_tx(&tx, None, 1, now).expect("code2");
            let id = repo
                .insert_cartridge_in_tx(&tx, &code, model_id, 1, Some(1), None, None, None, now)
                .expect("insert new cartridge");
            tx.commit().expect("commit");
            id
        };
        {
            let install_new = CartridgeTransitionOp::Install {
                date_utc: now,
                given_by_name: "Сидоров".into(),
                given_to_name: "Кузнецов".into(),
                place_id: None,
                printer_device_id: Some(printer_device_id),
                previous_cartridge_state_id: None,
                previous_cartridge_place_id: None,
            };
            let tx = conn.transaction().expect("tx");
            repo.transition_in_tx(&tx, new_id, 1, &install_new, now)
                .expect("install new cartridge");
            tx.commit().expect("commit");
        }

        let prev_row = repo
            .get(&conn, prev_id)
            .expect("get prev after auto-return");
        assert_eq!(prev_row.status_id, 1); // На складе (auto-returned)
        assert_eq!(prev_row.state_id, Some(3)); // На заправке — unchanged behavior
    }

    /// GAP-2 (SPEC-13-R4): `compatible_model_aggregates` returns RAW per-status
    /// counts (status_id 1/3/2 → in_stock/at_refill/in_use, state_id ignored)
    /// for every model whose compatibility list matches the printer's
    /// `devices.name` (LOWER(TRIM) on both sides). A model with NO matching
    /// compatibility row is ABSENT (no D-05 pass-through), and soft-deleted
    /// cartridges are not counted.
    #[test]
    fn compatible_model_aggregates_counts_raw_statuses_and_omits_unmatched() {
        let (mut conn, _g) = fresh_conn();
        // seed_device hardcodes name 'Test Printer', type_id=2.
        let printer_device_id = seed_device(&mut conn);
        let repo = SqliteCartridgeRepository;
        let now = 1_700_000_000_i64;

        // Model A: compatible with the printer. Use a case/whitespace variant of
        // the device name to also exercise the LOWER(TRIM) match.
        let model_a = seed_model(&mut conn, "Pantum", "TL-5120X");
        // Model B: also compatible — proves ORDER BY brand ASC (Cactus < Pantum)
        // and that multiple compatible models are returned independently.
        let model_b = seed_model(&mut conn, "Cactus", "CS-TL5120");
        // Model C: NO matching compatibility row → must be ABSENT (not zero-count).
        let model_c = seed_model(&mut conn, "Hewlett", "HP-999");

        {
            let tx = conn.transaction().expect("tx");
            repo.upsert_compatibility_in_tx(&tx, model_a, &["  test printer  ".to_string()])
                .expect("compat A");
            repo.upsert_compatibility_in_tx(&tx, model_b, &["Test Printer".to_string()])
                .expect("compat B");
            // Model C: compatibility points at a DIFFERENT printer name → no match.
            repo.upsert_compatibility_in_tx(&tx, model_c, &["Some Other Printer".to_string()])
                .expect("compat C");
            tx.commit().expect("commit");
        }

        // Model A cartridges: 2× status 1 (на складе), 1× status 3 (на заправке),
        // 3× status 2 (в работе). Include a status=1 unit in a "spent" state to
        // prove state_id is ignored in the RAW count. Plus 1 soft-deleted status=1
        // unit that must NOT be counted.
        {
            let tx = conn.transaction().expect("tx");
            for (i, (status, state)) in [
                (1_i64, Some(1_i64)),
                (1, Some(6)), // status=1 but state=6 (spent) — still counted in_stock.
                (3, Some(3)),
                (2, Some(2)),
                (2, Some(2)),
                (2, Some(2)),
            ]
            .iter()
            .enumerate()
            {
                let code = format!("A-{i:03}");
                repo.insert_cartridge_in_tx(
                    &tx, &code, model_a, *status, *state, None, None, None, now,
                )
                .expect("insert A cartridge");
            }
            // Soft-deleted status=1 cartridge for model A — excluded from counts.
            let soft_deleted_id = repo
                .insert_cartridge_in_tx(&tx, "A-DEL", model_a, 1, Some(1), None, None, None, now)
                .expect("insert soft-deleted A cartridge");
            tx.execute(
                "UPDATE cartridges SET deleted_at_utc = ?1 WHERE id = ?2",
                params![now, soft_deleted_id],
            )
            .expect("soft-delete cartridge");
            tx.commit().expect("commit");
        }

        // Model B cartridge: a single status=2 (в работе) unit.
        {
            let tx = conn.transaction().expect("tx");
            repo.insert_cartridge_in_tx(&tx, "B-001", model_b, 2, Some(2), None, None, None, now)
                .expect("insert B cartridge");
            tx.commit().expect("commit");
        }

        // Model C cartridge exists but model C is not compatible → never appears.
        {
            let tx = conn.transaction().expect("tx");
            repo.insert_cartridge_in_tx(&tx, "C-001", model_c, 1, Some(1), None, None, None, now)
                .expect("insert C cartridge");
            tx.commit().expect("commit");
        }

        let aggregates = repo
            .compatible_model_aggregates(&conn, printer_device_id)
            .expect("compatible_model_aggregates");

        // Only model A and model B are compatible → exactly two rows, ordered by
        // brand ASC: "Cactus" (B) before "Pantum" (A). Model C is ABSENT.
        assert_eq!(
            aggregates.len(),
            2,
            "exactly the two compatible models, model C (no match) absent; got {aggregates:?}"
        );
        assert_eq!(
            aggregates[0].model_id, model_b,
            "Cactus sorts first (brand ASC)"
        );
        assert_eq!(aggregates[1].model_id, model_a, "Pantum sorts second");

        // Model B: 0 in_stock, 0 at_refill, 1 in_use.
        let b = &aggregates[0];
        assert_eq!(b.in_stock, 0);
        assert_eq!(b.at_refill, 0);
        assert_eq!(b.in_use, 1);

        // Model A: RAW counts — 2 in_stock (state_id ignored, soft-deleted
        // excluded), 1 at_refill, 3 in_use.
        let a = &aggregates[1];
        assert_eq!(
            a.in_stock, 2,
            "two status=1 units counted (state ignored), soft-deleted excluded"
        );
        assert_eq!(a.at_refill, 1, "one status=3 unit");
        assert_eq!(a.in_use, 3, "three status=2 units");

        // Sanity: none of the compatible rows is model C.
        assert!(
            !aggregates.iter().any(|m| m.model_id == model_c),
            "model C must be absent (no pass-through for zero matching compat rows)"
        );
    }

    #[test]
    fn low_stock_returns_models_below_threshold() {
        let (mut conn, _g) = fresh_conn();
        let model_id = seed_model(&mut conn, "Cactus", "TL-5120P");
        let repo = SqliteCartridgeRepository;
        let now = 1_700_000_000_i64;

        // Default basis is now PrinterModel — explicitly select the legacy
        // cartridge_model basis to keep this test's per-model assertions
        // meaningful (quick task 260819-wq5).
        conn.execute(
            "INSERT INTO app_settings (key, value, created_at_utc, updated_at_utc) \
             VALUES ('low_stock_basis', 'cartridge_model', 0, 0)",
            [],
        )
        .expect("seed basis");

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
        assert_eq!(items[0].basis, LowStockBasis::CartridgeModel);
        assert_eq!(items[0].model_id, Some(model_id));
        assert_eq!(items[0].count, 1);
        assert_eq!(items[0].threshold, 2);
    }

    #[test]
    fn low_stock_basis_parse_rejects_unknown_and_empty() {
        assert_eq!(
            LowStockBasis::parse("cartridge_model"),
            Some(LowStockBasis::CartridgeModel)
        );
        assert_eq!(
            LowStockBasis::parse("printer_model"),
            Some(LowStockBasis::PrinterModel)
        );
        assert_eq!(LowStockBasis::parse(""), None);
        assert_eq!(LowStockBasis::parse("bogus"), None);
    }

    #[test]
    fn low_stock_printer_model_groups_by_compatible_printer_name() {
        let (mut conn, _g) = fresh_conn();
        let model_a = seed_model(&mut conn, "Contoso", "T-100");
        let model_b = seed_model(&mut conn, "Fabrikam", "F-200");
        let repo = SqliteCartridgeRepository;
        let now = 1_700_000_000_i64;

        {
            let tx = conn.transaction().expect("tx");
            repo.upsert_compatibility_in_tx(&tx, model_a, &["Contoso LaserJet 400".to_string()])
                .expect("compat a");
            repo.upsert_compatibility_in_tx(&tx, model_b, &[" contoso laserjet 400 ".to_string()])
                .expect("compat b");
            tx.commit().expect("commit");
        }

        // No app_settings.low_stock_basis row written — proving the default
        // is PrinterModel. Raise the threshold to 3 so the summed count of 2
        // (one full-stock cartridge per model) is still below threshold —
        // the default threshold of 2 would otherwise exclude a cnt=2 group
        // (HAVING cnt < threshold), which is unrelated to what this test
        // exercises (summing across models normalizing to the same printer
        // name).
        conn.execute(
            "INSERT INTO app_settings (key, value, created_at_utc, updated_at_utc) \
             VALUES ('low_stock_threshold', '3', 0, 0) \
             ON CONFLICT(key) DO UPDATE SET value = '3'",
            [],
        )
        .expect("seed threshold");

        {
            let tx = conn.transaction().expect("tx");
            let (code_a, _) =
                SqliteCartridgeRepository::assign_code_in_tx(&tx, None, 1, now).expect("code a");
            repo.insert_cartridge_in_tx(&tx, &code_a, model_a, 1, Some(1), None, None, None, now)
                .expect("insert a");
            let (code_b, _) =
                SqliteCartridgeRepository::assign_code_in_tx(&tx, None, 1, now).expect("code b");
            repo.insert_cartridge_in_tx(&tx, &code_b, model_b, 1, Some(1), None, None, None, now)
                .expect("insert b");
            tx.commit().expect("commit");
        }

        let items = repo.low_stock(&conn).expect("low_stock");
        assert_eq!(
            items.len(),
            1,
            "both models normalize to a single printer-name group"
        );
        assert_eq!(items[0].basis, LowStockBasis::PrinterModel);
        assert_eq!(items[0].model_id, None);
        assert!(
            items[0]
                .label
                .to_lowercase()
                .contains("contoso laserjet 400"),
            "label should reflect the compatible printer name, got {:?}",
            items[0].label
        );
        assert_eq!(items[0].count, 2, "counts from both compatible models sum");
    }

    #[test]
    fn low_stock_printer_model_zero_stock_printer_included() {
        let (mut conn, _g) = fresh_conn();
        let model_a = seed_model(&mut conn, "Northwind", "N-300");
        let repo = SqliteCartridgeRepository;

        {
            let tx = conn.transaction().expect("tx");
            repo.upsert_compatibility_in_tx(&tx, model_a, &["Fabrikam Mono 12".to_string()])
                .expect("compat");
            tx.commit().expect("commit");
        }

        // No cartridges seeded — printer name still has 0 < default threshold 2.
        let items = repo.low_stock(&conn).expect("low_stock");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].basis, LowStockBasis::PrinterModel);
        assert!(items[0].label.to_lowercase().contains("fabrikam mono 12"));
        assert_eq!(items[0].count, 0);
    }

    #[test]
    fn low_stock_falls_back_to_printer_model_default_on_garbage_basis_value() {
        // Plan-checker note: app_settings.low_stock_basis holding a garbage
        // string (not just a missing key) must also fall back to the
        // PrinterModel default (WR-06-style guarded parse), not error out
        // or silently coerce to CartridgeModel.
        let (mut conn, _g) = fresh_conn();
        let model_a = seed_model(&mut conn, "Adatum", "A-500");
        let repo = SqliteCartridgeRepository;

        conn.execute(
            "INSERT INTO app_settings (key, value, created_at_utc, updated_at_utc) \
             VALUES ('low_stock_basis', 'bogus', 0, 0)",
            [],
        )
        .expect("seed garbage basis");

        {
            let tx = conn.transaction().expect("tx");
            repo.upsert_compatibility_in_tx(&tx, model_a, &["Tailspin Color 700".to_string()])
                .expect("compat");
            tx.commit().expect("commit");
        }

        let items = repo.low_stock(&conn).expect("low_stock");
        assert_eq!(items.len(), 1);
        assert_eq!(
            items[0].basis,
            LowStockBasis::PrinterModel,
            "garbage low_stock_basis value must fall back to the PrinterModel default"
        );
        assert!(items[0].label.to_lowercase().contains("tailspin color 700"));
    }

    #[test]
    fn low_stock_printer_model_excludes_model_without_compatibility() {
        let (mut conn, _g) = fresh_conn();
        let model_no_compat = seed_model(&mut conn, "Wingtip", "W-999");
        let repo = SqliteCartridgeRepository;
        let now = 1_700_000_000_i64;

        // Several full-stock cartridges under a model with NO compatibility
        // rows — must never leak into any printer group in printer_model mode.
        {
            let tx = conn.transaction().expect("tx");
            for _ in 0..3 {
                let (code, _) =
                    SqliteCartridgeRepository::assign_code_in_tx(&tx, None, 1, now).expect("code");
                repo.insert_cartridge_in_tx(
                    &tx,
                    &code,
                    model_no_compat,
                    1,
                    Some(1),
                    None,
                    None,
                    None,
                    now,
                )
                .expect("insert");
            }
            tx.commit().expect("commit");
        }

        let items = repo.low_stock(&conn).expect("low_stock");
        assert!(
            !items
                .iter()
                .any(|i| i.label.to_lowercase().contains("wingtip")),
            "model without compatibility rows must not appear under any printer group, got {:?}",
            items
        );
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
