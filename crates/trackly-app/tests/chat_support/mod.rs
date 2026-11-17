//! Helpers for the chat suite (Phase 41.5). Fictional names only.
#![allow(dead_code)]

use rusqlite::params;
use trackly_app::context::AppCtx;
use trackly_app::dto::attachment::AttachmentUploadDto;
use trackly_app::services::attachment_service::{ParentRef, StageError};
use trackly_core::auth::{Identity, Role};
use trackly_core::error::AppError;
use trackly_infra::error_conversions::map_rusqlite;
use trackly_infra::repos::chat_sqlite::{ChatMessageNew, SqliteChatRepository};

pub fn ident(id: i64, role: Role) -> Identity {
    Identity {
        user_id: Some(id),
        role,
    }
}

pub async fn seed_user(ctx: &AppCtx, login: &str, full: &str, role: &str) -> i64 {
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

pub async fn seed_request(ctx: &AppCtx, author: i64, rtype: &str, status: &str) -> i64 {
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

pub async fn set_resolution(ctx: &AppCtx, request_id: i64, notes: &str) {
    let notes = notes.to_string();
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "UPDATE requests SET resolution_notes = ?1 WHERE id = ?2",
                params![notes, request_id],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("resolution");
}

pub async fn soft_delete_request(ctx: &AppCtx, request_id: i64) {
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "UPDATE requests SET deleted_at_utc = 1700000500 WHERE id = ?1",
                params![request_id],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("soft delete");
}

pub async fn add_participant(ctx: &AppCtx, request_id: i64, user_id: i64) {
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

/// Inserts an audit row of a request; returns its id.
pub async fn audit(
    ctx: &AppCtx,
    request_id: i64,
    action: &str,
    user: Option<i64>,
    payload: Option<&str>,
    at: i64,
) -> i64 {
    let (action, payload) = (action.to_string(), payload.map(str::to_string));
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO audit_log (entity_type, entity_id, action, user_id, payload_json, \
                 created_at_utc) VALUES ('request', ?1, ?2, ?3, ?4, ?5)",
                params![request_id, action, user, payload, at],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("audit")
}

/// Inserts a chat message through the repository; returns its id.
pub async fn message(
    ctx: &AppCtx,
    request_id: i64,
    author: Option<i64>,
    body: &str,
    at: i64,
    moved_from: Option<i64>,
) -> i64 {
    let body = body.to_string();
    ctx.writer
        .execute(move |conn| {
            SqliteChatRepository.insert_message_on(
                conn,
                &ChatMessageNew {
                    request_id,
                    author_user_id: author,
                    body: &body,
                    created_at_utc: at,
                    moved_from_request_id: moved_from,
                },
            )
        })
        .await
        .expect("message")
}

pub async fn count(ctx: &AppCtx, sql: &'static str) -> i64 {
    ctx.writer
        .execute(move |conn| conn.query_row(sql, [], |r| r.get(0)).map_err(map_rusqlite))
        .await
        .expect("count")
}

pub async fn upload(
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
