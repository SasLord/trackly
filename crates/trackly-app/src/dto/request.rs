//! Request DTOs — shared between Tauri command handlers and axum HTTP handlers.
//!
//! Snake_case JSON (S-2). All `i64` fields carry `#[specta(type = i32)]`.
//!
//! `RequestTransitionPayload` uses `#[serde(tag = "op")]` so the UI sends
//! `{ "op": "accept", "request_id": 3, "version": 1 }`.

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
        }
    }
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
        Self { offset: 0, limit: 50 }
    }
}

impl From<Pagination> for trackly_core::domain::requests::Pagination {
    fn from(p: Pagination) -> Self {
        Self { offset: p.offset, limit: p.limit }
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
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(tag = "op", rename_all = "camelCase")]
pub enum RequestTransitionPayload {
    Accept {
        #[specta(type = i32)]
        request_id: i64,
        #[specta(type = i32)]
        version: i64,
        assigned_to_user_id: Option<i32>,
    },
    Reject {
        #[specta(type = i32)]
        request_id: i64,
        #[specta(type = i32)]
        version: i64,
        notes: Option<String>,
    },
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

/// Paginated request list response.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RequestListResponse {
    pub items: Vec<RequestDto>,
    #[specta(type = i32)]
    pub total: i64,
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
