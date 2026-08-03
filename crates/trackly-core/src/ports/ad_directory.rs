//! `AdDirectory` port — abstraction for Active Directory service-account
//! directory lookups (SSO-01/SSO-03).
//!
//! Pattern: like `AdClient`, this trait lives in trackly-core but has NO
//! ldap3/hickory/tokio imports — I/O-free invariant enforced by `tests/no_io_deps.rs`.
//! The real impl (`RealAdDirectory`) lives in `trackly_infra::ad::directory`.
//! The mock impl (`MockAdDirectory`) lives in `trackly_infra::ad::directory_mock`.
//!
//! Runtime switching via `AppCtx::build` checks `TRACKLY_AD_MOCK` env var
//! or `config.ad.use_mock` (D-Mock-01), same switch used for `AdClient`.
//!
//! CRITICAL: This trait MUST NOT import tokio, ldap3, or hickory-resolver —
//! those are infra-layer deps. `async_trait` + `crate::auth::Role` are the
//! only allowed dependencies here (pure-data crate, enforced by
//! `tests/no_io_deps.rs`).

use async_trait::async_trait;

use crate::auth::Role;

/// Result of resolving a single AD account via the service-account bind.
///
/// Combines the displayName resolve (SSO-01) and the group-membership role
/// hint (SSO-03) into one round trip.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectoryResult {
    /// Resolved via displayName → cn → login fallback chain (D-Config-02),
    /// mirroring `AuthOutcome::Ok { display_name }`'s resolution order.
    pub display_name: String,
    /// `Some(role)` when the account matched a configured AD group mapping;
    /// `None` when the directory was reachable and checked, but no
    /// configured group matched — this is NOT a failure, the caller must
    /// treat it as "no elevation, default role applies" (SSO-03 regression
    /// case), never conflate it with `Err(DirectoryError::Unreachable)`.
    pub role: Option<Role>,
}

/// Failure modes for a directory resolve attempt.
///
/// Modeled as data, not collapsed into a boolean — mirrors `AuthOutcome`'s
/// 3-state philosophy in `crate::ports::ad`. Each variant is operationally
/// distinct and must be handled differently by the caller (Plan 31-03's
/// `AuthService::sso_login` fail-closed wiring):
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DirectoryError {
    /// Service-account bind is not configured at all (bind DN/password/base
    /// DN missing). Expected, silent-degrade state (Pitfall 5) — this is
    /// NOT an operational error worth logging per request, it just means
    /// the optional directory-enrichment feature is off.
    NotConfigured,
    /// The service account's OWN credentials were rejected by AD. This is a
    /// configuration error (wrong bind DN/password), distinct from an
    /// end-user credential failure — loggable, an admin needs to fix it.
    ServiceBindFailed,
    /// The AD server could not be reached (network/TLS/timeout failure)
    /// while attempting the resolve. Loggable. The caller (`AuthService`)
    /// MUST match on this variant explicitly to avoid role elevation —
    /// never treat "couldn't check" as "checked, not a member" (Pitfall 4).
    Unreachable,
}

/// AD directory port — implemented by `RealAdDirectory` and `MockAdDirectory`.
///
/// CRITICAL: This trait MUST NOT import tokio, ldap3, or hickory-resolver —
/// those are infra-layer deps. `async_trait` + `crate::auth::Role` are the
/// only allowed dependencies here (pure-data crate, enforced by
/// `tests/no_io_deps.rs`).
#[async_trait]
pub trait AdDirectory: Send + Sync {
    /// Resolve `sam_account_name` to its displayName and AD-group role hint
    /// in one combined round trip (mirrors the architecture diagram's
    /// `directory.resolve(ad_username)` call).
    ///
    /// # Arguments
    /// * `sam_account_name` - AD login as authenticated by Kerberos/SPNEGO
    ///   (`us100`, `user@domain.tld`, or `DOMAIN\user` — normalization is an
    ///   implementation detail of the adapter, not this port).
    async fn resolve(&self, sam_account_name: &str) -> Result<DirectoryResult, DirectoryError>;
}
