---
phase: 31-ad-bind-ad
plan: 01
subsystem: auth
tags: [ldap3, async-trait, ttl-cache, hexagonal-port, rust]

requires: []
provides:
  - "AdDirectory port trait (trackly-core), ldap3-free, mirrors AdClient's 3-state philosophy"
  - "DirectoryResult { display_name, role: Option<Role> } and DirectoryError { NotConfigured, ServiceBindFailed, Unreachable }"
  - "MockAdDirectory deterministic fixtures (us100 -> Manager, us200 -> None, unknown -> fallback, unreachable() -> typed error)"
  - "Generic TtlCache<V> hand-rolled primitive (put/get/expiry) for use as two independently-TTL'd instances"
affects: [31-02-real-ad-directory, 31-03-auth-service-wiring, 31-04-integration-test]

tech-stack:
  added: []
  patterns:
    - "AdDirectory port mirrors AdClient exactly: trait in trackly-core::ports, Real/Mock impls in trackly-infra::ad, ldap3-free core enforced by no_io_deps.rs"
    - "3-state (never boolean) outcome modeling extended from AuthOutcome to DirectoryError (NotConfigured/ServiceBindFailed/Unreachable)"
    - "Hand-rolled Mutex<HashMap<K,(V,Instant)>> TTL cache, generic over V, mirrors ReaderPool's small-primitive-no-crate convention"

key-files:
  created:
    - crates/trackly-core/src/ports/ad_directory.rs
    - crates/trackly-infra/src/ad/directory_mock.rs
    - crates/trackly-infra/src/ad/cache.rs
  modified:
    - crates/trackly-core/src/ports/mod.rs
    - crates/trackly-infra/src/ad/mod.rs

key-decisions:
  - "MockAdDirectory reuses existing us100/us200 fixture identities (no new placeholder names invented) per privacy discipline"
  - "TtlCache<V> is generic (not a fixed DirectoryCacheEntry shape) so RealAdDirectory (31-02) can hold two independently-TTL'd instances (display_name, role)"

patterns-established:
  - "AdDirectory::resolve() combines displayName + role lookup in one round trip (mirrors architecture diagram's directory.resolve(ad_username))"

requirements-completed: [SSO-01, SSO-03]

duration: 39min
completed: 2026-08-03
---

# Phase 31 Plan 01: Служебный AD-bind — port contract + mock + TTL cache Summary

**Defined the ldap3-free `AdDirectory` port (displayName + AD-group role resolve) plus its two dependency-free building blocks — a deterministic `MockAdDirectory` and a generic hand-rolled `TtlCache<V>` — all unit-tested and compiling in isolation ahead of the real LDAP adapter (Plan 31-02).**

## Performance

- **Duration:** ~39 min
- **Started:** 2026-08-03T18:16:48+07:00 (prior plan commit) / first task commit 18:24:28+07:00
- **Completed:** 2026-08-03T18:55:15+07:00
- **Tasks:** 3 completed
- **Files modified:** 5 (3 created, 2 modified)

## Accomplishments
- `AdDirectory` trait + `DirectoryResult`/`DirectoryError` defined in `trackly-core`, verified ldap3-free by `no_io_deps` gate
- `MockAdDirectory` with 5 passing unit tests covering known-user/no-group/unknown-fallback/UPN-NetBIOS-normalization/unreachable scenarios
- Generic `TtlCache<V>` with 4 passing unit tests (put/get, empty-miss, millisecond-scale TTL expiry, key isolation)

## Task Commits

Each task was committed atomically:

1. **Task 1: AdDirectory port contract (trackly-core)** - `6115c63` (feat)
2. **Task 2: MockAdDirectory fixtures (trackly-infra)** - `685ea5b` (test)
3. **Task 3: Generic TtlCache primitive** - `86ce4b3` (feat)
4. **Formatting fixup (Tasks 2/3 files)** - `cf8354c` (style)

**Plan metadata:** (this commit, follows)

## Files Created/Modified
- `crates/trackly-core/src/ports/ad_directory.rs` - `AdDirectory` trait, `DirectoryResult`, `DirectoryError` (3-variant, never boolean)
- `crates/trackly-core/src/ports/mod.rs` - registers `pub mod ad_directory;`
- `crates/trackly-infra/src/ad/directory_mock.rs` - `MockAdDirectory` fixtures (us100/us200 reused, `unreachable()` ctor)
- `crates/trackly-infra/src/ad/cache.rs` - `TtlCache<V>` generic hand-rolled primitive
- `crates/trackly-infra/src/ad/mod.rs` - registers `cache` (first) and `directory_mock` (before `discovery`); final order `cache, directory_mock, discovery, keytab, mock, real, sso`

## Decisions Made
- Reused the existing `us100`/`us200` fixture identities from `mock.rs` rather than inventing new placeholder names, per RESEARCH/PATTERNS explicit instruction and the project's privacy-placeholder discipline.
- `TtlCache<V>` implemented generically (not the fixed `DirectoryCacheEntry{display_name, role}` shape shown in RESEARCH's skeleton) so Plan 31-02's `RealAdDirectory` can hold two separately-configured instances (`TtlCache<String>` for display_name, `TtlCache<Option<Role>>` for role) with independent TTLs, per RESEARCH Open Question 2's resolved recommendation.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `cargo fmt` formatting corrections on Task 2/3 files**
- **Found during:** Post-task verification (`cargo fmt --check`)
- **Issue:** `directory_mock.rs`'s UPN-form test line and `cache.rs`'s `put_then_get_returns_value` assertion exceeded the project's line-width convention as originally written, producing a non-canonical multi-line wrap
- **Fix:** Ran `cargo fmt -p trackly-infra -p trackly-core`, then reverted the fmt tool's unrelated reformatting of `keytab.rs`/`sso.rs`/`repos/audit_log_sqlite.rs` (out of scope for this plan — pre-existing formatting drift in files this plan does not touch) via `git checkout --`, keeping only the in-scope `cache.rs`/`directory_mock.rs` fixes
- **Files modified:** crates/trackly-infra/src/ad/cache.rs, crates/trackly-infra/src/ad/directory_mock.rs
- **Verification:** `cargo fmt --check` now shows no diff for either file; `cargo test -p trackly-infra ad::` re-run, all 37 ad-module tests still pass
- **Committed in:** cf8354c

---

**Total deviations:** 1 auto-fixed (1 bug/formatting)
**Impact on plan:** Cosmetic only, no behavior change. No scope creep — out-of-scope formatting drift in unrelated files was explicitly excluded, not fixed.

## Issues Encountered
None beyond the formatting fixup above.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- `AdDirectory`/`DirectoryResult`/`DirectoryError`/`MockAdDirectory`/`TtlCache<V>` are all compiling, unit-tested, and ready for Plan 31-02 (`RealAdDirectory` — the real LDAP service-bind adapter using these building blocks).
- No blockers. `cargo clippy -p trackly-core -p trackly-infra -- -D warnings` and `cargo fmt --check` both clean for all files this plan touched.

---
*Phase: 31-ad-bind-ad*
*Completed: 2026-08-03*
