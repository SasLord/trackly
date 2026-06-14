//! Real SNMP client adapter using `snmp2::AsyncSession` (D-Mock-01).
//!
//! CRITICAL: This module is the ONLY place in the codebase that imports `snmp2`.
//! `trackly-core::ports::snmp::SnmpClient` trait must remain snmp2-free.
//!
//! Always wraps SNMP calls in `tokio::time::timeout` — `AsyncSession` has no
//! built-in timeout (Pitfall 1 from RESEARCH.md). Timeout/error → `Ok(None)`
//! (unreachable printer is normal, not an error for the caller).

use std::str::FromStr;
use std::time::Duration;

use async_trait::async_trait;
use snmp2::AsyncSession;
use tokio::time::timeout;
use trackly_core::error::AppError;
use trackly_core::ports::snmp::{OidValue, ProbedDevice, SnmpClient, SnmpValue};

/// Default probe timeout in seconds when none is specified.
const PROBE_TIMEOUT_SECS: u64 = 2;

/// OIDs used for discovery probe (sysObjectID, sysDescr, sysName).
const SYS_OBJECT_ID: &str = "1.3.6.1.2.1.1.2.0";
const SYS_DESCR: &str = "1.3.6.1.2.1.1.1.0";
const SYS_NAME: &str = "1.3.6.1.2.1.1.5.0";

/// Production SNMP client — uses `snmp2::AsyncSession` over UDP.
pub struct RealSnmpClient;

#[async_trait]
impl SnmpClient for RealSnmpClient {
    async fn get_oids(
        &self,
        target: &str,
        community: &str,
        oids: &[&str],
        timeout_secs: u64,
    ) -> Result<Option<Vec<OidValue>>, AppError> {
        // Append SNMP UDP port if not already present.
        let addr = if target.contains(':') {
            target.to_string()
        } else {
            format!("{target}:161")
        };

        // Parse all OIDs up front — invalid OID string is a programming error, not runtime error.
        let parsed: Vec<snmp2::Oid<'static>> = oids
            .iter()
            .filter_map(|s| snmp2::Oid::from_str(s).ok())
            .collect();

        if parsed.is_empty() {
            return Ok(Some(vec![]));
        }

        // Open v2c session — DNS resolution / bind failure → Ok(None).
        let mut sess = match timeout(
            Duration::from_secs(timeout_secs),
            AsyncSession::new_v2c(&addr, community.as_bytes(), 0),
        )
        .await
        {
            Ok(Ok(s)) => s,
            _ => return Ok(None),
        };

        // Build refs for get_many.
        let oid_refs: Vec<&snmp2::Oid<'_>> = parsed.iter().collect();

        // ALWAYS wrap in timeout — snmp2 has no built-in timeout (Pitfall 1).
        let pdu = match timeout(
            Duration::from_secs(timeout_secs),
            sess.get_many(&oid_refs),
        )
        .await
        {
            Ok(Ok(p)) => p,
            _ => return Ok(None), // timeout or SNMP error = unreachable
        };

        let values = pdu
            .varbinds
            .map(|(oid, val)| OidValue {
                oid: oid.to_id_string(),
                value: snmp_value_to_domain(val),
            })
            .collect();

        Ok(Some(values))
    }

    async fn probe(&self, target: &str, community: &str) -> Result<Option<ProbedDevice>, AppError> {
        let probe_oids = [SYS_OBJECT_ID, SYS_DESCR, SYS_NAME];
        let result = self
            .get_oids(target, community, &probe_oids, PROBE_TIMEOUT_SECS)
            .await?;

        let Some(oids) = result else {
            return Ok(None);
        };

        // Extract values by position (order matches probe_oids order).
        let sys_object_id = extract_string_value(&oids, 0);
        let sys_descr = extract_string_value(&oids, 1);
        let sys_name = extract_string_value(&oids, 2);

        Ok(Some(ProbedDevice {
            ip: target.split(':').next().unwrap_or(target).to_string(),
            sys_object_id,
            sys_descr,
            sys_name,
        }))
    }
}

/// Convert snmp2 `Value<'_>` to domain `SnmpValue`.
fn snmp_value_to_domain(val: snmp2::Value<'_>) -> SnmpValue {
    match val {
        snmp2::Value::Integer(i) => SnmpValue::Integer(i),
        snmp2::Value::OctetString(b) => SnmpValue::OctetString(b.to_vec()),
        snmp2::Value::Unsigned32(u) | snmp2::Value::Counter32(u) => SnmpValue::Gauge(u64::from(u)),
        snmp2::Value::Counter64(u) => SnmpValue::Counter(u),
        _ => SnmpValue::Unknown,
    }
}

/// Extract a string value from varbind list by position index.
/// Returns empty string if the index is out of range or value is not a string.
fn extract_string_value(oids: &[OidValue], idx: usize) -> String {
    oids.get(idx)
        .and_then(|v| match &v.value {
            SnmpValue::OctetString(b) => Some(String::from_utf8_lossy(b).into_owned()),
            SnmpValue::Integer(i) => Some(i.to_string()),
            _ => None,
        })
        .unwrap_or_default()
}
