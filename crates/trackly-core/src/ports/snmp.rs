//! `SnmpClient` port — abstraction for SNMP I/O.
//!
//! Pattern: like `Clock`, this trait lives in trackly-core but has NO tokio/snmp2
//! imports — I/O-free invariant enforced by `tests/no_io_deps.rs`.
//! The real impl (`RealSnmpClient`) lives in `trackly_infra::snmp::real`.
//! The mock impl (`MockSnmpClient`) lives in `trackly_infra::snmp::mock`.
//!
//! Runtime switching via `AppCtx::build` checks TRACKLY_SNMP_MOCK env var
//! or `config.snmp.use_mock` (D-Mock-01).

use async_trait::async_trait;

use crate::error::AppError;

/// A single OID value returned from an SNMP GET response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OidValue {
    /// The OID in dotted decimal notation.
    pub oid: String,
    pub value: SnmpValue,
}

/// Parsed SNMP varbind value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnmpValue {
    Integer(i64),
    OctetString(Vec<u8>),
    Gauge(u64),
    Counter(u64),
    /// Any other value type (OID, timeticks, IP, etc.) — raw bytes.
    Unknown,
}

/// Basic device info from SNMP discovery probe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbedDevice {
    pub ip: String,
    pub sys_object_id: String,
    pub sys_descr: String,
    pub sys_name: String,
}

/// SNMP client port — implemented by `RealSnmpClient` and `MockSnmpClient`.
///
/// CRITICAL: This trait MUST NOT import tokio or snmp2 — those are infra-layer deps.
/// `async_trait` is the only allowed external dependency here (pure-data crate).
#[async_trait]
pub trait SnmpClient: Send + Sync {
    /// Fetch OID values from a target via SNMP GET.
    ///
    /// Returns `None` if the target is unreachable or times out (not an error —
    /// unreachable printers are normal in LAN polling).
    ///
    /// # Arguments
    /// * `target` - IP address (without port; SNMP uses UDP/161 by default)
    /// * `community` - SNMP community string (e.g. "public")
    /// * `oids` - List of OIDs in dotted decimal notation
    /// * `timeout_secs` - Timeout in seconds (ALWAYS enforced — Pitfall 1 from RESEARCH.md)
    async fn get_oids(
        &self,
        target: &str,
        community: &str,
        oids: &[&str],
        timeout_secs: u64,
    ) -> Result<Option<Vec<OidValue>>, AppError>;

    /// Discovery probe: fetch sysObjectID + sysDescr + sysName.
    ///
    /// Returns `None` if unreachable/timeout. Used during subnet scan (D-Discovery-01).
    async fn probe(
        &self,
        target: &str,
        community: &str,
    ) -> Result<Option<ProbedDevice>, AppError>;
}
