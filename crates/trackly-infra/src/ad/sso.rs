//! Spike 001 — compile/link probe for server-side AD SSO (Kerberos/SPNEGO) via `sspi`.
//!
//! GOAL of this module: prove that the `sspi` crate (Devolutions) — including its
//! server-side Kerberos/Negotiate acceptance code and crypto backend — **compiles and
//! links for the portable Windows MSVC target** used by Trackly's release build, and
//! type-checks on the macOS dev box (`cargo check`). It is deliberately NOT the real
//! SSO endpoint — that is spike 002 (`negotiate-endpoint-h2off`).
//!
//! Why a probe is enough for the compile-proof: cargo builds the *entire* `sspi`
//! dependency crate for the target regardless of which items Trackly references, so the
//! server-side accept module (`sspi::kerberos::server`) and its crypto are compiled by
//! the mere presence of the dependency plus one real reference below. The single hardest
//! unknown this de-risks is whether that dependency (with `default-features = false`, i.e.
//! without the `aws-lc-rs` C-crypto TLS provider) builds on Windows MSVC in CI at all.
//!
//! The live Kerberos handshake against a real Domain Controller is validated separately
//! on Windows/AD hardware — it cannot be exercised from the dev macOS box (no domain
//! reachable), which is exactly why the whole feature is being spiked build-first.
//!
//! REAL server-accept shape (built out in spike 002), for reference:
//! - `sspi::KerberosServerConfig { kerberos_config, server_properties }`
//! - `Kerberos::new_server_from_config(cfg, props)` where
//!   `ServerProperties.ticket_decryption_key` / `.additional_service_keys` hold the
//!   keytab-derived service key — the ticket is decrypted **offline, no KDC round-trip**.
//! - accept loop: `acquire_credentials_handle().with_credential_use(Inbound)` then
//!   `accept_security_context().with_input(..).with_output(..).execute()` (per
//!   `sspi/examples/server.rs`).

use sspi::KerberosConfig;

/// Name reported by the probe — proves the module linked and ran.
pub const SSO_PROBE_TAG: &str = "trackly-ad-sso-spike-001";

/// Construct an `sspi` Kerberos *config* to type-check the crate's public surface that
/// spike 002 will build on. Constructing the config is side-effect-free — it only parses
/// the (here empty) KDC URL and stores the client computer name; no network, no KDC
/// contact. Returns whether a KDC URL was resolved from the input (empty here → `false`).
///
/// This function exists so `trackly-infra` genuinely *references* `sspi`, forcing the
/// dependency (server-accept code included) to be compiled and linked for the target.
pub fn sspi_link_probe(kdc_url: &str, client_computer_name: &str) -> bool {
    let cfg = KerberosConfig::new(kdc_url, client_computer_name.to_string());
    cfg.kdc_url.is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_tag_is_stable() {
        assert_eq!(SSO_PROBE_TAG, "trackly-ad-sso-spike-001");
    }

    #[test]
    fn empty_kdc_url_resolves_to_none() {
        // Empty input → no KDC URL parsed. This both type-checks `sspi`'s
        // `KerberosConfig` public API and forces the crate to link into the
        // test binary (the macOS half of the compile-proof).
        assert!(!sspi_link_probe("", "TRACKLY-SPIKE-HOST"));
    }

    #[test]
    fn explicit_kdc_url_resolves_to_some() {
        // A bare host:port is normalized to tcp://host:port by sspi's parser.
        assert!(sspi_link_probe("dc.example.test:88", "TRACKLY-SPIKE-HOST"));
    }
}
