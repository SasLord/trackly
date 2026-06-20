//! D-UX-02 («Запомнить меня») regression test — Phase 9 Plan 04.
//!
//! `POST /api/v1/auth_login` с `remember: true` должен выставить cookie с
//! `Max-Age`/`Expires` (постоянная, скользящее истечение 30 дней —
//! `Expiry::OnInactivity`). `remember: false` (или отсутствие поля,
//! `#[serde(default)]`) должен выставить cookie БЕЗ `Max-Age`/`Expires`
//! (`Expiry::OnSessionEnd` — браузер удаляет её при закрытии).
//!
//! **Почему реальный TCP, а не `oneshot`:** `/api/v1/auth_login` защищён
//! `GovernorLayer` (rate limit, D-Auth-02), который извлекает peer IP из
//! реального соединения. `oneshot()` не создаёт TCP-сокет → governor
//! возвращает 500 "Unable To Extract Key!" (см. `security_headers.rs`
//! `rate_limit_on_login` — тот тест явно избегает проверки статуса 200 по
//! этой причине). Этот тест поднимает `axum::serve` на реальном порту и
//! шлёт настоящий HTTP/1.1 запрос через `TcpStream`, чтобы governor мог
//! получить peer addr и не возвращал 500.

use std::time::Duration;

use axum::Router;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio_util::sync::CancellationToken;

use trackly_app::dto::auth::UserNew;
use trackly_app::http::build_router;
use trackly_app::server::rusqlite_session_store::RusqliteSessionStore;

/// Отправить raw HTTP/1.1 POST через TCP, вернуть (status_code, set_cookie_header, body).
async fn raw_post_login(addr: std::net::SocketAddr, body: &str) -> anyhow::Result<(u16, String)> {
    let mut stream = TcpStream::connect(addr).await?;
    let request = format!(
        "POST /api/v1/auth_login HTTP/1.1\r\n\
         Host: {addr}\r\n\
         Content-Type: application/json\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\
         \r\n\
         {body}",
        body.len()
    );
    stream.write_all(request.as_bytes()).await?;

    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).await?;
    let text = String::from_utf8_lossy(&raw).to_string();

    let mut lines = text.split("\r\n");
    let status_line = lines.next().unwrap_or_default();
    let status_code: u16 = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    let set_cookie = text
        .lines()
        .find(|l| l.to_lowercase().starts_with("set-cookie:"))
        .unwrap_or_default()
        .to_string();

    if status_code != 200 {
        eprintln!("DEBUG raw response:\n{text}");
    }

    Ok((status_code, set_cookie))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn login_remember_persistent_cookie() -> anyhow::Result<()> {
    tokio::time::timeout(Duration::from_secs(30), async {
        let dir = tempfile::TempDir::new()?;
        let paths = trackly_infra::Paths::resolve_for_exe_dir(dir.path().to_path_buf())?;
        let config = trackly_infra::AppConfig::default();
        let log_guard = trackly_app::logging::init(&paths, &config).or_else(|_| {
            let (_nb, guard) = tracing_appender::non_blocking(std::io::sink());
            Ok::<_, anyhow::Error>(guard)
        })?;
        let ctx = trackly_app::context::AppCtx::build(paths, config, log_guard).await?;

        ctx.auth
            .create_user(
                UserNew {
                    login: "remember_user".to_string(),
                    full_name: "Тест Remember".to_string(),
                    password: "password123".to_string(),
                    role: "employee".to_string(),
                    email: None,
                },
                &trackly_core::auth::Identity::trusted_admin(),
            )
            .await?;

        let session_store = RusqliteSessionStore::new(ctx.writer.clone(), ctx.readers.clone());
        let router: Router = build_router(&ctx, session_store);

        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;
        let shutdown = CancellationToken::new();
        let shutdown_clone = shutdown.clone();
        let server_handle = tokio::spawn(async move {
            // into_make_service_with_connect_info injects ConnectInfo<SocketAddr> per
            // connection, which tower_governor's PeerIpKeyExtractor needs to compute the
            // per-IP rate-limit bucket on /api/v1/auth_login. Plain axum::serve(listener,
            // router) leaves ConnectInfo absent → 500 "Unable To Extract Key!" (mirrors
            // the manual ConnectInfo injection in server/mod.rs's TLS accept loop).
            axum::serve(
                listener,
                router.into_make_service_with_connect_info::<std::net::SocketAddr>(),
            )
            .with_graceful_shutdown(async move { shutdown_clone.cancelled().await })
            .await
            .expect("axum::serve");
        });

        tokio::time::sleep(Duration::from_millis(50)).await;

        // remember=true → persistent cookie (Max-Age / Expires present).
        let (status_remember, cookie_remember) = raw_post_login(
            addr,
            &serde_json::json!({
                "req": {
                    "login": "remember_user",
                    "password": "password123",
                    "remember": true,
                }
            })
            .to_string(),
        )
        .await?;
        assert_eq!(
            status_remember, 200,
            "remember=true login должен вернуть 200, заголовок Set-Cookie: {cookie_remember}"
        );
        assert!(
            cookie_remember.to_lowercase().contains("max-age")
                || cookie_remember.to_lowercase().contains("expires"),
            "remember=true должен выставить постоянную cookie (Max-Age/Expires), \
             получили: {cookie_remember}"
        );

        // remember=false → session-only cookie (no Max-Age / Expires).
        let (status_no_remember, cookie_no_remember) = raw_post_login(
            addr,
            &serde_json::json!({
                "req": {
                    "login": "remember_user",
                    "password": "password123",
                    "remember": false,
                }
            })
            .to_string(),
        )
        .await?;
        assert_eq!(status_no_remember, 200);
        assert!(
            !cookie_no_remember.to_lowercase().contains("max-age")
                && !cookie_no_remember.to_lowercase().contains("expires"),
            "remember=false должен выставить session-only cookie (без Max-Age/Expires), \
             получили: {cookie_no_remember}"
        );

        // Default (no `remember` field at all) → behaves like remember=false
        // (#[serde(default)], D-UX-02).
        let (status_default, cookie_default) = raw_post_login(
            addr,
            &serde_json::json!({
                "req": {
                    "login": "remember_user",
                    "password": "password123",
                }
            })
            .to_string(),
        )
        .await?;
        assert_eq!(status_default, 200);
        assert!(
            !cookie_default.to_lowercase().contains("max-age")
                && !cookie_default.to_lowercase().contains("expires"),
            "remember отсутствует → default false → session-only cookie, \
             получили: {cookie_default}"
        );

        shutdown.cancel();
        let _ = tokio::time::timeout(Duration::from_secs(5), server_handle).await;
        ctx.shutdown.cancel();
        Ok::<(), anyhow::Error>(())
    })
    .await
    .expect("auth_remember_cookie exceeded 30 s budget")?;
    Ok(())
}
