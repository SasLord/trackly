//! axum HTTP routes for FS helper commands — Plan 05 B2 pinned strategy.
//!
//! Phase 2 registers routes; Phase 5+ activates full browser-mode binding.
//! Delegates to `build_*` helpers from `tauri_cmds::fs_helpers`.

use axum::{extract::State, routing::post, Json, Router};

use crate::context::AppCtx;
use crate::error_axum::AppErrorResponse;
use crate::tauri_cmds::fs_helpers::{build_read_file_bytes, build_write_file_bytes};

// ---------------------------------------------------------------------------
// Payload structs
// ---------------------------------------------------------------------------

#[derive(serde::Deserialize)]
pub struct ReadFileBytesPayload {
    pub path: String,
}

#[derive(serde::Deserialize)]
pub struct WriteFileBytesPayload {
    pub path: String,
    pub content: String,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

pub async fn handler_read_file_bytes(
    State(ctx): State<AppCtx>,
    Json(payload): Json<ReadFileBytesPayload>,
) -> Result<Json<Vec<u8>>, AppErrorResponse> {
    Ok(Json(
        build_read_file_bytes(&ctx, payload.path)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_write_file_bytes(
    State(ctx): State<AppCtx>,
    Json(payload): Json<WriteFileBytesPayload>,
) -> Result<Json<()>, AppErrorResponse> {
    build_write_file_bytes(&ctx, payload.path, payload.content)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(()))
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router() -> Router<AppCtx> {
    Router::new()
        .route("/api/v1/read_file_bytes", post(handler_read_file_bytes))
        .route("/api/v1/write_file_bytes", post(handler_write_file_bytes))
}
