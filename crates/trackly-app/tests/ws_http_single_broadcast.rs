//! Regression coverage for CR-01 (Phase 11): a single HTTP `requests_create`
//! must broadcast EXACTLY ONE `WsEvent` to subscribers, not two.
//!
//! Before the fix, `handler_create` (and `handler_transition` /
//! `handler_approve_ad_register`) re-sent the WsEvent on `ctx.ws_broadcast`
//! *after* `RequestService::create` had already sent the identical event on
//! its own `ws_tx`. Because `ctx.ws_broadcast` and `RequestService.ws_tx` are
//! the SAME `Arc<broadcast::Sender>`, this delivered each event twice to every
//! subscriber (the "WS toast spam" symptom). The fix removes the redundant
//! handler-level send; the service layer is the single broadcast owner.
//!
//! This test subscribes to `ctx.ws_broadcast` BEFORE driving one HTTP create
//! and asserts exactly one `WsEvent::NewRequest` arrives (and no second event
//! is queued).

use axum::body::Body;
use axum::http::{Request, StatusCode};
use time::{Duration, OffsetDateTime};
use tower::ServiceExt;
use tower_sessions::session::{Id, Record};
use tower_sessions::SessionStore;

use trackly_app::context::AppCtx;
use trackly_app::dto::auth::UserNew;
use trackly_app::dto::printer::WsEvent;
use trackly_app::http::auth::SessionIdentity;
use trackly_app::http::build_router;
use trackly_app::server::rusqlite_session_store::RusqliteSessionStore;
use trackly_core::auth::{Identity, Role};

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
        expiry_date: OffsetDateTime::now_utc() + Duration::days(1),
    };
    record
        .data
        .insert("identity".to_string(), serde_json::to_value(&si)?);
    store.create(&mut record).await?;
    Ok(format!("id={session_id}"))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn http_create_broadcasts_exactly_one_event() {
    tokio::time::timeout(std::time::Duration::from_secs(30), async {
        let (ctx, _dir) = make_test_ctx().await.expect("make_test_ctx");

        let admin_identity = Identity::trusted_admin();
        let employee_dto = ctx
            .auth
            .create_user(
                UserNew {
                    login: "ws_single_employee".to_string(),
                    full_name: "Employee".to_string(),
                    password: "password123".to_string(),
                    role: "employee".to_string(),
                    email: None,
                },
                &admin_identity,
            )
            .await
            .expect("create employee user");

        let session_store = RusqliteSessionStore::new(ctx.writer.clone(), ctx.readers.clone());
        let cookie = create_session_cookie(&session_store, employee_dto.id, Role::Employee)
            .await
            .expect("create employee session");

        // Subscribe to the SAME broadcast channel the service and the (former)
        // handler both used. Subscribe BEFORE the create so we capture every
        // event the mutation produces.
        let mut rx = ctx.ws_broadcast.subscribe();

        let app = build_router(
            &ctx,
            RusqliteSessionStore::new(ctx.writer.clone(), ctx.readers.clone()),
        );

        // One HTTP free_form create.
        let body = serde_json::json!({
            "dto": {
                "requestType": "free_form",
                "printerDeviceId": null,
                "cartridgeModelId": null,
                "categoryId": null,
                "description": "Тестовая заявка для проверки одиночного broadcast"
            }
        });
        let req = Request::builder()
            .method("POST")
            .uri("/api/v1/requests_create")
            .header("content-type", "application/json")
            .header("cookie", &cookie)
            .body(Body::from(body.to_string()))
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(
            res.status(),
            StatusCode::OK,
            "HTTP requests_create should succeed"
        );

        // First event must arrive.
        let first = rx
            .recv()
            .await
            .expect("exactly one WsEvent::NewRequest expected after one HTTP create");
        assert!(
            matches!(first, WsEvent::NewRequest { .. }),
            "expected NewRequest, got {first:?}"
        );

        // No SECOND event must be queued. Before the CR-01 fix, a duplicate
        // would already be buffered and `try_recv` would return it.
        match rx.try_recv() {
            Err(tokio::sync::broadcast::error::TryRecvError::Empty) => { /* good */ }
            Ok(extra) => panic!(
                "duplicate WsEvent broadcast on single HTTP create (CR-01 regression): {extra:?}"
            ),
            Err(other) => panic!("unexpected broadcast channel state: {other:?}"),
        }

        ctx.shutdown.cancel();
    })
    .await
    .expect("http_create_broadcasts_exactly_one_event exceeded 30s budget");
}
