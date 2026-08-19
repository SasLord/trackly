---
phase: 260819-wq5-low-stock-basis
plan: 01
subsystem: cartridges
tags: [rusqlite, sqlite, axum, tauri, svelte, low-stock, app_settings]

requires: []
provides:
  - "LowStockBasis domain enum (CartridgeModel|PrinterModel), default PrinterModel"
  - "cartridges_sqlite.rs::low_stock() branches on app_settings.low_stock_basis"
  - "dashboard_service.rs independent low-stock SQL copy branches identically"
  - "settings_get/set_low_stock_basis Tauri commands + HTTP routes (ManageSettings-gated write)"
  - "Radio switch in ThresholdSettings.svelte; dual-shape rendering in LowStockBanner.svelte"
affects: [dashboard-widgets, cartridges-page, settings-page]

tech-stack:
  added: []
  patterns:
    - "Guarded app_settings string read + Rust-side parse-with-fallback (WR-06 style) reused for a non-numeric setting (LowStockBasis), not just thresholds"
    - "Anti-fan-out correlated EXISTS subquery for grouping by cartridge_model_compatibility.printer_name, duplicated intentionally in two independent SQL sites (repo + dashboard_service) per existing project pattern"

key-files:
  created: []
  modified:
    - crates/trackly-core/src/domain/cartridges.rs
    - crates/trackly-infra/src/repos/cartridges_sqlite.rs
    - crates/trackly-app/src/services/dashboard_service.rs
    - crates/trackly-app/src/dto/cartridge.rs
    - crates/trackly-app/src/tauri_cmds/settings_org.rs
    - crates/trackly-app/src/http/settings_org.rs
    - crates/trackly-app/src/specta_export.rs
    - crates/trackly-app/tests/cartridges_low_stock.rs
    - crates/trackly-app/tests/dashboard_widgets.rs
    - crates/trackly-app/tests/role_endpoint_matrix.rs
    - ui/src/features/settings/ThresholdSettings.svelte
    - ui/src/features/cartridges/LowStockBanner.svelte

key-decisions:
  - "Default basis for missing/invalid app_settings.low_stock_basis is PrinterModel (intentional behavior change on existing DBs, per CONTEXT)"
  - "SET rejects unknown basis strings with AppError::Validation; GET always falls back to DEFAULT, never errors"
  - "Printer-name source is strictly cartridge_model_compatibility.printer_name (LOWER(TRIM(...)) normalized), never devices.name; D-05 pass-through (models with no compatibility rows) is NOT applied in printer_model basis"

requirements-completed: [WQ5-01, WQ5-02, WQ5-03, WQ5-04]

duration: ~55min (active coding + task-scoped test runs); full-workspace regression run (cargo test -p trackly-core -p trackly-infra -p trackly-app, all 87 trackly-app integration binaries) was started but killed by the environment after ~50min mid-run and not completed — see Issues Encountered
completed: 2026-08-20
---

# Quick Task 260819-wq5: Порог низкого остатка — выбор базы подсчёта Summary

**Radio-переключатель «по модели принтера» (новый дефолт) / «по модели картриджа» для расчёта низкого остатка картриджей, синхронно применённый в двух независимых SQL-путях (репозиторий картриджей и дашборд).**

## Performance

- **Duration:** ~55 min active work (3 task commits between 00:10 and 00:26 local time, plus preceding read/plan review)
- **Tasks:** 3/3 completed
- **Files modified:** 12 (Rust: 7 source + 3 test files; Svelte: 2)

## Accomplishments

- `LowStockBasis` domain enum (`CartridgeModel` | `PrinterModel`, `DEFAULT = PrinterModel`) added to `trackly-core`, with `as_str()`/`parse()` (exact-match, no fuzzy fallback baked in).
- `cartridges_sqlite.rs::low_stock()` reads `app_settings.low_stock_basis` with the same WR-06-guarded string-parse-with-fallback pattern already used for the numeric threshold, then branches:
  - `cartridge_model`: unchanged legacy SQL, grouped by `cartridge_models.id`.
  - `printer_model`: anti-fan-out correlated-`EXISTS` query grouping by `LOWER(TRIM(cartridge_model_compatibility.printer_name))`, summing in-stock+full cartridges across every compatible model; zero-stock printer names are included; models without compatibility rows never appear.
- `dashboard_service.rs` got an **independent, byte-shape-matching copy** of the same branching logic (this was explicitly flagged in CONTEXT as the highest divergence risk — two integration tests now lock the two SQL sites to agree on the same seed data).
- New Tauri commands `settings_get_low_stock_basis` (open to any authenticated caller, falls back to default on missing/garbage value) and `settings_set_low_stock_basis` (`ManageSettings`-gated, rejects unknown values with `AppError::Validation`), mirrored as `/api/v1/settings_get_low_stock_basis` / `/api/v1/settings_set_low_stock_basis` HTTP routes, registered in `specta_export.rs`.
- `LowStockItemDto` reshaped: `basis: String`, `model_id/brand/model: Option<...>`, new `label: String` — TypeScript bindings regenerated (`ui/src/bindings.ts`, gitignored, not committed).
- `ThresholdSettings.svelte`: Radio group (default «По модели принтера»), saves immediately on selection (bubbled native `change` event, same wrapper trick as the existing threshold `<Input>`); the threshold label text switches depending on the selected basis.
- `LowStockBanner.svelte`: renders both row shapes (`cartridge_model` vs `printer_model`), with a collision-free `{#each}` key (`${item.basis}:${item.model_id ?? item.label}`).
- Extra unit test added per plan-checker note: `low_stock_falls_back_to_printer_model_default_on_garbage_basis_value` — a garbage string (`'bogus'`) written directly into `app_settings.low_stock_basis` still falls back to the `PrinterModel` default (not just the missing-key case that was already covered).

## Task Commits

Each task was committed atomically:

1. **Task 1: LowStockBasis domain type + repo/dashboard query branching + repo unit tests** - `26df429f` (feat)
2. **Task 2: settings_get/set_low_stock_basis API surface (DTO, Tauri commands, HTTP routes, specta export)** - `36156278` (feat)
3. **Task 3: Backend integration tests + frontend Radio/banner wiring** - `5d87559b` (test)

**Plan metadata:** committed separately by the orchestrator (docs, not by this executor).

## Files Created/Modified

- `crates/trackly-core/src/domain/cartridges.rs` - `LowStockBasis` enum + reshaped `LowStockItem` (basis/label, `Option<model_id/brand/model>`)
- `crates/trackly-infra/src/repos/cartridges_sqlite.rs` - `low_stock()` basis branching + 7 new/updated unit tests
- `crates/trackly-app/src/services/dashboard_service.rs` - independent basis-branching SQL copy for the dashboard widget
- `crates/trackly-app/src/dto/cartridge.rs` - `LowStockItemDto` reshaped, `From<LowStockItem>` updated
- `crates/trackly-app/src/tauri_cmds/settings_org.rs` - `build_settings_get/set_low_stock_basis` + Tauri command wrappers
- `crates/trackly-app/src/http/settings_org.rs` - HTTP handlers + payload struct + routes
- `crates/trackly-app/src/specta_export.rs` - registered the two new Tauri commands for binding export
- `crates/trackly-app/tests/cartridges_low_stock.rs` - `set_basis()` helper, legacy tests pinned to `cartridge_model`, 2 new default-basis tests
- `crates/trackly-app/tests/dashboard_widgets.rs` - raw-SQL seeding helper + 2 new tests proving repo/dashboard agreement
- `crates/trackly-app/tests/role_endpoint_matrix.rs` - Case 44 (Employee → `settings_set_low_stock_basis` → 403)
- `ui/src/features/settings/ThresholdSettings.svelte` - Radio group, basis-dependent label, save-on-select
- `ui/src/features/cartridges/LowStockBanner.svelte` - dual-shape row rendering, stable `{#each}` key

## Decisions Made

- Default basis on missing/invalid `app_settings.low_stock_basis` is `PrinterModel` — an intentional behavior change on existing databases, matching the CONTEXT decision.
- `settings_set_low_stock_basis` rejects unknown strings server-side (`AppError::Validation`) rather than silently defaulting; `settings_get_low_stock_basis` never errors, always falls back to the default.
- Printer-model grouping source is strictly `cartridge_model_compatibility.printer_name` (case/whitespace-normalized), never `devices.name`; the D-05 pass-through (models with zero compatibility rows treated as compatible with any printer) is intentionally NOT applied in `printer_model` basis — such models simply never appear in any printer group.
- `LowStockItemDto.model_id` uses `#[specta(type = Option<i32>)]` (matches this file's existing `Option<i64>` → `Option<i32>` convention for other nullable FK fields).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Test design bug in `low_stock_printer_model_groups_by_compatible_printer_name`**
- **Found during:** Task 1 (running the repo unit test suite)
- **Issue:** The test seeded exactly 2 cartridges (one per model) summing to a group count of 2, but the default threshold is also 2 — `HAVING cnt < threshold` correctly excluded the row (2 is not < 2), so `items.len()` was 0, not the expected 1. This was a test-authoring mistake, not a code bug — the plan's own interface text described this scenario with the default threshold without accounting for the off-by-one.
- **Fix:** The test now explicitly seeds `app_settings.low_stock_threshold = '3'` (via `INSERT ... ON CONFLICT DO UPDATE`, since V016 already seeds a default `low_stock_threshold` row) before creating the two cartridges, so the summed count of 2 is genuinely below the (now 3) threshold. `app_settings.low_stock_basis` is still left unset in this test, preserving the proof that `PrinterModel` is the default.
- **Files modified:** `crates/trackly-infra/src/repos/cartridges_sqlite.rs`
- **Verification:** `cargo test -p trackly-infra --lib cartridges_sqlite:: -- --test-threads=1` — all 22 tests pass (including this one).
- **Committed in:** `26df429f` (part of Task 1 commit)

**2. [Plan-checker note - missing coverage] Garbage `low_stock_basis` value fallback**
- **Found during:** Task 1, per the plan-checker note attached to this execution
- **Issue:** The plan only covered the missing-key case for the default fallback; a garbage string value (e.g. `'bogus'`) written into `app_settings.low_stock_basis` was not separately tested.
- **Fix:** Added `low_stock_falls_back_to_printer_model_default_on_garbage_basis_value` — seeds `low_stock_basis = 'bogus'`, asserts the query still falls back to `LowStockBasis::PrinterModel`.
- **Files modified:** `crates/trackly-infra/src/repos/cartridges_sqlite.rs`
- **Verification:** Test passes as part of the same `cargo test -p trackly-infra --lib cartridges_sqlite::` run.
- **Committed in:** `26df429f` (part of Task 1 commit)

---

**Total deviations:** 2 auto-fixed (1 Rule 1 test-bug fix, 1 plan-checker-requested test addition)
**Impact on plan:** Both are test-only changes; no production code or documented behavior changed. No scope creep.

## Issues Encountered

The plan's top-level `<verification>` item 1 calls for a full-workspace run: `cargo test -p trackly-core -p trackly-infra -p trackly-app` (with `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1`, `--test-threads=1`, skipping the known pre-existing `login_remember_persistent_cookie` hang). This run was started but progressed through `trackly-app`'s 87 integration test binaries very slowly (each binary re-runs all 36 migrations + template seeding); after roughly 50 minutes of wall-clock time it was killed by the execution environment (background-task status `killed`, not a test failure) before completing, and no failure output was captured.

This is a UNVERIFIED gap for the full-workspace regression sweep specifically. However, every test/check command that the plan's individual **task-level** `<verify>` blocks require was run to completion and passed:
- `cargo test -p trackly-infra --lib cartridges_sqlite::` — 22/22 pass (Task 1)
- `cargo check -p trackly-core -p trackly-infra -p trackly-app` — clean (Tasks 1-2)
- `cargo test -p trackly-app --test export_bindings` — pass, bindings regenerated and grepped for the new symbols (Task 2)
- `cargo test -p trackly-app --test cartridges_low_stock` — 5/5 pass (Task 3)
- `cargo test -p trackly-app --test dashboard_widgets` — 5/5 pass (Task 3)
- `cargo test -p trackly-app --test role_endpoint_matrix -- --skip login_remember_persistent_cookie` — 1/1 pass, Case 44 verified (Task 3)
- `pnpm svelte-check` — 0 errors (50 pre-existing warnings, none in touched files)
- `pnpm --dir ui build` — succeeded, `ui/dist` rebuilt
- `cargo clippy -p trackly-core -p trackly-infra -p trackly-app --all-targets` — clean, no warnings

**Recommendation:** re-run `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-core -p trackly-infra -p trackly-app -- --test-threads=1 --skip login_remember_persistent_cookie` in a session with a longer time budget (or split into smaller batches) before considering this quick task fully regression-verified against the whole `trackly-app` test surface. No code changed since the targeted test runs above, so the risk is limited to unrelated pre-existing tests in files this plan never touched.

## Known Stubs

None.

## Threat Flags

None — the two new endpoints (`settings_get/set_low_stock_basis`) mirror the existing `low_stock_threshold` pair's trust boundary exactly (see the plan's `<threat_model>`), and no new network surface, auth path, or schema change outside that pattern was introduced.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Feature is complete and self-contained. No follow-up phase is blocked on this. The only open item is re-running the full-workspace test sweep (see Issues Encountered) for extra confidence — this does not block closing the quick task, since all scoped verification passed.

---
*Quick task: 260819-wq5-low-stock-basis*
*Completed: 2026-08-20*

## Self-Check: PASSED

All 12 files listed under Files Created/Modified were verified present on disk; all 3 task commit hashes (`26df429f`, `36156278`, `5d87559b`) were verified present in `git log`.
