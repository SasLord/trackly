---
phase: 09-ad
plan: 01
subsystem: auth
tags: [ldap3, hickory-resolver, rustls, active-directory, ldaps, async-trait, secret-zeroize]

# Dependency graph
requires:
  - phase: 03.3-snmp
    provides: SnmpClient port + RealSnmpClient/MockSnmpClient adapter pattern (mirrored exactly for AdClient)
provides:
  - AdClient trait + AuthOutcome enum (Ok/BadCreds/Unreachable) in I/O-free trackly-core
  - AdConfig section in AppConfig (bootstrap-only; live settings deferred to app_settings in plan 03)
  - RealAdClient: ldap3-based LDAPS simple_bind + display-name search, confined import of ldap3
  - MockAdClient: deterministic us100/us200 fixtures for dev macOS (no AD reachable)
  - discovery.rs: derive_base_dn pure transform + hickory-resolver DNS-SRV auto-detect with typed NoDomainDetected fallback
affects: [09-02, 09-03, 09-04, 09-05]

# Tech tracking
tech-stack:
  added: ["ldap3 0.12.1 (default-features=false, tls-rustls-ring)", "hickory-resolver 0.26.1"]
  patterns:
    - "Outcome-not-error: AdClient::authenticate returns Ok(AuthOutcome) for all auth results (success/bad-creds/unreachable); Err(AppError) reserved for genuine infra faults"
    - "Mock-via-trait: RealAdClient/MockAdClient behind AdClient trait, runtime-switched via config.ad.use_mock || TRACKLY_AD_MOCK env var (mirrors SnmpClient)"
    - "Single-module I/O confinement: ldap3 imported only in ad/real.rs; hickory-resolver only in ad/discovery.rs"
    - "No-enumeration: wrong-password and user-not-found both return AuthOutcome::BadCreds"

key-files:
  created:
    - crates/trackly-core/src/ports/ad.rs
    - crates/trackly-infra/src/ad/mod.rs
    - crates/trackly-infra/src/ad/mock.rs
    - crates/trackly-infra/src/ad/real.rs
    - crates/trackly-infra/src/ad/discovery.rs
  modified:
    - crates/trackly-core/src/ports/mod.rs
    - crates/trackly-infra/Cargo.toml
    - crates/trackly-infra/src/config.rs
    - crates/trackly-infra/src/lib.rs

key-decisions:
  - "Approved Task 0 package-legitimacy gate: ldap3 0.12.1 + hickory-resolver 0.26.1 confirmed legitimate, added with tls-rustls-ring (no OpenSSL)"
  - "AdConfig is bootstrap-only (TOML defaults + dev mock switch); live AD settings (enabled/host/domain) deferred to app_settings, wired in plan 03"
  - "Bind-name normalization: pass through login containing @ or \\, else append @domain from config (Pitfall 6)"
  - "Display name fallback chain: name_attr (displayName) -> cn -> raw login (D-Config-02)"
  - "hickory-resolver SRV extraction uses Record.data public field + RData::SRV match (no as_srv() convenience method exists in 0.26.1 — verified against installed source, not assumed from research skeleton)"

patterns-established:
  - "AD adapter triad (mod.rs/mock.rs/real.rs/discovery.rs) is a structural mirror of the SNMP triad — future port+adapter additions should follow the same shape"

requirements-completed: [USR-12]

# Metrics
duration: 8min
completed: 2026-06-20
---

# Phase 09 Plan 01: AD Client Port + Adapters Summary

**AdClient trait (I/O-free) with RealAdClient (ldap3 LDAPS simple_bind) and MockAdClient (us100/us200 fixtures) adapters, plus hickory-resolver-based DC auto-detect with a typed no-domain fallback**

## Performance

- **Duration:** 8 min (across Task 1 + Task 2; Task 0 approval occurred in a prior session)
- **Started:** 2026-06-19T23:52:48+07:00 (Task 1 commit)
- **Completed:** 2026-06-20T00:00:48+07:00 (Task 2 commit)
- **Tasks:** 2/2 (Task 0 checkpoint already approved by user before this session)
- **Files modified:** 9 (5 created, 4 modified)

## Accomplishments
- `AdClient` trait + `AuthOutcome` enum land in `trackly-core` with zero I/O imports — `no_io_deps` gate stays green
- `RealAdClient` performs LDAPS `simple_bind` via `ldap3`, normalizes bind names, escapes LDAP filter input, and resolves display name via `displayName` -> `cn` -> login fallback — all failure paths return typed outcomes, never `Err`
- `MockAdClient` ships 2 deterministic fixtures (us100/Иванов, us200/Петрова) covering success/wrong-password/not-found/unreachable/empty-password/UPN-format/NetBIOS-format — fully exercises USR-12 on dev macOS without a real domain controller
- `discovery.rs` derives LDAP base-DN from a DNS domain string (pure function) and performs an async DNS-SRV lookup via `hickory-resolver`, falling back to a typed `NoDomainDetected` result rather than panicking when no domain is reachable (the permanent dev-macOS state)
- `ldap3` confined to `real.rs`; `hickory-resolver` confined to `discovery.rs`; both verified via grep and `cargo tree` to bring in no OpenSSL/native-tls

## Task Commits

Each task was committed atomically:

1. **Task 0: Package legitimacy checkpoint (ldap3 + hickory-resolver)** - approved by user in a prior session (no code commit — verification-only gate)
2. **Task 1: AdClient port + AdConfig + dependencies** - `388fc62` (feat)
3. **Task 2: MockAdClient + RealAdClient + discovery (mirror SNMP triad)** - `247ea36` (feat)

**Plan metadata:** committed alongside this summary

_Note: tdd="true" markers in this plan describe "tests included with implementation" style (config.json `tdd_mode: false`), not strict RED/GREEN gate sequencing — both tasks shipped implementation + `#[cfg(test)]` suites in the same commit, matching how Task 1 was already executed in the prior session._

## Files Created/Modified
- `crates/trackly-core/src/ports/ad.rs` - `AdClient` trait + `AuthOutcome` enum (Ok/BadCreds/Unreachable), I/O-free, mirrors `ports/snmp.rs`
- `crates/trackly-core/src/ports/mod.rs` - wired `pub mod ad;`
- `crates/trackly-infra/Cargo.toml` - added `ldap3 0.12.1` (rustls-ring, no native-tls) + `hickory-resolver 0.26.1`
- `crates/trackly-infra/src/config.rs` - `AdConfig` struct + manual `Default`, wired into `AppConfig` root
- `crates/trackly-infra/src/ad/mod.rs` - module index, `pub mod mock; pub mod real; pub mod discovery;`
- `crates/trackly-infra/src/ad/mock.rs` - `MockAdClient` with 2 RU-named fixtures, 9 test cases
- `crates/trackly-infra/src/ad/real.rs` - `RealAdClient`, the only module importing `ldap3`
- `crates/trackly-infra/src/ad/discovery.rs` - `derive_base_dn`, `domain_from_env`, `discover_dc` (hickory SRV lookup)
- `crates/trackly-infra/src/lib.rs` - wired `pub mod ad;`

## Decisions Made
- ldap3/hickory-resolver package legitimacy approved (Task 0) — both confirmed on crates.io matching CLAUDE.md pins, no slopsquat risk
- `AdConfig` kept bootstrap-only per plan note; runtime-editable AD settings deferred to `app_settings` (plan 03)
- hickory-resolver SRV-record extraction implemented against the actual installed 0.26.1 API (public `Record.data` field + `RData::SRV` match) rather than the research skeleton's assumed `as_srv()` helper, which does not exist in this version — verified by reading installed crate source directly before writing code

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Corrected hickory-resolver 0.26.1 API usage in discovery.rs**
- **Found during:** Task 2 (discovery.rs implementation)
- **Issue:** Initial implementation (following a generic builder pattern) used `Resolver::builder_with_config` + `hickory_resolver::name_server::TokioConnectionProvider` (private module, does not exist as written) and `Record::data()` as a method call — both failed to compile (`E0433`, `E0603`, `E0599`)
- **Fix:** Switched to `Resolver::builder_tokio()?.build()?` (the actual public constructor) and accessed `record.data` as a public field (not a method), matching `Record<R>`'s real struct definition in hickory-proto 0.26.1
- **Files modified:** crates/trackly-infra/src/ad/discovery.rs
- **Verification:** `cargo build -p trackly-infra` succeeds; `cargo test -p trackly-infra ad::discovery` passes (5 tests)
- **Committed in:** `247ea36` (Task 2 commit — fixed before commit, not a follow-up)

---

**Total deviations:** 1 auto-fixed (1 blocking — incorrect assumed API corrected against installed crate source)
**Impact on plan:** No scope creep; fix was required to make the planned discovery.rs compile at all. RESEARCH.md's code skeleton for hickory-resolver was marked lower-confidence than the ldap3 skeleton, which is why this surfaced here and not in real.rs (ldap3's skeleton matched the installed API exactly on first attempt).

## Issues Encountered
None beyond the API-correction deviation above — all builds, tests, and clippy passes were clean on first or second attempt.

## User Setup Required
None - no external service configuration required. AD/LDAP connectivity is not testable from dev macOS by design (no domain reachable); `MockAdClient` covers all USR-12 scenarios locally. Real AD verification happens against a Windows-network DC in a later phase/manual test pass.

## Next Phase Readiness
- `AdClient` port + both adapters are ready for plan 02 (login flow wiring into `AppCtx`/Tauri command/axum handler) and plan 03 (admin-editable AD settings via `app_settings`, overriding the bootstrap `AdConfig` TOML defaults)
- `discovery.rs`'s `derive_base_dn`/`domain_from_env`/`discover_dc` are ready to back the "auto-detect" UX surface in 09-CONTEXT.md D-Config-01, but are not yet wired into any settings-resolution call site — that wiring belongs to plan 03
- No blockers identified for downstream AD-auth plans

---
*Phase: 09-ad*
*Completed: 2026-06-20*

## Self-Check: PASSED

All created files and commit hashes verified present on disk / in git history.
