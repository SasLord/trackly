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
/// 1. Все маршруты используют `SessionManagerLayer` — Session extractor работает везде.
/// 2. Публичные маршруты (auth_login, auth_status) — не требуют наличия identity,
///    но session layer необходима для Session extractor.
/// 3. Защищённые маршруты делают проверку identity в handlers (session_identity()).
/// 4. auth_login дополнительно обёрнут в GovernorLayer (rate limit).
/// 5. Security headers применяются ко всем ответам.
/// 6. Fallback: статические файлы Svelte SPA.
pub fn build_router(ctx: &AppCtx, session_store: RusqliteSessionStore) -> Router {
    // --- Session layer (применяется ко всему router) ---
    // Session extractor требует наличия SessionManagerLayer выше по стеку.
    // Публичные маршруты (login/status) имеют доступ к session но не требуют identity.
    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(true)
        .with_http_only(true)
        .with_same_site(SameSite::Strict)
        .with_expiry(Expiry::OnInactivity(time::Duration::days(30)));

    // --- Rate limit config для /api/v1/auth_login ---
    // burst=5, per_second=1 (строго 1 запрос в секунду, burst до 5) — D-Auth-02.
    let governor_conf = Arc::new(
        tower_governor::governor::GovernorConfigBuilder::default()
            .per_second(1)
            .burst_size(5)
            .finish()
            .expect("governor config build failed"),
    );

    // --- auth_login route с rate limit (применяется только к этому маршруту) ---
    // GovernorLayer применяется через .route_layer() — только к /api/v1/auth_login.
    let login_route = axum::routing::Router::new()
        .route(
            "/api/v1/auth_login",
            axum::routing::post(auth::handler_login),
        )
        .route_layer(tower_governor::GovernorLayer::new(governor_conf));

    // --- auth_status без rate limit ---
    let status_route = axum::routing::Router::new()
        .route(
            "/api/v1/auth_status",
            axum::routing::post(auth::handler_status),
        );

    // --- Весь API Router (public + protected routes) ---
    // Защита маршрутов реализована на уровне handlers через session_identity().
    // Session layer применяется ко всем маршрутам (Session extractor требует этого).
    let api_router = Router::new()
        .merge(login_route)
        .merge(status_route)
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
        // Session layer применяется ко всем маршрутам
        .layer(session_layer);

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
