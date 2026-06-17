---
phase: 07-reports-dashboard-settings
plan: "09"
subsystem: settings-ui
tags: [gap-closure, settings, tauri2, frontend-fix]
dependency_graph:
  requires: []
  provides: [GAP-S3-closed, GAP-S4-closed, GAP-S5-closed]
  affects: [StorageSettings, BackupSettings, ThresholdSettings]
tech_stack:
  added: []
  patterns:
    - "__TAURI_INTERNALS__ in window — canonical Tauri 2 desktop detection (aligned with transport.svelte.ts)"
    - "apiCall<T> with primitive T (string/number) — no wrapper DTO needed for plain Rust return types"
key_files:
  created: []
  modified:
    - ui/src/features/settings/StorageSettings.svelte
    - ui/src/features/settings/BackupSettings.svelte
    - ui/src/features/settings/ThresholdSettings.svelte
decisions:
  - "Tauri 2 detection: use '__TAURI_INTERNALS__' in window consistently across all settings components (matches existing transport.svelte.ts and App.svelte patterns)"
  - "Plain primitive return types from Rust (String / i64) must NOT be destructured — use apiCall<string> / apiCall<number> directly"
  - "ThresholdSettings spinner fix: padding-right: 2px and appearance: auto so native spinner arrows render flush at right border"
metrics:
  duration: "5 min"
  completed: "2026-06-17"
  tasks: 2
  files_modified: 3
---

# Phase 07 Plan 09: Settings gap-closure (GAP-S3/S4/S5) Summary

Three settings components had incorrect DTO shape expectations or wrong Tauri 2 environment detection; fixed with minimal, targeted changes — no Rust changes, no new packages.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | GAP-S3: StorageSettings DB path load + move detection | ee01045 | StorageSettings.svelte |
| 2 | GAP-S4/S5: BackupSettings detection + ThresholdSettings load + styling | 8910f19 | BackupSettings.svelte, ThresholdSettings.svelte |

## Changes Made

### Task 1 — StorageSettings.svelte (GAP-S3)

**Fix 1 — DB path load (stuck on "Загрузка…"):**
- Before: `apiCall<{ path: string }>('settings_get_db_path', {})` then `dbPath = result.path`
- After: `dbPath = await apiCall<string>('settings_get_db_path', {})`
- Root cause: backend `settings_get_db_path` returns `Result<String, AppError>` — a plain string, not `{ path: string }`. Destructuring `.path` on a string yields `undefined`, so `dbPath` was always falsy, showing "Загрузка…".

**Fix 2 — Move dialog detection (proceedWithMove):**
- Before: `!!(window as unknown as Record<string, unknown>).__TAURI__` (Tauri 1 API, always undefined in Tauri 2)
- After: `'__TAURI_INTERNALS__' in window` (Tauri 2 canonical detection)
- Root cause: `window.__TAURI__` is Tauri 1 only. In Tauri 2 the global is `__TAURI_INTERNALS__`. The old check always returned false inside the desktop app, causing the early-return error toast instead of opening the save dialog.

### Task 2 — BackupSettings.svelte (GAP-S4)

**Fix — Folder picker detection (pickFolder):**
- Before: `!!(window as unknown as Record<string, unknown>).__TAURI__` (Tauri 1)
- After: `'__TAURI_INTERNALS__' in window` (Tauri 2)
- Same root cause as GAP-S3: the check always returned false inside the desktop app, showing "Выбор папки доступен только в десктоп-приложении." error instead of opening the folder picker.

### Task 2 — ThresholdSettings.svelte (GAP-S5)

**Fix 1 — Threshold load (blank field on reopen):**
- Before: `apiCall<{ threshold: number }>('settings_get_low_stock_threshold', {})` then `threshold = result.threshold`
- After: `threshold = await apiCall<number>('settings_get_low_stock_threshold', {})`
- Root cause: backend `settings_get_low_stock_threshold` returns `Result<i64, AppError>` — a plain number. Destructuring `.threshold` on a number yields `undefined`, so threshold was NaN/undefined on every mount.

**Fix 2 — Number input styling (spinner at edge):**
- Before: `padding: var(--space-sm) var(--space-md)` — oversized right padding pushed spinner inward
- After: `padding: var(--space-xs) 2px var(--space-xs) var(--space-sm)` + `appearance: auto`
- Effect: native browser spinner arrows now render flush at the right border of the 80px input.

## Deviations from Plan

None — plan executed exactly as written.

## Threat Flags

None — no new network endpoints, auth paths, or schema changes introduced. The `__TAURI_INTERNALS__` detection fix correctly preserves the desktop-only gate for folder/file pickers (T-07-09-01, T-07-09-02 mitigations intact).

## Known Stubs

None — all fixes are production-ready wiring of existing backend commands.

## Self-Check

- [x] `grep -c '__TAURI_INTERNALS__' ui/src/features/settings/StorageSettings.svelte` = 1
- [x] `grep -c '__TAURI__' ui/src/features/settings/StorageSettings.svelte` = 0
- [x] `grep -c 'apiCall<string>' ui/src/features/settings/StorageSettings.svelte` = 1
- [x] `grep -c '__TAURI_INTERNALS__' ui/src/features/settings/BackupSettings.svelte` = 1
- [x] `grep -c '__TAURI__' ui/src/features/settings/BackupSettings.svelte` = 0
- [x] `grep -c 'apiCall<number>' ui/src/features/settings/ThresholdSettings.svelte` = 1
- [x] `pnpm svelte-check` exits 0 (232 files, 0 errors)
- [x] Commits exist: ee01045, 8910f19

## Self-Check: PASSED
