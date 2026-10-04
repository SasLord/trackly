//! DTO групп устройств (Phase 41) — транспортные контракты для `GroupService`,
//! общие для Tauri-команд и HTTP.
//!
//! Snake_case JSON, без `rename_all`. Токен `type_behavior` выходит строкой:
//! доменный enum не имеет serde/specta.

use serde::{Deserialize, Serialize};
use specta::Type;

/// Группа: поля для дерева, шапки карточки и подтверждения удаления.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct GroupDto {
    #[specta(type = i32)]
    pub id: i64,
    #[specta(type = i32)]
    pub type_id: i64,
    pub type_code: String,
    pub type_name: String,
    pub type_behavior: String,
    pub name: String,
    #[specta(type = i32)]
    pub seq: i64,
    /// `None` — «Без места» (D-21).
    #[specta(type = Option<i32>)]
    pub place_id: Option<i64>,
    /// Полный путь места; `None`, если места нет.
    pub place_path: Option<String>,
    #[specta(type = Option<i32>)]
    pub parent_group_id: Option<i64>,
    /// Корень цепочки вложенности (для корневой группы — она сама, D-20).
    #[specta(type = i32)]
    pub root_group_id: i64,
    pub root_group_name: String,
    #[specta(type = i32)]
    pub version: i64,
    /// Устройства группы ВКЛЮЧАЯ вложенные группы.
    #[specta(type = i32)]
    pub device_count: i64,
    /// Только прямые устройства — число в подтверждении удаления.
    #[specta(type = i32)]
    pub direct_device_count: i64,
    #[specta(type = i32)]
    pub nested_group_count: i64,
}

/// Создание группы. `name = None` — имя по умолчанию «{тип} #{seq}».
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct GroupCreateDto {
    #[specta(type = i32)]
    pub type_id: i64,
    pub name: Option<String>,
    #[specta(type = Option<i32>)]
    pub place_id: Option<i32>,
}

/// Строка устройства в составе группы (столбцы `PlaceContents`, D-05).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct GroupMemberDeviceDto {
    #[specta(type = i32)]
    pub device_id: i64,
    pub type_name: String,
    pub name: String,
    pub inventory_number: Option<String>,
    pub serial_number: Option<String>,
    #[specta(type = Option<i32>)]
    pub place_id: Option<i64>,
    pub place_path: Option<String>,
    /// Сокращённый путь считает сервер (`compute_place_path_short_with_conn`).
    pub place_path_short: Option<String>,
    pub status_name: Option<String>,
}

/// Состав группы: прямые устройства и прямые вложенные группы (D-03).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct GroupCompositionDto {
    pub devices: Vec<GroupMemberDeviceDto>,
    pub child_groups: Vec<GroupDto>,
}

/// Кандидат во вложенные группы (D-04).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct GroupSearchHitDto {
    #[specta(type = i32)]
    pub id: i64,
    pub name: String,
    pub type_name: String,
    #[specta(type = i32)]
    pub device_count: i64,
    pub has_parent: bool,
}

/// Членство устройства в группе (D-01/D-19).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct DeviceMembershipDto {
    #[specta(type = i32)]
    pub device_id: i64,
    #[specta(type = i32)]
    pub group_id: i64,
    pub group_name: String,
    /// `true`, если у группы задано место: только тогда действует запрет
    /// индивидуального перемещения (D-21).
    pub group_has_place: bool,
}

/// Итог удаления группы.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct GroupDeleteResultDto {
    /// Сколько устройств освободилось (только прямые члены удалённой группы).
    #[specta(type = i32)]
    pub released_devices: i32,
}

/// Запрос «Перенести» группу (D-17): `version` — CAS по версии группы.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct GroupMoveDto {
    #[specta(type = i32)]
    pub id: i64,
    #[specta(type = i32)]
    pub version: i64,
    #[specta(type = i32)]
    pub target_place_id: i64,
}

/// Итог переноса группы (D-24): данные для тоста и инвалидации счётчиков (D-08).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct GroupMoveResultDto {
    pub moved_devices: i32,
    pub moved_nested_groups: i32,
    /// Прежние и новые места без дублей.
    #[specta(type = Vec<i32>)]
    pub changed_place_ids: Vec<i64>,
    /// «группа и 6 устройств».
    pub summary: String,
    pub batch_id: Option<String>,
}

/// Добавление устройств в группу (D-01): один обработчик для строки-поиска и модалки.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct GroupAddDevicesDto {
    #[specta(type = i32)]
    pub group_id: i64,
    #[specta(type = Vec<i32>)]
    pub device_ids: Vec<i64>,
}

/// Итог добавления: сколько устройств вошло и какие места затронуты (D-08).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct GroupAddDevicesResultDto {
    pub added: i32,
    /// Прежние и новые места без дублей; пусто, если место ничьё не менялось.
    #[specta(type = Vec<i32>)]
    pub changed_place_ids: Vec<i64>,
}

/// Вывод устройств из группы (D-02): без подтверждения, место не меняется.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct GroupRemoveDevicesDto {
    #[specta(type = i32)]
    pub group_id: i64,
    #[specta(type = Vec<i32>)]
    pub device_ids: Vec<i64>,
}

/// Вложение группы в группу (D-04): `parent_group_id = None` — вывод в корень.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct GroupSetParentDto {
    #[specta(type = i32)]
    pub id: i64,
    #[specta(type = i32)]
    pub version: i64,
    #[specta(type = Option<i32>)]
    pub parent_group_id: Option<i64>,
}

/// Ссылка в значении свойства `users`/`device_refs`: `is_primary` — основной пользователь.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct GroupRefInputDto {
    #[specta(type = i32)]
    pub ref_id: i64,
    pub is_primary: bool,
}

/// Значение одного свойства при записи формы: скаляр в `text` либо связи в `refs`.
/// Пустое значение (пустой текст, пустой список) = удаление ранее сохранённого.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct GroupValueInputDto {
    #[specta(type = i32)]
    pub property_id: i64,
    pub text: Option<String>,
    pub refs: Vec<GroupRefInputDto>,
}

/// Запись значений свойств группы: `version` — CAS по версии группы.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct GroupSetValuesDto {
    #[specta(type = i32)]
    pub id: i64,
    #[specta(type = i32)]
    pub version: i64,
    pub values: Vec<GroupValueInputDto>,
}

/// Пользователь для выбора (D-11): только безопасные поля.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct UserOptionDto {
    #[specta(type = i32)]
    pub id: i64,
    pub full_name: String,
    pub login: String,
}

/// Пользователь в значении свойства `users`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct GroupUserDto {
    #[specta(type = i32)]
    pub user_id: i64,
    pub full_name: String,
    pub is_primary: bool,
}

/// Свойство типа вместе со значением группы (форма панели «Свойства»).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct GroupPropertyValueDto {
    #[specta(type = i32)]
    pub property_id: i64,
    pub name: String,
    pub data_type: String,
    #[specta(type = i32)]
    pub sort_order: i64,
    pub is_required: bool,
    pub show_on_map: bool,
    /// Нормализованный скаляр (text/number/ip/mac); для связей `None`.
    pub text: Option<String>,
    /// Живые пользователи значения (тип `users`), основной помечен.
    pub users: Vec<GroupUserDto>,
    /// Живые устройства явных ссылок (тип `device_refs`) по порядку позиций.
    #[specta(type = Vec<i32>)]
    pub ref_device_ids: Vec<i64>,
}

/// Подключённый принтер: USB-производный (`usb`) или явная ссылка (`link`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct GroupPrinterDto {
    #[specta(type = i32)]
    pub device_id: i64,
    pub name: String,
    pub inventory_number: Option<String>,
    pub serial_number: Option<String>,
    /// `usb` | `link`. При совпадении источников побеждает `usb`.
    pub origin: String,
    /// Есть явная ссылка (её можно удалить, даже если принтер виден по USB).
    pub has_explicit_link: bool,
}

/// Карточка группы: шапка, свойства со значениями и единый список принтеров (D-12).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct GroupCardDto {
    pub group: GroupDto,
    pub properties: Vec<GroupPropertyValueDto>,
    pub printers: Vec<GroupPrinterDto>,
    /// Первое живое свойство `device_refs` типа; `None`, если такого нет.
    #[specta(type = Option<i32>)]
    pub link_property_id: Option<i64>,
}
