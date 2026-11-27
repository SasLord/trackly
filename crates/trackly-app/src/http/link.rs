//! Публичные маршруты Trackly Link (Phase 44-11): enrollment, вход паролем,
//! автовход SSO, возобновление, выход.
//!
//! Маршруты стоят ВНЕ защиты сессией (вход ещё не выполнен), поэтому каждый вход
//! сначала проверяет устройство по заголовкам `x-link-device` / `x-link-secret`
//! (D-13) и только потом смотрит пароль или токен Kerberos. Тела ошибок — всегда
//! JSON `{ code, message }` без деталей Kerberos, keytab и БД. Пароли, токены и
//! заголовок `Authorization` в журнал не пишутся.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::{ConnectInfo, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Extension, Json, Router};
use link_proto::{
    EnrollRequest, EnrollResponse, LinkAuthResponse, LinkError, LinkLoginRequest, LinkMethod,
    LinkUser, ResumeRequest, HEADER_DEVICE, HEADER_SECRET, LINK_BAD_CREDENTIALS,
    LINK_DEVICE_BANNED, LINK_INVITE_EXPIRED, LINK_INVITE_INVALID, LINK_INVITE_USED, LINK_REVOKED,
    LINK_SSO_DENIED, LINK_SSO_DISABLED, LINK_USER_BLOCKED,
};
use tower_sessions::{Expiry, Session};
use trackly_core::error::AppError;

use crate::context::AppCtx;
use crate::dto::auth::{LoginRequest, UserDto};
use crate::http::auth::establish_session;
use crate::http::sso::{negotiate_header, negotiate_identity, SsoFailure};
use crate::services::link_service::{
    DeviceError, DeviceMeta, EnrollError, LoginMethod, ResumeError,
};

/// Ключ сессии: устройство Link, с которого выполнен вход.
pub const SESSION_DEVICE_KEY: &str = "link_device_id";
/// Ключ сессии: запись входа Link (для выхода и отзыва).
pub const SESSION_LOGIN_KEY: &str = "link_login_id";

const LINK_BAD_REQUEST: &str = "LINK_BAD_REQUEST";
const LINK_DEVICE_UNKNOWN: &str = "LINK_DEVICE_UNKNOWN";
const LINK_DEVICE_EXISTS: &str = "LINK_DEVICE_EXISTS";
const LINK_SSO_CHALLENGE: &str = "LINK_SSO_CHALLENGE";
const LINK_NO_SESSION: &str = "LINK_NO_SESSION";
const LINK_INTERNAL: &str = "LINK_INTERNAL";

const MAX_META_LEN: usize = 128;

// ---------------------------------------------------------------------------
// Ответы
// ---------------------------------------------------------------------------

pub(crate) fn link_err(status: StatusCode, code: &str, message: &str) -> Response {
    (
        status,
        Json(LinkError {
            code: code.to_string(),
            message: message.to_string(),
        }),
    )
        .into_response()
}

fn internal_err(e: &AppError) -> Response {
    tracing::error!("Link: внутренняя ошибка: {e}");
    link_err(
        StatusCode::INTERNAL_SERVER_ERROR,
        LINK_INTERNAL,
        "Внутренняя ошибка сервера",
    )
}

/// Ошибка входа/сопоставления учётки → ответ Link.
fn app_err_to_link(e: &AppError) -> Response {
    match e {
        AppError::Unauthorized => link_err(
            StatusCode::UNAUTHORIZED,
            LINK_BAD_CREDENTIALS,
            "Неверный логин или пароль",
        ),
        AppError::AccessBlocked { .. }
        | AppError::RegistrationPending { .. }
        | AppError::Forbidden => link_err(
            StatusCode::FORBIDDEN,
            LINK_USER_BLOCKED,
            "Доступ пользователя закрыт",
        ),
        other => internal_err(other),
    }
}

fn device_err(e: DeviceError) -> Response {
    match e {
        DeviceError::Banned => link_err(
            StatusCode::FORBIDDEN,
            LINK_DEVICE_BANNED,
            "Устройству запрещён вход",
        ),
        // Неизвестное устройство и неверный секрет неразличимы (без утечки причины).
        DeviceError::Unknown | DeviceError::BadSecret => link_err(
            StatusCode::UNAUTHORIZED,
            LINK_DEVICE_UNKNOWN,
            "Устройство не распознано",
        ),
        DeviceError::Internal(a) => internal_err(&a),
    }
}

fn parse_body<T: serde::de::DeserializeOwned>(body: &Bytes) -> Result<T, Box<Response>> {
    serde_json::from_slice(body).map_err(|_| {
        Box::new(link_err(
            StatusCode::BAD_REQUEST,
            LINK_BAD_REQUEST,
            "Некорректное тело запроса",
        ))
    })
}

fn peer_ip(ci: &Option<Extension<ConnectInfo<SocketAddr>>>) -> Option<String> {
    ci.as_ref().map(|Extension(c)| c.0.ip().to_string())
}

// ---------------------------------------------------------------------------
// Проверка устройства (D-13) — ПЕРЕД паролем / токеном
// ---------------------------------------------------------------------------

/// Заголовки устройства → `device_id`; ошибка — готовый ответ.
async fn verify_device_headers(
    ctx: &AppCtx,
    headers: &HeaderMap,
    ip: Option<&str>,
) -> Result<String, Box<Response>> {
    let get = |name: &str| {
        headers
            .get(name)
            .and_then(|v| v.to_str().ok())
            .map(str::trim)
            .filter(|v| !v.is_empty())
    };
    let (Some(device_id), Some(secret)) = (get(HEADER_DEVICE), get(HEADER_SECRET)) else {
        return Err(Box::new(device_err(DeviceError::Unknown)));
    };
    ctx.link
        .verify_device_from(device_id, secret, ip)
        .await
        .map_err(|e| Box::new(device_err(e)))?;
    Ok(device_id.to_string())
}

/// Сессия для устройства Link: identity + привязка к устройству и входу.
async fn open_link_session(
    session: &Session,
    user: &UserDto,
    device_id: &str,
    login_id: i64,
) -> Result<(), AppError> {
    // flush → insert identity → expiry (T-05-SF) — внутри establish_session.
    establish_session(
        session,
        user,
        Expiry::OnInactivity(time::Duration::days(30)),
    )
    .await?;
    for (k, v) in [(SESSION_DEVICE_KEY, device_id.to_string())] {
        session.insert(k, v).await.map_err(|e| AppError::Internal {
            source_chain: format!("session insert (link device): {e}"),
        })?;
    }
    session
        .insert(SESSION_LOGIN_KEY, login_id)
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("session insert (link login): {e}"),
        })?;
    Ok(())
}

fn auth_response(user: &UserDto, method: LinkMethod, device_token: Option<String>) -> Response {
    (
        StatusCode::OK,
        Json(LinkAuthResponse {
            user: LinkUser {
                id: user.id,
                login: user.login.clone(),
                display_name: user.full_name.clone(),
            },
            method,
            device_token,
        }),
    )
        .into_response()
}

// ---------------------------------------------------------------------------
// link_enroll
// ---------------------------------------------------------------------------

fn valid_meta(r: &EnrollRequest) -> bool {
    uuid::Uuid::parse_str(&r.device_id).is_ok()
        && [&r.hostname, &r.os, &r.arch, &r.client_version]
            .iter()
            .all(|s| s.chars().count() <= MAX_META_LEN)
}

/// `POST /api/v1/link_enroll` — погашение кода; только `deviceSecret`, без сессии (D-04).
pub async fn handler_enroll(
    State(ctx): State<AppCtx>,
    connect_info: Option<Extension<ConnectInfo<SocketAddr>>>,
    body: Bytes,
) -> Response {
    let req: EnrollRequest = match parse_body(&body) {
        Ok(r) => r,
        Err(resp) => return *resp,
    };
    if !valid_meta(&req) {
        return link_err(
            StatusCode::BAD_REQUEST,
            LINK_BAD_REQUEST,
            "Некорректные данные устройства",
        );
    }
    let meta = DeviceMeta {
        device_id: req.device_id.clone(),
        hostname: req.hostname.clone(),
        os: req.os.clone(),
        arch: req.arch.clone(),
        client_version: req.client_version.clone(),
        ip: peer_ip(&connect_info),
    };
    match ctx.link.enroll(&req.secret, meta).await {
        Ok(device_secret) => {
            (StatusCode::OK, Json(EnrollResponse { device_secret })).into_response()
        }
        Err(EnrollError::InviteInvalid) | Err(EnrollError::InviteRevoked) => link_err(
            StatusCode::BAD_REQUEST,
            LINK_INVITE_INVALID,
            "Код не распознан",
        ),
        Err(EnrollError::InviteExpired) => link_err(
            StatusCode::GONE,
            LINK_INVITE_EXPIRED,
            "Срок действия кода истёк",
        ),
        Err(EnrollError::InviteUsed) => link_err(
            StatusCode::CONFLICT,
            LINK_INVITE_USED,
            "Код уже использован",
        ),
        Err(EnrollError::DeviceExists) => link_err(
            StatusCode::CONFLICT,
            LINK_DEVICE_EXISTS,
            "Устройство уже зарегистрировано",
        ),
        Err(EnrollError::Internal(AppError::Validation { .. })) => link_err(
            StatusCode::BAD_REQUEST,
            LINK_BAD_REQUEST,
            "Некорректные данные устройства",
        ),
        Err(EnrollError::Internal(e)) => internal_err(&e),
    }
}

// ---------------------------------------------------------------------------
// link_auth_login
// ---------------------------------------------------------------------------

/// `POST /api/v1/link_auth_login` — вход паролем; выдаёт `deviceToken` (D-10).
pub async fn handler_login(
    State(ctx): State<AppCtx>,
    session: Session,
    connect_info: Option<Extension<ConnectInfo<SocketAddr>>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    // D-13: устройство — раньше пароля.
    let device_id =
        match verify_device_headers(&ctx, &headers, peer_ip(&connect_info).as_deref()).await {
            Ok(d) => d,
            Err(resp) => return *resp,
        };
    let req: LinkLoginRequest = match parse_body(&body) {
        Ok(r) => r,
        Err(resp) => return *resp,
    };
    let user = match ctx
        .auth
        .login(LoginRequest {
            login: req.login.clone(),
            password: req.password.clone(),
            remember: true,
        })
        .await
    {
        Ok(u) => u,
        Err(e) => return app_err_to_link(&e),
    };
    let issued = match ctx
        .link
        .record_login(&device_id, user.id, LoginMethod::Password)
        .await
    {
        Ok(i) => i,
        Err(e) => return internal_err(&e),
    };
    if let Err(e) = open_link_session(&session, &user, &device_id, issued.login_id).await {
        return internal_err(&e);
    }
    auth_response(&user, LinkMethod::Password, issued.device_token)
}

// ---------------------------------------------------------------------------
// link_auth_sso
// ---------------------------------------------------------------------------

fn sso_challenge(reply: Option<&[u8]>) -> Response {
    let mut resp = link_err(
        StatusCode::UNAUTHORIZED,
        LINK_SSO_CHALLENGE,
        "Требуется вход через Active Directory",
    );
    if let Some(hv) = negotiate_header(reply) {
        resp.headers_mut().insert(header::WWW_AUTHENTICATE, hv);
    }
    resp
}

/// Завершение SSO-входа после проверенного билета: сопоставление учётки,
/// запись входа, сессия. Вынесено, чтобы тестироваться без настоящего Kerberos.
pub async fn complete_sso_login(
    ctx: &AppCtx,
    session: &Session,
    device_id: &str,
    ad_username: &str,
    reply_token: &[u8],
) -> Response {
    let user = match ctx.auth.sso_login(ad_username, ad_username).await {
        Ok(u) => u,
        Err(AppError::Unauthorized) => {
            return link_err(
                StatusCode::UNAUTHORIZED,
                LINK_SSO_DENIED,
                "Не удалось подтвердить вход через Active Directory",
            )
        }
        Err(e) => return app_err_to_link(&e),
    };
    if !user.is_active {
        return link_err(
            StatusCode::FORBIDDEN,
            LINK_USER_BLOCKED,
            "Доступ пользователя закрыт",
        );
    }
    let issued = match ctx
        .link
        .record_login(device_id, user.id, LoginMethod::Sso)
        .await
    {
        Ok(i) => i,
        Err(e) => return internal_err(&e),
    };
    if let Err(e) = open_link_session(session, &user, device_id, issued.login_id).await {
        return internal_err(&e);
    }
    let mut resp = auth_response(&user, LinkMethod::Sso, None);
    if !reply_token.is_empty() {
        if let Some(hv) = negotiate_header(Some(reply_token)) {
            resp.headers_mut().insert(header::WWW_AUTHENTICATE, hv);
        }
    }
    resp
}

/// `GET /api/v1/link_auth_sso` — автовход по Kerberos (D-09).
pub async fn handler_sso(
    State(ctx): State<AppCtx>,
    session: Session,
    connect_info: Option<Extension<ConnectInfo<SocketAddr>>>,
    headers: HeaderMap,
) -> Response {
    // D-13: устройство — раньше токена Kerberos.
    let device_id =
        match verify_device_headers(&ctx, &headers, peer_ip(&connect_info).as_deref()).await {
            Ok(d) => d,
            Err(resp) => return *resp,
        };
    match negotiate_identity(&ctx, &headers).await {
        Ok(n) => complete_sso_login(&ctx, &session, &device_id, &n.username, &n.reply_token).await,
        Err(SsoFailure::Disabled) | Err(SsoFailure::KeytabUnavailable) => link_err(
            StatusCode::SERVICE_UNAVAILABLE,
            LINK_SSO_DISABLED,
            "Вход через Active Directory недоступен на этом сервере",
        ),
        Err(SsoFailure::Challenge(reply)) => sso_challenge(reply.as_deref()),
        Err(SsoFailure::BadToken) | Err(SsoFailure::Denied) => link_err(
            StatusCode::UNAUTHORIZED,
            LINK_SSO_DENIED,
            "Не удалось подтвердить вход через Active Directory",
        ),
    }
}

// ---------------------------------------------------------------------------
// link_resume
// ---------------------------------------------------------------------------

/// `POST /api/v1/link_resume` — вход по токену устройства без пароля.
pub async fn handler_resume(
    State(ctx): State<AppCtx>,
    session: Session,
    connect_info: Option<Extension<ConnectInfo<SocketAddr>>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let device_id =
        match verify_device_headers(&ctx, &headers, peer_ip(&connect_info).as_deref()).await {
            Ok(d) => d,
            Err(resp) => return *resp,
        };
    let req: ResumeRequest = match parse_body(&body) {
        Ok(r) => r,
        Err(resp) => return *resp,
    };
    let row = match ctx
        .link
        .resume_by_token(&req.device_token, &device_id)
        .await
    {
        Ok(r) => r,
        Err(ResumeError::Invalid) => {
            return link_err(
                StatusCode::UNAUTHORIZED,
                LINK_BAD_CREDENTIALS,
                "Токен входа недействителен",
            )
        }
        Err(ResumeError::Revoked) => {
            return link_err(
                StatusCode::UNAUTHORIZED,
                LINK_REVOKED,
                "Вход отозван, требуется повторный вход",
            )
        }
        Err(ResumeError::DeviceBanned) => {
            return link_err(
                StatusCode::FORBIDDEN,
                LINK_DEVICE_BANNED,
                "Устройству запрещён вход",
            )
        }
        Err(ResumeError::UserInactive) => {
            return link_err(
                StatusCode::FORBIDDEN,
                LINK_USER_BLOCKED,
                "Доступ пользователя закрыт",
            )
        }
        Err(ResumeError::Internal(e)) => return internal_err(&e),
    };
    let user = match ctx.auth.get_user_by_id(row.user_id).await {
        Ok(u) => u,
        Err(e) => return internal_err(&e),
    };
    if !user.is_active {
        return link_err(
            StatusCode::FORBIDDEN,
            LINK_USER_BLOCKED,
            "Доступ пользователя закрыт",
        );
    }
    if let Err(e) = open_link_session(&session, &user, &row.device_id, row.login_id).await {
        return internal_err(&e);
    }
    let method = if row.method == "sso" {
        LinkMethod::Sso
    } else {
        LinkMethod::Password
    };
    auth_response(&user, method, None)
}

// ---------------------------------------------------------------------------
// link_logout
// ---------------------------------------------------------------------------

/// `POST /api/v1/link_logout` — отзыв токена входа (устройство НЕ банится, D-11).
pub async fn handler_logout(State(ctx): State<AppCtx>, session: Session) -> Response {
    let login_id = match session.get::<i64>(SESSION_LOGIN_KEY).await {
        Ok(Some(id)) => id,
        _ => {
            return link_err(
                StatusCode::UNAUTHORIZED,
                LINK_NO_SESSION,
                "Нет активного входа Link",
            )
        }
    };
    if let Err(e) = ctx.link.logout(login_id).await {
        return internal_err(&e);
    }
    if let Err(e) = session.flush().await {
        tracing::error!("Link: не удалось сбросить сессию при выходе: {e}");
        return link_err(
            StatusCode::INTERNAL_SERVER_ERROR,
            LINK_INTERNAL,
            "Внутренняя ошибка сервера",
        );
    }
    (StatusCode::OK, Json(serde_json::json!({ "ok": true }))).into_response()
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

/// Свой rate limit (1 запрос/с, burst 5, по IP пира) — корзина не общая с auth_login.
macro_rules! own_governor {
    () => {
        tower_governor::GovernorLayer::new(Arc::new(
            tower_governor::governor::GovernorConfigBuilder::default()
                .per_second(1)
                .burst_size(5)
                .finish()
                .expect("governor config build failed"),
        ))
    };
}

/// Публичный router Link; сливается в `build_router` вне защищённых маршрутов.
pub fn router() -> Router<AppCtx> {
    // Свои корзины rate limit у enroll и у входа паролем: не делим с auth_login.
    let enroll = Router::new()
        .route("/api/v1/link_enroll", post(handler_enroll))
        .route_layer(own_governor!());
    let login = Router::new()
        .route("/api/v1/link_auth_login", post(handler_login))
        .route_layer(own_governor!());
    Router::new()
        .merge(enroll)
        .merge(login)
        .route("/api/v1/link_auth_sso", get(handler_sso))
        .route("/api/v1/link_resume", post(handler_resume))
        .route("/api/v1/link_logout", post(handler_logout))
}
