//! Attachments Tauri commands — Phase 41.3 Plan 10 (ATT-02).
//!
//! Pattern (S-1): `build_*` helper + thin `#[tauri::command] #[specta::specta]`
//! wrapper. Every command resolves the identity with `resolve_tauri_identity`
//! and calls the same `AttachmentService` method as the HTTP route (D-04):
//! there is no second authorization path.
//!
//! Upload uses a raw invoke body with percent-encoded headers
//! `x-att-parent-kind`, `x-att-parent-id`, `x-att-name`.

use crate::context::AppCtx;
use crate::dto::attachment::{
    AttachmentHistoryEventDto, AttachmentLimitDto, AttachmentListDto, AttachmentUploadDto,
    IntegrityReportDto,
};
use crate::services::attachment_service::{ParentRef, StageError};
use crate::tauri_cmds::users::resolve_tauri_identity;
use trackly_core::auth::Identity;
use trackly_core::domain::attachments::{ParentKind, RejectCode};
use trackly_core::error::AppError;

fn parent_ref(kind: &str, id: i64) -> Result<ParentRef, AppError> {
    let kind = ParentKind::parse(kind).ok_or_else(|| AppError::Validation {
        field: "parent_kind".to_string(),
        message: "Неизвестный тип родителя вложения".to_string(),
    })?;
    Ok(ParentRef { kind, id })
}

// ---------------------------------------------------------------------------
// build_* helpers (shared with axum handlers)
// ---------------------------------------------------------------------------

pub async fn build_attachments_list(
    ctx: &AppCtx,
    caller: &Identity,
    parent_kind: String,
    parent_id: i64,
) -> Result<AttachmentListDto, AppError> {
    ctx.attachments
        .list(parent_ref(&parent_kind, parent_id)?, caller)
        .await
}

pub async fn build_attachments_delete(
    ctx: &AppCtx,
    caller: &Identity,
    id: i64,
) -> Result<(), AppError> {
    ctx.attachments.delete(caller, id).await
}

pub async fn build_attachments_restore(
    ctx: &AppCtx,
    caller: &Identity,
    id: i64,
) -> Result<(), AppError> {
    ctx.attachments.restore(caller, id).await
}

pub async fn build_attachments_purge(
    ctx: &AppCtx,
    caller: &Identity,
    id: i64,
) -> Result<(), AppError> {
    ctx.attachments.purge(caller, id).await
}

pub async fn build_attachments_history(
    ctx: &AppCtx,
    caller: &Identity,
    parent_kind: String,
    parent_id: i64,
) -> Result<Vec<AttachmentHistoryEventDto>, AppError> {
    ctx.attachments
        .history(caller, parent_ref(&parent_kind, parent_id)?)
        .await
}

pub async fn build_attachments_get_limit(
    ctx: &AppCtx,
    caller: &Identity,
) -> Result<AttachmentLimitDto, AppError> {
    ctx.attachments.get_limit(caller).await
}

pub async fn build_attachments_set_limit(
    ctx: &AppCtx,
    caller: &Identity,
    bytes: i64,
) -> Result<(), AppError> {
    ctx.attachments.set_max_bytes(caller, bytes).await
}

pub async fn build_attachments_report(
    ctx: &AppCtx,
    caller: &Identity,
) -> Result<Option<IntegrityReportDto>, AppError> {
    ctx.attachments.get_report(caller).await
}

/// Upload of one file. Order matters: authorization first, then the size
/// check against the limit, and only then anything is written to disk.
pub async fn build_attachments_upload(
    ctx: &AppCtx,
    caller: &Identity,
    parent_kind: String,
    parent_id: i64,
    name: String,
    body: &[u8],
) -> Result<AttachmentUploadDto, AppError> {
    let parent = parent_ref(&parent_kind, parent_id)?;
    ctx.attachments.authorize_attach(caller, parent).await?;
    let limit = ctx.attachments.effective_limit().await?;
    if body.len() as i64 > limit {
        return Ok(AttachmentUploadDto {
            attachment: None,
            reject_code: Some(RejectCode::TooLarge.as_code().to_string()),
        });
    }
    let mut staged = ctx.attachments.begin_staging()?;
    match staged.write_chunk(body, limit) {
        Ok(()) => {}
        Err(StageError::Reject(code)) => {
            return Ok(AttachmentUploadDto {
                attachment: None,
                reject_code: Some(code.as_code().to_string()),
            });
        }
        Err(StageError::Internal(e)) => return Err(e),
    }
    ctx.attachments.ingest(caller, parent, &name, staged).await
}

fn header_decoded(req: &tauri::ipc::Request<'_>, name: &str) -> Result<String, AppError> {
    let raw = req
        .headers()
        .get(name)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::Validation {
            field: name.to_string(),
            message: format!("Нет заголовка {name}"),
        })?;
    percent_encoding::percent_decode_str(raw)
        .decode_utf8()
        .map(|s| s.into_owned())
        .map_err(|_| AppError::Validation {
            field: name.to_string(),
            message: format!("Заголовок {name} не UTF-8"),
        })
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

#[tauri::command]
#[specta::specta]
pub async fn attachments_list(
    state: tauri::State<'_, AppCtx>,
    parent_kind: String,
    parent_id: i32,
) -> Result<AttachmentListDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_attachments_list(state.inner(), &caller, parent_kind, parent_id as i64).await
}

#[tauri::command]
#[specta::specta]
pub async fn attachments_delete(state: tauri::State<'_, AppCtx>, id: i32) -> Result<(), AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_attachments_delete(state.inner(), &caller, id as i64).await
}

#[tauri::command]
#[specta::specta]
pub async fn attachments_restore(state: tauri::State<'_, AppCtx>, id: i32) -> Result<(), AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_attachments_restore(state.inner(), &caller, id as i64).await
}

#[tauri::command]
#[specta::specta]
pub async fn attachments_purge(state: tauri::State<'_, AppCtx>, id: i32) -> Result<(), AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_attachments_purge(state.inner(), &caller, id as i64).await
}

#[tauri::command]
#[specta::specta]
pub async fn attachments_history(
    state: tauri::State<'_, AppCtx>,
    parent_kind: String,
    parent_id: i32,
) -> Result<Vec<AttachmentHistoryEventDto>, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_attachments_history(state.inner(), &caller, parent_kind, parent_id as i64).await
}

#[tauri::command]
#[specta::specta]
pub async fn attachments_get_limit(
    state: tauri::State<'_, AppCtx>,
) -> Result<AttachmentLimitDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_attachments_get_limit(state.inner(), &caller).await
}

#[tauri::command]
#[specta::specta]
pub async fn attachments_set_limit(
    state: tauri::State<'_, AppCtx>,
    bytes: i32,
) -> Result<(), AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_attachments_set_limit(state.inner(), &caller, bytes as i64).await
}

#[tauri::command]
#[specta::specta]
pub async fn attachments_report(
    state: tauri::State<'_, AppCtx>,
) -> Result<Option<IntegrityReportDto>, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_attachments_report(state.inner(), &caller).await
}

/// Raw-body upload: headers `x-att-parent-kind`, `x-att-parent-id`,
/// `x-att-name` (percent-encoded), body = file bytes (`InvokeBody::Raw`).
///
/// Not registered with specta (`tauri::ipc::Request` has no `Type`): it is
/// dispatched by `main.rs` next to the specta handler and called from the UI
/// via a hand-written `invoke` wrapper.
#[tauri::command]
pub async fn attachments_upload(
    state: tauri::State<'_, AppCtx>,
    request: tauri::ipc::Request<'_>,
) -> Result<AttachmentUploadDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    let kind = header_decoded(&request, "x-att-parent-kind")?;
    let id: i64 = header_decoded(&request, "x-att-parent-id")?
        .parse()
        .map_err(|_| AppError::Validation {
            field: "x-att-parent-id".to_string(),
            message: "Некорректный идентификатор родителя".to_string(),
        })?;
    let name = header_decoded(&request, "x-att-name")?;
    let tauri::ipc::InvokeBody::Raw(bytes) = request.body() else {
        return Err(AppError::Validation {
            field: "body".to_string(),
            message: "Ожидалось бинарное тело запроса".to_string(),
        });
    };
    build_attachments_upload(state.inner(), &caller, kind, id, name, bytes).await
}
