//! Доменные типы групп (Phase 41): типы групп, их свойства, группы и значения.
//!
//! Чистый домен без I/O (проверяется `tests/no_io_deps.rs`). Транспортные DTO живут
//! в trackly-app. Токены `behavior`/`data_type` в строках хранятся как `String`:
//! сырой токен из БД не должен ронять чтение (для чтения — `from_str_lenient`),
//! а на записи токен проверяется строгим `from_str`.

use crate::error::AppError;

/// Максимальная длина названия (типа, свойства, группы).
pub const NAME_MAX_CHARS: usize = 200;
/// Максимальная длина текстового значения свойства.
pub const TEXT_MAX_CHARS: usize = 2000;
/// Максимальная длина числового значения свойства (строкой).
pub const NUMBER_MAX_CHARS: usize = 32;
/// Максимум связей (пользователи/устройства) в одном свойстве.
pub const MAX_REFS_PER_PROPERTY: usize = 100;
/// Максимум устройств в одной пакетной операции (защита единственного writer'а).
pub const MAX_DEVICES_PER_BATCH: usize = 500;

/// Поведение типа группы (GRP-01): закрытый набор из трёх токенов.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupBehavior {
    /// Контейнер: устройства состоят в группе и живут в её месте.
    Container,
    /// Подмена: группа-якорь с подменяемым составом.
    Substitute,
    /// Разбор: группа распадается при снятии.
    Teardown,
}

impl GroupBehavior {
    /// Строгий разбор для записи: неизвестный токен — русская `Validation`.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Result<Self, AppError> {
        Self::from_str_lenient(s).ok_or_else(|| AppError::Validation {
            field: "behavior".to_string(),
            message: format!(
                "Неизвестное поведение группы: «{s}». Допустимые значения: \
                 container, substitute, teardown."
            ),
        })
    }

    /// Мягкий разбор для чтения: неизвестный токен — `None`.
    pub fn from_str_lenient(s: &str) -> Option<Self> {
        match s {
            "container" => Some(Self::Container),
            "substitute" => Some(Self::Substitute),
            "teardown" => Some(Self::Teardown),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Container => "container",
            Self::Substitute => "substitute",
            Self::Teardown => "teardown",
        }
    }
}

/// Тип данных свойства (GRP-02): закрытый набор из шести токенов.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyDataType {
    Text,
    Number,
    Ip,
    Mac,
    /// Ссылки на пользователей.
    Users,
    /// Ссылки на устройства.
    DeviceRefs,
}

impl PropertyDataType {
    /// Строгий разбор для записи: неизвестный токен — русская `Validation`.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Result<Self, AppError> {
        Self::from_str_lenient(s).ok_or_else(|| AppError::Validation {
            field: "data_type".to_string(),
            message: format!(
                "Неизвестный тип данных свойства: «{s}». Допустимые значения: \
                 text, number, ip, mac, users, device_refs."
            ),
        })
    }

    /// Мягкий разбор для чтения: неизвестный токен — `None`.
    pub fn from_str_lenient(s: &str) -> Option<Self> {
        match s {
            "text" => Some(Self::Text),
            "number" => Some(Self::Number),
            "ip" => Some(Self::Ip),
            "mac" => Some(Self::Mac),
            "users" => Some(Self::Users),
            "device_refs" => Some(Self::DeviceRefs),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Number => "number",
            Self::Ip => "ip",
            Self::Mac => "mac",
            Self::Users => "users",
            Self::DeviceRefs => "device_refs",
        }
    }
}

/// Строка типа группы при чтении из репозитория.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupTypeRow {
    pub id: i64,
    pub code: String,
    pub name: String,
    /// Сырой токен из БД; разбирать через `GroupBehavior::from_str_lenient`.
    pub behavior: String,
    pub is_builtin: bool,
    pub sort_order: i64,
    pub quick_action_enabled: bool,
    pub quick_action_label: Option<String>,
    pub version: i64,
    pub created_at_utc: i64,
    pub updated_at_utc: i64,
}

/// Данные для создания типа группы.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupTypeNew {
    pub code: String,
    pub name: String,
    pub behavior: GroupBehavior,
    pub is_builtin: bool,
    pub sort_order: i64,
    pub quick_action_enabled: bool,
    pub quick_action_label: Option<String>,
}

/// Частичное обновление типа. `code` и `behavior` неизменяемы (триггер БД).
/// `quick_action_label: Some(None)` — явный сброс подписи.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GroupTypePatch {
    pub name: Option<String>,
    pub sort_order: Option<i64>,
    pub quick_action_enabled: Option<bool>,
    pub quick_action_label: Option<Option<String>>,
}

/// Строка свойства типа группы.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyRow {
    pub id: i64,
    pub type_id: i64,
    pub name: String,
    /// Сырой токен из БД; разбирать через `PropertyDataType::from_str_lenient`.
    pub data_type: String,
    pub sort_order: i64,
    pub is_required: bool,
    pub show_on_map: bool,
    pub archived_at_utc: Option<i64>,
    pub version: i64,
    pub created_at_utc: i64,
    pub updated_at_utc: i64,
}

/// Данные для создания свойства.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyNew {
    pub type_id: i64,
    pub name: String,
    pub data_type: PropertyDataType,
    pub is_required: bool,
    pub show_on_map: bool,
}

/// Частичное обновление свойства.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PropertyPatch {
    pub name: Option<String>,
    pub data_type: Option<PropertyDataType>,
    pub is_required: Option<bool>,
    pub show_on_map: Option<bool>,
}

/// Строка группы.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupRow {
    pub id: i64,
    pub type_id: i64,
    pub name: String,
    /// Порядковый номер внутри типа.
    pub seq: i64,
    pub place_id: Option<i64>,
    pub parent_group_id: Option<i64>,
    pub version: i64,
    pub created_at_utc: i64,
    pub updated_at_utc: i64,
}

/// Данные для создания группы.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupNew {
    pub type_id: i64,
    pub name: String,
    pub seq: i64,
    pub place_id: Option<i64>,
}

/// Одно значение свойства группы (скаляр в `value_text` или ссылка в `value_ref`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupValueRow {
    pub group_id: i64,
    pub property_id: i64,
    pub position: i64,
    pub value_text: Option<String>,
    pub value_ref: Option<i64>,
    pub is_primary: bool,
}

/// Название: trim, пустое — «Укажите название.», длиннее `NAME_MAX_CHARS` — ошибка.
pub fn validate_name(raw: &str, field: &str) -> Result<String, AppError> {
    let s = raw.trim();
    if s.is_empty() {
        return Err(AppError::Validation {
            field: field.to_string(),
            message: "Укажите название.".to_string(),
        });
    }
    if s.chars().count() > NAME_MAX_CHARS {
        return Err(AppError::Validation {
            field: field.to_string(),
            message: format!("Название слишком длинное: не более {NAME_MAX_CHARS} символов."),
        });
    }
    Ok(s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn behavior_round_trip_and_rejection() {
        for t in ["container", "substitute", "teardown"] {
            assert_eq!(GroupBehavior::from_str(t).unwrap().as_str(), t);
            assert!(GroupBehavior::from_str_lenient(t).is_some());
        }
        match GroupBehavior::from_str("bogus") {
            Err(AppError::Validation { field, message }) => {
                assert_eq!(field, "behavior");
                assert!(message.contains("Допустимые значения"));
            }
            other => panic!("ожидали Validation, получили {other:?}"),
        }
        assert_eq!(GroupBehavior::from_str_lenient("bogus"), None);
        assert_eq!(GroupBehavior::from_str_lenient(""), None);
    }

    #[test]
    fn data_type_round_trip_and_rejection() {
        for t in ["text", "number", "ip", "mac", "users", "device_refs"] {
            assert_eq!(PropertyDataType::from_str(t).unwrap().as_str(), t);
            assert!(PropertyDataType::from_str_lenient(t).is_some());
        }
        match PropertyDataType::from_str("date") {
            Err(AppError::Validation { field, .. }) => assert_eq!(field, "data_type"),
            other => panic!("ожидали Validation, получили {other:?}"),
        }
        assert_eq!(PropertyDataType::from_str_lenient("date"), None);
    }

    #[test]
    fn validate_name_trims_and_limits() {
        assert_eq!(validate_name("  Стойка 1 ", "name").unwrap(), "Стойка 1");
        assert!(matches!(
            validate_name("   ", "name"),
            Err(AppError::Validation { .. })
        ));
        assert!(validate_name(&"а".repeat(NAME_MAX_CHARS), "name").is_ok());
        assert!(matches!(
            validate_name(&"а".repeat(NAME_MAX_CHARS + 1), "name"),
            Err(AppError::Validation { .. })
        ));
    }

    #[test]
    fn limits_constants() {
        assert_eq!(MAX_DEVICES_PER_BATCH, 500);
        assert_eq!(MAX_REFS_PER_PROPERTY, 100);
        assert_eq!(NUMBER_MAX_CHARS, 32);
        assert_eq!(TEXT_MAX_CHARS, 2000);
    }
}
