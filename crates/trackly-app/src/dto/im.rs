//! DTO ядра мессенджера (Phase 43). Маршруты Tauri-близнецов не имеют (D-02),
//! поэтому specta не нужна — обычный serde.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImMessageDto {
    pub id: i64,
    pub conversation_id: i64,
    pub seq: i64,
    pub rev: i64,
    pub kind: String,
    pub author_user_id: Option<i64>,
    /// Отдаётся только автору сообщения.
    pub client_msg_id: Option<String>,
    pub is_own: bool,
    pub body_json: String,
    pub body_text: String,
    pub created_at_utc: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImSendRequestDto {
    pub conversation_id: i64,
    pub client_msg_id: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImSendOutcomeDto {
    pub message: ImMessageDto,
    pub replay: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImCursorDto {
    pub conversation_id: i64,
    pub rev: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImSyncRequestDto {
    #[serde(default)]
    pub cursors: Vec<ImCursorDto>,
    pub limit: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImConversationDto {
    pub id: i64,
    pub kind: String,
    pub title: Option<String>,
    pub last_seq: i64,
    pub last_read_seq: i64,
    pub read_only: bool,
    pub left_seq: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImSyncConversationDto {
    pub conversation: ImConversationDto,
    pub messages: Vec<ImMessageDto>,
    pub reload: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImSyncResponseDto {
    pub conversations: Vec<ImSyncConversationDto>,
}
