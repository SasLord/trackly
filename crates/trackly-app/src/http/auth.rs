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

use trackly_core::auth::{Identity, Role};
use trackly_core::error::AppError;

use crate::context::AppCtx;
use crate::dto::auth::{AuthStatusDto, LoginRequest, UserDto};
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
pub async fn build_auth_login(
    ctx: &AppCtx,
    session: Session,
    payload: LoginPayload,
) -> Result<UserDto, AppError> {
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

    Ok(user)
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
    })
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

// ---------------------------------------------------------------------------
// Routers
// ---------------------------------------------------------------------------

/// Публичные маршруты (без session gate): login, status.
pub fn public_router() -> Router<AppCtx> {
    Router::new()
        .route("/api/v1/auth_login", post(handler_login))
        .route("/api/v1/auth_status", post(handler_status))
}

/// Защищённые маршруты (за session middleware): logout, me.
pub fn protected_router() -> Router<AppCtx> {
    Router::new()
        .route("/api/v1/auth_logout", post(handler_logout))
        .route("/api/v1/auth_me", post(handler_me))
}
