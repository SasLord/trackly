//! Auth Tauri commands — Plan 03.
//!
//! Десктоп-транспорт для auth операций.
//! Tauri commands НЕ используют Session extractor — сессии только для HTTP.
//!
//! Паттерн: `build_*` helper + тонкий `#[tauri::command] #[specta::specta]` wrapper.
//! `#[specta::specta]` ПОСЛЕ `#[tauri::command]` — требование tauri-specta v2 rc.21.

use crate::context::AppCtx;
use crate::dto::auth::{
    AdSettingsDto, AuthStatusDto, LoginRequest, NetworkSettingsDto, ServerStatusDto, UserDto,
};
use crate::http::auth::{RequestAdRestoreRequest, SetAdPayload};
use crate::http::settings::NetworkPatch;
use crate::server::rusqlite_session_store::RusqliteSessionStore;
use crate::server::tls;
use crate::server::{start_server, ServerHandle};
use trackly_core::auth::Action;
use trackly_core::error::AppError;
use trackly_infra::error_conversions::map_rusqlite;

// ---------------------------------------------------------------------------
// build_* helpers
// ---------------------------------------------------------------------------

/// Таури-логин: аутентификация без Session.
pub async fn build_auth_login_tauri(ctx: &AppCtx, req: LoginRequest) -> Result<UserDto, AppError> {
    ctx.auth.login(req).await
}

/// Таури auth_status: текущее состояние загрузки.
///
/// Десктоп не имеет cookie-сессии — возвращаем только bootstrap-флаги.
/// user всегда None в десктоп-режиме (UI управляет сессией отдельно в Plan 05 UI).
pub async fn build_auth_status_tauri(ctx: &AppCtx) -> Result<AuthStatusDto, AppError> {
    let needs_bootstrap = ctx.auth.needs_bootstrap().await?;
    let desktop_lock_enabled = ctx.auth.get_desktop_lock_enabled().await?;
    Ok(AuthStatusDto {
        needs_bootstrap,
        desktop_lock_enabled,
        user: None,
    })
}

/// Таури-вариант явного запроса восстановления доступа (09-AD-GAPS
/// restoration-flow UX). Зеркалит `build_auth_login_tauri` — несёт
/// собственные credentials, без Session (у блокированного пользователя нет
/// сессии в десктоп-режиме).
pub async fn build_request_ad_restore_tauri(
    ctx: &AppCtx,
    req: RequestAdRestoreRequest,
) -> Result<(), AppError> {
    ctx.auth.request_ad_restore(&req.login, &req.password).await
}

/// Таури server_toggle: старт / стоп axum сервера.
pub async fn build_server_toggle_tauri(
    ctx: &AppCtx,
    enable: bool,
) -> Result<ServerStatusDto, AppError> {
    if !enable {
        let mut guard = ctx.server_ctl.lock().await;
        if let Some(handle) = guard.take() {
            handle.cancel.cancel();
            drop(handle.task);
        }
        return Ok(ServerStatusDto {
            running: false,
            url: None,
            fingerprint: None,
        });
    }

    // Остановить предыдущий инстанс.
    {
        let mut guard = ctx.server_ctl.lock().await;
        if let Some(handle) = guard.take() {
            handle.cancel.cancel();
            drop(handle.task);
        }
    }

    // Эффективные настройки: live app_settings поверх TOML-bootstrap. Это
    // путь, который реально использует десктоп-UI (server_toggle Tauri-команда)
    // — root-cause фикс server-bind-localhost-only: выбранный в Настройках
    // host=0.0.0.0 теперь доходит до TcpListener::bind, а не подменяется
    // дефолтным config.host=127.0.0.1.
    let net = crate::http::settings::resolve_effective_network(ctx).await?;

    // TLS bundle.
    let tls_bundle = if net.cert_path.is_empty() {
        let host = net.host.clone();
        tls::generate_self_signed(&host).map_err(|e| AppError::Internal {
            source_chain: format!("generate_self_signed: {e}"),
        })?
    } else {
        // WR-01: explicit/validated key-path resolution (no brittle .replace heuristic).
        tls::load_from_files(&net.cert_path, &net.key_path).map_err(|e| AppError::Internal {
            source_chain: format!("load_from_files: {e}"),
        })?
    };

    let fingerprint = tls_bundle.fingerprint_hex.clone();
    let host = net.host.clone();
    let port = net.port;
    // Адрес для пользователя: для wildcard-bind показываем реальный LAN-IP,
    // а не https://0.0.0.0:port (bind остаётся на `host`).
    let url = format!("https://{}:{}", tls::display_host(&host), port);

    let addr: std::net::SocketAddr =
        format!("{}:{}", host, port)
            .parse()
            .map_err(|e| AppError::Validation {
                field: "server.host".to_string(),
                message: format!("invalid bind address: {e}"),
            })?;

    let session_store = RusqliteSessionStore::new(ctx.writer.clone(), ctx.readers.clone());
    let router = crate::http::build_router(ctx, session_store);

    let child_token = ctx.shutdown.child_token();
    let cancel = child_token.clone();

    let task = tokio::spawn(async move {
        let listener = match tokio::net::TcpListener::bind(addr).await {
            Ok(l) => l,
            Err(e) => {
                tracing::error!("server bind failed on {addr}: {e}");
                return;
            }
        };
        if let Err(e) = start_server(router, listener, tls_bundle.acceptor, child_token).await {
            tracing::error!("server error: {e}");
        }
    });

    {
        let mut guard = ctx.server_ctl.lock().await;
        *guard = Some(ServerHandle { cancel, task });
    }

    Ok(ServerStatusDto {
        running: true,
        url: Some(url),
        fingerprint: Some(fingerprint),
    })
}

/// Таури server_status.
pub async fn build_server_status_tauri(ctx: &AppCtx) -> Result<ServerStatusDto, AppError> {
    let guard = ctx.server_ctl.lock().await;
    let running = guard.is_some();
    Ok(ServerStatusDto {
        running,
        url: None,
        fingerprint: None,
    })
}

/// Таури desktop_set_lock: установка флага desktop_lock_enabled.
///
/// **Безопасность (CR-01):** НЕ использовать hardcoded `trusted_admin()` —
/// это позволило бы любому неаутентифицированному локальному пользователю
/// отключить блокировку рабочего стола (полный обход аутентификации, D-Desktop-02).
///
/// Разрешаем caller через `resolve_tauri_identity`:
/// - lock=OFF → trusted_admin (нормально, режим без блокировки).
/// - lock=ON  → desktop_identity (ровно один admin → Some(id); 0/2+ → trusted_admin).
///
/// Когда блокировка ВКЛЮЧЕНА, отключить её может только подлинный
/// аутентифицированный admin (`user_id = Some(..)`). Синтетический
/// `trusted_admin` (`user_id = None`), который возвращается при 0/2+ admin'ах,
/// отклоняется — иначе вебвью могло бы вызвать `desktop_set_lock` до входа.
pub async fn build_desktop_set_lock_tauri(ctx: &AppCtx, enabled: bool) -> Result<(), AppError> {
    let caller = crate::tauri_cmds::users::resolve_tauri_identity(ctx).await?;
    if ctx.auth.get_desktop_lock_enabled().await? && caller.user_id.is_none() {
        return Err(AppError::Unauthorized);
    }
    ctx.auth.set_desktop_lock_enabled(enabled, &caller).await
}

// ---------------------------------------------------------------------------
// Tauri commands — тонкие wrappers
// ---------------------------------------------------------------------------

#[tauri::command]
#[specta::specta]
pub async fn auth_login(
    state: tauri::State<'_, AppCtx>,
    req: LoginRequest,
) -> Result<UserDto, AppError> {
    build_auth_login_tauri(state.inner(), req).await
}

#[tauri::command]
#[specta::specta]
pub async fn request_ad_restore(
    state: tauri::State<'_, AppCtx>,
    req: RequestAdRestoreRequest,
) -> Result<(), AppError> {
    build_request_ad_restore_tauri(state.inner(), req).await
}

/// auth_logout: для Tauri — no-op (нет cookie-сессии).
#[tauri::command]
#[specta::specta]
pub async fn auth_logout(_state: tauri::State<'_, AppCtx>) -> Result<(), AppError> {
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn auth_status(state: tauri::State<'_, AppCtx>) -> Result<AuthStatusDto, AppError> {
    build_auth_status_tauri(state.inner()).await
}

/// auth_me: для Tauri без lock — trusted_admin; с lock — будущий Plan 05 UI.
#[tauri::command]
#[specta::specta]
pub async fn auth_me(state: tauri::State<'_, AppCtx>) -> Result<Option<UserDto>, AppError> {
    // Desktop: нет cookie-сессии — возвращаем None.
    // Plan 05 UI реализует отслеживание текущего пользователя отдельно.
    let _ = state;
    Ok(None)
}

#[tauri::command]
#[specta::specta]
pub async fn server_toggle(
    state: tauri::State<'_, AppCtx>,
    enable: bool,
) -> Result<ServerStatusDto, AppError> {
    build_server_toggle_tauri(state.inner(), enable).await
}

#[tauri::command]
#[specta::specta]
pub async fn server_status(state: tauri::State<'_, AppCtx>) -> Result<ServerStatusDto, AppError> {
    build_server_status_tauri(state.inner()).await
}

#[tauri::command]
#[specta::specta]
pub async fn desktop_set_lock(
    state: tauri::State<'_, AppCtx>,
    enabled: bool,
) -> Result<(), AppError> {
    build_desktop_set_lock_tauri(state.inner(), enabled).await
}

/// Tauri-вариант сохранения сетевых настроек.
///
/// **Безопасность (T-05-SN-01/T-05-SN-02):** resolve_tauri_identity возвращает
/// trusted_admin при unlock-режиме, desktop_identity при lock-режиме.
/// authorize(&caller, ManageSettings) проверяет роль.
pub async fn build_settings_set_network_tauri(
    ctx: &AppCtx,
    patch: NetworkPatch,
) -> Result<(), AppError> {
    let caller = crate::tauri_cmds::users::resolve_tauri_identity(ctx).await?;
    trackly_core::auth::authorize(&caller, &Action::ManageSettings)?;

    // T-05-SN-03: validate port range.
    if patch.port < 1 || patch.port > 65535 {
        return Err(AppError::Validation {
            field: "port".to_string(),
            message: format!(
                "Порт должен быть в диапазоне 1..=65535, получено {}",
                patch.port
            ),
        });
    }

    let host = patch.host.clone();
    let port_str = patch.port.to_string();
    let cert_path = patch.cert_path.clone();
    let now = ctx.clock.unix_seconds();

    ctx.writer
        .execute(move |conn| {
            let upsert_sql =
                "INSERT INTO app_settings (key, value, created_at_utc, updated_at_utc) \
                              VALUES (?1, ?2, ?3, ?3) \
                              ON CONFLICT(key) DO UPDATE SET value = ?2, updated_at_utc = ?3";

            conn.execute(upsert_sql, rusqlite::params!["server_host", host, now])
                .map(|_| ())
                .map_err(map_rusqlite)?;

            conn.execute(upsert_sql, rusqlite::params!["server_port", port_str, now])
                .map(|_| ())
                .map_err(map_rusqlite)?;

            conn.execute(
                upsert_sql,
                rusqlite::params!["server_cert_path", cert_path, now],
            )
            .map(|_| ())
            .map_err(map_rusqlite)
        })
        .await
}

#[tauri::command]
#[specta::specta]
pub async fn settings_set_network(
    state: tauri::State<'_, AppCtx>,
    patch: NetworkPatch,
) -> Result<(), AppError> {
    build_settings_set_network_tauri(state.inner(), patch).await
}

/// Tauri-вариант чтения сетевых настроек + статуса сервера.
///
/// Зеркалит HTTP `build_settings_get_network`, но caller определяется через
/// `resolve_tauri_identity` (trusted_admin в unlock-режиме, desktop_identity в
/// lock-режиме) вместо session. `desktop_lock_enabled` читается из БД (live),
/// host/port/cert_path — из стартового config (D-Desktop-02).
pub async fn build_settings_get_network_tauri(
    ctx: &AppCtx,
) -> Result<NetworkSettingsDto, AppError> {
    let caller = crate::tauri_cmds::users::resolve_tauri_identity(ctx).await?;
    trackly_core::auth::authorize(&caller, &Action::ManageSettings)?;

    // Live app_settings поверх TOML-bootstrap — host/port/cert_path берутся из
    // сохранённых настроек (root-cause фикс server-bind-localhost-only), а не
    // только из стартового TOML-config.
    let net = crate::http::settings::resolve_effective_network(ctx).await?;
    let desktop_lock_enabled = ctx.auth.get_desktop_lock_enabled().await?;

    let running = {
        let guard = ctx.server_ctl.lock().await;
        guard.is_some()
    };

    let server_url = if running {
        Some(format!(
            "https://{}:{}",
            tls::display_host(&net.host),
            net.port
        ))
    } else {
        None
    };

    Ok(NetworkSettingsDto {
        enabled: ctx.config.server.enabled,
        host: net.host,
        port: net.port as i64,
        cert_path: net.cert_path,
        server_url,
        fingerprint: None,
        desktop_lock_enabled,
    })
}

#[tauri::command]
#[specta::specta]
pub async fn settings_get_network(
    state: tauri::State<'_, AppCtx>,
) -> Result<NetworkSettingsDto, AppError> {
    build_settings_get_network_tauri(state.inner()).await
}

/// Tauri-вариант чтения настроек AD: live (`enabled`/`auto_accept`) +
/// read-only bootstrap (`host`/.../`no_tls_verify` из `ctx.config.ad`).
///
/// Зеркалит `build_settings_get_ad` (HTTP), но caller определяется через
/// `resolve_tauri_identity` вместо session (Phase 9 Plan 04).
pub async fn build_settings_get_ad_tauri(ctx: &AppCtx) -> Result<AdSettingsDto, AppError> {
    let caller = crate::tauri_cmds::users::resolve_tauri_identity(ctx).await?;
    trackly_core::auth::authorize(&caller, &Action::ManageSettings)?;

    let enabled = ctx.auth.ad_enabled().await?;
    let auto_accept = ctx.auth.ad_auto_accept().await?;
    let sso_enabled = ctx.auth.ad_sso_enabled().await?;
    let ad_config = &ctx.config.ad;
    let sso_keytab_present =
        !ad_config.keytab_path.is_empty() && std::path::Path::new(&ad_config.keytab_path).is_file();

    Ok(AdSettingsDto {
        enabled,
        auto_accept,
        host: ad_config.host.clone(),
        port: ad_config.resolved_port() as i64,
        domain: ad_config.domain.clone(),
        base_dn: ad_config.base_dn.clone(),
        name_attr: ad_config.name_attr.clone(),
        no_tls_verify: ad_config.no_tls_verify,
        sso_enabled,
        sso_spn: ad_config.spn.clone(),
        sso_keytab_path: ad_config.keytab_path.clone(),
        sso_keytab_present,
    })
}

#[tauri::command]
#[specta::specta]
pub async fn settings_get_ad(state: tauri::State<'_, AppCtx>) -> Result<AdSettingsDto, AppError> {
    build_settings_get_ad_tauri(state.inner()).await
}

/// Tauri-вариант сохранения `enabled`/`auto_accept` (live AD toggle).
///
/// Подключение (host/port/domain/...) read-only — не редактируется здесь
/// (см. doc-comment `SetAdPayload` в http/auth.rs).
pub async fn build_settings_set_ad_tauri(
    ctx: &AppCtx,
    payload: SetAdPayload,
) -> Result<(), AppError> {
    let caller = crate::tauri_cmds::users::resolve_tauri_identity(ctx).await?;
    trackly_core::auth::authorize(&caller, &Action::ManageSettings)?;

    ctx.auth.set_ad_enabled(payload.enabled, &caller).await?;
    ctx.auth
        .set_ad_auto_accept(payload.auto_accept, &caller)
        .await?;
    ctx.auth
        .set_ad_sso_enabled(payload.sso_enabled, &caller)
        .await?;

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn settings_set_ad(
    state: tauri::State<'_, AppCtx>,
    payload: SetAdPayload,
) -> Result<(), AppError> {
    build_settings_set_ad_tauri(state.inner(), payload).await
}

/// Tauri-вариант проверки доступности AD-сервера ("Проверить подключение",
/// Phase 9 gap-closure). Caller определяется через `resolve_tauri_identity`
/// вместо session (зеркалирует `build_settings_set_ad_tauri`).
pub async fn build_ad_test_connection_tauri(ctx: &AppCtx) -> Result<(), AppError> {
    let caller = crate::tauri_cmds::users::resolve_tauri_identity(ctx).await?;
    ctx.auth.test_ad_connection(&caller).await
}

#[tauri::command]
#[specta::specta]
pub async fn ad_test_connection(state: tauri::State<'_, AppCtx>) -> Result<(), AppError> {
    build_ad_test_connection_tauri(state.inner()).await
}
