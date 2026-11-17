//! Request chat Tauri commands - Phase 41.5 Plan 07 (MSG-01, MSG-03, MSG-04).
//!
//! Pattern (S-1): `build_*` helper + thin `#[tauri::command] #[specta::specta]`
//! wrapper. No authorization logic here: access control lives only in
//! `ChatService`, so both transports share one check.

use crate::context::AppCtx;
use crate::dto::chat::{ChatFeedDto, ChatSendOutcomeDto, UnreadSummaryDto};
use crate::tauri_cmds::users::resolve_tauri_identity;
use trackly_core::auth::Identity;
use trackly_core::error::AppError;

pub async fn build_chat_feed(
    ctx: &AppCtx,
    caller: &Identity,
    request_id: i64,
) -> Result<ChatFeedDto, AppError> {
    ctx.chat.feed(caller, request_id).await
}

pub async fn build_chat_send(
    ctx: &AppCtx,
    caller: &Identity,
    request_id: i64,
    text: String,
    draft_attachment_ids: Vec<i64>,
) -> Result<ChatSendOutcomeDto, AppError> {
    ctx.chat
        .send(caller, request_id, &text, &draft_attachment_ids)
        .await
}

pub async fn build_chat_mark_read(
    ctx: &AppCtx,
    caller: &Identity,
    request_id: i64,
) -> Result<(), AppError> {
    ctx.chat.mark_read(caller, request_id).await
}

pub async fn build_chat_unread_summary(
    ctx: &AppCtx,
    caller: &Identity,
) -> Result<UnreadSummaryDto, AppError> {
    ctx.chat.unread_summary(caller).await
}

pub async fn build_chat_discard_drafts(
    ctx: &AppCtx,
    caller: &Identity,
    request_id: i64,
) -> Result<(), AppError> {
    ctx.chat.discard_drafts(caller, request_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn chat_feed(
    state: tauri::State<'_, AppCtx>,
    request_id: i32,
) -> Result<ChatFeedDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_chat_feed(state.inner(), &caller, request_id as i64).await
}

#[tauri::command]
#[specta::specta]
pub async fn chat_send(
    state: tauri::State<'_, AppCtx>,
    request_id: i32,
    text: String,
    draft_attachment_ids: Vec<i32>,
) -> Result<ChatSendOutcomeDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_chat_send(
        state.inner(),
        &caller,
        request_id as i64,
        text,
        draft_attachment_ids.into_iter().map(i64::from).collect(),
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn chat_mark_read(
    state: tauri::State<'_, AppCtx>,
    request_id: i32,
) -> Result<(), AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_chat_mark_read(state.inner(), &caller, request_id as i64).await
}

#[tauri::command]
#[specta::specta]
pub async fn chat_unread_summary(
    state: tauri::State<'_, AppCtx>,
) -> Result<UnreadSummaryDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_chat_unread_summary(state.inner(), &caller).await
}

#[tauri::command]
#[specta::specta]
pub async fn chat_discard_drafts(
    state: tauri::State<'_, AppCtx>,
    request_id: i32,
) -> Result<(), AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_chat_discard_drafts(state.inner(), &caller, request_id as i64).await
}
