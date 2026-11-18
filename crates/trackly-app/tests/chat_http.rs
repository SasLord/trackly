//! Server-mode chat transport (Phase 41.5 Plan 17, MSG-03): session gate, BOLA,
//! JSON shapes and 404 for a missing route (never index.html via spa_fallback).
//! Router-level (`oneshot`); names are fictional.

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use serde_json::{json, Value};
use time::{Duration, OffsetDateTime};
use tower::ServiceExt;
use tower_sessions::session::{Id, Record};
use tower_sessions::SessionStore;

use trackly_app::context::AppCtx;
use trackly_app::http::auth::SessionIdentity;
use trackly_app::http::build_router;
use trackly_app::server::rusqlite_session_store::RusqliteSessionStore;
use trackly_core::auth::Role;

mod chat_support;
use chat_support::{add_participant, seed_request, seed_user};

struct Env {
    ctx: AppCtx,
    _dir: tempfile::TempDir,
    author: String,
    participant: String,
    manager: String,
    admin: String,
    stranger: String,
    request_id: i64,
    closed_id: i64,
    author_id: i64,
    other_request_id: i64,
}

async fn cookie(ctx: &AppCtx, user_id: i64, role: Role) -> String {
    let store = RusqliteSessionStore::new(ctx.writer.clone(), ctx.readers.clone());
    let id = Id::default();
    let si = SessionIdentity {
        user_id: Some(user_id),
        role: role.as_str().to_string(),
    };
    let mut record = Record {
        id,
        data: Default::default(),
        expiry_date: OffsetDateTime::now_utc() + Duration::days(1),
    };
    record
        .data
        .insert("identity".to_string(), serde_json::to_value(&si).unwrap());
    store.create(&mut record).await.expect("session");
    format!("id={id}")
}

async fn env() -> Env {
    let dir = tempfile::TempDir::new().unwrap();
    let paths = trackly_infra::Paths::resolve_for_exe_dir(dir.path().to_path_buf()).unwrap();
    let config = trackly_infra::AppConfig::default();
    let log_guard = trackly_app::logging::init(&paths, &config).or_else(|_| {
        let (_nb, guard) = tracing_appender::non_blocking(std::io::sink());
        Ok::<_, anyhow::Error>(guard)
    });
    let ctx = AppCtx::build(paths, config, log_guard.unwrap())
        .await
        .unwrap();
    let author = seed_user(&ctx, "ch_author", "Иванов И.И.", "employee").await;
    let participant = seed_user(&ctx, "ch_part", "Сидорова А.А.", "employee").await;
    let manager = seed_user(&ctx, "ch_mgr", "Петров П.П.", "manager").await;
    let admin = seed_user(&ctx, "ch_admin", "Смирнов С.С.", "admin").await;
    let stranger = seed_user(&ctx, "ch_other", "Кузнецов К.К.", "employee").await;
    let request_id = seed_request(&ctx, author, "free_form", "open").await;
    let closed_id = seed_request(&ctx, author, "free_form", "completed").await;
    let other_request_id = seed_request(&ctx, stranger, "free_form", "open").await;
    add_participant(&ctx, request_id, participant).await;
    Env {
        author: cookie(&ctx, author, Role::Employee).await,
        participant: cookie(&ctx, participant, Role::Employee).await,
        manager: cookie(&ctx, manager, Role::Manager).await,
        admin: cookie(&ctx, admin, Role::Admin).await,
        stranger: cookie(&ctx, stranger, Role::Employee).await,
        ctx,
        _dir: dir,
        request_id,
        closed_id,
        author_id: author,
        other_request_id,
    }
}

fn app(ctx: &AppCtx) -> axum::Router {
    build_router(
        ctx,
        RusqliteSessionStore::new(ctx.writer.clone(), ctx.readers.clone()),
    )
}

/// POST JSON; returns status, content type and parsed body.
async fn post_raw(
    env: &Env,
    uri: &str,
    cookie: Option<&str>,
    body: Value,
) -> (StatusCode, String, Value) {
    let mut b = Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json");
    if let Some(c) = cookie {
        b = b.header("cookie", c);
    }
    let res = app(&env.ctx)
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
    (
        status,
        ct,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

async fn post(env: &Env, name: &str, cookie: Option<&str>, body: Value) -> (StatusCode, Value) {
    let (st, ct, v) = post_raw(env, &format!("/api/v1/{name}"), cookie, body).await;
    assert!(
        ct.contains("application/json"),
        "{name}: ожидался JSON, получен {ct:?} (spa_fallback?)"
    );
    (st, v)
}

fn send_body(rid: i64, text: &str) -> Value {
    json!({ "requestId": rid, "text": text, "draftAttachmentIds": [] })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn chat_http_access_by_role() {
    tokio::time::timeout(std::time::Duration::from_secs(60), async {
        let env = env().await;
        let rid = env.request_id;
        for (who, c) in [
            ("author", &env.author),
            ("participant", &env.participant),
            ("manager", &env.manager),
            ("admin", &env.admin),
        ] {
            let (st, body) = post(&env, "chat_feed", Some(c), json!({ "requestId": rid })).await;
            assert_eq!(st, StatusCode::OK, "{who} chat_feed: {body}");
            assert!(body["items"].is_array(), "{who}: не лента: {body}");
            assert_eq!(body["canPost"], json!(true), "{who}: canPost");
        }
        // Посторонний сотрудник: 403 на всех четырёх маршрутах с requestId.
        let stranger = Some(env.stranger.as_str());
        for (name, body) in [
            ("chat_feed", json!({ "requestId": rid })),
            ("chat_send", send_body(rid, "Привет")),
            ("chat_mark_read", json!({ "requestId": rid })),
            ("chat_discard_drafts", json!({ "requestId": rid })),
        ] {
            let (st, resp) = post(&env, name, stranger, body).await;
            assert_eq!(st, StatusCode::FORBIDDEN, "stranger {name}: {resp}");
        }
        // Автор не видит чат чужой заявки.
        let (st, _) = post(
            &env,
            "chat_feed",
            Some(&env.author),
            json!({ "requestId": env.other_request_id }),
        )
        .await;
        assert_eq!(st, StatusCode::FORBIDDEN);
        // Без сессии: 401 на всех пяти маршрутах.
        for (name, body) in [
            ("chat_feed", json!({ "requestId": rid })),
            ("chat_send", send_body(rid, "Привет")),
            ("chat_mark_read", json!({ "requestId": rid })),
            ("chat_unread_summary", json!({})),
            ("chat_discard_drafts", json!({ "requestId": rid })),
        ] {
            let (st, _, _) = post_raw(&env, &format!("/api/v1/{name}"), None, body).await;
            assert_eq!(st, StatusCode::UNAUTHORIZED, "no session {name}");
        }
        env.ctx.shutdown.cancel();
    })
    .await
    .expect("chat_http_access_by_role exceeded 60s");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn chat_http_send_and_closed() {
    tokio::time::timeout(std::time::Duration::from_secs(60), async {
        let env = env().await;
        let rid = env.request_id;
        let (st, body) = post(
            &env,
            "chat_send",
            Some(&env.author),
            send_body(rid, "Привет"),
        )
        .await;
        assert_eq!(st, StatusCode::OK, "{body}");
        assert_eq!(body["message"]["text"], json!("Привет"), "{body}");
        assert!(body["rejectCode"].is_null(), "{body}");

        let (_, feed) = post(
            &env,
            "chat_feed",
            Some(&env.participant),
            json!({ "requestId": rid }),
        )
        .await;
        let texts: Vec<&Value> = feed["items"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|i| i["kind"] == json!("message"))
            .map(|i| &i["text"])
            .collect();
        assert_eq!(texts, vec![&json!("Привет")], "лента: {feed}");

        // Закрытая заявка: reject_code, а не ошибка.
        let (st, body) = post(
            &env,
            "chat_send",
            Some(&env.author),
            send_body(env.closed_id, "Ещё"),
        )
        .await;
        assert_eq!(st, StatusCode::OK, "{body}");
        assert_eq!(body["rejectCode"], json!("closed"), "{body}");
        assert!(body["message"].is_null(), "{body}");
        env.ctx.shutdown.cancel();
    })
    .await
    .expect("chat_http_send_and_closed exceeded 60s");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn chat_http_routes_are_json_not_spa() {
    tokio::time::timeout(std::time::Duration::from_secs(60), async {
        let env = env().await;
        // Несуществующий маршрут: не index.html с 200.
        let (st, ct, _) = post_raw(
            &env,
            "/api/v1/chat_nope",
            Some(&env.author),
            json!({ "requestId": env.request_id }),
        )
        .await;
        assert_eq!(st, StatusCode::NOT_FOUND, "chat_nope: spa_fallback?");
        assert!(!ct.contains("text/html"), "chat_nope отдал HTML: {ct}");
        // Все пять реальных маршрутов отдают JSON (проверка в post()).
        let rid = env.request_id;
        for (name, body) in [
            ("chat_feed", json!({ "requestId": rid })),
            ("chat_send", send_body(rid, "Привет")),
            ("chat_mark_read", json!({ "requestId": rid })),
            ("chat_unread_summary", json!({})),
            ("chat_discard_drafts", json!({ "requestId": rid })),
        ] {
            let (st, _) = post(&env, name, Some(&env.author), body).await;
            assert_eq!(st, StatusCode::OK, "{name}");
        }
        env.ctx.shutdown.cancel();
    })
    .await
    .expect("chat_http_routes_are_json_not_spa exceeded 60s");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn chat_http_unread_summary_only_own_requests() {
    tokio::time::timeout(std::time::Duration::from_secs(60), async {
        let env = env().await;
        // Сообщения от менеджера в заявке автора и в заявке постороннего.
        chat_support::message(
            &env.ctx,
            env.request_id,
            None,
            "Для автора",
            1_700_000_100,
            None,
        )
        .await;
        chat_support::message(
            &env.ctx,
            env.other_request_id,
            None,
            "Чужое",
            1_700_000_101,
            None,
        )
        .await;
        let (st, body) = post(&env, "chat_unread_summary", Some(&env.author), json!({})).await;
        assert_eq!(st, StatusCode::OK, "{body}");
        let ids: Vec<i64> = body["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|i| i["requestId"].as_i64().unwrap())
            .collect();
        assert!(ids.contains(&env.request_id), "{body}");
        assert!(
            !ids.contains(&env.other_request_id),
            "чужая заявка в сводке: {body}"
        );
        assert!(env.author_id > 0);
        env.ctx.shutdown.cancel();
    })
    .await
    .expect("chat_http_unread_summary_only_own_requests exceeded 60s");
}
