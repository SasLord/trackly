//! Real AD client adapter using `ldap3::LdapConnAsync` (D-AD-01, D-Mock-01).
//!
//! CRITICAL: This module is the ONLY place in the codebase that imports `ldap3`.
//! `trackly-core::ports::ad::AdClient` trait must remain ldap3-free.
//!
//! Always wraps the connect call's outcome and the bind result-code into
//! `AuthOutcome` (never `Err`) — DC down / TLS failure / wrong creds are all
//! normal authentication outcomes, not infrastructure errors (mirrors
//! `RealSnmpClient`'s `Ok(None)`-for-unreachable philosophy).

use std::time::Duration;

use async_trait::async_trait;
use ldap3::{ldap_escape, LdapConnAsync, LdapConnSettings, Scope, SearchEntry};
use trackly_core::error::AppError;
use trackly_core::ports::ad::{AdClient, AuthOutcome};
use trackly_core::primitives::secret::Secret;

use crate::config::AdConfig;

/// Connection timeout for the initial LDAPS connect (Pitfall 7 — always
/// bound a timeout so a dead DC fails fast as `Unreachable`).
const CONN_TIMEOUT: Duration = Duration::from_secs(5);

/// Production AD client — binds via `ldap3` over LDAPS and resolves the
/// bound user's display name (D-Config-02: displayName → cn → login).
pub struct RealAdClient {
    cfg: AdConfig,
}

impl RealAdClient {
    pub fn new(cfg: AdConfig) -> Self {
        Self { cfg }
    }

    /// Normalize the bind name per Pitfall 6: AD `simple_bind` accepts
    /// `user@domain.tld` (UPN) or `DOMAIN\user`, but often rejects a bare
    /// short login. Pass through if the login already contains `@`/`\`;
    /// otherwise build `login@<domain>` from the configured domain.
    fn normalize_bind_name(&self, login: &str) -> String {
        if login.contains('@') || login.contains('\\') {
            login.to_string()
        } else {
            format!("{login}@{}", self.cfg.domain)
        }
    }
}

/// Build the user-search filter, escaping the login first (Pitfall 5 /
/// T-ldap-inj): LDAP filter metacharacters (`(`, `)`, `*`, `\`, NUL) in a
/// user-supplied login MUST be escaped before interpolation so a crafted
/// login cannot inject filter clauses. Extracted as a pure fn so the exact
/// production path is unit-testable without a live LDAPS connection.
fn build_user_search_filter(login: &str) -> String {
    let safe_login = ldap_escape(login);
    format!("(|(sAMAccountName={safe_login})(userPrincipalName={safe_login}))")
}

#[async_trait]
impl AdClient for RealAdClient {
    async fn authenticate(
        &self,
        login: &str,
        password: &Secret<String>,
    ) -> Result<AuthOutcome, AppError> {
        // CRITICAL (Pitfall 1 / T-09-01): reject empty/whitespace password
        // BEFORE attempting a bind — RFC 4513 §5.1.2 unauthenticated-bind trap.
        if password.expose().trim().is_empty() {
            return Ok(AuthOutcome::BadCreds);
        }

        let settings = LdapConnSettings::new()
            .set_conn_timeout(CONN_TIMEOUT)
            .set_no_tls_verify(self.cfg.no_tls_verify);
        let url = format!("ldaps://{}:{}", self.cfg.host, self.cfg.port);

        let (conn, mut ldap) = match LdapConnAsync::with_settings(settings, &url).await {
            Ok(v) => v,
            // Connect/TLS handshake failure → DC unreachable, not an error.
            Err(_) => return Ok(AuthOutcome::Unreachable),
        };
        // Pitfall 7: the connection driver task MUST be driven, or operations hang.
        ldap3::drive!(conn);

        let bind_name = self.normalize_bind_name(login);

        let bind_result = match ldap.simple_bind(&bind_name, password.expose()).await {
            Ok(res) => res,
            Err(_) => return Ok(AuthOutcome::Unreachable), // protocol/IO error mid-bind
        };

        if bind_result.success().is_err() {
            // rc != 0 (commonly rc=49 invalidCredentials) → generic BadCreds.
            // Do not leak account-state sub-codes to the caller (T-09-04).
            let _ = ldap.unbind().await;
            return Ok(AuthOutcome::BadCreds);
        }

        // Bound OK — search the user's own entry for the display name.
        // Pitfall 5: escape the login before interpolating into the filter
        // (see `build_user_search_filter`).
        let filter = build_user_search_filter(login);
        let attrs = vec![self.cfg.name_attr.as_str(), "cn"];

        let display_name = match ldap
            .search(&self.cfg.base_dn, Scope::Subtree, &filter, attrs)
            .await
            .and_then(|search_result| search_result.success())
        {
            Ok((entries, _res)) => entries
                .into_iter()
                .next()
                .map(SearchEntry::construct)
                .and_then(|entry| {
                    entry
                        .attrs
                        .get(&self.cfg.name_attr)
                        .and_then(|values| values.first().cloned())
                        .or_else(|| {
                            entry
                                .attrs
                                .get("cn")
                                .and_then(|values| values.first().cloned())
                        })
                })
                .unwrap_or_else(|| login.to_string()), // D-Config-02 fallback chain
            // Search failure after a successful bind is non-fatal — fall back to login.
            Err(_) => login.to_string(),
        };

        let _ = ldap.unbind().await;
        Ok(AuthOutcome::Ok { display_name })
    }

    /// Reachability probe — connects over LDAPS and issues an anonymous
    /// bind to confirm the server actually speaks LDAP (not just a TCP
    /// listener). No end-user credentials are involved.
    async fn test_connection(&self) -> Result<AuthOutcome, AppError> {
        let settings = LdapConnSettings::new()
            .set_conn_timeout(CONN_TIMEOUT)
            .set_no_tls_verify(self.cfg.no_tls_verify);
        let url = format!("ldaps://{}:{}", self.cfg.host, self.cfg.port);

        let (conn, mut ldap) = match LdapConnAsync::with_settings(settings, &url).await {
            Ok(v) => v,
            Err(_) => return Ok(AuthOutcome::Unreachable),
        };
        // Pitfall 7: the connection driver task MUST be driven, or operations hang.
        ldap3::drive!(conn);

        // Anonymous (unauthenticated) bind — RFC 4513 §5.1.2 allows this
        // explicitly when no credentials are presented (empty DN + empty
        // password), distinct from the anonymous-bind TRAP this module
        // guards against in `authenticate` (non-empty DN + empty password).
        let bind_result = match ldap.simple_bind("", "").await {
            Ok(res) => res,
            Err(_) => return Ok(AuthOutcome::Unreachable),
        };

        let _ = ldap.unbind().await;

        if bind_result.success().is_err() {
            // Server responded but rejected the anonymous bind (e.g. AD
            // configured to refuse unauthenticated binds) — the server IS
            // reachable, it just won't anonymous-bind. Treat as reachable.
            return Ok(AuthOutcome::Ok {
                display_name: String::new(),
            });
        }

        Ok(AuthOutcome::Ok {
            display_name: String::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Pitfall 5 / T-ldap-inj: the login is escaped before it is interpolated
    // into the LDAP search filter. These tests exercise the SAME function the
    // production `authenticate` path calls, so no live LDAPS connection is
    // needed to prove the injection guard holds.

    #[test]
    fn benign_login_builds_expected_filter() {
        // A plain login has no metacharacters, so it passes through unchanged
        // and appears in both the sAMAccountName and userPrincipalName branches.
        assert_eq!(
            build_user_search_filter("us100"),
            "(|(sAMAccountName=us100)(userPrincipalName=us100))"
        );
    }

    #[test]
    fn injection_payload_metacharacters_are_escaped() {
        // Classic LDAP filter-injection payload: without escaping, the raw
        // `)(` / `*` would close our clause and inject an attacker-controlled
        // filter. ldap_escape encodes `(`→\28, `)`→\29, `*`→\2a (RFC 4515).
        let payload = "*)(uid=*))(|(uid=*";
        let filter = build_user_search_filter(payload);

        // No raw filter metacharacter from the payload survives — the only `(`,
        // `)`, `*` left in the string are our own structural ones. (The raw
        // payload had `*`; after escaping there must be zero literal `*`.)
        assert!(
            !filter.contains('*'),
            "no raw `*` may survive escaping: {filter}"
        );
        assert!(
            !filter.contains(payload),
            "raw payload must not appear verbatim: {filter}"
        );
        // Escaped forms are present instead (RFC 4515: `*`→\2a, `(`→\28, `)`→\29).
        assert!(
            filter.contains("\\2a"),
            "`*` must be escaped to \\2a: {filter}"
        );
        assert!(
            filter.contains("\\28"),
            "`(` must be escaped to \\28: {filter}"
        );
        assert!(
            filter.contains("\\29"),
            "`)` must be escaped to \\29: {filter}"
        );

        // The filter still has exactly our two intended attribute clauses and
        // the single leading OR — the payload could not add another `(|`.
        assert!(filter.starts_with("(|(sAMAccountName="));
        assert_eq!(
            filter.matches("(|").count(),
            1,
            "no injected OR clause: {filter}"
        );
    }

    #[test]
    fn backslash_in_login_is_escaped() {
        // A backslash (e.g. DOMAIN\user typed into the search position) must be
        // escaped to \5c so it cannot start an escape sequence of its own.
        let filter = build_user_search_filter("dom\\ain");
        assert!(
            !filter.contains("dom\\ain"),
            "raw backslash must not survive: {filter}"
        );
        assert!(
            filter.contains("\\5c"),
            "`\\` must be escaped to \\5c: {filter}"
        );
    }
}
