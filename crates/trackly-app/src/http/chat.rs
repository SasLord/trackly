//! Request chat axum HTTP routes - Phase 41.5 Plan 07 (MSG-03).
//!
//! JSON POST routes mirror the Tauri commands by name and delegate to the
//! `build_chat_*` helpers. No authorization or WS logic here: both live in
//! `ChatService`.

use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use tower_sessions::Session;

use crate::context::AppCtx;
use crate::dto::chat::{ChatFeedDto, ChatSendOutcomeDto, UnreadSummaryDto};
use crate::error_axum::AppErrorResponse;
use crate::http::auth::session_identity;
use crate::tauri_cmds::chat::{
    build_chat_discard_drafts, build_chat_feed, build_chat_mark_read, build_chat_send,
    build_chat_unread_summary,
};

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestIdPayload {
    pub request_id: i64,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendPayload {
    pub request_id: i64,
    pub text: String,
    #[serde(default)]
    pub draft_attachment_ids: Vec<i64>,
}

pub async fn handler_feed(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<RequestIdPayload>,
) -> Result<Json<ChatFeedDto>, AppErrorResponse> {
    let who = session_identity(&session).await?;
    Ok(Json(build_chat_feed(&ctx, &who, p.request_id).await?))
}

pub async fn handler_send(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<SendPayload>,
) -> Result<Json<ChatSendOutcomeDto>, AppErrorResponse> {
    let who = session_identity(&session).await?;
    Ok(Json(
        build_chat_send(&ctx, &who, p.request_id, p.text, p.draft_attachment_ids).await?,
    ))
}

pub async fn handler_mark_read(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<RequestIdPayload>,
) -> Result<Json<()>, AppErrorResponse> {
    let who = session_identity(&session).await?;
    Ok(Json(build_chat_mark_read(&ctx, &who, p.request_id).await?))
}

pub async fn handler_unread_summary(
    State(ctx): State<AppCtx>,
    session: Session,
) -> Result<Json<UnreadSummaryDto>, AppErrorResponse> {
    let who = session_identity(&session).await?;
    Ok(Json(build_chat_unread_summary(&ctx, &who).await?))
}

pub async fn handler_discard_drafts(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<RequestIdPayload>,
) -> Result<Json<()>, AppErrorResponse> {
    let who = session_identity(&session).await?;
    Ok(Json(
        build_chat_discard_drafts(&ctx, &who, p.request_id).await?,
    ))
}

pub fn router() -> Router<AppCtx> {
    Router::new()
        .route("/api/v1/chat_feed", post(handler_feed))
        .route("/api/v1/chat_send", post(handler_send))
        .route("/api/v1/chat_mark_read", post(handler_mark_read))
        .route("/api/v1/chat_unread_summary", post(handler_unread_summary))
        .route("/api/v1/chat_discard_drafts", post(handler_discard_drafts))
}
