---
phase: 2
slug: ui
status: ready
nyquist_compliant: true
wave_0_complete: false
created: 2026-05-25
---

# Phase 2 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Populated from `02-RESEARCH.md` Validation Architecture and the established Phase 1 test patterns. Per-task table is filled by the planner once `*-PLAN.md` files exist.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` (Rust workspace) + `pnpm svelte-check` + `pnpm lint` (UI) + manual `pnpm tauri dev` smoke for UI interactivity |
| **Config file** | inherits Phase 1 — `Cargo.toml` workspace, `ui/package.json` |
| **Quick run command** | `cargo test --workspace --no-fail-fast -- --test-threads=1` |
| **Full suite command** | `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace --no-fail-fast -- --test-threads=1 && (cd ui && pnpm install --frozen-lockfile && pnpm svelte-check && pnpm lint)` |
| **Estimated runtime** | ~60–120 s on M1 dev box (cold), ~20–40 s warm. UI smoke (`pnpm tauri dev`) is manual. |

---

## Sampling Rate

- **After every task commit:** `cargo test -p <crate-touched>` (scoped). For UI-only tasks: `cd ui && pnpm svelte-check && pnpm lint`.
- **After every plan wave:** workspace-wide quick run.
- **Before `/gsd-verify-work`:** Full suite must be green AND manual `pnpm tauri dev` smoke validates: device create modal opens, autocomplete fires, sidebar nav switches, theme toggle works no-flash on reload.
- **Max feedback latency:** 30 s scoped, 90 s full workspace, 5 min for manual UI smoke.

---

## Per-Task Verification Map

> Per-task rows are populated by the planner once `*-PLAN.md` files exist. The plan-checker enforces that every task carries an `<automated>` verify line or a Wave 0 dependency.

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 2-01-01 | 01 | 0 | DEV-04..05 | — | N/A | structural | `cd ui && pnpm install` + svelte-check delta-zero | ❌ W0 | ⬜ pending |
| (rest) | … | … | … | … | … | … | … | … | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `ui/package.json` adds `@tauri-apps/api ^2.11`, `@tauri-apps/plugin-dialog ^2.7`, `svelte-spa-router ^5.1` to `dependencies` — closes Phase 1 deferred (D-Cleanup-01)
- [ ] `ui/pnpm-lock.yaml` regenerated and committed
- [ ] `.github/workflows/ci-fast.yml` + `ci-full.yml` — `continue-on-error: true` removed from `pnpm svelte-check` step
- [ ] `migrations/V013__devices_fts_triggers.sql` created (PRAGMA user_version = 13, triggers + 5 partial indexes + `notes→specs` rename)
- [ ] `crates/trackly-infra/src/db/migrations.rs::max_known_version()` updated to 13
- [ ] `crates/trackly-core/src/{ports,domain}/mod.rs + devices.rs` scaffolded
- [ ] `crates/trackly-infra/src/repos/{mod.rs,devices_sqlite.rs}` scaffolded
- [ ] `crates/trackly-app/src/services/{mod.rs,device_service.rs}` scaffolded
- [ ] `crates/trackly-app/src/{dto,tauri_cmds,http,csv}/{devices.rs}` scaffolded
- [ ] `crates/trackly-app/tests/devices_*.rs` integration test files scaffolded
- [ ] `crates/trackly-app/tests/fixtures/devices/{utf8.csv, utf8-bom.csv, cp1251-semi.csv, cp1251-comma.csv, malformed.csv}` committed
- [ ] `ui/src/lib/{api,stores,components,utils}/` scaffolded
- [ ] `ui/src/features/{layout,devices}/` scaffolded
- [ ] `ui/index.html` — inline no-flash script in `<head>`
- [ ] `ui/src/styles/_tokens.scss` extended with real palette + spacing scale
- [ ] `crates/trackly-app/tauri.conf.json` — capability declarations for `tauri-plugin-dialog`, `tauri-plugin-single-instance`
- [ ] `crates/trackly-app/capabilities/main.json` created (Tauri 2 capability schema)

`wave_0_complete` flips to `true` once Plan 02-01 (Phase 1 cleanup + scaffolding) commits land on `main`.

---

## Validation Architecture (from RESEARCH)

The following invariants MUST be enforceable from automated tests by phase end:

| Dimension | Invariant | Test Plan |
|-----------|-----------|-----------|
| **Schema (V013)** | FTS triggers fire on INSERT/UPDATE/DELETE; soft-delete (`deleted_at_utc IS NOT NULL`) removes row from FTS index | `tests/devices_fts_triggers.rs` — insert, update name → re-query FTS, soft-delete → FTS empty, restore (clear `deleted_at_utc`) → FTS repopulated |
| **Schema (V013)** | Partial indexes exist on `(name, model)`, `(name, location_id)`, `(name, status_id)`, `(name, condition)`, `(name, complectation)` | `tests/devices_fts_triggers.rs` — `PRAGMA index_list(devices)` + `pragma_index_info` assertions |
| **Schema (V013)** | `notes` column renamed to `specs` (or alternative path documented) | `tests/per_record_invariants.rs` extension — `pragma_table_info(devices)` includes `specs` not `notes` |
| **Repository** | Round-trip create→get→update→list→delete_soft→list excludes soft-deleted | `tests/devices_crud.rs` integration |
| **Optimistic lock** | `update_device(id, version=N, patch)` with stale version → `AppError::OptimisticLockMismatch{expected: N+k, actual: N}` | `tests/devices_optimistic_lock.rs` |
| **Audit log** | Every CRUD writes an audit_log row with `before_json`/`after_json` in SAME transaction as the device write | `tests/devices_audit_trail.rs` — count `audit_log` rows per operation |
| **Search (FTS5)** | Query "ноутб*" returns devices with name "Ноутбук Lenovo"; query "ABC123" matches `inventory_number`; query "S/N-007" matches `serial_number` | `tests/devices_search.rs` |
| **Autocomplete** | `devices_autocomplete("model", "Len")` returns DISTINCT models starting with "Len"; `devices_autocomplete("model", "", ctx_name="Принтер HP")` returns ONLY models previously used with name="Принтер HP" | `tests/devices_autocomplete.rs` |
| **Grouping** | Non-unique devices (both `inventory_number` and `serial_number` NULL) with same `(type, name, model, specs, complectation, condition, location_id, status_id)` collapse into one `DeviceGroup` with `count` and `ids[]` | `tests/devices_grouping.rs` |
| **CSV import** | UTF-8 (no BOM, `,`), UTF-8 BOM (`,`), CP1251 (`;`), CP1251 (`,`) all decode correctly; cyrillic strings (incl. «Сидоров-Петроградский Иван Александрович (ё) №42») round-trip without garbling | `tests/devices_csv_import.rs` over 4 fixture files |
| **CSV import — per-row errors** | A row violating a required-field constraint produces `RowError { row_index, error: Validation }` while OTHER rows still commit | `tests/devices_csv_import.rs` — malformed.csv fixture |
| **CSV import — preview/commit token** | Token expires after 5 min; second `import_csv_commit` with the same token returns `AppError::Validation{field: "token", message: "expired"}` | `tests/devices_csv_session.rs` |
| **CSV export** | First 3 bytes are `0xEF 0xBB 0xBF` (UTF-8 BOM); delimiter is `;`; headers are Russian (`Тип;Наименование;...`); opens in RU-locale Excel without mojibake (manual verification) | `tests/devices_csv_export.rs` — automated; manual Excel verification documented |
| **AppCtx extension** | `AppCtx.devices: Arc<DeviceService>` field populated by `AppCtx::build`; `Clone` impl still works | extension of `crates/trackly-app/tests/health_smoke.rs` pattern |
| **Specta export** | `ui/src/bindings.ts` after `cargo test --test export_bindings` contains: `DeviceDto`, `DeviceNew`, `DevicePatch`, `DeviceFilter`, `DeviceGroup`, `CsvImportPreview`, `CsvImportReport`, plus all 12 new device command types | extended `tests/export_bindings.rs` |
| **UI — theme** | `localStorage["trackly:theme"]` written on toggle; cold reload with stored "dark" renders dark immediately (no light flash); `matchMedia('(prefers-color-scheme: dark)')` change updates "system" mode live | `pnpm svelte-check` (compile gate) + manual smoke during `pnpm tauri dev` |
| **UI — transport** | `apiClient.devices.list()` in Tauri webview routes to `invoke`; same call in browser (when Phase 5 ships) routes to `fetch`. Same payload shape in both. | Phase 2: Tauri-side only via manual smoke. Phase 5 verifies HTTP side. |
| **UI — sidebar** | Sidebar items render in exact UI-01 order with `---` dividers; clicking Devices navigates to `#/devices` and highlights item | `pnpm svelte-check` + manual smoke |
| **UI — toast** | Backend returns `AppError::Validation { field: "name", message: "..." }` → toast appears with the Russian message; inline error shows under the field | manual smoke (component test in Phase 2 backlog if vitest later) |
| **Russian-only** | Every visible string in `.svelte` files + every `AppError.message` for DEV-* is Russian | `pnpm lint` (custom rule deferred); spot-check during smoke |
| **CI** | `pnpm svelte-check` is now BLOCKING (no `continue-on-error`); first push with `@tauri-apps/api` resolves all bindings.ts imports | watch Actions UI after Phase 2 close commits |

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| `pnpm tauri dev` opens window with sidebar | UI-01 | Tauri runtime can't run headlessly in CI | dev box: `cd crates/trackly-app && cargo tauri dev` after Phase 2 lands |
| Theme switcher persists across reload (no light flash) | UI-02 | Reload behavior requires actual browser environment | toggle Dark → close app → reopen → first paint is dark |
| Device create modal: fill 4 required fields → device appears in list | DEV-01, DEV-02 | Full UI flow | manual smoke per `02-SKELETON.md` if planner emits one |
| CSV export opens correctly in RU-locale Excel | DEV-13 | Excel behavior not automatable | Export → open in Russian Excel installation → verify cyrillic + no mojibake |
| ProcMon check on Windows still passes after Phase 2 adds logs/audit-rows | FOUND-11 | Windows runner only | first `ci-full.yml` run on `main` after Phase 2 close |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies (enforced by plan-checker once plans exist)
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references (`@tauri-apps/api`, `svelte-spa-router`, `tauri-plugin-dialog`, V013 migration, ports/domain/repo/service scaffolds, UI lib + features scaffold) — flips when Plan 02-01 lands
- [x] No watch-mode flags (`cargo test` one-shot; `pnpm svelte-check` one-shot)
- [x] Feedback latency < 90 s (workspace-wide quick run on M1; manual UI smoke excluded)
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** pending — flips to `approved YYYY-MM-DD` once Wave 0 (Plan 02-01) commits land + manual smoke confirms 5 success criteria.
