//! Place DTOs — transport contracts for the Places entity (Phase 39).
//!
//! `PlaceDto`/`PlaceTreeNodeDto`/`PlacePathDto` are the SOLE source of response shapes
//! for every downstream Phase 39 plan (08/12/13/14/19/20) — defined here, before any
//! transport-layer consumer, mirroring `dto/device.rs`'s `DeviceDto` convention exactly.
//!
//! Snake_case JSON — НИКАКИХ `rename_all = "camelCase"` (PATTERNS.md §Pattern 3).
//!
//! `domain::places::PlaceKind` has no serde/specta derive by design (Plan 02) — this
//! DTO layer owns the string conversion via `PlaceKind::as_str()`.

use serde::{Deserialize, Serialize};
use specta::Type;
use trackly_core::domain::places::{PlaceContentRow, PlaceKind, PlaceRow, SubtreeStats};
use trackly_core::error::AppError;

/// Place DTO — full field set returned to the frontend. Mirrors `PlaceRow` 1:1
/// except `deleted_at_utc` (internal soft-delete marker, never exposed on the wire).
///
/// `#[specta(type = i32)]` on every `i64` field — specta-typescript forbids BigInt
/// (i64/u64) by default; IDs/versions/timestamps all fit in i32 range for this app.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct PlaceDto {
    #[specta(type = i32)]
    pub id: i64,
    #[specta(type = Option<i32>)]
    pub parent_id: Option<i64>,
    /// One of the six closed `PlaceKind` tokens (`territory`/`zone`/`building`/`floor`/
    /// `room`/`outdoor`) — produced via `PlaceKind::as_str().to_string()`.
    pub kind: String,
    pub name: String,
    #[specta(type = Option<i32>)]
    pub level: Option<i64>,
    pub is_storage: bool,
    #[specta(type = Option<i32>)]
    pub sort_order: Option<i64>,
    #[specta(type = Option<i32>)]
    pub archived_at_utc: Option<i64>,
    pub notes: Option<String>,
    /// Resolved root-to-leaf path (via `place_full_paths`, always live — never cached).
    pub full_path: Option<String>,
    /// Raw per-place override token (`"ends"`/`"last_two"`/`"last"`), or `None` for
    /// «Как у родителя» (D-06). Populated for the place form to show the current
    /// value — NOT the resolved effective variant (that never reaches the wire here).
    pub path_variant_override: Option<String>,
    #[specta(type = i32)]
    pub created_at_utc: i64,
    #[specta(type = i32)]
    pub updated_at_utc: i64,
    #[specta(type = i32)]
    pub version: i64,
}

impl From<PlaceRow> for PlaceDto {
    fn from(row: PlaceRow) -> Self {
        Self {
            id: row.id,
            parent_id: row.parent_id,
            kind: row.kind.as_str().to_string(),
            name: row.name,
            level: row.level,
            is_storage: row.is_storage,
            sort_order: row.sort_order,
            archived_at_utc: row.archived_at_utc,
            notes: row.notes,
            full_path: row.full_path,
            path_variant_override: row.path_variant_override.map(|v| v.as_str().to_string()),
            created_at_utc: row.created_at_utc,
            updated_at_utc: row.updated_at_utc,
            version: row.version,
        }
    }
}

/// Tree-node DTO — every `PlaceDto` field plus `content_count` (D-25's per-node
/// counter, "sum including nested places"). Populated by Plan 08's subtree-stats-
/// backed list methods — this plan (05) only defines the shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct PlaceTreeNodeDto {
    #[specta(type = i32)]
    pub id: i64,
    #[specta(type = Option<i32>)]
    pub parent_id: Option<i64>,
    pub kind: String,
    pub name: String,
    #[specta(type = Option<i32>)]
    pub level: Option<i64>,
    pub is_storage: bool,
    #[specta(type = Option<i32>)]
    pub sort_order: Option<i64>,
    #[specta(type = Option<i32>)]
    pub archived_at_utc: Option<i64>,
    pub notes: Option<String>,
    pub full_path: Option<String>,
    #[specta(type = i32)]
    pub created_at_utc: i64,
    #[specta(type = i32)]
    pub updated_at_utc: i64,
    #[specta(type = i32)]
    pub version: i64,
    /// D-25: sum of devices/printers/cartridges/nested places under this node,
    /// INCLUDING nested places (Pattern 2 subtree-stats sum, not just direct children).
    #[specta(type = i32)]
    pub content_count: i64,
}

/// Place-search result row (the `places_search` result shape, D-17's flat-list
/// search-by-full-path mode). Populated by Plan 08 Task 2 — this plan only defines
/// the shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct PlacePathDto {
    #[specta(type = i32)]
    pub place_id: i64,
    pub full_path: String,
    pub kind: String,
}

/// Wire-facing input for `places_create` (Plan 12). Mirrors `DeviceNew`'s
/// dto/domain split convention: `domain::places::PlaceNew` has no serde/specta
/// derives by design (Plan 02), so this DTO is the sole `#[tauri::command]`/HTTP
/// input shape — converted via [`PlaceNewDto::into_domain`], which is fallible
/// because `kind` is a caller-supplied string that must be validated against the
/// six closed `PlaceKind` tokens (`PlaceKind::from_str`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct PlaceNewDto {
    #[specta(type = Option<i32>)]
    pub parent_id: Option<i64>,
    pub kind: String,
    pub name: String,
    #[specta(type = Option<i32>)]
    pub level: Option<i64>,
    pub is_storage: bool,
    #[specta(type = Option<i32>)]
    pub sort_order: Option<i64>,
    pub notes: Option<String>,
}

impl PlaceNewDto {
    /// Fallible conversion into the domain `PlaceNew` — validates `kind` against
    /// the six closed tokens via `PlaceKind::from_str`, returning the same
    /// Russian-language `AppError::Validation` the domain layer already produces
    /// for an unrecognized token.
    pub fn into_domain(self) -> Result<trackly_core::domain::places::PlaceNew, AppError> {
        Ok(trackly_core::domain::places::PlaceNew {
            parent_id: self.parent_id,
            kind: PlaceKind::from_str(&self.kind)?,
            name: self.name,
            level: self.level,
            is_storage: self.is_storage,
            sort_order: self.sort_order,
            notes: self.notes,
        })
    }
}

/// Wire-facing output for `places_subtree_stats` (Plan 12). `domain::places::
/// SubtreeStats` has no serde/specta derives (Plan 02 domain-layer convention),
/// so this DTO is the sole transport shape for D-14/D-21/D-25/PLC-06 counters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct SubtreeStatsDto {
    #[specta(type = i32)]
    pub direct_children: i64,
    #[specta(type = i32)]
    pub nested_places: i64,
    #[specta(type = i32)]
    pub device_count: i64,
    #[specta(type = i32)]
    pub cartridge_count: i64,
    /// CR-01 (phase 39 review): acts still referencing this subtree via a D-16
    /// frozen snapshot column, even though every device/cartridge/child place
    /// may already be gone. Currently used by `PlaceService::delete_hard`'s
    /// pre-flight check; not yet surfaced as a distinct UI badge (WR-01 tracks
    /// the separate stats-cache-never-invalidates issue for the badges already
    /// on screen).
    #[specta(type = i32)]
    pub referencing_act_count: i64,
}

impl From<SubtreeStats> for SubtreeStatsDto {
    fn from(s: SubtreeStats) -> Self {
        Self {
            direct_children: s.direct_children,
            nested_places: s.nested_places,
            device_count: s.device_count,
            cartridge_count: s.cartridge_count,
            referencing_act_count: s.referencing_act_count,
        }
    }
}

/// Wire-facing output row for `places_contents` (PLC-06 / D-23 — the "content of
/// place" listing). Mirrors `domain::places::PlaceContentRow` 1:1.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct PlaceContentDto {
    pub kind: String,
    #[specta(type = i32)]
    pub id: i64,
    pub name: String,
    pub inventory_or_code: Option<String>,
    pub full_path: String,
    /// Short path per the effective D-06 variant (D-17), symmetric with the
    /// device/cartridge list `place_path_short` field.
    pub place_path_short: String,
    pub status_name: Option<String>,
}

impl From<PlaceContentRow> for PlaceContentDto {
    fn from(row: PlaceContentRow) -> Self {
        Self {
            kind: row.kind,
            id: row.id,
            name: row.name,
            inventory_or_code: row.inventory_or_code,
            full_path: row.full_path,
            place_path_short: row.place_path_short,
            status_name: row.status_name,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use trackly_core::domain::places::PlaceKind;

    fn sample_row() -> PlaceRow {
        PlaceRow {
            id: 42,
            parent_id: Some(7),
            kind: PlaceKind::Room,
            name: "214".to_string(),
            level: None,
            is_storage: false,
            sort_order: None,
            archived_at_utc: None,
            notes: None,
            full_path: Some("Здание А / 2 этаж / 214".to_string()),
            path_variant_override: None,
            created_at_utc: 1_700_000_000,
            updated_at_utc: 1_700_001_000,
            deleted_at_utc: None,
            version: 1,
        }
    }

    #[test]
    fn from_place_row_converts_kind_via_as_str() {
        let dto = PlaceDto::from(sample_row());
        assert_eq!(dto.kind, "room");
        assert_eq!(dto.id, 42);
        assert_eq!(dto.full_path.as_deref(), Some("Здание А / 2 этаж / 214"));
    }

    #[test]
    fn serde_round_trip_place_dto() {
        let dto = PlaceDto::from(sample_row());
        let json = serde_json::to_string(&dto).expect("serialize");
        let back: PlaceDto = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, dto);
    }

    #[test]
    fn snake_case_json_invariant() {
        let dto = PlaceDto::from(sample_row());
        let json = serde_json::to_string(&dto).expect("serialize");
        assert!(
            json.contains("parent_id"),
            "должен содержать snake_case 'parent_id'"
        );
        assert!(
            json.contains("full_path"),
            "должен содержать snake_case 'full_path'"
        );
        assert!(!json.contains("parentId"), "НЕ должен содержать camelCase");
    }
}
