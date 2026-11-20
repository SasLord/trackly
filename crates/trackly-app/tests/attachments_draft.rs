//! Message drafts in the attachment service (Phase 41.5 Plan 04):
//! upload rights, closed/ad_register rejection, isolation from public paths,
//! no audit/WS noise, `from_message` flag. Names are fictional.

use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use rusqlite::params;
use time::{Duration as TimeDuration, OffsetDateTime};
use tower::ServiceExt;
use tower_sessions::session::{Id, Record};
use tower_sessions::SessionStore;

use trackly_app::context::AppCtx;
use trackly_app::dto::attachment::AttachmentUploadDto;
use trackly_app::dto::auth::UserNew;
use trackly_app::dto::printer::WsEvent;
use trackly_app::http::auth::SessionIdentity;
use trackly_app::http::build_router;
use trackly_app::server::rusqlite_session_store::RusqliteSessionStore;
use trackly_app::services::attachment_service::{ParentRef, ReadKind, StageError};
use trackly_core::auth::{Identity, Role};
use trackly_core::domain::attachments::ParentKind;
use trackly_core::error::AppError;
use trackly_infra::error_conversions::map_rusqlite;
use trackly_infra::repos::attachments_sqlite::{AttachmentNew, SqliteAttachmentRepository};

#[path = "entities_support/fixture.rs"]
#[allow(dead_code)]
mod fixture;

mod attachments_support;

fn ident(id: i64, role: Role) -> Identity {
    Identity {
        user_id: Some(id),
        role,
    }
}

async fn seed_user(ctx: &AppCtx, login: &str, full: &str, role: &str) -> i64 {
    let (login, full, role) = (login.to_string(), full.to_string(), role.to_string());
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO users (login, full_name, role, created_at_utc, updated_at_utc) \
                 VALUES (?1, ?2, ?3, 1700000000, 1700000000)",
                params![login, full, role],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed user")
}

async fn seed_request(ctx: &AppCtx, author: i64, rtype: &str, status: &str) -> i64 {
    let (rtype, status) = (rtype.to_string(), status.to_string());
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO requests (request_type, status, requested_by_user_id, \
                 created_at_utc, updated_at_utc) VALUES (?1, ?2, ?3, 1700000000, 1700000000)",
                params![rtype, status, author],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed request")
}

async fn add_participant(ctx: &AppCtx, request_id: i64, user_id: i64) {
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO request_participants (request_id, user_id, joined_at_utc) \
                 VALUES (?1, ?2, 1700000000)",
                params![request_id, user_id],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("participant");
}

fn draft(id: i64) -> ParentRef {
    ParentRef {
        kind: ParentKind::MessageDraft,
        id,
    }
}

fn req(id: i64) -> ParentRef {
    ParentRef {
        kind: ParentKind::Request,
        id,
    }
}

async fn upload(
    ctx: &AppCtx,
    who: &Identity,
    parent: ParentRef,
    name: &str,
    bytes: &[u8],
) -> Result<AttachmentUploadDto, AppError> {
    let svc = &ctx.attachments;
    svc.authorize_attach(who, parent).await?;
    let limit = svc.effective_limit().await?;
    let mut staged = svc.begin_staging()?;
    if let Err(e) = staged.write_chunk(bytes, limit) {
        return match e {
            StageError::Reject(c) => Ok(AttachmentUploadDto {
                attachment: None,
                reject_code: Some(c.as_code().to_string()),
            }),
            StageError::Internal(e) => Err(e),
        };
    }
    svc.ingest(who, parent, name, staged).await
}

async fn audit_count(ctx: &AppCtx) -> i64 {
    ctx.writer
        .execute(|conn| {
            conn.query_row("SELECT COUNT(*) FROM audit_log", [], |r| r.get(0))
                .map_err(map_rusqlite)
        })
        .await
        .unwrap()
}

macro_rules! timed {
    ($body:expr) => {
        tokio::time::timeout(Duration::from_secs(60), $body)
            .await
            .expect("timeout")
    };
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn attachments_draft_upload_roles_and_status() {
    timed!(async {
        let (ctx, _dir) = fixture::make_test_ctx().await;
        let author = seed_user(&ctx, "ivanov", "Иванов И.И.", "employee").await;
        let member = seed_user(&ctx, "sidorova", "Сидорова А.А.", "employee").await;
        let mgr = seed_user(&ctx, "petrov", "Петров П.П.", "manager").await;
        let stranger = seed_user(&ctx, "kuznetsov", "Кузнецов К.К.", "employee").await;

        for (i, status) in ["open", "in_progress", "in_repair"].iter().enumerate() {
            let r = seed_request(&ctx, author, "free_form", status).await;
            add_participant(&ctx, r, member).await;
            let callers = [
                ident(author, Role::Employee),
                ident(member, Role::Employee),
                ident(mgr, Role::Manager),
                Identity::trusted_admin(),
            ];
            for (j, who) in callers.iter().enumerate() {
                let bytes = attachments_support::png_sized(8 + i as u32, 8 + j as u32);
                let out = upload(&ctx, who, draft(r), "фото.png", &bytes)
                    .await
                    .expect("draft upload allowed");
                assert!(out.attachment.is_some(), "{status}/{j}");
            }
            let err = upload(
                &ctx,
                &ident(stranger, Role::Employee),
                draft(r),
                "фото.png",
                &attachments_support::tiny_png(),
            )
            .await
            .unwrap_err();
            assert!(matches!(err, AppError::Forbidden), "stranger {status}");
        }

        for status in ["completed", "rejected", "cancelled"] {
            let r = seed_request(&ctx, author, "free_form", status).await;
            let err = upload(
                &ctx,
                &ident(author, Role::Employee),
                draft(r),
                "фото.png",
                &attachments_support::tiny_png(),
            )
            .await
            .unwrap_err();
            assert!(matches!(err, AppError::Forbidden), "closed {status}");
        }

        let ad = seed_request(&ctx, author, "ad_register", "open").await;
        let err = upload(
            &ctx,
            &Identity::trusted_admin(),
            draft(ad),
            "фото.png",
            &attachments_support::tiny_png(),
        )
        .await
        .unwrap_err();
        assert!(matches!(err, AppError::Forbidden), "ad_register");
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn attachments_draft_no_audit_and_no_event() {
    timed!(async {
        let (ctx, _dir) = fixture::make_test_ctx().await;
        let author = seed_user(&ctx, "ivanov", "Иванов И.И.", "employee").await;
        let r = seed_request(&ctx, author, "free_form", "open").await;
        let mut rx = ctx.ws_broadcast.subscribe();
        fixture::drain(&mut rx);
        let before = audit_count(&ctx).await;

        let out = upload(
            &ctx,
            &ident(author, Role::Employee),
            draft(r),
            "фото.png",
            &attachments_support::tiny_png(),
        )
        .await
        .unwrap();
        assert!(out.attachment.is_some());
        assert_eq!(audit_count(&ctx).await, before);
        assert!(fixture::collect_events(&mut rx)
            .iter()
            .all(|e| !matches!(e, WsEvent::AttachmentsChanged { .. })));

        // Control: a plain request upload does write audit.
        upload(
            &ctx,
            &ident(author, Role::Employee),
            req(r),
            "скан.pdf",
            &attachments_support::tiny_pdf(),
        )
        .await
        .unwrap();
        assert!(audit_count(&ctx).await > before);
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn attachments_draft_hidden_from_public_paths() {
    timed!(async {
        let (ctx, _dir) = fixture::make_test_ctx().await;
        let author = seed_user(&ctx, "ivanov", "Иванов И.И.", "employee").await;
        let r = seed_request(&ctx, author, "free_form", "open").await;
        let emp = ident(author, Role::Employee);
        let admin = Identity::trusted_admin();
        let out = upload(
            &ctx,
            &emp,
            draft(r),
            "фото.png",
            &attachments_support::tiny_png(),
        )
        .await
        .unwrap();
        let id = out.attachment.unwrap().id;
        let svc = &ctx.attachments;

        for who in [&emp, &admin] {
            assert!(matches!(
                svc.list(draft(r), who).await,
                Err(AppError::Forbidden)
            ));
            assert!(matches!(
                svc.history(who, draft(r)).await,
                Err(AppError::Forbidden)
            ));
            assert!(matches!(
                svc.open_for_read(id, who, ReadKind::File).await,
                Err(AppError::Forbidden)
            ));
            assert!(matches!(
                svc.open_for_read(id, who, ReadKind::Thumb).await,
                Err(AppError::Forbidden)
            ));
            assert!(matches!(
                svc.delete(who, id).await,
                Err(AppError::Forbidden)
            ));
            assert!(matches!(
                svc.restore(who, id).await,
                Err(AppError::Forbidden)
            ));
            assert!(matches!(svc.purge(who, id).await, Err(AppError::Forbidden)));
        }

        // The request list never contains the draft.
        let l = svc.list(req(r), &admin).await.unwrap();
        assert!(l.items.is_empty() && l.deleted.is_empty());
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn attachments_draft_from_message_flag() {
    timed!(async {
        let (ctx, _dir) = fixture::make_test_ctx().await;
        let author = seed_user(&ctx, "ivanov", "Иванов И.И.", "employee").await;
        let r = seed_request(&ctx, author, "free_form", "open").await;
        let emp = ident(author, Role::Employee);
        let plain = upload(
            &ctx,
            &emp,
            req(r),
            "скан.pdf",
            &attachments_support::tiny_pdf(),
        )
        .await
        .unwrap()
        .attachment
        .unwrap();
        assert!(!plain.from_message);

        // Row with message_id set (as a promoted draft will look like).
        ctx.writer
            .execute(move |conn| {
                conn.execute(
                    "INSERT INTO chat_messages (request_id, author_user_id, body, \
                     created_at_utc) VALUES (?1, ?2, 'x', 1700000000)",
                    params![r, author],
                )
                .map_err(map_rusqlite)?;
                let mid = conn.last_insert_rowid();
                SqliteAttachmentRepository.insert_on(
                    conn,
                    &AttachmentNew {
                        parent_kind: ParentKind::Request.as_str(),
                        parent_id: r,
                        original_name: "из-чата.pdf",
                        mime: "application/pdf",
                        size_bytes: 4,
                        content_hash: &"ab".repeat(32),
                        uploaded_by_user_id: Some(author),
                        created_at_utc: 1_700_000_000,
                        message_id: Some(mid),
                    },
                )?;
                Ok(())
            })
            .await
            .expect("seed message file");
        let l = ctx.attachments.list(req(r), &emp).await.unwrap();
        assert_eq!(l.items.len(), 2);
        assert_eq!(l.items.iter().filter(|a| a.from_message).count(), 1);
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn attachments_draft_http_stranger_forbidden() {
    timed!(async {
        let (ctx, _dir) = fixture::make_test_ctx().await;
        let author = ctx
            .auth
            .create_user(
                UserNew {
                    login: "ivanov".into(),
                    full_name: "Иванов И.И.".into(),
                    password: "password123".into(),
                    role: "employee".into(),
                    email: None,
                },
                &Identity::trusted_admin(),
            )
            .await
            .unwrap()
            .id;
        let other = ctx
            .auth
            .create_user(
                UserNew {
                    login: "kuznetsov".into(),
                    full_name: "Кузнецов К.К.".into(),
                    password: "password123".into(),
                    role: "employee".into(),
                    email: None,
                },
                &Identity::trusted_admin(),
            )
            .await
            .unwrap()
            .id;
        let r = seed_request(&ctx, author, "free_form", "open").await;

        let store = RusqliteSessionStore::new(ctx.writer.clone(), ctx.readers.clone());
        let id = Id::default();
        let si = SessionIdentity {
            user_id: Some(other),
            role: "employee".into(),
        };
        let mut record = Record {
            id,
            data: Default::default(),
            expiry_date: OffsetDateTime::now_utc() + TimeDuration::days(1),
        };
        record
            .data
            .insert("identity".into(), serde_json::to_value(&si).unwrap());
        store.create(&mut record).await.unwrap();

        let boundary = "----trackly-draft";
        let mut body = Vec::new();
        body.extend_from_slice(
            format!(
                "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; \
                 filename=\"a.png\"\r\nContent-Type: application/octet-stream\r\n\r\n"
            )
            .as_bytes(),
        );
        body.extend_from_slice(&attachments_support::tiny_png());
        body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
        let res = build_router(
            &ctx,
            RusqliteSessionStore::new(ctx.writer.clone(), ctx.readers.clone()),
        )
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/v1/attachments_upload?parent_kind=message_draft&parent_id={r}"
                ))
                .header(
                    "content-type",
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .header("content-length", body.len().to_string())
                .header("cookie", format!("id={id}"))
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
        assert_eq!(res.status(), StatusCode::FORBIDDEN);
    })
}

async fn scalar(ctx: &AppCtx, sql: &'static str) -> i64 {
    ctx.writer
        .execute(move |conn| conn.query_row(sql, [], |r| r.get(0)).map_err(map_rusqlite))
        .await
        .expect("scalar")
}

/// Nyquist 41.5 G1 (MSG-01, UAT п.7): пакет «фото + .exe» в черновик сообщения.
/// `.exe` отклоняется кодом `type_not_allowed` и не оставляет строки; фото
/// принимается, уходит в сообщение, а в сообщении ровно один файл.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn attachments_draft_rejects_forbidden_type_in_batch() {
    timed!(async {
        let (ctx, _dir) = fixture::make_test_ctx().await;
        let author = seed_user(&ctx, "ivanov", "Иванов И.И.", "employee").await;
        let who = ident(author, Role::Employee);
        let r = seed_request(&ctx, author, "free_form", "open").await;

        let photo = upload(
            &ctx,
            &who,
            draft(r),
            "фото.png",
            &attachments_support::tiny_png(),
        )
        .await
        .expect("photo upload");
        let photo_id = photo.attachment.expect("photo accepted").id;
        assert!(photo.reject_code.is_none());

        let exe = upload(
            &ctx,
            &who,
            draft(r),
            "setup.exe",
            &attachments_support::pe_stub(),
        )
        .await
        .expect("exe upload is a reject, not an error");
        assert!(exe.attachment.is_none(), "exe must not be stored");
        assert_eq!(exe.reject_code.as_deref(), Some("type_not_allowed"));

        // Disguised name does not help either.
        let disguised = upload(
            &ctx,
            &who,
            draft(r),
            "фото2.png",
            &attachments_support::pe_stub(),
        )
        .await
        .expect("disguised upload is a reject");
        assert_eq!(disguised.reject_code.as_deref(), Some("type_not_allowed"));

        assert_eq!(
            scalar(
                &ctx,
                "SELECT COUNT(*) FROM attachments WHERE parent_kind = 'message_draft'"
            )
            .await,
            1,
            "only the photo is a draft"
        );

        ctx.chat
            .send(&who, r, "фото принтера", &[photo_id])
            .await
            .expect("send with the accepted photo");
        assert_eq!(scalar(&ctx, "SELECT COUNT(*) FROM chat_messages").await, 1);
        assert_eq!(
            scalar(
                &ctx,
                "SELECT COUNT(*) FROM attachments WHERE message_id IS NOT NULL"
            )
            .await,
            1,
            "the message carries exactly the accepted file"
        );
        assert_eq!(
            scalar(
                &ctx,
                "SELECT COUNT(*) FROM attachments WHERE parent_kind = 'message_draft'"
            )
            .await,
            0,
            "the draft was promoted"
        );
    })
}
