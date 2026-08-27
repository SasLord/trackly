//! Auth HTTP routes — Plan 03.
//!
//! Два router'а:
//! - `public_router()` — login + status (нет session gate)
//! - `protected_router()` — logout + me (за session middleware)
//!
//! **Session fixation prevention (T-05-SF):** build_auth_login вызывает
//! session.flush().await ПЕРЕД session.insert() — старая сессия уничтожается,
//! новый ID выдаётся для каждого нового логина.

use axum::{extract::State, routing::post, Json, Router};
use serde::{Deserialize, Serialize};
use tower_sessions::Session;

use trackly_core::auth::{Action, Identity, Role};
use trackly_core::error::AppError;

use crate::context::AppCtx;
use crate::dto::auth::{AdSettingsDto, AuthStatusDto, LoginRequest, UserDto};
use crate::error_axum::AppErrorResponse;

// ---------------------------------------------------------------------------
// Payload structs
// ---------------------------------------------------------------------------

/// Тело POST /api/v1/auth_login.
///
/// Обёртка `{ req: { login, password } }` — совпадает с формой, которую шлёт
/// фронтенд через `apiCall('auth_login', { req })` и которую ждёт Tauri-команда
/// `auth_login(req: LoginRequest)`. Один и тот же `apiCall` отправляет одинаковое
/// тело в оба транспорта, поэтому HTTP-сторона должна принимать `req`, а не
/// плоские поля (иначе браузерный логин падает на десериализации).
#[derive(Debug, Deserialize)]
pub struct LoginPayload {
    pub req: LoginRequest,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct StatusPayload {}

/// Тело POST /api/v1/request_ad_restore — явный запрос восстановления
/// доступа (09-AD-GAPS restoration-flow UX). Зеркалит `LoginPayload`'s
/// `{ req: ... }` envelope (тот же `apiCall('request_ad_restore', { req })`
/// паттерн фронтенда), но НЕ обёртка над `LoginRequest` — `remember` тут
/// бессмысленен (это не login, сессия не выдаётся).
#[derive(Debug, Deserialize, specta::Type)]
pub struct RequestAdRestorePayload {
    pub req: RequestAdRestoreRequest,
}

/// Credentials для явного re-request восстановления доступа. Неавторизованный
/// эндпойнт (как `auth_login`) — сам несёт credentials, у блокированного
/// пользователя нет сессии.
#[derive(Debug, Deserialize, specta::Type)]
pub struct RequestAdRestoreRequest {
    pub login: String,
    pub password: String,
}

// ---------------------------------------------------------------------------
// Session identity storage
// ---------------------------------------------------------------------------

/// Сериализуемая версия Identity для хранения в session store.
///
/// `Identity` из trackly-core не имеет Serialize/Deserialize (pure domain).
/// `SessionIdentity` — это сессионный DTO, который сериализуется в rmp-serde.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionIdentity {
    pub user_id: Option<i64>,
    pub role: String,
}

impl From<&Identity> for SessionIdentity {
    fn from(id: &Identity) -> Self {
        Self {
            user_id: id.user_id,
            role: id.role.as_str().to_string(),
        }
    }
}

impl TryFrom<SessionIdentity> for Identity {
    type Error = AppError;

    fn try_from(s: SessionIdentity) -> Result<Self, Self::Error> {
        Ok(Self {
            user_id: s.user_id,
            role: Role::from_str(&s.role)?,
        })
    }
}

// ---------------------------------------------------------------------------
// Session identity helpers
// ---------------------------------------------------------------------------

/// Загрузить Identity из session. Возвращает Unauthorized если сессия не содержит identity.
pub async fn session_identity(session: &Session) -> Result<Identity, AppError> {
    let si = session
        .get::<SessionIdentity>("identity")
        .await
        .map_err(|_| AppError::Unauthorized)?
        .ok_or(AppError::Unauthorized)?;
    si.try_into()
}

// ---------------------------------------------------------------------------
// build_* helpers
// ---------------------------------------------------------------------------

/// Аутентификация по логину + пароль → сессия.
///
/// **Session fixation mitigation (T-05-SF):**
/// 1. Сначала `session.flush()` — уничтожает любую предсуществующую сессию,
///    включая ту, что мог предустановить атакующий.
/// 2. Потом `session.insert("identity", ...)` — создаёт новую сессию с новым ID.
///
/// **D-UX-02 («Запомнить меня»):** после `insert()` явно выставляем
/// per-session expiry в зависимости от `remember`:
/// - `true` — `Expiry::OnInactivity(30 days)` (постоянная cookie, скользящее
///   истечение — совпадает с глобальным default из `http::build_router`, но
///   фиксируется явно на уровне сессии, чтобы не зависеть от global default).
/// - `false` — `Expiry::OnSessionEnd` (cookie без `Max-Age`/`Expires` —
///   браузер удаляет её при закрытии).
///
/// Expiry выставляется ПОСЛЕ `insert()` (не до) — `flush()` обнуляет
/// session state, а `set_expiry` после `insert` гарантирует что значение
/// применится к новой (после flush) сессии, а не будет потеряно при flush.
pub async fn build_auth_login(
    ctx: &AppCtx,
    session: Session,
    payload: LoginPayload,
) -> Result<UserDto, AppError> {
    let remember = payload.req.remember;
    let user = ctx.auth.login(payload.req).await?;

    // T-05-SF: flush BEFORE insert (session fixation prevention).
    session.flush().await.map_err(|e| AppError::Internal {
        source_chain: format!("session flush (login): {e}"),
    })?;

    let role = Role::from_str(&user.role)?;
    let identity = Identity {
        user_id: Some(user.id),
        role,
    };
    let session_identity = SessionIdentity::from(&identity);
    session
        .insert("identity", session_identity)
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("session insert (login): {e}"),
        })?;

    // D-UX-02: remember=true → persistent sliding 30-day cookie;
    // remember=false → session-only cookie (cleared on browser close).
    let expiry = if remember {
        tower_sessions::Expiry::OnInactivity(time::Duration::days(30))
    } else {
        tower_sessions::Expiry::OnSessionEnd
    };
    session.set_expiry(Some(expiry));

    Ok(user)
}

/// Явный запрос восстановления доступа (09-AD-GAPS restoration-flow UX).
/// Неаутентифицированный — несёт собственные credentials (как
/// `build_auth_login`), не трогает session (никакая сессия не выдаётся
/// блокированному/soft-deleted пользователю).
pub async fn build_request_ad_restore(
    ctx: &AppCtx,
    payload: RequestAdRestorePayload,
) -> Result<(), AppError> {
    ctx.auth
        .request_ad_restore(&payload.req.login, &payload.req.password)
        .await
}

/// Logout — flush session.
pub async fn build_auth_logout(session: Session) -> Result<(), AppError> {
    session.flush().await.map_err(|e| AppError::Internal {
        source_chain: format!("session flush (logout): {e}"),
    })
}

/// Вернуть текущего пользователя из сессии.
pub async fn build_auth_me(ctx: &AppCtx, session: Session) -> Result<UserDto, AppError> {
    let identity = session_identity(&session).await?;
    let user_id = identity.user_id.ok_or(AppError::Unauthorized)?;
    ctx.auth.get_user_by_id(user_id).await
}

/// Статус авторизации: needs_bootstrap, desktop_lock_enabled, user.
pub async fn build_auth_status(ctx: &AppCtx, session: Session) -> Result<AuthStatusDto, AppError> {
    let needs_bootstrap = ctx.auth.needs_bootstrap().await?;
    let desktop_lock_enabled = ctx.auth.get_desktop_lock_enabled().await?;

    // Попробовать загрузить пользователя из сессии (не ошибка если нет).
    let user = match session.get::<SessionIdentity>("identity").await {
        Ok(Some(si)) => {
            if let Some(uid) = si.user_id {
                ctx.auth.get_user_by_id(uid).await.ok()
            } else {
                None
            }
        }
        _ => None,
    };

    Ok(AuthStatusDto {
        needs_bootstrap,
        desktop_lock_enabled,
        user,
        place_path_display: ctx
            .config
            .organization
            .place_path_display
            .as_str()
            .to_string(),
    })
}

/// Тело запроса POST /api/v1/settings_set_ad — общий тип для HTTP body
/// и Tauri command param (зеркалирует `NetworkPatch` в http/settings.rs).
///
/// Только `enabled`/`auto_accept` доступны для записи (live `app_settings`,
/// ManageSettings-gated). `host`/`port`/`domain`/`base_dn`/`name_attr`/
/// `no_tls_verify` — read-only TOML bootstrap config (`ctx.config.ad`),
/// зеркалирует архитектуру `AdSettingsDto` (см. doc-comment в dto/auth.rs).
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SetAdPayload {
    pub enabled: bool,
    pub auto_accept: bool,
    /// Живой тумблер AD-SSO (Kerberos). Независим от `enabled`.
    /// `#[serde(default)]` — старые клиенты без этого поля шлют payload как
    /// раньше (тогда SSO трактуется как выключенный), backward-compat.
    #[serde(default)]
    pub sso_enabled: bool,
}

/// Вернуть текущие настройки AD: live (`enabled`/`auto_accept` из
/// `app_settings`) + read-only bootstrap (`host`/`port`/.../`no_tls_verify`
/// из `trackly.config.toml`).
///
/// **Безопасность (T-09-15):** require ManageSettings — только admin.
pub async fn build_settings_get_ad(
    ctx: &AppCtx,
    session: &Session,
) -> Result<AdSettingsDto, AppError> {
    let caller = session_identity(session).await?;
    trackly_core::auth::authorize(&caller, &Action::ManageSettings)?;

    let enabled = ctx.auth.ad_enabled().await?;
    let auto_accept = ctx.auth.ad_auto_accept().await?;
    let sso_enabled = ctx.auth.ad_sso_enabled().await?;
    let ad_config = &ctx.config.ad;
    let sso_keytab_present =
        !ad_config.keytab_path.is_empty() && std::path::Path::new(&ad_config.keytab_path).is_file();

    Ok(AdSettingsDto {
        enabled,
        auto_accept,
        host: ad_config.host.clone(),
        port: ad_config.resolved_port() as i64,
        domain: ad_config.domain.clone(),
        base_dn: ad_config.base_dn.clone(),
        name_attr: ad_config.name_attr.clone(),
        no_tls_verify: ad_config.no_tls_verify,
        sso_enabled,
        sso_spn: ad_config.spn.clone(),
        sso_keytab_path: ad_config.keytab_path.clone(),
        sso_keytab_present,
    })
}

/// Сохранить `enabled`/`auto_accept` в `app_settings` (live AD toggle).
///
/// Подключение (host/port/domain/...) НЕ редактируется через этот endpoint —
/// оно read-only bootstrap config (см. `SetAdPayload` doc-comment).
///
/// **Безопасность (T-09-15):** require ManageSettings — только admin.
/// Неаутентифицированный caller получает 401 от session_identity;
/// non-admin caller получает 403 от authorize.
pub async fn build_settings_set_ad(
    ctx: &AppCtx,
    session: &Session,
    payload: SetAdPayload,
) -> Result<(), AppError> {
    let caller = session_identity(session).await?;
    trackly_core::auth::authorize(&caller, &Action::ManageSettings)?;

    ctx.auth.set_ad_enabled(payload.enabled, &caller).await?;
    ctx.auth
        .set_ad_auto_accept(payload.auto_accept, &caller)
        .await?;
    ctx.auth
        .set_ad_sso_enabled(payload.sso_enabled, &caller)
        .await?;

    Ok(())
}

/// Проверить доступность AD-сервера (admin-действие "Проверить подключение",
/// Phase 9 gap-closure). НЕ требует пароль конечного пользователя — только
/// сетевая/протокольная доступность сконфигурированного AD-сервера.
///
/// **Безопасность:** require ManageSettings — только admin. Неаутентифицированный
/// caller получает 401 от session_identity; non-admin caller получает 403 от
/// `AuthService::test_ad_connection`'s internal `authorize` call.
pub async fn build_ad_test_connection(ctx: &AppCtx, session: &Session) -> Result<(), AppError> {
    let caller = session_identity(session).await?;
    ctx.auth.test_ad_connection(&caller).await
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

pub async fn handler_login(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(payload): Json<LoginPayload>,
) -> Result<Json<UserDto>, AppErrorResponse> {
    Ok(Json(
        build_auth_login(&ctx, session, payload)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_request_ad_restore(
    State(ctx): State<AppCtx>,
    Json(payload): Json<RequestAdRestorePayload>,
) -> Result<Json<()>, AppErrorResponse> {
    build_request_ad_restore(&ctx, payload)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(()))
}

pub async fn handler_logout(session: Session) -> Result<Json<()>, AppErrorResponse> {
    build_auth_logout(session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(()))
}

pub async fn handler_me(
    State(ctx): State<AppCtx>,
    session: Session,
) -> Result<Json<UserDto>, AppErrorResponse> {
    Ok(Json(
        build_auth_me(&ctx, session)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_status(
    State(ctx): State<AppCtx>,
    session: Session,
) -> Result<Json<AuthStatusDto>, AppErrorResponse> {
    Ok(Json(
        build_auth_status(&ctx, session)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_get_ad(
    State(ctx): State<AppCtx>,
    session: Session,
) -> Result<Json<AdSettingsDto>, AppErrorResponse> {
    Ok(Json(
        build_settings_get_ad(&ctx, &session)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_set_ad(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(payload): Json<SetAdPayload>,
) -> Result<Json<()>, AppErrorResponse> {
    build_settings_set_ad(&ctx, &session, payload)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(()))
}

pub async fn handler_ad_test_connection(
    State(ctx): State<AppCtx>,
    session: Session,
) -> Result<Json<()>, AppErrorResponse> {
    build_ad_test_connection(&ctx, &session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(()))
}

// ---------------------------------------------------------------------------
// Routers
// ---------------------------------------------------------------------------

/// Публичные маршруты (без session gate): login, status.
pub fn public_router() -> Router<AppCtx> {
    Router::new()
        .route("/api/v1/auth_login", post(handler_login))
        .route("/api/v1/auth_status", post(handler_status))
}

/// Защищённые маршруты (за session middleware): logout, me, AD settings.
pub fn protected_router() -> Router<AppCtx> {
    Router::new()
        .route("/api/v1/auth_logout", post(handler_logout))
        .route("/api/v1/auth_me", post(handler_me))
        .route("/api/v1/settings_get_ad", post(handler_get_ad))
        .route("/api/v1/settings_set_ad", post(handler_set_ad))
        .route(
            "/api/v1/ad_test_connection",
            post(handler_ad_test_connection),
        )
}
