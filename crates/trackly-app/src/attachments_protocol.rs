//! `trackly-att` URI scheme — desktop file/thumbnail serving (Phase 41.3 Plan 10).
//!
//! URL shape: `trackly-att://localhost/<id>/file` or `/<id>/thumb`
//! (Windows webview: `http://trackly-att.localhost/...`; only the path is used).
//!
//! The handler calls `AttachmentService::open_for_read` with the identity from
//! `resolve_tauri_identity` — the same single access check the HTTP route uses
//! (D-04). The disk path is never taken from the URI: the URI yields only an
//! integer id and a whitelisted kind; the path is derived from the DB hash.

use percent_encoding::{utf8_percent_encode, AsciiSet, NON_ALPHANUMERIC};
use tauri::http::{header, Response, StatusCode};

use crate::context::AppCtx;
use crate::services::attachment_service::{OpenedAttachment, ReadKind};
use crate::tauri_cmds::users::resolve_tauri_identity;
use trackly_core::error::AppError;

/// Upper bound of a file served through the scheme.
const MAX_SERVE_BYTES: u64 = 100 * 1024 * 1024;

const FILENAME_STAR: &AsciiSet = &NON_ALPHANUMERIC.remove(b'.').remove(b'-').remove(b'_');

/// Strict parser: `/<digits>/file` or `/<digits>/thumb` and nothing else.
#[allow(clippy::result_unit_err)] // any malformed path is simply "not found"
pub fn parse_uri(path: &str) -> Result<(i64, ReadKind), ()> {
    let rest = path.strip_prefix('/').ok_or(())?;
    let mut parts = rest.split('/');
    let id_s = parts.next().ok_or(())?;
    let kind_s = parts.next().ok_or(())?;
    if parts.next().is_some() {
        return Err(());
    }
    if id_s.is_empty() || id_s.len() > 18 || !id_s.bytes().all(|b| b.is_ascii_digit()) {
        return Err(());
    }
    let id: i64 = id_s.parse().map_err(|_| ())?;
    let kind = match kind_s {
        "file" => ReadKind::File,
        "thumb" => ReadKind::Thumb,
        _ => return Err(()),
    };
    Ok((id, kind))
}

/// `Content-Disposition` with a header-injection-safe ASCII fallback and an
/// RFC 5987 `filename*`.
pub fn content_disposition(name: &str, inline: bool) -> String {
    let fallback: String = name
        .chars()
        .filter(|c| !matches!(c, '\r' | '\n' | '"' | '\\' | ';'))
        .map(|c| {
            if c.is_ascii() && !c.is_control() {
                c
            } else {
                '_'
            }
        })
        .collect();
    let fallback = if fallback.is_empty() {
        "file".to_string()
    } else {
        fallback
    };
    let star: String = utf8_percent_encode(name, FILENAME_STAR).to_string();
    format!(
        "{}; filename=\"{}\"; filename*=UTF-8''{}",
        if inline { "inline" } else { "attachment" },
        fallback,
        star
    )
}

/// Safe header value: the DB mime is validated against a conservative charset.
pub fn safe_mime(mime: &str) -> &str {
    if !mime.is_empty()
        && mime
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'/' | b'.' | b'+' | b'-'))
    {
        mime
    } else {
        "application/octet-stream"
    }
}

/// Response for an opened attachment (`thumb` = true -> thumbnail, cacheable).
pub fn build_response(opened: &OpenedAttachment, bytes: Vec<u8>, thumb: bool) -> Response<Vec<u8>> {
    let disposition = if thumb {
        "inline".to_string()
    } else {
        content_disposition(&opened.original_name, opened.inline)
    };
    let cache = if thumb {
        "private, max-age=3600"
    } else {
        "private, no-store"
    };
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, safe_mime(&opened.mime))
        .header("X-Content-Type-Options", "nosniff")
        .header("Access-Control-Allow-Origin", "*")
        .header(header::CONTENT_DISPOSITION, disposition)
        .header(header::CACHE_CONTROL, cache)
        .body(bytes)
        .unwrap_or_else(|_| status_response(StatusCode::INTERNAL_SERVER_ERROR))
}

/// Empty-body response (no stack text leaks to the webview).
pub fn status_response(status: StatusCode) -> Response<Vec<u8>> {
    Response::builder()
        .status(status)
        .header("X-Content-Type-Options", "nosniff")
        .header("Access-Control-Allow-Origin", "*")
        .body(Vec::new())
        .expect("static response")
}

/// Service error -> status. NotFound -> 404, Forbidden -> 403, rest -> 500.
pub fn error_response(e: &AppError) -> Response<Vec<u8>> {
    let status = match e {
        AppError::NotFound { .. } => StatusCode::NOT_FOUND,
        AppError::Forbidden | AppError::Unauthorized => StatusCode::FORBIDDEN,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };
    status_response(status)
}

async fn serve(ctx: &AppCtx, path: &str) -> Response<Vec<u8>> {
    let Ok((id, kind)) = parse_uri(path) else {
        return status_response(StatusCode::NOT_FOUND);
    };
    let identity = match resolve_tauri_identity(ctx).await {
        Ok(i) => i,
        Err(e) => return error_response(&e),
    };
    let opened = match ctx.attachments.open_for_read(id, &identity, kind).await {
        Ok(o) => o,
        Err(e) => return error_response(&e),
    };
    let path = opened.path.clone();
    let read = tokio::task::spawn_blocking(move || -> std::io::Result<Vec<u8>> {
        let meta = std::fs::metadata(&path)?;
        if meta.len() > MAX_SERVE_BYTES {
            return Err(std::io::Error::other("too large"));
        }
        std::fs::read(&path)
    })
    .await;
    match read {
        Ok(Ok(bytes)) => build_response(&opened, bytes, matches!(kind, ReadKind::Thumb)),
        Ok(Err(e)) if e.kind() == std::io::ErrorKind::NotFound => {
            status_response(StatusCode::NOT_FOUND)
        }
        _ => status_response(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// Entry point registered in `main.rs`.
pub fn handle(
    ctx: AppCtx,
    request: tauri::http::Request<Vec<u8>>,
    responder: tauri::UriSchemeResponder,
) {
    let path = request.uri().path().to_string();
    tauri::async_runtime::spawn(async move {
        responder.respond(serve(&ctx, &path).await);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn opened(mime: &str, name: &str, inline: bool) -> OpenedAttachment {
        OpenedAttachment {
            path: PathBuf::from("/nonexistent"),
            mime: mime.to_string(),
            original_name: name.to_string(),
            size: 3,
            inline,
        }
    }

    #[test]
    fn parse_uri_accepts_only_id_and_kind() {
        assert!(matches!(parse_uri("/12/thumb"), Ok((12, ReadKind::Thumb))));
        assert!(matches!(parse_uri("/12/file"), Ok((12, ReadKind::File))));
        for bad in [
            "/x/file",
            "/12/other",
            "/12",
            "/../12/file",
            "",
            "/12/file/",
            "/12/file/x",
            "//file",
            "/-1/file",
            "/12%2F../file",
            "/99999999999999999999/file",
        ] {
            assert!(parse_uri(bad).is_err(), "must reject {bad:?}");
        }
    }

    #[test]
    fn response_headers_are_safe() {
        let o = opened("image/png", "Фото.png", true);
        let r = build_response(&o, vec![1, 2, 3], false);
        assert_eq!(r.status(), StatusCode::OK);
        assert_eq!(r.headers()[header::CONTENT_TYPE], "image/png");
        assert_eq!(r.headers()["X-Content-Type-Options"], "nosniff");
        assert_eq!(r.headers()["Access-Control-Allow-Origin"], "*");
        assert_eq!(r.headers()[header::CACHE_CONTROL], "private, no-store");
        let cd = r.headers()[header::CONTENT_DISPOSITION].to_str().unwrap();
        assert!(cd.starts_with("inline;"), "{cd}");
        assert!(cd.contains("filename*=UTF-8''"), "{cd}");
        assert_eq!(r.body(), &vec![1, 2, 3]);
    }

    #[test]
    fn non_inline_is_attachment_and_thumb_is_cacheable() {
        let o = opened("application/zip", "a.zip", false);
        let r = build_response(&o, vec![], false);
        let cd = r.headers()[header::CONTENT_DISPOSITION].to_str().unwrap();
        assert!(cd.starts_with("attachment;"), "{cd}");
        let t = build_response(&opened("image/jpeg", "a.jpg", false), vec![], true);
        assert!(t.headers()[header::CACHE_CONTROL]
            .to_str()
            .unwrap()
            .contains("max-age"));
    }

    #[test]
    fn hostile_mime_falls_back() {
        let o = opened("text/html\r\nSet-Cookie: x=1", "a", false);
        let r = build_response(&o, vec![], false);
        assert_eq!(
            r.headers()[header::CONTENT_TYPE],
            "application/octet-stream"
        );
    }

    #[test]
    fn content_disposition_strips_injection_chars() {
        let cd = content_disposition("a\r\nX: y\"\\b;.pdf", false);
        assert!(!cd.contains('\r') && !cd.contains('\n'), "{cd:?}");
        let fallback = cd
            .split("filename=\"")
            .nth(1)
            .unwrap()
            .split('"')
            .next()
            .unwrap();
        assert!(!fallback.contains('\\') && !fallback.contains(';'), "{cd}");
        assert!(cd.contains("filename*=UTF-8''"));
        assert!(content_disposition("", true).contains("filename=\"file\""));
    }

    #[test]
    fn service_errors_map_to_empty_statuses() {
        let nf = error_response(&AppError::NotFound {
            entity: "attachment",
            id: 1,
        });
        assert_eq!(nf.status(), StatusCode::NOT_FOUND);
        assert!(nf.body().is_empty());
        let fb = error_response(&AppError::Forbidden);
        assert_eq!(fb.status(), StatusCode::FORBIDDEN);
        assert!(fb.body().is_empty());
        let ie = error_response(&AppError::Internal {
            source_chain: "secret stack".into(),
        });
        assert_eq!(ie.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert!(ie.body().is_empty());
    }
}
