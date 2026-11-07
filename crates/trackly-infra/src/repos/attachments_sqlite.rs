//! SQLite-адаптер таблицы `attachments` (V050, Phase 41.3).
//!
//! Нулевой по размеру репозиторий со встроенными функциями над
//! `&Connection` (`Transaction` разыменовывается в `Connection`). Все запросы
//! параметризованы. Счётчик ссылок на хеш считает ВСЕ строки, включая мягко
//! удалённые (D-18): файл на диске живёт, пока на него ссылается хоть одна.

use rusqlite::{params, Connection, OptionalExtension, Row};
use trackly_core::error::AppError;

use crate::error_conversions::map_rusqlite;

/// SQLite-backed attachments repository (zero-sized).
#[derive(Debug, Default, Clone)]
pub struct SqliteAttachmentRepository;

/// Новая строка вложения.
#[derive(Debug, Clone)]
pub struct AttachmentNew<'a> {
    pub parent_kind: &'a str,
    pub parent_id: i64,
    pub original_name: &'a str,
    pub mime: &'a str,
    pub size_bytes: i64,
    pub content_hash: &'a str,
    pub uploaded_by_user_id: Option<i64>,
    pub created_at_utc: i64,
}

/// Строка вложения с именами пользователей (LEFT JOIN users).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttachmentRow {
    pub id: i64,
    pub parent_kind: String,
    pub parent_id: i64,
    pub original_name: String,
    pub mime: String,
    pub size_bytes: i64,
    pub content_hash: String,
    pub uploaded_by_user_id: Option<i64>,
    pub uploaded_by_name: Option<String>,
    pub created_at_utc: i64,
    pub deleted_at_utc: Option<i64>,
    pub deleted_by_user_id: Option<i64>,
    pub deleted_by_name: Option<String>,
}

const SELECT: &str = "SELECT a.id, a.parent_kind, a.parent_id, a.original_name, a.mime, \
    a.size_bytes, a.content_hash, a.uploaded_by_user_id, u.full_name, a.created_at_utc, \
    a.deleted_at_utc, a.deleted_by_user_id, d.full_name \
    FROM attachments a \
    LEFT JOIN users u ON u.id = a.uploaded_by_user_id \
    LEFT JOIN users d ON d.id = a.deleted_by_user_id";

fn map_row(r: &Row<'_>) -> rusqlite::Result<AttachmentRow> {
    Ok(AttachmentRow {
        id: r.get(0)?,
        parent_kind: r.get(1)?,
        parent_id: r.get(2)?,
        original_name: r.get(3)?,
        mime: r.get(4)?,
        size_bytes: r.get(5)?,
        content_hash: r.get(6)?,
        uploaded_by_user_id: r.get(7)?,
        uploaded_by_name: r.get(8)?,
        created_at_utc: r.get(9)?,
        deleted_at_utc: r.get(10)?,
        deleted_by_user_id: r.get(11)?,
        deleted_by_name: r.get(12)?,
    })
}

impl SqliteAttachmentRepository {
    /// Вставляет строку, возвращает новый id.
    pub fn insert_on(&self, conn: &Connection, n: &AttachmentNew<'_>) -> Result<i64, AppError> {
        conn.execute(
            "INSERT INTO attachments (parent_kind, parent_id, original_name, mime, size_bytes, \
             content_hash, uploaded_by_user_id, created_at_utc) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                n.parent_kind,
                n.parent_id,
                n.original_name,
                n.mime,
                n.size_bytes,
                n.content_hash,
                n.uploaded_by_user_id,
                n.created_at_utc,
            ],
        )
        .map_err(map_rusqlite)?;
        Ok(conn.last_insert_rowid())
    }

    pub fn get_by_id_on(
        &self,
        conn: &Connection,
        id: i64,
    ) -> Result<Option<AttachmentRow>, AppError> {
        conn.query_row(&format!("{SELECT} WHERE a.id = ?1"), params![id], map_row)
            .optional()
            .map_err(map_rusqlite)
    }

    /// Вложения родителя в порядке `created_at_utc, id`.
    pub fn list_by_parent_on(
        &self,
        conn: &Connection,
        kind: &str,
        parent_id: i64,
        include_deleted: bool,
    ) -> Result<Vec<AttachmentRow>, AppError> {
        let filter = if include_deleted {
            ""
        } else {
            " AND a.deleted_at_utc IS NULL"
        };
        let sql = format!(
            "{SELECT} WHERE a.parent_kind = ?1 AND a.parent_id = ?2{filter} \
             ORDER BY a.created_at_utc, a.id"
        );
        let mut stmt = conn.prepare(&sql).map_err(map_rusqlite)?;
        let rows = stmt
            .query_map(params![kind, parent_id], map_row)
            .map_err(map_rusqlite)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(map_rusqlite)?;
        Ok(rows)
    }

    /// Мягкое удаление; `false`, если строка уже удалена или не существует.
    pub fn soft_delete_on(
        &self,
        conn: &Connection,
        id: i64,
        user_id: Option<i64>,
        now: i64,
    ) -> Result<bool, AppError> {
        let n = conn
            .execute(
                "UPDATE attachments SET deleted_at_utc = ?1, deleted_by_user_id = ?2 \
                 WHERE id = ?3 AND deleted_at_utc IS NULL",
                params![now, user_id, id],
            )
            .map_err(map_rusqlite)?;
        Ok(n > 0)
    }

    /// Восстановление; `false`, если строка не была удалена.
    pub fn restore_on(&self, conn: &Connection, id: i64) -> Result<bool, AppError> {
        let n = conn
            .execute(
                "UPDATE attachments SET deleted_at_utc = NULL, deleted_by_user_id = NULL \
                 WHERE id = ?1 AND deleted_at_utc IS NOT NULL",
                params![id],
            )
            .map_err(map_rusqlite)?;
        Ok(n > 0)
    }

    /// Жёсткое удаление строки.
    pub fn delete_row_on(&self, conn: &Connection, id: i64) -> Result<bool, AppError> {
        let n = conn
            .execute("DELETE FROM attachments WHERE id = ?1", params![id])
            .map_err(map_rusqlite)?;
        Ok(n > 0)
    }

    /// Сколько строк (живых и мягко удалённых) ссылается на хеш.
    pub fn count_by_hash_on(
        &self,
        conn: &Connection,
        hash: &str,
        exclude_id: Option<i64>,
    ) -> Result<i64, AppError> {
        conn.query_row(
            "SELECT COUNT(*) FROM attachments WHERE content_hash = ?1 AND id <> ?2",
            params![hash, exclude_id.unwrap_or(-1)],
            |r| r.get(0),
        )
        .map_err(map_rusqlite)
    }

    /// `(hash, строк, суммарные байты)` по всем строкам, включая удалённые.
    pub fn hashes_with_totals_on(
        &self,
        conn: &Connection,
    ) -> Result<Vec<(String, i64, i64)>, AppError> {
        let mut stmt = conn
            .prepare(
                "SELECT content_hash, COUNT(*), COALESCE(SUM(size_bytes), 0) \
                 FROM attachments GROUP BY content_hash ORDER BY content_hash",
            )
            .map_err(map_rusqlite)?;
        let rows = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .map_err(map_rusqlite)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(map_rusqlite)?;
        Ok(rows)
    }

    /// `(число, байты)` мягко удалённых строк.
    pub fn count_deleted_with_bytes_on(&self, conn: &Connection) -> Result<(i64, i64), AppError> {
        conn.query_row(
            "SELECT COUNT(*), COALESCE(SUM(size_bytes), 0) FROM attachments \
             WHERE deleted_at_utc IS NOT NULL",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(map_rusqlite)
    }

    /// Различные `(hash, size_bytes)` для отчёта целостности.
    pub fn distinct_hash_sizes_on(
        &self,
        conn: &Connection,
    ) -> Result<Vec<(String, i64)>, AppError> {
        let mut stmt = conn
            .prepare(
                "SELECT DISTINCT content_hash, size_bytes FROM attachments \
                 ORDER BY content_hash, size_bytes",
            )
            .map_err(map_rusqlite)?;
        let rows = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .map_err(map_rusqlite)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(map_rusqlite)?;
        Ok(rows)
    }
}
