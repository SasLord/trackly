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
//!   [`MIN_EXEMPT_REASON_CHARS`] СИМВОЛОВ (`chars().count()`, не байт).
//!   Exempt = «функция НЕ шлёт событие и НЕ пишет сама»: механически она не
//!   содержит `broadcast_entities(..)` (сканер краснит такой вызов —
//!   `ExemptBroadcasts`, внутри скобок writer — `BroadcastInsideWriter`) и не
//!   владеет записью (ни `writer.execute(..)` в теле, ни транзитивного
//!   self-вызова владельца записи — иначе `ExemptOwnsWriter`);
//! * [`Verdict::ExemptWrites`] (конструктор `exw`) — ГРОМКИЙ вердикт: «функция
//!   пишет, но событие осознанно не из неё» (внутренний помощник, рассылает
//!   вызывающий; запись вне области событий). Причина подчиняется тому же
//!   [`MIN_EXEMPT_REASON_CHARS`]; рассылка в теле краснит так же, как у Exempt;
//!   `exw` на функции, которая больше не пишет, краснит
//!   (`ExemptWritesWithoutWrite`: верни `ex`). WR-07 ревью 41.7, quick 261121-hjw;
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
    /// Освобождена от рассылки и сама не пишет; причина обязательна.
    Exempt(&'static str),
    /// Пишет, но событие осознанно не из этой функции; причина обязательна.
    ExemptWrites(&'static str),
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

const fn exw(file: &'static str, func: &'static str, reason: &'static str) -> Entry {
    Entry {
        file,
        func,
        verdict: Verdict::ExemptWrites(reason),
    }
}

impl Verdict {
    /// Любое освобождение от рассылки (Exempt или ExemptWrites).
    pub const fn is_exempt(&self) -> bool {
        matches!(self, Verdict::Exempt(_) | Verdict::ExemptWrites(_))
    }

    /// Причина освобождения (для обоих вариантов).
    pub const fn exempt_reason(&self) -> Option<&'static str> {
        match self {
            Verdict::Exempt(r) | Verdict::ExemptWrites(r) => Some(r),
            _ => None,
        }
    }
}

/// Девять файлов области гейта (D-16; `repair_service.rs` добавлен в Phase 41.4, план 09).
pub const SCOPE_FILES: &[&str] = &[
    "place_service.rs",
    "group_service.rs",
    "group_place.rs",
    "group_membership.rs",
    "group_type_service.rs",
    "act_service.rs",
    "device_service.rs",
    "cartridge_service.rs",
    "repair_service.rs",
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
    ("repair_service.rs", "entities_changed_repairs.rs"),
];

/// Файлы `services/*.rs` ВНЕ области, в которых есть `writer.execute` (вне
/// тестовых элементов), с причиной (>= 40 символов). Составлен план 41.7-14 по
/// результату гарда области (`writer_owning_services_outside_scope_are_listed`)
/// и ПРОЧТЕНИЮ каждого файла: ни один не меняет места, состав мест, устройства,
/// картриджи или группы — иначе он вошёл бы в `SCOPE_FILES`, а не сюда (D-16).
/// `place_movement_service.rs` здесь не нужен: `writer` в нём — только слово в
/// doc-комментарии, сервис читает таймлайн перемещений и ничего не пишет.
pub const OUT_OF_SCOPE_WRITERS: &[(&str, &str)] = &[
    (
        "attachment_service.rs",
        "вложения меняют только файлы и метаданные: места, состав, устройства, картриджи и группы не затрагиваются; сигнал отдельное событие AttachmentsChanged (D-25)",
    ),
    (
        "attachment_admin.rs",
        "лимит размера вложений и отчёт целостности в app_settings; состав мест, устройств, картриджей и групп не меняется",
    ),
    (
        "auth.rs",
        "пользователи, сессии, настройки AD и аудит входа; ни места, ни устройства, ни картриджи, ни группы не затрагиваются",
    ),
    (
        "backup_service.rs",
        "настройки расписания резервных копий в app_settings; копирование БД читает, а не пишет, данные учёта",
    ),
    (
        "chat_service.rs",
        "переписка по заявкам: отметка прочтения в chat_reads, заглушённые заявки в chat_mutes, сообщения в chat_messages и черновики вложений; места, состав, устройства, картриджи и группы не затрагиваются; сигнал отдельные события ChatMessage и ChatReadChanged (D-19, D-23), черновики без сигнала",
    ),
    (
        "number_template_service.rs",
        "шаблоны номеров и память последнего шаблона; сами номера устройств, актов и картриджей он не меняет",
    ),
    (
        "org_db_service.rs",
        "реквизиты и логотип организации в org_settings для печатных форм; данные учёта не затрагиваются",
    ),
    (
        "printer_service.rs",
        "SNMP-опрос принтеров: показания, тревоги и строка printers; таблицы devices, places и cartridges не меняются",
    ),
    (
        "request_service.rs",
        "жизненный цикл заявок (requests) и одобрение AD-регистрации; устройства и места он не пишет, ремонтные записи вынесены в RepairService; привязанный картридж только читается",
    ),
    (
        "supervisor.rs",
        "фоновый планировщик: статусы и время запуска в scheduled_tasks; данные учёта не затрагиваются",
    ),
    (
        "template_service.rs",
        "шаблоны печатных форм document_templates и их посев при старте; места, состав и устройства не затрагиваются",
    ),
];

const PLACE: &str = "place_service.rs";
const GROUP: &str = "group_service.rs";
const GTYPE: &str = "group_type_service.rs";
const ACT: &str = "act_service.rs";
const DEVICE: &str = "device_service.rs";
const CART: &str = "cartridge_service.rs";
const GPLACE: &str = "group_place.rs";
const REPAIR: &str = "repair_service.rs";
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
    exw(
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
    ro(PLACE, "group_members"),
    b(PLACE, "move_subtree_contents"),
    ro(PLACE, "search"),
    // ---- group_service ---------------------------------------------------
    ro(GROUP, "new"),
    ro(GROUP, "with_ws_tx"),
    ro(GROUP, "list_groups"),
    ro(GROUP, "get_group"),
    ro(GROUP, "composition"),
    ro(GROUP, "act_expansion"),
    ro(GROUP, "search"),
    ro(GROUP, "for_devices"),
    b(GROUP, "create_group"),
    b(GROUP, "update_group"),
    b(GROUP, "delete_group"),
    b(GROUP, "move_group"),
    b(GROUP, "add_devices"),
    b(GROUP, "remove_devices"),
    b(GROUP, "move_devices"),
    b(GROUP, "write_off_device"),
    b(GROUP, "write_off_group"),
    b(GROUP, "set_parent"),
    // D-28: примитив внутри писательской транзакции создания детали; событие шлёт
    // вызывающий device_service (create/bulk_create_with_printer, group_ids = группа разбора).
    ex(GROUP, "join_origin_group_in_tx", TX_PRIMITIVE_REASON),
    ro(GROUP, "user_options"),
    ro(GROUP, "card"),
    b(GROUP, "set_values"),
    // ---- group_place (примитивы внутри транзакции) -------------------------
    ro(GPLACE, "new"),
    // Конструктор причины переноса (GroupMoveCause::act): чистая const-функция, ничего не пишет.
    ro(GPLACE, "act"),
    ex(GPLACE, "apply_group_place_to_device_in_tx", TX_PRIMITIVE_REASON),
    ex(GPLACE, "move_group_in_tx", TX_PRIMITIVE_REASON),
    ex(GPLACE, "move_nested_block_in_tx", TX_PRIMITIVE_REASON),
    ex(GPLACE, "propagate_group_place_in_tx", TX_PRIMITIVE_REASON),
    // ---- group_membership (примитивы внутри транзакции) --------------------
    ex(GMEMB, "release_device_in_tx", TX_PRIMITIVE_REASON),
    ex(GMEMB, "release_if_locked_device_in_tx", TX_PRIMITIVE_REASON),
    ex(GMEMB, "restore_membership_in_tx", TX_PRIMITIVE_REASON),
    ex(GMEMB, "restore_membership_with_policy_in_tx", TX_PRIMITIVE_REASON),
    ex(GMEMB, "anchor_restore_conflict_in_tx", TX_PRIMITIVE_REASON),
    // ---- group_type_service ------------------------------------------------
    ro(GTYPE, "plural_groups"),
    ro(GTYPE, "new"),
    ro(GTYPE, "list_types"),
    exw(GTYPE, "create_type", GTYPE_REASON),
    exw(
        GTYPE,
        "update_type",
        "СПОРНО (Q8): переименование типа меняет подписи дерева групп, но не место и не состав; страница типов перечитывает по refreshToken",
    ),
    exw(GTYPE, "delete_type", GTYPE_REASON),
    exw(GTYPE, "create_property", GTYPE_REASON),
    exw(GTYPE, "update_property", GTYPE_REASON),
    exw(GTYPE, "delete_property", GTYPE_REASON),
    exw(GTYPE, "unarchive_property", GTYPE_REASON),
    exw(GTYPE, "reorder_properties", GTYPE_REASON),
    ro(GTYPE, "empty_groups_for_property"),
    exw(
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
    // Тонкая обёртка над `do_return_with_split` (D-45): сама не пишет и не рассылает.
    exw(
        ACT,
        "do_return",
        "делегирует в do_return_with_split с выбором Release; запись и рассылка живут там",
    ),
    // Предпросмотр разбиения группы: только `readers`, ничего не пишет.
    ro(ACT, "return_group_split_preview"),
    b(ACT, "do_return_with_split"),
    // Тонкая обёртка над `update_return_with_split` (D-45): сама не пишет и не рассылает.
    exw(
        ACT,
        "update_return",
        "делегирует в update_return_with_split с выбором Release; запись и рассылка живут там",
    ),
    // Предпросмотр разбиения группы при правке возврата (CR-01): прогон
    // `update_return_with_split` в режиме Preview, его транзакция всегда откатывается.
    exw(
        ACT,
        "update_return_group_split_preview",
        "предпросмотр: update_return_with_split в режиме Preview откатывает транзакцию writer'а до коммита и не рассылает",
    ),
    b(ACT, "update_return_with_split"),
    ro(ACT, "get"),
    ro(ACT, "search"),
    ro(ACT, "list"),
    ro(ACT, "counts"),
    ro(ACT, "suggest_person"),
    // Тонкая обёртка над `delete_soft_with_choices` (D-42): writer.execute сама не зовёт и не
    // рассылает, но пишет транзитивно через делегата — поэтому exw (WR-07).
    exw(
        ACT,
        "delete_soft",
        "делегирует в delete_soft_with_choices с пустым списком выборов; запись и рассылка живут там",
    ),
    b(ACT, "delete_soft_with_choices"),
    ro(ACT, "render_pdf"),
    ro(ACT, "render_acceptance_pdf"),
    ro(ACT, "format_ru_date"),
    ro(ACT, "format_iso_date"),
    // Сериализует строку устройства для аудита и ничего не пишет; pub(crate) ради group_act_plan.
    ro(ACT, "device_snapshot_json"),
    // Единственная pub-функция ПОСЛЕ первого `#[cfg(test)]` (слепая зона отрезания хвоста).
    ro(ACT, "build_fts_query"),
    // ---- device_service ----------------------------------------------------
    ro(DEVICE, "new"),
    ro(DEVICE, "with_ws_tx"),
    exw(
        DEVICE,
        "insert_new_and_get",
        "приватный владелец записи; рассылают публичные вызывающие, свой вызов дал бы по событию на каждую строку пакета",
    ),
    // Тонкая обёртка над `create_with_printer_block` (quick 261105-s6s): сама не пишет и не рассылает.
    exw(
        DEVICE,
        "create",
        "делегирует в create_with_printer_block с блоком None; запись и рассылка живут там",
    ),
    b(DEVICE, "create_with_printer_block"),
    b(DEVICE, "create_single_with_number_check_with_printer"),
    exw(
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
    ro(DEVICE, "list_entries"),
    ro(DEVICE, "list_containers"),
    ro(DEVICE, "group_children"),
    ro(DEVICE, "status_counts"),
    ro(DEVICE, "list_statuses"),
    ro(DEVICE, "list_types"),
    ro(DEVICE, "import_csv_preview"),
    b(DEVICE, "import_csv_commit"),
    ro(DEVICE, "export_csv"),
    ro(DEVICE, "export_csv_with_limit"),
    ro(DEVICE, "list_by_ids"),
    b(DEVICE, "bulk_create_with_printer"),
    exw(
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
    exw(
        CART,
        "model_create",
        "справочник моделей картриджей, не место и не состав; экран моделей перечитывает данные сам",
    ),
    exw(
        CART,
        "model_update",
        "справочник моделей картриджей, не место и не состав; экран моделей перечитывает данные сам",
    ),
    exw(
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
    // ---- repair_service (Phase 41.4, план 09) -------------------------------
    ro(REPAIR, "new"),
    ro(REPAIR, "with_ws_tx"),
    ro(REPAIR, "device_options"),
    ro(REPAIR, "open_requests_for_devices"),
    ro(REPAIR, "last_places"),
    ro(REPAIR, "get_state"),
    ro(REPAIR, "device_history"),
    // 41.4-47: предикат роли (чистая функция), используется и request_service.rs
    ro(REPAIR, "is_staff"),
    // ---- переходы цикла (Phase 41.4, план 10) -------------------------------
    b(REPAIR, "accept"),
    b(REPAIR, "move_to_storage"),
    // ---- заход в сервис (Phase 41.4, план 12) -------------------------------
    b(REPAIR, "send"),
    b(REPAIR, "receive"),
    exw(
        REPAIR,
        "link_scans",
        "связывает уже загруженные вложения с заходом и не меняет места, устройства и группы; сигнал отдельным WsEvent RequestStatusChanged участникам заявки (NEW-7)",
    ),
    // ---- выход из цикла (Phase 41.4, план 13) -------------------------------
    b(REPAIR, "complete"),
    b(REPAIR, "cancel_repair"),
    b(REPAIR, "reject"),
    b(REPAIR, "cancel_by_author"),
    b(REPAIR, "assign_device"),
    // Дальнейшие переходы (планы 14+) добавят строки Broadcasts.
    exw(
        REPAIR,
        "create",
        "создание заявки не меняет места, устройства и группы; сигнал отдельные WsEvent NewRequest и RequestStatusChanged, а новая заявка на устройство — EmployeeScopeChanged для пометки «Есть заявка» у сотрудников (NEW-4)",
    ),
    exw(
        REPAIR,
        "merge_into_open",
        "слияние заявок не меняет места, устройства и группы; сообщения чата поглощаемой заявки переносятся в целевую (D-09); сигнал отдельным WsEvent RequestStatusChanged участникам обеих заявок; вложения цели (D-35) сигналятся отдельным WsEvent AttachmentsChanged",
    ),
];
