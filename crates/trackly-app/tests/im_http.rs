//! HTTP-транспорт IM (Phase 43-08, DLV-03, D-02, D-11): сессия, членство,
//! идемпотентный повтор, границы sync, JSON 404 на неизвестный im_-маршрут.
//! Router-level (`oneshot`); имена вымышленные.

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use serde_json::{json, Value};
use tower::ServiceExt;

use trackly_app::context::AppCtx;
use trackly_app::http::build_router;
use trackly_app::server::rusqlite_session_store::RusqliteSessionStore;
use trackly_core::auth::Role;

mod im_support;
use im_support::{build_ctx, cookie, seed_user};

fn app(ctx: &AppCtx) -> axum::Router {
    build_router(
        ctx,
        RusqliteSessionStore::new(ctx.writer.clone(), ctx.readers.clone()),
    )
}

async fn post(ctx: &AppCtx, name: &str, cookie: Option<&str>, body: Value) -> (StatusCode, Value) {
    let mut b = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/{name}"))
        .header("content-type", "application/json");
    if let Some(c) = cookie {
        b = b.header("cookie", c);
    }
    let res = app(ctx)
        .oneshot(b.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap();
    let status = res.status();
    let ct = res
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let bytes = axum::body::to_bytes(res.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    if status != StatusCode::UNAUTHORIZED {
        assert!(
            ct.contains("application/json"),
            "{name}: ожидался JSON, получен {ct:?} (spa_fallback?)"
        );
    }
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

struct Env {
    ctx: AppCtx,
    _dir: tempfile::TempDir,
    conv: i64,
    other_conv: i64,
    member: String,
    stranger: String,
    left: String,
    employee: String,
}

async fn env() -> Env {
    let (ctx, dir) = build_ctx().await;
    let a = seed_user(&ctx, "ih_a", "Иванов И.И.", "employee").await;
    let b = seed_user(&ctx, "ih_b", "Петров П.П.", "employee").await;
    let c = seed_user(&ctx, "ih_c", "Сидорова А.А.", "employee").await;
    let d = seed_user(&ctx, "ih_d", "Кузнецов К.К.", "admin").await;
    let conv = ctx
        .im
        .create_conversation(a, "room", Some("Общая"), &[b, c])
        .await
        .unwrap();
    let other_conv = ctx
        .im
        .create_conversation(d, "room", Some("Закрытая"), &[])
        .await
        .unwrap();
    ctx.im.remove_member(conv, c).await.unwrap();
    Env {
        member: cookie(&ctx, a, Role::Employee).await,
        employee: cookie(&ctx, b, Role::Employee).await,
        left: cookie(&ctx, c, Role::Employee).await,
        stranger: cookie(&ctx, d, Role::Admin).await,
        ctx,
        _dir: dir,
        conv,
        other_conv,
    }
}

fn send_body(conv: i64, id: &str, text: &str) -> Value {
    json!({ "conversationId": conv, "clientMsgId": id, "text": text })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn im_send_session_membership_and_replay() {
    tokio::time::timeout(std::time::Duration::from_secs(60), async {
        let e = env().await;
        let (st, _) = post(&e.ctx, "im_send", None, send_body(e.conv, "m1", "привет")).await;
        assert_eq!(st, StatusCode::UNAUTHORIZED);

        // Админ без членства — 404, а не 200 с пустым телом.
        let (st, body) = post(
            &e.ctx,
            "im_send",
            Some(&e.stranger),
            send_body(e.conv, "m1", "привет"),
        )
        .await;
        assert_eq!(st, StatusCode::NOT_FOUND, "{body}");
        assert!(body["code"].is_string(), "{body}");

        let (st, body) = post(
            &e.ctx,
            "im_send",
            Some(&e.left),
            send_body(e.conv, "m1", "привет"),
        )
        .await;
        assert_eq!(st, StatusCode::FORBIDDEN, "{body}");

        let (st, first) = post(
            &e.ctx,
            "im_send",
            Some(&e.member),
            send_body(e.conv, "m1", "привет"),
        )
        .await;
        assert_eq!(st, StatusCode::OK, "{first}");
        assert_eq!(first["replay"], json!(false));
        assert!(first["message"]["id"].is_i64(), "{first}");

        let (st, again) = post(
            &e.ctx,
            "im_send",
            Some(&e.member),
            send_body(e.conv, "m1", "другой текст"),
        )
        .await;
        assert_eq!(st, StatusCode::OK, "{again}");
        assert_eq!(again["replay"], json!(true));
        assert_eq!(again["message"]["id"], first["message"]["id"]);
        assert_eq!(im_support::count_messages(&e.ctx, e.conv).await, 1);

        // Сотрудник (роль employee), участник беседы — допущен: доступ по членству.
        let (st, body) = post(
            &e.ctx,
            "im_send",
            Some(&e.employee),
            send_body(e.conv, "m2", "ответ"),
        )
        .await;
        assert_eq!(st, StatusCode::OK, "{body}");
    })
    .await
    .expect("exceeded 60s");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn im_sync_membership_limit_and_unknown_route() {
    tokio::time::timeout(std::time::Duration::from_secs(60), async {
        let e = env().await;
        let (st, _) = post(&e.ctx, "im_sync", None, json!({})).await;
        assert_eq!(st, StatusCode::UNAUTHORIZED);

        for i in 0..205 {
            let (st, b) = post(
                &e.ctx,
                "im_send",
                Some(&e.member),
                send_body(e.conv, &format!("s{i}"), "текст"),
            )
            .await;
            assert_eq!(st, StatusCode::OK, "{b}");
        }

        // Employee-участник получает свою беседу; limit ограничен сервером.
        let (st, body) = post(
            &e.ctx,
            "im_sync",
            Some(&e.employee),
            json!({ "limit": 1_000_000 }),
        )
        .await;
        assert_eq!(st, StatusCode::OK, "{body}");
        let convs = body["conversations"].as_array().expect("conversations");
        assert_eq!(convs.len(), 1, "{body}");
        assert_eq!(convs[0]["conversation"]["id"], json!(e.conv));
        let n = convs[0]["messages"].as_array().unwrap().len() as i64;
        assert!(n <= trackly_core::domain::im::SYNC_PAGE_MAX, "n={n}");
        assert!(n > 0);

        // Курсор на чужую беседу её не раскрывает.
        let (st, body) = post(
            &e.ctx,
            "im_sync",
            Some(&e.employee),
            json!({ "cursors": [{ "conversationId": e.other_conv, "rev": 0 }] }),
        )
        .await;
        assert_eq!(st, StatusCode::OK, "{body}");
        for c in body["conversations"].as_array().unwrap() {
            assert_ne!(c["conversation"]["id"], json!(e.other_conv), "{body}");
        }

        // Неизвестный im_-маршрут — JSON 404, не index.html.
        let (st, _) = post(&e.ctx, "im_unknown", Some(&e.member), json!({})).await;
        assert_eq!(st, StatusCode::NOT_FOUND);
    })
    .await
    .expect("exceeded 60s");
}
