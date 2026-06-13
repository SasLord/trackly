//! Auth Tauri commands — Plan 03.
//!
//! Десктоп-транспорт для auth операций.
//! Tauri commands НЕ используют Session extractor — сессии только для HTTP.
//!
//! Паттерн: `build_*` helper + тонкий `#[tauri::command] #[specta::specta]` wrapper.
//! `#[specta::specta]` ПОСЛЕ `#[tauri::command]` — требование tauri-specta v2 rc.21.

use crate::context::AppCtx;
use crate::dto::auth::{AuthStatusDto, LoginRequest, ServerStatusDto, UserDto};
use crate::server::rusqlite_session_store::RusqliteSessionStore;
use crate::server::tls;
use crate::server::{start_server, ServerHandle};
use trackly_core::auth::Identity;
use trackly_core::error::AppError;

// ---------------------------------------------------------------------------
// build_* helpers
// ---------------------------------------------------------------------------

/// Таури-логин: аутентификация без Session.
pub async fn build_auth_login_tauri(
    ctx: &AppCtx,
    req: LoginRequest,
) -> Result<UserDto, AppError> {
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

/// Таури server_toggle: старт / стоп axum сервера.
pub async fn build_server_toggle_tauri(
    ctx: &AppCtx,
    enable: bool,
) -> Result<ServerStatusDto, AppError> {
    let config = &ctx.config.server;

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

    // TLS bundle.
    let tls_bundle = if config.cert_path.is_empty() {
        let host = config.host.clone();
        tls::generate_self_signed(&host).map_err(|e| AppError::Internal {
            source_chain: format!("generate_self_signed: {e}"),
        })?
    } else {
        let cert_pem = std::fs::read_to_string(&config.cert_path).map_err(|e| {
            AppError::Internal {
                source_chain: format!("read cert: {e}"),
            }
        })?;
        let key_path = config
            .cert_path
            .replace(".crt", ".key")
            .replace(".pem", ".key");
        let key_pem = std::fs::read_to_string(&key_path).map_err(|e| AppError::Internal {
            source_chain: format!("read key: {e}"),
        })?;
        tls::load_from_pem(&cert_pem, &key_pem).map_err(|e| AppError::Internal {
            source_chain: format!("load_from_pem: {e}"),
        })?
    };

    let fingerprint = tls_bundle.fingerprint_hex.clone();
    let host = config.host.clone();
    let port = config.port;
    let url = format!("https://{}:{}", host, port);

    let addr: std::net::SocketAddr = format!("{}:{}", host, port).parse().map_err(|e| {
        AppError::Validation {
            field: "server.host".to_string(),
            message: format!("invalid bind address: {e}"),
        }
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
/// Всегда использует trusted_admin — это команда настройки, доступна без входа.
pub async fn build_desktop_set_lock_tauri(
    ctx: &AppCtx,
    enabled: bool,
) -> Result<(), AppError> {
    ctx.auth
        .set_desktop_lock_enabled(enabled, &Identity::trusted_admin())
        .await
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

/// auth_logout: для Tauri — no-op (нет cookie-сессии).
#[tauri::command]
#[specta::specta]
pub async fn auth_logout(_state: tauri::State<'_, AppCtx>) -> Result<(), AppError> {
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn auth_status(
    state: tauri::State<'_, AppCtx>,
) -> Result<AuthStatusDto, AppError> {
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
pub async fn server_status(
    state: tauri::State<'_, AppCtx>,
) -> Result<ServerStatusDto, AppError> {
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
