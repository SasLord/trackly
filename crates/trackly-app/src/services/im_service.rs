//! ImService — ядро мессенджера (Phase 43, D-01/D-02/D-04/D-05/D-06/D-11).
//!
//! `send` и `sync` — то, что уходит наружу (маршруты — План 08);
//! `create_conversation`/`add_member`/`remove_member` — сервисные функции для
//! тестов, маршрутов у них нет (D-02). Членство строгое: ни администратор, ни
//! менеджер не получают доступа к чужой беседе.

use std::collections::HashMap;
use std::sync::Arc;

use trackly_core::auth::Identity;
use trackly_core::domain::chat::{exceeds_limit, normalize_text};
use trackly_core::domain::im::{
    body_doc_json, can_post, clamp_page_limit, dialog_key, is_valid_client_id, member_window,
    SYNC_RELOAD_THRESHOLD,
};
use trackly_core::error::AppError;
use trackly_core::primitives::clock::Clock;
use trackly_infra::db::{pools::ReaderPool, writer_worker::WriterHandle};
use trackly_infra::error_conversions::map_rusqlite;
use trackly_infra::repos::im_sqlite::{ImMessageNew, ImMessageRow, SqliteImRepository};

use super::im_hub::{ImHub, ImSignal};
use crate::dto::im::{
    ImConversationDto, ImMessageDto, ImSendOutcomeDto, ImSyncConversationDto, ImSyncRequestDto,
    ImSyncResponseDto,
};

/// Не более стольких курсоров учитывается в одном sync (T-43-04-05).
const MAX_CURSORS: usize = 500;

fn internal(e: tokio::task::JoinError) -> AppError {
    AppError::Internal {
        source_chain: format!("spawn_blocking: {e}"),
    }
}

fn validation(field: &str, message: &str) -> AppError {
    AppError::Validation {
        field: field.to_string(),
        message: message.to_string(),
    }
}

enum Written {
    Sent(ImMessageRow, Vec<i64>),
    Replay(ImMessageRow),
}

fn to_dto(row: &ImMessageRow, caller: i64) -> ImMessageDto {
    let own = row.author_user_id == Some(caller);
    ImMessageDto {
        id: row.id,
        conversation_id: row.conversation_id,
        seq: row.seq,
        rev: row.rev,
        kind: row.kind.clone(),
        author_user_id: row.author_user_id,
        // client_msg_id — только автору (T-43-04-04).
        client_msg_id: if own { row.client_msg_id.clone() } else { None },
        is_own: own,
        body_json: row.body_json.clone(),
        body_text: row.body_text.clone(),
        created_at_utc: row.created_at_utc,
    }
}

#[derive(Clone)]
pub struct ImService {
    pub writer: Arc<WriterHandle>,
    pub readers: Arc<ReaderPool>,
    clock: Arc<dyn Clock + Send + Sync>,
    repo: Arc<SqliteImRepository>,
    /// `None` в фикстурах без хаба: публикация тихо пропускается.
    hub: Option<Arc<ImHub>>,
}

impl ImService {
    pub fn new(
        writer: Arc<WriterHandle>,
        readers: Arc<ReaderPool>,
        clock: Arc<dyn Clock + Send + Sync>,
    ) -> Self {
        Self {
            writer,
            readers,
            clock,
            repo: Arc::new(SqliteImRepository),
            hub: None,
        }
    }

    #[must_use]
    pub fn with_im_hub(mut self, hub: Arc<ImHub>) -> Self {
        self.hub = Some(hub);
        self
    }

    /// Отправка сообщения с идемпотентностью по (беседа, автор, client_msg_id).
    pub async fn send(
        &self,
        caller: &Identity,
        conversation_id: i64,
        client_msg_id: &str,
        text: &str,
    ) -> Result<ImSendOutcomeDto, AppError> {
        let Some(user_id) = caller.user_id else {
            return Err(AppError::Forbidden);
        };
        if !is_valid_client_id(client_msg_id) {
            return Err(validation("client_msg_id", "недопустимый идентификатор"));
        }
        let body = normalize_text(text);
        if body.is_empty() {
            return Err(validation("text", "пустое сообщение"));
        }
        if exceeds_limit(&body) {
            return Err(validation("text", "сообщение слишком длинное"));
        }
        let body_json = body_doc_json(&body);
        let client_id = client_msg_id.to_string();
        let repo = self.repo.clone();
        let now = self.clock.unix_seconds();

        let written = self
            .writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;
                // Состояние, от которого зависит решение, читается здесь, в писателе (TOCTOU).
                let member = repo.member_row_on(&tx, conversation_id, user_id)?.ok_or(
                    AppError::NotFound {
                        entity: "im_conversation",
                        id: conversation_id,
                    },
                )?;
                if !can_post(member.left_at_utc) {
                    return Err(AppError::Forbidden);
                }
                if let Some(existing) =
                    repo.find_by_client_id_on(&tx, conversation_id, user_id, &client_id)?
                {
                    return Ok(Written::Replay(existing));
                }
                let seq = repo.bump_last_seq_on(&tx, conversation_id)?;
                let inserted = repo.insert_message_on(
                    &tx,
                    &ImMessageNew {
                        conversation_id,
                        seq,
                        rev: seq,
                        kind: "text",
                        author_user_id: Some(user_id),
                        client_msg_id: Some(&client_id),
                        body_json: &body_json,
                        body_text: &body,
                        created_at_utc: now,
                    },
                );
                let id = match inserted {
                    Ok(id) => id,
                    Err(e) if SqliteImRepository::is_unique_conflict(&e) => {
                        // Гонка повтора: откатываем счётчик и отдаём оригинал.
                        drop(tx);
                        let existing = repo
                            .find_by_client_id_on(conn, conversation_id, user_id, &client_id)?
                            .ok_or(e)?;
                        return Ok(Written::Replay(existing));
                    }
                    Err(e) => return Err(e),
                };
                let recipients = repo.active_member_ids_on(&tx, conversation_id)?;
                let row = repo.get_message_on(&tx, id)?.ok_or(AppError::NotFound {
                    entity: "im_message",
                    id,
                })?;
                tx.commit().map_err(map_rusqlite)?;
                Ok(Written::Sent(row, recipients))
            })
            .await?;

        match written {
            Written::Replay(row) => Ok(ImSendOutcomeDto {
                message: to_dto(&row, user_id),
                replay: true,
            }),
            Written::Sent(row, recipients) => {
                // D-04: только после коммита, вне замыкания писателя; при replay — нет.
                if let Some(hub) = &self.hub {
                    hub.publish(
                        &recipients,
                        ImSignal::Changed {
                            conversation_id,
                            rev: row.rev,
                        },
                    );
                }
                Ok(ImSendOutcomeDto {
                    message: to_dto(&row, user_id),
                    replay: false,
                })
            }
        }
    }

    /// Только для тестов (D-02). dialog — ровно двое, идемпотентно по dialog_key.
    pub async fn create_conversation(
        &self,
        creator_user_id: i64,
        kind: &str,
        title: Option<&str>,
        member_user_ids: &[i64],
    ) -> Result<i64, AppError> {
        let mut all: Vec<i64> = vec![creator_user_id];
        for m in member_user_ids {
            if !all.contains(m) {
                all.push(*m);
            }
        }
        let is_dialog = match kind {
            "dialog" => true,
            "room" => false,
            _ => return Err(validation("kind", "dialog или room")),
        };
        if is_dialog && all.len() != 2 {
            return Err(validation("members", "диалог — ровно двое"));
        }
        let key = is_dialog.then(|| dialog_key(all[0], all[1]));
        let title = title.map(str::to_string);
        let kind = kind.to_string();
        let repo = self.repo.clone();
        let now = self.clock.unix_seconds();
        self.writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;
                if let Some(k) = &key {
                    use rusqlite::OptionalExtension;
                    let existing: Option<i64> = tx
                        .query_row(
                            "SELECT id FROM im_conversations WHERE dialog_key = ?1",
                            [k],
                            |r| r.get(0),
                        )
                        .optional()
                        .map_err(map_rusqlite)?;
                    if let Some(id) = existing {
                        return Ok(id);
                    }
                }
                let id = repo.create_conversation_on(
                    &tx,
                    &kind,
                    title.as_deref(),
                    key.as_deref(),
                    creator_user_id,
                    now,
                )?;
                for u in &all {
                    let role = if !is_dialog && *u == creator_user_id {
                        "creator"
                    } else {
                        "member"
                    };
                    repo.insert_member_on(&tx, id, *u, role, now, 0)?;
                }
                tx.commit().map_err(map_rusqlite)?;
                Ok(id)
            })
            .await
    }

    /// Только для тестов (D-02). Новый участник и вернувшийся видят беседу
    /// с `last_seq + 1`: пропущенное за время отсутствия не раскрывается.
    pub async fn add_member(&self, conversation_id: i64, user_id: i64) -> Result<(), AppError> {
        let repo = self.repo.clone();
        let now = self.clock.unix_seconds();
        self.writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;
                let last = repo.last_seq_on(&tx, conversation_id)?;
                match repo.member_row_on(&tx, conversation_id, user_id)? {
                    None => repo.insert_member_on(
                        &tx,
                        conversation_id,
                        user_id,
                        "member",
                        now,
                        last + 1,
                    )?,
                    Some(m) if m.left_at_utc.is_some() => {
                        repo.rejoin_member_on(&tx, conversation_id, user_id, last + 1)?
                    }
                    Some(_) => {}
                }
                tx.commit().map_err(map_rusqlite)?;
                Ok(())
            })
            .await
    }

    /// Только для тестов (D-02). Исключённому сигнал не посылается — он узнает
    /// при следующем sync/реконнекте. Точка расширения Фазы 47: системное
    /// сообщение о составе беседы.
    pub async fn remove_member(&self, conversation_id: i64, user_id: i64) -> Result<(), AppError> {
        let repo = self.repo.clone();
        let now = self.clock.unix_seconds();
        self.writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;
                let m = repo.member_row_on(&tx, conversation_id, user_id)?.ok_or(
                    AppError::NotFound {
                        entity: "im_member",
                        id: user_id,
                    },
                )?;
                if m.left_at_utc.is_none() {
                    let last = repo.last_seq_on(&tx, conversation_id)?;
                    repo.mark_left_on(&tx, conversation_id, user_id, now, last)?;
                }
                tx.commit().map_err(map_rusqlite)?;
                Ok(())
            })
            .await
    }

    /// Курсорная синхронизация по rev; чтение — одна согласованная транзакция.
    pub async fn sync(
        &self,
        caller: &Identity,
        req: &ImSyncRequestDto,
    ) -> Result<ImSyncResponseDto, AppError> {
        let Some(user_id) = caller.user_id else {
            return Err(AppError::Forbidden);
        };
        let limit = clamp_page_limit(req.limit);
        let mut cursors: HashMap<i64, i64> = HashMap::new();
        for c in req.cursors.iter().take(MAX_CURSORS) {
            let e = cursors.entry(c.conversation_id).or_insert(c.rev);
            *e = (*e).max(c.rev);
        }
        let repo = self.repo.clone();
        let readers = self.readers.clone();
        tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            let tx = conn.unchecked_transaction().map_err(map_rusqlite)?;
            let mut out = Vec::new();
            // Чужие беседы сюда не попадают вовсе: список строится от членства,
            // id из cursors без членства игнорируются молча.
            for (conv, m) in repo.conversations_of_user_on(&tx, user_id)? {
                let cursor = cursors.get(&conv.id).copied();
                let (messages, reload, last_seq, read_only) = match m.left_seq {
                    // D-06: исключённый получает замороженную историю один раз.
                    // Выборки «rev > cursor» здесь нет намеренно: правка после
                    // выхода (rev > left_seq) не должна просочиться.
                    // Принятое ограничение Фазы 43: сообщение, изменённое после
                    // выхода, не отдаётся вовсе — снимка ревизии нет, путей правки
                    // в 43 нет. Фаза 47 обязана добавить снимок ревизии или
                    // надгробие, чтобы «история до выхода видна» выполнялась и для
                    // изменённых сообщений.
                    Some(left_seq) if m.left_at_utc.is_some() => {
                        if cursor.is_some_and(|c| c >= left_seq) {
                            continue;
                        }
                        let w = member_window(m.visible_from_seq, Some(left_seq));
                        (
                            repo.messages_last_page_on(&tx, conv.id, &w, limit)?,
                            true,
                            left_seq,
                            true,
                        )
                    }
                    _ => {
                        let w = member_window(m.visible_from_seq, None);
                        match cursor {
                            None => (
                                repo.messages_last_page_on(&tx, conv.id, &w, limit)?,
                                false,
                                conv.last_seq,
                                false,
                            ),
                            Some(c) if c >= conv.last_seq => continue,
                            Some(c) => {
                                let n = repo.count_after_rev_on(&tx, conv.id, c, &w)?;
                                if n > SYNC_RELOAD_THRESHOLD {
                                    (
                                        repo.messages_last_page_on(&tx, conv.id, &w, limit)?,
                                        true,
                                        conv.last_seq,
                                        false,
                                    )
                                } else {
                                    // n <= порога: дельта целиком, без усечения limit.
                                    (
                                        repo.messages_after_rev_on(
                                            &tx,
                                            conv.id,
                                            c,
                                            &w,
                                            SYNC_RELOAD_THRESHOLD,
                                        )?,
                                        false,
                                        conv.last_seq,
                                        false,
                                    )
                                }
                            }
                        }
                    }
                };
                out.push(ImSyncConversationDto {
                    conversation: ImConversationDto {
                        id: conv.id,
                        kind: conv.kind,
                        title: conv.title,
                        last_seq,
                        last_read_seq: m.last_read_seq,
                        read_only,
                        left_seq: if read_only { m.left_seq } else { None },
                    },
                    messages: messages.iter().map(|r| to_dto(r, user_id)).collect(),
                    reload,
                });
            }
            Ok::<_, AppError>(ImSyncResponseDto { conversations: out })
        })
        .await
        .map_err(internal)?
    }
}
