---
phase: 27-core-workflow-windows
plan: 08
subsystem: ui
tags: [svelte5, scss-tokens, design-system, table-primitive, printers]

# Dependency graph
requires:
  - phase: 25-dropdown
    provides: Table/TableRow/Checkbox primitives (D-03 источник паттернов)
  - phase: 26-core-workflow-windows-w1
    provides: DeviceList/DeviceListRow эталон применения Table к плоскому списку
provides:
  - DiscoveryResultsTable.svelte на Table/TableRow/Checkbox (сырой <table> удалён)
  - Подтверждённый аудит: PrinterCreateModal/DiscoveryModal/TonerGauge/PrinterAlertBanner уже на токенах (0 изменений)
affects: [27-core-workflow-windows остальные Printers-планы, Phase 30 (визуальный паритет/QA)]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Раскрытие таблицы через head-snippet + Checkbox в <th>/<td> с sr-only-label вместо aria-label (Checkbox не пробрасывает aria-атрибуты)"
    - "Dedup/визуальный modifier класса на TableRow пробрасывается через :global(tr.duplicate) > .cell (тот же паттерн, что group-last-child в DeviceListRow)"

key-files:
  created: []
  modified:
    - ui/src/features/printers/DiscoveryResultsTable.svelte

key-decisions:
  - "Task 1 (PrinterCreateModal/DiscoveryModal) и Task 3 (TonerGauge/PrinterAlertBanner) — аудит без изменений: файлы уже полностью на var(--tr-*) и примитивах Input/Select/Button/Modal, hardcode-цветов нет; коммитов не создавалось (нет диффа)"
  - "Checkbox-примитив не принимает aria-label — доступный текст передан через children-snippet с локальным .sr-only классом (прецедент: ChartWidget.svelte)"

patterns-established: []

requirements-completed: [WIN-05]

# Metrics
duration: ~15min
completed: 2026-07-21
---

# Phase 27 Plan 08: Принтеры — модалки и виджеты (D-04) + DiscoveryResultsTable на Table/TableRow Summary

**DiscoveryResultsTable переведён с сырого `<table class="results-table">` на примитивы `Table`/`TableRow`/`Checkbox`; PrinterCreateModal, DiscoveryModal, TonerGauge, PrinterAlertBanner подтверждены аудитом как уже полностью токенизированные — изменений не потребовалось.**

## Performance

- **Duration:** ~15 min
- **Started:** 2026-07-21T10:59:19Z (по STATE.md сессии)
- **Completed:** 2026-07-21T11:03:44Z
- **Tasks:** 3 (1 с реальным диффом, 2 — audit-only, без диффа)
- **Files modified:** 1

## Accomplishments
- `DiscoveryResultsTable.svelte`: сырой `<table class="results-table">` + ручные `th,td`-стили удалены; заменены на `Table` (columns=6, `head`-snippet с select-all `Checkbox`) + `TableRow` на каждую строку результата
- Все сырые `<input type="checkbox">` (select-all и per-row) заменены на `Checkbox`-примитив с доступным текстом через `children`-snippet (`.sr-only`)
- Пустое состояние («Принтеры не найдены» / подсказка) отдано `Table` (`emptyTitle`/`emptyBody`) — текст сохранён дословно
- Dedup-подсветка (`tr.duplicate` → приглушённый текст) сохранена через `class` pass-through на `TableRow` + `:global(tr.duplicate) > .cell`
- Аудит `PrinterCreateModal.svelte` и `DiscoveryModal.svelte` (Task 1): подтверждено 0 hardcode-цветов, все контролы уже на `Input`/`Button`/`Modal`/`LocationAutocomplete`/`Spinner`, стили целиком на `var(--tr-*)` — изменений не потребовалось
- Аудит `TonerGauge.svelte` и `PrinterAlertBanner.svelte` (Task 3): подтверждено 0 hardcode-цветов вне SVG-атрибутов, порог-логика TonerGauge (`<script>` 29–37) не тронута, `PrinterAlertBanner` структурно идентичен близнецу `LowStockBanner` — изменений не потребовалось

## Task Commits

Each task was committed atomically:

1. **Task 1: PrinterCreateModal + DiscoveryModal (D-04)** - аудит, без диффа (нет коммита — файлы уже соответствуют критериям приёмки)
2. **Task 2: DiscoveryResultsTable — сырой `<table>` → Table/TableRow (D-04/D-03)** - `f3d8b03` (feat)
3. **Task 3: TonerGauge (аудит) + PrinterAlertBanner (D-04)** - аудит, без диффа (нет коммита — файлы уже соответствуют критериям приёмки)

**Plan metadata:** см. финальный docs-коммит после этого summary

_Note: Task 1 и Task 3 — audit-only задачи без изменений кода; отдельных коммитов не создано, так как `git diff` был пуст после чтения/грепа файлов._

## Files Created/Modified
- `ui/src/features/printers/DiscoveryResultsTable.svelte` - сырой `<table>` → `Table`/`TableRow`/`Checkbox`, ручные `th,td`-стили и bespoke `.results-table`/`.empty` удалены, select-all/per-row/dedup-логика сохранена

## Decisions Made
- Task 1 и Task 3 — подтверждены аудитом как уже полностью соответствующие критериям D-04 (0 hardcode-цветов, `var(--tr-*)` присутствует, примитивы уже применены); реальных изменений не вносилось, чтобы не создавать шум в диффе без необходимости
- Доступный текст чекбоксов в `DiscoveryResultsTable` передан через `children`-snippet + локальный `.sr-only` (Checkbox-примитив не поддерживает `aria-label`-проп), а не через `aria-label` на native input напрямую — сохраняет screen-reader эквивалентность исходного `aria-label="Выбрать {ip}"` / `aria-label="...выбрать все"`

## Deviations from Plan

None - plan executed exactly as written. Task 1 и Task 3 оказались уже соответствующими критериям приёмки на момент чтения (предыдущие фазы/планы уже перевели эти файлы на токены) — это не отклонение от плана, а подтверждённый аудитом факт, зафиксированный в acceptance criteria самого плана ("TonerGauge подтверждён на 100% токенах").

## Issues Encountered
Первая версия `DiscoveryResultsTable.svelte` содержала строку `results-table` внутри code-comment (описание миграции), что ложно проваливало grep-критерий приёмки `grep -c "results-table" == 0`. Исправлено переформулировкой комментария без буквального упоминания старого имени класса — не влияет на функциональность, только на текст комментария.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- `DiscoveryResultsTable` теперь единственный потребитель `Table`/`TableRow` в разделе Принтеры (наравне с `ActFormItemsTable` в Актах) — паттерн доступен для остальных списков Принтеров (`PrintersList`/`PrinterListRow`, D-03) в последующих планах фазы 27
- Модалки и виджеты Принтеров (WIN-05, D-04) для `PrinterCreateModal`/`DiscoveryModal`/`TonerGauge`/`PrinterAlertBanner` закрыты — блокеров для следующих планов фазы 27 нет
- `check-tokens.mjs`, `svelte-check`, `lint`, `build` — все зелёные после изменений

---
*Phase: 27-core-workflow-windows*
*Completed: 2026-07-21*

## Self-Check: PASSED

- FOUND: ui/src/features/printers/DiscoveryResultsTable.svelte
- FOUND: .planning/phases/27-core-workflow-windows/27-08-SUMMARY.md
- FOUND commit: f3d8b03
