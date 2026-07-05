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
pub mod dashboard;
pub mod devices;
pub mod fs_helpers;
pub mod health;
pub mod organization;
pub mod printers;
pub mod reports;
pub mod requests;
pub mod settings;
pub mod settings_org;
pub mod templates;
pub mod users;
pub mod ws;

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
/// 2. Публичные маршруты (auth_login, request_ad_restore, auth_status) — не
///    требуют наличия identity, но session layer необходима для Session
///    extractor.
/// 3. Защищённые маршруты делают проверку identity в handlers (session_identity()).
/// 4. auth_login и request_ad_restore дополнительно обёрнуты в GovernorLayer
///    (rate limit) — оба несут пользовательские AD credentials.
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

    // --- /api/v1/request_ad_restore — отдельный governor (та же burst/rate
    // конфигурация, что и auth_login): этот endpoint ТАКЖЕ выполняет AD bind
    // с пользовательскими credentials (09-AD-GAPS restoration-flow UX), и
    // заслуживает той же brute-force защиты.
    let restore_governor_conf = Arc::new(
        tower_governor::governor::GovernorConfigBuilder::default()
            .per_second(1)
            .burst_size(5)
            .finish()
            .expect("governor config build failed"),
    );
    let restore_route = axum::routing::Router::new()
        .route(
            "/api/v1/request_ad_restore",
            axum::routing::post(auth::handler_request_ad_restore),
        )
        .route_layer(tower_governor::GovernorLayer::new(restore_governor_conf));

    // --- auth_status без rate limit ---
    let status_route = axum::routing::Router::new().route(
        "/api/v1/auth_status",
        axum::routing::post(auth::handler_status),
    );

    // --- Весь API Router (public + protected routes) ---
    // Защита маршрутов реализована на уровне handlers через session_identity().
    // Session layer применяется ко всем маршрутам (Session extractor требует этого).
    let api_router = Router::new()
        .merge(login_route)
        .merge(restore_route)
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
        .merge(printers::router())
        .merge(requests::router())
        .merge(ws::router())
        .merge(reports::router())
        .merge(dashboard::router())
        .merge(settings_org::router())
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
            // WR-07: drop 'unsafe-inline' from script-src — Vite emits external
            // bundles, so inline scripts are not required and 'unsafe-inline'
            // would neutralize CSP's XSS protection. Kept on style-src for
            // Svelte scoped styles.
            // T-06-12-I: connect-src includes wss: for same-origin WebSocket upgrade.
            // PDF-CSP: frame-src/object-src allow blob: so PdfPreviewModal and
            // TemplateEditor can render their `blob:` PDF URLs in <iframe>. Without
            // this, default-src 'self' blocks blob: framing and the preview is blank
            // (browser: "Refused to load blob:... neither frame-src nor default-src").
            // GAP-16-01: img-src 'self' data: — the act HTML embeds the org logo as a
            // data:image URI (act_handover.html). Without an explicit img-src it falls
            // back to default-src 'self', which does NOT permit data:, so the logo is
            // blocked in the LAN-browser preview and print (desktop opens file:// in the
            // system browser outside this CSP, so it was unaffected — LAN-only symptom).
            // data: images cannot execute scripts, so this does not weaken XSS defense.
            HeaderValue::from_static(
                "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src 'self' wss:; frame-src 'self' blob:; object-src 'self' blob:",
            ),
        ));

    // --- Svelte SPA fallback (embedded via rust-embed) ---
    // Assets are baked into the binary in release builds (portable: nothing to
    // ship beside the .exe) and read from ui/dist on disk in debug builds.
    api_router
        .fallback(spa_fallback)
        .layer(security_headers)
        .with_state(ctx.clone())
}

/// Svelte SPA build, embedded into the binary (release) or read from
/// `crates/trackly-app/../../ui/dist` on disk (debug). Built by the
/// `beforeBuildCommand` (`pnpm --dir ../ui build`) before `tauri build`.
#[derive(rust_embed::RustEmbed)]
#[folder = "../../ui/dist"]
struct SpaAssets;

/// Fallback handler that serves the embedded SPA. Serves the requested asset by
/// path; unknown paths fall back to `index.html` (the SPA uses a hash router, so
/// every client route is reachable from `/`).
async fn spa_fallback(uri: axum::http::Uri) -> axum::response::Response {
    use axum::http::{header, StatusCode};
    use axum::response::IntoResponse;

    let path = uri.path().trim_start_matches('/');
    let lookup = if path.is_empty() { "index.html" } else { path };

    if let Some(file) = SpaAssets::get(lookup) {
        let mime = file.metadata.mimetype().to_string();
        return ([(header::CONTENT_TYPE, mime)], file.data.into_owned()).into_response();
    }

    match SpaAssets::get("index.html") {
        Some(file) => (
            [(header::CONTENT_TYPE, "text/html; charset=utf-8".to_string())],
            file.data.into_owned(),
        )
            .into_response(),
        None => (
            StatusCode::NOT_FOUND,
            "UI assets are not bundled (ui/dist was missing at build time)",
        )
            .into_response(),
    }
}
