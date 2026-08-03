//! Real AD directory adapter — service-account LDAP bind + `displayName`/
//! `cn`/`memberOf` search + `LDAP_MATCHING_RULE_IN_CHAIN` group-membership
//! check (Phase 31, SSO-01/SSO-03).
//!
//! This module and `crate::ad::real` are the only two places in the codebase
//! that import `ldap3`. `trackly-core::ports::ad_directory::AdDirectory`
//! trait must remain ldap3-free (enforced by `tests/no_io_deps.rs`).
//!
//! Unlike `RealAdClient` (binds as the logged-in user), this adapter ALWAYS
//! binds as a FIXED service account (`cfg.bind_dn`/`cfg.bind_password`) — SSO
//! users have no password to bind with, so a dedicated read-only service
//! account performs the lookup on their behalf. Results are cached in TWO
//! independently-TTL'd `TtlCache` instances (display name, role) so repeat
//! SSO logins do not hit the DC on every request.

use std::time::Duration;

use async_trait::async_trait;
use ldap3::{ldap_escape, LdapConnAsync, LdapConnSettings, Scope, SearchEntry};
use trackly_core::auth::Role;
use trackly_core::ports::ad_directory::{AdDirectory, DirectoryError, DirectoryResult};

use crate::ad::cache::TtlCache;
use crate::config::AdConfig;

/// Connection timeout for the initial LDAPS connect (mirrors `real.rs`'s
/// Pitfall 7 — always bound a timeout so a dead DC fails fast).
const CONN_TIMEOUT: Duration = Duration::from_secs(5);

/// Production AD directory adapter — service-account bind + search + group
/// check over `ldap3`, with a two-instance TTL cache in front.
pub struct RealAdDirectory {
    cfg: AdConfig,
    display_name_cache: TtlCache<String>,
    role_cache: TtlCache<Option<Role>>,
}

impl RealAdDirectory {
    pub fn new(cfg: AdConfig) -> Self {
        let display_name_cache =
            TtlCache::new(Duration::from_secs(cfg.display_name_cache_ttl_secs));
        let role_cache = TtlCache::new(Duration::from_secs(cfg.group_cache_ttl_secs));
        Self {
            cfg,
            display_name_cache,
            role_cache,
        }
    }

    /// Normalize the SERVICE ACCOUNT's own bind name — duplicates
    /// `RealAdClient::normalize_bind_name`'s exact logic (independent copy,
    /// per this codebase's established "small independent adapters"
    /// convention — see `mock.rs`'s own independent `lookup_key` rather than
    /// importing from `real.rs`).
    fn normalize_bind_name(&self, login: &str) -> String {
        if login.contains('@') || login.contains('\\') {
            login.to_string()
        } else {
            format!("{login}@{}", self.cfg.domain)
        }
    }
}

/// Normalize a login/`sAMAccountName` for use as a cache key or search value:
/// strip the `@domain` (UPN) suffix, strip the `DOMAIN\` (NetBIOS) prefix,
/// then lowercase — Pitfall 3, applied identically to how
/// `MockAdDirectory`/`MockAdClient` normalize for fixture lookup, so UPN,
/// NetBIOS, and bare forms of the same identity share one cache entry.
fn cache_key(sam_account_name: &str) -> String {
    let without_upn_suffix = sam_account_name
        .split('@')
        .next()
        .unwrap_or(sam_account_name);
    let without_netbios_prefix = without_upn_suffix
        .rsplit('\\')
        .next()
        .unwrap_or(without_upn_suffix);
    without_netbios_prefix.to_lowercase()
}

/// Build the user-search filter, escaping the login first (same
/// injection-defense treatment as `real.rs::build_user_search_filter`).
fn build_user_search_filter(sam_account_name: &str) -> String {
    let safe = ldap_escape(sam_account_name);
    format!("(|(sAMAccountName={safe})(userPrincipalName={safe}))")
}

/// Build the group-membership filter using
/// `LDAP_MATCHING_RULE_IN_CHAIN` (OID `1.2.840.113556.1.4.1941`) — AD
/// expands nested group membership server-side in one query. BOTH operands
/// are escaped via `ldap3::ldap_escape` — never skip escaping `group_dn` just
/// because it comes from trusted config, defense in depth (matches
/// `real.rs`'s own module-level philosophy).
fn build_group_membership_filter(sam_account_name: &str, group_dn: &str) -> String {
    format!(
        "(&(objectClass=user)(sAMAccountName={})(memberOf:1.2.840.113556.1.4.1941:={}))",
        ldap3::ldap_escape(sam_account_name),
        ldap3::ldap_escape(group_dn),
    )
}

/// Highest-privilege-wins role selection (RESEARCH Open Question 3, RESOLVED):
/// Admin > Manager > Employee, regardless of slice order. Pure, LDAP-free,
/// directly unit-testable. `None` when no group matched at all — the normal
/// "no configured group" outcome, not an error.
fn pick_highest_role(matched_roles: &[Role]) -> Option<Role> {
    if matched_roles.contains(&Role::Admin) {
        Some(Role::Admin)
    } else if matched_roles.contains(&Role::Manager) {
        Some(Role::Manager)
    } else if matched_roles.contains(&Role::Employee) {
        Some(Role::Employee)
    } else {
        None
    }
}

#[async_trait]
impl AdDirectory for RealAdDirectory {
    async fn resolve(&self, sam_account_name: &str) -> Result<DirectoryResult, DirectoryError> {
        let key = cache_key(sam_account_name);

        // (1) Cache short-circuit — BOTH caches must hit, otherwise fall
        // through to a fresh lookup. No LDAP connection is attempted here.
        if let (Some(display_name), Some(role)) =
            (self.display_name_cache.get(&key), self.role_cache.get(&key))
        {
            return Ok(DirectoryResult { display_name, role });
        }

        // (2) Not-configured gate — Pitfall 5, checked BEFORE any network
        // attempt so the optional directory-enrichment feature degrades
        // silently when the service bind isn't set up.
        if self.cfg.bind_dn.is_empty() || self.cfg.base_dn.is_empty() {
            return Err(DirectoryError::NotConfigured);
        }

        // (3) Connect.
        let settings = LdapConnSettings::new()
            .set_conn_timeout(CONN_TIMEOUT)
            .set_no_tls_verify(self.cfg.no_tls_verify);
        let url = format!("ldaps://{}:{}", self.cfg.host, self.cfg.port);

        let (conn, mut ldap) = match LdapConnAsync::with_settings(settings, &url).await {
            Ok(v) => v,
            Err(_) => return Err(DirectoryError::Unreachable),
        };
        // Pitfall 7: the connection driver task MUST be driven, or operations hang.
        ldap3::drive!(conn);

        // (4) Service-account bind — fixed DN/password, never the end user's own.
        let bind_name = self.normalize_bind_name(&self.cfg.bind_dn);
        let bind_result = match ldap.simple_bind(&bind_name, &self.cfg.bind_password).await {
            Ok(res) => res,
            Err(_) => return Err(DirectoryError::Unreachable), // protocol/IO error mid-bind
        };
        if bind_result.success().is_err() {
            // rc != 0 — the SERVICE ACCOUNT's own credentials were rejected.
            // Config error, distinct from a network outage.
            let _ = ldap.unbind().await;
            return Err(DirectoryError::ServiceBindFailed);
        }

        // (5) Search displayName/cn/memberOf in ONE round trip.
        let filter = build_user_search_filter(sam_account_name);
        let attrs = vec![self.cfg.name_attr.as_str(), "cn", "memberOf"];

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
                .unwrap_or_else(|| sam_account_name.to_string()), // D-Config-02 fallback chain
            // Search failure after a successful bind is non-fatal for the
            // display name (falls back to login) — the bind itself already
            // proved reachability, so this does NOT return `Unreachable`.
            Err(_) => sam_account_name.to_string(),
        };

        // (6) Group/role resolution — one separate search per configured
        // mapping entry, in config order (order doesn't matter for the
        // final result since `pick_highest_role` re-sorts by priority).
        let mut matched_roles: Vec<Role> = Vec::new();
        for entry in &self.cfg.role_mapping {
            let role = match Role::from_str(&entry.role) {
                Ok(role) => role,
                // Unparseable role string in config — skip this entry rather
                // than failing the whole resolve (log-and-skip).
                Err(_) => {
                    tracing::warn!(
                        group_dn = %entry.group_dn,
                        role = %entry.role,
                        "unparseable role in AD role_mapping entry, skipping"
                    );
                    continue;
                }
            };
            let group_filter = build_group_membership_filter(sam_account_name, &entry.group_dn);
            match ldap
                .search(&self.cfg.base_dn, Scope::Subtree, &group_filter, vec!["dn"])
                .await
                .and_then(|search_result| search_result.success())
            {
                Ok((entries, _res)) => {
                    if !entries.is_empty() {
                        matched_roles.push(role);
                    }
                }
                // An I/O error DURING the group-check loop maps the WHOLE
                // resolve to `Unreachable` — fail-closed (Pitfall 4): a group
                // check that could not complete must never silently proceed
                // as "no groups matched".
                Err(_) => {
                    let _ = ldap.unbind().await;
                    return Err(DirectoryError::Unreachable);
                }
            }
        }

        let _ = ldap.unbind().await;

        let role = pick_highest_role(&matched_roles);

        // (7) Populate both caches with the resolved values.
        self.display_name_cache
            .put(key.clone(), display_name.clone());
        self.role_cache.put(key, role.clone());

        Ok(DirectoryResult { display_name, role })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- build_group_membership_filter: injection-defense tests, mirroring
    //     real.rs's build_user_search_filter test suite, parameterized over
    //     BOTH the sam_account_name and group_dn argument positions. ---

    #[test]
    fn benign_login_and_group_dn_build_expected_filter() {
        let filter =
            build_group_membership_filter("us100", "CN=IT-Admins,OU=Groups,DC=example,DC=local");
        assert_eq!(
            filter,
            "(&(objectClass=user)(sAMAccountName=us100)(memberOf:1.2.840.113556.1.4.1941:=CN=IT-Admins,OU=Groups,DC=example,DC=local))"
        );
    }

    #[test]
    fn injection_payload_metacharacters_are_escaped_in_both_positions() {
        let payload = "*)(uid=*))(|(uid=*";
        let filter = build_group_membership_filter(payload, payload);

        assert!(
            !filter.contains('*'),
            "no raw `*` may survive escaping: {filter}"
        );
        assert!(
            !filter.contains(payload),
            "raw payload must not appear verbatim in either position: {filter}"
        );
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
        // The filter still has exactly our intended structural clauses — the
        // payload could not inject an extra AND/OR at the top level.
        assert!(filter.starts_with("(&(objectClass=user)"));
        assert_eq!(
            filter.matches("(&").count(),
            1,
            "no injected top-level AND clause: {filter}"
        );
    }

    #[test]
    fn backslash_in_either_argument_is_escaped() {
        let filter = build_group_membership_filter("dom\\ain", "grp\\dn");
        assert!(
            !filter.contains("dom\\ain") || filter.contains("\\5c"),
            "raw backslash must be escaped: {filter}"
        );
        assert!(
            filter.contains("\\5c"),
            "`\\` must be escaped to \\5c: {filter}"
        );
        assert!(
            !filter.contains("grp\\dn"),
            "raw backslash in group_dn must not survive: {filter}"
        );
    }

    // --- pick_highest_role: pure priority-selection tests ---

    #[test]
    fn admin_wins_regardless_of_slice_order() {
        assert_eq!(
            pick_highest_role(&[Role::Manager, Role::Admin]),
            Some(Role::Admin)
        );
        assert_eq!(
            pick_highest_role(&[Role::Admin, Role::Manager]),
            Some(Role::Admin)
        );
    }

    #[test]
    fn empty_slice_returns_none() {
        assert_eq!(pick_highest_role(&[]), None);
    }

    #[test]
    fn employee_only_slice_returns_some_employee() {
        assert_eq!(pick_highest_role(&[Role::Employee]), Some(Role::Employee));
    }

    // --- Cache short-circuit tests (closes plan-checker BLOCKER 1) ---

    fn unreachable_but_configured_cfg() -> AdConfig {
        AdConfig {
            host: "127.0.0.1".to_string(),
            port: 1, // nothing listens here — any real connection attempt fails fast
            bind_dn: "svc-trackly-ro@example.local".to_string(),
            bind_password: "CHANGE_ME".to_string(),
            base_dn: "dc=example,dc=local".to_string(),
            ..AdConfig::default()
        }
    }

    #[tokio::test]
    async fn cache_hit_short_circuits_ldap_call() {
        let directory = RealAdDirectory::new(unreachable_but_configured_cfg());
        let key = cache_key("us100");
        directory
            .display_name_cache
            .put(key.clone(), "Иванов Иван Иванович".to_string());
        directory.role_cache.put(key, Some(Role::Manager));

        let result = directory.resolve("us100").await;
        assert_eq!(
            result,
            Ok(DirectoryResult {
                display_name: "Иванов Иван Иванович".to_string(),
                role: Some(Role::Manager),
            })
        );
    }

    #[tokio::test]
    async fn cache_miss_falls_through_to_fresh_unreachable_lookup() {
        let directory = RealAdDirectory::new(unreachable_but_configured_cfg());
        // No `.put(...)` pre-warm — cache miss must still ATTEMPT a fresh
        // lookup, never fabricate a hit.
        let result = directory.resolve("us100").await;
        assert!(matches!(result, Err(DirectoryError::Unreachable)));
    }
}
