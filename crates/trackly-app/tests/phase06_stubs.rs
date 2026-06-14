//! Phase 6 Wave-0 stub tests — trackly-app уровень.
//!
//! Тесты, помеченные #[ignore], будут реализованы в соответствующих волнах (Wave 2-5).
//! test_oid_profiles_seeded реализован в Wave 1 (06-01-PLAN.md).

use trackly_infra::test_support::test_db::test_db;

/// PRN-01: parse sysObjectID → vendor + oid_profile match
#[test]
#[ignore]
fn test_vendor_identify() {}

/// PRN-02: parse_toner_level(45, 100, "level_over_max") = Some(45); (-2,-2) = None
#[test]
#[ignore]
fn test_toner_percent() {}

/// PRN-03: SELECT COUNT(*) FROM oid_profiles = 5
#[test]
fn test_oid_profiles_seeded() {
    let (conn, _guard) = test_db();

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM oid_profiles", [], |r| r.get(0))
        .expect("query oid_profiles count");
    assert_eq!(count, 5, "expected 5 OID profiles: pantum/kyocera/hp/canon/rfc3805");

    // Verify Pantum uses 'percent' encoding (special case — toner value is already %).
    let pantum_encoding: String = conn
        .query_row(
            "SELECT toner_encoding FROM oid_profiles WHERE name = 'pantum'",
            [],
            |r| r.get(0),
        )
        .expect("query pantum profile");
    assert_eq!(pantum_encoding, "percent", "pantum must use 'percent' toner_encoding");

    // Verify RFC3805 fallback exists.
    let rfc3805_exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM oid_profiles WHERE name = 'rfc3805'",
            [],
            |r| r.get(0),
        )
        .expect("check rfc3805");
    assert_eq!(rfc3805_exists, 1, "rfc3805 fallback profile must exist");
}

/// PRN-04: printer with usb_host_device_id set + NULL ip_address — get возвращает usb_host_device_id
#[test]
#[ignore]
fn test_printer_usb_only() {}

/// PRN-06: status='error'|'offline' → upsert printer_alerts (dedup — второй upsert не дублирует)
#[test]
#[ignore]
fn test_alert_detection() {}

/// REQ-01: RequestService::create записывает строку в requests + audit_log
#[test]
#[ignore]
fn test_request_create() {}

/// REQ-03: Accept/Reject/Complete переходы; недопустимый → AppError::Validation
#[test]
#[ignore]
fn test_request_lifecycle() {}

/// REQ-04: broadcast::Sender получает WsEvent::NewRequest после create
#[test]
#[ignore]
fn test_ws_event_sent() {}

/// REQ-05: Complete{linked_cartridge_id} записывает completed_cartridge_id + статус→completed
#[test]
#[ignore]
fn test_req_cart_link() {}

/// D-Mock-01: AppCtx с TRACKLY_SNMP_MOCK=1 → MockSnmpClient (switch в AppCtx::build)
#[test]
#[ignore]
fn test_snmp_mock_switch() {}

/// D-Retention-01: prune_old_readings_in_tx удаляет строки старше retention_cutoff
#[test]
#[ignore]
fn test_readings_prune() {}

/// PRN-07: install картриджа с printer_device_id → current_cartridge_for_printer(printer_id) = Some(cartridge_id)
#[test]
#[ignore]
fn test_current_cartridge_for_printer() {}

/// ASVS V4: HTTP GET /api/v1/ws без session cookie → 401 (до upgrade)
#[test]
#[ignore]
fn test_ws_unauth_401() {}

/// CLAUDE.md / T-06-07-I: Secret<T> Debug не утекает значение (prints "***")
#[test]
#[ignore]
fn test_secret_debug() {}
