//! Settings + server control HTTP routes — Plan 03 / Plan 05-06.
//!
//! Маршруты:
//! - POST /api/v1/settings_get_network — чтение настроек сервера
//! - POST /api/v1/settings_set_network — сохранение настроек (host, port, cert_path)
//! - POST /api/v1/server_toggle — старт / стоп axum сервера
//! - POST /api/v1/server_status — текущий статус
//! - POST /api/v1/desktop_set_lock — установка флага desktop_lock_enabled
//!
//! Все маршруты защищены session middleware.

use axum::{extract::State, routing::post, Json, Router};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::net::SocketAddr;
use tower_sessions::Session;

use trackly_core::auth::Action;
use trackly_core::error::AppError;
use trackly_infra::error_conversions::map_rusqlite;

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
#[serde(rename_all = "camelCase")]
pub struct ServerTogglePayload {
    pub enable: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopSetLockPayload {
    pub enabled: bool,
}

/// Поля патча сетевых настроек, совпадают с тем, что шлёт saveSettings() в NetworkSettings.svelte.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct NetworkPatch {
    pub host: String,
    /// Порт сервера (1..=65535). UI передаёт `number`, поэтому i64 на уровне JSON.
    #[specta(type = i32)]
    pub port: i64,
    /// Путь к PEM-сертификату. Пустая строка → использовать самоподписанный.
    pub cert_path: String,
}

/// Тело запроса POST /api/v1/settings_set_network.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetNetworkPayload {
    pub patch: NetworkPatch,
}

// ---------------------------------------------------------------------------
// Effective network settings (live app_settings over TOML bootstrap)
// ---------------------------------------------------------------------------

/// Эффективные сетевые настройки сервера на момент bind'а.
///
/// **Почему это нужно (root-cause фикс server-bind-localhost-only):**
/// `settings_set_network` сохраняет `server_host`/`server_port`/`server_cert_path`
/// в таблицу `app_settings`, но раньше НИ ОДИН путь не читал их обратно —
/// и старт сервера (`main.rs`), и hot-toggle (`build_server_toggle`) биндили
/// из `ctx.config.server` (TOML, дефолт `127.0.0.1`). Поэтому выбранный в
/// Настройках `0.0.0.0` никогда не доходил до `TcpListener::bind`, и сервер
/// всегда слушал только localhost.
///
/// `app_settings` — live источник истины; `ctx.config.server` (TOML) —
/// bootstrap-дефолт на случай отсутствия ключа.
pub struct EffectiveNetwork {
    pub host: String,
    pub port: u16,
    pub cert_path: String,
    pub key_path: String,
}

/// Прочитать одно значение из `app_settings` по ключу (None если нет строки).
async fn read_app_setting(ctx: &AppCtx, key: &'static str) -> Result<Option<String>, AppError> {
    let readers = ctx.readers.clone();
    tokio::task::spawn_blocking(move || -> Result<Option<String>, AppError> {
        let conn = readers.acquire();
        let result: rusqlite::Result<String> = conn.query_row(
            "SELECT value FROM app_settings WHERE key = ?1",
            rusqlite::params![key],
            |r| r.get(0),
        );
        match result {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(map_rusqlite(e)),
        }
    })
    .await
    .map_err(|e| AppError::Internal {
        source_chain: format!("spawn_blocking read_app_setting {key}: {e}"),
    })?
}

/// Разрешить эффективные сетевые настройки: live `app_settings` поверх
/// TOML-bootstrap `ctx.config.server`.
pub async fn resolve_effective_network(ctx: &AppCtx) -> Result<EffectiveNetwork, AppError> {
    let cfg = &ctx.config.server;

    let host = match read_app_setting(ctx, "server_host").await? {
        Some(h) if !h.trim().is_empty() => h,
        _ => cfg.host.clone(),
    };

    let port = match read_app_setting(ctx, "server_port").await? {
        Some(p) => p.trim().parse::<u16>().unwrap_or(cfg.port),
        None => cfg.port,
    };

    let cert_path = match read_app_setting(ctx, "server_cert_path").await? {
        Some(c) => c,
        None => cfg.cert_path.clone(),
    };

    Ok(EffectiveNetwork {
        host,
        port,
        cert_path,
        key_path: cfg.key_path.clone(),
    })
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

    // Live app_settings поверх TOML-bootstrap — UI должен показывать то, что
    // реально попадёт в bind (root-cause фикс server-bind-localhost-only).
    let net = resolve_effective_network(ctx).await?;
    let desktop_lock_enabled = ctx.auth.get_desktop_lock_enabled().await?;

    let running = {
        let guard = ctx.server_ctl.lock().await;
        guard.is_some()
    };

    let server_url = if running {
        Some(format!("https://{}:{}", net.host, net.port))
    } else {
        None
    };

    Ok(NetworkSettingsDto {
        enabled: ctx.config.server.enabled,
        host: net.host,
        port: net.port as i64,
        cert_path: net.cert_path,
        server_url,
        fingerprint: None, // TODO: store fingerprint in server_ctl
        desktop_lock_enabled,
    })
}

/// Сохранить сетевые настройки (host, port, cert_path) в таблице app_settings.
///
/// **Безопасность (T-05-SN-01):** require ManageSettings — только admin.
/// Неаутентифицированный caller получает 401 от session_identity;
/// non-admin caller получает 403 от authorize.
pub async fn build_settings_set_network(
    ctx: &AppCtx,
    session: &Session,
    patch: NetworkPatch,
) -> Result<(), AppError> {
    let caller = session_identity(session).await?;
    trackly_core::auth::authorize(&caller, &Action::ManageSettings)?;

    // T-05-SN-03: validate port range before writing.
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

/// Старт / стоп axum сервера.
pub async fn build_server_toggle(
    ctx: &AppCtx,
    session: &Session,
    session_store: RusqliteSessionStore,
    enable: bool,
) -> Result<ServerStatusDto, AppError> {
    let caller = session_identity(session).await?;
    trackly_core::auth::authorize(&caller, &Action::ManageSettings)?;

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

    // Эффективные настройки: live app_settings поверх TOML-bootstrap
    // (root-cause фикс server-bind-localhost-only — host из Настроек теперь
    // реально доходит до bind, а не игнорируется в пользу config.host).
    let net = resolve_effective_network(ctx).await?;

    // Построить TLS bundle.
    let tls_bundle = if net.cert_path.is_empty() {
        let host = net.host.clone();
        tls::generate_self_signed(&host).map_err(|e| AppError::Internal {
            source_chain: format!("generate_self_signed: {e}"),
        })?
    } else {
        // WR-01: explicit/validated key-path resolution (no brittle .replace heuristic).
        tls::load_from_files(&net.cert_path, &net.key_path).map_err(|e| {
            AppError::Internal {
                source_chain: format!("load_from_files: {e}"),
            }
        })?
    };

    let fingerprint = tls_bundle.fingerprint_hex.clone();
    let host = net.host.clone();
    let port = net.port;
    let url = format!("https://{}:{}", host, port);

    let addr: SocketAddr =
        format!("{}:{}", host, port)
            .parse()
            .map_err(|e| AppError::Validation {
                field: "server.host".to_string(),
                message: format!("invalid bind address: {e}"),
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

pub async fn handler_set_network(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(payload): Json<SetNetworkPayload>,
) -> Result<Json<()>, AppErrorResponse> {
    build_settings_set_network(&ctx, &session, payload.patch)
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
        .route("/api/v1/settings_set_network", post(handler_set_network))
        .route("/api/v1/server_toggle", post(handler_server_toggle))
        .route("/api/v1/server_status", post(handler_server_status))
        .route("/api/v1/desktop_set_lock", post(handler_desktop_set_lock))
}
