//! Публичный маршрут link_enroll (Phase 44-11, D-03/D-04). Имена вымышленные.
//! Тесты идут через настоящий `build_router` и проверяют реальные JSON-ответы:
//! неизвестный `/api/` путь не должен маскироваться под SPA (INT-26).

mod im_support;
use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{header, Request, StatusCode};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use im_support::{build_ctx, seed_user};
use rusqlite::params;
use tower::ServiceExt;
use trackly_app::context::AppCtx;
use trackly_app::http::build_router;
use trackly_app::server::rusqlite_session_store::RusqliteSessionStore;
use trackly_infra::error_conversions::map_rusqlite;

const DEVICE: &str = "6f1d3c7e-8a52-4b19-9c0e-1a2b3c4d5e6f";

fn router(ctx: &AppCtx) -> axum::Router {
    build_router(
        ctx,
        RusqliteSessionStore::new(ctx.writer.clone(), ctx.readers.clone()),
    )
}

async fn send(
    app: &axum::Router,
    method: &str,
    uri: &str,
    body: Option<serde_json::Value>,
) -> (StatusCode, axum::http::HeaderMap, serde_json::Value) {
    let mut req = Request::builder().method(method).uri(uri);
    if body.is_some() {
        req = req.header(header::CONTENT_TYPE, "application/json");
    }
    let mut req = req
        .body(Body::from(body.map(|b| b.to_string()).unwrap_or_default()))
        .unwrap();
    req.extensions_mut()
        .insert(ConnectInfo::<std::net::SocketAddr>(
            "192.0.2.77:5555".parse().unwrap(),
        ));
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let headers = res.headers().clone();
    let bytes = axum::body::to_bytes(res.into_body(), 1 << 20)
        .await
        .unwrap();
    let json = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
    (status, headers, json)
}

async fn invite_secret(ctx: &AppCtx, admin: i64) -> (i64, String) {
    let issued = ctx
        .link
        .create_invite("trackly.example.test", 8443, [7u8; 32], 3600, admin)
        .await
        .unwrap();
    let decoded = link_proto::InviteCode::decode(&issued.code).unwrap();
    (issued.id, URL_SAFE_NO_PAD.encode(decoded.secret))
}

fn enroll_body(secret: &str, device: &str) -> serde_json::Value {
    serde_json::json!({
        "secret": secret, "deviceId": device, "hostname": "PC-EXAMPLE-01",
        "os": "windows", "arch": "x86_64", "clientVersion": "1.0.0"
    })
}

#[tokio::test]
async fn link_enroll_returns_only_device_secret_without_session() {
    let (ctx, _d) = build_ctx().await;
    let adm = seed_user(&ctx, "le_adm1", "Смирнов С.С.", "admin").await;
    let (_, secret) = invite_secret(&ctx, adm).await;
    let app = router(&ctx);

    let (st, headers, json) = send(
        &app,
        "POST",
        "/api/v1/link_enroll",
        Some(enroll_body(&secret, DEVICE)),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{json}");
    let keys: Vec<_> = json.as_object().unwrap().keys().cloned().collect();
    assert_eq!(
        keys,
        vec!["deviceSecret".to_string()],
        "только deviceSecret"
    );
    assert!(!json["deviceSecret"].as_str().unwrap().is_empty());
    assert!(
        headers.get(header::SET_COOKIE).is_none(),
        "enroll не создаёт сессию"
    );
    // Секрет реально годен для устройства.
    ctx.link
        .verify_device(DEVICE, json["deviceSecret"].as_str().unwrap())
        .await
        .unwrap();
}

#[tokio::test]
async fn link_enroll_distinguishes_used_expired_invalid() {
    let (ctx, _d) = build_ctx().await;
    let adm = seed_user(&ctx, "le_adm2", "Смирнов С.С.", "admin").await;
    let app = router(&ctx);

    let (id, secret) = invite_secret(&ctx, adm).await;
    let (st, _, _) = send(
        &app,
        "POST",
        "/api/v1/link_enroll",
        Some(enroll_body(&secret, DEVICE)),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    let (st, _, json) = send(
        &app,
        "POST",
        "/api/v1/link_enroll",
        Some(enroll_body(&secret, "0a1b2c3d-0000-4000-8000-000000000001")),
    )
    .await;
    assert_eq!(st, StatusCode::CONFLICT);
    assert_eq!(json["code"], "LINK_INVITE_USED");

    // Истёкший.
    let (id2, secret2) = invite_secret(&ctx, adm).await;
    assert_ne!(id, id2);
    ctx.writer
        .execute(move |c| {
            c.execute(
                "UPDATE link_invites SET expires_at_utc = 1 WHERE id = ?1",
                params![id2],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .unwrap();
    let (st, _, json) = send(
        &app,
        "POST",
        "/api/v1/link_enroll",
        Some(enroll_body(
            &secret2,
            "0a1b2c3d-0000-4000-8000-000000000002",
        )),
    )
    .await;
    assert_eq!(st, StatusCode::GONE);
    assert_eq!(json["code"], "LINK_INVITE_EXPIRED");

    // Мусор: тело без деталей БД.
    let (st, _, json) = send(
        &app,
        "POST",
        "/api/v1/link_enroll",
        Some(enroll_body(
            &URL_SAFE_NO_PAD.encode([5u8; 16]),
            "0a1b2c3d-0000-4000-8000-000000000003",
        )),
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    assert_eq!(json["code"], "LINK_INVITE_INVALID");
    let keys: std::collections::BTreeSet<_> = json.as_object().unwrap().keys().cloned().collect();
    assert_eq!(
        keys,
        ["code", "message"].into_iter().map(String::from).collect()
    );
}

#[tokio::test]
async fn link_enroll_rejects_bad_device_id_as_json() {
    let (ctx, _d) = build_ctx().await;
    let app = router(&ctx);
    let (st, _, json) = send(
        &app,
        "POST",
        "/api/v1/link_enroll",
        Some(enroll_body("AAAA", "not-a-uuid")),
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    assert_eq!(json["code"], "LINK_BAD_REQUEST");
}

/// spa_fallback не должен маскировать несуществующий маршрут под index.html с 200.
#[tokio::test]
async fn link_unknown_route_is_json_404_not_spa() {
    let (ctx, _d) = build_ctx().await;
    let app = router(&ctx);
    let mut req = Request::builder()
        .uri("/api/v1/link_nope")
        .body(Body::empty())
        .unwrap();
    req.extensions_mut()
        .insert(ConnectInfo::<std::net::SocketAddr>(
            "192.0.2.77:5555".parse().unwrap(),
        ));
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let ct = res.headers()[header::CONTENT_TYPE]
        .to_str()
        .unwrap()
        .to_string();
    assert!(ct.starts_with("application/json"), "content-type: {ct}");
    let bytes = axum::body::to_bytes(res.into_body(), 1 << 20)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).expect("json, не index.html");
    assert_eq!(json["code"], "NOT_FOUND");
}

#[tokio::test]
async fn link_enroll_rate_limited_after_burst() {
    let (ctx, _d) = build_ctx().await;
    let app = router(&ctx);
    let mut statuses = Vec::new();
    for _ in 0..6 {
        let (st, _, _) = send(
            &app,
            "POST",
            "/api/v1/link_enroll",
            Some(enroll_body(&URL_SAFE_NO_PAD.encode([9u8; 16]), DEVICE)),
        )
        .await;
        statuses.push(st);
    }
    assert!(
        statuses[..5].iter().all(|s| *s == StatusCode::BAD_REQUEST),
        "{statuses:?}"
    );
    assert_eq!(statuses[5], StatusCode::TOO_MANY_REQUESTS, "{statuses:?}");
}
