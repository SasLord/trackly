//! AD connection auto-detect (D-Config-01).
//!
//! Resolution order (each step falls back to the next):
//! 1. Manual override (`AdConfig.host`/`base_dn` filled in «Расширенные»).
//! 2. DNS SRV (`_ldap._tcp.dc._msdcs.<domain>`) via `hickory-resolver`.
//! 3. Environment (`USERDNSDOMAIN`, `LOGONSERVER`) on a domain-joined Windows host.
//! 4. Last resort: typed "no domain detected" result — NOT a panic. This is
//!    the dev-macOS path: `USERDNSDOMAIN` is unset and SRV lookup fails, so
//!    discovery cleanly reports "not a domain member" and the caller relies
//!    on `TRACKLY_AD_MOCK=1` instead.

use hickory_resolver::proto::rr::RData;
use hickory_resolver::Resolver;

/// Result of an auto-detect attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiscoveryResult {
    /// A domain controller hostname was found (via DNS SRV).
    Found { dc_host: String },
    /// No domain could be detected — dev-macOS / non-domain-joined host.
    /// This is a normal, expected outcome, not an error.
    NoDomainDetected,
}

/// Derive the LDAP base DN from a DNS domain name.
///
/// `corp.local` -> `dc=corp,dc=local`. Pure string transform, no I/O.
pub fn derive_base_dn(domain: &str) -> String {
    domain
        .split('.')
        .filter(|label| !label.is_empty())
        .map(|label| format!("dc={label}"))
        .collect::<Vec<_>>()
        .join(",")
}

/// Read the domain name from the environment (`USERDNSDOMAIN`, set on
/// domain-joined Windows hosts). Returns `None` on dev macOS / Linux / a
/// non-domain-joined Windows host — this is the expected dev-macOS path.
pub fn domain_from_env() -> Option<String> {
    std::env::var("USERDNSDOMAIN")
        .ok()
        .filter(|v| !v.trim().is_empty())
}

/// Auto-detect a domain controller via DNS SRV lookup
/// (`_ldap._tcp.dc._msdcs.<domain>`), falling back to a typed
/// "no domain detected" result on any failure (never panics).
///
/// `domain` is typically sourced from `domain_from_env()` first; callers
/// that already have a manual override should skip this entirely (step 1
/// of the resolution order lives in the caller, not here).
pub async fn discover_dc(domain: &str) -> DiscoveryResult {
    if domain.trim().is_empty() {
        return DiscoveryResult::NoDomainDetected;
    }

    let resolver = match Resolver::builder_tokio().and_then(|builder| builder.build()) {
        Ok(r) => r,
        Err(_) => return DiscoveryResult::NoDomainDetected,
    };

    let query = format!("_ldap._tcp.dc._msdcs.{domain}");
    let lookup = match resolver.srv_lookup(query).await {
        Ok(lookup) => lookup,
        Err(_) => return DiscoveryResult::NoDomainDetected,
    };

    let dc_host = lookup
        .answers()
        .iter()
        .filter_map(|record| match &record.data {
            RData::SRV(srv) => Some(srv),
            _ => None,
        })
        .min_by_key(|srv| srv.priority)
        .map(|srv| srv.target.to_string().trim_end_matches('.').to_string());

    match dc_host {
        Some(host) if !host.is_empty() => DiscoveryResult::Found { dc_host: host },
        _ => DiscoveryResult::NoDomainDetected,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derive_base_dn_single_label() {
        assert_eq!(derive_base_dn("local"), "dc=local");
    }

    #[test]
    fn derive_base_dn_multi_label() {
        assert_eq!(derive_base_dn("corp.local"), "dc=corp,dc=local");
    }

    #[test]
    fn derive_base_dn_three_labels() {
        assert_eq!(
            derive_base_dn("ad.corp.example.com"),
            "dc=ad,dc=corp,dc=example,dc=com"
        );
    }

    #[test]
    fn derive_base_dn_empty_domain_yields_empty_string() {
        assert_eq!(derive_base_dn(""), "");
    }

    #[tokio::test]
    async fn no_domain_returns_typed_result() {
        // Dev macOS never has a real AD domain reachable — discovery must
        // return a typed result, not panic, for an empty/bogus domain.
        let result = discover_dc("").await;
        assert_eq!(result, DiscoveryResult::NoDomainDetected);
    }

    #[tokio::test]
    async fn nonexistent_domain_returns_typed_result_not_panic() {
        let result = discover_dc("this-domain-does-not-exist.invalid").await;
        assert_eq!(result, DiscoveryResult::NoDomainDetected);
    }
}
