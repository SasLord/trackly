//! Acts Tauri commands — Plan 02 vertical slice.
//!
//! Pattern (S-1): `build_*` helper + thin `#[tauri::command] #[specta::specta]`
//! wrapper. Both transports (Tauri invoke + axum POST) delegate to the same
//! helper.
//!
//! The `#[specta::specta]` attribute MUST appear AFTER `#[tauri::command]`
//! — required by tauri-specta v2 rc.21.
//!
//! Phase 5 Plan 04: мутации (create, return, delete, render_pdf, render_acceptance_pdf)
//! требуют `caller: &Identity` с правом `MutateActs`. Tauri wrappers resolve identity
//! через `resolve_tauri_identity` (D-Desktop-01/02).

use crate::context::AppCtx;
use crate::dto::act::{
    ActCreateDto, ActDto, ActFilter, ActListResponse, ActReturnDto, ActsCountsDto, Pagination,
};
use crate::dto::suggest::SuggestPersonField;
use crate::tauri_cmds::users::resolve_tauri_identity;
use tauri_plugin_shell::ShellExt;
use trackly_core::auth::{authorize, Action, Identity};
use trackly_core::error::AppError;

// ---------------------------------------------------------------------------
// build_* helpers (shared with axum handlers)
// ---------------------------------------------------------------------------

pub async fn build_acts_list(
    ctx: &AppCtx,
    filter: ActFilter,
    pagination: Pagination,
) -> Result<ActListResponse, AppError> {
    ctx.acts.list(filter, pagination).await
}

pub async fn build_acts_search(
    ctx: &AppCtx,
    query: String,
    filter: ActFilter,
    pagination: Pagination,
) -> Result<ActListResponse, AppError> {
    ctx.acts.search(query, filter, pagination).await
}

pub async fn build_acts_get(ctx: &AppCtx, id: i64) -> Result<ActDto, AppError> {
    ctx.acts.get(id).await
}

/// Мутация: требует `caller` с правом `MutateActs` (Admin | Manager).
pub async fn build_acts_create(
    ctx: &AppCtx,
    caller: &Identity,
    payload: ActCreateDto,
) -> Result<ActDto, AppError> {
    authorize(caller, &Action::MutateActs)?;
    ctx.acts.create(payload).await
}

/// Мутация: требует `caller` с правом `MutateActs`.
pub async fn build_acts_return(
    ctx: &AppCtx,
    caller: &Identity,
    act_id: i64,
    payload: ActReturnDto,
) -> Result<ActDto, AppError> {
    authorize(caller, &Action::MutateActs)?;
    ctx.acts.do_return(act_id, payload).await
}

/// Мутация: требует `caller` с правом `MutateActs`.
pub async fn build_acts_delete(
    ctx: &AppCtx,
    caller: &Identity,
    id: i64,
    version: i64,
) -> Result<(), AppError> {
    authorize(caller, &Action::MutateActs)?;
    ctx.acts.delete_soft(id, version).await
}

pub async fn build_acts_counts(ctx: &AppCtx) -> Result<ActsCountsDto, AppError> {
    ctx.acts.counts().await
}

pub async fn build_acts_peek_next_number(ctx: &AppCtx) -> Result<i64, AppError> {
    ctx.acts.peek_next_number().await
}

/// Мутация (PDF generation tied to act): требует `caller` с правом `MutateActs`.
pub async fn build_acts_render_pdf(
    ctx: &AppCtx,
    caller: &Identity,
    act_id: i64,
) -> Result<Vec<u8>, AppError> {
    authorize(caller, &Action::MutateActs)?;
    ctx.acts.render_pdf(act_id).await
}

pub async fn build_acts_suggest_person(
    ctx: &AppCtx,
    field: SuggestPersonField,
    prefix: String,
) -> Result<Vec<String>, AppError> {
    ctx.acts.suggest_person(field, &prefix, 20).await
}

/// Мутация (acceptance PDF): требует `caller` с правом `MutateActs`.
pub async fn build_devices_render_acceptance_pdf(
    ctx: &AppCtx,
    caller: &Identity,
    device_id: i64,
    giver_name: String,
    receiver_name: String,
    date_utc: i64,
) -> Result<Vec<u8>, AppError> {
    authorize(caller, &Action::MutateActs)?;
    ctx.acts
        .render_acceptance_pdf(device_id, giver_name, receiver_name, date_utc)
        .await
}

// ---------------------------------------------------------------------------
// Thin Tauri wrappers
// ---------------------------------------------------------------------------

#[tauri::command]
#[specta::specta]
pub async fn acts_list(
    state: tauri::State<'_, AppCtx>,
    filter: ActFilter,
    pagination: Pagination,
) -> Result<ActListResponse, AppError> {
    build_acts_list(state.inner(), filter, pagination).await
}

#[tauri::command]
#[specta::specta]
pub async fn acts_search(
    state: tauri::State<'_, AppCtx>,
    query: String,
    filter: ActFilter,
    pagination: Pagination,
) -> Result<ActListResponse, AppError> {
    build_acts_search(state.inner(), query, filter, pagination).await
}

#[tauri::command]
#[specta::specta]
pub async fn acts_get(state: tauri::State<'_, AppCtx>, id: i32) -> Result<ActDto, AppError> {
    build_acts_get(state.inner(), id as i64).await
}

#[tauri::command]
#[specta::specta]
pub async fn acts_create(
    state: tauri::State<'_, AppCtx>,
    payload: ActCreateDto,
) -> Result<ActDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_acts_create(state.inner(), &caller, payload).await
}

#[tauri::command]
#[specta::specta]
pub async fn acts_return(
    state: tauri::State<'_, AppCtx>,
    act_id: i32,
    payload: ActReturnDto,
) -> Result<ActDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_acts_return(state.inner(), &caller, act_id as i64, payload).await
}

#[tauri::command]
#[specta::specta]
pub async fn acts_delete(
    state: tauri::State<'_, AppCtx>,
    id: i32,
    version: i32,
) -> Result<(), AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_acts_delete(state.inner(), &caller, id as i64, version as i64).await
}

#[tauri::command]
#[specta::specta]
pub async fn acts_counts(state: tauri::State<'_, AppCtx>) -> Result<ActsCountsDto, AppError> {
    build_acts_counts(state.inner()).await
}

#[tauri::command]
#[specta::specta]
pub async fn acts_peek_next_number(state: tauri::State<'_, AppCtx>) -> Result<i32, AppError> {
    let next = build_acts_peek_next_number(state.inner()).await?;
    Ok(next as i32)
}

#[tauri::command]
#[specta::specta]
pub async fn acts_render_pdf(
    state: tauri::State<'_, AppCtx>,
    act_id: i32,
) -> Result<Vec<u8>, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_acts_render_pdf(state.inner(), &caller, act_id as i64).await
}

#[tauri::command]
#[specta::specta]
pub async fn acts_suggest_person(
    state: tauri::State<'_, AppCtx>,
    field: SuggestPersonField,
    prefix: String,
) -> Result<Vec<String>, AppError> {
    build_acts_suggest_person(state.inner(), field, prefix).await
}

/// CR-02 (Phase 3.1 code review fix): secure wrapper для shell::open.
/// Frontend больше не имеет capability `shell:allow-open` — все open-операции
/// проходят через эту команду, которая валидирует path:
///   1. Canonicalize → защита от `../` traversal.
///   2. Path must start with `std::env::temp_dir()`.
///   3. Path must end with `.pdf` (lowercase).
/// Только при passing all guards path передаётся в tauri_plugin_shell::open.
#[tauri::command]
#[specta::specta]
pub async fn acts_open_pdf_in_system(app: tauri::AppHandle, path: String) -> Result<(), AppError> {
    let candidate = std::path::PathBuf::from(&path);
    let canonical = candidate.canonicalize().map_err(|e| AppError::Validation {
        field: "path".into(),
        message: format!("invalid path: {e}"),
    })?;
    let temp_dir = std::env::temp_dir()
        .canonicalize()
        .map_err(|e| AppError::Internal {
            source_chain: format!("temp_dir canonicalize: {e}"),
        })?;
    if !canonical.starts_with(&temp_dir) {
        return Err(AppError::Validation {
            field: "path".into(),
            message: "path is outside temp directory".into(),
        });
    }
    let ext_ok = canonical
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("pdf"))
        .unwrap_or(false);
    if !ext_ok {
        return Err(AppError::Validation {
            field: "path".into(),
            message: "only .pdf files allowed".into(),
        });
    }
    let canonical_str = canonical.to_string_lossy().to_string();
    // TODO(Phase 4): migrate to tauri-plugin-opener (shell::open deprecated в v2.3+).
    #[allow(deprecated)]
    app.shell()
        .open(canonical_str, None)
        .map_err(|e| AppError::Internal {
            source_chain: format!("shell::open failed: {e}"),
        })?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn devices_render_acceptance_pdf(
    state: tauri::State<'_, AppCtx>,
    device_id: i32,
    giver_name: String,
    receiver_name: String,
    date_utc: i32,
) -> Result<Vec<u8>, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_devices_render_acceptance_pdf(
        state.inner(),
        &caller,
        device_id as i64,
        giver_name,
        receiver_name,
        date_utc as i64,
    )
    .await
}
