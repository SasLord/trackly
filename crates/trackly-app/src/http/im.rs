//! Маршруты мессенджера — Phase 43 Plan 08 (D-02).
//!
//! Наружу выставлены только `im_send` и `im_sync`. Создания беседы и изменения
//! состава среди маршрутов нет. Хендлеры тонкие: авторизация (по членству в
//! беседе, а не по роли) и вся логика живут в `ImService`; здесь только
//! `session_identity` и делегирование. Tauri-близнецов нет — в десктопе чат скрыт.

use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use tower_sessions::Session;

use crate::context::AppCtx;
use crate::dto::im::{ImSendOutcomeDto, ImSendRequestDto, ImSyncRequestDto, ImSyncResponseDto};
use crate::error_axum::AppErrorResponse;
use crate::http::auth::session_identity;

pub async fn handler_send(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<ImSendRequestDto>,
) -> Result<Json<ImSendOutcomeDto>, AppErrorResponse> {
    let who = session_identity(&session).await?;
    Ok(Json(
        ctx.im
            .send(&who, p.conversation_id, &p.client_msg_id, &p.text)
            .await?,
    ))
}

pub async fn handler_sync(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<ImSyncRequestDto>,
) -> Result<Json<ImSyncResponseDto>, AppErrorResponse> {
    let who = session_identity(&session).await?;
    Ok(Json(ctx.im.sync(&who, &p).await?))
}

pub fn router() -> Router<AppCtx> {
    Router::new()
        .route("/api/v1/im_send", post(handler_send))
        .route("/api/v1/im_sync", post(handler_sync))
}
