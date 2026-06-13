//! Settings + server control HTTP routes — Plan 03.
//!
//! Маршруты:
//! - POST /api/v1/settings_get_network — чтение настроек сервера
//! - POST /api/v1/settings_set_network — сохранение настроек (TODO: Phase 5+)
//! - POST /api/v1/server_toggle — старт / стоп axum сервера
//! - POST /api/v1/server_status — текущий статус
//! - POST /api/v1/desktop_set_lock — установка флага desktop_lock_enabled
//!
//! Все маршруты защищены session middleware.

use axum::{extract::State, routing::post, Json, Router};
use serde::Deserialize;
use std::net::SocketAddr;
use tower_sessions::Session;

use trackly_core::auth::Action;
use trackly_core::error::AppError;

use crate::context::AppCtx;
use crate::dto::auth::{NetworkSettingsDto, ServerStatusDto};
use crate::error_axum::AppErrorResponse;
use crate::http::auth::session_identity;
use crate::server::rusqlite_session_store::RusqliteSessionStore;
use crate::server::tls;
use crate::server::{start_server, ServerHandle};

// ---------------------------------------------------------------------------
// Payload structs
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct ServerTogglePayload {
    pub enable: bool,
}

#[derive(Debug, Deserialize)]
pub struct DesktopSetLockPayload {
    pub enabled: bool,
}

// ---------------------------------------------------------------------------
// build_* helpers
// ---------------------------------------------------------------------------

/// Вернуть текущие сетевые настройки + статус сервера.
pub async fn build_settings_get_network(
    ctx: &AppCtx,
    session: &Session,
) -> Result<NetworkSettingsDto, AppError> {
    let caller = session_identity(session).await?;
    trackly_core::auth::authorize(&caller, &Action::ManageSettings)?;

    let config = &ctx.config.server;
    let desktop_lock_enabled = ctx.auth.get_desktop_lock_enabled().await?;

    let running = {
        let guard = ctx.server_ctl.lock().await;
        guard.is_some()
    };

    let server_url = if running {
        Some(format!("https://{}:{}", config.host, config.port))
    } else {
        None
    };

    Ok(NetworkSettingsDto {
        enabled: config.enabled,
        host: config.host.clone(),
        port: config.port as i64,
        cert_path: config.cert_path.clone(),
        server_url,
        fingerprint: None, // TODO: store fingerprint in server_ctl
        desktop_lock_enabled,
    })
}

/// Старт / стоп axum сервера.
pub async fn build_server_toggle(
    ctx: &AppCtx,
    session: &Session,
    session_store: RusqliteSessionStore,
    enable: bool,
) -> Result<ServerStatusDto, AppError> {
    let caller = session_identity(session).await?;
    trackly_core::auth::authorize(&caller, &Action::ManageSettings)?;

    let config = &ctx.config.server;

    if !enable {
        // Остановить сервер если запущен.
        let mut guard = ctx.server_ctl.lock().await;
        if let Some(handle) = guard.take() {
            handle.cancel.cancel();
            // Не ждём join — сервер останавливается асинхронно.
            drop(handle.task);
        }
        return Ok(ServerStatusDto {
            running: false,
            url: None,
            fingerprint: None,
        });
    }

    // Остановить предыдущий инстанс если есть.
    {
        let mut guard = ctx.server_ctl.lock().await;
        if let Some(handle) = guard.take() {
            handle.cancel.cancel();
            drop(handle.task);
        }
    }

    // Построить TLS bundle.
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
        // Предполагаем key_path = cert_path с расширением .key
        let key_path = config.cert_path.replace(".crt", ".key").replace(".pem", ".key");
        let key_pem =
            std::fs::read_to_string(&key_path).map_err(|e| AppError::Internal {
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

    let addr: SocketAddr = format!("{}:{}", host, port).parse().map_err(|e| {
        AppError::Validation {
            field: "server.host".to_string(),
            message: format!("invalid bind address: {e}"),
        }
    })?;

    // Построить Router.
    let router = crate::http::build_router(ctx, session_store);

    // Дочерний CancellationToken — не трогает мастер AppCtx.shutdown.
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

    let handle = ServerHandle { cancel, task };
    {
        let mut guard = ctx.server_ctl.lock().await;
        *guard = Some(handle);
    }

    Ok(ServerStatusDto {
        running: true,
        url: Some(url),
        fingerprint: Some(fingerprint),
    })
}

/// Текущий статус сервера.
pub async fn build_server_status(ctx: &AppCtx) -> Result<ServerStatusDto, AppError> {
    let guard = ctx.server_ctl.lock().await;
    let running = guard.is_some();
    Ok(ServerStatusDto {
        running,
        url: None,
        fingerprint: None,
    })
}

/// Установить флаг desktop_lock_enabled.
pub async fn build_desktop_set_lock_http(
    ctx: &AppCtx,
    session: &Session,
    enabled: bool,
) -> Result<(), AppError> {
    let caller = session_identity(session).await?;
    ctx.auth.set_desktop_lock_enabled(enabled, &caller).await
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

pub async fn handler_get_network(
    State(ctx): State<AppCtx>,
    session: Session,
) -> Result<Json<NetworkSettingsDto>, AppErrorResponse> {
    Ok(Json(
        build_settings_get_network(&ctx, &session)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_server_toggle(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(payload): Json<ServerTogglePayload>,
) -> Result<Json<ServerStatusDto>, AppErrorResponse> {
    let session_store = RusqliteSessionStore::new(ctx.writer.clone(), ctx.readers.clone());
    Ok(Json(
        build_server_toggle(&ctx, &session, session_store, payload.enable)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_server_status(
    State(ctx): State<AppCtx>,
) -> Result<Json<ServerStatusDto>, AppErrorResponse> {
    Ok(Json(
        build_server_status(&ctx)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_desktop_set_lock(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(payload): Json<DesktopSetLockPayload>,
) -> Result<Json<()>, AppErrorResponse> {
    build_desktop_set_lock_http(&ctx, &session, payload.enabled)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(()))
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router() -> Router<AppCtx> {
    Router::new()
        .route("/api/v1/settings_get_network", post(handler_get_network))
        .route("/api/v1/server_toggle", post(handler_server_toggle))
        .route("/api/v1/server_status", post(handler_server_status))
        .route("/api/v1/desktop_set_lock", post(handler_desktop_set_lock))
}
