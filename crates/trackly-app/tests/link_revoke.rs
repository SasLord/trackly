//! Отзыв Link на лету (Phase 44-14, D-12/D-14): guard запросов и разрыв WS.
//! Имена вымышленные. Роутер собран настоящим `build_router`.

mod im_support;
use std::net::SocketAddr;
use std::time::Duration;

use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{header, Request, StatusCode};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use futures_util::StreamExt;
use im_support::{build_ctx, seed_user};
use rusqlite::params;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::Message as TMessage;
use tower::ServiceExt;
use tower_sessions::session::{Id, Record};
use tower_sessions::SessionStore;
use trackly_app::context::AppCtx;
use trackly_app::http::auth::SessionIdentity;
use trackly_app::http::build_router;
use trackly_app::http::link::{SESSION_DEVICE_KEY, SESSION_LOGIN_KEY};
use trackly_app::server::rusqlite_session_store::RusqliteSessionStore;
use trackly_app::services::link_service::{DeviceMeta, LoginMethod};
use trackly_infra::error_conversions::map_rusqlite;

const DEV_A: &str = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa";
const DEV_B: &str = "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb";

fn store(ctx: &AppCtx) -> RusqliteSessionStore {
    RusqliteSessionStore::new(ctx.writer.clone(), ctx.readers.clone())
}

/// Регистрирует устройство через сервис.
async fn enroll(ctx: &AppCtx, admin: i64, device: &str) {
    let issued = ctx
        .link
        .create_invite("trackly.example.test", 8443, [7u8; 32], 3600, admin)
        .await
        .unwrap();
    let decoded = link_proto::InviteCode::decode(&issued.code).unwrap();
    ctx.link
        .enroll(
            &URL_SAFE_NO_PAD.encode(decoded.secret),
            DeviceMeta {
                device_id: device.to_string(),
                hostname: "PC-EXAMPLE-01".into(),
                os: "windows".into(),
                arch: "x86_64".into(),
                client_version: "1.0.0".into(),
                ip: None,
            },
        )
        .await
        .unwrap();
}

/// Link-сессия: запись в хранилище сессий с identity + привязкой к устройству и входу.
async fn link_cookie(ctx: &AppCtx, user_id: i64, device: &str, login_id: i64) -> String {
    let id = Id::default();
    let si = SessionIdentity {
        user_id: Some(user_id),
        role: "employee".to_string(),
    };
    let mut record = Record {
        id,
        data: Default::default(),
        expiry_date: time::OffsetDateTime::now_utc() + time::Duration::days(1),
    };
    record
        .data
        .insert("identity".into(), serde_json::to_value(&si).unwrap());
    record
        .data
        .insert(SESSION_DEVICE_KEY.into(), serde_json::json!(device));
    record
        .data
        .insert(SESSION_LOGIN_KEY.into(), serde_json::json!(login_id));
    store(ctx).create(&mut record).await.expect("session");
    format!("id={id}")
}

/// (id пользователя, id входа, cookie) для пользователя на устройстве.
async fn link_user(ctx: &AppCtx, login: &str, full: &str, device: &str) -> (i64, i64, String) {
    let uid = seed_user(ctx, login, full, "employee").await;
    let issued = ctx
        .link
        .record_login(device, uid, LoginMethod::Password)
        .await
        .unwrap();
    let cookie = link_cookie(ctx, uid, device, issued.login_id).await;
    (uid, issued.login_id, cookie)
}

struct Resp {
    status: StatusCode,
    json: serde_json::Value,
}

async fn status_call(app: &axum::Router, cookie: &str) -> Resp {
    let mut req = Request::builder()
        .method("POST")
        .uri("/api/v1/auth_status")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::COOKIE, cookie)
        .body(Body::from("{}"))
        .unwrap();
    req.extensions_mut().insert(ConnectInfo::<SocketAddr>(
        "192.0.2.55:4000".parse().unwrap(),
    ));
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = axum::body::to_bytes(res.into_body(), 1 << 20)
        .await
        .unwrap();
    Resp {
        status,
        json: serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null),
    }
}

async fn sql(ctx: &AppCtx, q: &'static str, p: i64) {
    ctx.writer
        .execute(move |c| {
            c.execute(q, params![p]).map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .unwrap();
}

struct Fx {
    ctx: AppCtx,
    _dir: tempfile::TempDir,
    app: axum::Router,
    admin: i64,
}

async fn fx() -> Fx {
    let (ctx, dir) = build_ctx().await;
    let admin = seed_user(&ctx, "lr_adm", "Смирнов С.С.", "admin").await;
    let app = build_router(&ctx, store(&ctx));
    Fx {
        ctx,
        _dir: dir,
        app,
        admin,
    }
}

/// `auth_status` отдаёт пользователя только при живой сессии.
fn authed(r: &Resp) -> bool {
    r.status == StatusCode::OK && !r.json["user"].is_null()
}

// --- Test 1 (D-14) ---------------------------------------------------------

#[tokio::test]
async fn revoked_login_gets_401_on_next_request_and_session_is_flushed() {
    let f = fx().await;
    enroll(&f.ctx, f.admin, DEV_A).await;
    let (_, login_id, cookie) = link_user(&f.ctx, "petrov", "Петров П.П.", DEV_A).await;

    assert!(authed(&status_call(&f.app, &cookie).await));
    f.ctx.link.revoke_login(login_id, f.admin).await.unwrap();

    let r = status_call(&f.app, &cookie).await;
    assert_eq!(r.status, StatusCode::UNAUTHORIZED, "{}", r.json);
    assert_eq!(r.json["code"], "LINK_REVOKED");
    // Сессия сброшена: повторный запрос с той же cookie идёт без identity.
    let again = status_call(&f.app, &cookie).await;
    assert!(
        !authed(&again),
        "после сброса сессия не должна нести identity: {}",
        again.json
    );
}

// --- Test 3 ----------------------------------------------------------------

#[tokio::test]
async fn browser_session_is_untouched_by_guard() {
    let f = fx().await;
    let uid = seed_user(&f.ctx, "sidorov", "Сидоров С.С.", "employee").await;
    let cookie = im_support::cookie(&f.ctx, uid, trackly_core::auth::Role::Employee).await;
    // Ломаем link-таблицы: браузерная сессия не должна к ним обращаться.
    f.ctx
        .writer
        .execute(|c| {
            c.execute_batch("ALTER TABLE link_logins RENAME TO link_logins_x")
                .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .unwrap();
    let r = status_call(&f.app, &cookie).await;
    assert!(authed(&r), "{}", r.json);
}

// --- Test 4 ----------------------------------------------------------------

#[tokio::test]
async fn inactive_user_link_session_gets_link_revoked() {
    let f = fx().await;
    enroll(&f.ctx, f.admin, DEV_A).await;
    let (uid, _, cookie) = link_user(&f.ctx, "kuznetsov", "Кузнецов К.К.", DEV_A).await;
    assert!(authed(&status_call(&f.app, &cookie).await));
    // Прямая запись в БД, минуя хуки: guard проверяет is_active независимо (T-44-52).
    sql(&f.ctx, "UPDATE users SET is_active = 0 WHERE id = ?1", uid).await;
    let r = status_call(&f.app, &cookie).await;
    assert_eq!(r.status, StatusCode::UNAUTHORIZED);
    assert_eq!(r.json["code"], "LINK_REVOKED");
}

// --- Test 5 ----------------------------------------------------------------

#[tokio::test]
async fn db_read_error_in_guard_does_not_log_out() {
    let f = fx().await;
    enroll(&f.ctx, f.admin, DEV_A).await;
    let (_, _, cookie) = link_user(&f.ctx, "orlov", "Орлов О.О.", DEV_A).await;
    f.ctx
        .writer
        .execute(|c| {
            c.execute_batch("ALTER TABLE link_logins RENAME TO link_logins_x")
                .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .unwrap();
    // Чтение состояния входа падает -> запрос пропущен, сессия цела.
    let r = status_call(&f.app, &cookie).await;
    assert!(
        authed(&r),
        "сбой чтения не должен разлогинивать: {}",
        r.json
    );
    f.ctx
        .writer
        .execute(|c| {
            c.execute_batch("ALTER TABLE link_logins_x RENAME TO link_logins")
                .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .unwrap();
    let r = status_call(&f.app, &cookie).await;
    assert!(authed(&r), "сессия должна пережить сбой: {}", r.json);
}

// --- WS (Tests 2, 6) -------------------------------------------------------

type Ws =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

async fn serve(ctx: &AppCtx) -> SocketAddr {
    let app = build_router(ctx, store(ctx));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let _ = axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await;
    });
    addr
}

async fn connect(addr: SocketAddr, cookie: &str) -> Ws {
    let mut req = format!("ws://{addr}/api/v1/ws")
        .into_client_request()
        .unwrap();
    req.headers_mut().insert("cookie", cookie.parse().unwrap());
    let (mut ws, _) = tokio_tungstenite::connect_async(req)
        .await
        .expect("ws connect");
    let m = tokio::time::timeout(Duration::from_secs(5), ws.next())
        .await
        .expect("hello")
        .expect("hello")
        .expect("hello");
    assert!(matches!(m, TMessage::Text(_)), "ждали hello");
    ws
}

/// Ждёт Close-кадр; возвращает его код. Прочие кадры пропускаются.
async fn close_code(ws: &mut Ws) -> Option<u16> {
    loop {
        match tokio::time::timeout(Duration::from_secs(5), ws.next()).await {
            Ok(Some(Ok(TMessage::Close(Some(f))))) => return Some(u16::from(f.code)),
            Ok(Some(Ok(TMessage::Close(None)))) => return None,
            Ok(Some(Ok(_))) => continue,
            other => panic!("ждали Close-кадр, получили {other:?}"),
        }
    }
}

/// Соединение живо: за короткое окно Close не приходит.
async fn stays_open(ws: &mut Ws) -> bool {
    !matches!(
        tokio::time::timeout(Duration::from_millis(400), ws.next()).await,
        Ok(Some(Ok(TMessage::Close(_)))) | Ok(None)
    )
}

#[tokio::test]
async fn ws_of_revoked_device_closes_with_4401_without_new_request() {
    let f = fx().await;
    enroll(&f.ctx, f.admin, DEV_A).await;
    let (_, login_id, cookie) = link_user(&f.ctx, "volkov", "Волков В.В.", DEV_A).await;
    let addr = serve(&f.ctx).await;
    let mut ws = connect(addr, &cookie).await;

    f.ctx.link.revoke_login(login_id, f.admin).await.unwrap();
    assert_eq!(
        close_code(&mut ws).await,
        Some(link_proto::dto::REVOKED_CLOSE_CODE)
    );
}

#[tokio::test]
async fn ws_of_another_device_survives_foreign_revoke() {
    let f = fx().await;
    enroll(&f.ctx, f.admin, DEV_A).await;
    enroll(&f.ctx, f.admin, DEV_B).await;
    // Один и тот же пользователь на двух устройствах.
    let uid = seed_user(&f.ctx, "morozov", "Морозов М.М.", "employee").await;
    let la = f
        .ctx
        .link
        .record_login(DEV_A, uid, LoginMethod::Password)
        .await
        .unwrap();
    let lb = f
        .ctx
        .link
        .record_login(DEV_B, uid, LoginMethod::Password)
        .await
        .unwrap();
    let ca = link_cookie(&f.ctx, uid, DEV_A, la.login_id).await;
    let cb = link_cookie(&f.ctx, uid, DEV_B, lb.login_id).await;
    let addr = serve(&f.ctx).await;
    let mut wa = connect(addr, &ca).await;
    let mut wb = connect(addr, &cb).await;

    f.ctx.link.revoke_login(la.login_id, f.admin).await.unwrap();
    assert_eq!(
        close_code(&mut wa).await,
        Some(link_proto::dto::REVOKED_CLOSE_CODE)
    );
    assert!(stays_open(&mut wb).await, "WS чужого устройства не рвётся");
}
