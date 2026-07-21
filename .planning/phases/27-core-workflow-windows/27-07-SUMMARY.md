---
phase: 27-core-workflow-windows
plan: 07
subsystem: ui
tags: [svelte5, design-system, printers, table, detail-panel, tabs, tokens]

# Dependency graph
requires:
  - phase: 27-core-workflow-windows
    provides: "DetailPanel/DetailSection/DetailField shared primitives (plan 27-01), Table/TableRow (Phase 25), PageHeader/Tabs (Phase 24/26)"
provides:
  - "Окно Принтеров (WIN-05) структурного слоя целиком на PageHeader/Tabs/Table/TableRow/DetailPanel"
  - "PrintersMasterDetail на --tr-surface-raised + --tr-elev-1 (D-13 регресс закрыт для Принтеров)"
affects: [27-09, 28-*]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "PrinterListRow: row-click on <td> cells (TableRow не форвардит onclick/role) — паттерн из ActListRow/DeviceListRow"
    - "PrinterDetail sections without heading prop for sections needing custom heading+action row (Данные устройства)"

key-files:
  created: []
  modified:
    - ui/src/features/printers/PrintersPage.svelte
    - ui/src/features/printers/PrintersMasterDetail.svelte
    - ui/src/features/printers/PrintersSearchAndTabs.svelte
    - ui/src/features/printers/PrintersList.svelte
    - ui/src/features/printers/PrinterListRow.svelte
    - ui/src/features/printers/PrinterDetail.svelte

key-decisions:
  - "Колонка тонера в списке показывает только первую запись tonerLevels (TonerGauge инлайн) — сохраняет поведение прежнего bespoke-виджета «краткого тонера», полная разбивка остаётся в PrinterDetail"
  - "Секция «Данные устройства» в PrinterDetail обёрнута в DetailSection БЕЗ heading-пропа — заголовок+кнопка «Редактировать» остались как локальная section-heading-row разметка внутри, чтобы не дублировать заголовок"

patterns-established: []

requirements-completed: [WIN-05]

# Metrics
duration: ~20min
completed: 2026-07-21
---

# Phase 27 Plan 07: Принтеры — структурный слой на дизайн-системе Summary

**Окно Принтеров (WIN-05) переведено на PageHeader/Tabs/Table+TableRow/DetailPanel: master-detail панели теперь всплывают на `--tr-surface-raised`+`--tr-elev-1` в обеих темах, список стал таблицей с инлайн-TonerGauge, деталь (603 строки) — на общий DetailPanel/DetailSection с сохранением всех 6 секций и async-подгрузки readings/агрегатов.**

## Performance

- **Duration:** ~20 min
- **Completed:** 2026-07-21
- **Tasks:** 3
- **Files modified:** 6

## Accomplishments

- `PrintersPage.svelte`: bespoke `<header class="page-header">` заменена на примитив `PageHeader` (title + `{#snippet actions}`), scoped `.page-header`/`.page-title` CSS убраны
- `PrintersMasterDetail.svelte` (D-02): обе панели переведены на `background: var(--tr-surface-raised)` + `border` + `box-shadow: var(--tr-elev-1)`; grid 35/65 и `<1100px` fallback сохранены дословно
- `PrintersSearchAndTabs.svelte` (D-05): самописный `role="tablist"` + `<button class="tab">` заменён на примитив `Tabs variant="underline"` со строковым адаптером ключей (`String(null)` ↔ `'null'`); debounce поиска и переключение фильтра не тронуты
- `PrintersList.svelte` + `PrinterListRow.svelte` (D-03): двухстрочный bespoke `.row` div переведён на 4-колоночную `TableRow` (имя+alert-dot / IP tabular-nums / статус-`Badge` / `TonerGauge` инлайном); `Table` теперь владеет рамкой/skeleton/empty-state; клик-на-строку перенесён на `<td>`-ячейки (паттерн `ActListRow`/`DeviceListRow`), т.к. `TableRow` не форвардит `onclick`/`role`/`tabindex`
- `PrinterDetail.svelte` (603 строки, D-01): переписан на общий `DetailPanel`/`DetailSection` по прецеденту `CartridgeDetail.svelte`; все 6 секций сохранены (уровни тонера, страничные счётчики, установленный картридж, совместимые модели картриджей — агрегаты, история статусов, данные устройства + метаданные); async-загрузка readings/aggregates/deviceData/installedCartridge (`<script>`) и `TonerGauge` не тронуты

## Task Commits

1. **Task 1: PrintersPage header→PageHeader + PrintersMasterDetail (D-02) + PrintersSearchAndTabs (D-05)** - `62195ee` (refactor)
2. **Task 2: PrintersList + PrinterListRow → Table/TableRow (D-03)** - `b25817d` (refactor)
3. **Task 3: PrinterDetail (603) → DetailPanel (D-01)** - `fddb9ea` (refactor)

## Files Created/Modified

- `ui/src/features/printers/PrintersPage.svelte` — `PageHeader` вместо bespoke шапки
- `ui/src/features/printers/PrintersMasterDetail.svelte` — обе панели на raised+elev-1 (D-02)
- `ui/src/features/printers/PrintersSearchAndTabs.svelte` — `Tabs` вместо самописного tablist (D-05)
- `ui/src/features/printers/PrintersList.svelte` — `Table` вместо bespoke rows/loading/empty/footer
- `ui/src/features/printers/PrinterListRow.svelte` — `TableRow` с колонками имя/IP/статус/тонер (D-03)
- `ui/src/features/printers/PrinterDetail.svelte` — `DetailPanel`/`DetailSection` вместо bespoke контейнеров (D-01)

## Decisions Made

- Колонка тонера в списке показывает первую запись `tonerLevels` через `TonerGauge` (не все цвета) — сохраняет объём информации прежнего bespoke «краткого тонера»; полная разбивка по всем картриджам остаётся в секции `PrinterDetail`
- Секция «Данные устройства» в `PrinterDetail` обёрнута в `DetailSection` без `heading`-пропа, чтобы сохранить локальную `section-heading-row` разметку (заголовок + кнопка «Редактировать») без дублирования заголовка

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Окно Принтеров структурно закрыто (WIN-05); регресс D-13 для Принтеров устранён
- `Table`/`TableRow`/`TonerGauge` не модифицированы — потребители (Устройства, Акты, Картриджи) не затронуты
- Human-check обеих тем (light/dark) для master-detail поверхностей и деталей принтера остаётся частью end-of-phase верификации (Phase 30 / финальный UAT фазы 27)

---
*Phase: 27-core-workflow-windows*
*Completed: 2026-07-21*

## Self-Check: PASSED

All created/modified files verified present on disk; all 3 task commit hashes (62195ee, b25817d, fddb9ea) verified in git log.
