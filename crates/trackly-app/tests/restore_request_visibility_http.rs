//! Regression repro (09-AD-GAPS restoration-flow UX) — HTTP-transport,
//! end-to-end, mirrors the EXACT browser flow reported by the user:
//!
//! 1. Blocked/soft-deleted AD user clicks "Запросить восстановление доступа"
//!    on `BlockedScreen` → `POST /api/v1/request_ad_restore`.
//! 2. The resulting restore request must show up for the admin (the request
//!    list the admin UI calls — `POST /api/v1/requests_list`).
//! 3. A subsequent plain login by the same blocked user must report
//!    `pending: true` (read via `AuthService::latest_restore_request_state`,
//!    surfaced through `AppError::AccessBlocked.details`).
//!
//! **Real TCP, not `oneshot()`:** `/api/v1/auth_login` and
//! `/api/v1/request_ad_restore` are both behind `GovernorLayer` (rate
//! limit, D-Auth-02), which extracts the peer IP from a real connection.
//! `oneshot()` has no socket → governor returns 500 "Unable To Extract
//! Key!" (see `auth_remember_cookie.rs`'s doc comment for the same
//! constraint). This test binds a real listener via `axum::serve` +
//! `into_make_service_with_connect_info` and speaks raw HTTP/1.1 over
//! `TcpStream`, mirroring `auth_remember_cookie.rs`'s harness.

use std::time::Duration;

use rusqlite::params;
use serde_json::Value;
use time::{Duration as TimeDuration, OffsetDateTime};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_util::sync::CancellationToken;
use tower_sessions::session::{Id, Record};
use tower_sessions::SessionStore;

use trackly_app::context::AppCtx;
use trackly_app::dto::auth::UserNew;
use trackly_app::http::auth::SessionIdentity;
use trackly_app::http::build_router;
use trackly_app::server::rusqlite_session_store::RusqliteSessionStore;
use trackly_core::auth::{Identity, Role};
use trackly_core::error::AppError;
use trackly_core::primitives::clock::Clock;
use trackly_infra::clock_impl::SystemClock;

async fn make_test_ctx() -> anyhow::Result<(AppCtx, tempfile::TempDir)> {
    let dir = tempfile::TempDir::new()?;
    let dir_path = dir.path().to_path_buf();
    let paths = trackly_infra::Paths::resolve_for_exe_dir(dir_path)?;
    let config = trackly_infra::AppConfig::default();
    let log_guard = trackly_app::logging::init(&paths, &config).or_else(|_| {
        let (_nb, guard) = tracing_appender::non_blocking(std::io::sink());
        Ok::<_, anyhow::Error>(guard)
    })?;
    let ctx = AppCtx::build(paths, config, log_guard).await?;
    Ok((ctx, dir))
}

async fn create_session_cookie(
    store: &RusqliteSessionStore,
    user_id: i64,
    role: Role,
) -> anyhow::Result<String> {
    let session_id = Id::default();
    let si = SessionIdentity {
        user_id: Some(user_id),
        role: role.as_str().to_string(),
    };
    let mut record = Record {
        id: session_id,
        data: Default::default(),
        expiry_date: OffsetDateTime::now_utc() + TimeDuration::days(1),
    };
    record
        .data
        .insert("identity".to_string(), serde_json::to_value(&si)?);
    store.create(&mut record).await?;
    Ok(format!("id={session_id}"))
}

/// Send a raw HTTP/1.1 POST over a real TCP connection (required so
/// `tower_governor`'s `PeerIpKeyExtractor` can resolve a peer IP for the
/// governed `auth_login`/`request_ad_restore` routes). Returns
/// (status_code, parsed JSON body — `Value::Null` if unparsable).
async fn raw_post(
    addr: std::net::SocketAddr,
    path: &str,
    body: &Value,
    cookie: Option<&str>,
) -> anyhow::Result<(u16, Value)> {
    let body_str = body.to_string();
    let mut stream = TcpStream::connect(addr).await?;
    let cookie_header = cookie
        .map(|c| format!("Cookie: {c}\r\n"))
        .unwrap_or_default();
    let request = format!(
        "POST {path} HTTP/1.1\r\n\
         Host: {addr}\r\n\
         Content-Type: application/json\r\n\
         {cookie_header}\
         Content-Length: {}\r\n\
         Connection: close\r\n\
         \r\n\
         {body_str}",
        body_str.len()
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

    // Body is whatever follows the blank line separating headers from body.
    let body_text = text.split("\r\n\r\n").nth(1).unwrap_or_default();
    let body_json: Value = serde_json::from_str(body_text).unwrap_or(Value::Null);

    eprintln!("RAW RESPONSE {path} status={status_code} body={body_text:?}");

    Ok((status_code, body_json))
}

/// Seed a soft-deleted AD-linked user row directly (mirrors `ad_register.rs`'s
/// `seed_blocked_ad_user(..., deleted: true)`), bypassing `create_user` (which
/// requires a password — AD users have `password_hash = NULL`).
async fn seed_soft_deleted_ad_user(ctx: &AppCtx, login: &str, full_name: &str) -> i64 {
    let now = SystemClock.unix_seconds();
    let login = login.to_string();
    let full_name = full_name.to_string();
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO users \
                 (login, full_name, password_hash, role, ad_user, is_active, \
                  deleted_at_utc, created_at_utc, updated_at_utc, version) \
                 VALUES (?1, ?2, NULL, 'employee', 1, 0, ?3, ?3, ?3, 1)",
                params![login, full_name, now],
            )
            .map_err(|e| AppError::Internal {
                source_chain: format!("seed_soft_deleted_ad_user: {e}"),
            })?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed soft-deleted AD user")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn blocked_user_restore_request_visible_to_admin_and_marks_pending_http(
) -> anyhow::Result<()> {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (ctx, _dir) = make_test_ctx().await?;
        let admin_identity = Identity::trusted_admin();

        // Enable AD (login()'s local->AD fallback only fires when enabled).
        ctx.auth.set_ad_enabled(true, &admin_identity).await?;

        let admin_dto = ctx
            .auth
            .create_user(
                UserNew {
                    login: "restore_admin_http".to_string(),
                    full_name: "Admin".to_string(),
                    password: "password123".to_string(),
                    role: "admin".to_string(),
                    email: None,
                },
                &admin_identity,
            )
            .await?;

        // (a) Seed a soft-deleted AD user (us200 mock fixture: Secret123).
        let blocked_user_id =
            seed_soft_deleted_ad_user(&ctx, "us200", "Петрова Анна Сергеевна").await;

        let session_store = RusqliteSessionStore::new(ctx.writer.clone(), ctx.readers.clone());
        let admin_cookie = create_session_cookie(&session_store, admin_dto.id, Role::Admin).await?;

        let router = build_router(&ctx, session_store);

        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;
        let shutdown = CancellationToken::new();
        let shutdown_clone = shutdown.clone();
        let server_handle = tokio::spawn(async move {
            axum::serve(
                listener,
                router.into_make_service_with_connect_info::<std::net::SocketAddr>(),
            )
            .with_graceful_shutdown(async move { shutdown_clone.cancelled().await })
            .await
            .expect("axum::serve");
        });
        tokio::time::sleep(Duration::from_millis(50)).await;

        // (b) Plain login by the blocked/soft-deleted user — read-only:
        // must report ACCESS_BLOCKED with pending == false, and must NOT
        // create any restore request row.
        let (status_login1, body_login1) = raw_post(
            addr,
            "/api/v1/auth_login",
            &serde_json::json!({
                "req": { "login": "us200", "password": "Secret123", "remember": false }
            }),
            None,
        )
        .await?;
        assert_eq!(
            status_login1, 403,
            "blocked plain login should map to a non-2xx AppError, body: {body_login1}"
        );
        assert_eq!(
            body_login1["code"], "ACCESS_BLOCKED",
            "expected ACCESS_BLOCKED code, got: {body_login1}"
        );
        assert_eq!(
            body_login1["details"]["pending"], false,
            "no restore request exists yet — pending must be false: {body_login1}"
        );

        let readers = ctx.readers.clone();
        let pre_count: i64 = {
            let readers = readers.clone();
            tokio::task::spawn_blocking(move || {
                let conn = readers.acquire();
                conn.query_row(
                    "SELECT COUNT(*) FROM requests WHERE requested_by_user_id = ?1",
                    params![blocked_user_id],
                    |r| r.get(0),
                )
                .expect("count requests pre-restore")
            })
            .await?
        };
        assert_eq!(
            pre_count, 0,
            "read-only blocked login must not create any request row"
        );

        // (c) Explicit restore request — the button the user actually clicks.
        let (status_restore, body_restore) = raw_post(
            addr,
            "/api/v1/request_ad_restore",
            &serde_json::json!({
                "req": { "login": "us200", "password": "Secret123" }
            }),
            None,
        )
        .await?;
        assert_eq!(
            status_restore, 200,
            "request_ad_restore should return 200, body: {body_restore}"
        );

        // (d) A restore request row now exists (status='open',
        // ad_subtype='restore') AND must be visible through the SAME
        // query/endpoint the ADMIN UI uses to list requests.
        let post_count: i64 = {
            let readers = readers.clone();
            tokio::task::spawn_blocking(move || {
                let conn = readers.acquire();
                conn.query_row(
                    "SELECT COUNT(*) FROM requests \
                     WHERE request_type = 'ad_register' AND ad_subtype = 'restore' \
                       AND requested_by_user_id = ?1 AND status = 'open' \
                       AND deleted_at_utc IS NULL",
                    params![blocked_user_id],
                    |r| r.get(0),
                )
                .expect("count restore requests post-restore")
            })
            .await?
        };
        assert_eq!(
            post_count, 1,
            "exactly one open restore request must exist after request_ad_restore"
        );

        let list_body = serde_json::json!({
            "filter": { "status": null, "requestType": null, "assignedToUserId": null, "requestedByUserId": null },
            "pagination": { "offset": 0, "limit": 50 }
        });
        let (status_list, body_list) =
            raw_post(addr, "/api/v1/requests_list", &list_body, Some(&admin_cookie)).await?;
        assert_eq!(
            status_list, 200,
            "admin requests_list should return 200, body: {body_list}"
        );
        let items = body_list["items"].as_array().expect("items array");
        let restore_request = items.iter().find(|r| {
            r["requestType"] == "ad_register"
                && r["adSubtype"] == "restore"
                && r["requestedByUserId"] == blocked_user_id
        });
        assert!(
            restore_request.is_some(),
            "the soft-deleted user's restore request MUST be visible to the admin \
             via /api/v1/requests_list — admin list body: {body_list}"
        );

        // (e) Plain login again — must now report pending == true.
        let (status_login2, body_login2) = raw_post(
            addr,
            "/api/v1/auth_login",
            &serde_json::json!({
                "req": { "login": "us200", "password": "Secret123", "remember": false }
            }),
            None,
        )
        .await?;
        assert_eq!(
            status_login2, 403,
            "second blocked login should still map to ACCESS_BLOCKED, body: {body_login2}"
        );
        assert_eq!(body_login2["code"], "ACCESS_BLOCKED");
        assert_eq!(
            body_login2["details"]["pending"], true,
            "after request_ad_restore, a subsequent blocked login MUST report \
             pending == true — got: {body_login2}"
        );

        shutdown.cancel();
        let _ = server_handle.await;
        ctx.shutdown.cancel();
        Ok::<(), anyhow::Error>(())
    })
    .await
    .expect("blocked_user_restore_request_visible_to_admin_and_marks_pending_http exceeded 30s budget")
}
