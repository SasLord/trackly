//! Place-movement timeline axum HTTP route — Plan 40-10 (HST-02).
//!
//! Одна route — POST `/api/v1/place_movements_get_timeline` (аналогично Tauri
//! command name). Handler — thin adapter, делегирует `build_place_movements_get_timeline`
//! из `tauri_cmds::place_movements` (mirrors `http/places.rs`'s exact structure —
//! PATTERNS.md §Pattern 1: "один DTO, два транспорта").
//!
//! D-12: gate (`Action::ReadPlaces`, Admin|Manager) is reached INSIDE
//! `build_place_movements_get_timeline` — this file never re-implements the gate, it
//! only resolves the session identity and delegates (T-40-22 mitigation: both
//! transports route through the same gated function).

use axum::{extract::State, routing::post, Json, Router};
use tower_sessions::Session;

use crate::context::AppCtx;
use crate::dto::place_movements::MovementEntryDto;
use crate::error_axum::AppErrorResponse;
use crate::http::auth::session_identity;
use crate::tauri_cmds::place_movements::build_place_movements_get_timeline;

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
) -> Result<Json<Vec<MovementEntryDto>>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_place_movements_get_timeline(&ctx, &identity, payload.entity_type, payload.entity_id)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router() -> Router<AppCtx> {
    Router::new().route(
        "/api/v1/place_movements_get_timeline",
        post(handler_get_timeline),
    )
}
