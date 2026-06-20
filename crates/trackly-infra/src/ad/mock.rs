//! Mock AD client — deterministic fixtures for dev macOS (D-Mock-01, USR-12).
//!
//! Used when `TRACKLY_AD_MOCK` env var is set or `config.ad.use_mock = true`.
//! Returns preset bind outcomes keyed by AD login (sAMAccountName-style),
//! mirroring `MockSnmpClient::default_fixtures` (`crates/trackly-infra/src/snmp/mock.rs`).
//!
//! 2 fixtures (per plan must_haves):
//!   us100 / Passw0rd! — Иванов Иван Иванович
//!   us200 / Secret123 — Петрова Анна Сергеевна

use std::collections::HashMap;

use async_trait::async_trait;
use trackly_core::error::AppError;
use trackly_core::ports::ad::{AdClient, AuthOutcome};
use trackly_core::primitives::secret::Secret;

/// Fixture for a single "domain user" in the mock.
#[derive(Clone)]
pub struct AdFixture {
    pub password: &'static str,
    pub display_name: &'static str,
}

/// Deterministic mock AD client for development (no real domain controller needed).
pub struct MockAdClient {
    pub users: HashMap<String, AdFixture>,
    /// When `true`, every `authenticate` call returns `Unreachable` —
    /// simulates the AD server being down.
    pub unreachable: bool,
}

impl MockAdClient {
    /// Create a mock client with 2 pre-configured domain-user fixtures
    /// covering the main scenarios: success (us100), success (us200).
    /// Wrong-password / not-found scenarios are exercised against these
    /// fixtures by tests, not separate fixture entries.
    pub fn default_fixtures() -> Self {
        let mut users = HashMap::new();
        users.insert(
            "us100".to_string(),
            AdFixture {
                password: "Passw0rd!",
                display_name: "Иванов Иван Иванович",
            },
        );
        users.insert(
            "us200".to_string(),
            AdFixture {
                password: "Secret123",
                display_name: "Петрова Анна Сергеевна",
            },
        );
        Self {
            users,
            unreachable: false,
        }
    }

    /// Create a mock client that always reports the AD server as
    /// unreachable — for testing the "AD недоступен" path (USR-12).
    pub fn unreachable() -> Self {
        Self {
            users: HashMap::new(),
            unreachable: true,
        }
    }

    /// Strip `@domain` (UPN) or `DOMAIN\` (NetBIOS) prefix/suffix from a
    /// login to derive the bare lookup key (mirrors Pitfall 6 normalization,
    /// applied in reverse for fixture lookup).
    fn lookup_key(login: &str) -> &str {
        let without_upn_suffix = login.split('@').next().unwrap_or(login);
        without_upn_suffix
            .rsplit('\\')
            .next()
            .unwrap_or(without_upn_suffix)
    }
}

#[async_trait]
impl AdClient for MockAdClient {
    async fn authenticate(
        &self,
        login: &str,
        password: &Secret<String>,
    ) -> Result<AuthOutcome, AppError> {
        // CRITICAL (Pitfall 1 / T-09-01): reject empty/whitespace password
        // BEFORE any lookup — anonymous-bind trap closed even in the mock.
        if password.expose().trim().is_empty() {
            return Ok(AuthOutcome::BadCreds);
        }

        if self.unreachable {
            return Ok(AuthOutcome::Unreachable);
        }

        let key = Self::lookup_key(login);
        match self.users.get(key) {
            Some(fixture) if fixture.password == password.expose() => Ok(AuthOutcome::Ok {
                display_name: fixture.display_name.to_string(),
            }),
            // Wrong password AND unknown user both return the same generic
            // outcome — no enumeration (T-09-04).
            Some(_) => Ok(AuthOutcome::BadCreds),
            None => Ok(AuthOutcome::BadCreds),
        }
    }

    /// Mock reachability probe — honors the same `unreachable` fixture flag
    /// used by `authenticate` (D-Mock-01 failure-injection convention).
    async fn test_connection(&self) -> Result<AuthOutcome, AppError> {
        if self.unreachable {
            return Ok(AuthOutcome::Unreachable);
        }
        Ok(AuthOutcome::Ok {
            display_name: String::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pwd(s: &str) -> Secret<String> {
        Secret::new(s.to_string())
    }

    #[tokio::test]
    async fn success() {
        let mock = MockAdClient::default_fixtures();
        let result = mock
            .authenticate("us100", &pwd("Passw0rd!"))
            .await
            .expect("no error");
        assert_eq!(
            result,
            AuthOutcome::Ok {
                display_name: "Иванов Иван Иванович".to_string()
            }
        );
    }

    #[tokio::test]
    async fn display_name_returned() {
        let mock = MockAdClient::default_fixtures();
        let result = mock
            .authenticate("us200", &pwd("Secret123"))
            .await
            .expect("no error");
        match result {
            AuthOutcome::Ok { display_name } => {
                assert_eq!(display_name, "Петрова Анна Сергеевна");
            }
            other => panic!("expected Ok variant, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn wrong_password() {
        let mock = MockAdClient::default_fixtures();
        let result = mock
            .authenticate("us100", &pwd("WrongPassword!"))
            .await
            .expect("no error");
        assert_eq!(result, AuthOutcome::BadCreds);
    }

    #[tokio::test]
    async fn not_found() {
        let mock = MockAdClient::default_fixtures();
        let result = mock
            .authenticate("us999", &pwd("Whatever123"))
            .await
            .expect("no error");
        // Not-found must be indistinguishable from wrong-password (no enumeration).
        assert_eq!(result, AuthOutcome::BadCreds);
    }

    #[tokio::test]
    async fn unreachable_scenario() {
        let mock = MockAdClient::unreachable();
        let result = mock
            .authenticate("us100", &pwd("Passw0rd!"))
            .await
            .expect("no error");
        assert_eq!(result, AuthOutcome::Unreachable);
    }

    #[tokio::test]
    async fn empty_password_rejected() {
        let mock = MockAdClient::default_fixtures();
        let result = mock
            .authenticate("us100", &pwd(""))
            .await
            .expect("no error");
        assert_eq!(
            result,
            AuthOutcome::BadCreds,
            "empty password must be rejected without a lookup (anonymous-bind trap)"
        );
    }

    #[tokio::test]
    async fn whitespace_password_rejected() {
        let mock = MockAdClient::default_fixtures();
        let result = mock
            .authenticate("us100", &pwd("   "))
            .await
            .expect("no error");
        assert_eq!(
            result,
            AuthOutcome::BadCreds,
            "whitespace-only password must be rejected without a lookup"
        );
    }

    #[tokio::test]
    async fn upn_format_login_resolves_to_same_fixture() {
        let mock = MockAdClient::default_fixtures();
        let result = mock
            .authenticate("us100@corp.local", &pwd("Passw0rd!"))
            .await
            .expect("no error");
        assert_eq!(
            result,
            AuthOutcome::Ok {
                display_name: "Иванов Иван Иванович".to_string()
            }
        );
    }

    #[tokio::test]
    async fn netbios_format_login_resolves_to_same_fixture() {
        let mock = MockAdClient::default_fixtures();
        let result = mock
            .authenticate(r"CORP\us100", &pwd("Passw0rd!"))
            .await
            .expect("no error");
        assert_eq!(
            result,
            AuthOutcome::Ok {
                display_name: "Иванов Иван Иванович".to_string()
            }
        );
    }

    #[tokio::test]
    async fn test_connection_reachable_by_default() {
        let mock = MockAdClient::default_fixtures();
        let result = mock.test_connection().await.expect("no error");
        assert_eq!(
            result,
            AuthOutcome::Ok {
                display_name: String::new()
            }
        );
    }

    #[tokio::test]
    async fn test_connection_unreachable_fixture() {
        let mock = MockAdClient::unreachable();
        let result = mock.test_connection().await.expect("no error");
        assert_eq!(result, AuthOutcome::Unreachable);
    }
}
