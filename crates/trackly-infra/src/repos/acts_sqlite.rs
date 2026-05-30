//! SQLite adapter for `ActRepository` + tx-helper methods used by the
//! service layer to compose multi-step write paths (handover create,
//! return, undo) inside a single transaction.
//!
//! All SQL is parameterised through `rusqlite::params![...]`. The
//! `*_in_tx` helpers expect the caller to own a `rusqlite::Transaction`
//! (started via `conn.transaction()` inside a `WriterHandle::execute`
//! closure — see D-Counter-Acts-01).

use rusqlite::{params, Connection, OptionalExtension, Transaction};
use trackly_core::domain::acts::{ActCounts, ActFilter, ActRow, ActType, Pagination};
use trackly_core::error::AppError;
use trackly_core::ports::acts::ActRepository;

use crate::error_conversions::map_rusqlite;

/// SQLite-backed act repository adapter (zero-sized).
#[derive(Debug, Default, Clone)]
pub struct SqliteActRepository;

/// SELECT with the column order expected by `from_row`.
///
/// Joins:
///   - `locations` for the human-readable location name.
///   - `acts p` (self-join) for the parent act's `number` (used by display rule).
///
/// `sibling_return_count` is a correlated subquery counting returns sharing
/// the same `parent_act_id` (for handover acts it counts their own returns —
/// see `from_row` for the rule that drops it to NULL on handover rows).
const SELECT_ACTS: &str = "
    SELECT a.id, a.number, a.sub_number, a.parent_act_id, a.act_type,
           a.giver_name, a.receiver_name, a.location_id, a.notes,
           a.deadline_utc, a.archived,
           a.created_at_utc, a.updated_at_utc, a.deleted_at_utc, a.version,
           l.name AS location_name,
           p.number AS parent_number,
           (SELECT COUNT(*) FROM acts r
              WHERE r.parent_act_id = COALESCE(a.parent_act_id, a.id)
                AND r.deleted_at_utc IS NULL) AS sibling_return_count
      FROM acts a
      LEFT JOIN locations l ON a.location_id = l.id
      LEFT JOIN acts p ON p.id = a.parent_act_id
";

/// Maps a row from `SELECT_ACTS` into `ActRow`.
fn from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ActRow> {
    let act_type_sql: String = row.get(4)?;
    let act_type = match act_type_sql.as_str() {
        "handover" => ActType::Handover,
        "return" => ActType::Return,
        // CHECK constraint guarantees one of the two — if we see another
        // value, the schema has been tampered with.
        other => {
            return Err(rusqlite::Error::FromSqlConversionFailure(
                4,
                rusqlite::types::Type::Text,
                format!("invalid act_type in DB: {other}").into(),
            ));
        }
    };
    Ok(ActRow {
        id: row.get(0)?,
        number: row.get(1)?,
        sub_number: row.get(2)?,
        parent_act_id: row.get(3)?,
        act_type,
        giver_name: row.get(5)?,
        receiver_name: row.get(6)?,
        location_id: row.get(7)?,
        notes: row.get(8)?,
        deadline_utc: row.get(9)?,
        archived: row.get::<_, i64>(10)? == 1,
        created_at_utc: row.get(11)?,
        updated_at_utc: row.get(12)?,
        deleted_at_utc: row.get(13)?,
        version: row.get(14)?,
        location: row.get(15)?,
        parent_number: row.get(16)?,
        sibling_return_count: row.get(17)?,
    })
}

impl SqliteActRepository {
    /// INSERT a new act row inside a transaction.
    ///
    /// `new.id` is ignored — assigned by AUTOINCREMENT and returned.
    /// `created_at_utc` is used for both created and updated columns,
    /// `version` is forced to 1. Counter increment and audit_log row
    /// are the service's responsibility (orchestrated alongside this call).
    pub fn insert_act_in_tx(&self, tx: &Transaction<'_>, new: &ActRow) -> Result<i64, AppError> {
        tx.execute(
            "INSERT INTO acts \
             (number, sub_number, parent_act_id, act_type, giver_name, \
              receiver_name, location_id, notes, deadline_utc, archived, \
              created_at_utc, updated_at_utc, version) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?11, 1)",
            params![
                new.number,
                new.sub_number,
                new.parent_act_id,
                new.act_type.to_sql(),
                new.giver_name,
                new.receiver_name,
                new.location_id,
                new.notes,
                new.deadline_utc,
                if new.archived { 1 } else { 0 },
                new.created_at_utc,
            ],
        )
        .map_err(map_rusqlite)?;
        Ok(tx.last_insert_rowid())
    }

    /// INSERT a single `act_items` row inside a transaction.
    pub fn insert_act_item_in_tx(
        &self,
        tx: &Transaction<'_>,
        act_id: i64,
        device_id: i64,
        quantity: i64,
        condition_at_time: Option<&str>,
        complectation_at_time: Option<&str>,
    ) -> Result<(), AppError> {
        tx.execute(
            "INSERT INTO act_items \
             (act_id, device_id, quantity, condition_at_time, complectation_at_time) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                act_id,
                device_id,
                quantity,
                condition_at_time,
                complectation_at_time
            ],
        )
        .map_err(map_rusqlite)?;
        Ok(())
    }

    /// Fetch a full act row (including JOIN'ed parent number + sibling count)
    /// inside an existing transaction. Used right after INSERT to capture
    /// the full snapshot for `audit_log.after_json`.
    pub fn fetch_full_in_tx(&self, tx: &Transaction<'_>, id: i64) -> Result<ActRow, AppError> {
        tx.query_row(
            &format!("{SELECT_ACTS} WHERE a.id = ?1"),
            params![id],
            from_row,
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => AppError::NotFound { entity: "act", id },
            other => map_rusqlite(other),
        })
    }

    /// Список **активных** (не soft-deleted) return-актов того же родителя,
    /// упорядоченный по `sub_number ASC`. Используется в `delete_soft`
    /// (cascade) и в `get` (заполнить `ActDto.return_ids`).
    pub fn list_returns_for_parent(
        &self,
        conn: &Connection,
        parent_act_id: i64,
    ) -> Result<Vec<ActRow>, AppError> {
        let mut stmt = conn
            .prepare(&format!(
                "{SELECT_ACTS} WHERE a.parent_act_id = ?1 AND a.deleted_at_utc IS NULL \
                 ORDER BY a.sub_number ASC, a.id ASC"
            ))
            .map_err(map_rusqlite)?;
        let rows = stmt
            .query_map(params![parent_act_id], from_row)
            .map_err(map_rusqlite)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(map_rusqlite)?);
        }
        Ok(out)
    }

    /// Tx-вариант `list_returns_for_parent` — используется в `delete_soft`
    /// внутри writer-tx, где нет доступа к `Connection`.
    pub fn list_returns_for_parent_in_tx(
        &self,
        tx: &Transaction<'_>,
        parent_act_id: i64,
    ) -> Result<Vec<ActRow>, AppError> {
        let mut stmt = tx
            .prepare(&format!(
                "{SELECT_ACTS} WHERE a.parent_act_id = ?1 AND a.deleted_at_utc IS NULL \
                 ORDER BY a.sub_number ASC, a.id ASC"
            ))
            .map_err(map_rusqlite)?;
        let rows = stmt
            .query_map(params![parent_act_id], from_row)
            .map_err(map_rusqlite)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(map_rusqlite)?);
        }
        Ok(out)
    }

    /// FTS5 + LIKE search over acts (ACT-04).
    ///
    /// Реализация per Phase 3 RESEARCH §«FTS search across acts joining
    /// act_items + devices_fts»: UNION двух CTE — `act_text_hits` (LIKE по
    /// числовому номеру, ФИО Сдал/Принял) и `device_text_hits` (FTS5 MATCH
    /// через `devices_fts` JOIN `act_items.device_id`). Без отдельного
    /// `acts_fts` (отложено до Phase 7).
    ///
    /// Параметры:
    ///   - `plain_query`: уже подготовленный LIKE pattern (например, `%Иван%`)
    ///     с escape'нутыми `%` и `_`.
    ///   - `fts_query`: уже подготовленный FTS5 MATCH expression
    ///     (`build_fts_query` из service layer). Если пустой — device-hit
    ///     ветка пропускается (LIKE-only fallback).
    ///   - `filter`: act_type/archived/include_deleted.
    ///   - `page`: limit/offset.
    pub fn search_acts(
        &self,
        conn: &Connection,
        plain_query: &str,
        fts_query: &str,
        filter: &ActFilter,
        page: &Pagination,
    ) -> Result<(Vec<ActRow>, u64), AppError> {
        let limit = page.limit.min(200) as i64;
        let offset = page.offset as i64;
        let include_deleted = filter.include_deleted;
        let act_type_sql: Option<&'static str> = filter.act_type.map(|t| t.to_sql());
        let archived_i64: Option<i64> = filter.archived.map(|b| if b { 1 } else { 0 });
        let fts_present = !fts_query.trim().is_empty();

        // Build hits CTE. When `fts_query` пустой — device_text_hits даёт
        // пустое множество (SELECT id FROM acts WHERE 0).
        let device_hits_cte = if fts_present {
            "SELECT DISTINCT ai.act_id AS id \
               FROM act_items ai \
               JOIN devices_fts f ON f.rowid = ai.device_id \
              WHERE devices_fts MATCH ?2"
        } else {
            "SELECT a.id FROM acts a WHERE 0"
        };

        let where_filters = "(?3 = 1 OR a.deleted_at_utc IS NULL) AND \
                             (?4 IS NULL OR a.act_type = ?4) AND \
                             (?5 IS NULL OR a.archived = ?5)";

        // COUNT
        let count_sql = format!(
            "WITH act_text_hits AS ( \
                 SELECT a.id FROM acts a \
                  WHERE (CAST(a.number AS TEXT) LIKE ?1 \
                         OR a.giver_name LIKE ?1 \
                         OR a.receiver_name LIKE ?1) \
             ), \
             device_text_hits AS ( {device_hits_cte} ) \
             SELECT COUNT(*) FROM acts a \
              WHERE a.id IN (SELECT id FROM act_text_hits \
                              UNION SELECT id FROM device_text_hits) \
                AND {where_filters}"
        );

        let total: i64 = conn
            .query_row(
                &count_sql,
                params![
                    plain_query,
                    fts_query,
                    include_deleted as i64,
                    act_type_sql,
                    archived_i64
                ],
                |r| r.get(0),
            )
            .map_err(map_rusqlite)?;

        // SELECT — переиспользуем SELECT_ACTS, добавляя CTE префикс и
        // фильтр по id IN union.
        let select_sql = format!(
            "WITH act_text_hits AS ( \
                 SELECT a.id FROM acts a \
                  WHERE (CAST(a.number AS TEXT) LIKE ?1 \
                         OR a.giver_name LIKE ?1 \
                         OR a.receiver_name LIKE ?1) \
             ), \
             device_text_hits AS ( {device_hits_cte} ) \
             {SELECT_ACTS} \
             WHERE a.id IN (SELECT id FROM act_text_hits \
                             UNION SELECT id FROM device_text_hits) \
               AND {where_filters} \
             ORDER BY a.created_at_utc DESC, a.id DESC \
             LIMIT ?6 OFFSET ?7"
        );

        let mut stmt = conn.prepare(&select_sql).map_err(map_rusqlite)?;
        let rows = stmt
            .query_map(
                params![
                    plain_query,
                    fts_query,
                    include_deleted as i64,
                    act_type_sql,
                    archived_i64,
                    limit,
                    offset
                ],
                from_row,
            )
            .map_err(map_rusqlite)?;

        let mut acts = Vec::new();
        for row in rows {
            acts.push(row.map_err(map_rusqlite)?);
        }
        Ok((acts, total as u64))
    }

    /// Soft-delete an act with optimistic-lock check. Hard-deletes the
    /// junction `act_items` rows in the same transaction (CASCADE does not
    /// fire on soft-delete; D-Soft-vs-Hard-Acts-01).
    pub fn soft_delete_in_tx(
        &self,
        tx: &Transaction<'_>,
        id: i64,
        version: i64,
        now_utc: i64,
    ) -> Result<(), AppError> {
        let affected = tx
            .execute(
                "UPDATE acts SET deleted_at_utc = ?1, version = version + 1, \
                 updated_at_utc = ?1 \
                 WHERE id = ?2 AND version = ?3 AND deleted_at_utc IS NULL",
                params![now_utc, id, version],
            )
            .map_err(map_rusqlite)?;

        if affected == 0 {
            let actual: Option<i64> = tx
                .query_row("SELECT version FROM acts WHERE id = ?1", params![id], |r| {
                    r.get(0)
                })
                .optional()
                .map_err(map_rusqlite)?;
            return match actual {
                None => Err(AppError::NotFound { entity: "act", id }),
                Some(actual) => Err(AppError::OptimisticLockMismatch {
                    entity: "act",
                    id,
                    expected: version,
                    actual,
                }),
            };
        }

        tx.execute("DELETE FROM act_items WHERE act_id = ?1", params![id])
            .map_err(map_rusqlite)?;
        Ok(())
    }
}

/// Atomically increment a named counter and return its new value.
///
/// MUST be called inside a `BEGIN IMMEDIATE` transaction (which `Connection::transaction`
/// supplies by default in rusqlite). Combined with the single-writer pattern
/// (D-WriterChannel-01) this guarantees no two callers see the same number.
pub fn increment_counter_in_tx(tx: &Transaction<'_>, name: &str) -> Result<i64, AppError> {
    tx.query_row(
        "UPDATE counters SET current_value = current_value + 1 \
         WHERE name = ?1 RETURNING current_value",
        params![name],
        |r| r.get::<_, i64>(0),
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => AppError::Internal {
            source_chain: format!("counter '{name}' not seeded"),
        },
        other => map_rusqlite(other),
    })
}

/// Read-only peek at a named counter's current value. Does NOT increment.
pub fn peek_counter(conn: &Connection, name: &str) -> Result<i64, AppError> {
    conn.query_row(
        "SELECT current_value FROM counters WHERE name = ?1",
        params![name],
        |r| r.get::<_, i64>(0),
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => AppError::Internal {
            source_chain: format!("counter '{name}' not seeded"),
        },
        other => map_rusqlite(other),
    })
}

/// `SELECT COALESCE(MAX(sub_number), 0) + 1` для возврат-актов того же
/// родителя. Используется в `do_return` per D-Numbering-01:
/// первый возврат получает sub_number=1, второй — 2, и т.д.
/// Single-writer + BEGIN IMMEDIATE гарантируют отсутствие race'ов.
pub fn next_sub_number_for_parent(
    tx: &Transaction<'_>,
    parent_act_id: i64,
) -> Result<i64, AppError> {
    tx.query_row(
        "SELECT COALESCE(MAX(sub_number), 0) + 1 FROM acts \
         WHERE parent_act_id = ?1 AND deleted_at_utc IS NULL",
        params![parent_act_id],
        |r| r.get::<_, i64>(0),
    )
    .map_err(map_rusqlite)
}

/// Пересчитывает поле `archived` родительского handover-акта на основе
/// «остатка в работе» (B-2 SUM(quantity) semantics).
///
/// `remaining` = SUM(ai.quantity) по тем act_items handover-акта, у которых
/// device всё ещё в status_id = 'в_работе'. Если `remaining == 0`
/// → `archived = 1` (полный возврат); иначе `archived = 0`.
///
/// Возвращает новое значение `archived`. Идемпотентно: если значение не
/// изменилось, `version` всё равно инкрементируется — это согласуется с
/// optimistic-lock семантикой и облегчает аудит изменений archived-флага.
pub fn recompute_parent_archived(
    tx: &Transaction<'_>,
    parent_act_id: i64,
    now_utc: i64,
) -> Result<bool, AppError> {
    let in_work_status_id: i64 = tx
        .query_row(
            "SELECT id FROM device_statuses WHERE code = 'в_работе'",
            [],
            |r| r.get(0),
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => AppError::Internal {
                source_chain: "device_statuses missing code='в_работе' — V014 not applied?".into(),
            },
            other => map_rusqlite(other),
        })?;

    let remaining: i64 = tx
        .query_row(
            "SELECT COALESCE(SUM(ai.quantity), 0) FROM act_items ai \
             JOIN devices d ON d.id = ai.device_id \
             WHERE ai.act_id = ?1 AND d.status_id = ?2",
            params![parent_act_id, in_work_status_id],
            |r| r.get(0),
        )
        .map_err(map_rusqlite)?;

    let archived = if remaining == 0 { 1 } else { 0 };
    tx.execute(
        "UPDATE acts SET archived = ?1, updated_at_utc = ?2, version = version + 1 \
         WHERE id = ?3",
        params![archived, now_utc, parent_act_id],
    )
    .map_err(map_rusqlite)?;
    Ok(archived == 1)
}

/// Peek a named counter inside an open transaction.
pub fn peek_counter_in_tx(tx: &Transaction<'_>, name: &str) -> Result<i64, AppError> {
    tx.query_row(
        "SELECT current_value FROM counters WHERE name = ?1",
        params![name],
        |r| r.get::<_, i64>(0),
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => AppError::Internal {
            source_chain: format!("counter '{name}' not seeded"),
        },
        other => map_rusqlite(other),
    })
}

impl ActRepository for SqliteActRepository {
    type Conn = Connection;

    fn get(&self, conn: &Self::Conn, id: i64) -> Result<ActRow, AppError> {
        conn.query_row(
            &format!("{SELECT_ACTS} WHERE a.id = ?1 AND a.deleted_at_utc IS NULL"),
            params![id],
            from_row,
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => AppError::NotFound { entity: "act", id },
            other => map_rusqlite(other),
        })
    }

    fn list(
        &self,
        conn: &Self::Conn,
        filter: &ActFilter,
        page: &Pagination,
    ) -> Result<(Vec<ActRow>, u64), AppError> {
        let limit = page.limit.min(200) as i64;
        let offset = page.offset as i64;
        let include_deleted = filter.include_deleted;
        let act_type_sql: Option<&'static str> = filter.act_type.map(|t| t.to_sql());
        let archived_i64: Option<i64> = filter.archived.map(|b| if b { 1 } else { 0 });

        // Build COUNT(*) over the same filter set.
        let total: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM acts a WHERE
                   (?1 = 1 OR a.deleted_at_utc IS NULL) AND
                   (?2 IS NULL OR a.act_type = ?2) AND
                   (?3 IS NULL OR a.archived = ?3)",
                params![include_deleted as i64, act_type_sql, archived_i64],
                |r| r.get(0),
            )
            .map_err(map_rusqlite)?;

        let mut stmt = conn
            .prepare(&format!(
                "{SELECT_ACTS} WHERE
                   (?1 = 1 OR a.deleted_at_utc IS NULL) AND
                   (?2 IS NULL OR a.act_type = ?2) AND
                   (?3 IS NULL OR a.archived = ?3)
                 ORDER BY a.created_at_utc DESC, a.id DESC
                 LIMIT ?4 OFFSET ?5"
            ))
            .map_err(map_rusqlite)?;

        let rows = stmt
            .query_map(
                params![
                    include_deleted as i64,
                    act_type_sql,
                    archived_i64,
                    limit,
                    offset
                ],
                from_row,
            )
            .map_err(map_rusqlite)?;

        let mut acts = Vec::new();
        for row in rows {
            acts.push(row.map_err(map_rusqlite)?);
        }
        Ok((acts, total as u64))
    }

    fn delete_soft(
        &self,
        conn: &mut Self::Conn,
        id: i64,
        version: i64,
        now_utc: i64,
    ) -> Result<(), AppError> {
        let tx = conn.transaction().map_err(map_rusqlite)?;
        self.soft_delete_in_tx(&tx, id, version, now_utc)?;
        tx.commit().map_err(map_rusqlite)?;
        Ok(())
    }

    fn peek_next_number(&self, conn: &Self::Conn) -> Result<i64, AppError> {
        let current = peek_counter(conn, "act_number")?;
        Ok(current + 1)
    }

    fn counts(&self, conn: &Self::Conn) -> Result<ActCounts, AppError> {
        // Switch-bar definitions per D-Acts-List-01:
        //   Акты   = handover, archived=0, not deleted
        //   Возвраты = return, not deleted (archived not applicable)
        //   Архив  = handover, archived=1, not deleted
        let handover_active: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM acts \
                 WHERE act_type='handover' AND archived=0 AND deleted_at_utc IS NULL",
                [],
                |r| r.get(0),
            )
            .map_err(map_rusqlite)?;
        let returns: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM acts \
                 WHERE act_type='return' AND deleted_at_utc IS NULL",
                [],
                |r| r.get(0),
            )
            .map_err(map_rusqlite)?;
        let archived: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM acts \
                 WHERE act_type='handover' AND archived=1 AND deleted_at_utc IS NULL",
                [],
                |r| r.get(0),
            )
            .map_err(map_rusqlite)?;
        Ok(ActCounts {
            handover_active,
            returns,
            archived,
        })
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
        let path = dir.path().join("acts-repo-test.db");
        let mut conn = Connection::open(&path).expect("open");
        apply_writer_pragmas(&conn).expect("writer pragmas");
        migrations::run(&mut conn).expect("migrations");
        (conn, dir)
    }

    #[test]
    fn device_status_codes_seeded() {
        let (conn, _g) = fresh_conn();
        for code in ["на_складе", "в_работе", "на_ремонте", "списано"]
        {
            let id: i64 = conn
                .query_row(
                    "SELECT id FROM device_statuses WHERE code = ?1",
                    params![code],
                    |r| r.get(0),
                )
                .unwrap_or_else(|_| panic!("missing device_statuses.code = {code}"));
            assert!(id > 0);
        }
    }

    #[test]
    fn act_items_quantity_column_exists_with_default_one() {
        let (conn, _g) = fresh_conn();
        let mut stmt = conn
            .prepare("PRAGMA table_info(act_items)")
            .expect("prepare");
        let rows: Vec<(String, i64, String)> = stmt
            .query_map([], |r| {
                let name: String = r.get(1)?;
                let notnull: i64 = r.get(3)?;
                let dflt_value: Option<String> = r.get(4)?;
                Ok((name, notnull, dflt_value.unwrap_or_default()))
            })
            .expect("query_map")
            .collect::<rusqlite::Result<_>>()
            .expect("collect");
        let qty = rows
            .iter()
            .find(|(n, _, _)| n == "quantity")
            .expect("quantity column missing");
        assert_eq!(qty.1, 1, "quantity must be NOT NULL");
        assert_eq!(qty.2, "1", "quantity DEFAULT must be 1");
    }

    #[test]
    fn round_trip_insert_get() {
        let (mut conn, _g) = fresh_conn();
        let now = 1_700_000_000_i64;
        let repo = SqliteActRepository;

        let row = ActRow {
            id: 0,
            number: 1,
            sub_number: None,
            parent_act_id: None,
            act_type: ActType::Handover,
            giver_name: "Иванов".into(),
            receiver_name: "Петров".into(),
            location_id: None,
            location: None,
            notes: Some("test".into()),
            deadline_utc: Some(now + 86_400),
            archived: false,
            created_at_utc: now,
            updated_at_utc: now,
            deleted_at_utc: None,
            version: 1,
            parent_number: None,
            sibling_return_count: None,
        };
        let id = {
            let tx = conn.transaction().expect("tx");
            let id = repo.insert_act_in_tx(&tx, &row).expect("insert");
            tx.commit().expect("commit");
            id
        };
        let back = repo.get(&conn, id).expect("get");
        assert_eq!(back.giver_name, "Иванов");
        assert_eq!(back.receiver_name, "Петров");
        assert_eq!(back.act_type, ActType::Handover);
        assert_eq!(back.notes.as_deref(), Some("test"));
        assert_eq!(back.deadline_utc, Some(now + 86_400));
        assert!(!back.archived);
    }

    #[test]
    fn increment_counter_returns_one_first() {
        let (mut conn, _g) = fresh_conn();
        let tx = conn.transaction().expect("tx");
        let n = increment_counter_in_tx(&tx, "act_number").expect("inc");
        assert_eq!(n, 1);
        let n2 = increment_counter_in_tx(&tx, "act_number").expect("inc2");
        assert_eq!(n2, 2);
        tx.commit().expect("commit");
        // peek does not increment
        let peeked = peek_counter(&conn, "act_number").expect("peek");
        assert_eq!(peeked, 2);
    }
}
