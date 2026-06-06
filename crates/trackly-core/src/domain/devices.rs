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
    pub location_id: Option<i64>,
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
    pub location_id: Option<i64>,
    pub status_id: Option<i64>,
}

/// Filter parameters for device list/search queries.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DeviceFilter {
    pub type_id: Option<i64>,
    pub location_id: Option<i64>,
    pub status_id: Option<i64>,
    pub state: Option<String>,
    pub name_prefix: Option<String>,
    /// Whether to include soft-deleted devices. Defaults to false.
    pub include_deleted: bool,
    /// Если true — GROUP BY включает d.condition (для акт-формы).
    /// Если false (по умолчанию) — без разбивки по condition (для страницы Устройств).
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
    pub location_id: Option<i64>,
    /// Resolved location name from the `locations` table (via LEFT JOIN on read paths).
    pub location: Option<String>,
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
    /// > 1 means the group has mixed conditions (only relevant when group_by_condition=false).
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
    /// Autocomplete returns distinct `locations.name` values via JOIN.
    /// Filtered by ctx_status_id / ctx_name when provided.
    Location,
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
            "location" => Ok(Self::Location),
            other => Err(AppError::Validation {
                field: "field".to_string(),
                message: format!(
                    "Неподдерживаемое поле автодополнения: «{other}». \
                     Поддерживаемые поля: name, model, specs, kit, state, location."
                ),
            }),
        }
    }

    /// Returns the SQL column name corresponding to this field.
    ///
    /// Column names come **only** from this match — user input is never interpolated.
    /// `Location` is handled separately in the adapter (JOIN query) — callers must
    /// check `AutocompleteField::is_location()` before using this method for that variant.
    pub fn sql_column(self) -> &'static str {
        match self {
            Self::Name => "name",
            Self::Model => "model",
            Self::Specs => "notes",
            Self::Kit => "complectation",
            Self::State => "condition",
            // Location is resolved via locations JOIN — this fallback is unused but
            // needs a value to satisfy the match.
            Self::Location => "location_id",
        }
    }

    /// Returns `true` for the `Location` variant, which requires special JOIN handling.
    pub fn is_location(self) -> bool {
        matches!(self, Self::Location)
    }
}
