//! РЕЕСТР вердиктов реестрового гейта (Phase 41.7, D-07/D-08/D-16).
//!
//! Единственная точка, где освобождение от рассылки `WsEvent::EntitiesChanged`
//! становится видимым в диффе ревью. На КАЖДУЮ функцию области (`pub`,
//! `pub(crate)`, `pub(super)`, а также приватную, владеющую
//! `writer.execute(..)`) здесь стоит ровно одна строка с ЯВНЫМ вердиктом:
//!
//! * [`Verdict::Broadcasts`] — функция меняет места/состав/устройства/
//!   картриджи/группы и обязана вызывать `self.broadcast_entities("<имя>", ..)`
//!   ПОСЛЕ коммита, вне скобок `writer.execute(..)`;
//! * [`Verdict::Exempt`] — освобождение с причиной не короче
//!   [`MIN_EXEMPT_REASON_CHARS`] СИМВОЛОВ (`chars().count()`, не байт);
//! * [`Verdict::ReadOnly`] — функция ничего не пишет.
//!
//! Значения по умолчанию нет: функция без строки не «проходит», а краснит
//! сканер (план 41.7-14, вид нарушения `NoVerdict`).
//!
//! Перечень снят по исходникам `src/services/*.rs` сплошным заходом от файла
//! (включая функции ПОСЛЕ тестового модуля посреди `act_service.rs`), а не по
//! списку имён из плана.

/// Минимальная длина причины освобождения — в СИМВОЛАХ. Кириллица занимает два
/// байта, поэтому `len()` пропустил бы вдвое более короткую причину.
pub const MIN_EXEMPT_REASON_CHARS: usize = 40;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Рассылает `EntitiesChanged` после коммита.
    Broadcasts,
    /// Освобождена от рассылки; причина обязательна.
    Exempt(&'static str),
    /// Ничего не пишет.
    ReadOnly,
}

#[derive(Debug, Clone, Copy)]
pub struct Entry {
    /// Путь относительно `src/services/`.
    pub file: &'static str,
    pub func: &'static str,
    pub verdict: Verdict,
}

const fn b(file: &'static str, func: &'static str) -> Entry {
    Entry {
        file,
        func,
        verdict: Verdict::Broadcasts,
    }
}

const fn ro(file: &'static str, func: &'static str) -> Entry {
    Entry {
        file,
        func,
        verdict: Verdict::ReadOnly,
    }
}

const fn ex(file: &'static str, func: &'static str, reason: &'static str) -> Entry {
    Entry {
        file,
        func,
        verdict: Verdict::Exempt(reason),
    }
}

/// Восемь файлов области гейта (D-16).
pub const SCOPE_FILES: &[&str] = &[
    "place_service.rs",
    "group_service.rs",
    "group_place.rs",
    "group_membership.rs",
    "group_type_service.rs",
    "act_service.rs",
    "device_service.rs",
    "cartridge_service.rs",
];

/// Файл сервиса с хотя бы одной строкой `Broadcasts` -> файл поведенческого
/// теста слоя (2) в `tests/`. Множество файлов обязано совпадать в обе стороны
/// с множеством файлов, имеющих строку `Broadcasts` (`registry_shape_is_valid`).
pub const BEHAVIOUR_TEST: &[(&str, &str)] = &[
    ("place_service.rs", "entities_changed_places.rs"),
    ("group_service.rs", "entities_changed_groups.rs"),
    ("act_service.rs", "entities_changed_acts.rs"),
    ("device_service.rs", "entities_changed_devices.rs"),
    ("cartridge_service.rs", "entities_changed_cartridges.rs"),
];

/// Файлы `services/*.rs` ВНЕ области, в которых есть `writer.execute`, с
/// причиной (>= 40 символов). Пустой каркас: список составляет план 41.7-14 по
/// результату гарда области (запись в allowlist вместо включения в область —
/// только для сервисов, не меняющих места/состав/устройства/картриджи/группы).
pub const OUT_OF_SCOPE_WRITERS: &[(&str, &str)] = &[];

const PLACE: &str = "place_service.rs";
const GROUP: &str = "group_service.rs";
const GTYPE: &str = "group_type_service.rs";
const ACT: &str = "act_service.rs";
const DEVICE: &str = "device_service.rs";
const CART: &str = "cartridge_service.rs";
const GPLACE: &str = "group_place.rs";
const GMEMB: &str = "group_membership.rs";

const GTYPE_REASON: &str =
    "схема типов и свойств групп, не место и не состав; страница перечитывает по refreshToken";
const TX_PRIMITIVE_REASON: &str =
    "примитив внутри транзакции вызывающего сервиса; коммит и рассылка — у вызывающего";

pub const REGISTRY: &[Entry] = &[
    // ---- place_service ---------------------------------------------------
    ro(PLACE, "new"),
    ro(PLACE, "with_ws_tx"),
    b(PLACE, "create"),
    b(PLACE, "rename"),
    b(PLACE, "set_path_variant"),
    b(PLACE, "move_node"),
    b(PLACE, "archive"),
    b(PLACE, "unarchive"),
    ex(
        PLACE,
        "set_archived",
        "приватный владелец записи; рассылают оба публичных вызывающих — archive и unarchive, свой вызов дал бы второе событие",
    ),
    b(PLACE, "delete_hard"),
    ro(PLACE, "get"),
    ro(PLACE, "list_children"),
    ro(PLACE, "list_all"),
    ro(PLACE, "subtree_stats"),
    ro(PLACE, "full_path"),
    ro(PLACE, "list_subtree_contents"),
    b(PLACE, "move_subtree_contents"),
    ro(PLACE, "search"),
    // ---- group_service ---------------------------------------------------
    ro(GROUP, "new"),
    ro(GROUP, "with_ws_tx"),
    ro(GROUP, "list_groups"),
    ro(GROUP, "get_group"),
    ro(GROUP, "composition"),
    ro(GROUP, "search"),
    ro(GROUP, "for_devices"),
    b(GROUP, "create_group"),
    b(GROUP, "update_group"),
    b(GROUP, "delete_group"),
    b(GROUP, "move_group"),
    b(GROUP, "add_devices"),
    b(GROUP, "remove_devices"),
    b(GROUP, "set_parent"),
    ro(GROUP, "user_options"),
    ro(GROUP, "card"),
    b(GROUP, "set_values"),
    // ---- group_place (примитивы внутри транзакции) -------------------------
    ro(GPLACE, "new"),
    ex(GPLACE, "apply_group_place_to_device_in_tx", TX_PRIMITIVE_REASON),
    ex(GPLACE, "move_group_in_tx", TX_PRIMITIVE_REASON),
    ex(GPLACE, "propagate_group_place_in_tx", TX_PRIMITIVE_REASON),
    // ---- group_membership (примитивы внутри транзакции) --------------------
    ex(GMEMB, "release_device_in_tx", TX_PRIMITIVE_REASON),
    ex(GMEMB, "release_if_locked_device_in_tx", TX_PRIMITIVE_REASON),
    // ---- group_type_service ------------------------------------------------
    ro(GTYPE, "plural_groups"),
    ro(GTYPE, "new"),
    ro(GTYPE, "list_types"),
    ex(GTYPE, "create_type", GTYPE_REASON),
    ex(
        GTYPE,
        "update_type",
        "СПОРНО (Q8): переименование типа меняет подписи дерева групп, но не место и не состав; страница типов перечитывает по refreshToken",
    ),
    ex(GTYPE, "delete_type", GTYPE_REASON),
    ex(GTYPE, "create_property", GTYPE_REASON),
    ex(GTYPE, "update_property", GTYPE_REASON),
    ex(GTYPE, "delete_property", GTYPE_REASON),
    ex(GTYPE, "unarchive_property", GTYPE_REASON),
    ex(GTYPE, "reorder_properties", GTYPE_REASON),
    ro(GTYPE, "empty_groups_for_property"),
    ex(
        GTYPE,
        "seed_builtin_types_on_startup",
        "посев встроенных типов групп при старте процесса; клиентов ещё нет, состав и места не меняются",
    ),
    // ---- act_service -------------------------------------------------------
    ro(ACT, "new"),
    ro(ACT, "with_ws_tx"),
    ro(ACT, "with_pdf_pipeline"),
    ro(ACT, "with_org_db"),
    b(ACT, "create"),
    b(ACT, "update"),
    b(ACT, "do_return"),
    b(ACT, "update_return"),
    ro(ACT, "get"),
    ro(ACT, "search"),
    ro(ACT, "list"),
    ro(ACT, "counts"),
    ro(ACT, "suggest_person"),
    b(ACT, "delete_soft"),
    ro(ACT, "render_pdf"),
    ro(ACT, "render_acceptance_pdf"),
    ro(ACT, "format_ru_date"),
    ro(ACT, "format_iso_date"),
    // Единственная pub-функция ПОСЛЕ первого `#[cfg(test)]` (слепая зона отрезания хвоста).
    ro(ACT, "build_fts_query"),
    // ---- device_service ----------------------------------------------------
    ro(DEVICE, "new"),
    ro(DEVICE, "with_ws_tx"),
    ex(
        DEVICE,
        "insert_new_and_get",
        "приватный владелец записи; рассылают публичные вызывающие, свой вызов дал бы по событию на каждую строку пакета",
    ),
    b(DEVICE, "create"),
    b(DEVICE, "create_single_with_number_check_with_printer"),
    ex(
        DEVICE,
        "create_single_with_number_check",
        "тонкая обёртка над рассылающим *_with_printer; собственный вызов дал бы ДВА события на одну операцию",
    ),
    ro(DEVICE, "get"),
    ro(DEVICE, "list"),
    b(DEVICE, "update"),
    b(DEVICE, "delete_soft"),
    ro(DEVICE, "state_hints"),
    ro(DEVICE, "search"),
    ro(DEVICE, "autocomplete"),
    ro(DEVICE, "list_grouped"),
    ro(DEVICE, "status_counts"),
    ro(DEVICE, "import_csv_preview"),
    b(DEVICE, "import_csv_commit"),
    ro(DEVICE, "export_csv"),
    ro(DEVICE, "list_by_ids"),
    b(DEVICE, "bulk_create_with_printer"),
    ex(
        DEVICE,
        "bulk_create",
        "тонкая обёртка над рассылающим bulk_create_with_printer; собственный вызов дал бы ДВА события на одну операцию",
    ),
    // ---- cartridge_service -------------------------------------------------
    ro(CART, "new"),
    ro(CART, "with_ws_tx"),
    b(CART, "create"),
    b(CART, "update"),
    b(CART, "delete"),
    ro(CART, "get"),
    ro(CART, "list"),
    ro(CART, "status_counts"),
    b(CART, "transition"),
    ro(CART, "search"),
    ro(CART, "get_history"),
    ro(CART, "low_stock"),
    ro(CART, "compatible_aggregates_for_printer"),
    ro(CART, "model_list"),
    ro(CART, "model_get"),
    ex(
        CART,
        "model_create",
        "справочник моделей картриджей, не место и не состав; экран моделей перечитывает данные сам",
    ),
    ex(
        CART,
        "model_update",
        "справочник моделей картриджей, не место и не состав; экран моделей перечитывает данные сам",
    ),
    ex(
        CART,
        "model_delete",
        "справочник моделей картриджей, не место и не состав; экран моделей перечитывает данные сам",
    ),
    ro(CART, "suggest_brand"),
    ro(CART, "suggest_model"),
    ro(CART, "suggest_compat_printer"),
    ro(CART, "storage_place_ids"),
    ro(CART, "operation_default_place"),
    ro(CART, "to_refill_last_send"),
];
