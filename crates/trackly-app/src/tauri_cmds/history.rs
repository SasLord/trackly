//! Команда ленты истории — фаза 41.10 (HST-05, D-02).
//!
//! `build_*` + тонкая обёртка tauri-command; HTTP-маршрут (`http/history.rs`) делегирует
//! той же `build_history_get_timeline`. Склейка ленты — на сервере (`HistoryService`).
//! specta-атрибут ПОСЛЕ tauri-command (tauri-specta v2 rc.21).

use crate::context::AppCtx;
use crate::dto::history::HistoryEntryDto;
use crate::tauri_cmds::users::resolve_tauri_identity;
use trackly_core::auth::{authorize, Action, Identity};
use trackly_core::error::AppError;

/// Чтение: `ReadPlaces` (Admin|Manager); `entity_type` — `device|group|cartridge`.
pub async fn build_history_get_timeline(
    ctx: &AppCtx,
    caller: &Identity,
    entity_type: String,
    entity_id: i64,
) -> Result<Vec<HistoryEntryDto>, AppError> {
    authorize(caller, &Action::ReadPlaces)?;
    ctx.history
        .get_timeline(caller, &entity_type, entity_id)
        .await
}

#[tauri::command]
#[specta::specta]
pub async fn history_get_timeline(
    state: tauri::State<'_, AppCtx>,
    entity_type: String,
    entity_id: i32,
) -> Result<Vec<HistoryEntryDto>, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_history_get_timeline(state.inner(), &caller, entity_type, entity_id as i64).await
}
