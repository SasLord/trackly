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
        // Pitfall 5: escape the login before interpolating into the filter.
        let safe_login = ldap_escape(login);
        let filter = format!("(|(sAMAccountName={safe_login})(userPrincipalName={safe_login}))");
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
