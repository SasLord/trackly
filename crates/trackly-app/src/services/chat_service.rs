//! `ChatService` - read side of the request chat (Phase 41.5).
//!
//! `authorize` is the single access check (D-06): feed, mark_read and the
//! unread summary scope all go through it, for both transports. The feed is
//! merged on read from `chat_messages`, `audit_log` and attachment events
//! (D-11); the service never copies messages into the audit log and never
//! offers edit/delete of a message (D-22).

use std::collections::HashMap;
use std::sync::Arc;

use rusqlite::params;
use trackly_core::auth::{Identity, Role};
use trackly_core::domain::chat::{author_role_label, can_post_status, request_type_has_chat};
use trackly_core::error::AppError;
use trackly_core::primitives::clock::Clock;
use trackly_infra::db::{pools::ReaderPool, writer_worker::WriterHandle};
use trackly_infra::error_conversions::map_rusqlite;
use trackly_infra::repos::chat_sqlite::{ReadMark, SqliteChatRepository};
use trackly_infra::repos::repairs_sqlite::SqliteRepairRepository;

use super::attachment_service::{AttachmentService, ParentRef};
use super::request_service::{notes_of, related_request_id_of};
use crate::dto::attachment::AttachmentDto;
use crate::dto::chat::{
    ChatAttachmentEventDto, ChatEventDto, ChatFeedDto, ChatItemDto, ChatMessageDto, UnreadItemDto,
    UnreadSummaryDto,
};
use crate::dto::printer::WsEvent;
use trackly_core::domain::attachments::ParentKind;

/// Audit actions that carry the request resolution (D-12).
const RESOLUTION_ACTIONS: [&str; 4] = [
    "complete",
    "reject",
    "repair_complete",
    "custom:repair_complete",
];

/// What `authorize` learned about the request.
#[derive(Debug, Clone)]
pub(crate) struct Access {
    pub requested_by: Option<i64>,
    pub participants: Vec<i64>,
    #[allow(dead_code)]
    pub request_type: String,
    pub status: String,
    pub resolution_notes: Option<String>,
}

fn internal(e: tokio::task::JoinError) -> AppError {
    AppError::Internal {
        source_chain: format!("spawn_blocking: {e}"),
    }
}

fn is_staff(caller: &Identity) -> bool {
    matches!(caller.role, Role::Admin | Role::Manager)
}

/// Application service for the request chat. `Arc` fields keep `Clone` O(1).
#[derive(Clone)]
pub struct ChatService {
    pub writer: Arc<WriterHandle>,
    pub readers: Arc<ReaderPool>,
    pub(crate) clock: Arc<dyn Clock + Send + Sync>,
    pub(crate) attachments: Arc<AttachmentService>,
    pub(crate) repo: Arc<SqliteChatRepository>,
    /// Shared WS sender; `None` in fixtures, wired by `AppCtx::build`.
    pub(crate) ws_tx: Option<Arc<tokio::sync::broadcast::Sender<WsEvent>>>,
}

impl ChatService {
    pub fn new(
        writer: Arc<WriterHandle>,
        readers: Arc<ReaderPool>,
        clock: Arc<dyn Clock + Send + Sync>,
        attachments: Arc<AttachmentService>,
    ) -> Self {
        Self {
            writer,
            readers,
            clock,
            attachments,
            repo: Arc::new(SqliteChatRepository),
            ws_tx: None,
        }
    }

    /// Builder: wire the shared WS broadcast sender.
    #[must_use]
    pub fn with_ws_tx(mut self, ws_tx: Arc<tokio::sync::broadcast::Sender<WsEvent>>) -> Self {
        self.ws_tx = Some(ws_tx);
        self
    }

    /// Single access check (D-06). `NotFound` for a missing/soft-deleted
    /// request, `Forbidden` for `ad_register` (D-05, everyone including Admin)
    /// and for a caller who is neither staff, author nor participant.
    pub(crate) async fn authorize(
        &self,
        caller: &Identity,
        request_id: i64,
    ) -> Result<Access, AppError> {
        let readers = self.readers.clone();
        let access = tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            let row: Option<(Option<i64>, String, String, Option<String>)> = {
                use rusqlite::OptionalExtension;
                conn.query_row(
                    "SELECT requested_by_user_id, request_type, status, resolution_notes \
                       FROM requests WHERE id = ?1 AND deleted_at_utc IS NULL",
                    params![request_id],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
                )
                .optional()
                .map_err(map_rusqlite)?
            };
            let (requested_by, request_type, status, resolution_notes) =
                row.ok_or(AppError::NotFound {
                    entity: "request",
                    id: request_id,
                })?;
            let participants = SqliteRepairRepository.participant_user_ids(&conn, request_id)?;
            Ok::<_, AppError>(Access {
                requested_by,
                participants,
                request_type,
                status,
                resolution_notes,
            })
        })
        .await
        .map_err(internal)??;
        if !request_type_has_chat(&access.request_type) {
            return Err(AppError::Forbidden);
        }
        if !is_staff(caller) {
            // Employee without a user id has no identity to match (Pitfall 7).
            let Some(uid) = caller.user_id else {
                return Err(AppError::Forbidden);
            };
            if access.requested_by != Some(uid) && !access.participants.contains(&uid) {
                return Err(AppError::Forbidden);
            }
        }
        Ok(access)
    }

    /// Whole feed of a request: messages, request events and attachment
    /// events in one order (D-11). Writes nothing.
    pub async fn feed(&self, caller: &Identity, request_id: i64) -> Result<ChatFeedDto, AppError> {
        let access = self.authorize(caller, request_id).await?;
        let caller_id = caller.user_id;

        let readers = self.readers.clone();
        let repo = self.repo.clone();
        let (messages, events, msg_atts, msg_att_ids, att_audit, read) =
            tokio::task::spawn_blocking(move || {
                let conn = readers.acquire();
                let messages = repo.list_messages_on(&conn, request_id)?;
                let events = repo.list_events_on(&conn, request_id)?;
                let msg_atts = trackly_infra::repos::attachments_sqlite::SqliteAttachmentRepository
                    .list_message_attachments_on(&conn, request_id)?;
                let msg_att_ids = repo.message_attachment_ids_on(&conn, request_id)?;
                let att_audit = repo.list_attachment_audit_on(&conn, request_id)?;
                let read = match caller_id {
                    Some(uid) => Some(repo.effective_read_on(&conn, uid, request_id)?),
                    None => None,
                };
                Ok::<_, AppError>((messages, events, msg_atts, msg_att_ids, att_audit, read))
            })
            .await
            .map_err(internal)??;

        let history = self
            .attachments
            .history(
                caller,
                ParentRef {
                    kind: ParentKind::Request,
                    id: request_id,
                },
            )
            .await?;
        let max_size_bytes = self.attachments.effective_limit().await?;

        // Attachments of messages, grouped by message id (Admin also sees soft-deleted).
        let admin = matches!(caller.role, Role::Admin);
        let mut by_message: HashMap<i64, Vec<AttachmentDto>> = HashMap::new();
        for row in &msg_atts {
            if row.deleted_at_utc.is_some() && !admin {
                continue;
            }
            if let Some(mid) = row.message_id {
                by_message
                    .entry(mid)
                    .or_default()
                    .push(self.attachments.to_dto(row));
            }
        }

        // Last resolution event gets show_notes (D-12).
        let last_resolution = events
            .iter()
            .rev()
            .find(|e| RESOLUTION_ACTIONS.contains(&e.action.as_str()))
            .map(|e| e.id);

        // (sort tuple, item)
        let mut rows: Vec<((i64, u8, i64, String), ChatItemDto)> = Vec::new();
        for m in &messages {
            let role = m.author_role.as_deref().and_then(|r| {
                author_role_label(
                    r,
                    m.author_user_id.is_some() && m.author_user_id == access.requested_by,
                    m.author_user_id
                        .is_some_and(|u| access.participants.contains(&u)),
                )
            });
            let dto = ChatMessageDto {
                key: format!("m{}", m.id),
                id: m.id,
                created_at_utc: m.created_at_utc,
                actor_user_id: m.author_user_id,
                actor_name: m.author_name.clone(),
                author_role: role.map(str::to_string),
                is_own: caller_id.is_some() && m.author_user_id == caller_id,
                text: m.body.clone(),
                moved_from_request_id: m.moved_from_request_id,
                attachments: by_message.remove(&m.id).unwrap_or_default(),
            };
            rows.push((
                (m.created_at_utc, 1, m.id, dto.key.clone()),
                ChatItemDto::Message(dto),
            ));
        }
        for e in &events {
            let dto = ChatEventDto {
                key: format!("e{}", e.id),
                id: e.id,
                created_at_utc: e.created_at_utc,
                actor_user_id: e.user_id,
                actor_name: e.user_name.clone(),
                action: e.action.clone(),
                notes: notes_of(e.payload_json.as_deref()),
                related_request_id: related_request_id_of(e.payload_json.as_deref()),
                show_notes: last_resolution == Some(e.id),
            };
            rows.push((
                (e.created_at_utc, 0, e.id, dto.key.clone()),
                ChatItemDto::Event(dto),
            ));
        }
        // D-13/D-16: files of messages show in the bubble, not as separate blocks.
        // key -> (audit id, audit user) for the unread computation.
        let mut att_sigs: HashMap<String, (i64, Option<i64>)> = HashMap::new();
        for ev in history {
            if msg_att_ids.contains(&ev.attachment_id) {
                continue;
            }
            let audit = att_audit.iter().find(|(_, _, action, at, payload)| {
                *at == ev.created_at_utc
                    && action.strip_prefix("custom:attachment_") == Some(ev.action.as_str())
                    && payload
                        .as_deref()
                        .and_then(|p| serde_json::from_str::<serde_json::Value>(p).ok())
                        .and_then(|v| v.get("attachment_id").and_then(|x| x.as_i64()))
                        == Some(ev.attachment_id)
            });
            let key = format!("a{}:{}:{}", ev.attachment_id, ev.action, ev.created_at_utc);
            let dto = ChatAttachmentEventDto {
                key: key.clone(),
                created_at_utc: ev.created_at_utc,
                event: ev,
            };
            let at = dto.created_at_utc;
            let sig = audit.map(|(id, uid, _, _, _)| (*id, *uid));
            if let Some(sig) = sig {
                att_sigs.insert(key.clone(), sig);
            }
            rows.push(((at, 0, 0, key), ChatItemDto::AttachmentEvent(dto)));
        }
        rows.sort_by(|a, b| a.0.cmp(&b.0));

        // D-18: first foreign item above the read mark.
        let first_unread_id = match (caller_id, read) {
            (Some(uid), Some(mark)) => rows.iter().find_map(|(_, item)| {
                let unread = match item {
                    ChatItemDto::Message(m) => {
                        m.actor_user_id != Some(uid) && m.id > mark.last_message_id
                    }
                    ChatItemDto::Event(e) => {
                        e.actor_user_id != Some(uid) && e.id > mark.last_audit_id
                    }
                    ChatItemDto::AttachmentEvent(a) => att_sigs
                        .get(&a.key)
                        .is_some_and(|(aid, auid)| *auid != Some(uid) && *aid > mark.last_audit_id),
                };
                unread.then(|| item.key().to_string())
            }),
            _ => None,
        };

        Ok(ChatFeedDto {
            items: rows.into_iter().map(|(_, i)| i).collect(),
            can_post: can_post_status(&access.status),
            max_size_bytes,
            first_unread_id,
            resolution_notes: access.resolution_notes,
        })
    }

    /// Moves the caller's read mark to the current maxima (D-17/D-18) and tells
    /// the caller's other devices (D-19), only after the commit.
    pub async fn mark_read(&self, caller: &Identity, request_id: i64) -> Result<(), AppError> {
        self.authorize(caller, request_id).await?;
        let Some(user_id) = caller.user_id else {
            return Err(AppError::Forbidden);
        };
        let repo = self.repo.clone();
        let now = self.clock.unix_seconds();
        self.writer
            .execute(move |conn| {
                let (last_message_id, last_audit_id) = repo.max_ids_on(conn, request_id)?;
                repo.upsert_read_on(
                    conn,
                    user_id,
                    request_id,
                    ReadMark {
                        last_message_id,
                        last_audit_id,
                    },
                    now,
                )
            })
            .await?;
        if let Some(tx) = &self.ws_tx {
            let _ = tx.send(WsEvent::ChatReadChanged { user_id });
        }
        Ok(())
    }

    /// Unread counts per request (D-19, D-20). Staff sees every chat request,
    /// an employee only own and joined ones.
    pub async fn unread_summary(&self, caller: &Identity) -> Result<UnreadSummaryDto, AppError> {
        let Some(user_id) = caller.user_id else {
            return Ok(UnreadSummaryDto::default());
        };
        let staff = is_staff(caller);
        let readers = self.readers.clone();
        let repo = self.repo.clone();
        let rows = tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            repo.unread_summary_on(&conn, user_id, staff)
        })
        .await
        .map_err(internal)??;
        let items: Vec<UnreadItemDto> = rows
            .into_iter()
            .map(|(request_id, count)| UnreadItemDto {
                request_id,
                count: count as i32,
            })
            .collect();
        Ok(UnreadSummaryDto {
            requests_with_unread: items.len() as i32,
            items,
        })
    }
}
