//! SQLite adapter for `ActRepository` + tx-helper methods used by the
//! service layer to compose multi-step write paths (handover create,
//! return, undo) inside a single transaction.
//!
//! All SQL is parameterised through `rusqlite::params![...]`. The
//! `*_in_tx` helpers expect the caller to own a `rusqlite::Transaction`
//! (started via `conn.transaction()` inside a `WriterHandle::execute`
//! closure — see D-Counter-Acts-01).

use rusqlite::{params, Connection, OptionalExtension, Transaction};
use trackly_core::domain::acts::{ActCounts, ActFilter, ActPatch, ActRow, ActType, Pagination};
use trackly_core::error::AppError;
use trackly_core::ports::acts::ActRepository;

use crate::error_conversions::map_rusqlite;

/// SQLite-backed act repository adapter (zero-sized).
#[derive(Debug, Default, Clone)]
pub struct SqliteActRepository;

/// SELECT with the column order expected by `from_row`.
///
/// Joins:
///   - `place_full_paths` for the live-resolved current place path
///     (`resolved_place_path` — distinct from the stored, frozen
///     `a.place_path_snapshot` column also selected directly below; D-16
///     requires BOTH to exist side by side: one for navigation/lists
///     [live], one for print [frozen]).
///   - `acts p` (self-join) for the parent act's `number` (used by display rule).
///
/// Phase 40.5 (D-03/D-16): the correlated sibling-count subquery was
/// removed — the return suffix no longer depends on siblings. Column order is
/// positional; `handover_date_utc` = 17, `place_path_snapshot` = 18.
const SELECT_ACTS: &str = "
    SELECT a.id, a.number, a.sub_number, a.parent_act_id, a.act_type,
           a.giver_name, a.receiver_name, a.place_id, a.notes,
           a.deadline_utc, a.archived,
           a.created_at_utc, a.updated_at_utc, a.deleted_at_utc, a.version,
           pfp.full_path AS resolved_place_path,
           p.number AS parent_number,
           a.handover_date_utc,
           a.place_path_snapshot
      FROM acts a
      LEFT JOIN place_full_paths pfp ON pfp.place_id = a.place_id
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
        // Phase 40.2 (NUM-14): `number` is TEXT (V042) — read as String.
        number: row.get(1)?,
        sub_number: row.get(2)?,
        parent_act_id: row.get(3)?,
        act_type,
        giver_name: row.get(5)?,
        receiver_name: row.get(6)?,
        place_id: row.get(7)?,
        notes: row.get(8)?,
        deadline_utc: row.get(9)?,
        archived: row.get::<_, i64>(10)? == 1,
        created_at_utc: row.get(11)?,
        updated_at_utc: row.get(12)?,
        deleted_at_utc: row.get(13)?,
        version: row.get(14)?,
        full_path: row.get(15)?,
        // Phase 40.2 (NUM-14): parent's `number` is also TEXT now.
        parent_number: row.get(16)?,
        handover_date_utc: row.get(17)?,
        place_path_snapshot: row.get(18)?,
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
              receiver_name, place_id, notes, deadline_utc, archived, \
              created_at_utc, updated_at_utc, version, handover_date_utc, \
              place_path_snapshot) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?11, 1, ?12, ?13)",
            params![
                new.number,
                new.sub_number,
                new.parent_act_id,
                new.act_type.to_sql(),
                new.giver_name,
                new.receiver_name,
                new.place_id,
                new.notes,
                new.deadline_utc,
                if new.archived { 1 } else { 0 },
                new.created_at_utc,
                new.handover_date_utc,
                new.place_path_snapshot,
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
             ORDER BY a.handover_date_utc DESC, a.id DESC \
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

    /// CAS header UPDATE for act editing (Phase 19, ACT-02).
    ///
    /// Touches only the mutable header fields: `giver_name`, `receiver_name`,
    /// `place_id`, `place_path_snapshot` (D-16 — re-captured server-side by
    /// the caller via `PlaceRepository::full_path`, passed explicitly since
    /// `ActPatch` itself doesn't carry it), `notes`, `deadline_utc`,
    /// `handover_date_utc` (D-01/D-04), and `number` (D-04 override,
    /// `COALESCE`d — only applied if `patch.number` is `Some`). Does NOT
    /// touch `sub_number`, `parent_act_id`, `act_type` (immutable identity
    /// fields) or `created_at_utc` (D-02: purely internal). Lock check is
    /// folded into the single `UPDATE` statement itself (not a separate
    /// read-then-write), structurally preventing a TOCTOU race — same
    /// pattern as `soft_delete_in_tx`.
    ///
    /// Caller — `ActService::update` (Plan 19-03, place-tree migration
    /// Plan 39-07).
    pub fn update_act_header_in_tx(
        &self,
        tx: &Transaction<'_>,
        id: i64,
        patch: &ActPatch,
        place_path_snapshot: Option<&str>,
        now_utc: i64,
    ) -> Result<(), AppError> {
        let affected = tx
            .execute(
                "UPDATE acts SET giver_name = ?1, receiver_name = ?2, \
                 place_id = ?3, place_path_snapshot = ?4, notes = ?5, deadline_utc = ?6, \
                 handover_date_utc = COALESCE(?7, handover_date_utc), \
                 number = COALESCE(?8, number), \
                 version = version + 1, updated_at_utc = ?9 \
                 WHERE id = ?10 AND version = ?11 AND deleted_at_utc IS NULL",
                params![
                    patch.giver_name,
                    patch.receiver_name,
                    patch.place_id,
                    place_path_snapshot,
                    patch.notes,
                    patch.deadline_utc,
                    patch.handover_date_utc,
                    patch.number,
                    now_utc,
                    id,
                    patch.expected_version,
                ],
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
                    expected: patch.expected_version,
                    actual,
                }),
            };
        }

        Ok(())
    }
}

/// Наименьший свободный положительный `sub_number` среди живых возвратов
/// родителя (Phase 40.5, D-04/D-05): удаление возврата освобождает его номер
/// для СЛЕДУЮЩЕГО созданного возврата; при нескольких «дырках» занимается
/// меньшая.
///
/// `is_blocked(k)` (D-17) — предикат слоя сервиса: «отображаемый номер
/// `{родитель}в{k}` занят ЧУЖИМ живым актом». Такие позиции перешагиваются, чтобы
/// акт с посторонним «42в1» не становился невозвратимым навсегда. Формула
/// отображения (`format_act_number`) живёт в слое сервиса, а разбирать
/// отображаемую строку запрещено (D-08) — поэтому сюда приходит предикат.
/// Именно предикат, а не заранее посчитанное множество в диапазоне: заблокированных
/// позиций не больше, чем чужих живых актов, но их ЗНАЧЕНИЯ числом чужих актов не
/// ограничены (возвраты в1, в2 и единственный посторонний «42в3» блокируют
/// позицию 3), поэтому поиск не имеет искусственной верхней границы.
///
/// Гарантии от гонок: единственная защита — single-writer worker
/// (`db::writer_worker`: один `Connection`, джобы сериализуются через mpsc).
/// Транзакции открываются через `conn.transaction()`, то есть DEFERRED;
/// режим IMMEDIATE (немедленная блокировка записи) в коде не используется. `idx_acts_number_sub_unique` —
/// это `UNIQUE(number, COALESCE(sub_number,0)) WHERE deleted_at_utc IS NULL`,
/// индекс по `number`, а не по `parent_act_id`: он закрывает уникальность
/// `sub_number` внутри родителя лишь пока каждый живой возврат хранит копию
/// номера родителя (инвариант каскада переименования в `act_service.rs`) и НЕ
/// является независимым backstop'ом. Новых механизмов (блокировок, retry) не
/// вводить (D-07).
pub fn next_sub_number_for_parent(
    tx: &Transaction<'_>,
    parent_act_id: i64,
    is_blocked: &dyn Fn(i64) -> bool,
) -> Result<i64, AppError> {
    let mut stmt = tx
        .prepare(
            "SELECT sub_number FROM acts \
             WHERE parent_act_id = ?1 AND deleted_at_utc IS NULL \
             AND sub_number IS NOT NULL",
        )
        .map_err(map_rusqlite)?;
    let live = stmt
        .query_map(params![parent_act_id], |r| r.get::<_, i64>(0))
        .map_err(map_rusqlite)?
        .collect::<rusqlite::Result<Vec<i64>>>()
        .map_err(map_rusqlite)?;
    Ok(smallest_free(live, is_blocked))
}

/// Наименьшее положительное целое, отсутствующее в `live` и не заблокированное
/// предикатом `is_blocked`. Верхней границы цикла нет: множества `live` и
/// заблокированных позиций конечны, поэтому свободный кандидат существует.
fn smallest_free(live: Vec<i64>, is_blocked: &dyn Fn(i64) -> bool) -> i64 {
    let used: std::collections::BTreeSet<i64> = live.into_iter().filter(|n| *n > 0).collect();
    let mut candidate = 1;
    loop {
        if !used.contains(&candidate) && !is_blocked(candidate) {
            return candidate;
        }
        candidate += 1;
    }
}

/// Пересчитывает поле `archived` родительского handover-акта на основе
/// COUNT-based формулы (G-11 / G-12 clone-on-handover model, V015+).
///
/// `handover_total` = COUNT(*) FROM act_items WHERE act_id = parent_act_id.
///   В новой модели каждая device-row в handover это 1 act_item, поэтому
///   COUNT(*) — это canonical «сколько устройств выдано».
///
/// `returned_total` = COUNT(DISTINCT rai.device_id) по всем НЕудалённым
///   return-актам с parent_act_id = parent_act_id.
///   COUNT(DISTINCT) защищает от случайных дубликатов device_id в return-act
///   (defence-in-depth поверх validate_return dedup).
///
/// `archived = (handover_total > 0 AND handover_total <= returned_total)`.
///
/// Note: `act_items` НЕ имеет `deleted_at_utc` колонки (B-1). Soft-delete
/// filter применяется ТОЛЬКО на parent `acts` row через JOIN. Caller гарантирует
/// что recompute вызывается только для не-удалённого parent.
///
/// Возвращает новое значение `archived`. Идемпотентно: если значение не
/// изменилось, `version` всё равно инкрементируется — это согласуется с
/// optimistic-lock семантикой и облегчает аудит изменений archived-флага.
pub fn recompute_parent_archived(
    tx: &Transaction<'_>,
    parent_act_id: i64,
    now_utc: i64,
) -> Result<bool, AppError> {
    let handover_total: i64 = tx
        .query_row(
            "SELECT COUNT(*) FROM act_items WHERE act_id = ?1",
            params![parent_act_id],
            |r| r.get(0),
        )
        .map_err(map_rusqlite)?;

    let returned_total: i64 = tx
        .query_row(
            "SELECT COUNT(DISTINCT rai.device_id) \
               FROM act_items rai \
               JOIN acts ra ON ra.id = rai.act_id \
              WHERE ra.parent_act_id = ?1 \
                AND ra.deleted_at_utc IS NULL",
            params![parent_act_id],
            |r| r.get(0),
        )
        .map_err(map_rusqlite)?;

    let archived = if handover_total > 0 && handover_total <= returned_total {
        1
    } else {
        0
    };
    tx.execute(
        "UPDATE acts SET archived = ?1, updated_at_utc = ?2, version = version + 1 \
         WHERE id = ?3",
        params![archived, now_utc, parent_act_id],
    )
    .map_err(map_rusqlite)?;
    Ok(archived == 1)
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
                 ORDER BY a.handover_date_utc DESC, a.id DESC
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
            number: "1".to_string(),
            sub_number: None,
            parent_act_id: None,
            act_type: ActType::Handover,
            giver_name: "Иванов".into(),
            receiver_name: "Петров".into(),
            place_id: None,
            full_path: None,
            place_path_snapshot: Some("Территория А / Склад 1".to_string()),
            notes: Some("test".into()),
            deadline_utc: Some(now + 86_400),
            archived: false,
            created_at_utc: now,
            updated_at_utc: now,
            deleted_at_utc: None,
            version: 1,
            handover_date_utc: now - 3_600,
            parent_number: None,
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
        // D-16: positional from_row indexes 17/18 — verify VALUES, not presence.
        assert_eq!(back.handover_date_utc, now - 3_600);
        assert_eq!(
            back.place_path_snapshot.as_deref(),
            Some("Территория А / Склад 1")
        );
    }

    #[test]
    fn smallest_free_empty_is_one() {
        assert_eq!(smallest_free(vec![], &|_| false), 1);
    }

    #[test]
    fn smallest_free_dense_run_is_next() {
        assert_eq!(smallest_free(vec![1, 2, 3], &|_| false), 4);
    }

    #[test]
    fn smallest_free_single_hole_at_start() {
        assert_eq!(smallest_free(vec![2, 3], &|_| false), 1);
    }

    #[test]
    fn smallest_free_hole_in_middle() {
        assert_eq!(smallest_free(vec![1, 3], &|_| false), 2);
    }

    #[test]
    fn smallest_free_two_holes_picks_smaller() {
        assert_eq!(smallest_free(vec![2, 4], &|_| false), 1);
    }

    #[test]
    fn smallest_free_skips_blocked_first_position() {
        assert_eq!(smallest_free(vec![], &|k| k == 1), 2);
    }

    #[test]
    fn smallest_free_skips_blocked_and_used() {
        assert_eq!(smallest_free(vec![2], &|k| k == 1), 3);
    }

    #[test]
    fn smallest_free_block_above_answer_does_not_shift_result() {
        assert_eq!(smallest_free(vec![1], &|k| k == 3), 2);
    }

    #[test]
    fn smallest_free_blocked_position_above_used_run() {
        // B-1: заблокированная позиция выше числа чужих актов (один чужой акт, k = 3).
        assert_eq!(smallest_free(vec![1, 2], &|k| k == 3), 4);
    }

    fn insert_raw(
        tx: &Transaction<'_>,
        number: &str,
        sub: Option<i64>,
        kind: &str,
        parent: Option<i64>,
    ) -> rusqlite::Result<usize> {
        tx.execute(
            "INSERT INTO acts (number, sub_number, parent_act_id, act_type, giver_name, \
             receiver_name, created_at_utc, updated_at_utc, handover_date_utc) \
             VALUES (?1, ?2, ?3, ?4, 'Иванов И.И.', 'Петров П.П.', 1, 1, 1)",
            params![number, sub, parent, kind],
        )
    }

    #[test]
    fn unique_index_rejects_duplicate_number_sub_number_pair() {
        let (mut conn, _g) = fresh_conn();
        let tx = conn.transaction().expect("tx");
        insert_raw(&tx, "1", None, "handover", None).expect("handover");
        let parent = tx.last_insert_rowid();
        insert_raw(&tx, "1", Some(1), "return", Some(parent)).expect("first return");
        let err = insert_raw(&tx, "1", Some(1), "return", Some(parent)).unwrap_err();
        assert!(
            matches!(
                err,
                rusqlite::Error::SqliteFailure(ref e, _)
                    if e.code == rusqlite::ErrorCode::ConstraintViolation
            ),
            "expected constraint violation, got {err:?}"
        );
    }

    #[test]
    fn next_sub_number_reuses_freed_position() {
        let (mut conn, _g) = fresh_conn();
        let tx = conn.transaction().expect("tx");
        insert_raw(&tx, "1", None, "handover", None).expect("handover");
        let parent = tx.last_insert_rowid();
        assert_eq!(
            next_sub_number_for_parent(&tx, parent, &|_| false).unwrap(),
            1
        );
        for sub in [1, 2, 3] {
            insert_raw(&tx, "1", Some(sub), "return", Some(parent)).expect("return");
        }
        assert_eq!(
            next_sub_number_for_parent(&tx, parent, &|_| false).unwrap(),
            4
        );
        tx.execute(
            "UPDATE acts SET deleted_at_utc = 2 WHERE parent_act_id = ?1 AND sub_number IN (1, 3)",
            params![parent],
        )
        .unwrap();
        assert_eq!(
            next_sub_number_for_parent(&tx, parent, &|_| false).unwrap(),
            1
        );
    }
}
