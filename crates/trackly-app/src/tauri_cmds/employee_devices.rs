//! Команда раздела «Устройства» сотрудника (Phase 41.6, EMP-02/EMP-03).
//!
//! Схема S-1: `build_employee_devices_list` общая для Tauri-команды и axum-маршрута.
//! Параметров нет (D-14): идентичность берётся только из сессии / системного
//! администратора десктопа, клиентские id устройств не принимаются.

use crate::context::AppCtx;
use crate::dto::employee_device::EmployeeDevicesDto;
use crate::tauri_cmds::users::resolve_tauri_identity;
use trackly_core::auth::{authorize, Action, Identity};
use trackly_core::error::AppError;

pub async fn build_employee_devices_list(
    ctx: &AppCtx,
    caller: &Identity,
) -> Result<EmployeeDevicesDto, AppError> {
    authorize(caller, &Action::CreateRequest)?;
    ctx.employee_devices.list_for_employee(caller).await
}

#[tauri::command]
#[specta::specta]
pub async fn employee_devices_list(
    state: tauri::State<'_, AppCtx>,
) -> Result<EmployeeDevicesDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_employee_devices_list(state.inner(), &caller).await
}
