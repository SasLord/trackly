//! Act DTOs — shared between Tauri command handlers and axum HTTP handlers.
//!
//! Snake_case JSON (S-2). All `i64` fields carry `#[specta(type = i32)]` so
//! TypeScript bindings see `number` rather than `bigint` — see `dto/device.rs`
//! module-doc for the rationale (S-3).
//!
//! `ActDto.number` is a `String` because the display rule «42» / «42в» /
//! «42в1» is applied at read time by the service layer
//! (`format_act_number`) — D-Numbering-01.

use serde::{Deserialize, Serialize};
use specta::Type;
use trackly_core::domain::acts::{ActCounts, ActFilter as DomainActFilter, ActRow, ActType};

/// Display-rule helper (D-Numbering-01).
///
/// - Handover: plain decimal — `"42"`
/// - Single return for a parent: drops the sub-number — `"42в"`
/// - Multiple returns for a parent: keeps it — `"42в1"`, `"42в2"`
///
/// `parent_number` and `sibling_return_count` come from the `ActRow` join
/// (see `SqliteActRepository::SELECT_ACTS`).
pub fn format_act_number(
    act_type: ActType,
    number: i64,
    sub_number: Option<i64>,
    parent_number: Option<i64>,
    sibling_return_count: Option<i64>,
) -> String {
    match act_type {
        ActType::Handover => number.to_string(),
        ActType::Return => {
            let sub = sub_number.unwrap_or(1);
            let parent = parent_number.unwrap_or(number);
            if sibling_return_count == Some(1) {
                format!("{parent}в")
            } else {
                format!("{parent}в{sub}")
            }
        }
    }
}

/// Public act DTO — what the UI receives.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ActDto {
    #[specta(type = i32)]
    pub id: i64,
    #[specta(type = i32)]
    pub version: i64,
    /// Formatted via `format_act_number` (D-Numbering-01).
    pub number: String,
    /// Raw counter value (without «в» suffix) — useful for sorting / re-display.
    #[specta(type = i32)]
    pub number_raw: i64,
    #[specta(type = Option<i32>)]
    pub sub_number: Option<i64>,
    /// `"handover"` | `"return"` (matches V004 CHECK constraint values).
    pub act_type: String,
    #[specta(type = Option<i32>)]
    pub parent_act_id: Option<i64>,
    pub giver_name: String,
    pub receiver_name: String,
    #[specta(type = Option<i32>)]
    pub location_id: Option<i64>,
    pub location: Option<String>,
    pub notes: Option<String>,
    #[specta(type = Option<i32>)]
    pub deadline_utc: Option<i64>,
    pub archived: bool,
    #[specta(type = i32)]
    pub created_at_utc: i64,
    #[specta(type = i32)]
    pub updated_at_utc: i64,
    pub items: Vec<ActItemDto>,
    /// IDs of linked return acts (for handover). Plan 02: always `[]`.
    /// Plan 03 (return lifecycle) populates this list; UI loads the full
    /// return rows separately via `acts_get` if needed.
    #[specta(type = Vec<i32>)]
    pub return_ids: Vec<i64>,
}

/// Single item line on an act (resolved with the joined device fields).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ActItemDto {
    #[specta(type = i32)]
    pub id: i64,
    #[specta(type = i32)]
    pub device_id: i64,
    #[specta(type = i32)]
    pub quantity: i64,
    pub device_name: String,
    pub inventory_no: Option<String>,
    pub serial_no: Option<String>,
    pub model: Option<String>,
    pub condition_at_time: Option<String>,
    pub complectation_at_time: Option<String>,
}

/// Payload sent by the UI when creating a handover act.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ActCreateDto {
    /// `None` → service increments `counters.act_number`.
    /// `Some(n)` → service uses `n` (audited as `custom:act_number_override`).
    #[specta(type = Option<i32>)]
    pub number_override: Option<i64>,
    pub giver_name: String,
    pub receiver_name: String,
    #[specta(type = Option<i32>)]
    pub location_id: Option<i64>,
    pub notes: Option<String>,
    #[specta(type = Option<i32>)]
    pub deadline_utc: Option<i64>,
    pub items: Vec<ActItemNewDto>,
}

/// Single item line in `ActCreateDto.items`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ActItemNewDto {
    #[specta(type = i32)]
    pub device_id: i64,
    /// Defaults to 1 in the UI; persisted in `act_items.quantity` (V014).
    #[specta(type = i32)]
    pub quantity: i64,
}

/// Switch-bar counters returned by `acts_counts`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type, Default)]
pub struct ActsCountsDto {
    #[specta(type = i32)]
    pub handover_active: i64,
    #[specta(type = i32)]
    pub returns: i64,
    #[specta(type = i32)]
    pub archived: i64,
}

impl From<ActCounts> for ActsCountsDto {
    fn from(c: ActCounts) -> Self {
        Self {
            handover_active: c.handover_active,
            returns: c.returns,
            archived: c.archived,
        }
    }
}

/// Filter passed by the UI to `acts_list`. `act_type` is the SQL string
/// (`"handover"` | `"return"` | absent).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type, Default)]
pub struct ActFilter {
    pub act_type: Option<String>,
    pub archived: Option<bool>,
    pub search: Option<String>,
    pub include_deleted: bool,
}

impl ActFilter {
    /// Convert into the domain filter, validating the act_type string.
    pub fn into_domain(self) -> Result<DomainActFilter, trackly_core::error::AppError> {
        let act_type = match self.act_type.as_deref() {
            None => None,
            Some(s) => Some(ActType::from_str(s)?),
        };
        Ok(DomainActFilter {
            act_type,
            archived: self.archived,
            search: self.search,
            include_deleted: self.include_deleted,
        })
    }
}

/// Pagination — mirrors `dto::device::Pagination` (same shape, distinct type
/// to keep modules decoupled).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct Pagination {
    #[specta(type = u32)]
    pub offset: u64,
    #[specta(type = u32)]
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

/// Response of `acts_list` / `acts_search`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ActListResponse {
    pub items: Vec<ActDto>,
    #[specta(type = u32)]
    pub total: u64,
}

/// Builder used by service `get` / `list` after fetching items separately.
///
/// `items` should already be resolved (joined with devices). `returns` is
/// populated by plan 03 — in plan 02 callers always pass `vec![]`.
pub fn act_dto_from_row(row: ActRow, items: Vec<ActItemDto>, return_ids: Vec<i64>) -> ActDto {
    let number = format_act_number(
        row.act_type,
        row.number,
        row.sub_number,
        row.parent_number,
        row.sibling_return_count,
    );
    ActDto {
        id: row.id,
        version: row.version,
        number,
        number_raw: row.number,
        sub_number: row.sub_number,
        act_type: row.act_type.to_sql().to_string(),
        parent_act_id: row.parent_act_id,
        giver_name: row.giver_name,
        receiver_name: row.receiver_name,
        location_id: row.location_id,
        location: row.location,
        notes: row.notes,
        deadline_utc: row.deadline_utc,
        archived: row.archived,
        created_at_utc: row.created_at_utc,
        updated_at_utc: row.updated_at_utc,
        items,
        return_ids,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_handover_is_plain_number() {
        assert_eq!(
            format_act_number(ActType::Handover, 42, None, None, None),
            "42"
        );
    }

    #[test]
    fn format_single_return_drops_sub_suffix() {
        assert_eq!(
            format_act_number(ActType::Return, 999, Some(1), Some(42), Some(1)),
            "42в"
        );
    }

    #[test]
    fn format_multiple_returns_use_sub_suffix() {
        assert_eq!(
            format_act_number(ActType::Return, 999, Some(1), Some(42), Some(2)),
            "42в1"
        );
        assert_eq!(
            format_act_number(ActType::Return, 1000, Some(2), Some(42), Some(2)),
            "42в2"
        );
    }

    #[test]
    fn snake_case_json_invariant() {
        let dto = ActCreateDto {
            number_override: Some(7),
            giver_name: "А".into(),
            receiver_name: "Б".into(),
            location_id: None,
            notes: None,
            deadline_utc: None,
            items: vec![],
        };
        let s = serde_json::to_string(&dto).expect("ser");
        assert!(
            s.contains("number_override"),
            "snake_case 'number_override'"
        );
        assert!(!s.contains("numberOverride"), "must NOT use camelCase");
    }
}
