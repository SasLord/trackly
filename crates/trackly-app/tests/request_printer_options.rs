//! Integration tests for `RequestService::printer_options` / the
//! `request_printer_options` endpoint (D-PRN-01, Phase 11 Plan 02).
//!
//! Phase 10 closed `ReadData`/`ReadPrinters` for Employee — this endpoint is
//! deliberately gated on `Action::CreateRequest` instead (every role has it,
//! Employee included). These tests prove:
//! 1. Employee session → 200 + a populated list.
//! 2. The serialized JSON contains ONLY `id`/`name`/`place` — no
//!    snmp/community/ip/serial keys leak (BOLA/BOPLA closure, T-11-02-I).
//! 3. Results are sorted by place, then name; printers without a
//!    place sort last.
//! 4. No session → 401.
//!
//! Session setup mirrors `role_endpoint_matrix.rs` / `requests_ad_register_http.rs`:
//! sessions are created programmatically via `RusqliteSessionStore`, bypassing
//! the `GovernorLayer` on `/auth_login` (which needs a real TCP peer IP).

use axum::body::Body;
use axum::http::{Request, StatusCode};
use rusqlite::params;
use time::{Duration, OffsetDateTime};
use tower::ServiceExt;
use tower_sessions::session::{Id, Record};
use tower_sessions::SessionStore;

use trackly_app::context::AppCtx;
use trackly_app::dto::auth::UserNew;
use trackly_app::http::auth::SessionIdentity;
use trackly_app::http::build_router;
use trackly_app::server::rusqlite_session_store::RusqliteSessionStore;
use trackly_core::auth::{Identity, Role};
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
        expiry_date: OffsetDateTime::now_utc() + Duration::days(1),
    };
    record
        .data
        .insert("identity".to_string(), serde_json::to_value(&si)?);
    store.create(&mut record).await?;
    Ok(format!("id={session_id}"))
}

async fn post_with_cookie(
    app: axum::Router,
    uri: &str,
    cookie: Option<&str>,
) -> (StatusCode, serde_json::Value) {
    let mut builder = Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json");
    if let Some(c) = cookie {
        builder = builder.header("cookie", c);
    }
    let req = builder.body(Body::from("{}")).unwrap();
    let res = app.oneshot(req).await.unwrap();
    let status = res.status();
    let body_bytes = axum::body::to_bytes(res.into_body(), 64 * 1024)
        .await
        .unwrap_or_default();
    let body_json = serde_json::from_slice(&body_bytes).unwrap_or(serde_json::Value::Null);
    (status, body_json)
}

/// Seed `n` printer devices (`type_id = 2`) directly via the writer, with the
/// given `(name, place_name)` pairs. `place_name = None` leaves `place_id`
/// NULL. Places are created as root-level `zone` nodes (kind is irrelevant
/// to this test — only the resolved `full_path`, which for a root node
/// equals its own `name`, matters). Returns nothing — fixtures are read
/// back by the endpoint under test.
async fn seed_printer_devices(ctx: &AppCtx, printers: &[(&str, Option<&str>)]) {
    let now = SystemClock.unix_seconds();
    let printers: Vec<(String, Option<String>)> = printers
        .iter()
        .map(|(name, loc)| (name.to_string(), loc.map(|s| s.to_string())))
        .collect();
    ctx.writer
        .execute(move |conn| {
            let tx = conn
                .transaction()
                .map_err(|e| trackly_core::error::AppError::Internal {
                    source_chain: format!("{e}"),
                })?;
            for (name, loc) in &printers {
                let place_id: Option<i64> = if let Some(place_name) = loc {
                    tx.execute(
                        "INSERT OR IGNORE INTO places \
                         (parent_id, kind, name, created_at_utc, updated_at_utc) \
                         VALUES (NULL, 'zone', ?1, ?2, ?2)",
                        params![place_name, now],
                    )
                    .map_err(|e| trackly_core::error::AppError::Internal {
                        source_chain: format!("{e}"),
                    })?;
                    let id: i64 = tx
                        .query_row(
                            "SELECT id FROM places WHERE parent_id IS NULL AND name = ?1",
                            params![place_name],
                            |r| r.get(0),
                        )
                        .map_err(|e| trackly_core::error::AppError::Internal {
                            source_chain: format!("{e}"),
                        })?;
                    Some(id)
                } else {
                    None
                };
                tx.execute(
                    "INSERT INTO devices \
                     (type_id, name, place_id, status_id, created_at_utc, updated_at_utc, version) \
                     VALUES (2, ?1, ?2, 1, ?3, ?3, 1)",
                    params![name, place_id, now],
                )
                .map_err(|e| trackly_core::error::AppError::Internal {
                    source_chain: format!("{e}"),
                })?;
            }
            tx.commit()
                .map_err(|e| trackly_core::error::AppError::Internal {
                    source_chain: format!("{e}"),
                })?;
            Ok(())
        })
        .await
        .expect("seed printer devices");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn employee_gets_printer_options_minimal_dto() {
    tokio::time::timeout(std::time::Duration::from_secs(30), async {
        let (ctx, _dir) = make_test_ctx().await.expect("make_test_ctx");

        let admin_identity = Identity::trusted_admin();
        let employee_dto = ctx
            .auth
            .create_user(
                UserNew {
                    login: "printer_opts_employee".to_string(),
                    full_name: "Employee".to_string(),
                    password: "password123".to_string(),
                    role: "employee".to_string(),
                    email: None,
                },
                &admin_identity,
            )
            .await
            .expect("create employee user");

        // Seed: 2 printers in "Офис Б" (alphabetically after "Офис А"), 1 in
        // "Офис А", 1 with no place — proves sort order (place, then
        // name; no-place last).
        seed_printer_devices(
            &ctx,
            &[
                ("Принтер Б2", Some("Офис Б")),
                ("Принтер А1", Some("Офис А")),
                ("Принтер Б1", Some("Офис Б")),
                ("Принтер Без Расположения", None),
            ],
        )
        .await;

        let session_store = RusqliteSessionStore::new(ctx.writer.clone(), ctx.readers.clone());
        let employee_cookie =
            create_session_cookie(&session_store, employee_dto.id, Role::Employee)
                .await
                .expect("create employee session");

        let app = build_router(
            &ctx,
            RusqliteSessionStore::new(ctx.writer.clone(), ctx.readers.clone()),
        );
        let (status, body) = post_with_cookie(
            app,
            "/api/v1/request_printer_options",
            Some(&employee_cookie),
        )
        .await;

        assert_eq!(
            status,
            StatusCode::OK,
            "Employee → request_printer_options expected 200, got {status} (body: {body:?})"
        );

        let items = body.as_array().expect("response body must be a JSON array");
        assert_eq!(items.len(), 4, "expected 4 seeded printers, got {items:?}");

        // Minimal DTO: only id/name/place keys, nothing else (BOLA/BOPLA
        // closure — no snmp/community/ip/serial fields).
        for item in items {
            let obj = item.as_object().expect("each item must be a JSON object");
            let mut keys: Vec<&str> = obj.keys().map(|k| k.as_str()).collect();
            keys.sort_unstable();
            assert_eq!(
                keys,
                vec!["id", "name", "place"],
                "request_printer_options item must contain ONLY id/name/place, got keys: {keys:?}"
            );
            for forbidden in [
                "snmp",
                "community",
                "ip",
                "ipAddress",
                "serial",
                "serialNo",
                "model",
            ] {
                assert!(
                    !obj.contains_key(forbidden),
                    "request_printer_options item leaked forbidden key '{forbidden}': {obj:?}"
                );
            }
        }

        // Sort order: "Офис А" group first, then "Офис Б" group (alphabetic
        // within group by name), then the NULL-place printer last.
        let names: Vec<&str> = items.iter().map(|i| i["name"].as_str().unwrap()).collect();
        assert_eq!(
            names,
            vec![
                "Принтер А1",
                "Принтер Б1",
                "Принтер Б2",
                "Принтер Без Расположения"
            ],
            "sort order must be place then name, NULL-place last; got {names:?}"
        );

        let places: Vec<Option<&str>> = items.iter().map(|i| i["place"].as_str()).collect();
        assert_eq!(
            places,
            vec![Some("Офис А"), Some("Офис Б"), Some("Офис Б"), None],
            "place values must match seeded data in sorted order; got {places:?}"
        );

        ctx.shutdown.cancel();
    })
    .await
    .expect("employee_gets_printer_options_minimal_dto exceeded 30s budget");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn no_session_gets_401() {
    tokio::time::timeout(std::time::Duration::from_secs(30), async {
        let (ctx, _dir) = make_test_ctx().await.expect("make_test_ctx");
        let app = build_router(
            &ctx,
            RusqliteSessionStore::new(ctx.writer.clone(), ctx.readers.clone()),
        );
        let (status, _body) = post_with_cookie(app, "/api/v1/request_printer_options", None).await;
        assert_eq!(
            status,
            StatusCode::UNAUTHORIZED,
            "No session → request_printer_options expected 401, got {status}"
        );
        ctx.shutdown.cancel();
    })
    .await
    .expect("no_session_gets_401 exceeded 30s budget");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn empty_printer_list_returns_empty_array() {
    tokio::time::timeout(std::time::Duration::from_secs(30), async {
        let (ctx, _dir) = make_test_ctx().await.expect("make_test_ctx");

        let admin_identity = Identity::trusted_admin();
        let employee_dto = ctx
            .auth
            .create_user(
                UserNew {
                    login: "printer_opts_employee_empty".to_string(),
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
        let employee_cookie =
            create_session_cookie(&session_store, employee_dto.id, Role::Employee)
                .await
                .expect("create employee session");

        let app = build_router(
            &ctx,
            RusqliteSessionStore::new(ctx.writer.clone(), ctx.readers.clone()),
        );
        let (status, body) = post_with_cookie(
            app,
            "/api/v1/request_printer_options",
            Some(&employee_cookie),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            body.as_array().map(|a| a.len()),
            Some(0),
            "no printers seeded → expected empty array, got {body:?}"
        );

        ctx.shutdown.cancel();
    })
    .await
    .expect("empty_printer_list_returns_empty_array exceeded 30s budget");
}
