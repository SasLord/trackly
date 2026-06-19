//! Newtype wrapper `AppErrorResponse` that implements `axum::response::IntoResponse`
//! for `AppError`.
//!
//! HTTP-маппинг:
//! - NotFound → 404
//! - Conflict / OptimisticLockMismatch → 409
//! - WriteQueueBusy / ServiceUnavailable → 503
//! - DatabaseFromNewerVersion / Internal → 500
//! - Validation → 400
//! - Unauthorized → 401
//! - Forbidden / RegistrationPending / AccessBlocked → 403

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use trackly_core::error::AppError;

/// Newtype-обёртка для `AppError`, реализующая `axum::IntoResponse`.
/// axum handlers возвращают `Result<T, AppErrorResponse>`.
///
/// Для удобства реализован `From<AppError> for AppErrorResponse`.
pub struct AppErrorResponse(pub AppError);

impl From<AppError> for AppErrorResponse {
    fn from(e: AppError) -> Self {
        Self(e)
    }
}

impl IntoResponse for AppErrorResponse {
    fn into_response(self) -> Response {
        let status = match &self.0 {
            AppError::NotFound { .. } => StatusCode::NOT_FOUND,
            AppError::Conflict { .. } => StatusCode::CONFLICT,
            AppError::OptimisticLockMismatch { .. } => StatusCode::CONFLICT,
            AppError::WriteQueueBusy => StatusCode::SERVICE_UNAVAILABLE,
            AppError::DatabaseFromNewerVersion { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Validation { .. } => StatusCode::BAD_REQUEST,
            AppError::Unauthorized => StatusCode::UNAUTHORIZED,
            AppError::Forbidden => StatusCode::FORBIDDEN,
            AppError::Internal { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::ServiceUnavailable { .. } => StatusCode::SERVICE_UNAVAILABLE,
            // AD bind succeeded but admission is not granted yet — caller must
            // see PendingScreen/BlockedScreen, not a generic 401 (D-REG-01/D-REG-03).
            AppError::RegistrationPending { .. } => StatusCode::FORBIDDEN,
            AppError::AccessBlocked { .. } => StatusCode::FORBIDDEN,
        };
        let body = Json(serde_json::json!({
            "code": self.0.code(),
            "message": self.0.to_string(),
        }));
        (status, body).into_response()
    }
}
