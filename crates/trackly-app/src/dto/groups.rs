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
    /// Устройство-якорь группы (D-13/D-15); `None` у обычной группы. По нему UI
    /// отличает якорные группы и скрывает «Вывести из состава» у якоря (D-14).
    #[specta(type = Option<i32>)]
    pub anchor_device_id: Option<i64>,
    pub anchor_device_name: Option<String>,
    /// Код статуса устройства-якоря (D-21): UI показывает «Списать» при «На списание»;
    /// сервер перепроверяет сам. `None` — у группы нет якоря.
    pub anchor_status_code: Option<String>,
}

/// Создание группы. `name = None` — имя по умолчанию «{тип} #{seq}».
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct GroupCreateDto {
    #[specta(type = i32)]
    pub type_id: i64,
    pub name: Option<String>,
    #[specta(type = Option<i32>)]
    pub place_id: Option<i32>,
    /// Создание группы ОТ устройства (быстрые действия Phase 41.1): место группы
    /// берётся у устройства, устройство входит в состав, для замещения и разбора
    /// становится якорем. `None` — прежнее создание. Клиент без поля работает как раньше.
    #[serde(default)]
    #[specta(type = Option<i32>)]
    pub source_device_id: Option<i64>,
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
    /// Код статуса: таблица состава выбирает тон по нему (D-09).
    pub status_code: Option<String>,
}

/// Призрак разбора: деталь, выведенная из состава, но хранящая происхождение (D-10..D-14).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct GroupGhostDeviceDto {
    #[specta(type = i32)]
    pub device_id: i64,
    pub type_name: String,
    pub name: String,
    pub inventory_number: Option<String>,
    pub serial_number: Option<String>,
    pub place_path_short: Option<String>,
    pub place_path: Option<String>,
    pub status_name: Option<String>,
    pub status_code: Option<String>,
    /// Деталь мягко удалена (D-14): призрак остаётся.
    pub is_deleted: bool,
    /// Группа, в которой деталь состоит сейчас (для «Вернуть в разбор»).
    #[specta(type = Option<i32>)]
    pub current_group_id: Option<i64>,
    pub current_group_name: Option<String>,
}

/// Состав группы: прямые устройства и прямые вложенные группы (D-03).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct GroupCompositionDto {
    pub devices: Vec<GroupMemberDeviceDto>,
    pub child_groups: Vec<GroupDto>,
    /// Призраки: только у групп типа «Разбор», иначе пусто (D-10).
    pub ghosts: Vec<GroupGhostDeviceDto>,
}

/// Кандидат во вложенные группы (D-04).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct GroupSearchHitDto {
    #[specta(type = i32)]
    pub id: i64,
    pub name: String,
    pub type_name: String,
    /// Поведение типа группы (D-17): UI скрывает «Разбор» в пикерах вложения.
    pub type_behavior: String,
    #[specta(type = i32)]
    pub device_count: i64,
    pub has_parent: bool,
    /// Сокращённый путь места группы (серверная формула); `None` — «Без места».
    /// Нужен подсказке-группе пикера акта (UI-SPEC §7.2, §7.6).
    pub place_path_short: Option<String>,
}

/// Единица акта: устройство, в которое разворачивается группа (Phase 41.2, D-03).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct GroupActUnitDto {
    #[specta(type = i32)]
    pub device_id: i64,
    pub name: String,
    pub inventory_number: Option<String>,
    pub serial_number: Option<String>,
    /// Имя вложенной группы, через которую пришло устройство; `None` — устройство корня.
    pub via_group_name: Option<String>,
}

/// Разворот группы в «единицы акта» (Phase 41.2, D-03, D-10): плоский список без
/// вложенности; якорная группа даёт единственную единицу — якорь.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct GroupActExpansionDto {
    #[specta(type = i32)]
    pub group_id: i64,
    pub group_name: String,
    /// Машинный ключ поведения типа: 'container' | 'substitute' | 'teardown'.
    pub type_behavior: String,
    #[specta(type = Option<i32>)]
    pub anchor_device_id: Option<i64>,
    pub units: Vec<GroupActUnitDto>,
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
    /// D-34: сдвинуть статусы устройств при переходе «склад <-> не склад»;
    /// отсутствие поля = false.
    #[serde(default)]
    pub change_status: Option<bool>,
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
    /// D-34: у скольких устройств вместе с местом сменился статус.
    pub status_changed_devices: i32,
}

/// D-34: направление перехода через границу «склад <-> не склад».
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum GroupMoveStatusTransitionDto {
    /// Со склада на не-склад.
    ToWork,
    /// С не-склада на склад.
    ToStorage,
}

/// D-34: запрос предпросмотра смены статусов при переносе.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct GroupMoveStatusPreviewQueryDto {
    #[specta(type = i32)]
    pub id: i64,
    #[specta(type = i32)]
    pub target_place_id: i64,
}

/// D-34: предпросмотр смены статусов при переносе (считает сервер тем же кодом,
/// что и перенос).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct GroupMoveStatusPreviewDto {
    pub transition: Option<GroupMoveStatusTransitionDto>,
    pub devices_affected: i32,
    /// Код и текущая подпись целевого статуса из справочника; без перехода - None.
    pub target_status_code: Option<String>,
    pub target_status_name: Option<String>,
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

/// Перенос устройств в группу одной операцией (D-04).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct GroupMoveDevicesDto {
    #[specta(type = i32)]
    pub group_id: i64,
    #[specta(type = Vec<i32>)]
    pub device_ids: Vec<i64>,
}

/// Итог переноса (D-18): всего введено и сколько из них пришло из других групп.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct GroupMoveDevicesResultDto {
    pub added: i32,
    pub moved: i32,
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
    /// Подтверждённая смена родителя (D-16): без флага занятая группа отказывает.
    #[serde(default)]
    pub replace_parent: bool,
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

/// Подключённый принтер: USB-производный (`usb`), явная ссылка (`link`) или состоит
/// в группе (`member`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct GroupPrinterDto {
    #[specta(type = i32)]
    pub device_id: i64,
    pub name: String,
    pub inventory_number: Option<String>,
    pub serial_number: Option<String>,
    /// `usb` | `link` | `member`. При совпадении источников побеждает `usb`, затем `link`,
    /// затем `member`.
    pub origin: String,
    /// Есть явная ссылка (её можно удалить, даже если принтер виден по USB или в составе).
    pub has_explicit_link: bool,
    /// Имя USB-хоста: заполнено только при `origin = usb`, во всех остальных случаях `null`.
    /// Сервер отдаёт готовое значение, клиент не обращается к устройству хоста отдельно.
    pub usb_host_name: Option<String>,
    /// Инвентарный номер USB-хоста (различает одноимённые хосты): заполнен только при
    /// `origin = usb`, иначе `null`.
    pub usb_host_inventory_number: Option<String>,
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

/// Списать устройство (D-19, D-20): `version` — CAS по версии устройства.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct GroupWriteOffDeviceDto {
    #[specta(type = i32)]
    pub device_id: i64,
    #[specta(type = i32)]
    pub version: i64,
    /// `true` — вывести из группы; `false` — оставить в группе неактивной строкой.
    #[serde(default)]
    pub release_from_group: bool,
}

/// Списать якорь группы (D-21): `version` — CAS по версии группы.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct GroupWriteOffGroupDto {
    #[specta(type = i32)]
    pub id: i64,
    #[specta(type = i32)]
    pub version: i64,
}

/// Итог списания.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct GroupWriteOffResultDto {
    /// Освобождено из группы (устройство при выводе или члены расформированной группы).
    #[specta(type = i32)]
    pub released_devices: i32,
    /// Группа расформирована.
    pub disbanded: bool,
    #[specta(type = Vec<i32>)]
    pub changed_place_ids: Vec<i64>,
}
