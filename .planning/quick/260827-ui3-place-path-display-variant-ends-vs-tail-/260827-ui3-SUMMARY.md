---
phase: 260827-ui3
plan: 01
subsystem: ui
tags: [svelte5, rust, config, dto, place-path, reports]

requires: []
provides:
  - "PlacePathDisplay enum (Ends/LastTwo/Full) in OrganizationConfig, `place_path_display` TOML key"
  - "AuthStatusDto.place_path_display — boot-time value on both HTTP and Tauri transports"
  - "ui/src/lib/utils/placePath.ts — single shortenPlacePath()/normalizePlacePathDisplay() implementation, replaces two duplicated shorteners"
  - "authStore.placePathDisplay — global reactive variant state, populated in App.svelte::loadAuthStatus()"
affects: [reports, places, devices, cartridges]

tech-stack:
  added: []
  patterns:
    - "Field-local degrade-on-unknown-value deserializer (deserialize_with) instead of whole-file fail-closed, for config fields whose blast radius via config_recovery's whole-config fallback is disproportionate to the field's own stakes."

key-files:
  created:
    - ui/src/lib/utils/placePath.ts
  modified:
    - crates/trackly-infra/src/config.rs
    - trackly.config.toml.example
    - crates/trackly-infra/tests/config_test.rs
    - crates/trackly-app/src/dto/auth.rs
    - crates/trackly-app/src/http/auth.rs
    - crates/trackly-app/src/tauri_cmds/auth.rs
    - ui/src/lib/stores/auth.svelte.ts
    - ui/src/App.svelte
    - ui/src/features/reports/ReportTable.svelte
    - ui/src/features/places/PlaceContents.svelte
    - ui/src/features/devices/DeviceListRow.svelte
    - ui/src/features/cartridges/CartridgeListRow.svelte

key-decisions:
  - "Orchestrator amendment: unrecognized place_path_display degrades LOCALLY to Ends (custom deserialize_with + tracing::warn!), NOT via the whole-file config_recovery fallback used by ldap_tls_mode — a typo in this cosmetic field must not reset paths.db_path/server.enabled."
  - "place_path_display piggybacks on the existing AuthStatusDto (boot-time, both transports) instead of a new settings_get_* route, avoiding a 401-redirect risk before login."

requirements-completed: [UI3-01, UI3-02]

duration: ~25min (Tasks 1-2; Task 3 is a blocking human-verify checkpoint, not yet resolved)
completed: 2026-08-27
---

# Quick 260827-ui3: Configurable place-path display variant Summary

**Configurable `place_path_display` (ends/last_two/full) in `trackly.config.toml`, plumbed through
`AuthStatusDto` on both transports, and a single `shortenPlacePath()` util replacing two duplicated
implementations — applied in Reports, PlaceContents, DeviceListRow, and CartridgeListRow. Task 3
(human UAT) is a blocking checkpoint and has NOT been performed — this summary covers Tasks 1-2 only.**

## Performance

- **Duration:** ~25 min (Tasks 1-2 only)
- **Started:** 2026-08-27T15:07Z (approx, first commit 15:17:33+07:00 = 08:17:33Z)
- **Completed (Tasks 1-2):** 2026-08-27T15:19:38Z
- **Tasks:** 2/3 (Task 3 is `checkpoint:human-verify`, blocking — paused, not executed)
- **Files modified:** 13 (1 created, 12 modified)

## Accomplishments

- `PlacePathDisplay` enum (`Ends` default / `LastTwo` / `Full`) added to
  `OrganizationConfig.place_path_display`, `#[serde(default)]`, documented in
  `trackly.config.toml.example`.
- `AuthStatusDto.place_path_display: String` added and populated identically on both
  `build_auth_status` (HTTP) and `build_auth_status_tauri` (Tauri) from
  `ctx.config.organization.place_path_display.as_str()`.
- New `ui/src/lib/utils/placePath.ts` — the single frontend implementation of place-path
  shortening (`shortenPlacePath`, `normalizePlacePathDisplay`), replacing the two independent
  duplicates that used to live in `ReportTable.svelte` and `PlaceContents.svelte`.
- `authStore.placePathDisplay` (default `'ends'`) populated at boot in
  `App.svelte::loadAuthStatus()`, applied in all 4 target locations: Отчёты (`ReportTable`,
  via the existing `formatPlaceCell`/`compositeWith` mechanism — 260827-gim untouched),
  «Содержимое места» (`PlaceContents`), список Устройств (`DeviceListRow` — new application),
  список Картриджей (`CartridgeListRow` — new application). `title` stays the full path
  everywhere.

## Task Commits

1. **Task 1: Бэкенд — конфигурируемый вариант в trackly.config.toml + boot-time DTO** -
   `173cbbb3` (feat)
2. **Task 2: Фронтенд — единый shortenPlacePath, boot-time store, применение в 4 местах** -
   `7d1f29fe` (feat)
3. **Task 3: Human UAT** - NOT executed (blocking checkpoint, see below)

## Files Created/Modified

- `crates/trackly-infra/src/config.rs` - `PlacePathDisplay` enum + `OrganizationConfig.place_path_display` field with a custom `deserialize_with` fallback (locally degrades to `Ends` on unrecognized value, logs `tracing::warn!`)
- `trackly.config.toml.example` - documents the new `[organization] place_path_display` key, three values, degrade-locally behavior
- `crates/trackly-infra/tests/config_test.rs` - 5 new tests (missing section, partial section, `last_two`, `full`, bogus-value-degrades-locally-not-whole-file)
- `crates/trackly-app/src/dto/auth.rs` - `AuthStatusDto.place_path_display: String` + unit test update
- `crates/trackly-app/src/http/auth.rs` - `build_auth_status` fills the new field
- `crates/trackly-app/src/tauri_cmds/auth.rs` - `build_auth_status_tauri` fills the new field identically
- `ui/src/lib/utils/placePath.ts` (new) - `shortenPlacePath`, `normalizePlacePathDisplay`, `PlacePathDisplay` type
- `ui/src/lib/stores/auth.svelte.ts` - `authStore.placePathDisplay` state, default `'ends'`
- `ui/src/App.svelte` - `loadAuthStatus()` populates `authStore.placePathDisplay`
- `ui/src/features/reports/ReportTable.svelte` - local `shortPlacePath` removed, `formatCellDisplay` now passes `(p) => shortenPlacePath(p, authStore.placePathDisplay)` as `transformPath`
- `ui/src/features/places/PlaceContents.svelte` - local `shortPath` removed, cell uses `shortenPlacePath`
- `ui/src/features/devices/DeviceListRow.svelte` - «Место» cell now shortened (new application), `title` unchanged
- `ui/src/features/cartridges/CartridgeListRow.svelte` - «Место» cell now shortened (new application), `title` unchanged

## Decisions Made

- **Orchestrator amendment applied verbatim (deviation from PLAN.md's stated design):** the plan's
  design note said an unrecognized `place_path_display` value should fail the whole TOML file,
  mirroring `ldap_tls_mode`'s precedent. The orchestrator scoped this down: `config_recovery::
  load_or_recover`'s whole-config fallback resets `paths.db_path`/`server.enabled` too, so a typo
  in this purely cosmetic field could silently switch databases or stop the LAN server. Implemented
  instead as a field-local `deserialize_with` function (`deserialize_place_path_display`) that
  degrades to `PlacePathDisplay::Ends` and emits `tracing::warn!` naming the offending value and the
  accepted ones — the rest of the config (including sibling sections like `[server]`) still parses
  normally. No other field's strictness (including `ldap_tls_mode` itself) was touched.
- `AuthStatusDto` (existing boot-time DTO, both transports) was reused rather than adding a new
  `settings_get_place_path_display` route, per the plan's design note — avoids a round-trip and a
  401-redirect risk on a session-gated route called before login.

## Deviations from Plan

### Auto-fixed Issues

**1. [Orchestrator amendment, treated as Rule 2 — correctness/blast-radius] Local degrade instead of whole-file fail-closed for `place_path_display`**
- **Found during:** Task 1 planning (pre-empted by explicit orchestrator instruction before execution started)
- **Issue:** Plan's original design mirrored `ldap_tls_mode`'s fail-closed precedent — an unrecognized value would fail the whole TOML file and fall through to `config_recovery`'s `AppConfig::default()` fallback, which also resets `paths.db_path` and `server.enabled`.
- **Fix:** Implemented a field-local `deserialize_with` (`deserialize_place_path_display`) that degrades only this field to `Ends` on an unrecognized value, with a `tracing::warn!`, leaving the rest of the config (including `[server]`) intact.
- **Files modified:** `crates/trackly-infra/src/config.rs`, `crates/trackly-infra/tests/config_test.rs`, `trackly.config.toml.example`
- **Verification:** New test `test_11_place_path_display_bogus_value_degrades_locally_not_whole_file` asserts a config with a bogus `place_path_display` AND a non-default `[server]` section still parses, keeps `server.enabled=true`/`server.host`/`server.port` as configured, and only `place_path_display` falls back to `Ends`.
- **Committed in:** `173cbbb3` (Task 1 commit)

---

**Total deviations:** 1 (orchestrator amendment, applied as instructed — not an autonomous Rule 1-3 fix)
**Impact on plan:** Reduces blast radius of a cosmetic-field typo; no scope creep beyond the single field's deserialization path. Threat register entry T-260827-ui3-01 in PLAN.md describes the original fail-closed design — the actual implementation is the amended local-degrade behavior documented above and in the `config.rs` doc comments.

## Issues Encountered

- **Pre-existing flaky test, out of scope:** `crates/trackly-app/tests/users_crud.rs::users_update_password_change` hit its internal 30s timeout budget when run as part of the full `cargo test -p trackly-app -- --skip login_remember_persistent_cookie` sweep (argon2 hashing contends with ~90 other test binaries running concurrently). Passes standalone in ~13.6s. Not touched by this task's files (config/DTO/frontend place-path wiring only — no password/auth-timing code touched). Logged in `.planning/quick/260827-ui3-place-path-display-variant-ends-vs-tail-/deferred-items.md`, not fixed (scope-boundary rule).

## Verification Performed (Tasks 1-2)

- `cargo test -p trackly-infra --test config_test --test config_example_test` — 11 + 2 = 13 passed, 0 failed.
- `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test export_bindings` — passed; `ui/src/bindings.ts` regenerated with `place_path_display: string` in `AuthStatusDto`.
- `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test auth_smoke -- --test-threads=1` — 6 passed, 0 failed.
- `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app -- --skip login_remember_persistent_cookie` (full package, single invocation) — 91/92 test binaries green; 1 flaky failure isolated to `users_update_password_change` (see Issues Encountered above; confirmed pre-existing/unrelated, passes standalone). All 4 binaries that didn't run due to cargo's fail-fast stop (`users_http_camelcase`, `ws_broadcast_fanout`, `ws_http_single_broadcast`, `ws_upgrade_serve_connection`) were run separately afterward — all green.
- `cargo fmt --check` — clean.
- `cargo clippy -p trackly-infra -p trackly-app --all-targets -- -D warnings` — clean.
- `pnpm --dir ui exec svelte-check --tsconfig ./tsconfig.json` — 0 errors, 57 pre-existing warnings in unrelated files.
- `pnpm --dir ui lint` — clean (eslint, prettier, tokens/contrast/focus-outline/pagedjs-csp-hash/print-isolation checks all pass).
- `pnpm --dir ui build` — succeeded, `ui/dist` rebuilt (required for LAN-browser verification in Task 3).
- Grepped `ui/src` for leftover `shortPlacePath`/`shortPath(` — none found outside a comment in `ReportTable.svelte`; both duplicate implementations fully removed.

**NOT verified (requires Task 3 live UAT):** actual rendering in a running app/browser — svelte-check/lint/build do not prove Svelte 5 rune runtime behavior (per project CLAUDE.md memory on this exact limitation).

## Known Stubs

None.

## Threat Flags

None beyond what PLAN.md's `<threat_model>` already covers (T-260827-ui3-01/02/03) — no new
network endpoints, auth paths, or trust-boundary changes introduced. Note that the *disposition*
described for T-260827-ui3-01 in PLAN.md's threat register table (whole-file fail-closed) was
superseded by the orchestrator amendment described above; the mitigation is still "mitigate"
(fails safe, doesn't panic, doesn't silently accept bad input) but the mechanism is now a
field-local warn+degrade rather than a whole-config fallback.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

Tasks 1-2 are code-complete, committed, and pass all automated gates. **Task 3 (blocking
`checkpoint:human-verify`) has not been run** — the orchestrator/user must perform the live UAT
described in `260827-ui3-PLAN.md`'s Task 3 (`<how-to-verify>`) before this quick task can be
considered done. See the "CHECKPOINT REACHED" block returned alongside this summary for the
exact verification steps.

---
*Quick task: 260827-ui3*
*Tasks 1-2 completed: 2026-08-27 (Task 3 pending)*

## Self-Check: PASSED

All 13 files listed under "Files Created/Modified" verified present on disk (`FOUND`, none
`MISSING`). Both task commits verified in `git log`: `173cbbb3` (Task 1), `7d1f29fe` (Task 2).
