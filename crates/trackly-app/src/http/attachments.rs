//! Attachments axum HTTP routes - Phase 41.3 Plan 11 (ATT-02, server mode).
//!
//! JSON POST routes mirror the Tauri commands by name; each handler delegates
//! to the `build_attachments_*` helper in `tauri_cmds/attachments.rs` (one DTO,
//! two transports). The two GET routes (`/attachments/{id}/file` and `/thumb`)
//! have no Tauri command: the desktop reads them through the `trackly-att` URI
//! scheme. Both transports use the same `AttachmentService::open_for_read`, so
//! there is no second authorization path (D-04).
//!
//! WS push is NOT done here: the service owns the `attachments-changed` event.

use axum::body::Body;
use axum::extract::{DefaultBodyLimit, Multipart, Path, Query, Request, State};
use axum::http::{header, HeaderMap, HeaderValue};
use axum::response::Response;
use axum::routing::{get, post};
use axum::{Json, Router};
use tower::ServiceExt;
use tower_http::services::ServeFile;
use tower_sessions::Session;

use crate::attachments_protocol::{content_disposition, safe_mime};
use crate::context::AppCtx;
use crate::dto::attachment::{
    AttachmentHistoryEventDto, AttachmentLimitDto, AttachmentListDto, AttachmentUploadDto,
    IntegrityReportDto,
};
use crate::error_axum::AppErrorResponse;
use crate::http::auth::session_identity;
use crate::services::attachment_service::{ReadKind, StageError};
use crate::tauri_cmds::attachments::{
    build_attachments_delete, build_attachments_get_limit, build_attachments_history,
    build_attachments_list, build_attachments_purge, build_attachments_report,
    build_attachments_restore, build_attachments_set_limit, build_attachments_upload_begin,
    build_attachments_upload_finish,
};
use trackly_core::auth::Identity;
use trackly_core::domain::attachments::{RejectCode, MAX_BYTES_CEILING};
use trackly_core::error::AppError;

/// Multipart framing overhead allowed on top of the file ceiling.
const MULTIPART_OVERHEAD: u64 = 1024 * 1024;

// ---------------------------------------------------------------------------
// Payload wrappers
// ---------------------------------------------------------------------------

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParentPayload {
    pub parent_kind: String,
    pub parent_id: i64,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdPayload {
    pub id: i64,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LimitPayload {
    pub bytes: i64,
}

/// Query keys are snake_case: `?parent_kind=..&parent_id=..`.
#[derive(serde::Deserialize)]
pub struct UploadQuery {
    pub parent_kind: String,
    pub parent_id: i64,
}

async fn identity(session: &Session) -> Result<Identity, AppErrorResponse> {
    session_identity(session)
        .await
        .map_err(AppErrorResponse::from)
}

// ---------------------------------------------------------------------------
// JSON handlers
// ---------------------------------------------------------------------------

pub async fn handler_list(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<ParentPayload>,
) -> Result<Json<AttachmentListDto>, AppErrorResponse> {
    let who = identity(&session).await?;
    Ok(Json(
        build_attachments_list(&ctx, &who, p.parent_kind, p.parent_id).await?,
    ))
}

pub async fn handler_delete(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<IdPayload>,
) -> Result<Json<()>, AppErrorResponse> {
    let who = identity(&session).await?;
    Ok(Json(build_attachments_delete(&ctx, &who, p.id).await?))
}

pub async fn handler_restore(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<IdPayload>,
) -> Result<Json<()>, AppErrorResponse> {
    let who = identity(&session).await?;
    Ok(Json(build_attachments_restore(&ctx, &who, p.id).await?))
}

pub async fn handler_purge(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<IdPayload>,
) -> Result<Json<()>, AppErrorResponse> {
    let who = identity(&session).await?;
    Ok(Json(build_attachments_purge(&ctx, &who, p.id).await?))
}

pub async fn handler_history(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<ParentPayload>,
) -> Result<Json<Vec<AttachmentHistoryEventDto>>, AppErrorResponse> {
    let who = identity(&session).await?;
    Ok(Json(
        build_attachments_history(&ctx, &who, p.parent_kind, p.parent_id).await?,
    ))
}

pub async fn handler_get_limit(
    State(ctx): State<AppCtx>,
    session: Session,
) -> Result<Json<AttachmentLimitDto>, AppErrorResponse> {
    let who = identity(&session).await?;
    Ok(Json(build_attachments_get_limit(&ctx, &who).await?))
}

pub async fn handler_set_limit(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<LimitPayload>,
) -> Result<Json<()>, AppErrorResponse> {
    let who = identity(&session).await?;
    Ok(Json(
        build_attachments_set_limit(&ctx, &who, p.bytes).await?,
    ))
}

pub async fn handler_report(
    State(ctx): State<AppCtx>,
    session: Session,
) -> Result<Json<Option<IntegrityReportDto>>, AppErrorResponse> {
    let who = identity(&session).await?;
    Ok(Json(build_attachments_report(&ctx, &who).await?))
}

// ---------------------------------------------------------------------------
// Multipart upload
// ---------------------------------------------------------------------------

fn reject(code: RejectCode) -> AttachmentUploadDto {
    AttachmentUploadDto {
        attachment: None,
        reject_code: Some(code.as_code().to_string()),
    }
}

fn multipart_validation(message: &str) -> AppError {
    AppError::Validation {
        field: "file".to_string(),
        message: message.to_string(),
    }
}

/// Streaming upload core of the HTTP transport (pair of
/// `build_attachments_upload_tauri`): authorize, cap by Content-Length, then
/// stream the single file field into staging chunk by chunk.
pub async fn build_attachments_upload_http(
    ctx: &AppCtx,
    caller: &Identity,
    kind: String,
    id: i64,
    content_length: Option<u64>,
    mut multipart: Multipart,
) -> Result<AttachmentUploadDto, AppError> {
    let (parent, limit) = build_attachments_upload_begin(ctx, caller, kind, id).await?;
    if content_length.is_some_and(|n| n > limit as u64 + MULTIPART_OVERHEAD) {
        return Ok(reject(RejectCode::TooLarge));
    }
    let mut staged = ctx.attachments.begin_staging()?;
    let name;
    {
        let Some(mut field) = multipart
            .next_field()
            .await
            .map_err(|_| multipart_validation("Некорректное тело multipart"))?
        else {
            return Err(multipart_validation("В запросе нет файла"));
        };
        name = field
            .file_name()
            .map(str::to_string)
            .ok_or_else(|| multipart_validation("У поля нет имени файла"))?;
        loop {
            match field.chunk().await {
                Ok(Some(chunk)) => match staged.write_chunk(&chunk, limit) {
                    Ok(()) => {}
                    Err(StageError::Reject(code)) => return Ok(reject(code)),
                    Err(StageError::Internal(e)) => return Err(e),
                },
                Ok(None) => break,
                Err(e) if e.status() == axum::http::StatusCode::PAYLOAD_TOO_LARGE => {
                    return Ok(reject(RejectCode::TooLarge));
                }
                Err(_) => return Err(multipart_validation("Обрыв загрузки файла")),
            }
        }
    }
    // Exactly one file per request.
    if multipart
        .next_field()
        .await
        .map_err(|_| multipart_validation("Некорректное тело multipart"))?
        .is_some()
    {
        return Err(multipart_validation("За один запрос принимается один файл"));
    }
    build_attachments_upload_finish(ctx, caller, parent, &name, staged).await
}

/// `Multipart` must stay the LAST extractor: it consumes the body, and the
/// authorization above it runs before the first body byte is read.
pub async fn handler_upload(
    State(ctx): State<AppCtx>,
    session: Session,
    Query(q): Query<UploadQuery>,
    headers: HeaderMap,
    multipart: Multipart,
) -> Result<Json<AttachmentUploadDto>, AppErrorResponse> {
    let who = identity(&session).await?;
    let content_length = headers
        .get(header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok());
    Ok(Json(
        build_attachments_upload_http(
            &ctx,
            &who,
            q.parent_kind,
            q.parent_id,
            content_length,
            multipart,
        )
        .await?,
    ))
}

// ---------------------------------------------------------------------------
// GET file / thumb
// ---------------------------------------------------------------------------

async fn serve(
    ctx: AppCtx,
    session: Session,
    id: i64,
    kind: ReadKind,
    req: Request,
) -> Result<Response, AppErrorResponse> {
    let who = identity(&session).await?;
    let opened = ctx.attachments.open_for_read(id, &who, kind).await?;
    let thumb = kind == ReadKind::Thumb;

    // ServeFile keeps Range / If-Range / conditional handling.
    let served = ServeFile::new(&opened.path).oneshot(req).await;
    let mut res = match served {
        Ok(r) => r.map(Body::new),
        Err(never) => match never {},
    };

    let disposition = if thumb {
        "inline".to_string()
    } else {
        content_disposition(&opened.original_name, opened.inline)
    };
    let cache = if thumb {
        "private, max-age=3600"
    } else {
        "private, no-store"
    };
    let h = res.headers_mut();
    if let Ok(v) = HeaderValue::from_str(safe_mime(&opened.mime)) {
        h.insert(header::CONTENT_TYPE, v);
    }
    h.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    if let Ok(v) = HeaderValue::from_str(&disposition) {
        h.insert(header::CONTENT_DISPOSITION, v);
    }
    h.insert(header::CACHE_CONTROL, HeaderValue::from_str(cache).unwrap());
    if !thumb {
        // Only the file route may be framed (inline PDF in the viewer iframe),
        // and only by the same origin. The global layer keeps DENY elsewhere.
        h.insert(
            header::HeaderName::from_static("x-frame-options"),
            HeaderValue::from_static("SAMEORIGIN"),
        );
    }
    Ok(res)
}

pub async fn handler_file(
    State(ctx): State<AppCtx>,
    session: Session,
    Path(id): Path<i64>,
    req: Request,
) -> Result<Response, AppErrorResponse> {
    serve(ctx, session, id, ReadKind::File, req).await
}

pub async fn handler_thumb(
    State(ctx): State<AppCtx>,
    session: Session,
    Path(id): Path<i64>,
    req: Request,
) -> Result<Response, AppErrorResponse> {
    serve(ctx, session, id, ReadKind::Thumb, req).await
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router() -> Router<AppCtx> {
    let upload = Router::new()
        .route("/api/v1/attachments_upload", post(handler_upload))
        .route_layer(DefaultBodyLimit::max(
            (MAX_BYTES_CEILING + MULTIPART_OVERHEAD) as usize,
        ));
    Router::new()
        .route("/api/v1/attachments_list", post(handler_list))
        .route("/api/v1/attachments_delete", post(handler_delete))
        .route("/api/v1/attachments_restore", post(handler_restore))
        .route("/api/v1/attachments_purge", post(handler_purge))
        .route("/api/v1/attachments_history", post(handler_history))
        .route("/api/v1/attachments_get_limit", post(handler_get_limit))
        .route("/api/v1/attachments_set_limit", post(handler_set_limit))
        .route("/api/v1/attachments_report", post(handler_report))
        .route("/api/v1/attachments/{id}/file", get(handler_file))
        .route("/api/v1/attachments/{id}/thumb", get(handler_thumb))
        .merge(upload)
}
