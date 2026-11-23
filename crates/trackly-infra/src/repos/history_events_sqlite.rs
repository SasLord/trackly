//! SQLite-адаптер таблицы `history_events` (фаза 41.10, миграция V057).
//!
//! Все записи идут на транзакции вызывающего (`*_in_tx`), чтобы событие попадало в ту же
//! транзакцию, что и мутация (D-01). Только `params!`, без форматирования SQL (T-41.10-01).
//! Токены `kind`/`source` читаются сырыми строками: разбор — забота читателя через
//! `from_str_lenient` (D-23). Единственный владелец SQL удаления по акту (D-24, UAT-1).

use rusqlite::{params, Connection, Row, Transaction};
use trackly_core::error::AppError;

use crate::error_conversions::map_rusqlite;

/// Zero-sized репозиторий событий истории.
#[derive(Debug, Default, Clone)]
pub struct SqliteHistoryEventsRepository;

/// Строка для вставки (все столбцы V057, кроме `id`).
#[derive(Debug, Clone, Default)]
pub struct NewHistoryEvent<'a> {
    pub operation_id: &'a str,
    pub subject_type: &'a str,
    pub subject_id: i64,
    pub subject_label: Option<&'a str>,
    pub kind: &'a str,
    pub from_group_id: Option<i64>,
    pub from_group_label: Option<&'a str>,
    pub to_group_id: Option<i64>,
    pub to_group_label: Option<&'a str>,
    pub status_from_code: Option<&'a str>,
    pub status_from_label: Option<&'a str>,
    pub status_to_code: Option<&'a str>,
    pub status_to_label: Option<&'a str>,
    pub source: &'a str,
    pub act_id: Option<i64>,
    pub request_id: Option<i64>,
    pub ref_group_id: Option<i64>,
    pub ref_group_label: Option<&'a str>,
    pub movement_id: Option<i64>,
    pub user_id: Option<i64>,
    pub actor_name_snapshot: Option<&'a str>,
    pub created_at_utc: i64,
}

/// Прочитанная строка `history_events`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryEventRow {
    pub id: i64,
    pub operation_id: String,
    pub subject_type: String,
    pub subject_id: i64,
    pub subject_label: Option<String>,
    pub kind: String,
    pub from_group_id: Option<i64>,
    pub from_group_label: Option<String>,
    pub to_group_id: Option<i64>,
    pub to_group_label: Option<String>,
    pub status_from_code: Option<String>,
    pub status_from_label: Option<String>,
    pub status_to_code: Option<String>,
    pub status_to_label: Option<String>,
    pub source: String,
    pub act_id: Option<i64>,
    pub request_id: Option<i64>,
    pub ref_group_id: Option<i64>,
    pub ref_group_label: Option<String>,
    pub movement_id: Option<i64>,
    pub user_id: Option<i64>,
    pub actor_name_snapshot: Option<String>,
    pub created_at_utc: i64,
}

const COLS: &str = "id, operation_id, subject_type, subject_id, subject_label, kind, \
    from_group_id, from_group_label, to_group_id, to_group_label, \
    status_from_code, status_from_label, status_to_code, status_to_label, \
    source, act_id, request_id, ref_group_id, ref_group_label, movement_id, \
    user_id, actor_name_snapshot, created_at_utc";

fn map_row(r: &Row<'_>) -> rusqlite::Result<HistoryEventRow> {
    Ok(HistoryEventRow {
        id: r.get(0)?,
        operation_id: r.get(1)?,
        subject_type: r.get(2)?,
        subject_id: r.get(3)?,
        subject_label: r.get(4)?,
        kind: r.get(5)?,
        from_group_id: r.get(6)?,
        from_group_label: r.get(7)?,
        to_group_id: r.get(8)?,
        to_group_label: r.get(9)?,
        status_from_code: r.get(10)?,
        status_from_label: r.get(11)?,
        status_to_code: r.get(12)?,
        status_to_label: r.get(13)?,
        source: r.get(14)?,
        act_id: r.get(15)?,
        request_id: r.get(16)?,
        ref_group_id: r.get(17)?,
        ref_group_label: r.get(18)?,
        movement_id: r.get(19)?,
        user_id: r.get(20)?,
        actor_name_snapshot: r.get(21)?,
        created_at_utc: r.get(22)?,
    })
}

impl SqliteHistoryEventsRepository {
    /// Вставка одного события; возвращает id строки.
    pub fn insert_in_tx(
        &self,
        tx: &Transaction<'_>,
        e: NewHistoryEvent<'_>,
    ) -> Result<i64, AppError> {
        tx.execute(
            "INSERT INTO history_events \
             (operation_id, subject_type, subject_id, subject_label, kind, \
              from_group_id, from_group_label, to_group_id, to_group_label, \
              status_from_code, status_from_label, status_to_code, status_to_label, \
              source, act_id, request_id, ref_group_id, ref_group_label, movement_id, \
              user_id, actor_name_snapshot, created_at_utc) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, \
                     ?16, ?17, ?18, ?19, ?20, ?21, ?22)",
            params![
                e.operation_id,
                e.subject_type,
                e.subject_id,
                e.subject_label,
                e.kind,
                e.from_group_id,
                e.from_group_label,
                e.to_group_id,
                e.to_group_label,
                e.status_from_code,
                e.status_from_label,
                e.status_to_code,
                e.status_to_label,
                e.source,
                e.act_id,
                e.request_id,
                e.ref_group_id,
                e.ref_group_label,
                e.movement_id,
                e.user_id,
                e.actor_name_snapshot,
                e.created_at_utc,
            ],
        )
        .map_err(map_rusqlite)?;
        Ok(tx.last_insert_rowid())
    }

    /// События устройства, новые первыми.
    pub fn list_for_device(
        &self,
        conn: &Connection,
        device_id: i64,
    ) -> Result<Vec<HistoryEventRow>, AppError> {
        let sql = format!(
            "SELECT {COLS} FROM history_events \
             WHERE subject_type = 'device' AND subject_id = ?1 \
             ORDER BY created_at_utc DESC, id DESC"
        );
        let mut stmt = conn.prepare(&sql).map_err(map_rusqlite)?;
        let rows = stmt
            .query_map(params![device_id], map_row)
            .map_err(map_rusqlite)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(map_rusqlite)?;
        Ok(rows)
    }

    /// События группы: о самой группе и о составе (to/from). Смены статуса не входят (D-11).
    pub fn list_for_group(
        &self,
        conn: &Connection,
        group_id: i64,
    ) -> Result<Vec<HistoryEventRow>, AppError> {
        let sql = format!(
            "SELECT {COLS} FROM history_events \
             WHERE ((subject_type = 'group' AND subject_id = ?1) \
                    OR to_group_id = ?1 OR from_group_id = ?1) \
               AND kind <> 'status_changed' \
             ORDER BY created_at_utc DESC, id DESC"
        );
        let mut stmt = conn.prepare(&sql).map_err(map_rusqlite)?;
        let rows = stmt
            .query_map(params![group_id], map_row)
            .map_err(map_rusqlite)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(map_rusqlite)?;
        Ok(rows)
    }

    /// D-24: единственное место SQL удаления событий по акту. Возвращает число строк.
    pub fn delete_by_act_id_in_tx(
        &self,
        tx: &Transaction<'_>,
        act_id: i64,
    ) -> Result<usize, AppError> {
        tx.execute(
            "DELETE FROM history_events WHERE act_id = ?1",
            params![act_id],
        )
        .map_err(map_rusqlite)
    }

    /// UAT-3 (фаза 41.10): события, приклеенные к стёртым строкам перемещений (выбор «стереть
    /// из истории перемещения после акта»). Возвращает число строк.
    pub fn delete_by_movement_ids_in_tx(
        &self,
        tx: &Transaction<'_>,
        movement_ids: &[i64],
    ) -> Result<usize, AppError> {
        let mut n = 0;
        for id in movement_ids {
            n += tx
                .execute(
                    "DELETE FROM history_events WHERE movement_id = ?1",
                    params![id],
                )
                .map_err(map_rusqlite)?;
        }
        Ok(n)
    }

    /// UAT-1 (D-24): события акта `act_id` по одному субъекту — правка исключила его из
    /// акта, и событие живёт ровно столько, сколько позиция акта. Возвращает число строк.
    pub fn delete_by_act_and_subject_in_tx(
        &self,
        tx: &Transaction<'_>,
        act_id: i64,
        subject_type: &str,
        subject_id: i64,
    ) -> Result<usize, AppError> {
        tx.execute(
            "DELETE FROM history_events \
             WHERE act_id = ?1 AND subject_type = ?2 AND subject_id = ?3",
            params![act_id, subject_type, subject_id],
        )
        .map_err(map_rusqlite)
    }

    /// UAT-1 (D-24): события акта `act_id` по одному субъекту, КРОМЕ вида `keep_kind`:
    /// правка исключила устройство, но одно следствие акта (вывод из группы) может пережить
    /// исключение, если состав не восстановился. Возвращает число строк.
    pub fn delete_by_act_and_subject_except_kind_in_tx(
        &self,
        tx: &Transaction<'_>,
        act_id: i64,
        subject_type: &str,
        subject_id: i64,
        keep_kind: &str,
    ) -> Result<usize, AppError> {
        tx.execute(
            "DELETE FROM history_events \
             WHERE act_id = ?1 AND subject_type = ?2 AND subject_id = ?3 AND kind <> ?4",
            params![act_id, subject_type, subject_id, keep_kind],
        )
        .map_err(map_rusqlite)
    }

    /// UAT-1 (D-24): события акта `act_id` одного вида по одному субъекту — правка отменила
    /// одно следствие акта (например, вывод из группы), а сама позиция в акте осталась.
    pub fn delete_by_act_subject_and_kind_in_tx(
        &self,
        tx: &Transaction<'_>,
        act_id: i64,
        subject_type: &str,
        subject_id: i64,
        kind: &str,
    ) -> Result<usize, AppError> {
        tx.execute(
            "DELETE FROM history_events \
             WHERE act_id = ?1 AND subject_type = ?2 AND subject_id = ?3 AND kind = ?4",
            params![act_id, subject_type, subject_id, kind],
        )
        .map_err(map_rusqlite)
    }
}
