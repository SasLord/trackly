//! DTO раздела «Устройства» сотрудника (Phase 41.6, EMP-02/EMP-03).
//!
//! Минимальный набор полей (BOPLA, D-17): для принтера только уровень тонера;
//! без сетевого адреса, секретов SNMP, SNMP-полей, USB-хоста, id групп и мест. Номер заявки
//! присутствует только там, где сотрудник автор или участник (D-07); по чужой
//! открытой заявке отдаётся лишь флаг `other_open_request`.

use serde::{Deserialize, Serialize};
use specta::Type;

/// Ответ команды `employee_devices_list`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct EmployeeDevicesDto {
    /// Состоит ли сотрудник хотя бы в одной группе (различие двух пустых состояний, D-08).
    pub has_groups: bool,
    pub items: Vec<EmployeeDeviceDto>,
}

/// Строка списка устройств сотрудника; карточка читает ту же строку (D-15).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct EmployeeDeviceDto {
    #[specta(type = i32)]
    pub id: i64,
    pub name: String,
    pub type_name: String,
    pub is_printer: bool,
    pub model: Option<String>,
    pub serial_number: Option<String>,
    pub inventory_number: Option<String>,
    pub place_full: Option<String>,
    pub status_code: Option<String>,
    pub status_name: String,
    /// Имена привязанных групп (АРМ), через которые устройство попало в область (D-04).
    pub arm_names: Vec<String>,
    /// У принтера: "usb" | "link" | "member" (D-05); у остальных `None`.
    pub printer_origin: Option<String>,
    pub toner: Option<EmployeeTonerDto>,
    /// Есть открытая ремонтная заявка (независимо от участия).
    pub in_repair: bool,
    /// Открытая ремонтная заявка, где сотрудник автор/участник (D-11).
    #[specta(type = Option<i32>)]
    pub my_repair_request_id: Option<i64>,
    /// Открытые заявки по устройству, где сотрудник автор/участник.
    pub my_open_requests: Vec<EmployeeRequestRefDto>,
    /// Есть открытая заявка, где сотрудник не участник (номер не отдаётся).
    pub other_open_request: bool,
    /// Заявки по устройству для карточки: открытые и не более 10 закрытых.
    pub my_requests: Vec<EmployeeRequestHistoryDto>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct EmployeeTonerDto {
    pub levels: Vec<EmployeeTonerLevelDto>,
    #[specta(type = Option<i32>)]
    pub polled_at_utc: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct EmployeeTonerLevelDto {
    pub label: String,
    /// `None` — уровень неизвестен.
    #[specta(type = Option<i32>)]
    pub percent: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct EmployeeRequestRefDto {
    #[specta(type = i32)]
    pub id: i64,
    pub request_type: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct EmployeeRequestHistoryDto {
    #[specta(type = i32)]
    pub id: i64,
    pub request_type: String,
    pub status: String,
    #[specta(type = i32)]
    pub created_at_utc: i64,
}
