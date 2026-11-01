//! Domain value types for the Printers entity.
//!
//! NO serde::Serialize/Deserialize or specta::Type derives here — those live
//! in the DTO layer in trackly-app. Only `#[derive(Debug, Clone, PartialEq, Eq)]`.
//!
//! See D-Schema-01 (printers extends devices), D-History-01 (printer_readings),
//! D-Alert-01 (printer_alerts), D-OID-01 (oid_profiles data-driven).

use crate::error::AppError;

/// Full printer row as returned from the repository read path.
/// Joined columns (device_name, device_place, device_place_id) come from the devices table.
/// community is NOT included — it is kept as Secret<String> in the service layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrinterRow {
    pub id: i64,
    pub device_id: i64,
    /// None for USB-only printers (PRN-04).
    pub ip_address: Option<String>,
    /// "v1" | "v2c" | "v3"
    pub snmp_version: String,
    /// Vendor detected at discovery (e.g. "Pantum", "HP").
    pub vendor: Option<String>,
    pub oid_profile_id: Option<i64>,
    pub last_seen_utc: Option<i64>,
    /// FK → devices(id) for the USB host workstation (PRN-04).
    pub usb_host_device_id: Option<i64>,
    /// Joined from `devices` по `usb_host_device_id` (имя USB-хоста). Без
    /// фильтра удаления: строка `printers` переживает удаление устройства, D-37.
    pub usb_host_name: Option<String>,
    /// Joined from `devices` по `usb_host_device_id` (инвентарный номер
    /// USB-хоста). Без фильтра удаления, D-37.
    pub usb_host_inventory_no: Option<String>,
    /// Joined from devices.name.
    pub device_name: Option<String>,
    /// Resolved current place path, joined from `place_full_paths` via
    /// `devices.place_id` (display text, not a raw id).
    pub device_place: Option<String>,
    /// Raw `devices.place_id`, joined straight with no text resolution —
    /// needed to prefill a `PlacePicker` selection by id when a printer is
    /// chosen for an Install operation.
    pub device_place_id: Option<i64>,
    /// True when the printer's SNMP community differs from the default
    /// `"public"` (WR-04). Derived in the SELECT as `community <> 'public'`
    /// so the raw secret community value never leaves the repository — only
    /// this safe boolean is carried. Consumed by `PrinterDto`.
    pub community_configured: bool,
    pub created_at_utc: i64,
    pub updated_at_utc: i64,
    pub version: i64,
}

/// One polling snapshot (D-History-01).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrinterReadingRow {
    pub id: i64,
    pub printer_id: i64,
    pub ts_utc: i64,
    /// JSON: {"black":{"level":45,"max":100,"pct":45}}; None if polling failed.
    pub toner_levels_json: String,
    pub page_count: Option<i64>,
    /// "ok" | "warning" | "error" | "offline" | "unknown"
    pub status: String,
}

/// Active alert row (D-Alert-01). UNIQUE per printer — one active alert at a time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrinterAlertRow {
    pub id: i64,
    pub printer_id: i64,
    /// "offline" | "error"
    pub alert_type: String,
    pub first_seen_utc: i64,
    pub last_seen_utc: i64,
    pub acknowledged_at_utc: Option<i64>,
}

/// OID profile row (D-OID-01). Seeded by V021, UI editor deferred to Phase 7.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OidProfileRow {
    pub id: i64,
    /// "pantum" | "kyocera" | "hp" | "canon" | "rfc3805"
    pub name: String,
    /// sysObjectID prefix for vendor matching (empty string = RFC3805 fallback).
    pub vendor_prefix: String,
    pub toner_level_oid: Option<String>,
    /// None for 'percent' toner_encoding (value is already %).
    pub toner_max_oid: Option<String>,
    /// "percent" | "level_over_max"
    pub toner_encoding: String,
    pub page_counter_oid: Option<String>,
    /// hrPrinterStatus OID.
    pub status_oid: String,
    pub serial_oid: Option<String>,
}

/// A device found during SNMP discovery scan (D-Discovery-01).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredPrinter {
    pub ip: String,
    pub sys_object_id: String,
    pub sys_descr: String,
    pub sys_name: String,
    /// Matched from oid_profiles; None if no profile matched.
    pub vendor: Option<String>,
    pub oid_profile_id: Option<i64>,
    /// True if a printer with this device's IP already exists in the DB.
    pub is_duplicate: bool,
}

/// Filter parameters for printer list queries.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PrinterFilter {
    /// "ok" | "warning" | "error" | "offline" | "unknown" | None (all)
    pub status: Option<String>,
    /// Full-text / partial name search.
    pub search: Option<String>,
}

/// Data needed to create a new printer record (D-Schema-01).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrinterNew {
    pub device_id: i64,
    /// None for USB-only printer.
    pub ip_address: Option<String>,
    /// SNMP community string (stored as plain text in DB; wrapped Secret in service).
    pub community_raw: String,
    /// "v1" | "v2c" | "v3"
    pub snmp_version: String,
    pub oid_profile_id: Option<i64>,
    /// USB host workstation device ID (PRN-04).
    pub usb_host_device_id: Option<i64>,
}

/// Pagination parameters for printer list queries.
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

/// Aggregate counts for the printer status switch-bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PrinterCounts {
    pub all: i64,
    pub ok: i64,
    pub warning: i64,
    pub error: i64,
    pub offline: i64,
}

/// Request status transitions enforced at service layer (D-Req-Lifecycle-01).
/// Lives here (in printers domain module) for co-location; requests domain
/// re-exports this type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RequestTransitionOp {
    /// open → in_progress
    Accept,
    /// open OR in_progress → rejected (GAP-12-07/A4: Специалист может
    /// отклонить заявку «В работе», не только «Новая»).
    Reject { notes: Option<String> },
    /// in_progress → completed
    Complete {
        notes: Option<String>,
        /// If Some, links a cartridge installation (REQ-05 / D-Req-CART07-01).
        linked_cartridge_id: Option<i64>,
    },
    /// open → cancelled. Author-only self-cancel (GAP-12-07/A4) — does NOT
    /// go through `RequestService::transition()`/`Action::TransitionRequests`
    /// (Admin|Manager only); enforced instead via the dedicated
    /// `RequestService::cancel()` + `Action::CancelOwnRequest` path.
    Cancel,
}

impl RequestTransitionOp {
    /// Validate that `current` status allows this transition.
    ///
    /// Rewritten from the original single-expected-status tuple match
    /// (`(expected, op_name)`) to an explicit per-variant boolean check —
    /// `Reject` now accepts TWO source statuses ("open" OR "in_progress"),
    /// which a single `expected` string cannot express.
    pub fn validate_from_status(&self, current: &str) -> Result<(), AppError> {
        let (ok, op_name) = match self {
            RequestTransitionOp::Accept => (current == "open", "Принять в работу"),
            RequestTransitionOp::Reject { .. } => {
                (current == "open" || current == "in_progress", "Отклонить")
            }
            RequestTransitionOp::Complete { .. } => (current == "in_progress", "Выполнить"),
            RequestTransitionOp::Cancel => (current == "open", "Отменить"),
        };
        if !ok {
            return Err(AppError::Validation {
                field: "status".into(),
                message: format!(
                    "Операция «{}» недопустима для статуса «{}»",
                    op_name, current
                ),
            });
        }
        Ok(())
    }

    pub fn target_status(&self) -> &'static str {
        match self {
            RequestTransitionOp::Accept => "in_progress",
            RequestTransitionOp::Reject { .. } => "rejected",
            RequestTransitionOp::Complete { .. } => "completed",
            RequestTransitionOp::Cancel => "cancelled",
        }
    }

    pub fn audit_action(&self) -> &'static str {
        match self {
            RequestTransitionOp::Accept => "custom:accept",
            RequestTransitionOp::Reject { .. } => "custom:reject",
            RequestTransitionOp::Complete { .. } => "custom:complete",
            RequestTransitionOp::Cancel => "custom:cancel",
        }
    }
}

/// Нормализовать IP-адрес (или имя хоста) принтера, пришедший от клиента.
///
/// Единая функция для всех трёх путей записи адреса: создание устройства с
/// блоком принтера, редактирование устройства и прямое создание принтера.
///
/// Правило намеренно мягкое. Отклоняется только то, что опасно или не работает:
/// - строка с двоеточием. `RealSnmpClient` (`snmp/real.rs`) дописывает порт
///   `:161` только если двоеточия в строке нет, иначе отдаёт строку сессии
///   как есть; значит `адрес:порт` направил бы UDP-запрос на произвольный порт.
///   Отсюда же следует, что IPv6 не поддерживался и раньше (любое двоеточие
///   принимается за порт), так что отказ для него — не регресс, а закрепление
///   существующего факта;
/// - пробелы и управляющие символы внутри строки.
///
/// Всё остальное — и IPv4 в точечной записи, и имя хоста — пропускается как
/// есть, без канонизации и без обращения к DNS: имена хостов сегодня работают
/// (строка без двоеточия уходит в резолвер), и принтеры ими адресуют. Адреса
/// вроде `0.0.0.0` или `127.x` тоже не отклоняются.
///
/// Края обрезаются; пустая строка (после обрезки) означает «нет адреса».
pub fn normalize_printer_ip(raw: &str) -> Result<Option<String>, AppError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if trimmed.contains(':') {
        return Err(AppError::Validation {
            field: "printer_ip_address".into(),
            message: "Порт указывать нельзя: опрос принтера всегда идёт на стандартный порт. \
                      Введите адрес или имя без двоеточия."
                .into(),
        });
    }
    if trimmed.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(AppError::Validation {
            field: "printer_ip_address".into(),
            message: "Адрес не должен содержать пробелов и управляющих символов.".into(),
        });
    }
    Ok(Some(trimmed.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transition_accept_validates_open_only() {
        assert!(RequestTransitionOp::Accept
            .validate_from_status("open")
            .is_ok());
        assert!(RequestTransitionOp::Accept
            .validate_from_status("in_progress")
            .is_err());
    }

    #[test]
    fn transition_complete_validates_in_progress_only() {
        let op = RequestTransitionOp::Complete {
            notes: None,
            linked_cartridge_id: None,
        };
        assert!(op.validate_from_status("in_progress").is_ok());
        assert!(op.validate_from_status("open").is_err());
    }

    #[test]
    fn transition_target_status() {
        assert_eq!(RequestTransitionOp::Accept.target_status(), "in_progress");
        assert_eq!(
            RequestTransitionOp::Reject { notes: None }.target_status(),
            "rejected"
        );
        assert_eq!(
            RequestTransitionOp::Complete {
                notes: None,
                linked_cartridge_id: None
            }
            .target_status(),
            "completed"
        );
    }

    // GAP-12-07/A4: Reject is now valid from "open" OR "in_progress".

    #[test]
    fn transition_reject_validates_open_or_in_progress() {
        let op = RequestTransitionOp::Reject { notes: None };
        assert!(op.validate_from_status("open").is_ok());
        assert!(op.validate_from_status("in_progress").is_ok());
        assert!(op.validate_from_status("completed").is_err());
    }

    // GAP-12-07/A4: new Cancel variant — author-only self-cancel, "open" only.

    #[test]
    fn transition_cancel_validates_open_only() {
        let op = RequestTransitionOp::Cancel;
        assert!(op.validate_from_status("open").is_ok());
        assert!(op.validate_from_status("in_progress").is_err());
        assert!(op.validate_from_status("completed").is_err());
    }

    #[test]
    fn transition_cancel_target_status_and_audit_action() {
        assert_eq!(RequestTransitionOp::Cancel.target_status(), "cancelled");
        assert_eq!(RequestTransitionOp::Cancel.audit_action(), "custom:cancel");
    }

    // Phase 41.13 (D-13): normalize_printer_ip.

    fn ip_ok(raw: &str) -> Option<String> {
        normalize_printer_ip(raw).expect("должно приниматься")
    }

    fn ip_err(raw: &str) -> (String, String) {
        match normalize_printer_ip(raw) {
            Err(AppError::Validation { field, message }) => (field, message),
            other => panic!("ожидался отказ для {raw:?}, получено {other:?}"),
        }
    }

    #[test]
    fn printer_ip_accepts_ipv4_and_trims_edges() {
        assert_eq!(ip_ok("192.0.2.50"), Some("192.0.2.50".to_string()));
        assert_eq!(ip_ok(" 192.0.2.50 "), Some("192.0.2.50".to_string()));
        assert_eq!(ip_ok("\t198.51.100.7\n"), Some("198.51.100.7".to_string()));
    }

    #[test]
    fn printer_ip_accepts_host_names() {
        // Имена хостов НЕ запрещены: сегодня они работают (уходят в резолвер).
        assert_eq!(ip_ok("printer.local"), Some("printer.local".to_string()));
        assert_eq!(ip_ok("printer-hall"), Some("printer-hall".to_string()));
    }

    #[test]
    fn printer_ip_empty_and_blank_mean_none() {
        assert_eq!(ip_ok(""), None);
        assert_eq!(ip_ok("   "), None);
        assert_eq!(ip_ok("\t\n"), None);
    }

    #[test]
    fn printer_ip_rejects_port_and_ipv6() {
        for raw in ["192.0.2.10:161", "printer.local:9999", "::1", "2001:db8::1"] {
            let (field, message) = ip_err(raw);
            assert_eq!(field, "printer_ip_address", "{raw}");
            assert!(message.contains("Порт"), "{raw}: {message}");
        }
    }

    #[test]
    fn printer_ip_rejects_inner_whitespace_and_control() {
        for raw in ["192.0.2. 1", "printer a", "192.0.2.\t1", "192.0.2.\n1"] {
            let (field, message) = ip_err(raw);
            assert_eq!(field, "printer_ip_address", "{raw:?}");
            assert!(message.contains("пробел"), "{raw:?}: {message}");
        }
        let (field, message) = ip_err("192.0.2.\u{7}1");
        assert_eq!(field, "printer_ip_address");
        assert!(message.contains("пробел"));
    }
}
