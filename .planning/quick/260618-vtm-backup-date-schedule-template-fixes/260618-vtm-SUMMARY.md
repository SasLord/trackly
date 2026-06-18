---
phase: quick-260618-vtm
plan: 01
subsystem: settings
tags: [backup, templates, dto-mapping, bugfix]
requires: []
provides:
  - "Correct Backups DTO field mapping (timestamp_utc, schedule sentinel)"
  - "template_service NotFound guard on 0 rows_affected"
affects:
  - ui/src/features/settings/BackupSettings.svelte
  - crates/trackly-app/src/services/template_service.rs
tech-stack:
  added: []
  patterns:
    - "Transport-boundary sentinel normalization (disabled <-> empty option)"
    - "rows_affected guard converting silent no-op to AppError::NotFound"
key-files:
  created: []
  modified:
    - ui/src/features/settings/BackupSettings.svelte
    - crates/trackly-app/src/services/template_service.rs
decisions:
  - "lastBackupTime kept session-local (no persisted-last-backup feature invented; out of scope per plan)"
  - "id: 0 convention reused for document_template NotFound (kind is not an i64 id)"
metrics:
  duration: 2 min
  completed: 2026-06-18
---

# Phase quick-260618-vtm Plan 01: Backup date / schedule / template fixes Summary

Three post-close Phase 07 «Round 3» fixes: corrected the Backups settings frontend DTO field mapping (fixes «Invalid Date» and blank-schedule-after-restart), and added a `rows_affected` guard to `template_service` so no-op template updates return `AppError::NotFound` instead of silent `Ok(())`.

## What Was Done

### Task 1 — Backups frontend DTO mapping (R3-1 + R3-2) — commit `1569c28`
- **R3-1 (Invalid Date):** `BackupResult` interface had `timestamp: number`, but backend returns `timestamp_utc: i64`. Reading the non-existent field gave `undefined * 1000 = NaN` → «Invalid Date». Changed interface to `{ timestamp_utc: number; file_path: string }` and `runManualBackup` now uses `new Date(result.timestamp_utc * 1000).toLocaleString('ru-RU')` (kept `* 1000`; backend value is unix seconds).
- **Phantom field:** removed `last_backup_time: string | null` from `BackupConfigDto` (backend never sends it) and deleted the `lastBackupTime = cfg.last_backup_time` assignment in `onMount`. `lastBackupTime` stays a session-local `$state` set only after a manual backup — no persisted-last-backup feature invented (out of scope).
- **R3-2 (schedule blank after restart):** backend stores the disabled state as `"disabled"` (backup_service.rs:164), but the `<select>` «Отключено» option uses `value=""`. Normalized at the transport boundary so the select domain stays `"" | "daily" | "weekly"`:
  - On load: `schedule = cfg.schedule === 'disabled' ? '' : (cfg.schedule ?? '')`.
  - On save: send `schedule: schedule === '' ? 'disabled' : schedule`.
- Folder-picker, retention, and SCSS untouched per plan.

### Task 2 — template_service rows_affected guard (R3-3 / CR-02) — commits `2d553de` (test), `19e8642` (impl)
- TDD: added a failing `#[tokio::test]` (`update_body_unknown_kind_returns_not_found`) using `Identity::trusted_admin()` and the existing `build_test_db()` helper. RED confirmed («expected NotFound, got Ok(())»).
- `update_body`: replaced `conn.execute(...).map(|_| ()).map_err(...)` with `let n = conn.execute(...).map_err(map_rusqlite)?;` then `if n == 0 { return Err(AppError::NotFound { entity: "document_template", id: 0 }); } Ok(())`.
- `reset_to_default`: applied the identical guard. The pre-DB `entity: "default_template"` guard (kind absent from `DEFAULT_TEMPLATES`) is unchanged; the new guard covers a known default kind whose DB rows are all soft-deleted (`entity: "document_template"`).
- `id: 0` convention mirrors the existing `get_active` NotFound (kind is not an i64 id).

## Verification

- `pnpm svelte-check`: **0 errors** (36 pre-existing warnings in unrelated files — out of scope).
- `cargo test -p trackly-app --lib template_service`: **3 passed** (new NotFound test + 2 existing validate_preview tests).
- `cargo clippy -p trackly-app --lib`: **clean** (no warnings).

## Deviations from Plan

None — plan executed exactly as written.

## Human-Check Items (desktop runtime — cannot confirm on this dev box)

The two runtime UX symptoms require a running Tauri shell to fully confirm; the code-level root causes are fixed and the static gates pass:

1. **R3-1:** In `cargo tauri dev` → Settings → Бэкапы → «Создать резервную копию» → «Последний бэкап: <дата>» should show a real ru-RU date, not «Invalid Date».
2. **R3-2:** Set schedule = Ежедневно, save, restart the app, reopen Settings → Бэкапы → the schedule select should show «Ежедневно» (not blank).

## Out of Scope (confirmed not touched)

- R3-4 / CR-01: WONTFIX for RU-only v1 — intentionally not touched.

## Known Stubs

None.

## Self-Check: PASSED

- FOUND: ui/src/features/settings/BackupSettings.svelte
- FOUND: crates/trackly-app/src/services/template_service.rs
- FOUND commit: 1569c28
- FOUND commit: 2d553de
- FOUND commit: 19e8642
