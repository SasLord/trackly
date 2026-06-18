//! Organisation settings / backup / templates axum HTTP handlers — Phase 7 Plan 07.
//!
//! Read handlers: session_identity only.
//! Mutation handlers: session_identity + authorize(ManageSettings).
//!
//! IMPORTANT: settings_move_db is NOT exposed here — it is Tauri-only (T-07-07-03).
//! app_restart is also Tauri-only (D-19).

use axum::extract::State;
use axum::{
    http::{header, StatusCode},
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use tower_sessions::Session;

use trackly_core::auth::{authorize, Action};

use crate::context::AppCtx;
use crate::dto::reports::{BackupConfigPatch, OrgPatch, OrgSettingsDto, TemplateEditorItem};
use crate::error_axum::AppErrorResponse;
use crate::http::auth::session_identity;
use crate::services::backup_service::{BackupConfigDto, BackupResult};
use crate::tauri_cmds::settings_org::{
    build_backup_run_manual, build_settings_get_backup_config, build_settings_get_db_path,
    build_settings_get_low_stock_threshold, build_settings_get_org, build_settings_get_org_logo,
    build_settings_remove_org_logo, build_settings_save_backup_config,
    build_settings_save_org_fields, build_settings_save_org_logo,
    build_settings_set_low_stock_threshold, build_templates_list_for_editor,
    build_templates_reset_to_default, build_templates_update_body,
    build_templates_validate_preview,
};

// ---------------------------------------------------------------------------
// Payload structs
// ---------------------------------------------------------------------------

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveOrgFieldsPayload {
    pub patch: OrgPatch,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveOrgLogoPayload {
    pub logo_bytes: Vec<u8>,
    pub logo_mime: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetLowStockPayload {
    pub threshold: i64,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveBackupConfigPayload {
    pub patch: BackupConfigPatch,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateUpdatePayload {
    pub kind: String,
    pub body: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateKindPayload {
    pub kind: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateValidatePayload {
    pub kind: String,
    pub body: String,
}

// ---------------------------------------------------------------------------
// Handlers — Organisation settings (read)
// ---------------------------------------------------------------------------

pub async fn handler_get_org(
    State(ctx): State<AppCtx>,
    session: Session,
) -> Result<Json<OrgSettingsDto>, AppErrorResponse> {
    let _identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_settings_get_org(&ctx)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_get_org_logo(
    State(ctx): State<AppCtx>,
    session: Session,
) -> Result<Json<Vec<u8>>, AppErrorResponse> {
    let _identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_settings_get_org_logo(&ctx)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_get_db_path(
    State(ctx): State<AppCtx>,
    session: Session,
) -> Result<Json<String>, AppErrorResponse> {
    let _identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_settings_get_db_path(&ctx)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_get_low_stock_threshold(
    State(ctx): State<AppCtx>,
    session: Session,
) -> Result<Json<i64>, AppErrorResponse> {
    let _identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_settings_get_low_stock_threshold(&ctx)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_get_backup_config(
    State(ctx): State<AppCtx>,
    session: Session,
) -> Result<Json<BackupConfigDto>, AppErrorResponse> {
    let _identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_settings_get_backup_config(&ctx)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_templates_list_for_editor(
    State(ctx): State<AppCtx>,
    session: Session,
) -> Result<Json<Vec<TemplateEditorItem>>, AppErrorResponse> {
    let _identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_templates_list_for_editor(&ctx)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

// ---------------------------------------------------------------------------
// Handlers — Organisation settings (mutations, ManageSettings required)
// ---------------------------------------------------------------------------

pub async fn handler_save_org_fields(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<SaveOrgFieldsPayload>,
) -> Result<Json<()>, AppErrorResponse> {
    let caller = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    authorize(&caller, &Action::ManageSettings).map_err(AppErrorResponse::from)?;
    build_settings_save_org_fields(&ctx, &caller, p.patch)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(()))
}

pub async fn handler_save_org_logo(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<SaveOrgLogoPayload>,
) -> Result<Json<()>, AppErrorResponse> {
    let caller = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    authorize(&caller, &Action::ManageSettings).map_err(AppErrorResponse::from)?;
    build_settings_save_org_logo(&ctx, &caller, p.logo_bytes, p.logo_mime)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(()))
}

pub async fn handler_remove_org_logo(
    State(ctx): State<AppCtx>,
    session: Session,
) -> Result<Json<()>, AppErrorResponse> {
    let caller = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    authorize(&caller, &Action::ManageSettings).map_err(AppErrorResponse::from)?;
    build_settings_remove_org_logo(&ctx, &caller)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(()))
}

pub async fn handler_set_low_stock_threshold(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<SetLowStockPayload>,
) -> Result<Json<()>, AppErrorResponse> {
    let caller = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    authorize(&caller, &Action::ManageSettings).map_err(AppErrorResponse::from)?;
    build_settings_set_low_stock_threshold(&ctx, &caller, p.threshold)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(()))
}

pub async fn handler_save_backup_config(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<SaveBackupConfigPayload>,
) -> Result<Json<()>, AppErrorResponse> {
    let caller = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    authorize(&caller, &Action::ManageSettings).map_err(AppErrorResponse::from)?;
    build_settings_save_backup_config(&ctx, &caller, p.patch)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(()))
}

pub async fn handler_backup_run_manual(
    State(ctx): State<AppCtx>,
    session: Session,
) -> Result<Json<BackupResult>, AppErrorResponse> {
    let caller = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    authorize(&caller, &Action::ManageSettings).map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_backup_run_manual(&ctx)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_templates_update_body(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<TemplateUpdatePayload>,
) -> Result<Json<()>, AppErrorResponse> {
    let caller = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    authorize(&caller, &Action::ManageSettings).map_err(AppErrorResponse::from)?;
    build_templates_update_body(&ctx, &caller, p.kind, p.body)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(()))
}

pub async fn handler_templates_reset_to_default(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<TemplateKindPayload>,
) -> Result<Json<()>, AppErrorResponse> {
    let caller = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    authorize(&caller, &Action::ManageSettings).map_err(AppErrorResponse::from)?;
    build_templates_reset_to_default(&ctx, &caller, p.kind)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(()))
}

pub async fn handler_templates_validate_preview(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<TemplateValidatePayload>,
) -> Result<impl IntoResponse, AppErrorResponse> {
    let caller = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    authorize(&caller, &Action::ManageSettings).map_err(AppErrorResponse::from)?;
    let bytes = build_templates_validate_preview(&ctx, &caller, p.kind, p.body)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/pdf")],
        bytes,
    ))
}

// ---------------------------------------------------------------------------
// Router — NOTE: settings_move_db is intentionally NOT included here (T-07-07-03)
// ---------------------------------------------------------------------------

pub fn router() -> Router<AppCtx> {
    Router::new()
        // Read endpoints
        .route("/api/v1/settings_get_org", post(handler_get_org))
        .route("/api/v1/settings_get_org_logo", post(handler_get_org_logo))
        .route("/api/v1/settings_get_db_path", post(handler_get_db_path))
        .route(
            "/api/v1/settings_get_low_stock_threshold",
            post(handler_get_low_stock_threshold),
        )
        .route(
            "/api/v1/settings_get_backup_config",
            post(handler_get_backup_config),
        )
        .route(
            "/api/v1/templates_list_for_editor",
            post(handler_templates_list_for_editor),
        )
        // Mutation endpoints (ManageSettings)
        .route(
            "/api/v1/settings_save_org_fields",
            post(handler_save_org_fields),
        )
        .route(
            "/api/v1/settings_save_org_logo",
            post(handler_save_org_logo),
        )
        .route(
            "/api/v1/settings_remove_org_logo",
            post(handler_remove_org_logo),
        )
        .route(
            "/api/v1/settings_set_low_stock_threshold",
            post(handler_set_low_stock_threshold),
        )
        .route(
            "/api/v1/settings_save_backup_config",
            post(handler_save_backup_config),
        )
        .route("/api/v1/backup_run_manual", post(handler_backup_run_manual))
        .route(
            "/api/v1/templates_update_body",
            post(handler_templates_update_body),
        )
        .route(
            "/api/v1/templates_reset_to_default",
            post(handler_templates_reset_to_default),
        )
        .route(
            "/api/v1/templates_validate_preview",
            post(handler_templates_validate_preview),
        )
}
