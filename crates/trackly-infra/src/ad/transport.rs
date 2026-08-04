//! Shared LDAP connection URL/settings construction (quick task 260804-ire).
//!
//! `RealAdClient::authenticate`, `RealAdClient::test_connection`, and
//! `RealAdDirectory::resolve` all need to build the same
//! `(url, LdapConnSettings)` pair from an `AdConfig`. This module is the
//! SINGLE place that does that — no other call site should construct a
//! `format!("ldaps://...")`/`format!("ldap://...")` literal directly.

use std::time::Duration;

use ldap3::LdapConnSettings;

use crate::config::{AdConfig, LdapTlsMode};

/// Connection timeout for the initial LDAP(S) connect (Pitfall 7 — always
/// bound a timeout so a dead DC fails fast as `Unreachable`).
pub(crate) const CONN_TIMEOUT: Duration = Duration::from_secs(5);

/// Builds the LDAP connection URL and `LdapConnSettings` for the given
/// config, per `cfg.ldap_tls_mode`:
/// - `Ldaps` → `ldaps://host:port`, settings unchanged (matches today's
///   behavior exactly — no StartTLS).
/// - `Plain` → `ldap://host:port`, settings unchanged (no StartTLS).
/// - `StartTls` → `ldap://host:port`, settings gains `.set_starttls(true)`.
///
/// `port` resolves via `AdConfig::resolved_port()` — an explicit `port`
/// always wins over the mode-derived default.
pub(crate) fn build_ldap_conn(cfg: &AdConfig) -> (String, LdapConnSettings) {
    let port = cfg.resolved_port();
    let mut settings = LdapConnSettings::new()
        .set_conn_timeout(CONN_TIMEOUT)
        // `no_tls_verify` has no public getter on `LdapConnSettings`, so it
        // is not independently assertable in the unit tests below — the
        // builder call itself is the only observable proof at this layer.
        .set_no_tls_verify(cfg.no_tls_verify);

    let url = match cfg.ldap_tls_mode {
        LdapTlsMode::Ldaps => format!("ldaps://{}:{}", cfg.host, port),
        LdapTlsMode::Plain => format!("ldap://{}:{}", cfg.host, port),
        LdapTlsMode::StartTls => {
            settings = settings.set_starttls(true);
            format!("ldap://{}:{}", cfg.host, port)
        }
    };

    (url, settings)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_cfg() -> AdConfig {
        AdConfig {
            host: "dc1.example.local".to_string(),
            ..AdConfig::default()
        }
    }

    #[test]
    fn ldaps_mode_builds_ldaps_url_no_starttls() {
        let cfg = AdConfig {
            ldap_tls_mode: LdapTlsMode::Ldaps,
            ..base_cfg()
        };
        let (url, settings) = build_ldap_conn(&cfg);
        assert_eq!(url, "ldaps://dc1.example.local:636");
        assert!(!settings.starttls());
    }

    #[test]
    fn plain_mode_builds_plain_url_no_starttls() {
        let cfg = AdConfig {
            ldap_tls_mode: LdapTlsMode::Plain,
            ..base_cfg()
        };
        let (url, settings) = build_ldap_conn(&cfg);
        assert_eq!(url, "ldap://dc1.example.local:389");
        assert!(!settings.starttls());
    }

    #[test]
    fn starttls_mode_builds_plain_url_with_starttls() {
        let cfg = AdConfig {
            ldap_tls_mode: LdapTlsMode::StartTls,
            ..base_cfg()
        };
        let (url, settings) = build_ldap_conn(&cfg);
        assert_eq!(url, "ldap://dc1.example.local:389");
        assert!(settings.starttls());
    }

    #[test]
    fn explicit_port_overrides_mode_default_on_any_mode() {
        let cfg = AdConfig {
            ldap_tls_mode: LdapTlsMode::Plain,
            port: Some(2389),
            ..base_cfg()
        };
        let (url, _settings) = build_ldap_conn(&cfg);
        assert_eq!(url, "ldap://dc1.example.local:2389");
    }
}
