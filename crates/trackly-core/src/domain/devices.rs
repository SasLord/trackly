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
    pub inventory_no: Option<String>,
    pub serial_no: Option<String>,
    pub model: Option<String>,
    pub specs: Option<String>,
    pub kit: Option<String>,
    pub state: Option<String>,
    pub place_id: Option<i64>,
    pub status_id: Option<i64>,
}

/// Filter parameters for device list/search queries.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DeviceFilter {
    pub type_id: Option<i64>,
    pub place_id: Option<i64>,
    pub status_id: Option<i64>,
    pub state: Option<String>,
    /// Многополевой FTS5-текстовый фильтр (Phase 18/AUTO-03).
    /// Используется ТОЛЬКО внутри `list_grouped` при `group_by_condition=true` —
    /// сопоставляет name/inventory_number/serial_number/model через
    /// `build_fts_query` sanitizer + `devices_fts MATCH`. В `list()`/`export_csv`
    /// поле остаётся неиспользуемым (pre-existing gap, вне скоупа Phase 18).
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
