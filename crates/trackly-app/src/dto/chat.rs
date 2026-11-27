//! Chat DTOs (Phase 41.5) - shared by Tauri commands and axum handlers.
//!
//! The feed is assembled on the server (D-11); the client only renders it.
//! No content hash is exposed: attachments reuse `AttachmentDto` (D-04, 41.3).

use serde::{Deserialize, Serialize};
use specta::Type;

use super::attachment::{AttachmentDto, AttachmentHistoryEventDto};

/// One chat message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessageDto {
    /// Stable key `m{id}`.
    pub key: String,
    #[specta(type = i32)]
    pub id: i64,
    #[specta(type = i32)]
    pub created_at_utc: i64,
    #[specta(type = Option<i32>)]
    pub actor_user_id: Option<i64>,
    pub actor_name: Option<String>,
    /// "admin" | "manager" | "requester" | "participant"; `None` for a deleted user.
    pub author_role: Option<String>,
    pub is_own: bool,
    pub text: String,
    #[specta(type = Option<i32>)]
    pub moved_from_request_id: Option<i64>,
    /// Клиентский id отправки; только автору, как в `ImMessageDto` (D-11, D-12).
    pub client_msg_id: Option<String>,
    pub attachments: Vec<AttachmentDto>,
}

/// One request event (audit row) in the feed. Labels are chosen by the UI by `action`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ChatEventDto {
    /// Stable key `e{id}`.
    pub key: String,
    #[specta(type = i32)]
    pub id: i64,
    #[specta(type = i32)]
    pub created_at_utc: i64,
    #[specta(type = Option<i32>)]
    pub actor_user_id: Option<i64>,
    pub actor_name: Option<String>,
    pub action: String,
    pub notes: Option<String>,
    #[specta(type = Option<i32>)]
    pub related_request_id: Option<i64>,
    /// Resolution block shows the notes only on the last complete/reject/repair_complete (D-12).
    pub show_notes: bool,
}

/// Attachment event of the request (added/deleted/restored/purged), D-13.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ChatAttachmentEventDto {
    /// Stable key `a{attachment_id}:{action}:{created_at_utc}`.
    pub key: String,
    #[specta(type = i32)]
    pub created_at_utc: i64,
    pub event: AttachmentHistoryEventDto,
}

/// Feed item; the discriminator is `kind`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ChatItemDto {
    Message(ChatMessageDto),
    Event(ChatEventDto),
    AttachmentEvent(ChatAttachmentEventDto),
}

impl ChatItemDto {
    pub fn key(&self) -> &str {
        match self {
            ChatItemDto::Message(m) => &m.key,
            ChatItemDto::Event(e) => &e.key,
            ChatItemDto::AttachmentEvent(a) => &a.key,
        }
    }
}

/// Whole feed of a request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ChatFeedDto {
    pub items: Vec<ChatItemDto>,
    /// The request is open and the caller may post (D-07).
    pub can_post: bool,
    #[specta(type = i32)]
    pub max_size_bytes: i64,
    /// Key of the first unread item before the read mark moves (D-18).
    pub first_unread_id: Option<String>,
    /// Resolution text of the request (D-12), shown in the resolution block.
    pub resolution_notes: Option<String>,
}

/// Outcome of sending a message (used by the send path).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ChatSendOutcomeDto {
    pub message: Option<ChatMessageDto>,
    pub reject_code: Option<String>,
}

/// Unread count of one request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct UnreadItemDto {
    #[specta(type = i32)]
    pub request_id: i64,
    #[specta(type = i32)]
    pub count: i32,
}

/// Unread summary of the caller (D-19, D-20).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct UnreadSummaryDto {
    pub items: Vec<UnreadItemDto>,
    pub requests_with_unread: i32,
    /// D-G3: requests whose notifications the caller muted (server-side, all devices).
    #[specta(type = Vec<i32>)]
    pub muted_request_ids: Vec<i64>,
}
