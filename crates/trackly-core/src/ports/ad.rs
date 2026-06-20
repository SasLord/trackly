//! `AdClient` port — abstraction for Active Directory authentication (USR-08/USR-12).
//!
//! Pattern: like `SnmpClient`, this trait lives in trackly-core but has NO
//! ldap3/hickory/tokio imports — I/O-free invariant enforced by `tests/no_io_deps.rs`.
//! The real impl (`RealAdClient`) lives in `trackly_infra::ad::real`.
//! The mock impl (`MockAdClient`) lives in `trackly_infra::ad::mock`.
//!
//! Runtime switching via `AppCtx::build` checks `TRACKLY_AD_MOCK` env var
//! or `config.ad.use_mock` (D-Mock-01).
//!
//! CRITICAL (Pitfall 1 / T-09-01): implementations MUST reject an empty or
//! whitespace-only password as `AuthOutcome::BadCreds` WITHOUT performing a
//! bind. Per RFC 4513 §5.1.2, a non-empty DN + empty password is an
//! *unauthenticated* bind that many LDAP servers accept (rc=0), which would
//! otherwise be misread as a successful login (anonymous-bind trap).

use async_trait::async_trait;

use crate::error::AppError;
use crate::primitives::secret::Secret;

/// Outcome of an AD authentication attempt.
///
/// Modeled as data, not an error — `BadCreds`/`Unreachable` are normal
/// outcomes the caller branches on (same philosophy as `SnmpClient::get_oids`
/// returning `Ok(None)` for an unreachable printer). `AppError` is reserved
/// for genuine infrastructure faults (e.g. malformed config).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthOutcome {
    /// Bind succeeded; `display_name` resolved via displayName → cn → login
    /// fallback chain (D-Config-02).
    Ok { display_name: String },
    /// Wrong password OR unknown user. Deliberately generic — both cases
    /// return the same variant to prevent user enumeration (T-09-04).
    BadCreds,
    /// The AD server could not be reached (network/TLS/timeout failure),
    /// distinct from `BadCreds` so the caller can surface a different
    /// message ("AD недоступен" vs "неверный логин или пароль").
    Unreachable,
}

/// AD client port — implemented by `RealAdClient` and `MockAdClient`.
///
/// CRITICAL: This trait MUST NOT import tokio, ldap3, or hickory-resolver —
/// those are infra-layer deps. `async_trait` + `crate::error::AppError` +
/// `crate::primitives::secret::Secret` are the only allowed dependencies
/// here (pure-data crate, enforced by `tests/no_io_deps.rs`).
#[async_trait]
pub trait AdClient: Send + Sync {
    /// Bind as `login` with `password`; on success resolve the user's
    /// display name.
    ///
    /// Implementations MUST reject an empty or whitespace-only password as
    /// `Ok(AuthOutcome::BadCreds)` BEFORE attempting any bind — see the
    /// module-level CRITICAL note (Pitfall 1 / RFC 4513 §5.1.2).
    ///
    /// # Arguments
    /// * `login` - AD login as entered by the user (`us100`, `user@domain.tld`,
    ///   or `DOMAIN\user` — see Pitfall 6 bind-name normalization).
    /// * `password` - wrapped in `Secret<String>`; never logged, never persisted.
    async fn authenticate(
        &self,
        login: &str,
        password: &Secret<String>,
    ) -> Result<AuthOutcome, AppError>;

    /// Verify AD server reachability WITHOUT any end-user credentials.
    ///
    /// This is NOT a credential check — it only confirms the configured AD
    /// server can be reached (TCP+TLS connect, optionally an anonymous/
    /// root-DSE probe). Used by the "Проверить подключение" admin action
    /// (Phase 9 gap-closure) so an admin can validate connectivity before
    /// any user attempts to log in.
    ///
    /// Reuses `AuthOutcome` rather than inventing a parallel result type:
    /// `Ok { .. }` → reachable, `Unreachable` → not reachable. `BadCreds`
    /// is never returned here (no credentials are presented).
    async fn test_connection(&self) -> Result<AuthOutcome, AppError>;
}
