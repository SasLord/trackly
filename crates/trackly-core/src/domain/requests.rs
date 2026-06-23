//! Domain value types for the Requests entity.
//!
//! NO serde::Serialize/Deserialize or specta::Type derives here — those live
//! in the DTO layer in trackly-app. Only `#[derive(Debug, Clone, PartialEq, Eq)]`.
//!
//! See D-Req-Form-01, D-Req-Categories-01, D-Req-Lifecycle-01.

pub use crate::domain::printers::RequestTransitionOp;

/// Full request row as returned from the repository read path.
/// Includes joined display columns (requester_name, printer_name).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestRow {
    pub id: i64,
    /// "cartridge_replace" | "free_form" | "ad_register"
    pub request_type: String,
    /// "open" | "in_progress" | "completed" | "rejected"
    pub status: String,
    pub requested_by_user_id: i64,
    pub assigned_to_user_id: Option<i64>,
    pub printer_device_id: Option<i64>,
    pub cartridge_model_id: Option<i64>,
    /// FK → request_categories(id) (for free_form requests).
    pub category_id: Option<i64>,
    /// FK → cartridges(id) — set on Complete transition (REQ-05).
    pub completed_cartridge_id: Option<i64>,
    pub description: Option<String>,
    pub resolution_notes: Option<String>,
    /// Joined: users.display_name of requester.
    pub requester_name: Option<String>,
    /// Joined: devices.name of the printer (for cartridge_replace requests).
    pub printer_name: Option<String>,
    /// Joined `locations.name` через `devices.location_id` принтера заявки
    /// (D-05, Phase 12); `None` если принтер не выбран или у него нет
    /// расположения.
    pub printer_location: Option<String>,
    pub created_at_utc: i64,
    pub updated_at_utc: i64,
    pub deleted_at_utc: Option<i64>,
    pub version: i64,
    /// Discriminator for `request_type = 'ad_register'` rows (V028, D-REG-03):
    /// `Some("register")` — new/unknown AD user; `Some("restore")` —
    /// blocked/soft-deleted AD user requesting reactivation. `None` for all
    /// other request types.
    pub ad_subtype: Option<String>,
    /// Joined: request_categories.name (D-CAT-01). `None` for requests
    /// without a category (e.g. cartridge_replace, or free_form with no
    /// category set).
    pub category_name: Option<String>,
}

/// Data needed to create a new request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestNew {
    /// "cartridge_replace" | "free_form" | "ad_register"
    pub request_type: String,
    pub requested_by_user_id: i64,
    /// Required for cartridge_replace type.
    pub printer_device_id: Option<i64>,
    /// Optional for cartridge_replace.
    pub cartridge_model_id: Option<i64>,
    /// Required for free_form type.
    pub category_id: Option<i64>,
    pub description: Option<String>,
    /// "register" | "restore" — only set when `request_type = 'ad_register'` (V028).
    pub ad_subtype: Option<String>,
}

/// Filter parameters for request list queries.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RequestFilter {
    /// Filter by status; None = all non-deleted.
    pub status: Option<String>,
    /// Filter by request_type; None = all.
    pub request_type: Option<String>,
    /// Filter by assigned user.
    pub assigned_to_user_id: Option<i64>,
    /// Requester user ID filter.
    pub requested_by_user_id: Option<i64>,
}

/// Aggregate counts for request status switch-bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RequestCounts {
    pub all: i64,
    pub open: i64,
    pub in_progress: i64,
    pub completed: i64,
    pub rejected: i64,
    pub cancelled: i64,
}

/// Pagination parameters for request list queries.
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
