//! Domain value types for the Acts entity (acts приёма-передачи).
//!
//! NO serde::Serialize/Deserialize or specta::Type derives here — those live
//! in the DTO layer in trackly-app. Only `#[derive(Debug, Clone, PartialEq, Eq)]`.
//!
//! See D-Counter-Acts-01 (atomic numbering), D-Numbering-01 (display rule
//! «в»/«в1»/«в2» — applied in DTO, not in domain), D-Archive-01 (derived
//! archive flag from remaining "in work" items).

use crate::error::AppError;

/// Type of act: handover (выдача) или return (возврат).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActType {
    /// Acts of handover. `sub_number = NULL`, `parent_act_id = NULL`.
    Handover,
    /// Acts of return. `sub_number = 1, 2, 3, …`, `parent_act_id = Some(...)`.
    Return,
}

impl ActType {
    /// SQL representation used in the `act_type` CHECK constraint
    /// (`'handover'` | `'return'`).
    pub fn to_sql(self) -> &'static str {
        match self {
            ActType::Handover => "handover",
            ActType::Return => "return",
        }
    }

    /// Parse from the SQL representation. Returns `AppError::Validation`
    /// for any string other than `"handover"` / `"return"`.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Result<Self, AppError> {
        match s {
            "handover" => Ok(ActType::Handover),
            "return" => Ok(ActType::Return),
            other => Err(AppError::Validation {
                field: "act_type".to_string(),
                message: format!(
                    "Неподдерживаемый тип акта: «{other}». \
                     Поддерживаемые: handover, return."
                ),
            }),
        }
    }
}

/// Data needed to create a new act.
///
/// `number_override = None` → service инкрементирует `counters.act_number`
/// и использует свежий номер. `number_override = Some(n)` → service проверяет
/// уникальность (включая soft-deleted, D-Soft-vs-Hard-Acts-01) и фиксирует
/// override в audit_log (D-Counter-Acts-01).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActNew {
    pub act_type: ActType,
    pub number_override: Option<i64>,
    /// Только для `ActType::Return`. Должно быть `None` для handover.
    pub parent_act_id: Option<i64>,
    pub giver_name: String,
    pub receiver_name: String,
    pub place_id: Option<i64>,
    pub notes: Option<String>,
    pub deadline_utc: Option<i64>,
    pub items: Vec<ActItemNew>,
}

/// Один пункт акта при создании.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActItemNew {
    pub device_id: i64,
    /// Количество единиц устройства, проходящих через этот пункт.
    /// Persisted in `act_items.quantity` (V014).
    pub quantity: i64,
    pub condition_at_time: Option<String>,
    pub complectation_at_time: Option<String>,
}

/// Domain payload for `ActService::do_return` — описывает возврат по
/// существующему handover-акту. См. D-Acts-Return-01.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActReturnNew {
    /// Bulk condition («Хорошее», «Б/У» …) применяется ко всем checked-row'ам,
    /// у которых нет per-row override (когда `apply_to_all = true`).
    pub bulk_condition: Option<String>,
    /// Bulk place_id применяется аналогично — куда вернуть на склад.
    pub bulk_place_id: Option<i64>,
    /// `true` → bulk-значения заполняют пропуски в items; `false` → каждый item
    /// обязан содержать собственные override-значения.
    pub apply_to_all: bool,
    pub items: Vec<ActReturnItem>,
}

/// Один пункт возврата в `ActReturnNew`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActReturnItem {
    /// id строки `act_items` родительского handover-акта.
    pub act_item_id: i64,
    pub device_id: i64,
    /// Количество единиц к возврату (в Phase 3 всегда 1).
    pub quantity: i64,
    /// Per-row override condition — побеждает bulk-значение.
    pub condition_override: Option<String>,
    /// Per-row override place_id — побеждает bulk-значение.
    pub place_id_override: Option<i64>,
}

/// Partial update for an act. First real consumer: `ActService::update`
/// (Phase 19 Plan 03), which builds this from `ActUpdateDto`
/// (`trackly-app::dto::act`) to edit an existing handover act's header
/// fields under an optimistic-lock (`expected_version`) CAS guard — see
/// `crate::repos::acts_sqlite::update_act_header_in_tx` (trackly-infra).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ActPatch {
    pub giver_name: Option<String>,
    pub receiver_name: Option<String>,
    pub place_id: Option<Option<i64>>,
    pub notes: Option<Option<String>>,
    pub deadline_utc: Option<Option<i64>>,
    /// Phase 19 (D-01/D-04): explicit override of the act's handover date
    /// («Когда отдали»). `None` = no change requested.
    pub handover_date_utc: Option<i64>,
    /// Phase 19 (D-04): explicit № override. `None` = no rename requested.
    pub number: Option<i64>,
    /// CAS token — always required (mirrors `soft_delete_in_tx`'s `version`
    /// argument), never `Option`. Compared against `acts.version` in
    /// `update_act_header_in_tx`'s `WHERE` clause.
    pub expected_version: i64,
}

/// Full act row as returned from the repository read path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActRow {
    pub id: i64,
    pub number: i64,
    pub sub_number: Option<i64>,
    pub parent_act_id: Option<i64>,
    pub act_type: ActType,
    pub giver_name: String,
    pub receiver_name: String,
    pub place_id: Option<i64>,
    /// Resolved current place path (live-resolved via `place_full_paths`).
    pub full_path: Option<String>,
    /// Frozen place path snapshot taken at write time (D-16 print fidelity),
    /// distinct from `full_path` which reflects the CURRENT tree state.
    pub place_path_snapshot: Option<String>,
    pub notes: Option<String>,
    pub deadline_utc: Option<i64>,
    pub archived: bool,
    pub created_at_utc: i64,
    pub updated_at_utc: i64,
    pub deleted_at_utc: Option<i64>,
    pub version: i64,
    /// Phase 3.1 Plan 04 (G-2): фактическая дата handover (когда отдали),
    /// independent of `created_at_utc`. Column добавлен V015 миграцией.
    /// На handover-актах — explicit value; на return-актах = parent.handover_date_utc.
    pub handover_date_utc: i64,
    /// Parent act's `number` joined via LEFT JOIN acts p ON p.id = a.parent_act_id.
    /// `None` for handover. Used by display-rule «в»/«в1»/«в2».
    pub parent_number: Option<i64>,
    /// Count of sibling return acts (same parent_act_id, not deleted).
    /// Used by display-rule to decide whether to suppress `sub_number`
    /// suffix («42в» vs «42в1»).
    pub sibling_return_count: Option<i64>,
}

/// Single act item row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActItemRow {
    pub id: i64,
    pub act_id: i64,
    pub device_id: i64,
    pub quantity: i64,
    pub condition_at_time: Option<String>,
    pub complectation_at_time: Option<String>,
}

/// Filter parameters for act list queries (switch-bar Акты / Возвраты / Архив).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ActFilter {
    pub act_type: Option<ActType>,
    pub archived: Option<bool>,
    pub search: Option<String>,
    pub include_deleted: bool,
}

/// Counts for the switch-bar tabs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ActCounts {
    pub handover_active: i64,
    pub returns: i64,
    pub archived: i64,
}

/// Pagination parameters for list queries.
///
/// Distinct from `crate::domain::devices::Pagination` to avoid coupling acts
/// to devices' module structure — same shape but independent type.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn act_type_roundtrip_sql() {
        assert_eq!(ActType::Handover.to_sql(), "handover");
        assert_eq!(ActType::Return.to_sql(), "return");
        assert_eq!(
            ActType::from_str("handover").expect("ok"),
            ActType::Handover
        );
        assert_eq!(ActType::from_str("return").expect("ok"), ActType::Return);
    }

    #[test]
    fn act_type_from_str_rejects_unknown() {
        let err = ActType::from_str("xyz").expect_err("unknown");
        match err {
            AppError::Validation { field, message } => {
                assert_eq!(field, "act_type");
                assert!(message.contains("xyz"));
            }
            other => panic!("expected Validation, got {other:?}"),
        }
    }
}
