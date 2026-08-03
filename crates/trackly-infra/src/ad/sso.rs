//! Spike 002 — server-side SPNEGO/Negotiate (Kerberos) acceptor via `sspi`.
//!
//! Validates a browser's `Authorization: Negotiate <base64>` token against the service's
//! keytab-derived key and, on success, returns the authenticated AD account name. This is
//! the Rust counterpart of adwebapp's gokrb5 `/auth/ad` acceptor.
//!
//! ⚠️ BUILD-VERIFIED, NOT LIVE-VERIFIED. This code compiles and links (spike 001 proved the
//! whole `sspi` server path builds for Windows MSVC), but the actual Kerberos handshake can
//! only be exercised against a real Domain Controller — done on Windows/AD hardware, not the
//! dev macOS box. The two spots whose *runtime* behavior is unproven until that test:
//!   1. `acquire_credentials_handle` for a Negotiate *server* (the service key is supplied via
//!      `ServerProperties`, so no `with_auth_data` — needs live confirmation).
//!   2. `resolve_with_client(&mut net)` — the accept API takes a `NetworkClient`. For an
//!      offline service-ticket decrypt (keytab key present, no U2U) it should never touch the
//!      network; `OfflineNetworkClient` errors loudly if it ever is called, which tomorrow's
//!      test will surface. If it turns out the network IS needed, we enable sspi's
//!      `network_client` feature or point it at the KDC.
//!
//! The keytab reading is in `super::keytab` and IS fully unit-tested (deterministic parsing).

use std::time::Duration;

use sspi::kerberos::ServerProperties;
use sspi::network_client::NetworkClient;
use sspi::{
    BufferType, CredentialUse, DataRepresentation, KerberosConfig, KerberosServerConfig, Negotiate,
    NegotiateConfig, NetworkRequest, SecurityBuffer, SecurityStatus, ServerRequestFlags, Sspi,
    SspiImpl,
};

/// How far the acceptor tolerates clock skew between client and this server when validating
/// the ticket's timestamps. Kerberos' customary allowance is 5 minutes.
const MAX_TIME_SKEW: Duration = Duration::from_secs(5 * 60);

/// Outcome of one SPNEGO accept step.
#[derive(Debug)]
pub enum SsoOutcome {
    /// Handshake complete — `username` is the authenticated AD account (SAM/UPN as sspi
    /// reports it). `reply_token` (possibly empty) must be returned to the client in the
    /// `WWW-Authenticate: Negotiate <base64>` header.
    Authenticated {
        username: String,
        reply_token: Vec<u8>,
    },
    /// Handshake needs another round trip — send `reply_token` back with a 401 and expect a
    /// follow-up token. (Kerberos usually completes in one step; NTLM-style continuation is
    /// where this occurs.)
    Continue { reply_token: Vec<u8> },
    /// Token rejected (bad/foreign ticket, wrong SPN, expired). Generic on purpose.
    Denied,
}

/// Local error type — keeps `sspi` out of the trackly-core port surface. Never carries key
/// material.
#[derive(Debug, thiserror::Error)]
pub enum SsoError {
    #[error("SSO server not configured (keytab/SPN missing)")]
    NotConfigured,
    #[error("keytab: {0}")]
    Keytab(#[from] super::keytab::KeytabError),
    #[error("sspi negotiate accept failed: {0}")]
    Sspi(String),
}

/// A `NetworkClient` that refuses to do any network I/O. For an offline service-ticket
/// decrypt (keytab key present) the acceptor must not contact the KDC; if it ever tries,
/// this returns an error rather than silently reaching out, and the failure tells us the
/// offline assumption was wrong (a spike-002 finding to resolve before shipping).
struct OfflineNetworkClient;

impl NetworkClient for OfflineNetworkClient {
    fn send(&self, _request: &NetworkRequest) -> sspi::Result<Vec<u8>> {
        Err(sspi::Error::new(
            sspi::ErrorKind::NoAuthenticatingAuthority,
            "AD SSO acceptor attempted a network/KDC call, but only offline keytab \
             validation is configured (spike-002: offline assumption violated)",
        ))
    }
}

/// Accept one SPNEGO/Negotiate token.
///
/// * `spn` — the service principal in `SERVICE/host` form, e.g. `HTTP/web.example.local`
///   (realm-agnostic; must match the SPN the keytab was generated for and the name the
///   browser used in the address bar).
/// * `keytab_bytes` — raw contents of the `.keytab` file (`ktpass … /crypto AES256-SHA1`).
/// * `client_computer_name` — this server's own machine/workstation name (diagnostic only).
/// * `input_token` — the raw (already base64-decoded) bytes from `Authorization: Negotiate`.
pub fn accept_spnego(
    spn: &str,
    keytab_bytes: &[u8],
    client_computer_name: &str,
    input_token: &[u8],
) -> Result<SsoOutcome, SsoError> {
    if spn.is_empty() || keytab_bytes.is_empty() {
        return Err(SsoError::NotConfigured);
    }

    // Pull the AES256 service key for this SPN out of the keytab (unit-tested parser).
    let service_key = super::keytab::aes256_key_for_spn(keytab_bytes, spn)?;

    // SPN "HTTP/web.example.local" → sname components ["HTTP", "web.example.local"].
    let sname: Vec<&str> = spn.split('/').collect();

    // Server config: Kerberos server keyed by the keytab-derived long-term key. No KDC URL —
    // the ticket is decrypted locally (offline), matching adwebapp's model.
    let kerberos_config = KerberosConfig::new("", client_computer_name.to_string());
    let server_properties = ServerProperties::new(
        &sname,
        None, // no bound user credentials — pure acceptor
        MAX_TIME_SKEW,
        Some(service_key.into()),
    )
    .map_err(|e| SsoError::Sspi(e.to_string()))?;
    let server_config = KerberosServerConfig {
        kerberos_config,
        server_properties,
    };

    let negotiate_config = NegotiateConfig::from_protocol_config(
        Box::new(server_config),
        client_computer_name.to_string(),
    );
    let mut server = Negotiate::new_server(negotiate_config, Vec::new())
        .map_err(|e| SsoError::Sspi(e.to_string()))?;

    // Inbound credentials handle (server side). The service key lives in ServerProperties, so
    // no `with_auth_data` here — see the module-level runtime-unknown note (1).
    let mut acq = server
        .acquire_credentials_handle()
        .with_credential_use(CredentialUse::Inbound)
        .execute(&mut server)
        .map_err(|e| SsoError::Sspi(e.to_string()))?;

    let mut input = [SecurityBuffer::new(input_token.to_vec(), BufferType::Token)];
    let mut output = vec![SecurityBuffer::new(
        Vec::with_capacity(1024),
        BufferType::Token,
    )];

    let builder = server
        .accept_security_context()
        .with_credentials_handle(&mut acq.credentials_handle)
        .with_context_requirements(ServerRequestFlags::ALLOCATE_MEMORY)
        .with_target_data_representation(DataRepresentation::Native)
        .with_input(&mut input)
        .with_output(&mut output);

    let net = OfflineNetworkClient;
    let result = server
        .accept_security_context_impl(builder)
        .map_err(|e| SsoError::Sspi(e.to_string()))?
        .resolve_with_client(&net)
        .map_err(|e| SsoError::Sspi(e.to_string()))?;

    let reply_token = output.remove(0).buffer;

    match result.status {
        SecurityStatus::Ok
        | SecurityStatus::CompleteNeeded
        | SecurityStatus::CompleteAndContinue => {
            // Authenticated — read the client's AD account name off the established context.
            let username = server
                .query_context_names()
                .map_err(|e| SsoError::Sspi(e.to_string()))?
                .username
                .inner()
                .to_string();
            Ok(SsoOutcome::Authenticated {
                username,
                reply_token,
            })
        }
        SecurityStatus::ContinueNeeded => Ok(SsoOutcome::Continue { reply_token }),
        _ => Ok(SsoOutcome::Denied),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_config_is_not_configured() {
        // Guards the fast-path rejection without needing any sspi machinery — the live
        // handshake itself is real-AD-only (see module note).
        let out = accept_spnego("", b"", "HOST", b"\x01\x02");
        assert!(matches!(out, Err(SsoError::NotConfigured)));
        let out2 = accept_spnego("HTTP/web.example.local", b"", "HOST", b"\x01");
        assert!(matches!(out2, Err(SsoError::NotConfigured)));
    }

    #[test]
    fn bad_keytab_surfaces_keytab_error() {
        // A non-empty but invalid keytab must produce a typed Keytab error, not a panic.
        let out = accept_spnego("HTTP/web.example.local", b"not-a-keytab", "HOST", b"\x01");
        assert!(matches!(out, Err(SsoError::Keytab(_))));
    }
}
