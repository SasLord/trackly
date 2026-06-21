//! Request DTOs — shared between Tauri command handlers and axum HTTP handlers.
//!
//! Snake_case JSON (S-2). All `i64` fields carry `#[specta(type = i32)]`.
//!
//! `RequestTransitionPayload` uses `#[serde(tag = "op")]` so the UI sends
//! `{ "op": "accept", "requestId": 3, "version": 1 }` — this is one of the
//! few camelCase exceptions to S-2 (see the doc comment on the enum itself
//! for why each variant needs its own `rename_all`).

use serde::{Deserialize, Serialize};
use specta::Type;
use trackly_core::domain::requests::RequestRow;

/// Public request DTO — what the UI receives.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RequestDto {
    #[specta(type = i32)]
    pub id: i64,
    pub request_type: String,
    pub status: String,
    #[specta(type = i32)]
    pub requested_by_user_id: i64,
    #[specta(type = Option<i32>)]
    pub assigned_to_user_id: Option<i64>,
    #[specta(type = Option<i32>)]
    pub printer_device_id: Option<i64>,
    #[specta(type = Option<i32>)]
    pub cartridge_model_id: Option<i64>,
    #[specta(type = Option<i32>)]
    pub category_id: Option<i64>,
    #[specta(type = Option<i32>)]
    pub completed_cartridge_id: Option<i64>,
    pub description: Option<String>,
    pub resolution_notes: Option<String>,
    pub requester_name: Option<String>,
    pub printer_name: Option<String>,
    #[specta(type = i32)]
    pub created_at_utc: i64,
    #[specta(type = i32)]
    pub updated_at_utc: i64,
    #[specta(type = Option<i32>)]
    pub deleted_at_utc: Option<i64>,
    #[specta(type = i32)]
    pub version: i64,
    /// "register" | "restore" | null — only set for `request_type = 'ad_register'` (V028).
    pub ad_subtype: Option<String>,
    /// Joined: request_categories.name (D-CAT-01) — display name for `category_id`.
    /// `None` when the request has no category (e.g. cartridge_replace).
    pub category_name: Option<String>,
}

impl From<RequestRow> for RequestDto {
    fn from(r: RequestRow) -> Self {
        Self {
            id: r.id,
            request_type: r.request_type,
            status: r.status,
            requested_by_user_id: r.requested_by_user_id,
            assigned_to_user_id: r.assigned_to_user_id,
            printer_device_id: r.printer_device_id,
            cartridge_model_id: r.cartridge_model_id,
            category_id: r.category_id,
            completed_cartridge_id: r.completed_cartridge_id,
            description: r.description,
            resolution_notes: r.resolution_notes,
            requester_name: r.requester_name,
            printer_name: r.printer_name,
            created_at_utc: r.created_at_utc,
            updated_at_utc: r.updated_at_utc,
            deleted_at_utc: r.deleted_at_utc,
            version: r.version,
            ad_subtype: r.ad_subtype,
            category_name: r.category_name,
        }
    }
}

/// A single request category option `{ id, name }` (D-CAT-01).
///
/// Replaces the old bare `Vec<String>` shape — the form needs the FK id to
/// send a correct `category_id`, not just the display name.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RequestCategoryDto {
    #[specta(type = i32)]
    pub id: i64,
    pub name: String,
}

/// A single printer option `{ id, name, location }` for the create-request
/// form's printer dropdown (D-PRN-01).
///
/// Gated behind `Action::CreateRequest` (employee has it) — deliberately NOT
/// the closed `ReadData`/`ReadPrinters` actions (Phase 10 BFLA closure). The
/// shape is intentionally minimal: no SNMP/community/IP/serial fields leave
/// the server, since an Employee caller must not be able to read device
/// internals through this endpoint (BOLA/BOPLA closure, T-11-02-I).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RequestPrinterOptionDto {
    /// Device id — sent back as `printerDeviceId` on `RequestCreateDto`.
    #[specta(type = i32)]
    pub id: i64,
    pub name: String,
    /// Joined `locations.name` — `None` when the printer has no location set.
    pub location: Option<String>,
}

/// Filter parameters for request list queries.
///
/// Used by Tauri commands and axum HTTP handlers.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RequestFilter {
    pub status: Option<String>,
    pub request_type: Option<String>,
    pub assigned_to_user_id: Option<i32>,
    pub requested_by_user_id: Option<i32>,
}

impl From<RequestFilter> for trackly_core::domain::requests::RequestFilter {
    fn from(f: RequestFilter) -> Self {
        Self {
            status: f.status,
            request_type: f.request_type,
            assigned_to_user_id: f.assigned_to_user_id.map(|id| id as i64),
            requested_by_user_id: f.requested_by_user_id.map(|id| id as i64),
        }
    }
}

/// Pagination for request list.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
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

impl From<Pagination> for trackly_core::domain::requests::Pagination {
    fn from(p: Pagination) -> Self {
        Self {
            offset: p.offset,
            limit: p.limit,
        }
    }
}

/// Input DTO for creating a new request.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RequestCreateDto {
    /// "cartridge_replace" | "free_form"
    pub request_type: String,
    /// Required for cartridge_replace; which printer.
    pub printer_device_id: Option<i32>,
    /// Optional: which cartridge model to replace.
    pub cartridge_model_id: Option<i32>,
    /// For free_form requests: category.
    pub category_id: Option<i32>,
    pub description: Option<String>,
}

/// Lifecycle transition payload — UI sends the op discriminant + params.
///
/// Wire format: `{ "op": "accept", "requestId": 3, "version": 1 }`
///
/// `rename_all = "camelCase"` on the enum container only renames the `op`
/// tag values (variant names) for an internally-tagged enum — it does NOT
/// cascade to each variant's field names (serde semantics, confirmed via
/// minimal repro during the 09-AD-GAPS fix). Each variant therefore needs
/// its OWN `#[serde(rename_all = "camelCase")]` so `requestId` deserializes
/// into `request_id` etc. Without this, every real JSON call (HTTP body or
/// Tauri IPC payload — both go through this same `Deserialize` impl) fails
/// with "missing field `request_id`", which axum's default `Json` rejection
/// then returns as a plain-text 422 body, not a structured AppError — this
/// is what surfaced as the generic "Не удалось связаться с приложением"
/// toast on reject (Defect 2, 09-AD-GAPS).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(tag = "op", rename_all = "camelCase")]
pub enum RequestTransitionPayload {
    #[serde(rename_all = "camelCase")]
    Accept {
        #[specta(type = i32)]
        request_id: i64,
        #[specta(type = i32)]
        version: i64,
        assigned_to_user_id: Option<i32>,
    },
    #[serde(rename_all = "camelCase")]
    Reject {
        #[specta(type = i32)]
        request_id: i64,
        #[specta(type = i32)]
        version: i64,
        notes: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    Complete {
        #[specta(type = i32)]
        request_id: i64,
        #[specta(type = i32)]
        version: i64,
        notes: Option<String>,
        /// Links a cartridge installation (REQ-05 / D-Req-CART07-01).
        linked_cartridge_id: Option<i32>,
    },
}

/// Approve an `ad_register` request — admin selects the role to grant
/// (defaults to "employee" if absent, D-REG-02). Distinct from
/// `RequestTransitionPayload` because approve has side effects on the
/// `users` table (activate or revive) that generic transitions don't have.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ApproveAdRegisterDto {
    #[specta(type = i32)]
    pub request_id: i64,
    #[specta(type = i32)]
    pub version: i64,
    /// "admin" | "manager" | "employee" — defaults to "employee" if None (D-REG-02).
    pub role: Option<String>,
}

/// Paginated request list response.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RequestListResponse {
    pub items: Vec<RequestDto>,
    #[specta(type = i32)]
    pub total: i64,
}

/// Single audit_log entry for the request History block (REQ-07).
///
/// camelCase JSON to match the rest of this module and the frontend
/// `RequestHistoryEntry` interface (`createdAtUtc`, `actorName`, `notes`).
/// Distinct from the cartridge `AuditEntryDto` (snake_case) — this one joins
/// the actor name and surfaces transition notes so the UI can render
/// `дата — действие; автор; примечание`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RequestHistoryEntryDto {
    /// Primary key of the audit_log row — stable unique key for UI list keying.
    #[specta(type = i32)]
    pub id: i64,
    pub action: String,
    #[specta(type = i32)]
    pub created_at_utc: i64,
    /// Full name of the user who performed the action (NULL for system rows).
    pub actor_name: Option<String>,
    /// Free-text notes captured with the action (reject/complete reason).
    pub notes: Option<String>,
}

/// Aggregate counts for the status switch-bar.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RequestCountsDto {
    #[specta(type = i32)]
    pub all: i64,
    #[specta(type = i32)]
    pub open: i64,
    #[specta(type = i32)]
    pub in_progress: i64,
    #[specta(type = i32)]
    pub completed: i64,
    #[specta(type = i32)]
    pub rejected: i64,
}

#[cfg(test)]
mod wire_contract_tests {
    //! Locks in the exact JSON wire shape the frontend sends for
    //! `RequestTransitionPayload` (09-AD-GAPS Defect 2). Every existing
    //! integration test constructed this enum directly as a Rust value,
    //! never round-tripping through `serde_json` — which is exactly how a
    //! per-variant `rename_all` gap on an internally-tagged enum went
    //! undetected: the enum-level `rename_all = "camelCase"` only renames
    //! the `op` tag values, not each variant's field names.
    use super::RequestTransitionPayload;

    #[test]
    fn reject_deserializes_camel_case_wire_format() {
        let json = r#"{"op":"reject","requestId":5,"version":1,"notes":"дубликат"}"#;
        let payload: RequestTransitionPayload =
            serde_json::from_str(json).expect("camelCase wire format must deserialize");
        match payload {
            RequestTransitionPayload::Reject {
                request_id,
                version,
                notes,
            } => {
                assert_eq!(request_id, 5);
                assert_eq!(version, 1);
                assert_eq!(notes, Some("дубликат".to_string()));
            }
            other => panic!("expected Reject, got {other:?}"),
        }
    }

    #[test]
    fn accept_deserializes_camel_case_wire_format() {
        let json = r#"{"op":"accept","requestId":7,"version":2,"assignedToUserId":3}"#;
        let payload: RequestTransitionPayload =
            serde_json::from_str(json).expect("camelCase wire format must deserialize");
        match payload {
            RequestTransitionPayload::Accept {
                request_id,
                version,
                assigned_to_user_id,
            } => {
                assert_eq!(request_id, 7);
                assert_eq!(version, 2);
                assert_eq!(assigned_to_user_id, Some(3));
            }
            other => panic!("expected Accept, got {other:?}"),
        }
    }

    #[test]
    fn complete_deserializes_camel_case_wire_format() {
        let json =
            r#"{"op":"complete","requestId":9,"version":1,"notes":null,"linkedCartridgeId":42}"#;
        let payload: RequestTransitionPayload =
            serde_json::from_str(json).expect("camelCase wire format must deserialize");
        match payload {
            RequestTransitionPayload::Complete {
                request_id,
                version,
                notes,
                linked_cartridge_id,
            } => {
                assert_eq!(request_id, 9);
                assert_eq!(version, 1);
                assert_eq!(notes, None);
                assert_eq!(linked_cartridge_id, Some(42));
            }
            other => panic!("expected Complete, got {other:?}"),
        }
    }
}
