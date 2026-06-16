---
phase: 07-reports-dashboard-settings
plan: "04"
subsystem: settings-ui
tags: [svelte5, ui, settings, tauri, pdf-preview, logo-upload, backup]

# Dependency graph
requires:
  - phase: 07-02
    provides: OrgDbService, BackupService, TemplateService extensions, settings commands
provides:
  - ui/src/features/settings/OrgSettings.svelte (org fields + logo upload/display/delete)
  - ui/src/features/settings/StorageSettings.svelte (DB path + open folder + move DB)
  - ui/src/features/settings/BackupSettings.svelte (manual backup + auto-backup config)
  - ui/src/features/settings/ThresholdSettings.svelte (low-stock threshold on-blur save)
  - ui/src/features/settings/TemplateEditor.svelte (template select + textarea + PDF preview + save + reset)
  - ui/src/pages/SettingsPage.svelte (extended with 5 new sections)
affects:
  - All 5 new settings sections visible in the Settings page

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Svelte 5 runes: $state, $derived, $effect, $props in all new components"
    - "Tauri context detection: typeof window.__TAURI__ !== 'undefined' for dual desktop/browser paths"
    - "Logo upload: tauri-plugin-dialog + tauri-plugin-fs on desktop; hidden input[type=file] in browser; frontend 512 KB size guard (T-07-04-01)"
    - "SVG logo served as <img src=data:...> not raw HTML/SVG — XSS guard (T-07-04-05)"
    - "Template body: sent to backend for validation (templates_validate_preview), never eval'd in browser (T-07-04-02)"
    - "PDF preview: Blob from Uint8Array(bytes) → URL.createObjectURL → <iframe> — same pattern as PdfPreviewModal.svelte"
    - "$derived isDirty = body !== originalBody for unsaved indicator in TemplateEditor"
    - "$effect on selectedKind → sync body/originalBody from local templates cache, revokeObjectURL on switch"
    - "StorageSettings: browser context guard shows error for DB move (T-07-04-03)"
    - "TemplateEditor: full-width card (no max-width: 640px) per UI-SPEC SET-09/D-20 exception"
    - "BackupSettings: folder required guard before running manual backup; folder persisted immediately via settings_save_backup_config"

key-files:
  created:
    - ui/src/features/settings/OrgSettings.svelte
    - ui/src/features/settings/StorageSettings.svelte
    - ui/src/features/settings/BackupSettings.svelte
    - ui/src/features/settings/ThresholdSettings.svelte
    - ui/src/features/settings/TemplateEditor.svelte
  modified:
    - ui/src/pages/SettingsPage.svelte

key-decisions:
  - "TemplateEditor full-width: UI-SPEC explicitly exempts the template editor from max-width: 640px so the monospace textarea can use full available width"
  - "logo display via <img> not raw SVG injection: T-07-04-05 mitigated — SVG served as data: URI in img context, scripts blocked"
  - "autocorrect attr removed from textarea: not in TypeScript HTMLProps types; spellcheck=false is sufficient"
  - "backupFolder required guard: runManualBackup shows error toast if no folder selected, matching UX spec"
  - "$effect for selectedKind sync: loads body/originalBody from local templates array on kind change without extra API call"

# Metrics
duration: ~4 min
completed: 2026-06-16
---

# Phase 7 Plan 04: Settings UI Summary

**5 new Svelte 5 components wired to 07-02 settings backend: OrgSettings (org fields + logo), StorageSettings (DB path + move), BackupSettings (manual + auto-backup config), ThresholdSettings (on-blur save), TemplateEditor (minijinja textarea + PDF preview iframe + reset modal)**

## Performance

- **Duration:** ~4 min
- **Started:** 2026-06-16
- **Completed:** 2026-06-16
- **Tasks:** 2
- **Files created/modified:** 6

## Accomplishments

### Task 1: OrgSettings + StorageSettings + ThresholdSettings

- `OrgSettings.svelte`: loads org data on mount (settings_get_org); form fields for org_name/inn/kpp/address; save via settings_save_org_fields; logo section with display (<img>, T-07-04-05 XSS guard), delete (settings_remove_org_logo), upload (Tauri: plugin-dialog + plugin-fs; browser: hidden input[type=file] with 512 KB client-side guard per T-07-04-01); object URL lifecycle managed with revokeObjectURL
- `StorageSettings.svelte`: DB path display via settings_get_db_path; "Открыть папку с базой данных" button (fs_open_folder); "Сменить расположение" destructive button → confirmation Modal → Tauri save dialog → settings_move_db + app_restart; browser guard shows descriptive error (T-07-04-03); restart overlay during operation
- `ThresholdSettings.svelte`: compact card; settings_get_low_stock_threshold on mount; number input 1-999; saves on blur via settings_set_low_stock_threshold; success toast "Порог обновлён"

### Task 2: BackupSettings + TemplateEditor + SettingsPage wire-up

- `BackupSettings.svelte`: manual backup (backup_run_manual) with folder required guard; folder picker via tauri-plugin-dialog (desktop) or error toast (browser); schedule/retention config with settings_save_backup_config; last backup timestamp displayed after successful backup
- `TemplateEditor.svelte`: full-width card (no max-width); templates_list_for_editor on mount; template kind selector; variables panel (collapsible `<details>`); monospace textarea 320px min-height; $derived isDirty indicator; validateAndPreview → templates_validate_preview → blob URL iframe (T-07-04-02: template body never eval'd in browser); templates_update_body with originalBody sync; templates_reset_to_default with confirmation Modal; blobUrl revoked on kind switch and unmount
- `SettingsPage.svelte`: imports all 5 new components; renders in UI-SPEC order: NetworkSettings → OrgSettings → StorageSettings → BackupSettings → ThresholdSettings → TemplateEditor

## Task Commits

1. **Task 1** - `3208d35` feat(07-04): OrgSettings + StorageSettings + ThresholdSettings components
2. **Task 2** - `43023e4` feat(07-04): BackupSettings + TemplateEditor + SettingsPage wire-up

## Verification Results

| Check | Result |
|-------|--------|
| ls ui/src/features/settings/ | All 5 new .svelte files present |
| svelte-check (222 files) | 0 errors, 31 warnings (all pre-existing) |
| settings_get_org + settings_save_org_fields in OrgSettings | 4 occurrences |
| backup_run_manual in BackupSettings | 1 occurrence |
| templates_validate_preview in TemplateEditor | 1 occurrence |
| $state runes in BackupSettings | 7 occurrences |
| OrgSettings/BackupSettings/TemplateEditor in SettingsPage | 6 occurrences (import + usage) |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `autocorrect` attribute removed from textarea**
- **Found during:** Task 2 — svelte-check reported `'autocorrect' does not exist in type HTMLProps<"textarea">`
- **Issue:** `autocorrect="off"` is a non-standard HTML attribute not present in TypeScript's HTMLAttributes types
- **Fix:** Removed `autocorrect` and `autocapitalize` attributes; `spellcheck="false"` is sufficient for a code textarea
- **Files modified:** ui/src/features/settings/TemplateEditor.svelte
- **Commit:** 43023e4 (same task commit, fixed before commit)

## Known Stubs

None — all 5 components call real backend API commands wired in plan 07-02. Logo display requires `settings_get_org_logo` which returns bytes; backup folder requires `settings_get_backup_config`; templates require `templates_list_for_editor`. All commands are implemented and tested in 07-02.

## Threat Flags

All threat register items mitigated as planned:

| T-ID | Mitigation | Status |
|------|-----------|--------|
| T-07-04-01 | Frontend 512 KB size check before API call; backend enforces same limit in save_logo() | Implemented |
| T-07-04-02 | Template body sent to backend validate endpoint; never eval'd in browser | Implemented |
| T-07-04-03 | DB move requires Tauri context check; browser shows error; user confirms via modal | Implemented |
| T-07-04-04 | Backup folder path at org-admin trust level (accepted) | — |
| T-07-04-05 | SVG logo rendered via `<img src="data:image/...">` not raw HTML injection | Implemented |

---
*Phase: 07-reports-dashboard-settings*
*Completed: 2026-06-16*
