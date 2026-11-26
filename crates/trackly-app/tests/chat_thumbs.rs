//! Thumbnail of a chat-message attachment on all server read paths (Phase 41.5
//! Plan 21, UAT gap 4): service, HTTP via `build_router`, `trackly-att` URI.
//! Names are fictional.

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use time::{Duration, OffsetDateTime};
use tower::ServiceExt;
use tower_sessions::session::{Id, Record};
use tower_sessions::SessionStore;

use trackly_app::attachments_protocol::{build_response, parse_uri};
use trackly_app::context::AppCtx;
use trackly_app::dto::chat::ChatItemDto;
use trackly_app::http::auth::SessionIdentity;
use trackly_app::http::build_router;
use trackly_app::server::rusqlite_session_store::RusqliteSessionStore;
use trackly_app::services::attachment_service::{ParentRef, ReadKind};
use trackly_core::auth::Role;
use trackly_core::domain::attachments::ParentKind;

mod attachments_support;
mod chat_support;
use chat_support::{ident, seed_request, seed_user, upload};

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

async fn get(ctx: &AppCtx, uri: &str, cookie: Option<&str>) -> (StatusCode, String, Vec<u8>) {
    let mut b = Request::builder().method("GET").uri(uri);
    if let Some(c) = cookie {
        b = b.header("cookie", c);
    }
    let router = build_router(
        ctx,
        RusqliteSessionStore::new(ctx.writer.clone(), ctx.readers.clone()),
    );
    let res = router
        .oneshot(b.body(Body::empty()).unwrap())
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
    (status, ct, bytes.to_vec())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn message_attachment_thumb_on_every_read_path() {
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
    let author = seed_user(&ctx, "th_author", "Иванов И.И.", "employee").await;
    let manager = seed_user(&ctx, "th_mgr", "Петров П.П.", "manager").await;
    let stranger = seed_user(&ctx, "th_other", "Кузнецов К.К.", "employee").await;
    let r = seed_request(&ctx, author, "free_form", "open").await;
    let who = ident(author, Role::Employee);
    let mgr = ident(manager, Role::Manager);

    // Photo as a draft, then sent with the message.
    let draft = ParentRef {
        kind: ParentKind::MessageDraft,
        id: r,
    };
    let att_id = upload(
        &ctx,
        &who,
        draft,
        "photo.png",
        &attachments_support::tiny_png(),
    )
    .await
    .expect("upload")
    .attachment
    .expect("attachment")
    .id;
    let out = ctx
        .chat
        .send(&who, r, "", &[att_id], None)
        .await
        .expect("send");
    assert!(out.reject_code.is_none(), "{:?}", out.reject_code);

    // Path 1: service.
    let feed = ctx.chat.feed(&who, r).await.unwrap();
    let att = feed
        .items
        .iter()
        .find_map(|i| match i {
            ChatItemDto::Message(m) => m.attachments.iter().find(|a| a.id == att_id),
            _ => None,
        })
        .expect("attachment in feed");
    assert!(att.has_thumbnail, "has_thumbnail");
    assert!(!att.missing, "missing");
    assert_eq!(att.mime, "image/png");
    for caller in [&who, &mgr] {
        let o = ctx
            .attachments
            .open_for_read(att_id, caller, ReadKind::Thumb)
            .await
            .expect("open thumb");
        assert!(o.path.is_file(), "thumb on disk");
        assert_eq!(o.mime, "image/jpeg");
    }

    // Path 2: HTTP through the real router.
    let uri = format!("/api/v1/attachments/{att_id}/thumb");
    for c in [
        cookie(&ctx, author, Role::Employee).await,
        cookie(&ctx, manager, Role::Manager).await,
    ] {
        let (st, ct, body) = get(&ctx, &uri, Some(&c)).await;
        assert_eq!(st, StatusCode::OK);
        assert!(ct.starts_with("image/jpeg"), "content-type {ct:?}");
        assert_eq!(&body[..2], &[0xFF, 0xD8], "JPEG magic");
    }
    let sc = cookie(&ctx, stranger, Role::Employee).await;
    let (st, _, _) = get(&ctx, &uri, Some(&sc)).await;
    assert_eq!(st, StatusCode::FORBIDDEN, "stranger");
    let (st, _, _) = get(&ctx, &uri, None).await;
    assert_eq!(st, StatusCode::UNAUTHORIZED, "no cookie");

    // Path 3: trackly-att URI (both platform URL forms use the same path).
    for url in [
        format!("trackly-att://localhost/{att_id}/thumb"),
        format!("http://trackly-att.localhost/{att_id}/thumb"),
    ] {
        let parsed: tauri::http::Uri = url.parse().unwrap();
        let (id, kind) = parse_uri(parsed.path()).expect("parse");
        assert_eq!(id, att_id, "{url}");
        assert!(matches!(kind, ReadKind::Thumb), "{url}");
        let o = ctx
            .attachments
            .open_for_read(id, &who, kind)
            .await
            .expect("open");
        let bytes = std::fs::read(&o.path).unwrap();
        let resp = build_response(&o, bytes, true);
        assert_eq!(resp.status(), 200);
        assert_eq!(
            resp.headers().get(header::CONTENT_TYPE).unwrap(),
            "image/jpeg"
        );
        assert_eq!(&resp.body()[..2], &[0xFF, 0xD8]);
    }
}
