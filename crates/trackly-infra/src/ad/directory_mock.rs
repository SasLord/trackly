//! Mock AD directory — deterministic fixtures for dev macOS (D-Mock-01, SSO-01/SSO-03).
//!
//! Used when `TRACKLY_AD_MOCK` env var is set or `config.ad.use_mock = true`.
//! Returns preset `DirectoryResult`s keyed by AD login (sAMAccountName-style),
//! reusing the SAME `us100`/`us200` fixture identities already established in
//! `mock.rs` (privacy-placeholder discipline — no new fixture names invented).
//!
//! 2 fixtures (per plan must_haves):
//!   us100 — Иванов Иван Иванович, role: Some(Role::Manager)
//!   us200 — Петрова Анна Сергеевна, role: None (no configured group)

use std::collections::HashMap;

use async_trait::async_trait;
use trackly_core::auth::Role;
use trackly_core::ports::ad_directory::{AdDirectory, DirectoryError, DirectoryResult};

/// Fixture for a single "domain user" in the directory mock.
#[derive(Clone)]
pub struct DirectoryFixture {
    pub display_name: &'static str,
    pub role: Option<Role>,
}

/// Deterministic mock AD directory for development (no real domain
/// controller / service-account bind needed).
pub struct MockAdDirectory {
    pub users: HashMap<String, DirectoryFixture>,
    /// When `true`, every `resolve` call returns `Err(DirectoryError::Unreachable)` —
    /// simulates the AD server (or service-account bind) being down.
    pub unreachable: bool,
}

impl MockAdDirectory {
    /// Create a mock directory with 2 pre-configured fixtures covering the
    /// main scenarios: known user with a mapped group role (us100), known
    /// user with no matched group (us200).
    pub fn default_fixtures() -> Self {
        let mut users = HashMap::new();
        users.insert(
            "us100".to_string(),
            DirectoryFixture {
                display_name: "Иванов Иван Иванович",
                role: Some(Role::Manager),
            },
        );
        users.insert(
            "us200".to_string(),
            DirectoryFixture {
                display_name: "Петрова Анна Сергеевна",
                role: None,
            },
        );
        Self {
            users,
            unreachable: false,
        }
    }

    /// Create a mock directory that always reports the AD server as
    /// unreachable — for testing the fail-closed path (SSO-03).
    pub fn unreachable() -> Self {
        Self {
            users: HashMap::new(),
            unreachable: true,
        }
    }

    /// Strip `@domain` (UPN) or `DOMAIN\` (NetBIOS) prefix/suffix from a
    /// login to derive the bare lookup key (mirrors `MockAdClient::lookup_key`
    /// exactly — Pitfall 3 — duplicated here rather than shared, matching
    /// the codebase's established "small independent adapters" convention).
    fn lookup_key(login: &str) -> &str {
        let without_upn_suffix = login.split('@').next().unwrap_or(login);
        without_upn_suffix
            .rsplit('\\')
            .next()
            .unwrap_or(without_upn_suffix)
    }
}

#[async_trait]
impl AdDirectory for MockAdDirectory {
    async fn resolve(&self, sam_account_name: &str) -> Result<DirectoryResult, DirectoryError> {
        if self.unreachable {
            return Err(DirectoryError::Unreachable);
        }

        let key = Self::lookup_key(sam_account_name);
        match self.users.get(key) {
            Some(fixture) => Ok(DirectoryResult {
                display_name: fixture.display_name.to_string(),
                role: fixture.role.clone(),
            }),
            // Unknown-but-Kerberos-already-authenticated login degrades
            // gracefully to the login itself — never happens in practice
            // (Kerberos already proved the account exists), but the mock
            // must not panic on it (mirrors real.rs's D-Config-02 fallback).
            None => Ok(DirectoryResult {
                display_name: sam_account_name.to_string(),
                role: None,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn known_user_resolves_display_name_and_role() {
        let mock = MockAdDirectory::default_fixtures();
        let result = mock.resolve("us100").await.expect("no error");
        assert_eq!(
            result,
            DirectoryResult {
                display_name: "Иванов Иван Иванович".to_string(),
                role: Some(Role::Manager),
            }
        );
    }

    #[tokio::test]
    async fn user_with_no_group_resolves_none_role() {
        let mock = MockAdDirectory::default_fixtures();
        let result = mock.resolve("us200").await.expect("no error");
        assert_eq!(
            result,
            DirectoryResult {
                display_name: "Петрова Анна Сергеевна".to_string(),
                role: None,
            }
        );
    }

    #[tokio::test]
    async fn unknown_login_falls_back_to_login_itself() {
        let mock = MockAdDirectory::default_fixtures();
        let result = mock.resolve("us999").await.expect("no error");
        assert_eq!(
            result,
            DirectoryResult {
                display_name: "us999".to_string(),
                role: None,
            }
        );
    }

    #[tokio::test]
    async fn upn_and_netbios_forms_resolve_to_same_fixture() {
        let mock = MockAdDirectory::default_fixtures();

        let upn_result = mock
            .resolve("us100@example.local")
            .await
            .expect("no error");
        assert_eq!(
            upn_result,
            DirectoryResult {
                display_name: "Иванов Иван Иванович".to_string(),
                role: Some(Role::Manager),
            }
        );

        let netbios_result = mock.resolve(r"EXAMPLE\us100").await.expect("no error");
        assert_eq!(
            netbios_result,
            DirectoryResult {
                display_name: "Иванов Иван Иванович".to_string(),
                role: Some(Role::Manager),
            }
        );
    }

    #[tokio::test]
    async fn unreachable_fixture_returns_typed_error() {
        let mock = MockAdDirectory::unreachable();
        let result = mock.resolve("us100").await;
        assert_eq!(result, Err(DirectoryError::Unreachable));
    }
}
