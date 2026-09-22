//! SQLite adapter for `NumberTemplateRepository` (Phase 40.2, Plan 03).
//!
//! Implements the read-only port trait (`get`, `list`, `get_context_template_id`,
//! `fetch_matching_values`) plus inherent `*_in_tx` write helpers, mirroring
//! `cartridges_sqlite.rs`'s `insert_model_in_tx`/`update_model_in_tx` shape:
//! write methods are NOT part of the trait, they are adapter-specific
//! inherent methods that the service layer (Plan 04) composes inside its own
//! transaction.
//!
//! `compute_next_for_template` is the single bridge between "text column in
//! SQLite" and "numeric sequence": it reads the raw live values for a
//! template's physical space (`fetch_matching_values`), converts each one
//! through `mask::extract_digits` (values that don't match the mask's
//! prefix/suffix/width are silently skipped — they belong to some other
//! sequence or were entered by hand), and feeds the resulting set of numbers
//! into `sequence::compute_next`.
//!
//! All SQL is parameterised through `rusqlite::params![...]` — no user input
//! (mask text, template type) is ever concatenated into a query string. The
//! mask only participates in Rust-side `mask::extract_digits`, never in a
//! `WHERE` clause (T-40.2-05).

use rusqlite::{params, Connection, OptionalExtension, Transaction};

use trackly_core::domain::number_templates::{NextNumberResult, NumberTemplateRow, TemplateType};
use trackly_core::error::AppError;
use trackly_core::ports::number_templates::NumberTemplateRepository;
use trackly_core::text::{mask, sequence};

use crate::error_conversions::map_rusqlite;

/// SQLite-backed number template repository adapter (zero-sized).
#[derive(Debug, Default, Clone)]
pub struct SqliteNumberTemplateRepository;

/// Maps a row from a `number_templates` SELECT (column order: id, type,
/// mask, created_at_utc, updated_at_utc, version) into `NumberTemplateRow`.
fn map_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<NumberTemplateRow> {
    let type_str: String = row.get(1)?;
    let template_type = TemplateType::from_str(&type_str).map_err(|_| {
        rusqlite::Error::FromSqlConversionFailure(
            1,
            rusqlite::types::Type::Text,
            Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("unknown number_templates.type value: {type_str}"),
            )),
        )
    })?;
    Ok(NumberTemplateRow {
        id: row.get(0)?,
        template_type,
        mask: row.get(2)?,
        created_at_utc: row.get(3)?,
        updated_at_utc: row.get(4)?,
        version: row.get(5)?,
    })
}

/// BE-WR-01: a template may only be used/remembered by a context that
/// numbers the same space (`device_inventory` for device/printer create,
/// `act_number` for «Новый акт», …).
pub fn ensure_template_fits_context(
    row: &NumberTemplateRow,
    context: &trackly_core::domain::number_templates::TemplateContext,
) -> Result<(), AppError> {
    let expected = context.template_type();
    if row.template_type != expected {
        return Err(AppError::Validation {
            field: "template_id".to_string(),
            message: format!(
                "Шаблон «{}» относится к разделу «{}» и не подходит для раздела «{}».",
                row.mask,
                row.template_type.display_name_ru(),
                expected.display_name_ru()
            ),
        });
    }
    Ok(())
}

const SELECT_NUMBER_TEMPLATES: &str =
    "SELECT id, type, mask, created_at_utc, updated_at_utc, version FROM number_templates";

impl NumberTemplateRepository for SqliteNumberTemplateRepository {
    type Conn = Connection;

    fn get(&self, conn: &Self::Conn, id: i64) -> Result<NumberTemplateRow, AppError> {
        conn.query_row(
            &format!("{SELECT_NUMBER_TEMPLATES} WHERE id = ?1"),
            params![id],
            map_row,
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => AppError::NotFound {
                entity: "number_template",
                id,
            },
            other => map_rusqlite(other),
        })
    }

    fn list(
        &self,
        conn: &Self::Conn,
        template_type: Option<TemplateType>,
    ) -> Result<Vec<NumberTemplateRow>, AppError> {
        let mut out = Vec::new();
        match template_type {
            Some(t) => {
                let mut stmt = conn
                    .prepare(&format!(
                        "{SELECT_NUMBER_TEMPLATES} WHERE type = ?1 ORDER BY id"
                    ))
                    .map_err(map_rusqlite)?;
                let rows = stmt
                    .query_map(params![t.as_str()], map_row)
                    .map_err(map_rusqlite)?;
                for row in rows {
                    out.push(row.map_err(map_rusqlite)?);
                }
            }
            None => {
                let mut stmt = conn
                    .prepare(&format!("{SELECT_NUMBER_TEMPLATES} ORDER BY id"))
                    .map_err(map_rusqlite)?;
                let rows = stmt.query_map([], map_row).map_err(map_rusqlite)?;
                for row in rows {
                    out.push(row.map_err(map_rusqlite)?);
                }
            }
        }
        Ok(out)
    }

    fn get_context_template_id(
        &self,
        conn: &Self::Conn,
        context: trackly_core::domain::number_templates::TemplateContext,
    ) -> Result<Option<i64>, AppError> {
        conn.query_row(
            "SELECT template_id FROM number_template_contexts WHERE context = ?1",
            params![context.as_str()],
            |r| r.get(0),
        )
        .map_err(map_rusqlite)
    }

    /// See doc-comment on the trait method for the per-space rationale
    /// (act returns excluded, CartridgeCode/DrumCode share one column,
    /// soft-deleted rows never included).
    fn fetch_matching_values(
        &self,
        conn: &Self::Conn,
        template_type: TemplateType,
        _today_utc: i64,
    ) -> Result<Vec<String>, AppError> {
        let sql = match template_type {
            TemplateType::ActNumber => {
                "SELECT number FROM acts WHERE act_type = 'handover' AND deleted_at_utc IS NULL"
            }
            TemplateType::DeviceInventory => {
                "SELECT inventory_number FROM devices \
                 WHERE deleted_at_utc IS NULL AND inventory_number IS NOT NULL"
            }
            TemplateType::CartridgeCode | TemplateType::DrumCode => {
                "SELECT code FROM cartridges WHERE deleted_at_utc IS NULL"
            }
        };

        // `acts.number` is stored as INTEGER, `devices.inventory_number` and
        // `cartridges.code` as TEXT — read every value through rusqlite's
        // `ValueRef`-based dynamic accessor and normalise to `String` here,
        // rather than adding a per-space return type. `mask::extract_digits`
        // only ever cares about the string form.
        let mut stmt = conn.prepare(sql).map_err(map_rusqlite)?;
        let rows = stmt
            .query_map([], |r| {
                let value_ref = r.get_ref(0)?;
                let as_string = match value_ref.data_type() {
                    rusqlite::types::Type::Integer => value_ref.as_i64()?.to_string(),
                    _ => value_ref.as_str()?.to_string(),
                };
                Ok(as_string)
            })
            .map_err(map_rusqlite)?;

        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(map_rusqlite)?);
        }
        Ok(out)
    }
}

impl SqliteNumberTemplateRepository {
    // -----------------------------------------------------------------------
    // Write helpers (in-tx) — NOT part of the port trait, composed by the
    // service layer (Plan 04) inside its own transaction.
    // -----------------------------------------------------------------------

    /// INSERT a new `number_templates` row inside a transaction.
    ///
    /// Does NOT pre-check for a duplicate `(type, mask)` pair — that
    /// human-readable pre-check lives in the service layer (Plan 04). This
    /// method is a thin SQL adapter; the `UNIQUE(type, mask)` constraint from
    /// V041 is the last line of defence (`map_rusqlite` turns a unique
    /// violation into `AppError::Conflict` with raw SQLite text — acceptable
    /// only as a safety net, not the primary UX path).
    pub fn insert_in_tx(
        &self,
        tx: &Transaction<'_>,
        template_type: TemplateType,
        mask: &str,
        now_utc: i64,
    ) -> Result<i64, AppError> {
        tx.execute(
            "INSERT INTO number_templates (type, mask, created_at_utc, updated_at_utc, version) \
             VALUES (?1, ?2, ?3, ?3, 1)",
            params![template_type.as_str(), mask, now_utc],
        )
        .map_err(map_rusqlite)?;
        Ok(tx.last_insert_rowid())
    }

    /// UPDATE the `mask` of an existing template inside a transaction, with
    /// optimistic locking on `version`.
    ///
    /// `template_type` is intentionally NOT a parameter — D-10 makes the
    /// type immutable after creation, and this signature makes it
    /// structurally impossible to change it through this method, not just
    /// documented as a rule.
    pub fn update_mask_in_tx(
        &self,
        tx: &Transaction<'_>,
        id: i64,
        mask: &str,
        version: i64,
        now_utc: i64,
    ) -> Result<(), AppError> {
        let affected = tx
            .execute(
                "UPDATE number_templates SET mask=?1, updated_at_utc=?2, version=version+1 \
                 WHERE id=?3 AND version=?4",
                params![mask, now_utc, id, version],
            )
            .map_err(map_rusqlite)?;

        if affected == 0 {
            let actual: Option<i64> = tx
                .query_row(
                    "SELECT version FROM number_templates WHERE id = ?1",
                    params![id],
                    |r| r.get(0),
                )
                .optional()
                .map_err(map_rusqlite)?;
            return match actual {
                None => Err(AppError::NotFound {
                    entity: "number_template",
                    id,
                }),
                Some(actual) => Err(AppError::OptimisticLockMismatch {
                    entity: "number_template",
                    id,
                    expected: version,
                    actual,
                }),
            };
        }
        Ok(())
    }

    /// Physically DELETE a template row inside a transaction (D-12 — not a
    /// soft-delete, unlike `cartridge_models`). `ON DELETE SET NULL` on
    /// `number_template_contexts.template_id` (V041) handles clearing any
    /// context's memory automatically — no extra step needed here.
    pub fn delete_in_tx(&self, tx: &Transaction<'_>, id: i64) -> Result<(), AppError> {
        let affected = tx
            .execute("DELETE FROM number_templates WHERE id = ?1", params![id])
            .map_err(map_rusqlite)?;
        if affected == 0 {
            return Err(AppError::NotFound {
                entity: "number_template",
                id,
            });
        }
        Ok(())
    }

    /// UPDATE `number_template_contexts.template_id` for `context` inside a
    /// transaction. The row for every context already exists (seeded by
    /// V041) — this is always an `UPDATE`, never an `INSERT`.
    ///
    /// BE-WR-01: the template must exist AND be of the type this context
    /// numbers (`TemplateContext::template_type`) — otherwise a device
    /// template could be remembered as the shared default of «Новый акт».
    /// Missing template → `NotFound`, wrong type → `Validation`.
    pub fn set_context_in_tx(
        &self,
        tx: &Transaction<'_>,
        context: trackly_core::domain::number_templates::TemplateContext,
        template_id: Option<i64>,
        now_utc: i64,
    ) -> Result<(), AppError> {
        if let Some(id) = template_id {
            let row = self.get(tx, id)?;
            ensure_template_fits_context(&row, &context)?;
        }
        tx.execute(
            "UPDATE number_template_contexts SET template_id=?1, updated_at_utc=?2 \
             WHERE context=?3",
            params![template_id, now_utc, context.as_str()],
        )
        .map_err(map_rusqlite)?;
        Ok(())
    }

    /// Lenient variant used INSIDE an entity-creating writer transaction
    /// (BE-WR-02): the template was already validated for this context by
    /// the caller's pre-check, so the only way it can be missing/foreign now
    /// is a concurrent delete — then "no template" is remembered instead of
    /// rolling back the record the user just created.
    pub fn remember_context_in_tx(
        &self,
        tx: &Transaction<'_>,
        context: trackly_core::domain::number_templates::TemplateContext,
        template_id: Option<i64>,
        now_utc: i64,
    ) -> Result<(), AppError> {
        tx.execute(
            "UPDATE number_template_contexts \
                SET template_id = (SELECT id FROM number_templates WHERE id = ?1 AND type = ?4), \
                    updated_at_utc = ?2 \
              WHERE context = ?3",
            params![
                template_id,
                now_utc,
                context.as_str(),
                context.template_type().as_str()
            ],
        )
        .map_err(map_rusqlite)?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Read helper: bridges "text column in SQLite" to "numeric sequence".
    // -----------------------------------------------------------------------

    /// Compute the next free number for `template`'s mask, `today_utc`
    /// deciding what any date tokens in the mask expand to.
    ///
    /// Reads the live matching values for `template.template_type` via
    /// `fetch_matching_values`, converts each through
    /// `mask::extract_digits` (values that don't match this mask's
    /// prefix/digit-width/suffix are silently skipped — they are not part of
    /// THIS template's sequence, e.g. a manually entered number that doesn't
    /// follow any mask), then delegates to `sequence::compute_next`.
    pub fn compute_next_for_template(
        &self,
        conn: &Connection,
        template: &NumberTemplateRow,
        today_utc: i64,
    ) -> Result<NextNumberResult, AppError> {
        let parsed = mask::parse_and_expand(&template.mask, today_utc)?;
        let values = self.fetch_matching_values(conn, template.template_type.clone(), today_utc)?;

        let taken: Vec<u64> = values
            .iter()
            .filter_map(|v| mask::extract_digits(&parsed, v))
            .collect();

        Ok(sequence::compute_next(&taken, parsed.digit_width))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal in-memory harness for the CRUD unit tests below — creates
    /// only the two tables this module touches (not a full migrated DB;
    /// full-stack repo-level tests live in
    /// `tests/number_templates_repo.rs`).
    fn test_conn() -> Connection {
        let conn = Connection::open_in_memory().expect("open in-memory");
        conn.execute_batch(
            "CREATE TABLE number_templates (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                type            TEXT    NOT NULL CHECK (type IN ('device_inventory', 'act_number', 'cartridge_code', 'drum_code')),
                mask            TEXT    NOT NULL,
                created_at_utc  INTEGER NOT NULL,
                updated_at_utc  INTEGER NOT NULL,
                version         INTEGER NOT NULL DEFAULT 1,
                UNIQUE(type, mask)
            );
            CREATE TABLE number_template_contexts (
                context         TEXT    PRIMARY KEY CHECK (context IN ('device_create', 'printer_create', 'act_create', 'cartridge_create', 'drum_create')),
                template_id     INTEGER NULL REFERENCES number_templates(id) ON DELETE SET NULL,
                updated_at_utc  INTEGER NOT NULL
            );
            INSERT INTO number_template_contexts (context, template_id, updated_at_utc)
                VALUES ('act_create', NULL, 0);
            PRAGMA foreign_keys = ON;
            ",
        )
        .expect("create schema");
        conn
    }

    #[test]
    fn insert_update_delete_roundtrip_and_version_bump() {
        let mut conn = test_conn();
        let repo = SqliteNumberTemplateRepository;

        let tx = conn.transaction().expect("begin tx");
        let id = repo
            .insert_in_tx(&tx, TemplateType::ActNumber, "[X]", 100)
            .expect("insert");
        tx.commit().expect("commit insert");

        let row = repo.get(&conn, id).expect("get after insert");
        assert_eq!(row.mask, "[X]");
        assert_eq!(row.version, 1);

        let tx = conn.transaction().expect("begin tx 2");
        repo.update_mask_in_tx(&tx, id, "АКТ-[XXXX]", row.version, 200)
            .expect("update mask");
        tx.commit().expect("commit update");

        let updated = repo.get(&conn, id).expect("get after update");
        assert_eq!(updated.mask, "АКТ-[XXXX]");
        assert_eq!(updated.version, 2, "version must have incremented");

        let tx = conn.transaction().expect("begin tx 3");
        repo.delete_in_tx(&tx, id).expect("delete");
        tx.commit().expect("commit delete");

        let err = repo
            .get(&conn, id)
            .expect_err("must be NotFound after delete");
        match err {
            AppError::NotFound { entity, id: got_id } => {
                assert_eq!(entity, "number_template");
                assert_eq!(got_id, id);
            }
            other => panic!("expected NotFound, got {other:?}"),
        }
    }

    #[test]
    fn update_mask_optimistic_lock_mismatch() {
        let mut conn = test_conn();
        let repo = SqliteNumberTemplateRepository;

        let tx = conn.transaction().expect("begin tx");
        let id = repo
            .insert_in_tx(&tx, TemplateType::CartridgeCode, "C-[XXXX]", 100)
            .expect("insert");
        tx.commit().expect("commit insert");

        let tx = conn.transaction().expect("begin tx 2");
        let err = repo
            .update_mask_in_tx(&tx, id, "C-[XXX]", 99, 200)
            .expect_err("stale version must fail");
        match err {
            AppError::OptimisticLockMismatch {
                entity,
                id: got_id,
                expected,
                actual,
            } => {
                assert_eq!(entity, "number_template");
                assert_eq!(got_id, id);
                assert_eq!(expected, 99);
                assert_eq!(actual, 1);
            }
            other => panic!("expected OptimisticLockMismatch, got {other:?}"),
        }
    }

    #[test]
    fn delete_template_nulls_out_context_reference() {
        let mut conn = test_conn();
        let repo = SqliteNumberTemplateRepository;

        let tx = conn.transaction().expect("begin tx");
        let id = repo
            .insert_in_tx(&tx, TemplateType::ActNumber, "[X]", 100)
            .expect("insert");
        tx.execute(
            "UPDATE number_template_contexts SET template_id = ?1 WHERE context = 'act_create'",
            params![id],
        )
        .expect("wire context");
        tx.commit().expect("commit setup");

        let tx = conn.transaction().expect("begin tx 2");
        repo.delete_in_tx(&tx, id).expect("delete template");
        tx.commit().expect("commit delete");

        let template_id: Option<i64> = conn
            .query_row(
                "SELECT template_id FROM number_template_contexts WHERE context = 'act_create'",
                [],
                |r| r.get(0),
            )
            .expect("read context row");
        assert_eq!(
            template_id, None,
            "ON DELETE SET NULL must clear the context's memory through this code path too"
        );
    }
}
