---
phase: 260819-vfg
plan: 01
subsystem: ui
tags: [svelte, settings, navigation]

# Dependency graph
requires: []
provides:
  - "Вкладка «Хранилище» настроек показывает карточки «Хранилище данных» и «Бэкапы» подряд"
  - "Подменю «Настройки» сокращено с 7 до 6 вкладок (убрана отдельная вкладка «Бэкапы»)"
affects: [settings]

# Tech tracking
tech-stack:
  added: []
  patterns: []

key-files:
  created: []
  modified:
    - ui/src/features/settings/SettingsSubNav.svelte
    - ui/src/pages/SettingsPage.svelte

key-decisions:
  - "Alias/редирект со старого ключа вкладки 'backup' не нужен — activeSection чисто локальный $state, без URL/localStorage адресации (расследовано на этапе планирования)"

patterns-established: []

requirements-completed: [VFG-01]

# Metrics
duration: 5min
completed: 2026-08-19
---

# Quick Task 260819-vfg: Объединение вкладок «Хранилище» и «Бэкапы» Summary

**Карточка BackupSettings перенесена под StorageSettings внутри единой вкладки «Хранилище»; отдельная вкладка «Бэкапы» убрана из подменю настроек (6 вкладок вместо 7).**

## Performance

- **Duration:** ~5 min
- **Started:** 2026-08-19T15:22:00Z
- **Completed:** 2026-08-19T15:26:28Z
- **Tasks:** 1 completed
- **Files modified:** 2

## Accomplishments
- `SettingsSubNav.svelte`: массив `SECTIONS` больше не содержит запись `{ key: 'backup', label: 'Бэкапы' }` — 6 вкладок вместо 7; поясняющий комментарий над массивом обновлён на «6 sections».
- `SettingsPage.svelte`: отдельная ветка `{:else if activeSection === 'backup'}` удалена; `<BackupSettings />` теперь рендерится сразу после `<StorageSettings />` внутри ветки `activeSection === 'storage'`, оба компонента — прямые дети `.settings-content` (существующий `flex-column` + `gap`), поэтому карточки стекаются вертикально без дополнительного wrapper-а.
- Импорт `BackupSettings` в `SettingsPage.svelte` сохранён без изменений — компонент по-прежнему используется, просто в другой ветке.
- Бэкенд-команды (`settings_get_db_path`, `settings_get_backup_config`, `settings_save_backup_config`, `backup_run_manual` и т.д.) и структура БД не тронуты — правки чисто фронтендовые (расположение существующих компонентов).

## Task Commits

1. **Task 1: Объединить вкладки «Хранилище» и «Бэкапы» в настройках** - `a6f283b7` (feat)

**Plan metadata:** committed separately by orchestrator (docs commit)

## Files Created/Modified
- `ui/src/features/settings/SettingsSubNav.svelte` - удалена запись 'backup' из SECTIONS, комментарий обновлён на «6 sections»
- `ui/src/pages/SettingsPage.svelte` - ветка 'backup' удалена, `<BackupSettings />` перенесён в ветку 'storage' сразу после `<StorageSettings />`

## Decisions Made
- Alias/редирект со старого ключа вкладки `'backup'` на `'storage'` не требуется: `activeSection` — чисто локальный `$state` компонента `SettingsPage.svelte`, не читается из URL/hash (роутер маршрутизирует только `/settings` целиком) и не сохраняется в `localStorage`/cookie. Живых путей навигации к удалённой ветке извне не существует (подтверждено на этапе планирования, повторно не проверялось).

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- Реорганизация UI завершена; визуальная UAT-проверка (6 вкладок, карточка «Бэкапы» под «Хранилище данных», рабочие кнопки бэкапа) выполняется пользователем в живом приложении — синтетические харнессы не считаются верификацией для Svelte/WKWebView.
- Блокеров нет.

---
*Phase: 260819-vfg*
*Completed: 2026-08-19*

## Self-Check: PASSED
