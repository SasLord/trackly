---
phase: 28-support-admin-windows
plan: 06
subsystem: ui
tags: [svelte, design-system, tokens, settings, D-04, DS-03]

# Dependency graph
requires:
  - phase: 24-base-components
    provides: Input/Select/Checkbox primitives (--tr-* tokenized)
  - phase: 23-tokens-foundations
    provides: "--tr-text-mono token + global .tr-mono utility class"
provides:
  - "BackupSettings.svelte re-tokenized (Select для расписания, Input для ретенции, --tr-text-mono для пути к папке)"
  - "NetworkSettings.svelte re-tokenized (Input для порта/сертификата, Select для bind-адреса, Checkbox для desktop-lock)"
affects: [28-support-admin-windows остальные планы, 30-quality]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Wrapper div (.select-shrink/.input-shrink) вокруг block-level Input/Select для сохранения узкой ширины поля вместо растяжения на 100%"
    - "--tr-text-mono + глобальный класс .tr-mono для моноширинных путей/кодов (замена bare font-family: monospace)"

key-files:
  created: []
  modified:
    - ui/src/features/settings/BackupSettings.svelte
    - ui/src/features/settings/NetworkSettings.svelte

key-decisions:
  - "BackupSettings/NetworkSettings: Select/Input — block-level (width:100%) компоненты обёрнуты в узкие wrapper-div (.select-shrink/.input-shrink), чтобы визуально сохранить прежнюю компактную ширину полей (schedule-select ~180px, retention-input 80px) вместо растягивания на всю ширину контейнера"
  - "NetworkSettings.fingerprint (отпечаток сертификата) намеренно НЕ тронут — вне явного объёма плана, симметрично StorageSettings.db-path-code"

patterns-established: []

requirements-completed: [WIN-08]

# Metrics
duration: 8min
completed: 2026-07-22
---

# Phase 28 Plan 06: Настройки — BackupSettings + NetworkSettings ре-токенизация Summary

**BackupSettings и NetworkSettings переведены с raw `<select>`/`<input>`/checkbox на Select/Input/Checkbox примитивы; путь к бэкап-папке переведён с bare `font-family: monospace` на обязательный токен `--tr-text-mono` (DS-03).**

## Performance

- **Duration:** 8 min
- **Started:** 2026-07-22T06:24:00Z
- **Completed:** 2026-07-22T06:32:00Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments
- `.folder-code` в BackupSettings переведён на `--tr-text-mono` (обязательный DS-03 фикс) — теперь моноширинный путь к папке рендерится через глобальный токен вместо bare `monospace`
- BackupSettings: расписание бэкапа (`<select class="form-select">`) → `Select`, ретенция (`<input class="form-input" type="number">`) → `Input`
- NetworkSettings: порт/bind-адрес/путь к сертификату → `Input`/`Select`, чекбокс «Требовать вход в десктопе» → `Checkbox`
- Удалены все bespoke `.form-select`/`.form-input`/`.checkbox-label`/`.checkbox-text` стили из обоих файлов
- Disabled-условия (`!backupFolder`, `saving||serverRunning`, `lockToggling`), сохранение конфигурации, ручной бэкап, старт/стоп сервера и desktop-lock toggle — поведение не изменено (SC #4)

## Task Commits

Each task was committed atomically:

1. **Task 1: BackupSettings ре-токенизация + обязательный --tr-text-mono (D-04, DS-03)** - `8f7e7fc` (feat)
2. **Task 2: NetworkSettings ре-токенизация (D-04)** - `98ae562` (feat)

**Plan metadata:** (this commit)

## Files Created/Modified
- `ui/src/features/settings/BackupSettings.svelte` - Select/Input для расписания/ретенции; `.folder-code` на `--tr-text-mono`
- `ui/src/features/settings/NetworkSettings.svelte` - Input/Select/Checkbox для порта/bind-адреса/сертификата/desktop-lock

## Decisions Made
- Обернул `Select`/`Input` (block-level, `width: 100%`) в узкие wrapper-div (`.select-shrink`/`.input-shrink`) в BackupSettings, чтобы сохранить прежнюю компактную ширину полей расписания (~180px) и ретенции (80px) — без этого поля растянулись бы на всю ширину `.config-row`, меняя визуальный вид (хотя это не запрещено SC, ближе к «без изменений» трактовке плана)
- `.fingerprint` в NetworkSettings оставлен нетронутым (bare `font-family: monospace`) — явно исключено из объёма плана (симметрично `.db-path-code` StorageSettings), подтверждено `read_first` плана

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

BackupSettings и NetworkSettings полностью на примитивах, `check-tokens.mjs` и `svelte-check` (0 errors) проходят, `pnpm --dir ui build` собирается без новых ошибок. Готово к следующим планам Фазы 28 (OrgSettings/ActiveDirectorySettings/ThresholdSettings/TemplateEditor и остальные D-04 панели).

---
*Phase: 28-support-admin-windows*
*Completed: 2026-07-22*

## Self-Check: PASSED

- FOUND: ui/src/features/settings/BackupSettings.svelte
- FOUND: ui/src/features/settings/NetworkSettings.svelte
- FOUND: .planning/phases/28-support-admin-windows/28-06-SUMMARY.md
- FOUND: 8f7e7fc (Task 1 commit)
- FOUND: 98ae562 (Task 2 commit)
- FOUND: 5133cbd (Summary commit)
