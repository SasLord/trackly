//! SQLite-адаптер переписки по заявкам (V054, Phase 41.5).
//!
//! Нулевой по размеру репозиторий со встроенными функциями над `&Connection`
//! (`Transaction` разыменовывается). Все запросы параметризованы. Сообщения живут
//! в `chat_messages`; в `audit_log` репозиторий только читает (слияние на чтении, D-11).

use std::collections::HashSet;

use rusqlite::{params, Connection, OptionalExtension, Row};
use trackly_core::error::AppError;

use crate::error_conversions::map_rusqlite;
use crate::repos::attachments_sqlite::AttachmentRow;
use crate::repos::repairs_sqlite::request_visibility_clause;

/// SQLite-backed chat repository (zero-sized).
#[derive(Debug, Default, Clone)]
pub struct SqliteChatRepository;

/// Новое сообщение.
#[derive(Debug, Clone)]
pub struct ChatMessageNew<'a> {
    pub request_id: i64,
    pub author_user_id: Option<i64>,
    pub body: &'a str,
    pub created_at_utc: i64,
    pub moved_from_request_id: Option<i64>,
}

/// Сообщение с именем и ролью автора.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatMessageRow {
    pub id: i64,
    pub request_id: i64,
    pub author_user_id: Option<i64>,
    pub author_name: Option<String>,
    pub author_role: Option<String>,
    pub body: String,
    pub created_at_utc: i64,
    pub moved_from_request_id: Option<i64>,
}

/// Событие аудита для ленты.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatEventRow {
    pub id: i64,
    pub action: String,
    pub user_id: Option<i64>,
    pub user_name: Option<String>,
    pub payload_json: Option<String>,
    pub created_at_utc: i64,
}

/// Отметка прочтения по паре (пользователь, заявка).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadMark {
    pub last_message_id: i64,
    pub last_audit_id: i64,
}

/// Маркер файла сообщения в payload аудита (D-16): такая строка не даёт второго +1.
const MESSAGE_MARKER_FILTER: &str = "COALESCE(a.payload_json, '') NOT LIKE '%\"message_id\"%'";

const MSG_SELECT: &str = "SELECT m.id, m.request_id, m.author_user_id, u.full_name, u.role, \
    m.body, m.created_at_utc, m.moved_from_request_id \
    FROM chat_messages m LEFT JOIN users u ON u.id = m.author_user_id";

fn map_msg(r: &Row<'_>) -> rusqlite::Result<ChatMessageRow> {
    Ok(ChatMessageRow {
        id: r.get(0)?,
        request_id: r.get(1)?,
        author_user_id: r.get(2)?,
        author_name: r.get(3)?,
        author_role: r.get(4)?,
        body: r.get(5)?,
        created_at_utc: r.get(6)?,
        moved_from_request_id: r.get(7)?,
    })
}

const ATT_SELECT: &str = "SELECT a.id, a.parent_kind, a.parent_id, a.original_name, a.mime, \
    a.size_bytes, a.content_hash, a.uploaded_by_user_id, u.full_name, a.created_at_utc, \
    a.deleted_at_utc, a.deleted_by_user_id, d.full_name, a.message_id \
    FROM attachments a \
    LEFT JOIN users u ON u.id = a.uploaded_by_user_id \
    LEFT JOIN users d ON d.id = a.deleted_by_user_id";

fn map_att(r: &Row<'_>) -> rusqlite::Result<AttachmentRow> {
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
        message_id: r.get(13)?,
    })
}

const BASELINE_SQL: &str = "COALESCE((SELECT CAST(value AS INTEGER) FROM app_settings \
    WHERE key = 'chat_unread_baseline_audit_id'), 0)";

impl SqliteChatRepository {
    pub fn insert_message_on(
        &self,
        conn: &Connection,
        n: &ChatMessageNew<'_>,
    ) -> Result<i64, AppError> {
        conn.execute(
            "INSERT INTO chat_messages (request_id, author_user_id, body, created_at_utc, \
             moved_from_request_id) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                n.request_id,
                n.author_user_id,
                n.body,
                n.created_at_utc,
                n.moved_from_request_id
            ],
        )
        .map_err(map_rusqlite)?;
        Ok(conn.last_insert_rowid())
    }

    pub fn get_message_on(
        &self,
        conn: &Connection,
        id: i64,
    ) -> Result<Option<ChatMessageRow>, AppError> {
        conn.query_row(
            &format!("{MSG_SELECT} WHERE m.id = ?1"),
            params![id],
            map_msg,
        )
        .optional()
        .map_err(map_rusqlite)
    }

    /// Сообщения заявки по возрастанию id.
    pub fn list_messages_on(
        &self,
        conn: &Connection,
        request_id: i64,
    ) -> Result<Vec<ChatMessageRow>, AppError> {
        let mut stmt = conn
            .prepare(&format!(
                "{MSG_SELECT} WHERE m.request_id = ?1 ORDER BY m.id ASC"
            ))
            .map_err(map_rusqlite)?;
        let rows = stmt
            .query_map(params![request_id], map_msg)
            .map_err(map_rusqlite)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(map_rusqlite)?;
        Ok(rows)
    }

    /// События аудита заявки для ленты (по возрастанию времени).
    pub fn list_events_on(
        &self,
        conn: &Connection,
        request_id: i64,
    ) -> Result<Vec<ChatEventRow>, AppError> {
        let sql = format!(
            "SELECT a.id, a.action, a.user_id, u.full_name, a.payload_json, a.created_at_utc \
               FROM audit_log a \
               LEFT JOIN users u ON u.id = a.user_id \
              WHERE a.entity_type = 'request' \
                AND a.entity_id = ?1 \
                AND a.action NOT IN ('list', 'get') \
                AND a.action NOT LIKE 'custom:attachment_%' \
                AND {MESSAGE_MARKER_FILTER} \
              ORDER BY a.created_at_utc ASC, a.id ASC"
        );
        let mut stmt = conn.prepare(&sql).map_err(map_rusqlite)?;
        let rows = stmt
            .query_map(params![request_id], |r| {
                Ok(ChatEventRow {
                    id: r.get(0)?,
                    action: r.get(1)?,
                    user_id: r.get(2)?,
                    user_name: r.get(3)?,
                    payload_json: r.get(4)?,
                    created_at_utc: r.get(5)?,
                })
            })
            .map_err(map_rusqlite)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(map_rusqlite)?;
        Ok(rows)
    }

    /// События вложений заявки (без файлов сообщений) с id аудита:
    /// `(audit_id, user_id, action, created_at_utc, payload_json)`; нужны для
    /// вычисления первого непрочитанного элемента ленты (D-18).
    #[allow(clippy::type_complexity)]
    pub fn list_attachment_audit_on(
        &self,
        conn: &Connection,
        request_id: i64,
    ) -> Result<Vec<(i64, Option<i64>, String, i64, Option<String>)>, AppError> {
        let sql = format!(
            "SELECT a.id, a.user_id, a.action, a.created_at_utc, a.payload_json \
               FROM audit_log a \
              WHERE a.entity_type = 'request' AND a.entity_id = ?1 \
                AND a.action LIKE 'custom:attachment_%' \
                AND {MESSAGE_MARKER_FILTER} \
              ORDER BY a.created_at_utc ASC, a.id ASC"
        );
        let mut stmt = conn.prepare(&sql).map_err(map_rusqlite)?;
        let rows = stmt
            .query_map(params![request_id], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
            })
            .map_err(map_rusqlite)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(map_rusqlite)?;
        Ok(rows)
    }

    /// Id вложений заявки, привязанных к сообщениям (включая мягко удалённые).
    pub fn message_attachment_ids_on(
        &self,
        conn: &Connection,
        request_id: i64,
    ) -> Result<HashSet<i64>, AppError> {
        let mut stmt = conn
            .prepare(
                "SELECT id FROM attachments WHERE parent_kind = 'request' AND parent_id = ?1 \
                 AND message_id IS NOT NULL",
            )
            .map_err(map_rusqlite)?;
        let ids = stmt
            .query_map(params![request_id], |r| r.get::<_, i64>(0))
            .map_err(map_rusqlite)?
            .collect::<Result<HashSet<_>, _>>()
            .map_err(map_rusqlite)?;
        Ok(ids)
    }

    /// Водяной знак аудита; нет строки = 0.
    pub fn baseline_audit_id_on(&self, conn: &Connection) -> Result<i64, AppError> {
        conn.query_row(&format!("SELECT {BASELINE_SQL}"), [], |r| r.get(0))
            .map_err(map_rusqlite)
    }

    pub fn get_read_on(
        &self,
        conn: &Connection,
        user_id: i64,
        request_id: i64,
    ) -> Result<Option<ReadMark>, AppError> {
        conn.query_row(
            "SELECT last_message_id, last_audit_id FROM chat_reads \
             WHERE user_id = ?1 AND request_id = ?2",
            params![user_id, request_id],
            |r| {
                Ok(ReadMark {
                    last_message_id: r.get(0)?,
                    last_audit_id: r.get(1)?,
                })
            },
        )
        .optional()
        .map_err(map_rusqlite)
    }

    /// Отметка с учётом водяного знака: нет строки = (0, baseline).
    pub fn effective_read_on(
        &self,
        conn: &Connection,
        user_id: i64,
        request_id: i64,
    ) -> Result<ReadMark, AppError> {
        if let Some(m) = self.get_read_on(conn, user_id, request_id)? {
            return Ok(m);
        }
        Ok(ReadMark {
            last_message_id: 0,
            last_audit_id: self.baseline_audit_id_on(conn)?,
        })
    }

    /// `(MAX(chat_messages.id), MAX(audit_log.id))` по заявке, 0 если пусто.
    pub fn max_ids_on(&self, conn: &Connection, request_id: i64) -> Result<(i64, i64), AppError> {
        conn.query_row(
            "SELECT \
               COALESCE((SELECT MAX(id) FROM chat_messages WHERE request_id = ?1), 0), \
               COALESCE((SELECT MAX(id) FROM audit_log \
                          WHERE entity_type = 'request' AND entity_id = ?1), 0)",
            params![request_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(map_rusqlite)
    }

    pub fn upsert_read_on(
        &self,
        conn: &Connection,
        user_id: i64,
        request_id: i64,
        mark: ReadMark,
        now: i64,
    ) -> Result<(), AppError> {
        conn.execute(
            "INSERT INTO chat_reads (user_id, request_id, last_message_id, last_audit_id, \
             updated_at_utc) VALUES (?1, ?2, ?3, ?4, ?5) \
             ON CONFLICT(user_id, request_id) DO UPDATE SET \
               last_message_id = excluded.last_message_id, \
               last_audit_id = excluded.last_audit_id, \
               updated_at_utc = excluded.updated_at_utc",
            params![
                user_id,
                request_id,
                mark.last_message_id,
                mark.last_audit_id,
                now
            ],
        )
        .map_err(map_rusqlite)?;
        Ok(())
    }

    /// Непрочитанное по заявкам: `(request_id, count)`, только count > 0.
    /// Свои сообщения и события не считаются (D-17); для не-staff действует
    /// `request_visibility_clause` (D-20).
    pub fn unread_summary_on(
        &self,
        conn: &Connection,
        user_id: i64,
        staff: bool,
    ) -> Result<Vec<(i64, i64)>, AppError> {
        let visibility = if staff {
            String::new()
        } else {
            format!(" AND {}", request_visibility_clause("?1"))
        };
        let sql = format!(
            "SELECT id, cnt FROM ( \
               SELECT r.id AS id, \
                 (SELECT COUNT(*) FROM chat_messages m \
                   WHERE m.request_id = r.id \
                     AND m.author_user_id IS NOT ?1 \
                     AND m.id > COALESCE(cr.last_message_id, 0)) \
               + (SELECT COUNT(*) FROM audit_log a \
                   WHERE a.entity_type = 'request' AND a.entity_id = r.id \
                     AND a.action NOT IN ('list', 'get') \
                     AND a.user_id IS NOT ?1 \
                     AND a.id > COALESCE(cr.last_audit_id, {BASELINE_SQL}) \
                     AND {MESSAGE_MARKER_FILTER}) AS cnt \
                 FROM requests r \
                 LEFT JOIN chat_reads cr ON cr.request_id = r.id AND cr.user_id = ?1 \
                WHERE r.deleted_at_utc IS NULL AND r.request_type <> 'ad_register'{visibility} \
             ) WHERE cnt > 0 ORDER BY id"
        );
        let mut stmt = conn.prepare(&sql).map_err(map_rusqlite)?;
        let rows = stmt
            .query_map(params![user_id], |r| Ok((r.get(0)?, r.get(1)?)))
            .map_err(map_rusqlite)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(map_rusqlite)?;
        Ok(rows)
    }

    /// Черновики вложений пользователя по заявке.
    pub fn list_drafts_on(
        &self,
        conn: &Connection,
        request_id: i64,
        user_id: i64,
    ) -> Result<Vec<AttachmentRow>, AppError> {
        let mut stmt = conn
            .prepare(&format!(
                "{ATT_SELECT} WHERE a.parent_kind = 'message_draft' AND a.parent_id = ?1 \
                 AND a.uploaded_by_user_id = ?2 AND a.message_id IS NULL \
                 AND a.deleted_at_utc IS NULL ORDER BY a.id"
            ))
            .map_err(map_rusqlite)?;
        let rows = stmt
            .query_map(params![request_id, user_id], map_att)
            .map_err(map_rusqlite)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(map_rusqlite)?;
        Ok(rows)
    }

    /// Повышает черновики до вложений заявки с `message_id`. Любой id, не
    /// удовлетворяющий условию черновика, даёт `Forbidden`; вызывающий откатывает
    /// транзакцию (T-41.5-08).
    pub fn promote_drafts_on(
        &self,
        conn: &Connection,
        request_id: i64,
        user_id: i64,
        draft_ids: &[i64],
        message_id: i64,
    ) -> Result<Vec<AttachmentRow>, AppError> {
        let mut out = Vec::with_capacity(draft_ids.len());
        for &id in draft_ids {
            let n = conn
                .execute(
                    "UPDATE attachments SET parent_kind = 'request', message_id = ?1 \
                     WHERE id = ?2 AND parent_kind = 'message_draft' AND parent_id = ?3 \
                       AND uploaded_by_user_id = ?4 AND message_id IS NULL \
                       AND deleted_at_utc IS NULL",
                    params![message_id, id, request_id, user_id],
                )
                .map_err(map_rusqlite)?;
            if n != 1 {
                return Err(AppError::Forbidden);
            }
            let row = conn
                .query_row(
                    &format!("{ATT_SELECT} WHERE a.id = ?1"),
                    params![id],
                    map_att,
                )
                .map_err(map_rusqlite)?;
            out.push(row);
        }
        Ok(out)
    }

    pub fn delete_draft_rows_on(&self, conn: &Connection, ids: &[i64]) -> Result<(), AppError> {
        for &id in ids {
            conn.execute(
                "DELETE FROM attachments WHERE id = ?1 AND parent_kind = 'message_draft'",
                params![id],
            )
            .map_err(map_rusqlite)?;
        }
        Ok(())
    }
}
