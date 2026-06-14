//! Phase 6 Wave-0 stub tests — trackly-infra уровень.
//!
//! PRN-08: MockSnmpClient::default_fixtures() — детерминированные OidValue.
//! Реализован в Wave 1 (06-01-PLAN.md).

use trackly_infra::snmp::mock::MockSnmpClient;
use trackly_core::ports::snmp::SnmpClient;

/// PRN-08: MockSnmpClient::default_fixtures() get_oids → детерминированные OidValue
#[tokio::test]
async fn test_mock_snmp() {
    let mock = MockSnmpClient::default_fixtures();

    // Fixture 1: Pantum (192.168.1.100) — should return OID values (status ok)
    let result = mock
        .get_oids(
            "192.168.1.100",
            "public",
            &[
                "1.3.6.1.2.1.25.3.5.1.1.1",   // hrPrinterStatus
                "1.3.6.1.4.1.40093.10.3.1.1",  // Pantum page counter
            ],
            2,
        )
        .await
        .expect("MockSnmpClient::get_oids must not error");

    assert!(result.is_some(), "known IP (Pantum) must return Some(OidValues)");
    let vals = result.unwrap();
    assert_eq!(vals.len(), 2, "expected 2 OidValue entries");
    // OID strings must be populated.
    for v in &vals {
        assert!(!v.oid.is_empty(), "OidValue.oid must not be empty");
    }

    // Fixture 2: HP (192.168.1.101) — warning status, should still respond.
    let result2 = mock
        .get_oids(
            "192.168.1.101",
            "public",
            &["1.3.6.1.2.1.25.3.5.1.1.1"],
            2,
        )
        .await
        .expect("no error");
    assert!(result2.is_some(), "HP fixture must return Some");

    // Fixture 3: Canon (192.168.1.102) — offline, must return None (timeout simulation).
    let result3 = mock
        .get_oids(
            "192.168.1.102",
            "public",
            &["1.3.6.1.2.1.25.3.5.1.1.1"],
            2,
        )
        .await
        .expect("no error");
    assert!(result3.is_none(), "offline printer must return None (simulates timeout)");
}
