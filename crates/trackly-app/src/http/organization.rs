//! Organization axum HTTP routes — Phase 3 Plan 04.

use axum::{extract::State, routing::post, Json, Router};
use tower_sessions::Session;

use crate::context::AppCtx;
use crate::dto::OrgDto;
use crate::error_axum::AppErrorResponse;
use crate::http::auth::session_identity;
use crate::tauri_cmds::organization::build_organization_get;

pub async fn handler_get(
    State(ctx): State<AppCtx>,
    session: Session,
) -> Result<Json<OrgDto>, AppErrorResponse> {
    let caller = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_organization_get(&ctx, &caller)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub fn router() -> Router<AppCtx> {
    Router::new().route("/api/v1/organization_get", post(handler_get))
}
