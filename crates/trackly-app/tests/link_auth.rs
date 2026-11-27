//! Публичные маршруты входа Link (Phase 44-11): link_auth_login, link_auth_sso,
//! link_resume, link_logout. Имена вымышленные. Тесты идут через настоящий
//! `build_router` и проверяют тела JSON, а не только статус (INT-26).

mod im_support;
use axum::body::Body;
use axum::extract::{ConnectInfo, State};
use axum::http::{header, HeaderMap, Request, StatusCode};
use axum::response::Response;
use im_support::{build_ctx, seed_user};
use rusqlite::params;
use sha2::{Digest, Sha256};
use tower::ServiceExt;
use tower_sessions::session::Id;
use tower_sessions::{Session, SessionManagerLayer, SessionStore};
use trackly_app::context::AppCtx;
use trackly_app::dto::auth::UserNew;
use trackly_app::http::build_router;
use trackly_app::http::link::{complete_sso_login, SESSION_DEVICE_KEY, SESSION_LOGIN_KEY};
use trackly_app::server::rusqlite_session_store::RusqliteSessionStore;
use trackly_app::services::link_service::DeviceMeta;
use trackly_core::auth::Identity;
use trackly_infra::error_conversions::map_rusqlite;

const PASSWORD: &str = "Пароль-Тест-123";

fn store(ctx: &AppCtx) -> RusqliteSessionStore {
    RusqliteSessionStore::new(ctx.writer.clone(), ctx.readers.clone())
}

fn router(ctx: &AppCtx) -> axum::Router {
    build_router(ctx, store(ctx))
}

struct Resp {
    status: StatusCode,
    headers: HeaderMap,
    json: serde_json::Value,
}

impl Resp {
    fn cookie(&self) -> Option<String> {
        self.headers
            .get(header::SET_COOKIE)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.split(';').next())
            .map(str::to_string)
    }
}

async fn call(
    app: &axum::Router,
    method: &str,
    uri: &str,
    device: Option<(&str, &str)>,
    cookie: Option<&str>,
    extra: Option<(&str, &str)>,
    body: Option<serde_json::Value>,
) -> Resp {
    let mut b = Request::builder().method(method).uri(uri);
    if body.is_some() {
        b = b.header(header::CONTENT_TYPE, "application/json");
    }
    if let Some((d, s)) = device {
        b = b.header("x-link-device", d).header("x-link-secret", s);
    }
    if let Some(c) = cookie {
        b = b.header(header::COOKIE, c);
    }
    if let Some((k, v)) = extra {
        b = b.header(k, v);
    }
    let mut req = b
        .body(Body::from(body.map(|v| v.to_string()).unwrap_or_default()))
        .unwrap();
    req.extensions_mut()
        .insert(ConnectInfo::<std::net::SocketAddr>(
            "192.0.2.55:4000".parse().unwrap(),
        ));
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let headers = res.headers().clone();
    let bytes = axum::body::to_bytes(res.into_body(), 1 << 20)
        .await
        .unwrap();
    let json = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
    Resp {
        status,
        headers,
        json,
    }
}

async fn make_user(ctx: &AppCtx, login: &str, name: &str) -> i64 {
    ctx.auth
        .create_user(
            UserNew {
                login: login.to_string(),
                full_name: name.to_string(),
                password: PASSWORD.to_string(),
                role: "employee".to_string(),
                email: None,
            },
            &Identity::trusted_admin(),
        )
        .await
        .unwrap()
        .id
}

/// Регистрирует устройство напрямую через сервис; возвращает его секрет.
async fn enrolled(ctx: &AppCtx, admin: i64, device: &str) -> String {
    let issued = ctx
        .link
        .create_invite("trackly.example.test", 8443, [7u8; 32], 3600, admin)
        .await
        .unwrap();
    let decoded = link_proto::InviteCode::decode(&issued.code).unwrap();
    let secret = {
        use base64::engine::general_purpose::URL_SAFE_NO_PAD;
        use base64::Engine;
        URL_SAFE_NO_PAD.encode(decoded.secret)
    };
    ctx.link
        .enroll(
            &secret,
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
        .unwrap()
}

async fn sql(ctx: &AppCtx, q: &'static str, p: String) {
    ctx.writer
        .execute(move |c| {
            c.execute(q, params![p]).map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .unwrap();
}

fn login_body(login: &str, password: &str) -> serde_json::Value {
    serde_json::json!({ "login": login, "password": password })
}

struct Fixture {
    ctx: AppCtx,
    _dir: tempfile::TempDir,
    app: axum::Router,
    admin: i64,
}

async fn fixture() -> Fixture {
    let (ctx, dir) = build_ctx().await;
    let admin = seed_user(&ctx, "la_adm", "Смирнов С.С.", "admin").await;
    let app = router(&ctx);
    Fixture {
        ctx,
        _dir: dir,
        app,
        admin,
    }
}

// --- Test 1 + 9 ------------------------------------------------------------

#[tokio::test]
async fn link_auth_login_issues_token_and_binds_session() {
    let f = fixture().await;
    make_user(&f.ctx, "ivanov", "Иванов И.И.").await;
    let dev = "11111111-1111-4111-8111-111111111111";
    let sec = enrolled(&f.ctx, f.admin, dev).await;

    // Предсуществующая (подложенная) сессия: после входа она обязана исчезнуть (T-05-SF).
    let old_cookie = im_support::cookie(&f.ctx, f.admin, trackly_core::auth::Role::Admin).await;
    let r = call(
        &f.app,
        "POST",
        "/api/v1/link_auth_login",
        Some((dev, &sec)),
        Some(&old_cookie),
        None,
        Some(login_body("ivanov", PASSWORD)),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    assert_eq!(r.json["method"], "password");
    assert_eq!(r.json["user"]["login"], "ivanov");
    assert_eq!(r.json["user"]["displayName"], "Иванов И.И.");
    let token = r.json["deviceToken"]
        .as_str()
        .expect("deviceToken")
        .to_string();
    assert!(token.len() >= 43, "токен 256 бит в base64url");
    let cookie = r.cookie().expect("Set-Cookie сессии");
    assert_ne!(cookie, old_cookie, "идентификатор сессии сменился");

    // В БД — только хэш токена (SHA-256 строки токена), сам токен не хранится.
    let want: Vec<u8> = Sha256::digest(token.as_bytes()).to_vec();
    let want2 = want.clone();
    let tok2 = token.clone();
    let (has_hash, has_plain): (i64, i64) = f
        .ctx
        .writer
        .execute(move |c| {
            let a: i64 = c
                .query_row(
                    "SELECT COUNT(*) FROM link_logins WHERE token_hash = ?1",
                    [&want2],
                    |r| r.get(0),
                )
                .map_err(map_rusqlite)?;
            let b: i64 = c
                .query_row(
                    "SELECT COUNT(*) FROM link_logins WHERE CAST(token_hash AS TEXT) = ?1",
                    [&tok2],
                    |r| r.get(0),
                )
                .map_err(map_rusqlite)?;
            Ok((a, b))
        })
        .await
        .unwrap();
    assert_eq!((has_hash, has_plain), (1, 0));

    // Сессия: identity + link_device_id + link_login_id; старая сессия уничтожена.
    let st = store(&f.ctx);
    let new_id: Id = cookie.trim_start_matches("id=").parse().unwrap();
    let rec = st.load(&new_id).await.unwrap().expect("новая сессия");
    assert!(rec.data.contains_key("identity"));
    assert_eq!(rec.data[SESSION_DEVICE_KEY], serde_json::json!(dev));
    assert!(rec.data[SESSION_LOGIN_KEY].is_i64() || rec.data[SESSION_LOGIN_KEY].is_u64());
    let old_id: Id = old_cookie.trim_start_matches("id=").parse().unwrap();
    assert!(st.load(&old_id).await.unwrap().is_none(), "flush до insert");
}

// --- Test 2 + 3 + 4 --------------------------------------------------------

#[tokio::test]
async fn link_auth_login_checks_device_before_password() {
    let f = fixture().await;
    make_user(&f.ctx, "petrov", "Петров П.П.").await;
    let dev = "22222222-2222-4222-8222-222222222222";
    let sec = enrolled(&f.ctx, f.admin, dev).await;
    let url = "/api/v1/link_auth_login";

    // Запрещённое устройство: даже с НЕВЕРНЫМ паролем ответ — BANNED, а не BAD_CREDENTIALS.
    sql(
        &f.ctx,
        "UPDATE link_devices SET revoked_at_utc = 1700000000 WHERE device_id = ?1",
        dev.to_string(),
    )
    .await;
    for pw in [PASSWORD, "неверный"] {
        let r = call(
            &f.app,
            "POST",
            url,
            Some((dev, &sec)),
            None,
            None,
            Some(login_body("petrov", pw)),
        )
        .await;
        assert_eq!(r.status, StatusCode::FORBIDDEN);
        assert_eq!(r.json["code"], "LINK_DEVICE_BANNED");
        assert!(r.cookie().is_none());
    }
    sql(
        &f.ctx,
        "UPDATE link_devices SET revoked_at_utc = NULL WHERE device_id = ?1",
        dev.to_string(),
    )
    .await;

    // Неизвестное устройство и неверный секрет — один и тот же ответ.
    let unknown = call(
        &f.app,
        "POST",
        url,
        Some(("33333333-3333-4333-8333-333333333333", &sec)),
        None,
        None,
        Some(login_body("petrov", PASSWORD)),
    )
    .await;
    let badsec = call(
        &f.app,
        "POST",
        url,
        Some((dev, "чужой-секрет")),
        None,
        None,
        Some(login_body("petrov", PASSWORD)),
    )
    .await;
    let nohdr = call(
        &f.app,
        "POST",
        url,
        None,
        None,
        None,
        Some(login_body("petrov", PASSWORD)),
    )
    .await;
    for r in [&unknown, &badsec, &nohdr] {
        assert_eq!(r.status, StatusCode::UNAUTHORIZED);
        assert_eq!(r.json["code"], "LINK_DEVICE_UNKNOWN");
    }
    assert_eq!(unknown.json, badsec.json);

    // Неверный пароль — единое сообщение (новый router: корзина rate limit пуста).
    let app = router(&f.ctx);
    let bad = call(
        &app,
        "POST",
        url,
        Some((dev, &sec)),
        None,
        None,
        Some(login_body("petrov", "неверный")),
    )
    .await;
    assert_eq!(bad.status, StatusCode::UNAUTHORIZED);
    assert_eq!(bad.json["code"], "LINK_BAD_CREDENTIALS");
    let nouser = call(
        &app,
        "POST",
        url,
        Some((dev, &sec)),
        None,
        None,
        Some(login_body("нет-такого", "x")),
    )
    .await;
    assert_eq!(nouser.status, StatusCode::UNAUTHORIZED);
    assert_eq!(nouser.json, bad.json, "логин не раскрывается");
}

// --- Test 5 + 8 ------------------------------------------------------------

#[tokio::test]
async fn link_resume_and_logout_lifecycle() {
    let f = fixture().await;
    let uid = make_user(&f.ctx, "sidorov", "Сидоров С.С.").await;
    let dev = "44444444-4444-4444-8444-444444444444";
    let sec = enrolled(&f.ctx, f.admin, dev).await;
    let login = call(
        &f.app,
        "POST",
        "/api/v1/link_auth_login",
        Some((dev, &sec)),
        None,
        None,
        Some(login_body("sidorov", PASSWORD)),
    )
    .await;
    assert_eq!(login.status, StatusCode::OK, "{}", login.json);
    let token = login.json["deviceToken"].as_str().unwrap().to_string();
    let cookie = login.cookie().unwrap();
    let resume = |t: String| serde_json::json!({ "deviceToken": t });

    // resume с верным токеном: 200 + новая сессия, без нового токена.
    let ok = call(
        &f.app,
        "POST",
        "/api/v1/link_resume",
        Some((dev, &sec)),
        None,
        None,
        Some(resume(token.clone())),
    )
    .await;
    assert_eq!(ok.status, StatusCode::OK, "{}", ok.json);
    assert_eq!(ok.json["user"]["login"], "sidorov");
    assert!(ok.json.get("deviceToken").is_none());
    assert!(ok.cookie().is_some());

    // Токен другого устройства не годится.
    let dev2 = "55555555-5555-4555-8555-555555555555";
    let sec2 = enrolled(&f.ctx, f.admin, dev2).await;
    let other = call(
        &f.app,
        "POST",
        "/api/v1/link_resume",
        Some((dev2, &sec2)),
        None,
        None,
        Some(resume(token.clone())),
    )
    .await;
    assert_eq!(other.status, StatusCode::UNAUTHORIZED);

    // Выход: токен отзывается, устройство НЕ банится, сессия сброшена.
    let out = call(
        &f.app,
        "POST",
        "/api/v1/link_logout",
        None,
        Some(&cookie),
        None,
        None,
    )
    .await;
    assert_eq!(out.status, StatusCode::OK, "{}", out.json);
    assert_eq!(out.json["ok"], true);
    let revoked = call(
        &f.app,
        "POST",
        "/api/v1/link_resume",
        Some((dev, &sec)),
        None,
        None,
        Some(resume(token.clone())),
    )
    .await;
    assert_eq!(revoked.status, StatusCode::UNAUTHORIZED);
    assert_eq!(revoked.json["code"], "LINK_REVOKED");
    f.ctx
        .link
        .verify_device(dev, &sec)
        .await
        .expect("устройство не забанено");
    let again = call(
        &f.app,
        "POST",
        "/api/v1/link_logout",
        None,
        Some(&cookie),
        None,
        None,
    )
    .await;
    assert_eq!(again.status, StatusCode::UNAUTHORIZED);
    assert_eq!(again.json["code"], "LINK_NO_SESSION");

    // Неактивный пользователь: новый вход, затем блокировка.
    let login = call(
        &f.app,
        "POST",
        "/api/v1/link_auth_login",
        Some((dev, &sec)),
        None,
        None,
        Some(login_body("sidorov", PASSWORD)),
    )
    .await;
    let token = login.json["deviceToken"].as_str().unwrap().to_string();
    sql(
        &f.ctx,
        "UPDATE users SET is_active = 0 WHERE id = ?1",
        uid.to_string(),
    )
    .await;
    let blocked = call(
        &f.app,
        "POST",
        "/api/v1/link_resume",
        Some((dev, &sec)),
        None,
        None,
        Some(resume(token)),
    )
    .await;
    assert_eq!(blocked.status, StatusCode::FORBIDDEN);
    assert_eq!(blocked.json["code"], "LINK_USER_BLOCKED");

    // Запрещённое устройство отказывает и на resume.
    sql(
        &f.ctx,
        "UPDATE link_devices SET revoked_at_utc = 1700000000 WHERE device_id = ?1",
        dev.to_string(),
    )
    .await;
    let banned = call(
        &f.app,
        "POST",
        "/api/v1/link_resume",
        Some((dev, &sec)),
        None,
        None,
        Some(resume("x".into())),
    )
    .await;
    assert_eq!(banned.status, StatusCode::FORBIDDEN);
    assert_eq!(banned.json["code"], "LINK_DEVICE_BANNED");
}

#[tokio::test]
async fn link_logout_without_link_session_is_401_json() {
    let f = fixture().await;
    let r = call(
        &f.app,
        "POST",
        "/api/v1/link_logout",
        None,
        None,
        None,
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::UNAUTHORIZED);
    assert_eq!(r.json["code"], "LINK_NO_SESSION");
}

// --- Test 6 (D-09) ---------------------------------------------------------

async fn sso_fixture(enable: bool) -> (Fixture, String, String) {
    let dir = tempfile::TempDir::new().unwrap();
    let paths = trackly_infra::Paths::resolve_for_exe_dir(dir.path().to_path_buf()).unwrap();
    let mut config = trackly_infra::AppConfig::default();
    let keytab = dir.path().join("fake.keytab");
    std::fs::write(&keytab, b"not-a-keytab").unwrap();
    config.ad.spn = "HTTP/trackly.example.test".to_string();
    config.ad.keytab_path = keytab.to_string_lossy().to_string();
    let (_nb, guard) = tracing_appender::non_blocking(std::io::sink());
    let ctx = AppCtx::build(paths, config, guard).await.unwrap();
    if enable {
        ctx.auth
            .set_ad_sso_enabled(true, &Identity::trusted_admin())
            .await
            .unwrap();
    }
    let admin = seed_user(&ctx, "la_adm", "Смирнов С.С.", "admin").await;
    let dev = "66666666-6666-4666-8666-666666666666".to_string();
    let sec = enrolled(&ctx, admin, &dev).await;
    let app = router(&ctx);
    (
        Fixture {
            ctx,
            _dir: dir,
            app,
            admin,
        },
        dev,
        sec,
    )
}

#[tokio::test]
async fn link_auth_sso_distinguishes_disabled_challenge_denied() {
    let url = "/api/v1/link_auth_sso";

    // Выключен → 503 LINK_SSO_DISABLED.
    let (f, dev, sec) = sso_fixture(false).await;
    let r = call(&f.app, "GET", url, Some((&dev, &sec)), None, None, None).await;
    assert_eq!(r.status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(r.json["code"], "LINK_SSO_DISABLED");

    // Включён, без токена → 401 + Negotiate.
    let (f, dev, sec) = sso_fixture(true).await;
    let r = call(&f.app, "GET", url, Some((&dev, &sec)), None, None, None).await;
    assert_eq!(r.status, StatusCode::UNAUTHORIZED);
    assert_eq!(r.headers[header::WWW_AUTHENTICATE], "Negotiate");
    assert_eq!(r.json["code"], "LINK_SSO_CHALLENGE");

    // С токеном, который не проходит проверку → 401 LINK_SSO_DENIED без деталей.
    let r = call(
        &f.app,
        "GET",
        url,
        Some((&dev, &sec)),
        None,
        Some(("authorization", "Negotiate Zm9vYmFy")),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::UNAUTHORIZED);
    assert_eq!(r.json["code"], "LINK_SSO_DENIED");
    assert!(!r.json.to_string().to_lowercase().contains("keytab"));

    // Устройство проверяется до токена Kerberos: запрещённое — BANNED, а не challenge.
    sql(
        &f.ctx,
        "UPDATE link_devices SET revoked_at_utc = 1700000000 WHERE device_id = ?1",
        dev.clone(),
    )
    .await;
    let r = call(&f.app, "GET", url, Some((&dev, &sec)), None, None, None).await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);
    assert_eq!(r.json["code"], "LINK_DEVICE_BANNED");
    let r = call(&f.app, "GET", url, None, None, None, None).await;
    assert_eq!(r.status, StatusCode::UNAUTHORIZED);
    assert_eq!(r.json["code"], "LINK_DEVICE_UNKNOWN");
}

// --- Test 7: завершение SSO (билет Kerberos без настоящего KDC) -------------

async fn sso_probe(State(ctx): State<AppCtx>, session: Session, headers: HeaderMap) -> Response {
    let user = headers["x-test-user"].to_str().unwrap().to_string();
    let dev = headers["x-test-device"].to_str().unwrap().to_string();
    complete_sso_login(&ctx, &session, &dev, &user, &[]).await
}

#[tokio::test]
async fn link_sso_completion_maps_account_and_binds_session() {
    let (f, dev, _sec) = sso_fixture(true).await;
    f.ctx
        .auth
        .set_ad_enabled(true, &Identity::trusted_admin())
        .await
        .unwrap();
    seed_user(&f.ctx, "us100", "Иванов И.И.", "employee").await;
    let app = axum::Router::new()
        .route("/probe", axum::routing::get(sso_probe))
        .with_state(f.ctx.clone())
        .layer(SessionManagerLayer::new(store(&f.ctx)));

    let mut req = Request::builder()
        .uri("/probe")
        .header("x-test-user", "us100")
        .header("x-test-device", &dev)
        .body(Body::empty())
        .unwrap();
    req.extensions_mut()
        .insert(ConnectInfo::<std::net::SocketAddr>(
            "192.0.2.55:4000".parse().unwrap(),
        ));
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let cookie = res.headers()[header::SET_COOKIE]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_string();
    let bytes = axum::body::to_bytes(res.into_body(), 1 << 20)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["method"], "sso");
    assert!(json.get("deviceToken").is_none(), "у SSO токена нет");
    assert_eq!(json["user"]["login"], "us100");

    let id: Id = cookie.trim_start_matches("id=").parse().unwrap();
    let rec = store(&f.ctx).load(&id).await.unwrap().unwrap();
    assert_eq!(rec.data[SESSION_DEVICE_KEY], serde_json::json!(dev));
    assert!(rec.data.contains_key(SESSION_LOGIN_KEY));
    assert!(rec.data.contains_key("identity"));

    // Неизвестная учётка при выключенном автоприёме → отказ без сессии.
    let mut req = Request::builder()
        .uri("/probe")
        .header("x-test-user", "us999")
        .header("x-test-device", &dev)
        .body(Body::empty())
        .unwrap();
    req.extensions_mut()
        .insert(ConnectInfo::<std::net::SocketAddr>(
            "192.0.2.55:4000".parse().unwrap(),
        ));
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    let bytes = axum::body::to_bytes(res.into_body(), 1 << 20)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["code"], "LINK_USER_BLOCKED");

    // AD вход выключен → учётка не сопоставляется → LINK_SSO_DENIED.
    f.ctx
        .auth
        .set_ad_enabled(false, &Identity::trusted_admin())
        .await
        .unwrap();
    let mut req = Request::builder()
        .uri("/probe")
        .header("x-test-user", "us100")
        .header("x-test-device", &dev)
        .body(Body::empty())
        .unwrap();
    req.extensions_mut()
        .insert(ConnectInfo::<std::net::SocketAddr>(
            "192.0.2.55:4000".parse().unwrap(),
        ));
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    let bytes = axum::body::to_bytes(res.into_body(), 1 << 20)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["code"], "LINK_SSO_DENIED");
    let _ = f.admin;
}

// --- Test 10 ---------------------------------------------------------------

#[tokio::test]
async fn link_auth_login_rate_limited_after_burst() {
    let f = fixture().await;
    let mut statuses = Vec::new();
    for _ in 0..6 {
        let r = call(
            &f.app,
            "POST",
            "/api/v1/link_auth_login",
            None,
            None,
            None,
            Some(login_body("x", "y")),
        )
        .await;
        statuses.push(r.status);
    }
    assert!(
        statuses[..5].iter().all(|s| *s == StatusCode::UNAUTHORIZED),
        "{statuses:?}"
    );
    assert_eq!(statuses[5], StatusCode::TOO_MANY_REQUESTS, "{statuses:?}");
}
