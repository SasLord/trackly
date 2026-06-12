//! Cartridge DTOs — shared between Tauri command handlers and axum HTTP handlers.
//!
//! Snake_case JSON (S-2). All `i64` fields carry `#[specta(type = i32)]` so
//! TypeScript bindings see `number` rather than `bigint` — same convention as
//! `dto/act.rs` and `dto/device.rs`.
//!
//! `CartridgeTransitionPayload` uses `#[serde(tag = "op")]` so the UI sends
//! `{ "op": "install", "cartridge_id": 7, ... }` — the discriminant is the
//! operation name (D-Op-Modal-01).

use serde::{Deserialize, Serialize};
use specta::Type;
use trackly_core::domain::cartridges::{
    CartridgeCounts, CartridgeModelRow as DomainModelRow, CartridgeRow, LowStockItem,
};

/// Public cartridge DTO — what the UI receives.
///
/// All FK / counter fields that would become `bigint` in TypeScript carry
/// `#[specta(type = i32)]` so the binding stays `number` (S-3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct CartridgeDto {
    #[specta(type = i32)]
    pub id: i64,
    #[specta(type = i32)]
    pub version: i64,
    /// Human-visible code (C-NNNNNN or custom barcode).
    pub code: String,
    #[specta(type = i32)]
    pub model_id: i64,
    pub model_brand: Option<String>,
    pub model_name: Option<String>,
    #[specta(type = Option<i32>)]
    pub model_kind_id: Option<i64>,
    #[specta(type = i32)]
    pub status_id: i64,
    pub status_name: Option<String>,
    #[specta(type = Option<i32>)]
    pub state_id: Option<i64>,
    pub state_name: Option<String>,
    pub location: Option<String>,
    pub holder_name: Option<String>,
    pub notes: Option<String>,
    #[specta(type = i32)]
    pub created_at_utc: i64,
    #[specta(type = i32)]
    pub updated_at_utc: i64,
    #[specta(type = Option<i32>)]
    pub deleted_at_utc: Option<i64>,
}

impl From<CartridgeRow> for CartridgeDto {
    fn from(r: CartridgeRow) -> Self {
        Self {
            id: r.id,
            version: r.version,
            code: r.code,
            model_id: r.model_id,
            model_brand: r.model_brand,
            model_name: r.model_name,
            model_kind_id: r.model_kind_id,
            status_id: r.status_id,
            status_name: r.status_name,
            state_id: r.state_id,
            state_name: r.state_name,
            location: r.location,
            holder_name: r.holder_name,
            notes: r.notes,
            created_at_utc: r.created_at_utc,
            updated_at_utc: r.updated_at_utc,
            deleted_at_utc: r.deleted_at_utc,
        }
    }
}

/// Public cartridge model DTO.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct CartridgeModelDto {
    #[specta(type = i32)]
    pub id: i64,
    #[specta(type = i32)]
    pub version: i64,
    pub brand: String,
    pub model: String,
    #[specta(type = i32)]
    pub kind_id: i64,
    pub color: Option<String>,
    pub notes: Option<String>,
    #[specta(type = i32)]
    pub created_at_utc: i64,
    #[specta(type = i32)]
    pub updated_at_utc: i64,
    /// Compatibility pairs (printer_brand, printer_model).
    pub compatibility: Vec<(String, String)>,
}

impl CartridgeModelDto {
    /// Build a DTO from a domain row + loaded compatibility pairs.
    pub fn from_row(r: DomainModelRow, compatibility: Vec<(String, String)>) -> Self {
        Self {
            id: r.id,
            version: r.version,
            brand: r.brand,
            model: r.model,
            kind_id: r.kind_id,
            color: r.color,
            notes: r.notes,
            created_at_utc: r.created_at_utc,
            updated_at_utc: r.updated_at_utc,
            compatibility,
        }
    }
}

/// Payload sent by the UI when creating a new cartridge instance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type, Default)]
pub struct CartridgeCreateDto {
    #[specta(type = i32)]
    pub model_id: i64,
    /// None → auto-code C-NNNNNN from `cartridge_seq` counter.
    /// Some(s) → custom barcode / inventory code (validated 1-32 chars, no ctrl chars).
    pub code_override: Option<String>,
    #[specta(type = Option<i32>)]
    pub state_id: Option<i64>,
    pub location: Option<String>,
    pub notes: Option<String>,
}

/// Payload sent by the UI when creating a new cartridge model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type, Default)]
pub struct CartridgeModelCreateDto {
    pub brand: String,
    pub model: String,
    #[specta(type = i32)]
    pub kind_id: i64,
    pub color: Option<String>,
    pub notes: Option<String>,
    /// Compatibility pairs (printer_brand, printer_model).
    #[serde(default)]
    pub compatibility: Vec<(String, String)>,
}

/// Payload for updating an existing cartridge model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type, Default)]
pub struct CartridgeModelPatchDto {
    #[specta(type = i32)]
    pub id: i64,
    #[specta(type = i32)]
    pub version: i64,
    pub brand: String,
    pub model: String,
    #[specta(type = i32)]
    pub kind_id: i64,
    pub color: Option<String>,
    pub notes: Option<String>,
    #[serde(default)]
    pub compatibility: Vec<(String, String)>,
}

/// Lifecycle transition payload.
///
/// `#[serde(tag = "op")]` means the UI sends `{ "op": "install", ... }`.
/// The discriminant string matches the domain audit_action minus the `custom:` prefix.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(tag = "op")]
pub enum CartridgeTransitionPayload {
    /// Установить в принтер: На складе (1) → В работе (2).
    #[serde(rename = "install")]
    Install {
        #[specta(type = i32)]
        cartridge_id: i64,
        #[specta(type = i32)]
        version: i64,
        #[specta(type = i32)]
        date_utc: i64,
        given_by_name: String,
        given_to_name: String,
        location: String,
    },
    /// Вернуть на склад: В работе (2) → На складе (1).
    #[serde(rename = "return_to_stock")]
    ReturnToStock {
        #[specta(type = i32)]
        cartridge_id: i64,
        #[specta(type = i32)]
        version: i64,
        #[specta(type = i32)]
        state_id: i64,
        location: String,
        notes: Option<String>,
    },
    /// Отправить на заправку: На складе (1) → На заправке (3).
    #[serde(rename = "to_refill")]
    ToRefill {
        #[specta(type = i32)]
        cartridge_id: i64,
        #[specta(type = i32)]
        version: i64,
        #[specta(type = i32)]
        date_utc: i64,
        given_by_name: String,
        given_to_name: String,
        location: String,
    },
    /// Забрать с заправки: На заправке (3) → На складе (1).
    #[serde(rename = "from_refill")]
    FromRefill {
        #[specta(type = i32)]
        cartridge_id: i64,
        #[specta(type = i32)]
        version: i64,
        #[specta(type = i32)]
        state_id: i64,
        location: String,
        notes: Option<String>,
    },
    /// Списать: любой статус != 4 → Списано (4).
    #[serde(rename = "write_off")]
    WriteOff {
        #[specta(type = i32)]
        cartridge_id: i64,
        #[specta(type = i32)]
        version: i64,
        #[specta(type = i32)]
        date_utc: i64,
        notes: Option<String>,
    },
}

impl CartridgeTransitionPayload {
    /// Extract the cartridge_id from any variant.
    pub fn cartridge_id(&self) -> i64 {
        match self {
            Self::Install { cartridge_id, .. } => *cartridge_id,
            Self::ReturnToStock { cartridge_id, .. } => *cartridge_id,
            Self::ToRefill { cartridge_id, .. } => *cartridge_id,
            Self::FromRefill { cartridge_id, .. } => *cartridge_id,
            Self::WriteOff { cartridge_id, .. } => *cartridge_id,
        }
    }

    /// Extract the optimistic-lock version from any variant.
    pub fn version(&self) -> i64 {
        match self {
            Self::Install { version, .. } => *version,
            Self::ReturnToStock { version, .. } => *version,
            Self::ToRefill { version, .. } => *version,
            Self::FromRefill { version, .. } => *version,
            Self::WriteOff { version, .. } => *version,
        }
    }
}

/// Convert DTO payload into the domain enum for service/infra use.
impl From<CartridgeTransitionPayload>
    for trackly_core::domain::cartridges::CartridgeTransitionOp
{
    fn from(p: CartridgeTransitionPayload) -> Self {
        use trackly_core::domain::cartridges::CartridgeTransitionOp;
        match p {
            CartridgeTransitionPayload::Install {
                date_utc,
                given_by_name,
                given_to_name,
                location,
                ..
            } => CartridgeTransitionOp::Install {
                date_utc,
                given_by_name,
                given_to_name,
                location,
            },
            CartridgeTransitionPayload::ReturnToStock {
                state_id,
                location,
                notes,
                ..
            } => CartridgeTransitionOp::ReturnToStock {
                state_id,
                location,
                notes,
            },
            CartridgeTransitionPayload::ToRefill {
                date_utc,
                given_by_name,
                given_to_name,
                location,
                ..
            } => CartridgeTransitionOp::ToRefill {
                date_utc,
                given_by_name,
                given_to_name,
                location,
            },
            CartridgeTransitionPayload::FromRefill {
                state_id,
                location,
                notes,
                ..
            } => CartridgeTransitionOp::FromRefill {
                state_id,
                location,
                notes,
            },
            CartridgeTransitionPayload::WriteOff {
                date_utc, notes, ..
            } => CartridgeTransitionOp::WriteOff { date_utc, notes },
        }
    }
}

/// Filter passed by the UI to `cartridges_list` / `cartridges_search`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type, Default)]
pub struct CartridgeFilter {
    #[specta(type = Option<i32>)]
    pub status_id: Option<i64>,
    #[specta(type = Option<i32>)]
    pub kind_id: Option<i64>,
    #[specta(type = Option<i32>)]
    pub model_id: Option<i64>,
    pub search: Option<String>,
    #[serde(default)]
    pub include_deleted: bool,
}

impl CartridgeFilter {
    /// Convert into the domain filter for repository calls.
    pub fn into_domain(self) -> trackly_core::domain::cartridges::CartridgeFilter {
        trackly_core::domain::cartridges::CartridgeFilter {
            status_id: self.status_id,
            kind_id: self.kind_id,
            model_id: self.model_id,
            search: self.search,
            include_deleted: self.include_deleted,
        }
    }
}

/// Pagination — mirrors `dto::act::Pagination`.
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

impl From<Pagination> for trackly_core::domain::cartridges::Pagination {
    fn from(p: Pagination) -> Self {
        Self {
            offset: p.offset,
            limit: p.limit,
        }
    }
}

/// Response of `cartridges_list`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct CartridgeListResponse {
    pub items: Vec<CartridgeDto>,
    #[specta(type = u32)]
    pub total: u64,
}

/// Status switch-bar counters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type, Default)]
pub struct CartridgeCountsDto {
    #[specta(type = i32)]
    pub all: i64,
    #[specta(type = i32)]
    pub in_stock: i64,
    #[specta(type = i32)]
    pub in_use: i64,
    #[specta(type = i32)]
    pub at_refill: i64,
    #[specta(type = i32)]
    pub written_off: i64,
}

impl From<CartridgeCounts> for CartridgeCountsDto {
    fn from(c: CartridgeCounts) -> Self {
        Self {
            all: c.all,
            in_stock: c.in_stock,
            in_use: c.in_use,
            at_refill: c.at_refill,
            written_off: c.written_off,
        }
    }
}

/// A model below the low-stock threshold.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct LowStockItemDto {
    #[specta(type = i32)]
    pub model_id: i64,
    pub brand: String,
    pub model: String,
    #[specta(type = i32)]
    pub count: i64,
    #[specta(type = i32)]
    pub threshold: i64,
}

impl From<LowStockItem> for LowStockItemDto {
    fn from(i: LowStockItem) -> Self {
        Self {
            model_id: i.model_id,
            brand: i.brand,
            model: i.model,
            count: i.count,
            threshold: i.threshold,
        }
    }
}

/// A single row from the cartridge audit history.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct AuditEntryDto {
    /// Primary key of the audit_log row — stable unique key for UI list rendering.
    #[specta(type = i32)]
    pub id: i64,
    pub action: String,
    pub payload_json: Option<String>,
    pub before_json: Option<String>,
    pub after_json: Option<String>,
    #[specta(type = i32)]
    pub created_at_utc: i64,
}
