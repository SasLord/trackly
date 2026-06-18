---
phase: quick-260618-vtm
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - ui/src/features/settings/BackupSettings.svelte
  - crates/trackly-app/src/services/template_service.rs
autonomous: true
requirements: [R3-1, R3-2, R3-3]

must_haves:
  truths:
    - "After «Создать резервную копию» the info text shows a valid Russian-locale date, not «Invalid Date»"
    - "Auto-backup schedule selected before restart is shown selected after restart (not blank)"
    - "template_service.update_body / reset_to_default return a NotFound AppError when 0 rows match instead of silent Ok(())"
  artifacts:
    - path: "ui/src/features/settings/BackupSettings.svelte"
      provides: "Correct BackupResult/BackupConfigDto field mapping + schedule sentinel normalization"
    - path: "crates/trackly-app/src/services/template_service.rs"
      provides: "rows_affected guard on update_body and reset_to_default"
  key_links:
    - from: "BackupSettings.svelte runManualBackup"
      to: "BackupResult.timestamp_utc"
      via: "new Date(timestamp_utc * 1000)"
      pattern: "timestamp_utc"
    - from: "template_service.update_body"
      to: "AppError::NotFound"
      via: "rows_affected == 0 guard"
      pattern: "rows_affected"
---

<objective>
Three small post-close fixes from Phase 07 «Round 3» backlog (R3-1, R3-2, R3-3). R3-4/CR-01 is WONTFIX for RU-only v1 and is NOT in scope.

Purpose: Backups info text currently shows «Последний бэкап: Invalid Date», the auto-backup schedule renders blank after restart, and the template service swallows no-op updates. These are latent/UX defects on an otherwise-closed phase.

Output: Patched `BackupSettings.svelte` (frontend DTO field mapping + schedule sentinel) and `template_service.rs` (rows_affected guards), plus a regression test for the template guard.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
</execution_context>

<context>
@.planning/STATE.md
@.planning/phases/07-reports-dashboard-settings/07-HUMAN-UAT.md

<interfaces>
<!-- Root-cause facts confirmed during planning — executor should use these directly. -->

Backend BackupResult (crates/trackly-app/src/services/backup_service.rs:24-29):
```rust
pub struct BackupResult {
    #[specta(type = i32)]
    pub timestamp_utc: i64,   // unix SECONDS — frontend currently reads `timestamp` (does not exist)
    pub file_path: String,
}
```

Backend BackupConfigDto (backup_service.rs:32-38) — note: NO `last_backup_time` field exists:
```rust
pub struct BackupConfigDto {
    pub backup_folder: Option<String>,
    pub schedule: String,     // backend default sentinel is "disabled" (backup_service.rs:164), NOT ""
    #[specta(type = i32)]
    pub retention: i64,
}
```

Frontend BackupSettings.svelte CURRENT (wrong) declarations:
- `interface BackupResult { timestamp: number }`            → field name mismatch → `undefined * 1000 = NaN` → "Invalid Date"
- `interface BackupConfigDto { ...; last_backup_time: string | null }` → phantom field, backend never sends it
- `<option value="">Отключено</option>`                     → never matches backend "disabled" → select shows blank after reload

Schedule `<select>` option values in the template (lines ~146-155): "" (Отключено), "daily" (Ежедневно), "weekly" (Еженедельно).

template_service.rs update_body (line ~157-169) and reset_to_default (line ~189-201) both do
`conn.execute(...).map(|_| ()).map_err(map_rusqlite)` — discarding the `usize` rows_affected.
`conn.execute` returns `Result<usize, rusqlite::Error>`. AppError::NotFound is
`{ entity: &'static str, id: i64 }`; the existing `get_active` (line ~224) already uses
`AppError::NotFound { entity: "document_template", id: 0 }` — mirror it.
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Fix Backups frontend — Invalid Date (R3-1) + schedule-after-restart (R3-2)</name>
  <files>ui/src/features/settings/BackupSettings.svelte</files>
  <action>
Fix three frontend/DTO mismatches in BackupSettings.svelte. All are pure field-mapping / value-mapping corrections — no backend change needed (backend DTOs are the source of truth).

R3-1 (Invalid Date): The `BackupResult` interface declares `timestamp: number` but the backend returns `timestamp_utc: i64`. Change the interface to `interface BackupResult { timestamp_utc: number; file_path: string }` and update `runManualBackup` to compute `lastBackupTime = new Date(result.timestamp_utc * 1000).toLocaleString('ru-RU')`. Keep the `* 1000` (backend value is unix SECONDS).

Also remove the phantom `last_backup_time` field: the backend `BackupConfigDto` has no such field, so `cfg.last_backup_time` is always undefined. Delete `last_backup_time: string | null;` from the `BackupConfigDto` interface and delete the `lastBackupTime = cfg.last_backup_time;` assignment in `onMount`. `lastBackupTime` stays a local `$state` that is only set after a manual backup in the current session (the «Последний бэкап» label is session-local, which is acceptable — do NOT invent a persisted-last-backup feature, that is out of scope).

R3-2 (schedule blank after restart): The backend stores/returns the disabled state as the string `"disabled"` (backup_service.rs:164), but the `<select>` «Отключено» option uses `value=""`. So a reloaded `"disabled"` matches no option and the select renders blank; and a saved `""` is stored verbatim, then re-read as the literal `""` which the backend treats inconsistently. Normalize at the transport boundary so the select's domain stays `"" | "daily" | "weekly"`:
- On load (onMount): map the backend sentinel to the empty option — `schedule = cfg.schedule === 'disabled' ? '' : (cfg.schedule ?? '')`.
- On save (saveConfig): map the empty option back to the backend sentinel — send `schedule: schedule === '' ? 'disabled' : schedule` inside the `patch`. Do this for the explicit save and leave `retention` as-is.
Mirror the GAP-S5 load-on-mount pattern already established (value read in onMount and bound into the field) — the schedule was already read on mount; this fixes the value mapping so a saved `daily`/`weekly`/disabled round-trips correctly.

Do NOT touch the folder-picker `__TAURI_INTERNALS__` logic, retention handling, or any SCSS.
  </action>
  <verify>
    <automated>cd /Users/madsas/Projects/trackly/ui && pnpm svelte-check 2>&1 | tail -5</automated>
    <human-check>In `cargo tauri dev`: create a backup → «Последний бэкап: &lt;дата&gt;» shows a real ru-RU date (not Invalid Date). Set schedule = Ежедневно, save, restart the app, reopen Settings → Бэкапы → the schedule select shows «Ежедневно» (not blank).</human-check>
  </verify>
  <done>BackupResult uses timestamp_utc; phantom last_backup_time removed; schedule maps "disabled"↔"" on load/save; svelte-check passes with 0 errors.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: template_service rows_affected guard (R3-3 / CR-02)</name>
  <files>crates/trackly-app/src/services/template_service.rs</files>
  <behavior>
    - update_body on a non-existent kind (no active row matches WHERE kind=?1 AND is_active=1 AND deleted_at_utc IS NULL) returns Err(AppError::NotFound { entity: "document_template", .. }) instead of Ok(()).
    - reset_to_default on a kind present in DEFAULT_TEMPLATES but with no active DB row (all soft-deleted) returns Err(AppError::NotFound) instead of Ok(()).
    - update_body / reset_to_default on an existing active row still return Ok(()) and bump version (existing happy path unchanged).
  </behavior>
  <action>
In `update_body` (the `self.writer.execute(...)` closure, ~line 158-168): the `conn.execute(...)` call returns `Result<usize, rusqlite::Error>` — the number of rows affected. Currently it is `.map(|_| ()).map_err(map_rusqlite)`, discarding the count. Replace with: bind the result `let n = conn.execute(...).map_err(map_rusqlite)?;` then `if n == 0 { return Err(AppError::NotFound { entity: "document_template", id: 0 }); }` then `Ok(())`. Use the same `AppError::NotFound { entity: "document_template", id: 0 }` shape already used by `get_active` in this file (the kind isn't an i64 id, so id: 0 is the established convention).

Apply the identical rows_affected guard to `reset_to_default` (~line 190-200). Note `reset_to_default` already returns NotFound earlier with `entity: "default_template"` when the kind is absent from the embedded DEFAULT_TEMPLATES; that pre-DB guard stays. The NEW guard covers the case where the kind IS a known default but has no active DB row (all soft-deleted) — use `entity: "document_template"` for that branch, consistent with update_body.

Add a `#[tokio::test]` to the existing `mod tests` block exercising the NotFound paths: build a test DB via the existing `build_test_db()` helper, construct an admin `Identity` (look at how other tests in the trackly-app crate build an admin/manager Identity for ManageSettings — e.g. grep the tests dir for `Identity {` or a test helper), then assert `update_body(&admin, "nonexistent_kind", "{}".into()).await` returns `Err(AppError::NotFound { .. })`. Keep the test minimal and matching existing test style in this file. Do NOT add a dependency or change the HTTP/Tauri adapter layer — this is a service-module fix and both transports already call through the service (dual-transport rule preserved).
  </action>
  <verify>
    <automated>cd /Users/madsas/Projects/trackly && cargo test -p trackly-app --lib template_service 2>&1 | tail -20</automated>
    <automated>cd /Users/madsas/Projects/trackly && cargo clippy -p trackly-app --lib 2>&1 | tail -5</automated>
  </verify>
  <done>update_body and reset_to_default return AppError::NotFound on 0 rows_affected; new test asserts the NotFound path; existing template_service tests still pass; clippy clean.</done>
</task>

</tasks>

<threat_model>
No new trust boundaries, external packages, or network surface. R3-1/R3-2 are frontend display-only field mappings; R3-3 tightens a service-layer return value (no new input path). No STRIDE-relevant changes; no package installs (legitimacy gate N/A).
</threat_model>

<verification>
- `pnpm svelte-check` passes (0 errors) after Task 1.
- `cargo test -p trackly-app --lib template_service` passes including the new NotFound test after Task 2.
- `cargo clippy -p trackly-app --lib` clean.
- Human-verify (desktop): manual backup shows a valid ru-RU date; schedule survives restart.
</verification>

<success_criteria>
- R3-1: «Последний бэкап» renders a valid ru-RU date after a manual backup (no «Invalid Date»).
- R3-2: a saved auto-backup schedule (Ежедневно/Еженедельно/Отключено) is shown correctly selected after an app restart.
- R3-3: template update_body/reset_to_default return AppError::NotFound on a 0-row match; regression test green.
- R3-4/CR-01 intentionally NOT touched (WONTFIX v1).
</success_criteria>

<output>
Create `.planning/quick/260618-vtm-backup-date-schedule-template-fixes/260618-vtm-SUMMARY.md` when done.
</output>
