---
phase: 09-ad
plan: 02
subsystem: auth

tags: [ldap3, rusqlite, argon2, app-settings, ad-fallback]

# Dependency graph
requires:
  - phase: 09-ad (plan 01)
    provides: AdClient trait + AuthOutcome enum (trackly-core), MockAdClient/RealAdClient/discovery (trackly-infra), AdConfig (bootstrap config)
provides:
  - AuthService.login() local→AD fallback (try_local_login + try_ad_login), preserving the CR-05 constant-time dummy-hash path
  - find_user_any_state(login) read seam (active/blocked/deleted/unknown discrimination)
  - on_ad_bind_success() active-user happy path (blocked/deleted/unknown left as typed TODO for plan 03)
  - ad_enabled / ad_auto_accept app_settings readers+writers (ManageSettings-gated)
  - AppError::ServiceUnavailable variant (503) — distinct from Unauthorized for AD-unreachable
  - V028 migration: requests.ad_subtype discriminator column (D-REG-03)
  - context.rs TRACKLY_AD_MOCK / config.ad.use_mock runtime switch, injecting Arc<dyn AdClient> into AuthService
affects: [09-ad (plan 03 — registration/restoration write paths, unknown/blocked branches)]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Local-then-AD login fallback: try_local_login distinguishes UnknownLogin (eligible for AD fallback) from BadPassword (never falls back — avoids a second timing oracle for known logins)"
    - "AD-only user rows have password_hash=NULL; get_password_hash query now filters password_hash IS NOT NULL so NULL doesn't surface as a column-type error instead of QueryReturnedNoRows"
    - "AppError variant additions are additive/non-breaking for specta bindings (AppErrorRepr is a fixed {code,message,details} shape — no per-variant TS type needed)"

key-files:
  created:
    - migrations/V028__ad_register_subtype.sql
    - crates/trackly-app/tests/ad_auth.rs
    - .planning/phases/09-ad/deferred-items.md
  modified:
    - crates/trackly-app/src/services/auth.rs
    - crates/trackly-app/src/context.rs
    - crates/trackly-core/src/error.rs
    - crates/trackly-app/src/error_axum.rs
    - crates/trackly-app/src/http/health.rs
    - crates/trackly-app/src/tauri_cmds/health.rs
    - crates/trackly-app/tests/auth_smoke.rs
    - crates/trackly-app/tests/users_crud.rs
    - crates/trackly-app/tests/specta_roundtrip.rs
    - crates/trackly-infra/src/test_support/test_db.rs
    - crates/trackly-infra/tests/migration_idempotency.rs

key-decisions:
  - "AD fallback only triggers on UnknownLogin, never on BadPassword for a known local login — prevents a local user with a forgotten password from accidentally authenticating via AD bind under the same login name"
  - "Added AppError::ServiceUnavailable{service} instead of overloading WriteQueueBusy — AD-unreachable is a distinct infra fault from db write contention, and the UI needs to show 'AD недоступен' specifically"
  - "find_user_any_state and on_ad_bind_success scoped to active-user-only for this plan; blocked/deleted/unknown branches return a typed TODO referencing plan 03 rather than stubbing a v1-simplified user creation"
  - "get_password_hash query gained an explicit password_hash IS NOT NULL filter (Rule 1 bug fix) — without it, AD-only users (NULL hash) would hit InvalidColumnType instead of QueryReturnedNoRows, breaking the fallback path entirely"

patterns-established:
  - "Settings mock/real runtime switch (TRACKLY_<X>_MOCK env or config.<x>.use_mock) hoisted before the consuming service's constructor call when the service needs the client at construction time, vs. after when only a later-constructed service needs it"

requirements-completed: [USR-08, USR-10]

duration: ~75min
completed: 2026-06-19
---

# Phase 9 Plan 2: AD client wiring + login fallback Summary

**AuthService now does local→AD login fallback (try_local_login/try_ad_login) with constant-time anti-enumeration preserved, a find_user_any_state read seam, ad_enabled/ad_auto_accept settings, and a V028 ad_subtype migration column for the upcoming registration flow.**

## Performance

- **Duration:** ~75 min
- **Started:** 2026-06-19T16:18:00Z (approx, prior session)
- **Completed:** 2026-06-19T17:33:19Z
- **Tasks:** 2/2 completed
- **Files modified:** 14 (3 created, 11 modified)

## Accomplishments
- `login()` tries local argon2id auth first (dummy-hash constant-time path untouched), then falls back to `AdClient.authenticate()` only when the local login is genuinely unknown and `ad_enabled=true`
- Pitfall 1 (RFC 4513 §5.1.2 anonymous-bind trap) closed at the service layer: empty/whitespace password rejected before any AD bind attempt, proven by `empty_password_rejected` test
- `AuthOutcome::Unreachable` now surfaces as `AppError::ServiceUnavailable{service:"ad"}` — a distinct, UI-mappable error instead of generic `Unauthorized`
- `find_user_any_state(login)` read seam lets post-bind logic distinguish active / blocked / soft-deleted / unknown without the active-only filter `get_by_login` uses
- V028 migration adds `requests.ad_subtype` (nullable, no CHECK rebuild) per D-REG-03, ready for plan 03's register-vs-restore discriminator
- `context.rs` wires `TRACKLY_AD_MOCK` / `config.ad.use_mock` exactly mirroring the existing SNMP mock switch, hoisted before `AuthService::new()` since auth needs the client at construction time

## Task Commits

Each task was committed atomically:

1. **Task 1: V028 ad_subtype migration + AuthService AD wiring + settings readers** - `ab6b390` (feat)
2. **Task 2: login() local→AD fallback + find_user_any_state + on_ad_bind_success** - `d8bb288` (feat)
3. **Fix: migration_idempotency hardcoded count** - `c31bb3c` (fix, Rule 1 — direct consequence of V028)

**Plan metadata:** (pending — final docs commit below)

## Files Created/Modified
- `migrations/V028__ad_register_subtype.sql` - adds `requests.ad_subtype TEXT NULL`, bumps `PRAGMA user_version` to 28
- `crates/trackly-app/tests/ad_auth.rs` - 5 integration tests: empty/whitespace password trap, active AD fallback, AD-disabled no-op, Unreachable distinct error, local-login regression
- `crates/trackly-app/src/services/auth.rs` - `AuthService.ad_client` field; `try_local_login`/`try_ad_login`/`on_ad_bind_success`; `find_user_any_state`; `ad_enabled`/`set_ad_enabled`/`ad_auto_accept`/`set_ad_auto_accept`; `get_password_hash` NULL-hash fix
- `crates/trackly-app/src/context.rs` - AD mock/real runtime switch, injected into `AuthService::new`
- `crates/trackly-core/src/error.rs` - `AppError::ServiceUnavailable{service}` variant + `code()`/`details_value()`/serialize tests
- `crates/trackly-app/src/error_axum.rs` - 503 mapping for `ServiceUnavailable`
- `crates/trackly-app/src/http/health.rs`, `tauri_cmds/health.rs` - test-only `minimal_ctx()` builders updated for the new `AuthService::new` arity
- `crates/trackly-app/tests/auth_smoke.rs`, `users_crud.rs`, `specta_roundtrip.rs` - test fixtures updated for the new `ad_client` constructor param
- `crates/trackly-infra/src/test_support/test_db.rs` - `user_version` assertion bumped 27→28
- `crates/trackly-infra/tests/migration_idempotency.rs` - switched hardcoded `27` to dynamic `migrations::max_known_version()`
- `.planning/phases/09-ad/deferred-items.md` - logged 2 pre-existing, out-of-scope failures discovered during verification

## Decisions Made
- AD fallback gated strictly on `UnknownLogin`, not `BadPassword`, to avoid creating a second enumeration/confusion vector for users who exist locally but mistype their password.
- Added a dedicated `AppError::ServiceUnavailable{service: &'static str}` variant rather than reusing `WriteQueueBusy` — semantically distinct infra fault, and the `service` field lets the same variant cover future external-service-unavailable cases beyond AD.
- `on_ad_bind_success` deliberately scoped to active-user-only for this plan; blocked/deleted/unknown branches return `Err(AppError::Unauthorized)` with an explicit `TODO(09-03)` comment rather than a "simplified" registration stub — full write paths land in plan 03 as specified.
- `get_password_hash`'s `SELECT` gained `AND password_hash IS NOT NULL` — without it, an AD-only user row (created in plan 03, seeded directly in this plan's tests) would have surfaced `rusqlite::Error::InvalidColumnType` instead of `QueryReturnedNoRows`, silently breaking the entire AD-fallback path. Classified as Rule 1 (bug fix), since `try_local_login`'s match only special-cased `AppError::Unauthorized`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] get_password_hash crashes (InvalidColumnType) on AD-only user rows instead of returning Unauthorized**
- **Found during:** Task 2 (writing `ad_fallback_active_user` test, seeding an AD-only user with `password_hash = NULL`)
- **Issue:** `get_password_hash`'s query had no `password_hash IS NOT NULL` filter; for a row with `password_hash = NULL`, `row.get::<_, String>(0)` returns `Err(InvalidColumnType)`, which `try_local_login` did not special-case (only `AppError::Unauthorized` triggers the dummy-hash/AD-fallback path) — so AD-only users could never reach the AD fallback branch, defeating the entire feature for the one user type it's meant to serve.
- **Fix:** Added `AND password_hash IS NOT NULL` to the `SELECT password_hash FROM users WHERE ...` query so AD-only rows correctly produce `QueryReturnedNoRows` → `AppError::Unauthorized` → `LocalLoginOutcome::UnknownLogin` → AD fallback eligible.
- **Files modified:** `crates/trackly-app/src/services/auth.rs`
- **Verification:** `ad_fallback_active_user` test passes; `local_user_still_works` regression test confirms local (non-NULL-hash) users are unaffected.
- **Committed in:** `d8bb288` (Task 2 commit)

**2. [Rule 1 - Bug] migration_idempotency.rs hardcoded migration count broken by V028**
- **Found during:** Task 2 plan-level verification (`cargo test -p trackly-infra`, run proactively beyond the plan's specified `cargo test -p trackly-app --test ad_auth` to catch cross-crate regressions)
- **Issue:** `migrations_are_idempotent_and_wal_persists_across_reopens` hardcoded `applied_count == 27` / `schema_version == 27`; adding V028 broke it (`left: 28, right: 27`).
- **Fix:** Replaced the hardcoded `27` with `migrations::max_known_version()`, mirroring the dynamic pattern already used by `health_smoke.rs` and `test_db.rs` — prevents this test from breaking on every future migration addition.
- **Files modified:** `crates/trackly-infra/tests/migration_idempotency.rs`
- **Verification:** `cargo test -p trackly-infra` — all 67+ tests green (2 pre-existing ignored tests unaffected).
- **Committed in:** `c31bb3c` (standalone fix commit, since it's a cross-crate consequence of the V028 migration rather than part of either task's core deliverable)

---

**Total deviations:** 2 auto-fixed (both Rule 1 — bugs directly caused by this plan's own changes)
**Impact on plan:** Both fixes were necessary for the plan's stated `must_haves` to actually hold (AD fallback for active users genuinely working; migration count assertions not flaking on every future schema change). No scope creep — no unrelated files touched beyond what V028/the AD wiring required.

## Issues Encountered

Two pre-existing, out-of-scope test failures were discovered during verification and are NOT fixed in this plan (logged to `.planning/phases/09-ad/deferred-items.md` per the executor scope-boundary rule):

1. **`graceful_shutdown_drain` test binary** — both tests panic with a rustls `CryptoProvider` installation error. Reproduced identically on a clean `git stash` (pre-dates this plan). Touches Phase 7/8 server-mode TLS bring-up, unrelated to AD auth.
2. **`template_service.rs` clippy::len_zero under `--tests`** — two `assert!(bytes.len() > 0, ...)` calls trip `clippy::len_zero` only when running `cargo clippy --tests` (not the plan's specified bare `cargo clippy -p trackly-app -- -D warnings`, which is clean). Reproduced identically on a clean `git stash`. One-line fix deferred to a future cleanup pass.

## Known Stubs

`on_ad_bind_success` intentionally returns `Err(AppError::Unauthorized)` for the blocked/deleted and unknown-in-DB branches (`crates/trackly-app/src/services/auth.rs`, `TODO(09-03)` comments at both arms). This is documented in-plan as deferred scope, not an unintentional stub — plan 09-03 implements the `ad_register` request-creation write paths (register + restore subtypes) that these branches will call instead of returning an error. The V028 `ad_subtype` column exists specifically to support that follow-up work.

## Threat Flags

No new surface beyond what the plan's `<threat_model>` already covers (T-09-06 through T-09-09 — all addressed: empty-password reject before bind, constant-time dummy-hash preserved, `Secret<String>` wrap for the AD bind password, `find_user_any_state` preventing silent re-admit of blocked/deleted users).

## User Setup Required

None - no external service configuration required. `TRACKLY_AD_MOCK=1` (or `config.ad.use_mock = true`) continues to work with zero AD infrastructure for dev/test on macOS.

## Next Phase Readiness

- Service-layer AD login fallback is complete and tested end-to-end with the mock fixtures (`us100`/`Passw0rd!` → `Иванов Иван Иванович`).
- Plan 09-03 can build directly on `find_user_any_state`, `on_ad_bind_success`'s TODO branches, `ad_auto_accept()`, and the `ad_subtype` column to implement the registration/restoration request write paths and the unknown/blocked branches.
- No blockers. The two deferred pre-existing failures are independent of plan 03's scope.

---
*Phase: 09-ad*
*Completed: 2026-06-19*
