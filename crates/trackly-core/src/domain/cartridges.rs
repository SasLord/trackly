//! Domain value types for the Cartridges entity (картриджи и фотобарабаны).
//!
//! NO serde::Serialize/Deserialize or specta::Type derives here — those live
//! in the DTO layer in trackly-app. Only `#[derive(Debug, Clone, PartialEq, Eq)]`.
//!
//! See D-Code-01 (auto-code C-NNNNNN from cartridge_seq counter),
//! D-Op-Transitions-01 (lifecycle transitions by status),
//! D-LowStock-01 (low stock threshold from app_settings).

use crate::error::AppError;

/// Full cartridge row as returned from the repository read path.
/// Joined columns (model_*, status_name, state_name) may be None if
/// the FK target was soft-deleted or the column is nullable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CartridgeRow {
    pub id: i64,
    /// Human-visible code in format C-NNNNNN (from cartridge_seq counter).
    pub code: String,
    pub model_id: i64,
    /// Joined: cartridge_models.brand
    pub model_brand: Option<String>,
    /// Joined: cartridge_models.model
    pub model_name: Option<String>,
    /// Joined: cartridge_models.kind_id (1=Картридж, 2=Фотобарабан)
    pub model_kind_id: Option<i64>,
    /// FK to cartridge_statuses (1=На складе, 2=В работе, 3=На заправке, 4=Списано)
    pub status_id: i64,
    /// Joined: cartridge_statuses.name
    pub status_name: Option<String>,
    /// FK to cartridge_states (1=Полный, 2=Частичный, 3=Пустой); NULL = unknown
    pub state_id: Option<i64>,
    /// Joined: cartridge_states.name
    pub state_name: Option<String>,
    /// Live-resolved place_id (D-12: cartridge has its own place, not derived
    /// from the printer at read time).
    pub place_id: Option<i64>,
    /// Live-resolved display path via `place_full_paths`.
    pub full_path: Option<String>,
    /// Denormalised current holder (кому выдано).
    pub holder_name: Option<String>,
    pub notes: Option<String>,
    pub created_at_utc: i64,
    pub updated_at_utc: i64,
    pub deleted_at_utc: Option<i64>,
    pub version: i64,
}

/// Full cartridge model row as returned from the repository read path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CartridgeModelRow {
    pub id: i64,
    pub brand: String,
    pub model: String,
    /// FK to cartridge_kinds (1=Картридж, 2=Фотобарабан)
    pub kind_id: i64,
    /// Color text (e.g. "Чёрный"); None for Фотобарабан.
    pub color: Option<String>,
    pub notes: Option<String>,
    pub created_at_utc: i64,
    pub updated_at_utc: i64,
    pub deleted_at_utc: Option<i64>,
    pub version: i64,
}

/// Data needed to create a new cartridge instance.
///
/// `code_override = None` → service increments cartridge_seq and formats C-NNNNNN.
/// `code_override = Some(s)` → custom code (barcode from packaging), counter NOT incremented.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CartridgeNew {
    pub model_id: i64,
    /// None → auto-code from cartridge_seq; Some → custom code (conflict checked by repo).
    pub code_override: Option<String>,
    /// Initial charge state (1=Полный, 2=Частичный, 3=Пустой); None = unset.
    pub state_id: Option<i64>,
    pub place_id: Option<i64>,
    pub notes: Option<String>,
}

/// Data needed to create a new cartridge model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CartridgeModelNew {
    pub brand: String,
    pub model: String,
    /// FK to cartridge_kinds (1=Картридж, 2=Фотобарабан).
    pub kind_id: i64,
    /// Color; should be None when kind_id=2 (Фотобарабан).
    pub color: Option<String>,
    pub notes: Option<String>,
}

/// Lifecycle transition payload — one enum covers all ops (D-Op-Modal-01).
///
/// NO serde/specta derives here — those live in CartridgeTransitionPayload in
/// trackly-app/src/dto/cartridge.rs. The DTO layer converts to this domain enum
/// before calling CartridgeService::transition.
///
/// Allowed transitions (D-Op-Transitions-01):
///   Install:       status 1 (На складе) → 2 (В работе)
///   ReturnToStock: status 2 (В работе)  → 1 (На складе)
///   ToRefill:      status 1 (На складе) → 3 (На заправке)
///   FromRefill:    status 3 (На заправке) → 1 (На складе)
///   WriteOff:      status != 4 → 4 (Списано)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CartridgeTransitionOp {
    /// Установить в принтер: На складе → В работе.
    /// state_id NOT changed by this operation.
    Install {
        date_utc: i64,
        given_by_name: String,
        given_to_name: String,
        place_id: Option<i64>,
        /// Принтер, в который устанавливается картридж (device_id, FK на devices).
        /// None допустим для обратной совместимости со старым cartridge-centric
        /// входом (D-08), где принтер не указывается явно — авто-возврат
        /// предыдущего картриджа (D-16) в этом случае не выполняется.
        printer_device_id: Option<i64>,
        /// Override для D-16: заряд предыдущего картриджа при авто-возврате.
        /// None = kind-aware дефолт, применяемый на уровне репозитория
        /// (`cartridges_sqlite.rs`): 3 (Пустой) для картриджей (kind_id=1),
        /// 5 (Изношенный) для барабанов (kind_id=2) — см. R7, Phase 13.
        previous_cartridge_state_id: Option<i64>,
        /// Override для D-16: place_id предыдущего картриджа при
        /// авто-возврате; None = дефолт (нет места).
        previous_cartridge_place_id: Option<i64>,
    },
    /// Вернуть на склад: В работе → На складе.
    /// holder_name cleared; state_id set to payload value (default: 3=Пустой).
    ReturnToStock {
        /// New charge state (default 3=Пустой, editable by user).
        state_id: i64,
        place_id: Option<i64>,
        notes: Option<String>,
    },
    /// Отправить на заправку: На складе → На заправке.
    ToRefill {
        date_utc: i64,
        given_by_name: String,
        given_to_name: String,
        place_id: Option<i64>,
    },
    /// Забрать с заправки: На заправке → На складе.
    /// state_id set to payload value (default: 1=Полный).
    FromRefill {
        /// New charge state (default 1=Полный, editable by user).
        state_id: i64,
        place_id: Option<i64>,
        notes: Option<String>,
    },
    /// Списать: любой статус != 4 → 4 (Списано).
    WriteOff {
        date_utc: i64,
        notes: Option<String>,
    },
}

impl CartridgeTransitionOp {
    /// Validate that the given `current_status_id` allows this transition.
    /// Returns `AppError::Validation` if the transition is not allowed.
    pub fn validate_from_status(&self, current_status_id: i64) -> Result<(), AppError> {
        let (expected, op_name) = match self {
            CartridgeTransitionOp::Install { .. } => (1, "Установить в принтер"),
            CartridgeTransitionOp::ReturnToStock { .. } => (2, "Вернуть на склад"),
            CartridgeTransitionOp::ToRefill { .. } => (1, "Отправить на заправку"),
            CartridgeTransitionOp::FromRefill { .. } => (3, "Забрать с заправки"),
            CartridgeTransitionOp::WriteOff { .. } => {
                if current_status_id == 4 {
                    return Err(AppError::Validation {
                        field: "status_id".to_string(),
                        message: "Картридж уже списан".to_string(),
                    });
                }
                return Ok(());
            }
        };
        if current_status_id != expected {
            return Err(AppError::Validation {
                field: "status_id".to_string(),
                message: format!(
                    "Операция «{}» недопустима для текущего статуса картриджа (id={})",
                    op_name, current_status_id
                ),
            });
        }
        Ok(())
    }

    /// Returns the audit_log action string for this operation (D-History-01).
    pub fn audit_action(&self) -> &'static str {
        match self {
            CartridgeTransitionOp::Install { .. } => "custom:install",
            CartridgeTransitionOp::ReturnToStock { .. } => "custom:return_to_stock",
            CartridgeTransitionOp::ToRefill { .. } => "custom:to_refill",
            CartridgeTransitionOp::FromRefill { .. } => "custom:from_refill",
            CartridgeTransitionOp::WriteOff { .. } => "custom:write_off",
        }
    }

    /// Returns the new status_id after the transition.
    pub fn target_status_id(&self) -> i64 {
        match self {
            CartridgeTransitionOp::Install { .. } => 2, // В работе
            CartridgeTransitionOp::ReturnToStock { .. } => 1, // На складе
            CartridgeTransitionOp::ToRefill { .. } => 3, // На заправке
            CartridgeTransitionOp::FromRefill { .. } => 1, // На складе
            CartridgeTransitionOp::WriteOff { .. } => 4, // Списано
        }
    }
}

/// Filter parameters for cartridge list queries.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CartridgeFilter {
    /// Filter by status (1=На складе, 2=В работе, 3=На заправке, 4=Списано); None = all.
    pub status_id: Option<i64>,
    /// Filter by kind (1=Картридж, 2=Фотобарабан); None = all.
    pub kind_id: Option<i64>,
    /// Filter by model_id; None = all.
    pub model_id: Option<i64>,
    /// Full-text search query (applied by search_acts in repo, not list).
    pub search: Option<String>,
    /// Include soft-deleted rows.
    pub include_deleted: bool,
    /// Только статус «На складе» (1) и kind-aware заряд: для картриджей (kind_id=1) —
    /// Полный(1)/Частичный(2); для фотобарабанов (kind_id=2) — Новый(4)/Изношенный(5)
    /// (Отработанный(6) уже отдельно отбраковывается при установке). Для селектора
    /// установки из заявки (D-01, Phase 12; kind-aware fix — CR-01/WR-01).
    pub installable_only: bool,
    /// Когда задан — ограничивает выборку моделями, чья
    /// `cartridge_model_compatibility.printer_name` совпадает
    /// (регистронезависимо, с TRIM) с `devices.name` связанного принтера;
    /// пустой набор строк совместимости для модели не сужает выборку (D-05).
    /// Phase 13 redesign — per-printer-name compatibility, supersedes the
    /// V029 per-device junction table.
    pub compatible_with_printer_device_id: Option<i64>,
}

/// Counts for the status switch-bar (D-Filters-01).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CartridgeCounts {
    /// All non-deleted cartridges.
    pub all: i64,
    /// status_id = 1 (На складе)
    pub in_stock: i64,
    /// status_id = 2 (В работе)
    pub in_use: i64,
    /// status_id = 3 (На заправке)
    pub at_refill: i64,
    /// status_id = 4 (Списано)
    pub written_off: i64,
}

/// The basis (grouping key) used to compute low-stock warnings — read from
/// `app_settings.low_stock_basis` (quick task 260819-wq5).
///
/// - `CartridgeModel`: legacy behavior — group in-stock+full cartridges by
///   `cartridge_models.id`. Unchanged since D-LowStock-02.
/// - `PrinterModel`: group by printer name sourced strictly from
///   `cartridge_model_compatibility.printer_name` (never `devices.name`) —
///   different cartridge-model brands compatible with the same printer are
///   summed together. This is the DEFAULT for missing/invalid values,
///   intentionally changing behavior on existing databases per the CONTEXT
///   decision ("Хранение настройки").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LowStockBasis {
    CartridgeModel,
    PrinterModel,
}

impl LowStockBasis {
    /// Default basis when `app_settings.low_stock_basis` is missing or holds
    /// an unrecognized value (GET-only fallback; SET rejects unknown values).
    pub const DEFAULT: LowStockBasis = LowStockBasis::PrinterModel;

    pub fn as_str(self) -> &'static str {
        match self {
            LowStockBasis::CartridgeModel => "cartridge_model",
            LowStockBasis::PrinterModel => "printer_model",
        }
    }

    /// Exact match only — the caller decides whether to fall back to
    /// `DEFAULT` (GET) or reject (SET).
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "cartridge_model" => Some(LowStockBasis::CartridgeModel),
            "printer_model" => Some(LowStockBasis::PrinterModel),
            _ => None,
        }
    }
}

/// A model (or printer name) below the low-stock threshold (D-LowStock-02),
/// grouped by `basis` (quick task 260819-wq5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LowStockItem {
    /// Which grouping produced this row — see [`LowStockBasis`].
    pub basis: LowStockBasis,
    /// Some for `CartridgeModel` rows (cartridge_models.id); None for
    /// `PrinterModel` rows (no single model backs a printer-name group).
    pub model_id: Option<i64>,
    /// Some for `CartridgeModel` rows (cartridge_models.brand).
    pub brand: Option<String>,
    /// Some for `CartridgeModel` rows (cartridge_models.model).
    pub model: Option<String>,
    /// Display label: "{brand} {model}" for `CartridgeModel` rows; the
    /// printer's display name (one of the written variants within its
    /// normalized group) for `PrinterModel` rows.
    pub label: String,
    /// Count of in-stock + full cartridges (status=1 AND state=1).
    pub count: i64,
    /// The configured threshold (from app_settings.low_stock_threshold).
    pub threshold: i64,
}

/// Aggregate counts (by status) for a single cartridge model compatible with
/// a given printer device — backs the printer card's "Совместимые модели
/// картриджей" widget (R4, Phase 13).
///
/// Built from `cartridge_model_compatibility.printer_name` matching
/// `devices.name` (case-insensitive, TRIM'd) — see
/// `SqliteCartridgeRepository::compatible_model_aggregates`. Unlike the
/// `compatible_with_printer_device_id` filter in `CartridgeFilter` (which
/// pass-throughs when a model has no compatibility rows at all, D-05), this
/// aggregate does NOT pass through: a model with zero matching compatibility
/// rows for this printer is simply absent from the result (R4/D-07) so the
/// UI can render "Нет совместимых моделей картриджей." when appropriate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatibleModelAggregate {
    pub model_id: i64,
    pub brand: String,
    pub model: String,
    /// Live (non-deleted) cartridges of this model with status_id = 1 (На складе).
    ///
    /// NOTE (WR-03): this is a RAW status count, NOT an "installable" count.
    /// It deliberately mirrors the "На складе" UI label, which reflects the
    /// physical/storage status (status_id = 1) regardless of `state_id`. For
    /// drums (kind_id = 2) a status=1 unit in state=6 (Отработанный) is counted
    /// here even though `transition_in_tx` would reject installing it — the
    /// installable predicate `(kind_id=1 AND state_id IN (1,2)) OR
    /// (kind_id=2 AND state_id IN (4,5))` used by `CartridgeRepository::list`'s
    /// `installable_only` filter is intentionally NOT applied to this count.
    pub in_stock: i64,
    /// Live (non-deleted) cartridges of this model with status_id = 3 (На заправке).
    pub at_refill: i64,
    /// Live (non-deleted) cartridges of this model with status_id = 2 (В работе).
    pub in_use: i64,
}

/// Pagination parameters for list queries.
///
/// Distinct from `crate::domain::acts::Pagination` to avoid coupling cartridges
/// to acts' module structure — same shape but independent type.
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
    fn transition_op_validate_install_from_stock() {
        let op = CartridgeTransitionOp::Install {
            date_utc: 0,
            given_by_name: "A".into(),
            given_to_name: "B".into(),
            place_id: Some(1),
            printer_device_id: None,
            previous_cartridge_state_id: None,
            previous_cartridge_place_id: None,
        };
        assert!(op.validate_from_status(1).is_ok());
        assert!(op.validate_from_status(2).is_err()); // wrong status
        assert!(op.validate_from_status(3).is_err());
    }

    #[test]
    fn transition_op_validate_return_to_stock() {
        let op = CartridgeTransitionOp::ReturnToStock {
            state_id: 3,
            place_id: Some(2),
            notes: None,
        };
        assert!(op.validate_from_status(2).is_ok());
        assert!(op.validate_from_status(1).is_err());
    }

    #[test]
    fn transition_op_validate_write_off() {
        let op = CartridgeTransitionOp::WriteOff {
            date_utc: 0,
            notes: None,
        };
        // Any non-4 status is allowed
        assert!(op.validate_from_status(1).is_ok());
        assert!(op.validate_from_status(2).is_ok());
        assert!(op.validate_from_status(3).is_ok());
        // Already written off — error
        assert!(op.validate_from_status(4).is_err());
    }

    #[test]
    fn transition_op_audit_actions() {
        assert_eq!(
            CartridgeTransitionOp::Install {
                date_utc: 0,
                given_by_name: "A".into(),
                given_to_name: "B".into(),
                place_id: Some(1),
                printer_device_id: None,
                previous_cartridge_state_id: None,
                previous_cartridge_place_id: None,
            }
            .audit_action(),
            "custom:install"
        );
        assert_eq!(
            CartridgeTransitionOp::WriteOff {
                date_utc: 0,
                notes: None,
            }
            .audit_action(),
            "custom:write_off"
        );
    }

    #[test]
    fn transition_op_target_status() {
        assert_eq!(
            CartridgeTransitionOp::Install {
                date_utc: 0,
                given_by_name: String::new(),
                given_to_name: String::new(),
                place_id: None,
                printer_device_id: None,
                previous_cartridge_state_id: None,
                previous_cartridge_place_id: None,
            }
            .target_status_id(),
            2
        );
        assert_eq!(
            CartridgeTransitionOp::WriteOff {
                date_utc: 0,
                notes: None,
            }
            .target_status_id(),
            4
        );
    }
}
