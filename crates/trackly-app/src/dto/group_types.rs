//! DTO типов групп и их свойств (Phase 41) — транспортные контракты для
//! `GroupTypeService`, общие для Tauri-команд и HTTP.
//!
//! Snake_case JSON, без `rename_all` (camelCase бывает только у HTTP-payload'ов).
//! Токены `behavior`/`data_type` выходят строками: доменные enum'ы не имеют
//! serde/specta. `GroupTypeUpdateDto` намеренно несёт `code` и `behavior`:
//! иначе serde молча отбросил бы неизвестные поля, и проверка неизменяемости
//! через транспорт была бы вакуумной — сервис отклоняет любое отличие.

use serde::{Deserialize, Serialize};
use specta::Type;
use trackly_core::domain::groups::{GroupTypeRow, PropertyRow};

/// Свойство типа группы.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct GroupTypePropertyDto {
    #[specta(type = i32)]
    pub id: i64,
    #[specta(type = i32)]
    pub type_id: i64,
    pub name: String,
    pub data_type: String,
    #[specta(type = i32)]
    pub sort_order: i64,
    pub is_required: bool,
    /// Потребитель — фаза 43 (D-15): здесь только хранится и возвращается.
    pub show_on_map: bool,
    /// Свойство скрыто (`archived_at_utc IS NOT NULL`).
    pub archived: bool,
    /// Число групп с заполненным значением: по нему UI блокирует смену типа данных (D-16).
    #[specta(type = i32)]
    pub filled_group_count: i64,
    #[specta(type = i32)]
    pub version: i64,
}

impl GroupTypePropertyDto {
    pub fn from_row(row: PropertyRow, filled_group_count: i64) -> Self {
        Self {
            id: row.id,
            type_id: row.type_id,
            name: row.name,
            data_type: row.data_type,
            sort_order: row.sort_order,
            is_required: row.is_required,
            show_on_map: row.show_on_map,
            archived: row.archived_at_utc.is_some(),
            filled_group_count,
            version: row.version,
        }
    }
}

/// Тип группы со свойствами.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct GroupTypeDto {
    #[specta(type = i32)]
    pub id: i64,
    pub code: String,
    pub name: String,
    pub behavior: String,
    pub is_builtin: bool,
    #[specta(type = i32)]
    pub sort_order: i64,
    pub quick_action_enabled: bool,
    pub quick_action_label: Option<String>,
    #[specta(type = i32)]
    pub group_count: i64,
    #[specta(type = i32)]
    pub version: i64,
    pub properties: Vec<GroupTypePropertyDto>,
}

impl GroupTypeDto {
    pub fn from_row(
        row: GroupTypeRow,
        group_count: i64,
        properties: Vec<GroupTypePropertyDto>,
    ) -> Self {
        Self {
            id: row.id,
            code: row.code,
            name: row.name,
            behavior: row.behavior,
            is_builtin: row.is_builtin,
            sort_order: row.sort_order,
            quick_action_enabled: row.quick_action_enabled,
            quick_action_label: row.quick_action_label,
            group_count,
            version: row.version,
            properties,
        }
    }
}

/// Создание пользовательского типа.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct GroupTypeCreateDto {
    pub name: String,
    pub behavior: String,
}

/// Обновление типа. `code` и `behavior` — только для проверки неизменяемости.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type, Default)]
pub struct GroupTypeUpdateDto {
    pub name: Option<String>,
    #[specta(type = Option<i32>)]
    pub sort_order: Option<i64>,
    #[serde(default, with = "serde_with::rust::double_option")]
    pub quick_action_label: Option<Option<String>>,
    pub quick_action_enabled: Option<bool>,
    pub code: Option<String>,
    pub behavior: Option<String>,
}

/// Создание свойства.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct PropertyCreateDto {
    #[specta(type = i32)]
    pub type_id: i64,
    pub name: String,
    pub data_type: String,
    pub is_required: bool,
    pub show_on_map: bool,
}

/// Обновление свойства.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type, Default)]
pub struct PropertyUpdateDto {
    pub name: Option<String>,
    pub data_type: Option<String>,
    pub is_required: Option<bool>,
    pub show_on_map: Option<bool>,
}

/// Исход удаления свойства: `archived = true` — скрыто (значения остались),
/// `false` — удалено физически.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct PropertyDeleteOutcomeDto {
    pub archived: bool,
}

/// Ссылка на группу `(id, name)` — для попапа нарушителей обязательности (D-14).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct GroupRefDto {
    #[specta(type = i32)]
    pub id: i64,
    pub name: String,
}
