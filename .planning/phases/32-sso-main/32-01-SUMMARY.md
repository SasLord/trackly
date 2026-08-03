---
phase: 32-sso-main
plan: 01
subsystem: auth
tags: [rust, serde, toml, ad, sso, config]

# Dependency graph
requires:
  - phase: 31-ad-bind-ad
    provides: "AdConfig.role_mapping field + Debug/Default wiring pattern to mirror"
provides:
  - "AdConfig.admin_logins: Vec<String> field, #[serde(default)], defaults to empty"
  - "Documented trackly.config.toml.example admin_logins block with security warning"
affects: [32-02-provision-forced-admin, 32-sso-main-merge-release]

# Tech tracking
tech-stack:
  added: []
  patterns: ["Config field mirrors role_mapping exactly (flat Vec<String> instead of array-of-tables)"]

key-files:
  created: []
  modified:
    - crates/trackly-infra/src/config.rs
    - trackly.config.toml.example

key-decisions:
  - "admin_logins stored as flat TOML string array (no [[ad.admin_logins]] table-array needed) since it's Vec<String>, not Vec<RoleMappingEntry> (D-01)"
  - "Debug-printed unredacted (no secrets, unlike bind_password) per D-02/T-32-02 disposition"

patterns-established:
  - "Config field addition mirrors an existing analog field's struct/Default/Debug/test quartet exactly, minimizing review surface"

requirements-completed: [SSO-02]

# Metrics
duration: ~35min
completed: 2026-08-03
---

# Phase 32 Plan 01: Admin-logins Config Field Summary

**AdConfig.admin_logins: Vec<String> field added (config/parsing layer only) — deployment-time TOML source of truth for the auto-admin-by-login-list feature (SSO-02), mirroring the existing role_mapping pattern.**

## Performance

- **Duration:** ~35 min
- **Started:** 2026-08-03T16:57:23Z
- **Completed:** 2026-08-03T17:31:57Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments
- Added `admin_logins: Vec<String>` field to `AdConfig` with `#[serde(default)]`, wired into the manual `Debug` impl (unredacted — no secrets) and `Default` impl
- Added parsing test `admin_logins_flat_array_deserializes_and_defaults_empty` covering both the empty-default case and the populated flat-array case; extended two existing backward-compat tests to also assert `admin_logins` defaults to empty
- Documented the field in `trackly.config.toml.example` with a security warning (bypasses `ad_auto_accept`, pending registration, and manual block/soft-delete per D-07) and a restart-required note (no live reload)

## Task Commits

Each task was committed atomically:

1. **Task 1: Add admin_logins field to AdConfig with parsing tests** - `7df340e` (feat)
2. **Task 2: Document admin_logins in trackly.config.toml.example** - `8a46fda` (docs)

**Plan metadata:** (pending — final docs commit below)

## Files Created/Modified
- `crates/trackly-infra/src/config.rs` - New `admin_logins: Vec<String>` field on `AdConfig` (struct, Debug, Default, tests)
- `trackly.config.toml.example` - New documented `admin_logins` block after `role_mapping`, with security warning + restart-required note

## Decisions Made
- Followed 32-CONTEXT.md D-01/D-02/D-03/D-09 exactly: TOML-config storage (not DB/UI), flat string array, case-insensitive matching deferred to Plan 02's `is_admin_login` helper, empty/absent = feature off
- No new dependency, no new parsing mechanism — pure field addition mirroring `role_mapping`'s established quartet (field/Debug/Default/tests)

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required. `admin_logins` remains unset by default in `trackly.config.toml.example` (commented out); operators opt in explicitly per D-03.

## Next Phase Readiness

- `AdConfig.admin_logins` compiles, deserializes, defaults to empty, and is ready for Plan 02 to consume via `config.ad.admin_logins.clone()` when wiring `AuthService::with_admin_logins(...)` in `context.rs`.
- No blockers for Plan 02 (Wave 2 runtime provisioning logic).

---
*Phase: 32-sso-main*
*Completed: 2026-08-03*
