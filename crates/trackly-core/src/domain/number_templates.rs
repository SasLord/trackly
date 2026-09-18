//! Domain value types for numbering templates (Phase 40.2, D-15/D-16/D-17).
//!
//! NO serde::Serialize/Deserialize or specta::Type derives here — those live
//! in the DTO layer in trackly-app (`trackly_app::dto::number_template`,
//! Plan 04). Only `#[derive(Debug, Clone, PartialEq, Eq)]`.
//!
//! These are the contracts every later plan in Phase 40.2 references — the
//! string values returned by `as_str()` MUST match the SQL `CHECK (type IN
//! (...))` / `CHECK (context IN (...))` constraints introduced by
//! `migrations/V041__number_templates.sql`.

use crate::error::AppError;

// ---------------------------------------------------------------------------
// TemplateType
// ---------------------------------------------------------------------------

/// The four spaces a numbering template can generate values for.
///
/// `as_str()` values are the literal `type` column values in
/// `number_templates` — changing them requires a matching migration, they
/// cannot be renamed independently of the SQL `CHECK` constraint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemplateType {
    /// Инвентарные номера устройств/принтеров.
    DeviceInventory,
    /// Номера актов приёма-передачи.
    ActNumber,
    /// Коды картриджей.
    CartridgeCode,
    /// Коды фотобарабанов.
    DrumCode,
}

impl TemplateType {
    /// SQL representation used in the `type` CHECK constraint
    /// (`'device_inventory'` | `'act_number'` | `'cartridge_code'` | `'drum_code'`).
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::DeviceInventory => "device_inventory",
            Self::ActNumber => "act_number",
            Self::CartridgeCode => "cartridge_code",
            Self::DrumCode => "drum_code",
        }
    }

    /// Parse from the SQL representation. Returns `AppError::Validation`
    /// for any string other than the four values above.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Result<Self, AppError> {
        match s {
            "device_inventory" => Ok(Self::DeviceInventory),
            "act_number" => Ok(Self::ActNumber),
            "cartridge_code" => Ok(Self::CartridgeCode),
            "drum_code" => Ok(Self::DrumCode),
            other => Err(AppError::Validation {
                field: "template_type".to_string(),
                message: format!("unknown template type: {other}"),
            }),
        }
    }

    /// Русское человекочитаемое имя типа шаблона (Copywriting Contract
    /// UI-SPEC §Copywriting) — единственный источник истины, фронтенд и
    /// Rust-код не должны хардкодить эти строки порознь.
    pub fn display_name_ru(&self) -> &'static str {
        match self {
            Self::DeviceInventory => "Устройства и принтеры",
            Self::ActNumber => "Акты",
            Self::CartridgeCode => "Картриджи",
            Self::DrumCode => "Фотобарабаны",
        }
    }
}

// ---------------------------------------------------------------------------
// TemplateContext
// ---------------------------------------------------------------------------

/// The five UI contexts ("Вставка" popups) that can have a default template
/// assigned via `number_template_contexts`.
///
/// `as_str()` values are the literal `context` column values in
/// `number_template_contexts` — must match the SQL `CHECK` constraint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemplateContext {
    /// Форма создания устройства.
    DeviceCreate,
    /// Форма создания принтера.
    PrinterCreate,
    /// Форма создания акта.
    ActCreate,
    /// Форма создания картриджа.
    CartridgeCreate,
    /// Форма создания фотобарабана.
    DrumCreate,
}

impl TemplateContext {
    /// SQL representation used in the `context` CHECK constraint /
    /// `PRIMARY KEY`.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::DeviceCreate => "device_create",
            Self::PrinterCreate => "printer_create",
            Self::ActCreate => "act_create",
            Self::CartridgeCreate => "cartridge_create",
            Self::DrumCreate => "drum_create",
        }
    }

    /// Parse from the SQL representation. Returns `AppError::Validation`
    /// for any string other than the five values above.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Result<Self, AppError> {
        match s {
            "device_create" => Ok(Self::DeviceCreate),
            "printer_create" => Ok(Self::PrinterCreate),
            "act_create" => Ok(Self::ActCreate),
            "cartridge_create" => Ok(Self::CartridgeCreate),
            "drum_create" => Ok(Self::DrumCreate),
            other => Err(AppError::Validation {
                field: "template_context".to_string(),
                message: format!("unknown template context: {other}"),
            }),
        }
    }

    /// The `TemplateType` that a template assigned to this context must
    /// have — единственный источник истины «какой тип шаблонов показывать
    /// в меню „Вставка“ для данного попапа», без него последующие планы
    /// будут дублировать эту матрицу.
    pub fn template_type(&self) -> TemplateType {
        match self {
            Self::DeviceCreate | Self::PrinterCreate => TemplateType::DeviceInventory,
            Self::ActCreate => TemplateType::ActNumber,
            Self::CartridgeCreate => TemplateType::CartridgeCode,
            Self::DrumCreate => TemplateType::DrumCode,
        }
    }
}

// ---------------------------------------------------------------------------
// NumberTemplateRow
// ---------------------------------------------------------------------------

/// A single row of the `number_templates` table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NumberTemplateRow {
    pub id: i64,
    pub template_type: TemplateType,
    pub mask: String,
    pub created_at_utc: i64,
    pub updated_at_utc: i64,
    pub version: i64,
}

// ---------------------------------------------------------------------------
// NextNumberResult
// ---------------------------------------------------------------------------

/// Result of computing the next free number for a template's mask.
///
/// `has_gap = first_free != max_plus_one` — `true` means there is a hole in
/// the existing sequence (e.g. #3 was deleted) and `first_free` reuses it
/// instead of extending the sequence.
///
/// `overflowed` is `true` only when the mask has a fixed digit width
/// (`[X…N]`) and every value in that width is already taken — there is no
/// free number left, digit width does not auto-expand. For an unbounded
/// mask (`[X]`, no width specifier) `overflowed` is always `false`, since
/// the numeric space is not limited. This rule underlies NUM-04/NUM-05 in
/// every subsequent plan of this phase.
///
/// `max_plus_one_fits_width` is `false` when `max_plus_one` itself does not
/// fit the fixed digit width even though a free number still exists lower
/// in the range (NUM-05: "разрыв есть, но max+1 не помещается") — the
/// consumer uses this to decide whether to show the ↑/↓ gap toggle at all,
/// separately from `overflowed` (no free numbers left at all). Always
/// `true` for an unbounded mask.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NextNumberResult {
    pub first_free: u64,
    pub max_plus_one: u64,
    pub has_gap: bool,
    pub max_plus_one_fits_width: bool,
    pub overflowed: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_type_round_trip() {
        for t in [
            TemplateType::DeviceInventory,
            TemplateType::ActNumber,
            TemplateType::CartridgeCode,
            TemplateType::DrumCode,
        ] {
            let s = t.as_str();
            assert_eq!(TemplateType::from_str(s).expect("round-trip"), t);
        }
    }

    #[test]
    fn template_type_from_str_rejects_unknown() {
        let err = TemplateType::from_str("bogus").expect_err("must reject unknown");
        match err {
            AppError::Validation { field, .. } => assert_eq!(field, "template_type"),
            other => panic!("expected Validation, got {other:?}"),
        }
    }

    #[test]
    fn template_type_display_name_ru_all_variants() {
        assert_eq!(
            TemplateType::DeviceInventory.display_name_ru(),
            "Устройства и принтеры"
        );
        assert_eq!(TemplateType::ActNumber.display_name_ru(), "Акты");
        assert_eq!(TemplateType::CartridgeCode.display_name_ru(), "Картриджи");
        assert_eq!(TemplateType::DrumCode.display_name_ru(), "Фотобарабаны");
    }

    #[test]
    fn template_context_round_trip() {
        for c in [
            TemplateContext::DeviceCreate,
            TemplateContext::PrinterCreate,
            TemplateContext::ActCreate,
            TemplateContext::CartridgeCreate,
            TemplateContext::DrumCreate,
        ] {
            let s = c.as_str();
            assert_eq!(TemplateContext::from_str(s).expect("round-trip"), c);
        }
    }

    #[test]
    fn template_context_from_str_rejects_unknown() {
        let err = TemplateContext::from_str("bogus").expect_err("must reject unknown");
        match err {
            AppError::Validation { field, .. } => assert_eq!(field, "template_context"),
            other => panic!("expected Validation, got {other:?}"),
        }
    }

    #[test]
    fn template_context_maps_to_expected_template_type() {
        assert_eq!(
            TemplateContext::DeviceCreate.template_type(),
            TemplateType::DeviceInventory
        );
        assert_eq!(
            TemplateContext::PrinterCreate.template_type(),
            TemplateType::DeviceInventory
        );
        assert_eq!(
            TemplateContext::ActCreate.template_type(),
            TemplateType::ActNumber
        );
        assert_eq!(
            TemplateContext::CartridgeCreate.template_type(),
            TemplateType::CartridgeCode
        );
        assert_eq!(
            TemplateContext::DrumCreate.template_type(),
            TemplateType::DrumCode
        );
    }

    #[test]
    fn number_template_row_constructible() {
        let row = NumberTemplateRow {
            id: 1,
            template_type: TemplateType::ActNumber,
            mask: "[X]".to_string(),
            created_at_utc: 0,
            updated_at_utc: 0,
            version: 1,
        };
        assert_eq!(row.template_type, TemplateType::ActNumber);
        assert_eq!(row.mask, "[X]");
    }

    #[test]
    fn next_number_result_no_gap_when_first_free_equals_max_plus_one() {
        let r = NextNumberResult {
            first_free: 5,
            max_plus_one: 5,
            has_gap: 5 != 5,
            max_plus_one_fits_width: true,
            overflowed: false,
        };
        assert!(!r.has_gap);
    }

    #[test]
    fn next_number_result_has_gap_when_first_free_differs() {
        let has_gap = 3_u64 != 6_u64;
        let r = NextNumberResult {
            first_free: 3,
            max_plus_one: 6,
            has_gap,
            max_plus_one_fits_width: true,
            overflowed: false,
        };
        assert!(r.has_gap);
    }
}
