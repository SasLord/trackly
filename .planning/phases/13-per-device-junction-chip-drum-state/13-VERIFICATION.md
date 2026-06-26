---
phase: 13-per-device-junction-chip-drum-state
verified: 2026-06-26T09:40:00Z
status: passed
score: 8/8 must-haves verified
overrides_applied: 0
---

# Phase 13: Редизайн совместимости Принтеры↔Картриджи + свёрнутые chip-задачи Verification Report

**Phase Goal:** Модель совместимости «принтер↔картридж» переходит с per-device junction-таблицы (V029) на free-text-связь по уникальному наименованию принтера (V005); UI совместимости консолидирован в один блок на стороне модели картриджа; карточка принтера получает read-only агрегаты совместимости и блок данных устройства с редактированием; устранены два сопутствующих дефекта (kind-aware дефолт авто-возврата фотобарабана, рассогласование лимита списка принтеров).

**Verified:** 2026-06-26T09:40:00Z
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths (SPEC-13-R1..R8)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 (R1) | V029 `printer_cartridge_models` junction table is dropped; no live Rust/SQL reference remains except the DROP migration and historical comments | ✓ VERIFIED | `migrations/V032__cartridge_model_compatibility_printer_name.sql:56` contains `DROP TABLE printer_cartridge_models;`. `grep -r "printer_cartridge_models" crates/ migrations/` returns only: the V029 table-definition file itself, a historical comment in V030, and comments inside V032. Zero live Rust code references. `cargo build --workspace` and `cargo clippy --workspace -- -D warnings` both pass clean. |
| 2 (R2) | Compatibility resolved via V005 free-text `cartridge_model_compatibility.printer_name`, matched case-insensitively+trimmed against the target printer's `devices.name`, with D-05 pass-through when a model has zero compatibility rows | ✓ VERIFIED | `cartridges_sqlite.rs:1167-1172` (COUNT) and `:1196-1201` (main SELECT) both implement `LOWER(TRIM(cmc.printer_name)) = LOWER(TRIM(d.name))` with the `NOT EXISTS ... OR EXISTS ...` pass-through pattern. Integration tests `printer_compatib_case_insensitive_match`, `printer_compatib_list_narrows_to_linked_model`, `printer_compatib_unconfigured_device_does_not_narrow` all pass (`cargo test -p trackly-app --test cartridges_crud printer_compatib`: 3/3 ok). |
| 3 (R3) | Cartridge-model form shows exactly ONE "Совместимые принтеры" block; `CompatibleDevicesEditor.svelte` is deleted; autocomplete suggests DISTINCT printer names + allows free-text entry | ✓ VERIFIED | `ls ui/src/features/cartridges/CompatibleDevicesEditor.svelte` → No such file. `ModelFormModal.svelte:436-449` contains exactly one compat block (`<h3 class="compat-heading">Совместимые принтеры</h3>` + single `<CompatibilityEditor>`). `CompatibilityEditor.svelte` reworked to single free-text field per row with inline autocomplete dropdown that explicitly allows "Нет совпадений — будет сохранено как есть" (free entry). `suggest_compat_printer` (`cartridge_service.rs:829-849`) queries `SELECT DISTINCT name FROM devices WHERE type_id = 2 AND deleted_at_utc IS NULL`. |
| 4 (R4) | Printer card shows a strictly read-only compatibility-aggregate block (no add/remove controls); `CompatibleModelsEditor.svelte` deleted | ✓ VERIFIED | `ls ui/src/features/printers/CompatibleModelsEditor.svelte` → No such file. `PrinterDetail.svelte:273-287` renders `{agg.brand} {agg.model}: На складе {agg.inStock}, На заправке {agg.atRefill}, В работе {agg.inUse}` per D-07 order, no «Списано», no editing controls in this block. Backend `compatible_model_aggregates()` (`cartridges_sqlite.rs:346-389`) and `printers_get_compatible_aggregates` Tauri+HTTP command (RBAC `role_endpoint_matrix.rs` Case 41, confirmed passing: `cargo test -p trackly-app --test role_endpoint_matrix` → ok). |
| 5 (R5) | Printer card shows a device-data block (Инвентарный №, Серийный №, Расположение, Состояние) with an edit button opening "Редактирование устройства" (reusing `DeviceFormModal`) | ✓ VERIFIED | `PrinterDetail.svelte:315-343` renders all four fields via `meta-row` divs sourced from `deviceData` (fetched via `devices.get(p.deviceId)`); `:319-321` "Редактировать" button toggles `deviceEditOpen`; `:375-385` mounts `<DeviceFormModal target={deviceData} ... onSaved={...refetch...}>` — the existing reusable component (D-09), not a forked dialog. |
| 6 (R6) | Installed cartridge on printer card shown as code (C-XXXXXX) + model name, not internal numeric id | ✓ VERIFIED | `PrinterDetail.svelte:262-264`: `{installedCartridge.code} — {installedCartridge.model_brand} {installedCartridge.model_name}`. Loading-gap state renders `…`, never the raw id (line 266). |
| 7 (R7) | Auto-return of previous cartridge picks a kind-aware default state (drum→5 «Изношенный», cartridge→3 «На заправке»); `OperationModal.svelte` no longer hardcodes states 1/2/3 | ✓ VERIFIED | `cartridges_sqlite.rs:555-561`: `previous_cartridge_state_id.unwrap_or_else(|| if prev_current.model_kind_id == Some(2) { 5 } else { 3 })`. Regression tests `auto_return_uses_kind_aware_default_state_for_drum` and `auto_return_keeps_state_3_default_for_regular_cartridge` **executed directly by this verifier** (`cargo test -p trackly-infra --lib auto_return`) — both pass. Frontend: `OperationModal.svelte:450-451` defines `prevIsDrum`/`prevStateOptions` reusing `DRUM_STATES`/`CARTRIDGE_STATES`; no hardcoded `<option value="1/2/3">` literals remain (grep confirmed empty). |
| 8 (R8) | Printer-list cap no longer truncates below the frontend's requested limit; a printer beyond the old 200-row cutoff is not lost | ✓ VERIFIED | `printers_sqlite.rs:274`: `let limit = page.limit as i64;` — the `.min(200)` cap is gone entirely (D-13 uncapped read). Regression test `list_returns_all_printers_above_old_cap` (seeds 250 printers, asserts all returned) **executed directly by this verifier** (`cargo test -p trackly-infra --lib list_returns_all_printers_above_old_cap`) — passes. Frontend `OperationModal.svelte:345` requests `limit: 500`, now fully honored end-to-end. |

**Score:** 8/8 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `migrations/V032__cartridge_model_compatibility_printer_name.sql` | Collapses printer_brand+printer_model → printer_name; drops V029 | ✓ VERIFIED | Read in full; correct create-new/copy-transform/drop/rename pattern; `PRAGMA user_version = 32`. Applies cleanly in full migration chain (confirmed via `cargo test -p trackly-app --test role_endpoint_matrix`, which boots a fresh DB through V1..V32, and via `migration_idempotency` test). |
| `crates/trackly-infra/src/repos/cartridges_sqlite.rs` | `upsert_compatibility_in_tx`/`get_compatibility` on `Vec<String>`; `list()` filter rewritten; new `compatible_model_aggregates()`; kind-aware auto-return | ✓ VERIFIED | All four pieces read directly and confirmed wired/tested. |
| `crates/trackly-infra/src/repos/printers_sqlite.rs` | `.min(200)` cap removed | ✓ VERIFIED | `let limit = page.limit as i64;` confirmed, no cap. |
| `crates/trackly-app/src/tauri_cmds/printers.rs`, `http/printers.rs`, `specta_export.rs` | 4 V029 commands removed; new `printers_get_compatible_aggregates` added | ✓ VERIFIED | grep across all three confirms zero V029 command references; new command registered in specta_export and exercised by RBAC Case 41. |
| `ui/src/features/cartridges/CompatibleDevicesEditor.svelte` | Deleted | ✓ VERIFIED | File does not exist. |
| `ui/src/features/printers/CompatibleModelsEditor.svelte` | Deleted | ✓ VERIFIED | File does not exist. |
| `ui/src/features/cartridges/CompatibilityEditor.svelte` | Reworked to single free-text field + autocomplete | ✓ VERIFIED | Read in full; single `<input>` per row, debounced autocomplete, free-entry fallback. |
| `ui/src/features/cartridges/ModelFormModal.svelte` | Single compatibility block; trimmed+deduped submit payload | ✓ VERIFIED | One `<CompatibilityEditor>` instance; `filteredCompatibility = Array.from(new Set(...))` sent in payload. |
| `ui/src/features/printers/PrinterDetail.svelte` | Read-only aggregates + device block + code-based cartridge display | ✓ VERIFIED | All three sections read and confirmed present, correctly wired, no edit controls in the aggregates block. |
| `ui/src/features/cartridges/OperationModal.svelte` | No V029 calls; kind-aware previous-state Select; consistent printer-list limit | ✓ VERIFIED | `compatibleDeviceIds` derived client-side from V005 `compatibility: string[]` + `printerOptions`; `prevStateOptions`/`prevIsDrum` reuse pattern confirmed; no removed-command calls remain. |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `ModelFormModal.svelte` | `cartridges.suggestCompatPrinter(prefix)` | `CompatibilityEditor`'s `suggestFn` prop | WIRED | Confirmed call site at `ModelFormModal.svelte:447`. |
| `ModelFormModal.svelte` submit | `cartridge_models_create/patch` DTO `.compatibility: Vec<String>` | `filteredCompatibility` payload field | WIRED | Confirmed dedupe+trim at submit, DTO type matches (`crates/trackly-app/src/dto/cartridge.rs`). |
| `PrinterDetail.svelte` | `printers_get_compatible_aggregates` | `printers.getCompatibleAggregates(p.deviceId)` | WIRED | `$effect` at `PrinterDetail.svelte:57-75`; backend command authorized via `Action::ReadData`, RBAC Case 41 passes. |
| `PrinterDetail.svelte` device block | `DeviceFormModal` | `deviceEditOpen` toggle + `target={deviceData}` | WIRED | Confirmed mount + `onSaved` refetch at `PrinterDetail.svelte:375-385`. |
| `cartridges_sqlite.rs::list()` filter | `cartridge_model_compatibility` ↔ `devices.name` | `compatible_with_printer_device_id` param | WIRED | SQL confirmed; integration tests pass. |
| `OperationModal.svelte` | `printers.getCompatibleAggregates` / client-side V005 matching | Effects replacing deleted V029 calls | WIRED | Confirmed no dead command references remain; `svelte-check` reports 0 errors. |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|---------------------|--------|
| `PrinterDetail.svelte` compat block | `compatAggregates` | `printers.getCompatibleAggregates(deviceId)` → `compatible_model_aggregates()` SQL (real JOIN over `cartridge_models`/`devices`/`cartridges`, grouped counts) | Yes | ✓ FLOWING |
| `PrinterDetail.svelte` device block | `deviceData` | `devices.get(p.deviceId)` → real `devices` row | Yes | ✓ FLOWING |
| `PrinterDetail.svelte` installed cartridge | `installedCartridge` | `cartridges.get(currentCartridgeId)` → real `cartridges` row (code, model_brand, model_name) | Yes | ✓ FLOWING |
| `OperationModal.svelte` `compatibleDeviceIds` | client-derived `Set<deviceId>` | `cartridges.modelsGet(model_id).compatibility` (real V005 rows) × `printerOptions` (real `printers.list()` rows) | Yes | ✓ FLOWING |

### Behavioral Spot-Checks (Executed Directly by Verifier — Not SUMMARY Claims)

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| R7 drum auto-return defaults to state 5 | `cargo test -p trackly-infra --lib auto_return_uses_kind_aware_default_state_for_drum` | `test result: ok. 1 passed` (asserts `state_id == Some(5)`, not 3) | ✓ PASS |
| R7 regular-cartridge auto-return keeps state 3 | `cargo test -p trackly-infra --lib auto_return_keeps_state_3_default_for_regular_cartridge` | `test result: ok. 1 passed` | ✓ PASS |
| R8 printer list returns all rows above old 200 cap | `cargo test -p trackly-infra --lib list_returns_all_printers_above_old_cap` | `test result: ok. 1 passed` (250 seeded, highest-id row present) | ✓ PASS |
| R2/D-03 case-insensitive+trim compatibility matching | `cargo test -p trackly-app --test cartridges_crud printer_compatib` | `test result: ok. 3 passed; 0 failed` | ✓ PASS |
| R4 RBAC: Employee denied `printers_get_compatible_aggregates` | `cargo test -p trackly-app --test role_endpoint_matrix` | `test result: ok. 1 passed` (full matrix incl. Case 41) | ✓ PASS |
| Migration chain incl. V032 applies cleanly | `cargo test -p trackly-infra --test migration_idempotency` | `test result: ok. 1 passed` | ✓ PASS |
| Workspace builds clean | `cargo build --workspace` | `Finished` (0 errors) | ✓ PASS |
| Workspace lints clean | `cargo clippy --workspace -- -D warnings` | `Finished` (0 warnings) | ✓ PASS |
| Frontend type-checks clean | `pnpm exec svelte-check` (ui/) | `COMPLETED 242 FILES 0 ERRORS 36 WARNINGS` | ✓ PASS |
| Frontend production build | `pnpm --dir ui build` | `✓ 361 modules transformed`, dist produced | ✓ PASS |

### Environmental Test Failures (Confirmed Pre-Existing, Not Phase 13 Regressions)

| Test | Command | Result | Disposition |
|------|---------|--------|-------------|
| `restore_request_visibility_http.rs::blocked_user_restore_request_visible_to_admin_and_marks_pending_http` | `cargo test -p trackly-app --test restore_request_visibility_http` | `503 service unavailable: ad` (expected 403) | NOT a regression — `git log 7597801..HEAD` shows zero touches to this file in Phase 13; last touched in Phase 9. Fails because `ad_mode="real"` and no AD server is reachable from this dev macOS box (documented project constraint). |
| `settings_ad.rs::ad_test_connection_admin_succeeds_in_mock_mode` | `cargo test -p trackly-app --test settings_ad` | `503` (expected 200) | Same disposition — file untouched by Phase 13, same AD-unreachable root cause. |

### Requirements Coverage

| Requirement | Source Plan(s) | Description | Status | Evidence |
|-------------|----------------|--------------|--------|----------|
| SPEC-13-R1 | 13-01, 13-02, 13-03, 13-08 | Drop V029 junction | ✓ SATISFIED | Migration + grep + build/clippy clean |
| SPEC-13-R2 | 13-01, 13-02, 13-05, 13-08 | Compatibility via V005 free-text | ✓ SATISFIED | SQL matching logic + passing tests |
| SPEC-13-R3 | 13-02, 13-06 | Single "Совместимые принтеры" block | ✓ SATISFIED | ModelFormModal + CompatibilityEditor read directly |
| SPEC-13-R4 | 13-01, 13-03, 13-07 | Read-only printer-card aggregates | ✓ SATISFIED | PrinterDetail.svelte + aggregate SQL + RBAC test |
| SPEC-13-R5 | 13-07 | Device-data block + edit dialog | ✓ SATISFIED | PrinterDetail.svelte device block + DeviceFormModal reuse |
| SPEC-13-R6 | 13-07 | Installed cartridge by code+name | ✓ SATISFIED | PrinterDetail.svelte cartridge-row rendering |
| SPEC-13-R7 | 13-04, 13-08 | Kind-aware auto-return default | ✓ SATISFIED | Backend branch + 2 regression tests executed directly + frontend reuse pattern |
| SPEC-13-R8 | 13-04 | Printer-list cap fix | ✓ SATISFIED | Cap removed + regression test executed directly |

Per task instructions, SPEC-13-R1..R8 are tracked in phase-local `13-SPEC.md`, not in milestone-level `.planning/REQUIREMENTS.md` — this is expected for Phase 13's lightweight spec flow, not a gap.

### Anti-Patterns Found

None. Scanned all 21 files touched across the 8 plans (migration, 14 Rust source/test files, 6 Svelte/TS files) for `TBD`/`FIXME`/`XXX`/`TODO`/`HACK`/`PLACEHOLDER` markers, empty-return stubs, and hardcoded-empty-data patterns — zero hits. `deferred-items.md` documents a historical sequence of out-of-scope discoveries logged by earlier plans (13-02, 13-06) that were fully closed by later plans in the same phase (13-07 deleted `CompatibleModelsEditor.svelte`; 13-08 fixed `OperationModal.svelte`'s two stale call sites) — confirmed via direct grep, no live dangling references remain anywhere in `ui/src/`.

### Human Verification Required

None. All 8 requirements are verifiable via direct code inspection, passing automated tests (including 3 tests run directly by this verifier, not just trusted from SUMMARY claims), and clean build/lint/type-check gates. No visual-only, real-time, or external-service-dependent behavior remains unverified in this phase's scope.

### Gaps Summary

No gaps. All 8 SPEC-13 requirements are independently verified against the actual codebase:
- V029 junction table is dropped; zero live references remain.
- V005 free-text compatibility (`printer_name` column, case-insensitive+trim matching against `devices.name`, D-05 pass-through) is the sole source of truth, confirmed by passing integration tests.
- Exactly one compatibility block exists in `ModelFormModal.svelte`; both old editors (`CompatibleDevicesEditor.svelte`, `CompatibleModelsEditor.svelte`) are deleted from disk.
- Printer card shows strictly read-only aggregates (D-07 order, no «Списано», no edit controls) plus a device-data block with a working edit dialog (reusing `DeviceFormModal`).
- Installed cartridge is shown by code+model name, never by raw internal id.
- The kind-aware auto-return default (drum→5, cartridge→3) is implemented server-side and frontend-side, proven by 2 regression tests executed directly by this verifier.
- The printer-list cap mismatch (500 vs 200) is fixed by removing the backend cap entirely, proven by a regression test executed directly by this verifier.
- The two failing tests in the suite (`restore_request_visibility_http.rs`, `settings_ad.rs`) are confirmed pre-existing AD-environment failures, unrelated to and untouched by Phase 13.

Phase 13 goal is achieved. Ready to proceed.

---

*Verified: 2026-06-26T09:40:00Z*
*Verifier: Claude (gsd-verifier)*
