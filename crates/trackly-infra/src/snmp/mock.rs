//! Mock SNMP client — deterministic fixtures for dev macOS (D-Mock-01).
//!
//! Used when `TRACKLY_SNMP_MOCK` env var is set or `config.snmp.use_mock = true`.
//! Returns preset toner levels, page counts, and status values keyed by IP address.
//!
//! 3 fixtures (per plan must_haves):
//!   192.168.1.100 — Pantum BM5100ADN, toner 45%, ok
//!   192.168.1.101 — HP LaserJet M403dn, toner 8%, warning
//!   192.168.1.102 — Canon iR2206, offline (for alert testing)

use std::collections::HashMap;

use async_trait::async_trait;
use trackly_core::error::AppError;
use trackly_core::ports::snmp::{OidValue, ProbedDevice, SnmpClient, SnmpValue};

/// Fixture for a single "printer" in the mock.
#[derive(Clone)]
pub struct PrinterFixture {
    pub toner_pct: u8,
    pub page_count: i64,
    /// "ok" | "warning" | "error" | "offline"
    pub status: &'static str,
    pub vendor: &'static str,
    pub model: &'static str,
    pub sys_object_id: &'static str,
}

/// Deterministic mock SNMP client for development (no real printers needed).
pub struct MockSnmpClient {
    pub fixtures: HashMap<String, PrinterFixture>,
}

impl MockSnmpClient {
    /// Create mock client with 3 pre-configured printer fixtures covering
    /// the main scenarios: ok, low-toner warning, offline alert.
    pub fn default_fixtures() -> Self {
        let mut map = HashMap::new();
        map.insert(
            "192.168.1.100".into(),
            PrinterFixture {
                toner_pct: 45,
                page_count: 12345,
                status: "ok",
                vendor: "Pantum",
                model: "BM5100ADN",
                sys_object_id: "1.3.6.1.4.1.40093.1",
            },
        );
        map.insert(
            "192.168.1.101".into(),
            PrinterFixture {
                toner_pct: 8,
                page_count: 54321,
                status: "warning",
                vendor: "HP",
                model: "LaserJet M403dn",
                sys_object_id: "1.3.6.1.4.1.11.2.3.9.1",
            },
        );
        // Simulate offline printer for alert testing (PRN-06).
        map.insert(
            "192.168.1.102".into(),
            PrinterFixture {
                toner_pct: 0,
                page_count: 0,
                status: "offline",
                vendor: "Canon",
                model: "iR2206",
                sys_object_id: "1.3.6.1.4.1.1602.1.1",
            },
        );
        Self { fixtures: map }
    }

    /// Extract IP from target (strips port if present).
    fn ip_from_target(target: &str) -> &str {
        target.split(':').next().unwrap_or(target)
    }
}

#[async_trait]
impl SnmpClient for MockSnmpClient {
    async fn get_oids(
        &self,
        target: &str,
        _community: &str,
        oids: &[&str],
        _timeout_secs: u64,
    ) -> Result<Option<Vec<OidValue>>, AppError> {
        let ip = Self::ip_from_target(target);
        let Some(fixture) = self.fixtures.get(ip) else {
            // Unknown IP → unreachable (return None like real client).
            return Ok(None);
        };

        // Offline printers respond to nothing.
        if fixture.status == "offline" {
            return Ok(None);
        }

        // Build deterministic varbind responses.
        let values = oids
            .iter()
            .map(|oid| {
                let value = mock_value_for_oid(oid, fixture);
                OidValue {
                    oid: (*oid).to_string(),
                    value,
                }
            })
            .collect();

        Ok(Some(values))
    }

    async fn probe(
        &self,
        target: &str,
        _community: &str,
    ) -> Result<Option<ProbedDevice>, AppError> {
        let ip = Self::ip_from_target(target);
        let Some(fixture) = self.fixtures.get(ip) else {
            return Ok(None);
        };

        if fixture.status == "offline" {
            return Ok(None);
        }

        Ok(Some(ProbedDevice {
            ip: ip.to_string(),
            sys_object_id: fixture.sys_object_id.to_string(),
            sys_descr: format!("{} {}", fixture.vendor, fixture.model),
            sys_name: format!("printer-{}", ip.replace('.', "-")),
        }))
    }
}

/// Return a deterministic SnmpValue for a given OID based on fixture data.
fn mock_value_for_oid(oid: &str, fixture: &PrinterFixture) -> SnmpValue {
    // hrPrinterStatus (status OID) → integer 3=idle (ok), 5=error
    if oid.starts_with("1.3.6.1.2.1.25.3.5") {
        let status_int: i64 = match fixture.status {
            "ok" => 3,      // idle
            "warning" => 3, // idle but low toner
            "error" => 5,   // error
            _ => 1,         // unknown
        };
        return SnmpValue::Integer(status_int);
    }
    // Page counter OIDs
    if oid.starts_with("1.3.6.1.2.1.43.10") || oid.starts_with("1.3.6.1.4.1.40093.10") {
        return SnmpValue::Integer(fixture.page_count);
    }
    // Toner level OIDs — return toner_pct as integer (Pantum = percent, others = level value)
    if oid.starts_with("1.3.6.1.2.1.43.11") || oid.starts_with("1.3.6.1.4.1.40093.6") {
        return SnmpValue::Integer(i64::from(fixture.toner_pct));
    }
    // sysDescr
    if oid == "1.3.6.1.2.1.1.1.0" {
        return SnmpValue::OctetString(
            format!("{} {}", fixture.vendor, fixture.model).into_bytes(),
        );
    }
    // sysObjectID
    if oid == "1.3.6.1.2.1.1.2.0" {
        return SnmpValue::OctetString(fixture.sys_object_id.as_bytes().to_vec());
    }
    // sysName
    if oid == "1.3.6.1.2.1.1.5.0" {
        return SnmpValue::OctetString(
            format!("printer-mock-{}", fixture.vendor.to_lowercase()).into_bytes(),
        );
    }
    SnmpValue::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn default_fixtures_get_oids_known_ip_returns_some() {
        let mock = MockSnmpClient::default_fixtures();
        let result = mock
            .get_oids(
                "192.168.1.100",
                "public",
                &["1.3.6.1.2.1.25.3.5.1.1.1", "1.3.6.1.4.1.40093.10.3.1.1"],
                2,
            )
            .await
            .expect("no error");
        assert!(result.is_some(), "known IP must return Some(varbinds)");
        let vals = result.unwrap();
        assert_eq!(vals.len(), 2, "should have 2 OID values");
    }

    #[tokio::test]
    async fn default_fixtures_offline_printer_returns_none() {
        let mock = MockSnmpClient::default_fixtures();
        let result = mock
            .get_oids("192.168.1.102", "public", &["1.3.6.1.2.1.25.3.5.1.1.1"], 2)
            .await
            .expect("no error");
        assert!(result.is_none(), "offline printer must return None");
    }

    #[tokio::test]
    async fn default_fixtures_unknown_ip_returns_none() {
        let mock = MockSnmpClient::default_fixtures();
        let result = mock
            .get_oids("10.0.0.99", "public", &["1.3.6.1.2.1.25.3.5.1.1.1"], 2)
            .await
            .expect("no error");
        assert!(result.is_none(), "unknown IP must return None");
    }

    #[tokio::test]
    async fn probe_returns_device_for_known_ip() {
        let mock = MockSnmpClient::default_fixtures();
        let result = mock
            .probe("192.168.1.100", "public")
            .await
            .expect("no error");
        assert!(result.is_some());
        let dev = result.unwrap();
        assert_eq!(dev.ip, "192.168.1.100");
        assert!(!dev.sys_object_id.is_empty());
    }
}
