//! Number template DTOs — shared contracts for `NumberTemplateService`
//! (Phase 40.2, Plan 04) between Tauri command handlers and axum HTTP
//! handlers.
//!
//! ## `*SaveOutcome` convention (mandatory for Wave 5 — Plans 06/07/08)
//!
//! Pitfall 7 (`40.2-RESEARCH.md`): a save that needs the user to confirm a
//! non-blocking warning (script-mix / doppelganger) is NOT an error — it is
//! a *different kind of success*. Reusing `AppError::Conflict` for this would
//! force the client to special-case one specific error shape instead of a
//! typed alternative outcome; a bare boolean loses the warning payload.
//!
//! The fix is **not** a single generic `NumberCheckOutcome<T>` enum — this
//! project's `specta::Type` derive does not support generic parameters (see
//! `trackly_core::error::AppError`'s own manual, non-generic `specta::Type`
//! impl for the same reason). Instead, **every Wave 5 consumer defines its
//! own concrete, non-generic enum** next to its own entity DTO, following
//! this exact shape:
//!
//! ```ignore
//! #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
//! #[serde(tag = "outcome", rename_all = "snake_case")]
//! pub enum XxxSaveOutcome {
//!     Created(XxxDto),
//!     NeedsConfirmation(NumberWarningDto),
//! }
//! ```
//!
//! Concretely: `DeviceSaveOutcome` (`dto/device.rs`, Plan 08),
//! `ActSaveOutcome` (`dto/act.rs`, Plan 06), `CartridgeSaveOutcome`
//! (`dto/cartridge.rs`, Plan 07). None of the three are created by this
//! plan — each lives beside the entity DTO it wraps, defined by the plan
//! that owns that entity. This module only fixes the convention so the
//! three plans do not each invent a different response shape for the same
//! D-01 confirmation chain (occupied → template mismatch → script-mix).

use serde::{Deserialize, Serialize};
use specta::Type;

use trackly_core::domain::number_templates::{TemplateContext, TemplateType};

// ---------------------------------------------------------------------------
// TemplateTypeDto
// ---------------------------------------------------------------------------

/// DTO mirror of `trackly_core::domain::number_templates::TemplateType`.
///
/// String values match `TemplateType::as_str()` byte-for-byte (and therefore
/// the `number_templates.type` SQL `CHECK` constraint from V041) — this is
/// not an independent naming choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum TemplateTypeDto {
    DeviceInventory,
    ActNumber,
    CartridgeCode,
    DrumCode,
}

impl From<TemplateType> for TemplateTypeDto {
    fn from(t: TemplateType) -> Self {
        match t {
            TemplateType::DeviceInventory => Self::DeviceInventory,
            TemplateType::ActNumber => Self::ActNumber,
            TemplateType::CartridgeCode => Self::CartridgeCode,
            TemplateType::DrumCode => Self::DrumCode,
        }
    }
}

impl From<TemplateTypeDto> for TemplateType {
    fn from(t: TemplateTypeDto) -> Self {
        match t {
            TemplateTypeDto::DeviceInventory => Self::DeviceInventory,
            TemplateTypeDto::ActNumber => Self::ActNumber,
            TemplateTypeDto::CartridgeCode => Self::CartridgeCode,
            TemplateTypeDto::DrumCode => Self::DrumCode,
        }
    }
}

// ---------------------------------------------------------------------------
// TemplateContextDto
// ---------------------------------------------------------------------------

/// DTO mirror of `trackly_core::domain::number_templates::TemplateContext`.
///
/// String values match `TemplateContext::as_str()` byte-for-byte (and the
/// `number_template_contexts.context` SQL `CHECK` constraint from V041).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum TemplateContextDto {
    DeviceCreate,
    PrinterCreate,
    ActCreate,
    CartridgeCreate,
    DrumCreate,
}

impl From<TemplateContext> for TemplateContextDto {
    fn from(c: TemplateContext) -> Self {
        match c {
            TemplateContext::DeviceCreate => Self::DeviceCreate,
            TemplateContext::PrinterCreate => Self::PrinterCreate,
            TemplateContext::ActCreate => Self::ActCreate,
            TemplateContext::CartridgeCreate => Self::CartridgeCreate,
            TemplateContext::DrumCreate => Self::DrumCreate,
        }
    }
}

impl From<TemplateContextDto> for TemplateContext {
    fn from(c: TemplateContextDto) -> Self {
        match c {
            TemplateContextDto::DeviceCreate => Self::DeviceCreate,
            TemplateContextDto::PrinterCreate => Self::PrinterCreate,
            TemplateContextDto::ActCreate => Self::ActCreate,
            TemplateContextDto::CartridgeCreate => Self::CartridgeCreate,
            TemplateContextDto::DrumCreate => Self::DrumCreate,
        }
    }
}

// ---------------------------------------------------------------------------
// NumberTemplateDto
// ---------------------------------------------------------------------------

/// A single numbering template, as listed/edited in Settings → Организация.
///
/// `next_first_free`/`next_max_plus_one` are already rendered through
/// `trackly_core::text::mask::render_number` — the client never renders a
/// number from raw digits itself (RESEARCH.md Anti-Pattern: "клиент не
/// рендерит номер").
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct NumberTemplateDto {
    #[specta(type = i32)]
    pub id: i64,
    pub template_type: TemplateTypeDto,
    pub mask: String,
    pub next_first_free: String,
    pub next_max_plus_one: String,
    pub has_gap: bool,
    pub overflowed: bool,
    #[specta(type = i32)]
    pub version: i64,
}

// ---------------------------------------------------------------------------
// NextNumberDto
// ---------------------------------------------------------------------------

/// Response of `peek_next`/`preview_mask` — what to show as "следующий
/// номер" (and its gap-toggle alternative, NUM-07) before the user commits
/// to using this template.
///
/// `alt_rendered` is `Some(...)` only when `has_gap && !overflowed &&
/// max_plus_one_fits_width` — the caller uses its presence alone to decide
/// whether to show the ↑/↓ gap toggle at all, it never needs the raw
/// `NextNumberResult` booleans.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct NextNumberDto {
    pub rendered: String,
    pub alt_rendered: Option<String>,
    pub has_gap: bool,
    pub overflowed: bool,
}

// ---------------------------------------------------------------------------
// OccupyingRecordDto
// ---------------------------------------------------------------------------

/// The record currently holding a number — backs both the blocking "Номер
/// занят" popup (D-01) and the doppelganger paragraph of the script-mix
/// warning (NUM-12).
///
/// `kind` is a plain technical discriminant — `"device" | "printer" |
/// "cartridge" | "drum" | "act"` — NOT pre-translated Russian text. The
/// client owns its own kind→Russian label map (see
/// `NumberScriptWarningPopup.svelte`'s `KIND_LABEL_LOWER`, Plan 11); the
/// server never emits UI copy through this field.
///
/// Deliberately a FLAT struct (planner decision, not an oversight) instead
/// of an enum with one variant per record kind — mirrors the
/// `CsvImportReport`/`RowError` precedent in `dto/device.rs` ("plain struct
/// without a nested AppError for a simple specta::Type/JSON shape"). This
/// sacrifices some per-kind richness:
/// - Device/printer: `title` = `devices.name` ("Наименование"), `subtitle` =
///   `devices.model` ("Модель"), `status` = `device_statuses.name`
///   ("Статус").
/// - Cartridge/drum: `title` = `"{brand} {model}"` ("Модель"), `subtitle` =
///   `None`, `status` = `cartridge_states.name` ("Состояние" — NOT
///   `cartridge_statuses`, per the UI-SPEC card for this kind).
/// - Act: `title` = `"Акт передачи"` / `"Акт возврата"` (Copywriting
///   Contract's "Вид" for this card), `subtitle` = `"Передал: {X} · Принял:
///   {Y}"` (this is the doc-commented, intentional double-duty of
///   `subtitle` the plan calls for). The card's "Дата" column has no slot in
///   this flat shape and is intentionally dropped — acts are identified
///   unambiguously by number + type for this warning card without it.
///
/// `number` (BE-WR-09) is the record's OWN number as stored/displayed
/// (`inventory_number`, `cartridges.code`, or an act's DISPLAYED number —
/// `42в1` for a return), trimmed. For the doppelganger paragraph it differs
/// from the user's candidate by look-alike letters, so the client must show
/// THIS value, not echo the input.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct OccupyingRecordDto {
    pub kind: String,
    pub number: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub place: Option<String>,
    pub status: Option<String>,
}

// ---------------------------------------------------------------------------
// NumberWarningKind / NumberWarningDto
// ---------------------------------------------------------------------------

/// The two non-blocking warning kinds in the D-01 save chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum NumberWarningKind {
    /// Number mixes Cyrillic and Latin look-alike letters (NUM-12).
    ScriptMix,
    /// Number does not match the active template's mask (NUM-11).
    Mismatch,
}

/// A non-blocking warning returned instead of (or alongside) a save result.
///
/// `message` is the FULL, final Russian text — dословно matching the
/// Copywriting Contract (`40.2-UI-SPEC.md` "Попапы цепочки сохранения"). The
/// client renders `message` verbatim; it never assembles warning copy
/// itself from raw fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct NumberWarningDto {
    pub kind: NumberWarningKind,
    pub message: String,
    pub doppelganger: Option<OccupyingRecordDto>,
}

// ---------------------------------------------------------------------------
// NumberFieldInput
// ---------------------------------------------------------------------------

/// Shared input sub-contract for the "number/code" field of every Wave 5
/// create/update DTO (`DeviceCreateDto`/`ActCreateDto`/`CartridgeCreateDto`
/// and their `*PatchDto` siblings embed this as `number_input:
/// NumberFieldInput`, replacing the current ad hoc `number_override`/
/// `code_override: Option<String>` fields).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type, Default)]
#[serde(rename_all = "camelCase")]
pub struct NumberFieldInput {
    pub value: String,
    #[specta(type = Option<i32>)]
    pub template_id: Option<i64>,
    pub confirm_mismatch: bool,
    pub confirm_script_mix: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_type_dto_round_trip_matches_core_as_str() {
        for (core, dto) in [
            (
                TemplateType::DeviceInventory,
                TemplateTypeDto::DeviceInventory,
            ),
            (TemplateType::ActNumber, TemplateTypeDto::ActNumber),
            (TemplateType::CartridgeCode, TemplateTypeDto::CartridgeCode),
            (TemplateType::DrumCode, TemplateTypeDto::DrumCode),
        ] {
            assert_eq!(TemplateTypeDto::from(core.clone()), dto);
            assert_eq!(TemplateType::from(dto), core);
            let json = serde_json::to_string(&dto).expect("serialize");
            let back: TemplateTypeDto = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(back, dto);
        }
        assert_eq!(
            serde_json::to_string(&TemplateTypeDto::DeviceInventory).unwrap(),
            "\"device_inventory\""
        );
        assert_eq!(
            serde_json::to_string(&TemplateTypeDto::ActNumber).unwrap(),
            "\"act_number\""
        );
        assert_eq!(
            serde_json::to_string(&TemplateTypeDto::CartridgeCode).unwrap(),
            "\"cartridge_code\""
        );
        assert_eq!(
            serde_json::to_string(&TemplateTypeDto::DrumCode).unwrap(),
            "\"drum_code\""
        );
    }

    #[test]
    fn template_context_dto_round_trip_matches_core_as_str() {
        for (core, dto) in [
            (
                TemplateContext::DeviceCreate,
                TemplateContextDto::DeviceCreate,
            ),
            (
                TemplateContext::PrinterCreate,
                TemplateContextDto::PrinterCreate,
            ),
            (TemplateContext::ActCreate, TemplateContextDto::ActCreate),
            (
                TemplateContext::CartridgeCreate,
                TemplateContextDto::CartridgeCreate,
            ),
            (TemplateContext::DrumCreate, TemplateContextDto::DrumCreate),
        ] {
            assert_eq!(TemplateContextDto::from(core.clone()), dto);
            assert_eq!(TemplateContext::from(dto), core);
            let json = serde_json::to_string(&dto).expect("serialize");
            let back: TemplateContextDto = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(back, dto);
        }
        assert_eq!(
            serde_json::to_string(&TemplateContextDto::DeviceCreate).unwrap(),
            "\"device_create\""
        );
        assert_eq!(
            serde_json::to_string(&TemplateContextDto::DrumCreate).unwrap(),
            "\"drum_create\""
        );
    }

    #[test]
    fn serde_round_trip_number_template_dto() {
        let dto = NumberTemplateDto {
            id: 1,
            template_type: TemplateTypeDto::ActNumber,
            mask: "[YYYY]/[MM]-[X]".to_string(),
            next_first_free: "2026/09-17".to_string(),
            next_max_plus_one: "2026/09-21".to_string(),
            has_gap: true,
            overflowed: false,
            version: 3,
        };
        let json = serde_json::to_string(&dto).expect("serialize");
        let back: NumberTemplateDto = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, dto);
    }

    #[test]
    fn serde_round_trip_next_number_dto() {
        let dto = NextNumberDto {
            rendered: "C-0017".to_string(),
            alt_rendered: Some("C-0021".to_string()),
            has_gap: true,
            overflowed: false,
        };
        let json = serde_json::to_string(&dto).expect("serialize");
        let back: NextNumberDto = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, dto);
    }

    #[test]
    fn serde_round_trip_occupying_record_dto() {
        let dto = OccupyingRecordDto {
            kind: "printer".to_string(),
            number: "ИНВ-000012".to_string(),
            title: "Принтер бухгалтерии".to_string(),
            subtitle: Some("HP LaserJet Pro".to_string()),
            place: Some("Здание А / Кабинет 12".to_string()),
            status: Some("В работе".to_string()),
        };
        let json = serde_json::to_string(&dto).expect("serialize");
        let back: OccupyingRecordDto = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, dto);
    }

    #[test]
    fn serde_round_trip_number_warning_dto_with_doppelganger() {
        let dto = NumberWarningDto {
            kind: NumberWarningKind::ScriptMix,
            message: "В номере «OPГ-00-000001» смешаны русские и латинские буквы.".to_string(),
            doppelganger: Some(OccupyingRecordDto {
                kind: "device".to_string(),
                number: "ОРГ-00-000001".to_string(),
                title: "Ноутбук Иванова И.И.".to_string(),
                subtitle: None,
                place: None,
                status: None,
            }),
        };
        let json = serde_json::to_string(&dto).expect("serialize");
        let back: NumberWarningDto = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, dto);
    }

    #[test]
    fn serde_round_trip_number_warning_dto_without_doppelganger() {
        let dto = NumberWarningDto {
            kind: NumberWarningKind::Mismatch,
            message: "Номер «7» не подходит под шаблон «C-[XXXX]».".to_string(),
            doppelganger: None,
        };
        let json = serde_json::to_string(&dto).expect("serialize");
        let back: NumberWarningDto = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, dto);
    }

    #[test]
    fn serde_round_trip_number_field_input() {
        let dto = NumberFieldInput {
            value: "C-000017".to_string(),
            template_id: Some(2),
            confirm_mismatch: false,
            confirm_script_mix: true,
        };
        let json = serde_json::to_string(&dto).expect("serialize");
        let back: NumberFieldInput = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, dto);
    }
}
