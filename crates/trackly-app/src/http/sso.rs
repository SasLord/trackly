//! Spike 002/003 — AD SSO (Kerberos/SPNEGO) HTTP endpoint.
//!
//! `GET /api/v1/auth_ad_sso` performs the server side of the browser Negotiate handshake,
//! mirroring adwebapp's `/auth/ad`:
//!   1. No `Authorization` header → `401 WWW-Authenticate: Negotiate` (challenge). A
//!      domain-joined browser then transparently resends the request with a Kerberos
//!      ticket — no username/password prompt.
//!   2. `Authorization: Negotiate <base64>` → validate the ticket against the service
//!      keytab (`trackly_infra::ad::sso::accept_spnego`, offline, no KDC). On success we
//!      resolve the AD account to a Trackly user (`AuthService::sso_login`) and issue the
//!      SAME session cookie as password login.
//!
//! Server-mode / LAN-browser only. Additive and gated: with `ad.sso_enabled = false`
//! (the default) or an unset SPN/keytab, it returns 503 and nothing else changes — the
//! existing login/password + LDAPS-bind paths are untouched.
//!
//! ⚠️ BUILD-VERIFIED, live handshake is real-AD-only (see `trackly_infra::ad::sso`). The
//! endpoint shape, config gating, base64/header handling and session issuance are exercised
//! by build + the acceptor's guard tests; the actual Kerberos exchange is tomorrow's AD test.

use axum::{
    extract::State,
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use base64::Engine;
use serde_json::json;
use tower_sessions::Session;

use trackly_core::auth::{Identity, Role};
use trackly_core::error::AppError;
use trackly_infra::ad::sso::{accept_spnego, SsoOutcome};

use crate::context::AppCtx;
use crate::error_axum::AppErrorResponse;
use crate::http::auth::SessionIdentity;

const NEGOTIATE_PREFIX: &str = "Negotiate ";

fn b64() -> base64::engine::general_purpose::GeneralPurpose {
    base64::engine::general_purpose::STANDARD
}

/// Build a `401 + WWW-Authenticate: Negotiate [<base64 reply>]` challenge response.
fn negotiate_challenge(reply: Option<&[u8]>) -> Response {
    let header_value = match reply {
        Some(r) if !r.is_empty() => format!("Negotiate {}", b64().encode(r)),
        _ => "Negotiate".to_string(),
    };
    let mut resp = (
        StatusCode::UNAUTHORIZED,
        Json(json!({ "error": "Требуется вход через Active Directory" })),
    )
        .into_response();
    if let Ok(hv) = HeaderValue::from_str(&header_value) {
        resp.headers_mut().insert(header::WWW_AUTHENTICATE, hv);
    }
    resp
}

/// After a successful accept, resolve the AD account to a Trackly user and issue the same
/// session cookie as password login (T-05-SF: flush before insert). Returns the display
/// name for the JSON body. Mirrors `build_auth_login`'s session handling.
async fn issue_sso_session(
    ctx: &AppCtx,
    session: &Session,
    ad_username: &str,
) -> Result<String, AppError> {
    let user = ctx.auth.sso_login(ad_username, ad_username).await?;

    session.flush().await.map_err(|e| AppError::Internal {
        source_chain: format!("session flush (sso): {e}"),
    })?;

    let role = Role::from_str(&user.role)?;
    let identity = Identity {
        user_id: Some(user.id),
        role,
    };
    session
        .insert("identity", SessionIdentity::from(&identity))
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("session insert (sso): {e}"),
        })?;
    // AD SSO happens on a domain machine the user trusts → persistent sliding cookie,
    // same as password login with "remember me".
    session.set_expiry(Some(tower_sessions::Expiry::OnInactivity(
        time::Duration::days(30),
    )));

    Ok(user.full_name)
}

/// `GET /api/v1/auth_ad_sso` — the SPNEGO/Negotiate handshake endpoint.
pub async fn handler_ad_sso(
    State(ctx): State<AppCtx>,
    session: Session,
    headers: HeaderMap,
) -> Response {
    let ad = &ctx.config.ad;

    // Gate: SSO must be explicitly enabled and configured. Default off ⇒ 503, nothing else.
    if !ad.sso_enabled || ad.spn.is_empty() || ad.keytab_path.is_empty() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Вход через Active Directory недоступен на этом сервере" })),
        )
            .into_response();
    }

    // Extract the Negotiate token, or challenge if absent.
    let auth_header = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok());
    let token_b64 = match auth_header {
        Some(h) if h.len() > NEGOTIATE_PREFIX.len()
            && h[..NEGOTIATE_PREFIX.len()].eq_ignore_ascii_case(NEGOTIATE_PREFIX) =>
        {
            h[NEGOTIATE_PREFIX.len()..].trim()
        }
        // No/foreign auth scheme → send the Negotiate challenge (browser retries with a ticket).
        _ => return negotiate_challenge(None),
    };

    let token = match b64().decode(token_b64) {
        Ok(t) => t,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "Некорректный Negotiate-токен" })),
            )
                .into_response()
        }
    };

    // Read the service keytab (bytes never logged). Missing/unreadable ⇒ 503.
    let keytab_bytes = match std::fs::read(&ad.keytab_path) {
        Ok(b) => b,
        Err(e) => {
            tracing::error!("AD SSO: не удалось прочитать keytab {}: {e}", ad.keytab_path);
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({ "error": "Файл keytab недоступен на сервере" })),
            )
                .into_response();
        }
    };

    let client_computer_name = hostname::get()
        .ok()
        .and_then(|h| h.into_string().ok())
        .unwrap_or_else(|| "TRACKLY".to_string());

    match accept_spnego(&ad.spn, &keytab_bytes, &client_computer_name, &token) {
        Ok(SsoOutcome::Authenticated {
            username,
            reply_token,
        }) => match issue_sso_session(&ctx, &session, &username).await {
            Ok(full_name) => {
                let mut resp = (
                    StatusCode::OK,
                    Json(json!({ "ok": true, "fullName": full_name })),
                )
                    .into_response();
                if !reply_token.is_empty() {
                    if let Ok(hv) =
                        HeaderValue::from_str(&format!("Negotiate {}", b64().encode(&reply_token)))
                    {
                        resp.headers_mut().insert(header::WWW_AUTHENTICATE, hv);
                    }
                }
                resp
            }
            // Pending/blocked/etc. — surface the same typed error password AD login returns.
            Err(e) => AppErrorResponse::from(e).into_response(),
        },
        // Kerberos usually completes in one step; a continuation just re-challenges.
        Ok(SsoOutcome::Continue { reply_token }) => negotiate_challenge(Some(&reply_token)),
        Ok(SsoOutcome::Denied) => (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "Не удалось подтвердить вход через Active Directory" })),
        )
            .into_response(),
        Err(e) => {
            // Do not leak internals (bad SPN, keytab mismatch, crypto detail) to the client.
            tracing::warn!("AD SSO accept failed: {e}");
            (
                StatusCode::UNAUTHORIZED,
                Json(json!({ "error": "Не удалось подтвердить вход через Active Directory" })),
            )
                .into_response()
        }
    }
}

/// SSO router — public (establishes the session), merged into the API router in `http::mod`.
pub fn router() -> Router<AppCtx> {
    Router::new().route("/api/v1/auth_ad_sso", get(handler_ad_sso))
}
