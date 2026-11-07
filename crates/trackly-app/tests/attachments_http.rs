//! Server-mode attachments transport (Phase 41.3 Plan 11, ATT-02):
//! session gate, BOLA, 404 JSON (never index.html), safe headers, Range,
//! multipart upload. Router-level (`oneshot`), names are fictional.

use axum::body::Body;
use axum::http::{header, Request, Response, StatusCode};
use rusqlite::params;
use serde_json::{json, Value};
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
use trackly_core::domain::attachments::ParentKind;
use trackly_infra::error_conversions::map_rusqlite;
use trackly_infra::repos::attachments_sqlite::{AttachmentNew, SqliteAttachmentRepository};

mod attachments_support;

const NOW: i64 = 1_700_000_000;
const BOUNDARY: &str = "----trackly-test-boundary";

struct Env {
    ctx: AppCtx,
    _dir: tempfile::TempDir,
    admin_cookie: String,
    author_cookie: String,
    other_cookie: String,
    author_id: i64,
    request_id: i64,
}

async fn make_user(ctx: &AppCtx, login: &str, full: &str, role: Role) -> i64 {
    ctx.auth
        .create_user(
            UserNew {
                login: login.to_string(),
                full_name: full.to_string(),
                password: "password123".to_string(),
                role: role.as_str().to_string(),
                email: None,
            },
            &Identity::trusted_admin(),
        )
        .await
        .expect("create user")
        .id
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
    let admin = make_user(&ctx, "att_admin", "Иванов И.И.", Role::Admin).await;
    let author = make_user(&ctx, "att_author", "Петров П.П.", Role::Employee).await;
    let other = make_user(&ctx, "att_other", "Сидоров С.С.", Role::Employee).await;
    let admin_cookie = cookie(&ctx, admin, Role::Admin).await;
    let author_cookie = cookie(&ctx, author, Role::Employee).await;
    let other_cookie = cookie(&ctx, other, Role::Employee).await;
    let request_id = seed_request(&ctx, author).await;
    Env {
        ctx,
        _dir: dir,
        admin_cookie,
        author_cookie,
        other_cookie,
        author_id: author,
        request_id,
    }
}

async fn seed_request(ctx: &AppCtx, author: i64) -> i64 {
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO requests (request_type, status, requested_by_user_id, \
                 created_at_utc, updated_at_utc) VALUES ('free_form', 'open', ?1, 1700000000, 1700000000)",
                params![author],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed request")
}

/// Row plus a blob with the given bytes (hash is a fake, content arbitrary).
async fn seed_attachment(
    env: &Env,
    name: &str,
    mime: &str,
    seed: u8,
    bytes: &[u8],
    uploader: i64,
) -> i64 {
    let h = format!("{seed:02x}").repeat(32);
    let path = env.ctx.attachments.store().blob_path(&h).unwrap();
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, bytes).unwrap();
    let (name, mime, size, rid) = (
        name.to_string(),
        mime.to_string(),
        bytes.len() as i64,
        env.request_id,
    );
    env.ctx
        .writer
        .execute(move |conn| {
            SqliteAttachmentRepository.insert_on(
                conn,
                &AttachmentNew {
                    parent_kind: ParentKind::Request.as_str(),
                    parent_id: rid,
                    original_name: &name,
                    mime: &mime,
                    size_bytes: size,
                    content_hash: &h,
                    uploaded_by_user_id: Some(uploader),
                    created_at_utc: NOW,
                },
            )
        })
        .await
        .expect("seed attachment")
}

fn app(ctx: &AppCtx) -> axum::Router {
    build_router(
        ctx,
        RusqliteSessionStore::new(ctx.writer.clone(), ctx.readers.clone()),
    )
}

async fn get(env: &Env, uri: &str, cookie: Option<&str>, range: Option<&str>) -> Response<Body> {
    let mut b = Request::builder().method("GET").uri(uri);
    if let Some(c) = cookie {
        b = b.header("cookie", c);
    }
    if let Some(r) = range {
        b = b.header("range", r);
    }
    app(&env.ctx)
        .oneshot(b.body(Body::empty()).unwrap())
        .await
        .unwrap()
}

async fn post_json(
    env: &Env,
    name: &str,
    cookie: Option<&str>,
    body: Value,
) -> (StatusCode, Value) {
    let mut b = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/{name}"))
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
    assert!(
        ct.contains("application/json"),
        "{name}: ожидался JSON, получен content-type {ct:?} (spa_fallback?)"
    );
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

async fn body_json(res: Response<Body>) -> Value {
    let bytes = axum::body::to_bytes(res.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap_or(Value::Null)
}

fn ct(res: &Response<Body>) -> String {
    res.headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string()
}

fn hdr(res: &Response<Body>, name: &str) -> String {
    res.headers()
        .get(name)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string()
}

fn multipart(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut out = Vec::new();
    for (name, data) in files {
        out.extend_from_slice(
            format!(
                "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"file\"; \
                 filename=\"{name}\"\r\nContent-Type: application/octet-stream\r\n\r\n"
            )
            .as_bytes(),
        );
        out.extend_from_slice(data);
        out.extend_from_slice(b"\r\n");
    }
    out.extend_from_slice(format!("--{BOUNDARY}--\r\n").as_bytes());
    out
}

async fn upload(env: &Env, cookie: Option<&str>, body: Vec<u8>) -> (StatusCode, String, Value) {
    let mut b = Request::builder()
        .method("POST")
        .uri(format!(
            "/api/v1/attachments_upload?parent_kind=request&parent_id={}",
            env.request_id
        ))
        .header(
            "content-type",
            format!("multipart/form-data; boundary={BOUNDARY}"),
        )
        .header("content-length", body.len().to_string());
    if let Some(c) = cookie {
        b = b.header("cookie", c);
    }
    let res = app(&env.ctx)
        .oneshot(b.body(Body::from(body)).unwrap())
        .await
        .unwrap();
    let status = res.status();
    let acao = hdr(&res, "access-control-allow-origin");
    (status, acao, body_json(res).await)
}

// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn unauthenticated_file_thumb_and_json_are_401() {
    let env = env().await;
    let id = seed_attachment(
        &env,
        "a.png",
        "image/png",
        1,
        &attachments_support::tiny_png(),
        env.author_id,
    )
    .await;
    for uri in [
        format!("/api/v1/attachments/{id}/file"),
        format!("/api/v1/attachments/{id}/thumb"),
    ] {
        let res = get(&env, &uri, None, None).await;
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED, "{uri}");
        assert!(ct(&res).contains("application/json"), "{uri}: {}", ct(&res));
    }
    let (st, _) = post_json(
        &env,
        "attachments_list",
        None,
        json!({"parentKind": "request", "parentId": env.request_id}),
    )
    .await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn not_found_json_for_unknown_id_and_missing_blob() {
    let env = env().await;
    let res = get(
        &env,
        "/api/v1/attachments/999999/file",
        Some(&env.admin_cookie),
        None,
    )
    .await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    assert!(ct(&res).contains("application/json"), "{}", ct(&res));
    let v = body_json(res).await;
    assert_eq!(v["code"], "NOT_FOUND", "{v}");

    let id = seed_attachment(
        &env,
        "gone.pdf",
        "application/pdf",
        2,
        b"%PDF-1.4 x",
        env.author_id,
    )
    .await;
    let h = format!("{:02x}", 2u8).repeat(32);
    std::fs::remove_file(env.ctx.attachments.store().blob_path(&h).unwrap()).unwrap();
    let res = get(
        &env,
        &format!("/api/v1/attachments/{id}/file"),
        Some(&env.admin_cookie),
        None,
    )
    .await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    assert!(ct(&res).contains("application/json"), "{}", ct(&res));

    // Thumb of a non-photo is a 404 JSON as well.
    let pdf = seed_attachment(
        &env,
        "doc.pdf",
        "application/pdf",
        3,
        b"%PDF-1.4 y",
        env.author_id,
    )
    .await;
    let res = get(
        &env,
        &format!("/api/v1/attachments/{pdf}/thumb"),
        Some(&env.admin_cookie),
        None,
    )
    .await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    assert!(ct(&res).contains("application/json"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn employee_on_foreign_request_is_forbidden_author_is_allowed() {
    let env = env().await;
    let id = seed_attachment(
        &env,
        "a.png",
        "image/png",
        4,
        &attachments_support::tiny_png(),
        env.author_id,
    )
    .await;
    for kind in ["file"] {
        let uri = format!("/api/v1/attachments/{id}/{kind}");
        let res = get(&env, &uri, Some(&env.other_cookie), None).await;
        assert_eq!(res.status(), StatusCode::FORBIDDEN, "foreign {uri}");
        assert!(ct(&res).contains("application/json"));
        let res = get(&env, &uri, Some(&env.author_cookie), None).await;
        assert_eq!(res.status(), StatusCode::OK, "author {uri}");
    }
    // Thumb: the row has a PNG mime but no generated thumbnail -> authorization
    // must still come first (403 for the foreign user, not 404).
    let res = get(
        &env,
        &format!("/api/v1/attachments/{id}/thumb"),
        Some(&env.other_cookie),
        None,
    )
    .await;
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    let body = json!({"parentKind": "request", "parentId": env.request_id});
    let (st, _) = post_json(
        &env,
        "attachments_list",
        Some(&env.other_cookie),
        body.clone(),
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    let (st, v) = post_json(&env, "attachments_list", Some(&env.author_cookie), body).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(v["items"].as_array().map(|a| a.len()), Some(1), "{v}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn headers_png_docx_and_other_routes_keep_deny() {
    let env = env().await;
    let png = seed_attachment(
        &env,
        "фото.txt",
        "image/png",
        5,
        &attachments_support::tiny_png(),
        env.author_id,
    )
    .await;
    let res = get(
        &env,
        &format!("/api/v1/attachments/{png}/file"),
        Some(&env.author_cookie),
        None,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(ct(&res), "image/png", "mime из БД, не из имени");
    assert_eq!(hdr(&res, "x-content-type-options"), "nosniff");
    let cd = hdr(&res, "content-disposition");
    assert!(cd.starts_with("inline"), "{cd}");
    assert!(cd.contains("filename*=UTF-8''"), "{cd}");
    let cc = hdr(&res, "cache-control");
    assert!(cc.contains("private") && cc.contains("no-store"), "{cc}");
    assert_eq!(hdr(&res, "x-frame-options"), "SAMEORIGIN");

    let docx = seed_attachment(
        &env,
        "отчёт.docx",
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        6,
        b"PK\x03\x04docx",
        env.author_id,
    )
    .await;
    let res = get(
        &env,
        &format!("/api/v1/attachments/{docx}/file"),
        Some(&env.author_cookie),
        None,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    assert!(hdr(&res, "content-disposition").starts_with("attachment"));

    // CR/LF and quotes in the stored name never reach the header raw.
    let evil = seed_attachment(
        &env,
        "a\r\nX-Evil: 1\"b.png",
        "image/png",
        7,
        &attachments_support::tiny_png(),
        env.author_id,
    )
    .await;
    let res = get(
        &env,
        &format!("/api/v1/attachments/{evil}/file"),
        Some(&env.author_cookie),
        None,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    assert!(res.headers().get("x-evil").is_none());
    let cd = hdr(&res, "content-disposition");
    assert!(!cd.contains('\r') && !cd.contains('\n'), "{cd:?}");

    // Any other route: DENY.
    let res = app(&env.ctx)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth_status")
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(hdr(&res, "x-frame-options"), "DENY");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn range_request_on_pdf_is_206() {
    let env = env().await;
    let data: Vec<u8> = (0..200u8).collect();
    let id = seed_attachment(&env, "doc.pdf", "application/pdf", 8, &data, env.author_id).await;
    let res = get(
        &env,
        &format!("/api/v1/attachments/{id}/file"),
        Some(&env.author_cookie),
        Some("bytes=0-9"),
    )
    .await;
    assert_eq!(res.status(), StatusCode::PARTIAL_CONTENT);
    assert!(hdr(&res, "content-range").starts_with("bytes 0-9/200"));
    assert_eq!(ct(&res), "application/pdf");
    let bytes = axum::body::to_bytes(res.into_body(), 1024).await.unwrap();
    assert_eq!(bytes.len(), 10);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn multipart_upload_single_file_ok_without_cors() {
    let env = env().await;
    let png = attachments_support::tiny_png();
    let (st, acao, v) = upload(
        &env,
        Some(&env.author_cookie),
        multipart(&[("фото.png", &png)]),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{v}");
    assert_eq!(acao, "", "upload не должен получать CORS-заголовки");
    assert!(v["rejectCode"].is_null(), "{v}");
    assert_eq!(v["attachment"]["originalName"], "фото.png", "{v}");
    assert_eq!(v["attachment"]["mime"], "image/png", "{v}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn multipart_single_file_rule_rejects_second_field() {
    let env = env().await;
    let png = attachments_support::tiny_png();
    let (st, _, v) = upload(
        &env,
        Some(&env.author_cookie),
        multipart(&[("a.png", &png), ("b.png", &png)]),
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST, "{v}");
    assert_eq!(v["code"], "VALIDATION", "{v}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn multipart_too_large_is_reject_code_not_transport_error() {
    let env = env().await;
    let (st, v) = post_json(
        &env,
        "attachments_set_limit",
        Some(&env.admin_cookie),
        json!({"bytes": 1024 * 1024}),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{v}");
    // Content-Length within the overhead allowance: caught by the stream cap.
    let data = vec![0u8; 1024 * 1024 + 4096];
    let (st, _, v) = upload(
        &env,
        Some(&env.author_cookie),
        multipart(&[("big.bin", &data)]),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{v}");
    assert_eq!(v["rejectCode"], "too_large", "{v}");
    assert!(v["attachment"].is_null());
    // Far beyond the allowance: early Content-Length reject, same shape.
    let data = vec![0u8; 3 * 1024 * 1024];
    let (st, _, v) = upload(
        &env,
        Some(&env.author_cookie),
        multipart(&[("big.bin", &data)]),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{v}");
    assert_eq!(v["rejectCode"], "too_large", "{v}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn multipart_forbidden_before_body_is_read() {
    let env = env().await;
    // Over the transport limit (100 MiB + 1 MiB): without authorize-before-read
    // this would be 413. A foreign employee must get 403 first.
    let data = vec![0u8; 101 * 1024 * 1024 + 4096];
    let (st, _, v) = upload(
        &env,
        Some(&env.other_cookie),
        multipart(&[("x.bin", &data)]),
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN, "{v}");
    let (st, _, _) = upload(&env, None, multipart(&[("x.bin", b"1")])).await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn json_routes_are_reachable_and_return_real_json() {
    let env = env().await;
    let id = seed_attachment(
        &env,
        "doc.pdf",
        "application/pdf",
        9,
        b"%PDF-1.4 z",
        env.author_id,
    )
    .await;
    let a = Some(env.admin_cookie.as_str());
    let parent = json!({"parentKind": "request", "parentId": env.request_id});

    let (st, v) = post_json(&env, "attachments_list", a, parent.clone()).await;
    assert_eq!(st, StatusCode::OK);
    assert!(v["items"].is_array(), "{v}");
    let (st, v) = post_json(&env, "attachments_history", a, parent).await;
    assert_eq!(st, StatusCode::OK);
    assert!(v.is_array(), "{v}");
    let (st, v) = post_json(&env, "attachments_get_limit", a, json!({})).await;
    assert_eq!(st, StatusCode::OK);
    assert!(v["maxBytes"].is_number(), "{v}");
    let (st, v) = post_json(
        &env,
        "attachments_set_limit",
        a,
        json!({"bytes": 2 * 1024 * 1024}),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{v}");
    let (st, v) = post_json(&env, "attachments_report", a, json!({})).await;
    assert_eq!(st, StatusCode::OK);
    assert!(v.is_null() || v.is_object(), "{v}");
    let (st, _) = post_json(&env, "attachments_delete", a, json!({"id": id})).await;
    assert_eq!(st, StatusCode::OK);
    let (st, _) = post_json(&env, "attachments_restore", a, json!({"id": id})).await;
    assert_eq!(st, StatusCode::OK);
    let (st, _) = post_json(&env, "attachments_delete", a, json!({"id": id})).await;
    assert_eq!(st, StatusCode::OK);
    let (st, _) = post_json(&env, "attachments_purge", a, json!({"id": id})).await;
    assert_eq!(st, StatusCode::OK);
    // Gone for real: a second purge is a 404 JSON, not index.html.
    let (st, v) = post_json(&env, "attachments_purge", a, json!({"id": id})).await;
    assert_eq!(st, StatusCode::NOT_FOUND, "{v}");
    assert_eq!(v["code"], "NOT_FOUND");
}
