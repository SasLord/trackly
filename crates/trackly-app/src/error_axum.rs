//! Newtype wrapper `AppErrorResponse` that implements `axum::response::IntoResponse`
//! for `AppError`.
//!
//! HTTP-маппинг:
//! - NotFound → 404
//! - Conflict / OptimisticLockMismatch / AlreadyInGroup → 409
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
            AppError::AlreadyInGroup { .. } => StatusCode::CONFLICT,
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
        // Serialize via `AppError`'s own `Serialize` impl (not a hand-rolled
        // json!{} of code+message) — `AccessBlocked` carries `pending`/
        // `rejection_reason` in `details` that the BlockedScreen UI needs
        // (09-AD-GAPS restoration-flow UX). A hand-rolled body would have
        // silently dropped `details` for every variant, not just this one.
        let body = Json(&self.0);
        (status, body).into_response()
    }
}

#[cfg(test)]
mod tests {
    //! GRW-04 (фаза 41.8): «уже состоит в другой группе» уходит по HTTP как 409
    //! с телом `{ code: "ALREADY_IN_GROUP", details: { items: [...] } }` —
    //! на этот статус и форму опирается шаг переноса в UI (D-18). Сериализацию
    //! самого `AppError` держит trackly-core; здесь — именно HTTP-адаптер.
    use super::*;
    use trackly_core::error::OccupiedItem;

    async fn status_and_body(e: AppError) -> (StatusCode, serde_json::Value) {
        let res = AppErrorResponse(e).into_response();
        let status = res.status();
        let bytes = axum::body::to_bytes(res.into_body(), 1024 * 1024)
            .await
            .expect("read body");
        let v: serde_json::Value = serde_json::from_slice(&bytes).expect("json body");
        (status, v)
    }

    #[tokio::test]
    async fn already_in_group_maps_to_409_with_items() {
        let (status, v) = status_and_body(AppError::AlreadyInGroup {
            items: vec![
                OccupiedItem::device(5, "Монитор 24\"".into(), 7, "АРМ #3".into()),
                OccupiedItem::group(9, "Разбор #7".into(), 8, "АРМ #4".into()),
            ],
        })
        .await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(v["code"], "ALREADY_IN_GROUP");
        let items = v["details"]["items"].as_array().expect("details.items");
        assert_eq!(items.len(), 2);
        assert_eq!(items[0]["kind"], "device");
        assert_eq!(items[0]["id"], 5);
        assert_eq!(items[0]["group_id"], 7);
        assert_eq!(items[0]["group_name"], "АРМ #3");
        assert_eq!(items[1]["kind"], "group");
        assert_eq!(items[1]["id"], 9);
        assert_eq!(items[1]["group_id"], 8);
    }

    #[tokio::test]
    async fn already_in_group_empty_items_still_409() {
        // Пустой пакет не должен «проваливаться» в иной статус (400/500).
        let (status, v) = status_and_body(AppError::AlreadyInGroup { items: vec![] }).await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(v["code"], "ALREADY_IN_GROUP");
        assert_eq!(
            v["details"]["items"].as_array().map(Vec::len),
            Some(0),
            "details.items обязан быть массивом даже пустым"
        );
    }
}
