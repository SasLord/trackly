//! `NumberTemplateRepository` port — repository trait for numbering
//! templates (Phase 40.2).
//!
//! Pattern: associated `type Conn` keeps rusqlite out of trackly-core.
//! The concrete type (`rusqlite::Connection`) is specified in the adapter
//! impl in `trackly_infra::repos::number_templates_sqlite` (Plan 03).
//!
//! This trait fixes only the READING half of the port surface. Write
//! methods (create/update mask/delete/set context) participate in larger
//! transactions and depend on details of the service-layer transaction
//! that Plan 03 decides — they are deliberately left out of this trait so
//! Plan 03 is free to shape them as `*_in_tx` helpers if that simplifies
//! orchestration, mirroring the precedent in `ports::acts`/`ports::cartridges`.

use crate::domain::number_templates::{NumberTemplateRow, TemplateContext, TemplateType};
use crate::error::AppError;

/// Repository port for numbering templates. Implemented by
/// `SqliteNumberTemplateRepository` in trackly-infra.
pub trait NumberTemplateRepository {
    /// The connection type provided by the adapter (e.g. `rusqlite::Connection`).
    type Conn;

    /// Fetch a single template by ID. Returns `AppError::NotFound` if
    /// absent — templates are physically deleted (D-12), there is no
    /// soft-delete state to distinguish from "never existed".
    fn get(&self, conn: &Self::Conn, id: i64) -> Result<NumberTemplateRow, AppError>;

    /// List templates, optionally filtered by `template_type`. No
    /// pagination — the number of templates is always small.
    fn list(
        &self,
        conn: &Self::Conn,
        template_type: Option<TemplateType>,
    ) -> Result<Vec<NumberTemplateRow>, AppError>;

    /// The template assigned to `context`, if any (`number_template_contexts.template_id`).
    fn get_context_template_id(
        &self,
        conn: &Self::Conn,
        context: TemplateContext,
    ) -> Result<Option<i64>, AppError>;

    /// Raw text values of live records in the space that `template_type`
    /// covers, used to compute the next free number for a mask.
    ///
    /// - `DeviceInventory` → `devices.inventory_number` (live rows only).
    /// - `ActNumber` → `acts.number` restricted to `act_type = 'handover'`
    ///   ONLY — returns (возвраты) do not participate in the act-number
    ///   sequence (SPEC NUM-14).
    /// - `CartridgeCode` and `DrumCode` → BOTH read `cartridges.code`
    ///   without any `kind_id` filter. The two template types exist for
    ///   CRUD/UI grouping only; they do not correspond to a physical split
    ///   of the underlying data. Two templates with colliding masks
    ///   assigned to "Картриджи" and "Фотобарабаны" will therefore
    ///   legitimately observe the same sequence, since both read the same
    ///   `cartridges.code` column — this is expected behavior, not a bug.
    ///
    /// `today_utc` is passed through for masks with a date component
    /// (implemented by Plan 03+; unused by masks without one).
    fn fetch_matching_values(
        &self,
        conn: &Self::Conn,
        template_type: TemplateType,
        today_utc: i64,
    ) -> Result<Vec<String>, AppError>;
}
