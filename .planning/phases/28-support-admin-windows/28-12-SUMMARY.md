---
phase: 28-support-admin-windows
plan: 12
subsystem: ui-settings
tags: [ui, dropdown, gap-closure, settings]
requires: []
provides:
  - "BackupSettings.svelte Расписание picker on custom Dropdown"
  - "NetworkSettings.svelte Bind-адрес picker on custom Dropdown"
  - "TemplateEditor.svelte Шаблон kind-select on custom Dropdown"
affects:
  - "ui/src/features/settings/BackupSettings.svelte"
  - "ui/src/features/settings/NetworkSettings.svelte"
  - "ui/src/features/settings/TemplateEditor.svelte"
tech-stack:
  added: []
  patterns:
    - "Dropdown flat + variant=\"select\" implicit-label wrapping (CartridgeFormBody.svelte / Phase 27-G1 precedent)"
key-files:
  created: []
  modified:
    - "ui/src/features/settings/BackupSettings.svelte"
    - "ui/src/features/settings/NetworkSettings.svelte"
    - "ui/src/features/settings/TemplateEditor.svelte"
decisions:
  - "D-08 boundary (TemplateEditor) respected: only the kind-selector control was migrated; textarea/preview/API surface untouched, verified via a diff-scoped grep gate"
metrics:
  duration: "~20 min"
  completed: "2026-07-22"
---

# Phase 28 Plan 12: Настройки — миграция трёх оставшихся Select на Dropdown (GAP-1) Summary

Закрыт GAP-1 (28-VERIFICATION.md) для трёх оставшихся точек с нативным `<select>` в окне
**Настройки**: `BackupSettings.svelte` (частота бэкапа), `NetworkSettings.svelte` (Bind-адрес),
`TemplateEditor.svelte` (тип шаблона, с сохранением D-08). Все три заменены на кастомный
`Dropdown.svelte` (flat + `variant="select"`), по образцу Phase 27 Cartridges (`CartridgeFormBody.svelte`).

## What Was Built

1. **BackupSettings.svelte** — «Расписание» picker переведён на `Dropdown`: `SCHEDULE_OPTIONS`
   (Отключено/Ежедневно/Еженедельно), `scheduleLabel` derived, `noExpandSchedule()` заглушка,
   implicit-label обёртка (`<label class="dropdown-label">`), тот же `disabled={!backupFolder}`
   и та же запись в `schedule`.
2. **NetworkSettings.svelte** — «Bind-адрес» picker переведён на `Dropdown`: `HOST_OPTIONS`
   (0.0.0.0/127.0.0.1), `hostLabel` derived, `noExpandHost()` заглушка, тот же
   `disabled={saving || serverRunning}` и та же запись в `settings.host`.
3. **TemplateEditor.svelte** — «Шаблон» kind-selector переведён на `Dropdown`: `templateOptions`
   derived (маппинг `templates` через `KIND_LABELS`/`tmpl.label`), `selectedKindLabel` derived,
   `noExpandKind()` заглушка. D-08 соблюдён: textarea, `.editor-wrapper`, preview-модалка и все
   `apiCall('templates_*', ...)` не тронуты — подтверждено grep-гейтом по diff (0 совпадений).

Во всех трёх файлах добавлено CSS-правило `.dropdown-label` (flex column, small gap) для
implicit-label обёртки, повторяющее паттерн `CartridgeFormBody.svelte`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - blocking] `pnpm lint` (prettier --check) flagged formatting on BackupSettings.svelte / TemplateEditor.svelte**
- **Found during:** post-task-3 plan-level verification (`pnpm --dir ui lint`)
- **Issue:** the Dropdown-migration edits in tasks 1 and 3 left minor formatting inconsistent with the project's Prettier config (blank-line placement around new derived/const declarations)
- **Fix:** ran `pnpm prettier --write` on the two flagged files only; re-ran `pnpm svelte-check` (still 0 errors) and re-verified the D-08 diff-scope grep gate (still 0 matches) before committing
- **Files modified:** `ui/src/features/settings/BackupSettings.svelte`, `ui/src/features/settings/TemplateEditor.svelte`
- **Commit:** `6a6b6d7`

### Out-of-scope / pre-existing (not fixed, logged per SCOPE BOUNDARY)

- **`pnpm --dir ui build` fails** with a pre-existing backend compile error
  (`crates/trackly-app/src/http/mod.rs:185/190` — `SpaAssets::get` not found). This is the same
  issue flagged in the 28-11 plan/prior gap-closure rounds; it blocks the whole-project build
  regardless of this plan's changes and is unrelated to the Select→Dropdown migration. Verification
  for this plan relied on `pnpm --dir ui svelte-check` (0 errors), `pnpm --dir ui lint` (pass), and
  file-scoped greps instead, per the executor's out-of-scope note.

## Verification Evidence

- `grep -c "import Select from" <each file>` == 0 for all three files
- `grep -c "import Dropdown from" <each file>` == 1 for all three files
- `SCHEDULE_OPTIONS` / `HOST_OPTIONS` / `templateOptions` each present (>=1)
- `onPickGroup={(o) => (schedule = ...)}` / `(settings.host = ...)` / `(selectedKind = ...)` each present exactly once
- `disabled={!backupFolder}` / `disabled={saving || serverRunning}` preserved
- `git diff ui/src/features/settings/TemplateEditor.svelte | grep -c "textarea\|editor-wrapper\|apiCall('templates_"` == 0 (D-08 gate)
- `pnpm --dir ui svelte-check` — 262 files, **0 errors**, 48 pre-existing warnings (unrelated files)
- `pnpm --dir ui lint` — eslint + prettier --check + check-tokens.mjs all pass

## Known Stubs

None.

## Threat Flags

None — component swap only inside existing authenticated admin-only Настройки screens; no new
trust boundary, no new network surface, no new packages (per plan's threat_model, `T-28-12-SC`
accepted with no new dependencies).

## Self-Check: PASSED
