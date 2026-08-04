---
phase: 31-ad-bind-ad
plan: 02
subsystem: auth
tags: [ldap3, service-bind, ldap-matching-rule-in-chain, ttl-cache, config, rust]

requires:
  - "31-01: AdDirectory port, DirectoryResult/DirectoryError, TtlCache<V>, MockAdDirectory"
provides:
  - "AdConfig extended: bind_dn, bind_password (manual redacting Debug), base_dn, role_mapping table, display_name_cache_ttl_secs, group_cache_ttl_secs"
  - "RealAdDirectory: service-account simple bind -> sAMAccountName search -> displayName/cn/login fallback (SSO-01)"
  - "Group-membership resolution via LDAP_MATCHING_RULE_IN_CHAIN (OID 1.2.840.113556.1.4.1941), highest-privilege-wins, fail-closed 3-state (SSO-03)"
  - "Two-instance TTL cache short-circuit (display_name + role) so repeat SSO logins skip the DC"
  - "trackly.config.toml.example refreshed with placeholder [ad] section"
affects: [31-03-auth-service-wiring, 31-04-integration-test]

tech-stack:
  added: []
  patterns:
    - "RealAdDirectory mirrors RealAdClient's ldap3 connect+bind+search shape but binds a FIXED service account (SSO users have no password)"
    - "ldap_escape applied to BOTH sam_account_name and group_dn (defense-in-depth, same treatment as real.rs)"
    - "bind_password guarded by a manual redacting Debug impl on AdConfig (Secret<T> forbids Deserialize); AppConfig's derived Debug propagates the redaction"
    - "Group-check I/O error maps the WHOLE resolve to Unreachable (fail-closed); post-bind search failure for displayName degrades to login (bind already proved reachability)"

key-files:
  created:
    - crates/trackly-infra/src/ad/directory.rs
  modified:
    - crates/trackly-infra/src/config.rs
    - trackly.config.toml.example
    - crates/trackly-infra/src/ad/mod.rs
    - crates/trackly-infra/src/ad/real.rs
    - crates/trackly-infra/src/ad/sso.rs

key-decisions:
  - "Group->role mapping is TOML-only ([[ad.role_mapping]] with full group DN), per RESEARCH Open Question 1 (RESOLVED) — no Settings-UI CRUD in this phase"
  - "Split cache TTLs: display_name default 1800s (cosmetic), group/role default 300s (authorization, faster revocation) — RESEARCH Open Question 2 (RESOLVED)"
  - "Highest-privilege-wins (Admin > Manager > Employee) via pure pick_highest_role — RESEARCH Open Question 3 (RESOLVED)"
  - "Cache short-circuit requires BOTH caches to hit; NotConfigured checked before any network attempt so unconfigured directory degrades silently (Pitfall 5)"

patterns-established:
  - "RealAdDirectory::resolve does user-search (displayName) + per-mapping group-search in one bound session, then populates both caches"

requirements-completed: [SSO-01, SSO-03]

duration: orchestrator-closed
completed: 2026-08-03
---

# Phase 31 Plan 02: RealAdDirectory + AdConfig extension Summary

**Implemented the real LDAP service-account directory adapter — fixed-account bind, `sAMAccountName`→`displayName` resolve with cache, and `LDAP_MATCHING_RULE_IN_CHAIN` group→role mapping with fail-closed 3-state error handling — plus the `AdConfig` schema (redacting `Debug` for the bind password) and a refreshed placeholder config example.**

## Accomplishments
- `AdConfig` extended with `bind_dn`, `bind_password` (manual redacting `Debug` — no leak via `{:?}`), `role_mapping` table, and split `display_name_cache_ttl_secs` / `group_cache_ttl_secs`
- `RealAdDirectory` implements `AdDirectory`: cache short-circuit → NotConfigured gate → LDAPS connect (5s timeout) → service bind → one-round-trip displayName search → per-mapping nested-group query → highest-privilege role → cache populate
- `ldap_escape` applied to both `sam_account_name` and `group_dn`; injection-defense unit tests (both argument positions) + `pick_highest_role` tests + the two cache-short-circuit tests (closes plan-checker BLOCKER 1)
- `trackly.config.toml.example` refreshed with a placeholder `[ad]` section (example.local, svc-trackly-ro, CN=...,DC=example,DC=local — placeholders only)

## Task Commits
1. **Task 1: Extend AdConfig + refresh trackly.config.toml.example** — `d65c847` (feat)
2. **Task 2: RealAdDirectory (service-account bind + group-membership check)** — `77153fb` (feat)
3. **Deviation: pre-existing clippy fix in ad/sso.rs** — `a33e4fc` (fix)

## Verification
All commands run one-at-a-time (per `cargo_no_concurrent_test`):
- `cargo build -p trackly-infra` — clean (2m 03s cold)
- `cargo test -p trackly-infra` — all pass (113 lib tests incl. config redacting-Debug, `ad::directory` incl. cache short-circuit pair, `ad::cache`; + integration bins), 0 failed
- `cargo test -p trackly-core --test no_io_deps` — 1 passed (core stays ldap3-free; `directory.rs` correctly lives in trackly-infra)
- `cargo clippy -p trackly-infra -- -D warnings` — clean (after the sso.rs deviation fix)
- `rustfmt --check` on `directory.rs` — clean

## Deviations from Plan

**1. [Rule 1 — Bug] Pre-existing `clippy::unnecessary_mut_passed` in `ad/sso.rs` surfaced by the recompile**
- **Found during:** `cargo clippy -p trackly-infra -- -D warnings` (Wave 2 gate)
- **Issue:** Adding `ad::directory` forced clippy to re-lint the whole crate, surfacing a latent lint at `sso.rs:145` — `resolve_with_client(&mut net)` passes `&mut` where the sspi API takes an immutable borrow (`OfflineNetworkClient` is never mutated). Plan 31-01's clippy run had hit an incremental-cache clean result for the unchanged `sso.rs`.
- **Fix:** `let mut net` → `let net`, `&mut net` → `&net`. Semantically neutral (offline client only ever errors on a call).
- **Files modified:** crates/trackly-infra/src/ad/sso.rs (out of plan `files_modified` — required to keep the `-D warnings` gate green)
- **Committed in:** `a33e4fc`

**2. [Process] Orchestrator close-out after executor stall**
- The plan executor wrote all Task 1/Task 2 code correctly but repeatedly backgrounded the slow `cargo` runs (transient resource contention made a `cargo test` appear to hang ~20 min) and returned before committing. The orchestrator verified the build/tests/clippy/fmt directly (all green), applied the two mechanical fixes above, and created the atomic task commits + this SUMMARY. No code logic was authored by the orchestrator beyond the one-line sso.rs clippy fix and formatting `directory.rs`.

**Total deviations:** 2 (1 auto-fixed bug, 1 process). No scope creep — pre-existing repo-wide fmt drift in unrelated files was deliberately left untouched.

## Issues Encountered
- Executor stalls on slow `cargo test` (see Deviation 2). Cache is now warm; subsequent waves should run faster.

## User Setup Required
- None for dev/test (mock path). For production: populate the `[ad]` service-bind fields and `[[ad.role_mapping]]` entries in the gitignored `trackly.config.toml` on the Windows/domain machine.

## Next Phase Readiness
- `RealAdDirectory` + `AdConfig` schema are ready for Plan 31-03 (wire `AdDirectory` into `AuthService.sso_login`, thread role into the two hardcoded-`'employee'` INSERT sites, fix all 8 `AuthService::new` call sites).
- No blockers.

---
*Phase: 31-ad-bind-ad*
*Completed: 2026-08-03*

## Self-Check: PASSED

`directory.rs` present on disk; commit hashes `d65c847`, `77153fb`, `a33e4fc` verified in `git log`; all verification commands green.
