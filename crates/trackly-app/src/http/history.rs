//! Маршрут ленты истории — фаза 41.10 (HST-05).
//!
//! POST `/api/v1/history_get_timeline`; тонкий адаптер над
//! `tauri_cmds::history::build_history_get_timeline` (ворота `ReadPlaces` внутри).

use axum::{extract::State, routing::post, Json, Router};
use tower_sessions::Session;

use crate::context::AppCtx;
use crate::dto::history::HistoryEntryDto;
use crate::error_axum::AppErrorResponse;
use crate::http::auth::session_identity;
use crate::tauri_cmds::history::build_history_get_timeline;

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetTimelinePayload {
    pub entity_type: String,
    pub entity_id: i64,
}

pub async fn handler_get_timeline(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(payload): Json<GetTimelinePayload>,
) -> Result<Json<Vec<HistoryEntryDto>>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_history_get_timeline(&ctx, &identity, payload.entity_type, payload.entity_id)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub fn router() -> Router<AppCtx> {
    Router::new().route("/api/v1/history_get_timeline", post(handler_get_timeline))
}
