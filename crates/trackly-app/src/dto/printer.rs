//! Printer DTOs — shared between Tauri command handlers and axum HTTP handlers.
//!
//! Snake_case JSON (S-2). All `i64` fields carry `#[specta(type = i32)]`.
//! community NEVER appears in PrinterDto (Pitfall 4 from RESEARCH.md) —
//! only `community_configured: bool` is exposed as a safe indicator.

use serde::{Deserialize, Serialize};
use specta::Type;
use trackly_core::auth::{Identity, Role};
use trackly_core::domain::printers::PrinterRow;

/// Public printer DTO — what the UI receives.
///
/// `community` is deliberately absent — only `community_configured: bool`
/// indicates that a community string has been set (T-06-07-I).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PrinterDto {
    #[specta(type = i32)]
    pub id: i64,
    #[specta(type = i32)]
    pub device_id: i64,
    pub ip_address: Option<String>,
    pub snmp_version: String,
    pub vendor: Option<String>,
    #[specta(type = Option<i32>)]
    pub oid_profile_id: Option<i64>,
    #[specta(type = Option<i32>)]
    pub last_seen_utc: Option<i64>,
    /// community deliberately absent — never serialize to frontend (Pitfall 4).
    pub community_configured: bool,
    pub device_name: Option<String>,
    pub device_location: Option<String>,
    #[specta(type = Option<i32>)]
    pub usb_host_device_id: Option<i64>,
    /// Latest reading fields (denormalized for card display).
    /// Parsed from toner_levels JSON; None if no reading yet.
    pub toner_levels: Option<serde_json::Value>,
    #[specta(type = Option<i32>)]
    pub page_count: Option<i64>,
    pub status: Option<String>,
    /// Alert indicator (true if an un-acknowledged alert exists).
    pub has_alert: bool,
    pub alert_type: Option<String>,
    /// Current cartridge installed in this printer (D-PRN07-01).
    #[specta(type = Option<i32>)]
    pub current_cartridge_id: Option<i64>,
    #[specta(type = i32)]
    pub version: i64,
}

impl From<PrinterRow> for PrinterDto {
    fn from(r: PrinterRow) -> Self {
        Self {
            id: r.id,
            device_id: r.device_id,
            ip_address: r.ip_address,
            snmp_version: r.snmp_version,
            vendor: r.vendor,
            oid_profile_id: r.oid_profile_id,
            last_seen_utc: r.last_seen_utc,
            // community_configured is set to true as a placeholder;
            // the service layer sets it to true when community != default.
            // Always true here since we never store empty community.
            community_configured: true,
            device_name: r.device_name,
            device_location: r.device_location,
            usb_host_device_id: r.usb_host_device_id,
            // Reading fields populated by service (get_last_reading).
            toner_levels: None,
            page_count: None,
            status: None,
            has_alert: false,
            alert_type: None,
            // current_cartridge_id populated by service (current_cartridge_for_printer).
            current_cartridge_id: None,
            version: r.version,
        }
    }
}

/// Filter parameters for printer list queries.
///
/// Used by Tauri commands and axum HTTP handlers.
/// Converts to domain `PrinterFilter` for service calls.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PrinterFilter {
    /// "ok" | "warning" | "error" | "offline" | "unknown" | null (all)
    pub status: Option<String>,
    pub search: Option<String>,
}

impl From<PrinterFilter> for trackly_core::domain::printers::PrinterFilter {
    fn from(f: PrinterFilter) -> Self {
        Self {
            status: f.status,
            search: f.search,
        }
    }
}

/// Pagination for printer list.
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

impl From<Pagination> for trackly_core::domain::printers::Pagination {
    fn from(p: Pagination) -> Self {
        Self {
            offset: p.offset,
            limit: p.limit,
        }
    }
}

/// Input DTO for creating a printer record.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PrinterCreateDto {
    pub device_id: i32,
    pub ip_address: Option<String>,
    /// `Some(s)` = set/change community, `None` = keep 'public' default (Pitfall 4).
    pub community_update: Option<String>,
    pub snmp_version: String,
    #[specta(type = Option<i32>)]
    pub oid_profile_id: Option<i64>,
    pub usb_host_device_id: Option<i32>,
}

/// A printer found during SNMP discovery scan (D-Discovery-01).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveredPrinterDto {
    pub ip: String,
    pub vendor: Option<String>,
    pub model: Option<String>,
    pub sys_name: String,
    #[specta(type = Option<i32>)]
    pub oid_profile_id: Option<i64>,
    /// True if a printer with this IP already exists in the DB.
    pub is_duplicate: bool,
}

/// Paginated printer list response.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PrinterListResponse {
    pub items: Vec<PrinterDto>,
    #[specta(type = i32)]
    pub total: i64,
}

/// WebSocket broadcast event — fan-out to all connected clients (D-Notify-01).
///
/// `#[serde(tag = "type", rename_all = "snake_case")]` so the wire format is:
///   `{ "type": "new_request", "request_id": 7, ... }`
///
/// Role-based visibility is enforced by `is_visible_to` — server-side filter
/// before forwarding over WS (T-06-06-I).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsEvent {
    /// A new request has been submitted.
    NewRequest {
        #[specta(type = i32)]
        request_id: i64,
        request_type: String,
        requester_name: String,
    },
    /// A request has changed status (Accept/Reject/Complete).
    /// NOTE: must be `RequestStatusChanged` — NOT `RequestUpdated` (06-CONTEXT sync).
    /// `requested_by_user_id` (D-WS-01) carries the request's author so
    /// `is_visible_to` can let the employee-author see their own status
    /// change, without opening the event to every employee (BOLA guard).
    RequestStatusChanged {
        #[specta(type = i32)]
        request_id: i64,
        new_status: String,
        #[specta(type = i32)]
        requested_by_user_id: i64,
    },
    /// A printer has an active alert (error or offline).
    PrinterAlert {
        #[specta(type = i32)]
        printer_id: i64,
        printer_name: String,
        alert_type: String,
    },
}

impl WsEvent {
    /// Returns true if this event should be forwarded to `identity`.
    ///
    /// - `PrinterAlert` → Admin | Manager only (T-06-06-I).
    /// - `NewRequest` → Admin | Manager only (сотрудник не должен видеть чужие
    ///   новые заявки — только админ/менеджер обрабатывают входящие).
    /// - `RequestStatusChanged` (D-WS-01) → Admin | Manager (видят все, как и
    ///   раньше) OR the employee who authored the request
    ///   (`identity.user_id == Some(requested_by_user_id)`) — split arm so the
    ///   author gets realtime status updates on their OWN request without
    ///   leaking other employees' request statuses (BOLA guard, T-11-03-I).
    pub fn is_visible_to(&self, identity: &Identity) -> bool {
        match self {
            WsEvent::PrinterAlert { .. } => {
                matches!(identity.role, Role::Admin | Role::Manager)
            }
            WsEvent::NewRequest { .. } => {
                matches!(identity.role, Role::Admin | Role::Manager)
            }
            WsEvent::RequestStatusChanged {
                requested_by_user_id,
                ..
            } => {
                matches!(identity.role, Role::Admin | Role::Manager)
                    || identity.user_id == Some(*requested_by_user_id)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity(user_id: Option<i64>, role: Role) -> Identity {
        Identity { user_id, role }
    }

    #[test]
    fn request_status_changed_visible_to_author_employee() {
        let event = WsEvent::RequestStatusChanged {
            request_id: 1,
            new_status: "in_progress".to_string(),
            requested_by_user_id: 42,
        };
        assert!(event.is_visible_to(&identity(Some(42), Role::Employee)));
    }

    #[test]
    fn request_status_changed_not_visible_to_other_employee() {
        let event = WsEvent::RequestStatusChanged {
            request_id: 1,
            new_status: "in_progress".to_string(),
            requested_by_user_id: 42,
        };
        assert!(!event.is_visible_to(&identity(Some(7), Role::Employee)));
    }

    #[test]
    fn request_status_changed_visible_to_admin_and_manager() {
        let event = WsEvent::RequestStatusChanged {
            request_id: 1,
            new_status: "completed".to_string(),
            requested_by_user_id: 42,
        };
        assert!(event.is_visible_to(&identity(None, Role::Admin)));
        assert!(event.is_visible_to(&identity(Some(99), Role::Manager)));
    }

    #[test]
    fn new_request_still_admin_manager_only_after_split() {
        let event = WsEvent::NewRequest {
            request_id: 1,
            request_type: "free_form".to_string(),
            requester_name: "Иванов И.И.".to_string(),
        };
        assert!(!event.is_visible_to(&identity(Some(42), Role::Employee)));
        assert!(event.is_visible_to(&identity(None, Role::Admin)));
        assert!(event.is_visible_to(&identity(Some(99), Role::Manager)));
    }
}
