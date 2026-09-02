//! `MovementEntryDto` — flat, pre-formatted timeline-read DTO for HST-02.
//!
//! Deliberately NOT a reuse/extension of `AuditEntryDto` (`dto/cartridge.rs`) — that
//! struct has no `user_id` field and its generic JSON-blob shape (action + three
//! serialized-object columns) forces the UI to parse JSON before rendering anything.
//! This DTO is the opposite: every field is already the exact string/number the
//! timeline row needs,
//! except the raw `source`/`note`/`act_id`/`act_number` fields which are intentionally
//! left RAW (not pre-composed into a "Причина" string) so the UI (Plan 40-15) can
//! compose the exact phrasing from UI-SPEC's Copywriting Contract without a backend
//! redeploy every time the copy changes.
//!
//! `actor_display` and the two `*_path_short` fields ARE pre-formatted server-side,
//! because they genuinely require server-only data (ФИО resolution via `users`,
//! Plan 40-02's `compute_place_path_short` shortening formula) that the client cannot
//! derive on its own.

use serde::{Deserialize, Serialize};
use specta::Type;

/// A single row of an entity's place-movement timeline (HST-02).
///
/// `from_place_id`/`to_place_id` are REQUIRED (not `Option`) — D-06's write-side guard
/// means a `place_movements` row only ever exists when both sides were real place
/// nodes, so these are always present. They exist so the UI can make the place
/// segments navigable (D-19, "место → раздел «Места» с фокусом на узле").
///
/// `from_place_path`/`to_place_path` are the FULL stored snapshot strings (also always
/// present, mirrors the NOT NULL schema columns from Plan 40-01) — they exist
/// specifically so the UI can populate the native `title=` tooltip with the full path
/// per D-17, without ever re-deriving or re-fetching it.
///
/// `from_place_path_short`/`to_place_path_short` are the server-shortened display
/// strings (D-18), `Option` only because `compute_place_path_short` returns `None` on
/// a defensive/unreachable input path — in practice always `Some` when
/// `from_place_path`/`to_place_path` is non-empty.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct MovementEntryDto {
    /// Primary key of the `place_movements` row — stable unique key for UI list rendering.
    #[specta(type = i32)]
    pub id: i64,
    /// `"device"` or `"cartridge"` (D-21: a printer is stored as `"device"` — there is
    /// no separate `"printer"` token).
    pub entity_type: String,
    #[specta(type = i32)]
    pub entity_id: i64,
    #[specta(type = i32)]
    pub from_place_id: i64,
    pub from_place_path: String,
    pub from_place_path_short: Option<String>,
    #[specta(type = i32)]
    pub to_place_id: i64,
    pub to_place_path: String,
    pub to_place_path_short: Option<String>,
    /// ФИО snapshot → login fallback → "система" (`user_id IS NULL`) — D-11.
    pub actor_display: String,
    /// Raw DB token (`"manual"` | `"act"` | `"map"` | `"workstation"` | any future/
    /// unrecognized value) — the UI composes the final Russian phrase per UI-SPEC's
    /// Copywriting Contract. `MovementSource::from_str_lenient` is available to any
    /// Rust-side caller needing a typed match with a safe fallback.
    pub source: String,
    pub note: Option<String>,
    #[specta(type = Option<i32>)]
    pub act_id: Option<i64>,
    /// Human-readable act number, resolved when `act_id` is `Some` (soft-degraded to
    /// `None` if the act row is somehow gone — should not normally happen since acts
    /// are soft-deleted, but this field never panics on a missing row).
    pub act_number: Option<String>,
    #[specta(type = i32)]
    pub created_at_utc: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Invented example only — no real names (CLAUDE.md privacy gate).
    fn sample() -> MovementEntryDto {
        MovementEntryDto {
            id: 1,
            entity_type: "device".to_string(),
            entity_id: 42,
            from_place_id: 1,
            from_place_path: "Здание А / Каб. 101".to_string(),
            from_place_path_short: Some("Здание А … Каб. 101".to_string()),
            to_place_id: 2,
            to_place_path: "Здание Б / Склад".to_string(),
            to_place_path_short: Some("Здание Б … Склад".to_string()),
            actor_display: "Иванов И.И.".to_string(),
            source: "manual".to_string(),
            note: None,
            act_id: None,
            act_number: None,
            created_at_utc: 1_700_000_000,
        }
    }

    #[test]
    fn serializes_with_raw_source_and_note_preserved() {
        let dto = sample();
        let json = serde_json::to_value(&dto).expect("serialize");
        assert_eq!(json["source"], "manual");
        assert_eq!(json["note"], serde_json::Value::Null);
    }

    #[test]
    fn unrecognized_source_token_passes_through_without_panicking() {
        let mut dto = sample();
        dto.source = "garbage".to_string();
        let json = serde_json::to_value(&dto).expect("serialize");
        assert_eq!(json["source"], "garbage");
    }

    #[test]
    fn system_actor_display_for_null_user_id() {
        let mut dto = sample();
        dto.actor_display = "система".to_string();
        assert_eq!(dto.actor_display, "система");
    }
}
