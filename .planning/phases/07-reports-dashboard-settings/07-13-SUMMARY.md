---
phase: 07-reports-dashboard-settings
plan: 13
subsystem: settings-frontend
tags: [gap-closure, settings, tauri2, frontend-bugfix]
dependency_graph:
  requires: [07-12]
  provides: [G2-1-fix, G2-2-fix, G2-3-fix]
  affects: [OrgSettings, StorageSettings, BackupSettings]
tech_stack:
  added: []
  patterns:
    - Tauri 2 detection via '__TAURI_INTERNALS__' in window (not window.__TAURI__)
    - plugin-fs readFile for image files with fs:allow-read-file capability
    - BackupConfigPatch nested under { patch: { ... } } with snake_case fields
key_files:
  modified:
    - ui/src/features/settings/OrgSettings.svelte
    - ui/src/features/settings/StorageSettings.svelte
    - ui/src/features/settings/BackupSettings.svelte
    - crates/trackly-app/capabilities/main.json
decisions:
  - "Tauri 2: '__TAURI_INTERNALS__' in window is the correct predicate; window.__TAURI__ is undefined in Tauri 2"
  - "plugin-fs readFile for images: cannot reuse read_file_bytes (CSV-only extension check); fs:allow-read-file granted without scope restriction (path comes from OS native dialog)"
  - "settings_open_db_folder takes no path arg: backend derives folder from AppCtx.paths.db_path() internally"
  - "BackupConfigPatch fields are snake_case (no rename_all); specta emits backup_folder, schedule, retention exactly as declared"
metrics:
  duration: 2m
  completed: 2026-06-18
  tasks_completed: 2
  files_modified: 4
---

# Phase 07 Plan 13: Settings Frontend Gap-Closure Round 2 (G2-1, G2-2, G2-3) Summary

**One-liner:** Fixed three frontend arg/detection bugs in Settings components: Tauri2 detection predicate in OrgSettings uploadLogo, broken command name in StorageSettings openFolder, and flat-vs-nested arg shape in BackupSettings both save sites.

## Tasks Completed

| # | Task | Commit | Files |
|---|------|--------|-------|
| 1 | Fix OrgSettings Tauri2 detection + grant fs:allow-read-file (G2-1) | cc04d33 | OrgSettings.svelte, capabilities/main.json |
| 2 | Fix StorageSettings command name (G2-2) + BackupSettings arg wrapping (G2-3) | e60b7b2 | StorageSettings.svelte, BackupSettings.svelte |

## What Was Fixed

### G2-1 — OrgSettings.svelte uploadLogo (OrgSettings.svelte line 105)

**Bug:** `!!(window as unknown as Record<string, unknown>).__TAURI__` — in Tauri 2, `window.__TAURI__` is `undefined`, so the predicate always evaluated to `false`. The Tauri desktop code path in `uploadLogo` was never reached; the browser file-input fallback was triggered instead (silently, no error).

**Fix:** Changed to `'__TAURI_INTERNALS__' in window` — the documented Tauri 2 reliable detection predicate.

**Capability:** Added `"fs:allow-read-file"` to `crates/trackly-app/capabilities/main.json`. Without it, the `plugin-fs readFile()` call returned permission denied. The grant has no scope restriction because the file path originates from a native OS dialog (user explicitly chose the file).

### G2-2 — StorageSettings.svelte openFolder (line 31)

**Bug:** `apiCall<void>('fs_open_folder', { path: dbPath })` — command `fs_open_folder` does not exist; the real command added in plan 07-12 is `settings_open_db_folder`. Also the `path` argument was incorrect — the backend derives the DB folder from `AppCtx.paths.db_path()` internally and takes no user-supplied path.

**Fix:** `apiCall<void>('settings_open_db_folder', {})` — correct command name, empty args object.

### G2-3 — BackupSettings.svelte both save sites

**Bug (pickFolder, line 78):** `apiCall<void>('settings_save_backup_config', { backup_folder: selected })` — flat arg instead of nested under `patch`. The Rust command signature is `settings_save_backup_config(state, patch: BackupConfigPatch)`, so the outer param name must be `patch`.

**Bug (saveConfig, line 94):** `apiCall<void>('settings_save_backup_config', { schedule, retention })` — same flat-vs-nested issue.

**Fix:** Both calls now wrap in `{ patch: { ... } }` with snake_case field names (`backup_folder`, `schedule`, `retention`) matching `BackupConfigPatch` as emitted by specta (no `rename_all` annotation on the struct).

## Verification Results

| Check | Result |
|-------|--------|
| `pnpm svelte-check` — 0 errors | PASS (233 files, 0 errors, 36 warnings) |
| `__TAURI__` absent from OrgSettings.svelte | PASS (0 matches) |
| `__TAURI_INTERNALS__` in OrgSettings.svelte | PASS (1 match) |
| `fs:allow-read-file` in capabilities/main.json | PASS (1 match) |
| `fs_open_folder` gone from StorageSettings.svelte | PASS (0 matches) |
| `settings_open_db_folder` in StorageSettings.svelte | PASS (1 match) |
| `patch: { backup_folder: selected` in BackupSettings | PASS (1 match) |
| `patch:` count in BackupSettings | PASS (2 matches) |

## Deviations from Plan

None — plan executed exactly as written. All three bugs were isolated to the identified lines and fixed with minimal changes.

## Known Stubs

None introduced in this plan.

## Threat Flags

No new security surface introduced. Changes are purely frontend argument fixes and a capability grant for a file path already controlled by the OS native dialog.

## Self-Check: PASSED

- cc04d33 exists: confirmed
- e60b7b2 exists: confirmed
- OrgSettings.svelte modified: confirmed
- capabilities/main.json modified: confirmed
- StorageSettings.svelte modified: confirmed
- BackupSettings.svelte modified: confirmed
