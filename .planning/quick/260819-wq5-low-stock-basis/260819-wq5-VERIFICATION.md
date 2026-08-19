---
phase: 260819-wq5-low-stock-basis
verified: 2026-08-20T00:00:00Z
status: human_needed
score: 7/7 must-haves verified
overrides_applied: 0
human_verification:
  - test: "Открыть Настройки → «Порог низкого остатка» на непронастроенной (свежей) БД, убедиться визуально, что Radio по умолчанию стоит на «По модели принтера», что выбор сохраняется сразу при клике (toast «База подсчёта обновлена») и переживает перезагрузку окна; переключить на «По модели картриджа» и обратно, наблюдая, что предупреждающий блок на Дашборде и на странице Картриджи меняется синхронно."
    expected: "Radio по умолчанию = «По модели принтера»; клик сохраняет мгновенно; после перезагрузки состояние сохранено; оба предупреждающих блока (Дашборд, Картриджи) показывают одинаковую группировку при одинаковой настройке."
    why_human: "Backend/wiring полностью проверены кодом и автотестами (Tauri/HTTP команды, SQL-ветвление, DTO, bindings.ts), но фактический рендер Radio-компонента, toast и визуальная синхронизация Дашборда/Картриджи в живом WKWebView/браузере не покрыты автотестами — по проектному соглашению («Synthetic harness not verification») визуальная UI-проверка выполняется в реальном приложении, не эмулятором."
---

# Quick Task 260819-wq5: Порог низкого остатка — выбор базы подсчёта Verification Report

**Task Goal:** Настройки → «Порог низкого остатка»: выбор базы подсчёта низкого остатка (Radio) — по модели картриджа или по модели принтера, дефолт — по модели принтера; влияет на предупреждающий блок в Дашборде и на странице Картриджи.

**Verified:** 2026-08-20
**Status:** human_needed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Radio в Настройках, дефолт «По модели принтера» на непронастроенной БД | ✓ VERIFIED | `ThresholdSettings.svelte`: `let basis = $state<...>('printer_model')`; backend default `LowStockBasis::DEFAULT = PrinterModel` (`crates/trackly-core/src/domain/cartridges.rs:271`); `build_settings_get_low_stock_basis` falls back to DEFAULT on missing row (`tauri_cmds/settings_org.rs:241-260`) |
| 2 | Выбор сохраняется сразу при клике и переживает перезагрузку | ✓ VERIFIED | `ThresholdSettings.svelte` wraps both `<Radio>` in `<div onchange={saveBasis}>` → `apiCall('settings_set_low_stock_basis', {basis})`; `onMount` reloads via `apiCall('settings_get_low_stock_basis', {})`; write path persists to `app_settings` via `INSERT...ON CONFLICT DO UPDATE` (`tauir_cmds/settings_org.rs:280-289`) |
| 3 | printer_model группирует по `DISTINCT LOWER(TRIM(cartridge_model_compatibility.printer_name))`, не по `devices.name`, не по моделям картриджей | ✓ VERIFIED | SQL in `cartridges_sqlite.rs:970-1005` uses only `cartridge_model_compatibility.printer_name`, normalized via `LOWER(TRIM(...))`, grouped in a subquery `pg`; no reference to `devices` table anywhere in the branch. Confirmed by passing test `low_stock_printer_model_groups_by_compatible_printer_name` (case/whitespace variants collapse to one group, counts sum) |
| 4 | Принтер с нулевым остатком отображается (0 < порога) | ✓ VERIFIED | Test `low_stock_printer_model_zero_stock_printer_included` passes: printer with compatibility row but 0 cartridges still returned with `count: 0` |
| 5 | Модель картриджа без строк совместимости не отображается ни в одном принтере в printer_model; `list()` pass-through не тронут | ✓ VERIFIED | Test `low_stock_printer_model_excludes_model_without_compatibility` passes (3 full-stock cartridges under a model with no compat rows never appear); `git show 26df429f -- cartridges_sqlite.rs` contains no diff to `fn list(` — pass-through logic untouched |
| 6 | Дашборд и «Картриджи» показывают одинаковую группировку при одинаковой настройке (обе SQL-копии ветвятся синхронно) | ✓ VERIFIED | `dashboard_service.rs:154-228` contains a byte-identical anti-fan-out `EXISTS` SQL copy to `cartridges_sqlite.rs:970-1005`, reading the same `app_settings.low_stock_basis` key with the same guarded-parse/default logic. Cross-consistency proven by passing tests `dashboard_low_stock_printer_model_default_matches_repo_grouping` and `dashboard_low_stock_cartridge_model_basis_matches_legacy_grouping` |
| 7 | cartridge_model режим ведёт себя как раньше (group by model_id, HAVING cnt < threshold) | ✓ VERIFIED | Legacy SQL block in `cartridges_sqlite.rs:931-967` unchanged from pre-plan shape; test `low_stock_returns_models_below_threshold` (explicitly seeded `basis='cartridge_model'`) passes with expected `Some(model_id)`/`count`/`threshold` |

**Score:** 7/7 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/trackly-core/src/domain/cartridges.rs` | `LowStockBasis` enum + reshaped `LowStockItem` | ✓ VERIFIED | Enum + struct present exactly as planned, `DEFAULT`, `as_str()`, `parse()` all present and tested |
| `crates/trackly-infra/src/repos/cartridges_sqlite.rs` | `low_stock()` branches on `app_settings.low_stock_basis`; new unit tests | ✓ VERIFIED | Branching logic confirmed line-by-line; 6 new/updated tests, all passing (19/19 in module) |
| `crates/trackly-app/src/services/dashboard_service.rs` | Second independent low_stock SQL copy branches identically | ✓ VERIFIED | Confirmed byte-identical SQL shape and same read pattern |
| `crates/trackly-app/src/dto/cartridge.rs` | `LowStockItemDto` reshaped with `basis`, `Option<...>`, `label` | ✓ VERIFIED | Struct matches plan exactly; `From<LowStockItem>` maps `basis.as_str()` |
| `crates/trackly-app/src/tauir_cmds/settings_org.rs` | get/set Tauri commands, ManageSettings-gated write, reject-unknown validation | ✓ VERIFIED | `build_settings_get/set_low_stock_basis` + `#[tauri::command]` wrappers present; SET calls `authorize(..., ManageSettings)` and rejects unparseable strings with `AppError::Validation` |
| `crates/trackly-app/src/http/settings_org.rs` | HTTP routes mirroring threshold pair | ✓ VERIFIED | `handler_get/set_low_stock_basis`, `SetLowStockBasisPayload`, routes registered at `/api/v1/settings_get_low_stock_basis` and `/api/v1/settings_set_low_stock_basis` |
| `ui/src/features/settings/ThresholdSettings.svelte` | Radio group bound to basis, save-on-select, threshold label reflects basis | ✓ VERIFIED | Full file read; matches plan (Radio pair, bubbled-onchange save, conditional label text) |
| `ui/src/features/cartridges/LowStockBanner.svelte` | Dual-shape rendering, stable `{#each}` key | ✓ VERIFIED | Conditional render on `item.basis`, key `${item.basis}:${item.model_id ?? item.label}` |
| `ui/src/bindings.ts` (generated, gitignored) | New commands + reshaped `LowStockItemDto` | ✓ VERIFIED | `grep` confirms `settingsGetLowStockBasis`, `settingsSetLowStockBasis`, and `LowStockItemDto` type shape present in the current file on disk |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|----|--------|---------|
| `cartridges_sqlite.rs::low_stock()` | `app_settings.low_stock_basis` | `SELECT value FROM app_settings WHERE key = 'low_stock_basis'` | ✓ WIRED | Line 921-928 |
| `dashboard_service.rs` | `app_settings.low_stock_basis` (independent copy) | same SELECT | ✓ WIRED | Line 154-162 |
| `ThresholdSettings.svelte` | `settings_set_low_stock_basis` | `apiCall('settings_set_low_stock_basis', {basis})` inside `onchange` wrapper | ✓ WIRED | Confirmed in file body |
| `LowStockBanner.svelte` | `LowStockItemDto.basis` | conditional `{#if item.basis === 'cartridge_model'}` | ✓ WIRED | Confirmed in file body |
| `cartridges_sqlite.rs low_stock() printer_model branch` | `cartridge_model_compatibility.printer_name` | `LOWER(TRIM(printer_name))` grouping + correlated `EXISTS` | ✓ WIRED | SQL confirmed, matches `compatible_model_aggregates` anti-fan-out pattern |
| `CartridgesPage.svelte` | `LowStockBanner` | `<LowStockBanner items={lowStockItems} />` | ✓ WIRED | Confirmed usage |
| `DashboardPage.svelte` → `StatWidget.svelte` | `dto.low_stock_models` | `warningItems={widgetData?.low_stock_models ?? []}` | ✓ WIRED | Confirmed usage — dashboard renders basis-branched labels via existing widget component |
| `SettingsPage.svelte` | `ThresholdSettings` | component import/render | ✓ WIRED | Confirmed present |

### Behavioral Spot-Checks / Test Runs (independently re-executed, not trusted from SUMMARY)

| Command | Result | Status |
|---------|--------|--------|
| `cargo check -p trackly-core -p trackly-infra -p trackly-app` | Finished, no errors | ✓ PASS |
| `cargo test -p trackly-infra --lib cartridges_sqlite:: -- --test-threads=1` | 19/19 passed | ✓ PASS |
| `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test cartridges_low_stock -- --test-threads=1` | 5/5 passed | ✓ PASS |
| `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test dashboard_widgets -- --test-threads=1` | 5/5 passed | ✓ PASS |
| `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test role_endpoint_matrix -- --test-threads=1 --skip login_remember_persistent_cookie` | 1/1 passed (Case 44 confirmed present in test body, 403 assertion) | ✓ PASS |
| `pnpm svelte-check` (ui/) | 0 errors, 50 pre-existing warnings (none in touched files) | ✓ PASS |

Full-workspace regression sweep (`cargo test -p trackly-core -p trackly-infra -p trackly-app` unrestricted) was NOT re-run here — SUMMARY documents it was killed by environment timeout during execution and never completed; not re-attempted during verification per the task's own scoping note (targeted binaries only). This is an accepted known limitation, not a gap against this quick task's specific must-haves, since every task-scoped `<verify>` command from the PLAN was independently re-executed above and passed.

### Anti-Patterns Found

None. Grepped all 8 core touched files for `TODO|FIXME|XXX|TBD|placeholder|not implemented|coming soon` — zero matches in files changed by this quick task. One pre-existing unrelated `TODO(Phase 4)` comment exists in `tauri_cmds/settings_org.rs` (predates this task, introduced by commit `939f2ac4`, outside `low_stock_basis` code).

### Privacy Check (repository is public — hard constraint)

Grepped diffs of all three commits (`26df429f`, `36156278`, `5d87559b`) for org/PII markers (ООО, ИНН, КПП, ОГРН, ОКПО, email domains, common Russian test surnames) — no matches other than the commit author's own git-metadata email (expected/normal, not injected data). All new test fixtures use fictional brand/printer names ("Contoso", "Fabrikam", "Northwind", "Adatum", "Tailspin", "Wingtip", "Cactus") — no real organization data introduced.

### Requirements Coverage

Quick task — no `.planning/REQUIREMENTS.md` entries expected or found for `WQ5-*` IDs (consistent with project convention that only phase-level milestones populate REQUIREMENTS.md).

## Human Verification Required

### 1. Live UI confirmation of Radio default + save-on-select + cross-page consistency

**Test:** In `cargo tauri dev` (or LAN browser against rebuilt `ui/dist`), open Настройки → «Порог низкого остатка» on a fresh/unconfigured DB. Confirm the Radio defaults to «По модели принтера». Click «По модели картриджа», confirm a success toast appears and the choice persists across a reload. Then open Дашборд and Картриджи and confirm both low-stock warning blocks show matching, basis-consistent groupings.
**Expected:** Radio default = «По модели принтера»; selection saves immediately with toast feedback; state survives reload; Дашборд and Картриджи warning blocks agree with each other for the same basis setting.
**Why human:** All backend logic, SQL branching, API wiring, and DTO/bindings shape are verified by passing automated tests and direct code inspection above. The remaining gap is purely visual/interactive confirmation inside the real WKWebView/browser runtime (component render, toast timing, cross-page visual sync), which per project convention ("Synthetic harness not verification") cannot be substituted with a synthetic DOM harness.

## Gaps Summary

No code-level gaps found. All 7 must-have truths, 9 required artifacts, and 8 key links are verified present, substantive, and wired, backed by passing targeted test runs re-executed independently during this verification (not merely trusted from SUMMARY.md). The single outstanding item is a live human UI walkthrough of the Radio control and cross-page visual consistency, which is inherently outside static/automated verification reach.

---
_Verified: 2026-08-20T00:00:00Z_
_Verifier: Claude (gsd-verifier)_
