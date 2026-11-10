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

/// Выбор в попапе «Перемещение устройства» при принятии заявки (D-03):
/// переместить на место либо оставить где стоит.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum RepairMoveChoiceDto {
    #[serde(rename_all = "camelCase")]
    Move {
        #[specta(type = i32)]
        place_id: i64,
    },
    Stay,
}

/// Итог перехода цикла ремонта: обновлённая заявка и состояние ремонта.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RepairTransitionResultDto {
    pub request: RequestDto,
    pub state: RepairStateDto,
}

/// Вход «Отправить в ремонт» (D-04, D-05): заход в сервис с номером акта.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RepairSendDto {
    #[specta(type = i32)]
    pub request_id: i64,
    #[specta(type = i32)]
    pub version: i64,
    /// Дата отправки, unix-секунды UTC.
    #[specta(type = i32)]
    pub date_utc: i64,
    pub giver_name: String,
    pub receiver_name: String,
    #[specta(type = i32)]
    pub service_place_id: i64,
    pub fault_description: String,
}

/// Вход «Забрать из ремонта» (D-07, D-08, D-29).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RepairReceiveDto {
    #[specta(type = i32)]
    pub request_id: i64,
    #[specta(type = i32)]
    pub version: i64,
    /// Дата возврата, unix-секунды UTC.
    #[specta(type = i32)]
    pub date_utc: i64,
    pub received_by_name: String,
    #[specta(type = i32)]
    pub place_id: i64,
    /// D-08: вернуть на рабочее место и сразу завершить заявку.
    pub return_to_workplace: bool,
    pub works_done: Option<String>,
    /// Стоимость строкой («1 500,50»); разбирается на сервере.
    pub cost: Option<String>,
}

/// Вход «Завершить» (D-09, D-16): место возврата обязательно, только если устройство
/// перемещалось в ходе ремонта.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RepairCompleteDto {
    #[specta(type = i32)]
    pub request_id: i64,
    #[specta(type = i32)]
    pub version: i64,
    #[specta(type = Option<i32>)]
    pub place_id: Option<i64>,
    pub notes: Option<String>,
}

/// Вход «Отклонить» ремонтную заявку.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RepairRejectDto {
    #[specta(type = i32)]
    pub request_id: i64,
    #[specta(type = i32)]
    pub version: i64,
    pub notes: Option<String>,
}

/// Вход «Отменить ремонт» (D-12) и отмены автором.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RepairCancelDto {
    #[specta(type = i32)]
    pub request_id: i64,
    #[specta(type = i32)]
    pub version: i64,
}

/// Вход «Указать устройство» для заявки «Другое» (D-24).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RepairAssignDeviceDto {
    #[specta(type = i32)]
    pub request_id: i64,
    #[specta(type = i32)]
    pub version: i64,
    #[specta(type = i32)]
    pub device_id: i64,
}

/// Итог «Указать устройство»: либо обновлённая заявка, либо id заявки, занявшей устройство.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RepairAssignResultDto {
    pub request: Option<RequestDto>,
    #[specta(type = Option<i32>)]
    pub conflict_request_id: Option<i64>,
}
