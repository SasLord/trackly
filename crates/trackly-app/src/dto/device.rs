//! Device DTOs — scaffold only in Plan 01.
//!
//! Full DTO struct definitions (`DeviceDto`, `DeviceNew`, `DevicePatch`, etc.)
//! land in Plan 03 once the CRUD impl is ready.
//!
//! `STATE_HINTS` is defined here now because:
//!   - It's referenced in D-DeviceHints-01 (CONTEXT.md Phase 2)
//!   - It's used to populate the quick-pick UI for the "Состояние" field
//!   - 6 entries per DEV-10 / D-DeviceHints-01

/// Quick-pick hints for the device "Состояние" (condition/state) field.
///
/// These are static UI affordances — not database-driven. A user clicks one of
/// these to fill in the state field, then can still type free-form text.
///
/// Per DEV-10 and D-DeviceHints-01 (Phase 2 CONTEXT.md).
pub const STATE_HINTS: &[&str] = &[
    "Новое",
    "Новый в заводской упаковке, не вскрытый",
    "Новый в заводской упаковке, вскрытый, настроенное рабочее окружение (ОС)",
    "Хорошее",
    "Среднее",
    "Б/У",
];
