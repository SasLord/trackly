//! DTO ремонтного домена (Phase 41.4): опции устройств, создание заявки,
//! состояние ремонта для карточки, заходы в сервис.
//!
//! Числа и id уходят в specta как `i32` (как во всех DTO проекта); деньги —
//! копейками плюс готовая строка для показа.

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::dto::request::RequestDto;

/// Опция выпадающего списка устройств формы «Ремонт» (REP-01).
///
/// Минимальный набор полей (BOPLA): без id чужой заявки, без сетевых полей;
/// «уже в ремонте» — только булев признак.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RepairDeviceOptionDto {
    #[specta(type = i32)]
    pub id: i64,
    pub name: String,
    pub inventory_no: Option<String>,
    pub type_name: String,
    pub place: Option<String>,
    pub in_repair: bool,
}

/// Вход создания ремонтной заявки. `device_id = None` — вариант «Другое» (D-24).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RepairCreateDto {
    #[specta(type = Option<i32>)]
    pub device_id: Option<i64>,
    pub description: String,
}

/// Итог создания: новая заявка или присоединение к открытой (D-20).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RepairCreateResultDto {
    pub request: RequestDto,
    pub joined: bool,
}

/// Заход устройства в сервис (D-13) с готовыми подписями мест и стоимости.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RepairVisitDto {
    #[specta(type = i32)]
    pub id: i64,
    #[specta(type = i32)]
    pub request_id: i64,
    #[specta(type = i32)]
    pub device_id: i64,
    #[specta(type = i32)]
    pub seq: i64,
    #[specta(type = i32)]
    pub service_place_id: i64,
    pub service_place_path: Option<String>,
    #[specta(type = Option<i32>)]
    pub from_place_id: Option<i64>,
    pub from_place_path: Option<String>,
    #[specta(type = i32)]
    pub sent_at_utc: i64,
    #[specta(type = Option<i32>)]
    pub returned_at_utc: Option<i64>,
    pub giver_name: String,
    pub receiver_name: String,
    pub fault_description: String,
    #[specta(type = Option<i32>)]
    pub returned_to_place_id: Option<i64>,
    pub returned_to_place_path: Option<String>,
    pub received_by_name: Option<String>,
    pub works_done: Option<String>,
    #[specta(type = Option<i32>)]
    pub cost_kopecks: Option<i64>,
    pub cost_display: Option<String>,
    pub act_number: String,
    #[specta(type = i32)]
    pub scan_count: i64,
    #[specta(type = i32)]
    pub version: i64,
}

/// Состояние ремонта для карточки заявки.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RepairStateDto {
    #[specta(type = i32)]
    pub request_id: i64,
    #[specta(type = Option<i32>)]
    pub device_id: Option<i64>,
    pub device_name: Option<String>,
    pub device_status_code: Option<String>,
    #[specta(type = Option<i32>)]
    pub current_place_id: Option<i64>,
    pub current_place_path: Option<String>,
    /// Место, сохранённое на заявке при первом перемещении (D-16).
    #[specta(type = Option<i32>)]
    pub origin_place_id: Option<i64>,
    pub origin_place_path: Option<String>,
    pub moved: bool,
    pub is_anchor: bool,
    pub group_nested: bool,
    /// Число деталей блока якорного устройства (без самого якоря).
    #[specta(type = i32)]
    pub group_block_size: i64,
    #[specta(type = Option<i32>)]
    pub suggested_return_place_id: Option<i64>,
    pub suggested_return_place_path: Option<String>,
    pub has_open_visit: bool,
    pub visits: Vec<RepairVisitDto>,
    pub can_cancel_repair: bool,
}

/// Последние выбранные места (D-25), раздельно для перемещения и сервиса.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RepairLastPlacesDto {
    #[specta(type = Option<i32>)]
    pub move_place_id: Option<i64>,
    #[specta(type = Option<i32>)]
    pub service_place_id: Option<i64>,
}

/// Пара «устройство -> открытая заявка» для пометки «уже в ремонте» персоналу.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct OpenRepairRefDto {
    #[specta(type = i32)]
    pub device_id: i64,
    #[specta(type = i32)]
    pub request_id: i64,
}
