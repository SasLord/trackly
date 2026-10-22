//! Domain value types for the Devices entity.
//!
//! These types use UI-friendly field names (Path B from Phase 2 PATTERNS.md):
//! - `inventory_no` (mapped to SQL `inventory_number` in DTO layer)
//! - `serial_no`    (mapped to SQL `serial_number`)
//! - `state`        (mapped to SQL `condition`)
//! - `kit`          (mapped to SQL `complectation`)
//! - `specs`        (mapped to SQL `notes`)
//!
//! NO serde::Serialize/Deserialize or specta::Type derives here — those live
//! in the DTO layer in trackly-app. Only `#[derive(Debug, Clone, PartialEq, Eq)]`.

use crate::error::AppError;

/// Data needed to create a new device record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceNew {
    pub type_id: i64,
    pub name: String,
    pub inventory_no: Option<String>,
    pub serial_no: Option<String>,
    pub model: Option<String>,
    pub specs: Option<String>,
    pub kit: Option<String>,
    pub state: Option<String>,
    pub place_id: Option<i64>,
    pub status_id: i64,
}

/// Partial update for a device — all fields optional.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DevicePatch {
    pub type_id: Option<i64>,
    pub name: Option<String>,
    /// Двойной `Option` (Phase 40.2 Plan 08, D-08/NUM-09): `None` — поле не
    /// передано, номер не меняется; `Some(None)` — передано явно с
    /// намерением очистить номер (-> NULL); `Some(Some(v))` — передано с
    /// новым (уже `.trim()`-нутым, проверенным на occupied/script-mix)
    /// значением. Тот же паттерн, что `place_id` ниже — плоский
    /// `Option<String>` делал "не передано" и "передано явно как пустая
    /// строка" неразличимыми на уровне SQL COALESCE, а с частичным
    /// UNIQUE INDEX (V044) две записи, обе очищенные до `''` (а не NULL),
    /// стали бы ложно конфликтовать друг с другом.
    pub inventory_no: Option<Option<String>>,
    /// Двойной `Option`: `None` — поле не передано, серийный номер не
    /// меняется; `Some(None)` — передано явно с намерением очистить
    /// (serial_number -> NULL); `Some(Some(v))` — передано с новым
    /// значением. Тот же паттерн, что `inventory_no`/`place_id` в этом же
    /// struct — плоский `Option<String>` делал "не передано" и "передано
    /// явно как NULL" неразличимыми на уровне SQL COALESCE (WR-03,
    /// Milestone v1.4 audit): очистка серийного номера через форму правки
    /// устройства молча не срабатывала.
    pub serial_no: Option<Option<String>>,
    /// Двойной `Option`: `None` — поле не передано, модель не меняется;
    /// `Some(None)` — передано явно с намерением очистить (model -> NULL);
    /// `Some(Some(v))` — передано с новым значением. См. `serial_no` выше
    /// (WR-03).
    pub model: Option<Option<String>>,
    /// Двойной `Option`: `None` — поле не передано, характеристики
    /// (SQL `notes`) не меняются; `Some(None)` — передано явно с намерением
    /// очистить; `Some(Some(v))` — передано с новым значением. См.
    /// `serial_no` выше (WR-03).
    pub specs: Option<Option<String>>,
    /// Двойной `Option`: `None` — поле не передано, комплектация
    /// (SQL `complectation`) не меняется; `Some(None)` — передано явно с
    /// намерением очистить; `Some(Some(v))` — передано с новым значением.
    /// См. `serial_no` выше (WR-03).
    pub kit: Option<Option<String>>,
    /// Двойной `Option`: `None` — поле не передано, состояние
    /// (SQL `condition`) не меняется; `Some(None)` — передано явно с
    /// намерением очистить; `Some(Some(v))` — передано с новым значением.
    /// См. `serial_no` выше (WR-03).
    pub state: Option<Option<String>>,
    /// Двойной `Option`: `None` — поле не передано, значение места не
    /// меняется; `Some(None)` — поле передано явно с намерением очистить
    /// место (place_id -> NULL); `Some(Some(id))` — поле передано с новым
    /// местом. Уплощение в одинарный `Option<i64>` (как раньше) делает
    /// "не передано" и "передано явно как NULL" неразличимыми на уровне SQL
    /// COALESCE — Phase 40-28 (CR-03 test-blocker): очистка места принтера
    /// через `DeviceService::update` молча не срабатывала.
    pub place_id: Option<Option<i64>>,
    pub status_id: Option<i64>,
}

/// Filter parameters for device list/search queries.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DeviceFilter {
    pub type_id: Option<i64>,
    /// Место с поддеревом (Phase 41.11, D-02): само место и все вложенные, без
    /// удалённых. Применяется в `list`/`list_all`; `list_grouped` его игнорирует.
    pub place_id: Option<i64>,
    pub status_id: Option<i64>,
    /// Состояние (`condition`, свободный текст): нормализованное равенство — `trim`
    /// и без учёта регистра, включая кириллицу (Phase 41.11, D-04). Пустая после
    /// `trim` строка — «не задан». Применяется в `list`/`list_all`.
    pub state: Option<String>,
    /// Многополевой FTS5-текстовый фильтр (Phase 18/AUTO-03, Phase 41.11/D-03):
    /// сопоставляет name/inventory_number/serial_number/model через
    /// `build_fts_query` sanitizer + `devices_fts MATCH`. Одно поведение во всех
    /// путях: `list`/`list_all`/`export_csv` применяют его всегда, `list_grouped` —
    /// только при `group_by_condition=true`. Пустая после `trim` строка — «не задан».
    pub name_prefix: Option<String>,
    /// Whether to include soft-deleted devices. Defaults to false.
    pub include_deleted: bool,
    /// Если true (акт-форма/пикер устройства, Phase 18/D-04/D-05) — `list_grouped`
    /// группирует по `(type_id, name, model)` (НЕ по condition), сортирует группы
    /// по `count DESC` (остаток по убыванию), и поддерживает текстовый фильтр
    /// через `name_prefix`.
    /// Если false (по умолчанию, страница Устройств) — группировка по
    /// `(type_id, name)`, сортировка по имени; поведение не изменено Phase 18.
    pub group_by_condition: bool,
    /// Phase 41.1 (D-10), opt-in: скрыть устройства, входящие в якорные группы
    /// (включая вложенные), кроме самого якоря. Пикеры актов флаг не ставят.
    pub hide_anchor_members: bool,
    /// Phase 41.1, opt-in: вернуть только якорные устройства.
    pub anchors_only: bool,
    /// Phase 41.1 (D-11), opt-in: исключить якорные устройства (используется
    /// в `list_grouped`, чтобы якорь не попадал в свёртку одинаковых).
    pub exclude_anchors: bool,
    /// Phase 41.1, opt-in: вернуть устройства группы и её вложенных групп
    /// без якоря этой группы.
    pub member_of_group_id: Option<i64>,
    /// Phase 41.2 (D-02), opt-in: только устройства, состоящие в живой группе;
    /// фильтр статуса к ним не применяется вызывающим. Сужает только `list`.
    pub group_members_only: bool,
}

/// Pagination parameters for list queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pagination {
    pub offset: u64,
    pub limit: u64,
}

impl Default for Pagination {
    fn default() -> Self {
        Self {
            offset: 0,
            limit: 50,
        }
    }
}

/// A single device row as returned from the repository read path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceRow {
    pub id: i64,
    pub type_id: i64,
    pub name: String,
    pub inventory_no: Option<String>,
    pub serial_no: Option<String>,
    pub model: Option<String>,
    pub specs: Option<String>,
    pub kit: Option<String>,
    pub state: Option<String>,
    pub place_id: Option<i64>,
    /// Resolved full path from `place_full_paths` (via LEFT JOIN on read paths).
    pub full_path: Option<String>,
    /// Resolved short path per `place_effective_variant` + `shorten_place_path`
    /// (Phase 39.1 / PLC-08). Populated ONLY on `list`/`search_fts`/`list_grouped`
    /// (D-17); `None` on `autocomplete`/`restore_from_snapshot_in_tx`/`get`/
    /// `list_by_ids` (D-19 — those read paths never join `place_effective_variant`)
    /// and on any row where `place_id IS NULL`.
    pub place_path_short: Option<String>,
    pub status_id: i64,
    pub created_at_utc: i64,
    pub updated_at_utc: i64,
    pub deleted_at_utc: Option<i64>,
    pub version: i64,
    /// Машинный ключ статуса (`device_statuses.code`) для `status_id`. Меню строки
    /// и тон бейджа решаются по нему, а не по русской подписи (Phase 41.1, D-02/D-06).
    pub status_code: String,
    /// Неэксклюзивная ссылка на группу разбора, из которой деталь происходит
    /// (`devices.origin_group_id`, Phase 41.1, D-16). Вход и выход из других групп
    /// её не меняют. В `list_grouped` у представителя группы всегда `None`.
    pub origin_group_id: Option<i64>,
}

/// A grouped device row for non-unique devices (D-Group-01).
/// Represents multiple devices with the same (type, name, model, specs, kit, state, location, status).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceGroupRow {
    /// Representative row data (first match in group).
    pub repr: DeviceRow,
    /// All device IDs in this group.
    pub ids: Vec<i64>,
    /// Total count in this group.
    pub count: i64,
    /// Number of distinct condition values in this group.
    /// When `group_by_condition=false`: > 1 means the group has mixed conditions
    /// (displayed as «разное» on the frontend).
    /// When `group_by_condition=true` (Phase 18+): condition is NO LONGER part of
    /// the group key (the key is `(type_id, name, model)`) — this field instead
    /// signals to the frontend that a drill-in sub-grouping by condition is needed
    /// (D-07), it does not reflect variation within the group key itself.
    pub condition_distinct_count: i64,
    /// Number of distinct place values in this group (Phase 40 Plan 26 — Фикс B).
    /// A value greater than 1 means the group's members are spread across
    /// different places; the frontend must suppress `repr.place_path_short`
    /// (show «—») instead of showing a place belonging to an arbitrary
    /// member (`MAX(d2.place_id)`).
    pub place_distinct_count: i64,
}

/// Whitelisted fields for autocomplete queries (D-AutocompleteEndpoint-01, T-02-04-02).
///
/// This enum prevents SQL injection through the `field` parameter — only
/// columns in this list are permitted; the SQL column name is derived from
/// the enum value, never from user input directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutocompleteField {
    /// `name` column
    Name,
    /// `model` column
    Model,
    /// `notes` column (specs in DTO)
    Specs,
    /// `complectation` column (kit in DTO)
    Kit,
    /// `condition` column (state in DTO)
    State,
}

impl AutocompleteField {
    /// Parse from a user-supplied string. Returns `AppError::Validation` for unknown values.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Result<Self, AppError> {
        match s {
            "name" => Ok(Self::Name),
            "model" => Ok(Self::Model),
            "specs" => Ok(Self::Specs),
            "kit" => Ok(Self::Kit),
            "state" => Ok(Self::State),
            other => Err(AppError::Validation {
                field: "field".to_string(),
                message: format!(
                    "Неподдерживаемое поле автодополнения: «{other}». \
                     Поддерживаемые поля: name, model, specs, kit, state."
                ),
            }),
        }
    }

    /// Returns the SQL column name corresponding to this field.
    ///
    /// Column names come **only** from this match — user input is never interpolated.
    pub fn sql_column(self) -> &'static str {
        match self {
            Self::Name => "name",
            Self::Model => "model",
            Self::Specs => "notes",
            Self::Kit => "complectation",
            Self::State => "condition",
        }
    }
}
