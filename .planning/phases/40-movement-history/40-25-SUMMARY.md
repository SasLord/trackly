---
phase: 40-movement-history
plan: 25
subsystem: ui
tags: [svelte, reports, structural-gate, gap-closure]

# Dependency graph
requires:
  - phase: 40-movement-history
    provides: "живая таблица отчётов (ReportTable) + отчёт «Перемещения» с is_deleted на строках"
provides:
  - "ReportsPage.svelte передаёт ReportTable нормализованный reportType={reportTypeKey()} вместо ключа вкладки activeReport"
  - "ui/scripts/check-report-type-parity.mjs — структурный гейт в pnpm lint против повторного рассинхрона экрана и экспорта"
affects: [reports, movements]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Структурный zero-dependency гейт (fs/path/url, regex + скобочный баланс), по образцу check-place-path-short.mjs, с поддержкой --src=<dir> для самотеста мутацией"

key-files:
  created:
    - ui/scripts/check-report-type-parity.mjs
  modified:
    - ui/src/features/reports/ReportsPage.svelte
    - ui/package.json

key-decisions:
  - "Гейт проверяет INV-1 (проп reportType читает reportTypeKey(), не activeReport напрямую) и INV-2 (строковый литерал в ReportTable.showDeletedBadge — return-значение внутри reportTypeKey())"
  - "Мутационная проверка гейта делалась во временной копии в scratchpad (--src=<dir>), не в рабочем дереве — исходники не трогались длительно"

patterns-established:
  - "Третья площадка известной коллизии ключа 'all' между доменами «Заявки»/«Перемещения» (после currentColumns()/currentCmd()) теперь имеет собственный durable-гейт, а не полагается на ревью"

requirements-completed: [HST-04]

# Metrics
duration: ~15min
completed: 2026-09-03
---

# Phase 40 Plan 25: Deleted-badge live-table parity Summary

**Бейдж «Удалено» теперь виден в живой таблице отчёта «Перемещения» (не только в CSV/PDF) + новый структурный гейт `check-report-type-parity.mjs` в `pnpm lint` не даёт этому рассинхрону вернуться.**

## Performance

- **Duration:** ~15 min
- **Completed:** 2026-09-03
- **Tasks:** 2/2
- **Files modified:** 3 (1 fix, 1 new gate script, 1 package.json wiring)

## Accomplishments
- Закрыт gap UAT-40 test 13 «deleted-badge-missing-in-live-report-table»: `ReportsPage.svelte` передаёт `ReportTable` `reportType={reportTypeKey()}` — тот же нормализованный источник, что уже используют пути `exportCsv`/`exportPdf`.
- Подтверждено отсутствие регрессии: для доменов «Устройства»/«Картриджи»/«Заявки» `reportTypeKey()` никогда не возвращает `'movements'`, значит бейдж по-прежнему не рисуется вне домена «Перемещения».
- Добавлен структурный гейт `ui/scripts/check-report-type-parity.mjs`, включённый в `pnpm lint`; мутационно проверен на обоих инвариантах (INV-1 и INV-2) во временной копии — гейт падает с понятным сообщением при откате фикса или при рассинхроне литерала в `showDeletedBadge`.

## Task Commits

Each task was committed atomically:

1. **Task 1: Передавать нормализованный reportType в живую таблицу** - `99f067ec` (fix)
2. **Task 2: Регрессионный гейт «reportType экрана = reportType экспорта»** - `df302fbf` (test)

**Plan metadata:** (this commit)

## Files Created/Modified
- `ui/src/features/reports/ReportsPage.svelte` - `<ReportTable reportType={...} />` теперь читает `reportTypeKey()` вместо `activeReport`
- `ui/scripts/check-report-type-parity.mjs` - новый структурный гейт: INV-1 (проп reportType на ReportTable читает reportTypeKey()), INV-2 (литерал сравнения в showDeletedBadge — return-значение reportTypeKey())
- `ui/package.json` - добавлен вызов `node scripts/check-report-type-parity.mjs` в конец скрипта `lint` (после `check-path-settings-form.mjs`, аддитивно — чтобы план 40-27 мог дописать свой гейт следом)

## Decisions Made
- Гейт написан по образцу `check-place-path-short.mjs` (тот же стиль парсинга, TAG-префикс, `--src=` для самотеста) для консистентности с существующими durable-гейтами в `ui/scripts/`.
- Мутационная проверка гейта (обязательное правило: «якорь мутации должен быть уникальным») выполнена во временной копии файлов в scratchpad-каталоге через `--src=<dir>`, а не прямой правкой рабочего дерева — исключает риск случайно закоммитить сломанный код между шагами.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Gap UAT-40 test 13 закрыт; готово для следующего плана волны (40-27, который допишет ещё один гейт в конец того же `lint`-скрипта — точка вставки оставлена аддитивной).
- Ручная проверка (открыть отчёт «Перемещения» с мягко удалённым предметом в приложении) осталась не выполненной агентом — это часть live-UAT, а не автоматизируемая проверка этого плана.

---
*Phase: 40-movement-history*
*Completed: 2026-09-03*

## Self-Check: PASSED

- FOUND: ui/scripts/check-report-type-parity.mjs
- FOUND: ui/src/features/reports/ReportsPage.svelte
- FOUND commit: 99f067ec
- FOUND commit: df302fbf
