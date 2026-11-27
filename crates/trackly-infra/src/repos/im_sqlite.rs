//! SQLite-адаптер ядра мессенджера im_* (V058, Phase 43).
//!
//! Нулевой по размеру репозиторий со встроенными функциями над `&Connection`
//! (`Transaction` разыменовывается). Все запросы параметризованы. Бизнес-проверок
//! членства и публикации событий здесь нет — это слой сервиса.

use rusqlite::{params, Connection, OptionalExtension, Row};
use trackly_core::domain::im::VisibleWindow;
use trackly_core::error::AppError;

use crate::error_conversions::map_rusqlite;

/// SQLite-backed IM repository (zero-sized).
#[derive(Debug, Default, Clone)]
pub struct SqliteImRepository;

/// Беседа.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImConversationRow {
    pub id: i64,
    pub kind: String,
    pub title: Option<String>,
    pub dialog_key: Option<String>,
    pub created_by_user_id: i64,
    pub created_at_utc: i64,
    pub last_seq: i64,
}

/// Строка участника.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImMemberRow {
    pub role: String,
    pub left_at_utc: Option<i64>,
    pub left_seq: Option<i64>,
    pub visible_from_seq: i64,
    pub last_read_seq: i64,
}

/// Сообщение.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImMessageRow {
    pub id: i64,
    pub conversation_id: i64,
    pub seq: i64,
    pub rev: i64,
    pub kind: String,
    pub author_user_id: Option<i64>,
    pub client_msg_id: Option<String>,
    pub body_json: String,
    pub body_text: String,
    pub created_at_utc: i64,
}

/// Новое сообщение.
#[derive(Debug, Clone)]
pub struct ImMessageNew<'a> {
    pub conversation_id: i64,
    pub seq: i64,
    pub rev: i64,
    pub kind: &'a str,
    pub author_user_id: Option<i64>,
    pub client_msg_id: Option<&'a str>,
    pub body_json: &'a str,
    pub body_text: &'a str,
    pub created_at_utc: i64,
}

const MSG_COLS: &str = "id, conversation_id, seq, rev, kind, author_user_id, client_msg_id, \
    body_json, body_text, created_at_utc";

fn map_msg(r: &Row<'_>) -> rusqlite::Result<ImMessageRow> {
    Ok(ImMessageRow {
        id: r.get(0)?,
        conversation_id: r.get(1)?,
        seq: r.get(2)?,
        rev: r.get(3)?,
        kind: r.get(4)?,
        author_user_id: r.get(5)?,
        client_msg_id: r.get(6)?,
        body_json: r.get(7)?,
        body_text: r.get(8)?,
        created_at_utc: r.get(9)?,
    })
}

fn map_conv(r: &Row<'_>) -> rusqlite::Result<ImConversationRow> {
    Ok(ImConversationRow {
        id: r.get(0)?,
        kind: r.get(1)?,
        title: r.get(2)?,
        dialog_key: r.get(3)?,
        created_by_user_id: r.get(4)?,
        created_at_utc: r.get(5)?,
        last_seq: r.get(6)?,
    })
}

fn map_member(r: &Row<'_>, off: usize) -> rusqlite::Result<ImMemberRow> {
    Ok(ImMemberRow {
        role: r.get(off)?,
        left_at_utc: r.get(off + 1)?,
        left_seq: r.get(off + 2)?,
        visible_from_seq: r.get(off + 3)?,
        last_read_seq: r.get(off + 4)?,
    })
}

/// Предикат окна: `seq >= lo`, `seq <= hi`, `rev <= max_rev` (NULL = без границы).
/// Параметры ?2 (lo), ?3 (hi), ?4 (max_rev); ?1 — беседа.
const WINDOW_SQL: &str = "conversation_id = ?1 AND seq >= ?2 \
    AND (?3 IS NULL OR seq <= ?3) AND (?4 IS NULL OR rev <= ?4)";

impl SqliteImRepository {
    /// Любой Conflict (UNIQUE, CHECK, FK) при вставке. Безопасность держится на
    /// повторном поиске по ключу: не найден — исходная ошибка пробрасывается.
    pub fn is_constraint_conflict(err: &AppError) -> bool {
        matches!(err, AppError::Conflict { .. })
    }

    pub fn create_conversation_on(
        &self,
        conn: &Connection,
        kind: &str,
        title: Option<&str>,
        dialog_key: Option<&str>,
        created_by: i64,
        now: i64,
    ) -> Result<i64, AppError> {
        conn.execute(
            "INSERT INTO im_conversations (kind, title, dialog_key, created_by_user_id, \
             created_at_utc) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![kind, title, dialog_key, created_by, now],
        )
        .map_err(map_rusqlite)?;
        Ok(conn.last_insert_rowid())
    }

    pub fn insert_member_on(
        &self,
        conn: &Connection,
        conversation_id: i64,
        user_id: i64,
        role: &str,
        now: i64,
        visible_from_seq: i64,
    ) -> Result<(), AppError> {
        conn.execute(
            "INSERT INTO im_members (conversation_id, user_id, role, joined_at_utc, \
             visible_from_seq) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![conversation_id, user_id, role, now, visible_from_seq],
        )
        .map_err(map_rusqlite)?;
        Ok(())
    }

    pub fn member_row_on(
        &self,
        conn: &Connection,
        conversation_id: i64,
        user_id: i64,
    ) -> Result<Option<ImMemberRow>, AppError> {
        conn.query_row(
            "SELECT role, left_at_utc, left_seq, visible_from_seq, last_read_seq \
             FROM im_members WHERE conversation_id = ?1 AND user_id = ?2",
            params![conversation_id, user_id],
            |r| map_member(r, 0),
        )
        .optional()
        .map_err(map_rusqlite)
    }

    pub fn active_member_ids_on(
        &self,
        conn: &Connection,
        conversation_id: i64,
    ) -> Result<Vec<i64>, AppError> {
        let mut st = conn
            .prepare(
                "SELECT user_id FROM im_members WHERE conversation_id = ?1 \
                 AND left_at_utc IS NULL ORDER BY user_id",
            )
            .map_err(map_rusqlite)?;
        let rows = st
            .query_map([conversation_id], |r| r.get(0))
            .map_err(map_rusqlite)?;
        rows.collect::<Result<Vec<i64>, _>>().map_err(map_rusqlite)
    }

    /// `left_at_utc` и `left_seq` ставятся вместе (D-06).
    pub fn mark_left_on(
        &self,
        conn: &Connection,
        conversation_id: i64,
        user_id: i64,
        now: i64,
        left_seq: i64,
    ) -> Result<(), AppError> {
        conn.execute(
            "UPDATE im_members SET left_at_utc = ?3, left_seq = ?4 \
             WHERE conversation_id = ?1 AND user_id = ?2",
            params![conversation_id, user_id, now, left_seq],
        )
        .map_err(map_rusqlite)?;
        Ok(())
    }

    pub fn rejoin_member_on(
        &self,
        conn: &Connection,
        conversation_id: i64,
        user_id: i64,
        visible_from_seq: i64,
    ) -> Result<(), AppError> {
        conn.execute(
            "UPDATE im_members SET left_at_utc = NULL, left_seq = NULL, \
             visible_from_seq = ?3 WHERE conversation_id = ?1 AND user_id = ?2",
            params![conversation_id, user_id, visible_from_seq],
        )
        .map_err(map_rusqlite)?;
        Ok(())
    }

    /// D-04: новый seq/rev одним запросом; вызывать на соединении единственного
    /// писателя внутри транзакции (откат возвращает счётчик).
    pub fn bump_last_seq_on(
        &self,
        conn: &Connection,
        conversation_id: i64,
    ) -> Result<i64, AppError> {
        conn.query_row(
            "UPDATE im_conversations SET last_seq = last_seq + 1 WHERE id = ?1 \
             RETURNING last_seq",
            [conversation_id],
            |r| r.get(0),
        )
        .map_err(map_rusqlite)
    }

    pub fn last_seq_on(&self, conn: &Connection, conversation_id: i64) -> Result<i64, AppError> {
        conn.query_row(
            "SELECT last_seq FROM im_conversations WHERE id = ?1",
            [conversation_id],
            |r| r.get(0),
        )
        .map_err(map_rusqlite)
    }

    pub fn find_by_client_id_on(
        &self,
        conn: &Connection,
        conversation_id: i64,
        author_user_id: i64,
        client_msg_id: &str,
    ) -> Result<Option<ImMessageRow>, AppError> {
        conn.query_row(
            &format!(
                "SELECT {MSG_COLS} FROM im_messages WHERE conversation_id = ?1 \
                 AND author_user_id = ?2 AND client_msg_id = ?3"
            ),
            params![conversation_id, author_user_id, client_msg_id],
            map_msg,
        )
        .optional()
        .map_err(map_rusqlite)
    }

    pub fn insert_message_on(
        &self,
        conn: &Connection,
        n: &ImMessageNew<'_>,
    ) -> Result<i64, AppError> {
        conn.execute(
            "INSERT INTO im_messages (conversation_id, seq, rev, kind, author_user_id, \
             client_msg_id, body_json, body_text, created_at_utc) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                n.conversation_id,
                n.seq,
                n.rev,
                n.kind,
                n.author_user_id,
                n.client_msg_id,
                n.body_json,
                n.body_text,
                n.created_at_utc
            ],
        )
        .map_err(map_rusqlite)?;
        Ok(conn.last_insert_rowid())
    }

    pub fn get_message_on(
        &self,
        conn: &Connection,
        id: i64,
    ) -> Result<Option<ImMessageRow>, AppError> {
        conn.query_row(
            &format!("SELECT {MSG_COLS} FROM im_messages WHERE id = ?1"),
            [id],
            map_msg,
        )
        .optional()
        .map_err(map_rusqlite)
    }

    /// Беседы пользователя, включая те, из которых он вышел (для read_only).
    pub fn conversations_of_user_on(
        &self,
        conn: &Connection,
        user_id: i64,
    ) -> Result<Vec<(ImConversationRow, ImMemberRow)>, AppError> {
        let mut st = conn
            .prepare(
                "SELECT c.id, c.kind, c.title, c.dialog_key, c.created_by_user_id, \
                 c.created_at_utc, c.last_seq, m.role, m.left_at_utc, m.left_seq, \
                 m.visible_from_seq, m.last_read_seq \
                 FROM im_members m JOIN im_conversations c ON c.id = m.conversation_id \
                 WHERE m.user_id = ?1 ORDER BY c.id",
            )
            .map_err(map_rusqlite)?;
        let rows = st
            .query_map([user_id], |r| Ok((map_conv(r)?, map_member(r, 7)?)))
            .map_err(map_rusqlite)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(map_rusqlite)
    }

    /// Сообщения окна с `rev > after_rev`, по возрастанию rev.
    pub fn messages_after_rev_on(
        &self,
        conn: &Connection,
        conversation_id: i64,
        after_rev: i64,
        w: &VisibleWindow,
        limit: i64,
    ) -> Result<Vec<ImMessageRow>, AppError> {
        let mut st = conn
            .prepare(&format!(
                "SELECT {MSG_COLS} FROM im_messages WHERE {WINDOW_SQL} AND rev > ?5 \
                 ORDER BY rev ASC LIMIT ?6"
            ))
            .map_err(map_rusqlite)?;
        let rows = st
            .query_map(
                params![
                    conversation_id,
                    w.lo_seq,
                    w.hi_seq,
                    w.max_rev,
                    after_rev,
                    limit
                ],
                map_msg,
            )
            .map_err(map_rusqlite)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(map_rusqlite)
    }

    /// Окно целиком (без фильтра по rev клиента), по возрастанию seq.
    pub fn messages_window_on(
        &self,
        conn: &Connection,
        conversation_id: i64,
        w: &VisibleWindow,
        limit: i64,
    ) -> Result<Vec<ImMessageRow>, AppError> {
        let mut st = conn
            .prepare(&format!(
                "SELECT {MSG_COLS} FROM im_messages WHERE {WINDOW_SQL} \
                 ORDER BY seq ASC LIMIT ?5"
            ))
            .map_err(map_rusqlite)?;
        let rows = st
            .query_map(
                params![conversation_id, w.lo_seq, w.hi_seq, w.max_rev, limit],
                map_msg,
            )
            .map_err(map_rusqlite)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(map_rusqlite)
    }

    /// Последние `limit` сообщений окна по seq, отдаются по возрастанию.
    pub fn messages_last_page_on(
        &self,
        conn: &Connection,
        conversation_id: i64,
        w: &VisibleWindow,
        limit: i64,
    ) -> Result<Vec<ImMessageRow>, AppError> {
        let mut st = conn
            .prepare(&format!(
                "SELECT {MSG_COLS} FROM im_messages WHERE {WINDOW_SQL} \
                 ORDER BY seq DESC LIMIT ?5"
            ))
            .map_err(map_rusqlite)?;
        let rows = st
            .query_map(
                params![conversation_id, w.lo_seq, w.hi_seq, w.max_rev, limit],
                map_msg,
            )
            .map_err(map_rusqlite)?;
        let mut v = rows.collect::<Result<Vec<_>, _>>().map_err(map_rusqlite)?;
        v.reverse();
        Ok(v)
    }

    /// Число сообщений окна с `rev > after_rev` — для порога reload.
    pub fn count_after_rev_on(
        &self,
        conn: &Connection,
        conversation_id: i64,
        after_rev: i64,
        w: &VisibleWindow,
    ) -> Result<i64, AppError> {
        conn.query_row(
            &format!("SELECT COUNT(*) FROM im_messages WHERE {WINDOW_SQL} AND rev > ?5"),
            params![conversation_id, w.lo_seq, w.hi_seq, w.max_rev, after_rev],
            |r| r.get(0),
        )
        .map_err(map_rusqlite)
    }
}
