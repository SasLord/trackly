//! HTTP-маршрут раздела «Устройства» сотрудника (Phase 41.6): зеркало
//! `tauri_cmds::employee_devices`. Тела запроса нет; идентичность из сессии.

use axum::{extract::State, routing::post, Json, Router};
use tower_sessions::Session;

use crate::context::AppCtx;
use crate::dto::employee_device::EmployeeDevicesDto;
use crate::error_axum::AppErrorResponse;
use crate::http::auth::session_identity;
use crate::tauri_cmds::employee_devices::build_employee_devices_list;

pub async fn handler(
    State(ctx): State<AppCtx>,
    session: Session,
) -> Result<Json<EmployeeDevicesDto>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_employee_devices_list(&ctx, &identity)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub fn router() -> Router<AppCtx> {
    Router::new().route("/api/v1/employee_devices_list", post(handler))
}
