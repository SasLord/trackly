//! Templates axum HTTP routes — Phase 3 Plan 04.

use axum::http::{header, StatusCode};
use axum::response::IntoResponse;
use axum::{extract::State, routing::post, Json, Router};
use tower_sessions::Session;

use crate::context::AppCtx;
use crate::error_axum::AppErrorResponse;
use crate::http::auth::session_identity;
use crate::tauri_cmds::templates::{build_templates_get_active, build_templates_render_preview};

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetActivePayload {
    pub kind: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderPreviewPayload {
    pub kind: String,
    pub sample_act_id: i64,
}

pub async fn handler_get_active(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<GetActivePayload>,
) -> Result<Json<String>, AppErrorResponse> {
    let caller = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_templates_get_active(&ctx, &caller, p.kind)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_render_preview(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<RenderPreviewPayload>,
) -> Result<impl IntoResponse, AppErrorResponse> {
    let caller = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    let bytes = build_templates_render_preview(&ctx, &caller, p.kind, p.sample_act_id)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/pdf")],
        bytes,
    ))
}

pub fn router() -> Router<AppCtx> {
    Router::new()
        .route("/api/v1/templates_get_active", post(handler_get_active))
        .route(
            "/api/v1/templates_render_preview",
            post(handler_render_preview),
        )
}
