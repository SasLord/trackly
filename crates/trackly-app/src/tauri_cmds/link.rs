//! Админ-API раздела «Link» (Phase 44-16, D-05/D-12/D-13): коды-приглашения,
//! активные входы, запрещённые компьютеры, отзыв и допуск, входы пользователя.
//!
//! `build_link_*` — общие хелперы двух транспортов: Tauri-команды ниже и
//! axum-обработчики из `http/link.rs` вызывают ровно их. Серверная проверка
//! роли (`Action::ManageUsers`, только Admin) стоит в КАЖДОМ хелпере — UI-скрытие
//! границей безопасности не является (T-44-60).

use crate::context::AppCtx;
use crate::dto::link::{
    BannedDeviceDto, CreateInviteRequest, InviteIssuedDto, InviteRowDto, LinkLoginRowDto,
    RevokeResultDto, UserLinkStatusDto,
};
use crate::server::tls;
use crate::services::link_service::ttl_hours_to_secs;
use crate::tauri_cmds::users::resolve_tauri_identity;
use trackly_core::auth::{authorize, Action, Identity};
use trackly_core::error::AppError;

/// Единая серверная граница роли для админ-API Link.
fn require_admin(caller: &Identity) -> Result<i64, AppError> {
    authorize(caller, &Action::ManageUsers)?;
    // Системный администратор десктопа не имеет user_id: колонка без FK.
    Ok(caller.user_id.unwrap_or(0))
}

/// Код выпускается с адресом и SPKI ЗАПУЩЕННОГО сервера (T-44-64): ни хост, ни
/// отпечаток из запроса не доверяются как источник отпечатка.
pub async fn build_link_invites_create(
    ctx: &AppCtx,
    caller: &Identity,
    req: CreateInviteRequest,
) -> Result<InviteIssuedDto, AppError> {
    let admin_id = require_admin(caller)?;
    let ttl = ttl_hours_to_secs(req.ttl_hours)?;
    let Some(info) = ctx.server_tls_info().await else {
        return Err(AppError::Conflict {
            reason: "Сервер не запущен: включите режим сервера, чтобы выпустить код".into(),
        });
    };
    let spki = link_proto::parse_colon_hex(&info.spki_fingerprint_hex).map_err(|_| {
        AppError::Internal {
            source_chain: "некорректный SPKI-отпечаток запущенного сервера".into(),
        }
    })?;
    let requested = req.host.trim();
    let host = if requested.is_empty() {
        tls::display_host(&info.host)
    } else {
        requested.to_string()
    };
    let issued = ctx
        .link
        .create_invite(&host, info.port, spki, ttl, admin_id)
        .await?;
    Ok(InviteIssuedDto {
        id: issued.id,
        code: issued.code,
        expires_at: issued.expires_at,
        host,
    })
}

pub async fn build_link_invites_list(
    ctx: &AppCtx,
    caller: &Identity,
) -> Result<Vec<InviteRowDto>, AppError> {
    require_admin(caller)?;
    ctx.link.list_invite_dtos().await
}

pub async fn build_link_invites_revoke(
    ctx: &AppCtx,
    caller: &Identity,
    id: i64,
) -> Result<(), AppError> {
    let admin_id = require_admin(caller)?;
    ctx.link.revoke_invite(id, admin_id).await
}

pub async fn build_link_logins_list(
    ctx: &AppCtx,
    caller: &Identity,
) -> Result<Vec<LinkLoginRowDto>, AppError> {
    require_admin(caller)?;
    ctx.link.list_login_dtos().await
}

pub async fn build_link_logins_revoke(
    ctx: &AppCtx,
    caller: &Identity,
    login_id: i64,
) -> Result<(), AppError> {
    let admin_id = require_admin(caller)?;
    ctx.link.revoke_login(login_id, admin_id).await
}

pub async fn build_link_devices_list(
    ctx: &AppCtx,
    caller: &Identity,
) -> Result<Vec<BannedDeviceDto>, AppError> {
    require_admin(caller)?;
    ctx.link.list_banned_dtos().await
}

pub async fn build_link_devices_allow(
    ctx: &AppCtx,
    caller: &Identity,
    device_id: String,
) -> Result<(), AppError> {
    require_admin(caller)?;
    ctx.link.allow_device(&device_id).await
}

pub async fn build_users_link_status(
    ctx: &AppCtx,
    caller: &Identity,
    user_id: i64,
) -> Result<UserLinkStatusDto, AppError> {
    require_admin(caller)?;
    ctx.link.user_link_status(user_id).await
}

/// «Отозвать входы Link»: все входы пользователя сразу, устройства под запрет.
pub async fn build_users_revoke_link(
    ctx: &AppCtx,
    caller: &Identity,
    user_id: i64,
) -> Result<RevokeResultDto, AppError> {
    let admin_id = require_admin(caller)?;
    let n = ctx.link.revoke_user_logins(user_id, admin_id, true).await?;
    Ok(RevokeResultDto {
        revoked: i32::try_from(n).unwrap_or(i32::MAX),
    })
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

#[tauri::command]
#[specta::specta]
pub async fn link_invites_create(
    state: tauri::State<'_, AppCtx>,
    req: CreateInviteRequest,
) -> Result<InviteIssuedDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_link_invites_create(state.inner(), &caller, req).await
}

#[tauri::command]
#[specta::specta]
pub async fn link_invites_list(
    state: tauri::State<'_, AppCtx>,
) -> Result<Vec<InviteRowDto>, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_link_invites_list(state.inner(), &caller).await
}

#[tauri::command]
#[specta::specta]
pub async fn link_invites_revoke(state: tauri::State<'_, AppCtx>, id: i32) -> Result<(), AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_link_invites_revoke(state.inner(), &caller, i64::from(id)).await
}

#[tauri::command]
#[specta::specta]
pub async fn link_logins_list(
    state: tauri::State<'_, AppCtx>,
) -> Result<Vec<LinkLoginRowDto>, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_link_logins_list(state.inner(), &caller).await
}

#[tauri::command]
#[specta::specta]
pub async fn link_logins_revoke(
    state: tauri::State<'_, AppCtx>,
    login_id: i32,
) -> Result<(), AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_link_logins_revoke(state.inner(), &caller, i64::from(login_id)).await
}

#[tauri::command]
#[specta::specta]
pub async fn link_devices_list(
    state: tauri::State<'_, AppCtx>,
) -> Result<Vec<BannedDeviceDto>, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_link_devices_list(state.inner(), &caller).await
}

#[tauri::command]
#[specta::specta]
pub async fn link_devices_allow(
    state: tauri::State<'_, AppCtx>,
    device_id: String,
) -> Result<(), AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_link_devices_allow(state.inner(), &caller, device_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn users_link_status(
    state: tauri::State<'_, AppCtx>,
    user_id: i32,
) -> Result<UserLinkStatusDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_users_link_status(state.inner(), &caller, i64::from(user_id)).await
}

#[tauri::command]
#[specta::specta]
pub async fn users_revoke_link(
    state: tauri::State<'_, AppCtx>,
    user_id: i32,
) -> Result<RevokeResultDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_users_revoke_link(state.inner(), &caller, i64::from(user_id)).await
}
