//! Запрет устройства на всех путях входа Link (Phase 44-14, D-13). Имена вымышленные.
//! Устройство запрещается настоящим `revoke_login` (как это делает админ), а не
//! прямой записью в БД; проверка идёт через `build_router`.

mod im_support;
use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{header, Request, StatusCode};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use im_support::{build_ctx, seed_user};
use tower::ServiceExt;
use trackly_app::context::AppCtx;
use trackly_app::dto::auth::UserNew;
use trackly_app::http::build_router;
use trackly_app::server::rusqlite_session_store::RusqliteSessionStore;
use trackly_app::services::link_service::DeviceMeta;
use trackly_core::auth::Identity;

const PASSWORD: &str = "Пароль-Тест-123";
const DEV: &str = "cccccccc-cccc-4ccc-8ccc-cccccccccccc";

async fn call(
    app: &axum::Router,
    method: &str,
    uri: &str,
    device: (&str, &str),
    extra: Option<(&str, &str)>,
    body: Option<serde_json::Value>,
) -> (StatusCode, serde_json::Value) {
    let mut b = Request::builder()
        .method(method)
        .uri(uri)
        .header("x-link-device", device.0)
        .header("x-link-secret", device.1);
    if body.is_some() {
        b = b.header(header::CONTENT_TYPE, "application/json");
    }
    if let Some((k, v)) = extra {
        b = b.header(k, v);
    }
    let mut req = b
        .body(Body::from(body.map(|v| v.to_string()).unwrap_or_default()))
        .unwrap();
    req.extensions_mut()
        .insert(ConnectInfo::<std::net::SocketAddr>(
            "192.0.2.77:4000".parse().unwrap(),
        ));
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = axum::body::to_bytes(res.into_body(), 1 << 20)
        .await
        .unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null),
    )
}

struct Fx {
    ctx: AppCtx,
    _dir: tempfile::TempDir,
    app: axum::Router,
    admin: i64,
    secret: String,
}

async fn fx() -> Fx {
    let (ctx, dir) = build_ctx().await;
    let admin = seed_user(&ctx, "rd_adm", "Смирнов С.С.", "admin").await;
    ctx.auth
        .create_user(
            UserNew {
                login: "fedorov".into(),
                full_name: "Фёдоров Ф.Ф.".into(),
                password: PASSWORD.into(),
                role: "employee".into(),
                email: None,
            },
            &Identity::trusted_admin(),
        )
        .await
        .unwrap();
    let issued = ctx
        .link
        .create_invite("trackly.example.test", 8443, [7u8; 32], 3600, admin)
        .await
        .unwrap();
    let decoded = link_proto::InviteCode::decode(&issued.code).unwrap();
    let secret = ctx
        .link
        .enroll(
            &URL_SAFE_NO_PAD.encode(decoded.secret),
            DeviceMeta {
                device_id: DEV.into(),
                hostname: "PC-EXAMPLE-02".into(),
                os: "windows".into(),
                arch: "x86_64".into(),
                client_version: "1.0.0".into(),
                ip: None,
            },
        )
        .await
        .unwrap();
    let app = build_router(
        &ctx,
        RusqliteSessionStore::new(ctx.writer.clone(), ctx.readers.clone()),
    );
    Fx {
        ctx,
        _dir: dir,
        app,
        admin,
        secret,
    }
}

fn login_body() -> serde_json::Value {
    serde_json::json!({ "login": "fedorov", "password": PASSWORD })
}

/// Вход паролем; возвращает токен устройства и id входа.
async fn login(f: &Fx) -> (String, i64) {
    let (st, json) = call(
        &f.app,
        "POST",
        "/api/v1/link_auth_login",
        (DEV, &f.secret),
        None,
        Some(login_body()),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{json}");
    let token = json["deviceToken"].as_str().unwrap().to_string();
    let id = f.ctx.link.list_active_logins().await.unwrap()[0].login_id;
    (token, id)
}

/// Test 4 (D-13): запрещённое устройство не входит ни паролем, ни SSO, ни по токену.
#[tokio::test]
async fn banned_device_cannot_login_by_password_sso_or_token() {
    let f = fx().await;
    let (token, login_id) = login(&f).await;
    f.ctx.link.revoke_login(login_id, f.admin).await.unwrap();

    let (st, json) = call(
        &f.app,
        "POST",
        "/api/v1/link_auth_login",
        (DEV, &f.secret),
        None,
        Some(login_body()),
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN, "{json}");
    assert_eq!(json["code"], "LINK_DEVICE_BANNED");

    let (st, json) = call(
        &f.app,
        "GET",
        "/api/v1/link_auth_sso",
        (DEV, &f.secret),
        Some(("authorization", "Negotiate Zm9vYmFy")),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN, "{json}");
    assert_eq!(json["code"], "LINK_DEVICE_BANNED");

    let (st, json) = call(
        &f.app,
        "POST",
        "/api/v1/link_resume",
        (DEV, &f.secret),
        None,
        Some(serde_json::json!({ "deviceToken": token })),
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN, "{json}");
    assert_eq!(json["code"], "LINK_DEVICE_BANNED");
}

/// Test 5: allow_device снимает запрет для нового входа; старый токен мёртв.
#[tokio::test]
async fn allow_device_restores_password_login_but_not_old_token() {
    let f = fx().await;
    let (old_token, login_id) = login(&f).await;
    f.ctx.link.revoke_login(login_id, f.admin).await.unwrap();
    f.ctx.link.allow_device(DEV).await.unwrap();

    let (st, json) = call(
        &f.app,
        "POST",
        "/api/v1/link_auth_login",
        (DEV, &f.secret),
        None,
        Some(login_body()),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{json}");
    assert_ne!(json["deviceToken"].as_str().unwrap(), old_token);

    let (st, json) = call(
        &f.app,
        "POST",
        "/api/v1/link_resume",
        (DEV, &f.secret),
        None,
        Some(serde_json::json!({ "deviceToken": old_token })),
    )
    .await;
    assert_eq!(st, StatusCode::UNAUTHORIZED, "{json}");
    assert_eq!(json["code"], "LINK_REVOKED");
}
