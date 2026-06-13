//! axum HTTP routes. Phase 5: auth + users + settings + session middleware +
//! security headers + rate limiting.
//!
//! `build_router(ctx, session_store)` — единая точка сборки Router со всеми
//! middleware. Используется в main.rs (server auto-start) и server_toggle команде.
//!
//! Инвариант «один DTO, два транспорта»: каждый axum handler делегирует тому же
//! `build_*` helper что и Tauri command.

pub mod acts;
pub mod auth;
pub mod cartridges;
pub mod devices;
pub mod fs_helpers;
pub mod health;
pub mod organization;
pub mod settings;
pub mod templates;
pub mod users;

use axum::{http::HeaderValue, Router};
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::set_header::SetResponseHeaderLayer;
use tower_sessions::{cookie::SameSite, Expiry, SessionManagerLayer};

use crate::context::AppCtx;
use crate::server::rusqlite_session_store::RusqliteSessionStore;

/// Построить полный axum Router со всеми middleware.
///
/// Топология:
/// 1. Публичные маршруты (auth_login, auth_status) — без session gate, с rate limit.
/// 2. Защищённые маршруты — за `SessionManagerLayer`.
/// 3. Security headers применяются ко всем ответам.
/// 4. Fallback: статические файлы Svelte SPA.
pub fn build_router(ctx: &AppCtx, session_store: RusqliteSessionStore) -> Router {
    // --- Session layer ---
    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(true)
        .with_http_only(true)
        .with_same_site(SameSite::Strict)
        .with_expiry(Expiry::OnInactivity(time::Duration::days(30)));

    // --- Rate limit config для /api/v1/auth_login ---
    // burst=5, per_second=1 (строго 1 запрос в секунду, burst до 5).
    let governor_conf = Arc::new(
        tower_governor::governor::GovernorConfigBuilder::default()
            .per_second(1)
            .burst_size(5)
            .finish()
            .expect("governor config build failed"),
    );

    // --- Публичные маршруты (без session gate) ---
    // auth_login с rate limit, auth_status без.
    let public_routes = auth::public_router()
        .layer(tower_governor::GovernorLayer::new(governor_conf));

    // --- Защищённые маршруты ---
    let protected_routes = Router::new()
        .merge(auth::protected_router())
        .merge(users::router())
        .merge(settings::router())
        .merge(health::router())
        .merge(devices::router())
        .merge(acts::router())
        .merge(cartridges::router())
        .merge(organization::router())
        .merge(templates::router())
        .merge(fs_helpers::router())
        .layer(session_layer);

    // --- Объединённый Router ---
    let api_router = Router::new()
        .merge(public_routes)
        .merge(protected_routes);

    // --- Security headers (T-05-14, глобально) ---
    let security_headers = ServiceBuilder::new()
        .layer(SetResponseHeaderLayer::overriding(
            axum::http::header::HeaderName::from_static("x-frame-options"),
            HeaderValue::from_static("DENY"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            axum::http::header::HeaderName::from_static("x-content-type-options"),
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            axum::http::header::HeaderName::from_static("content-security-policy"),
            HeaderValue::from_static(
                "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'",
            ),
        ));

    // --- Svelte SPA fallback ---
    // ServeDir из exe_dir/ui/dist. Если каталог не существует — silently falls back.
    let ui_dist = ctx.paths.exe_dir().join("ui/dist");
    let fallback_service = tower_http::services::ServeDir::new(ui_dist)
        .append_index_html_on_directories(true);

    api_router
        .fallback_service(fallback_service)
        .layer(security_headers)
        .with_state(ctx.clone())
}
