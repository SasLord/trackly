---
phase: 27-core-workflow-windows
plan: 01
subsystem: ui
tags: [svelte5, design-system, tr-tokens, detail-panel]

# Dependency graph
requires:
  - phase: 26-windows-with-layout
    provides: PageHeader.svelte Snippet-slot precedent (title + actions), --tr-* token layer, Table/TableRow primitives
provides:
  - "DetailPanel.svelte — общий контейнер+скролл, empty-state, шапка (title/actions) для детальных панелей"
  - "DetailSection.svelte — секция детали (heading + children)"
  - "DetailField.svelte — поле field-grid (label/value, fallback «—»), контракт совместим с ActHeaderField"
affects: [27-02-acts-window, 27-04-cartridges-window, 27-07-printers-window, 28-support-admin-windows]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "D-01: разделяемый примитив детальной панели в ui/src/lib/components/, по прецеденту PageHeader.svelte (Snippet-слоты title/actions/children/emptyActions)"
    - "DetailPanel НЕ красит фон панели — поверхность даёт обёртка master-detail (D-02), избегая двойной заливки"

key-files:
  created:
    - ui/src/lib/components/DetailPanel.svelte
    - ui/src/lib/components/DetailSection.svelte
    - ui/src/lib/components/DetailField.svelte
  modified: []

key-decisions:
  - "DetailField.field использует var(--tr-space-3xs) (=2px) вместо жёсткого gap: 2px из CartridgeDetail-прообраза — миграция по значению, весь код на токенах"
  - "DetailField повторяет контракт ActHeaderField ({label, value|null}), но использует value ?? '—' (без спец-обработки пустой строки) — точно по тексту плана; окна wave-2 сами решат, нужен ли им muted-стиль пустой строки"

patterns-established:
  - "Shared detail-panel primitive (D-01): DetailPanel/DetailSection/DetailField — потребляются планами 27-02/27-04/27-07 и Фазой 28"

requirements-completed: [WIN-03, WIN-04, WIN-05]

# Metrics
duration: 4min
completed: 2026-07-21
---

# Phase 27 Plan 01: Общий паттерн детальной панели (D-01) Summary

**Три компонента-примитива (DetailPanel, DetailSection, DetailField) извлекают общий словарь трёх окон детали (Акты/Картриджи/Принтеры) — контейнер+скролл, empty-state, шапка title+actions, секция, field-grid — целиком на токенах `--tr-*`, без изменения полей/структуры (SC #4).**

## Performance

- **Duration:** 4 min
- **Started:** 2026-07-21T00:28:51Z
- **Completed:** 2026-07-21T00:31:43Z
- **Tasks:** 2 completed
- **Files modified:** 3 (все новые)

## Accomplishments
- `DetailPanel.svelte` — скролл-контейнер, empty-state (title/body/actions), шапка (title + actions Snippet), body Snippet — по прецеденту `PageHeader.svelte`
- `DetailSection.svelte` — секция (heading + children Snippet)
- `DetailField.svelte` — field-grid поле (label/value, fallback «—»), контракт совместим с существующим `ActHeaderField.svelte`
- Все три компонента целиком на `var(--tr-*)`, без hardcode-цветов; `DetailPanel` намеренно не красит фон (фон даёт обёртка master-detail из D-02, планы 27-02/04/07)

## Task Commits

Each task was committed atomically:

1. **Task 1: DetailPanel.svelte — контейнер, скролл, empty-state, шапка** - `bbcaa5a` (feat)
2. **Task 2: DetailSection.svelte + DetailField.svelte — секция и field-grid** - `9fd8fd1` (feat)

**Plan metadata:** committed with this summary (docs: complete plan)

## Files Created/Modified
- `ui/src/lib/components/DetailPanel.svelte` - контейнер+скролл детали, loading/empty-блок, шапка (title + actions Snippet), body Snippet
- `ui/src/lib/components/DetailSection.svelte` - секция детали: heading + children Snippet
- `ui/src/lib/components/DetailField.svelte` - поле field-grid: label + value (fallback «—» на null)

## Decisions Made
- `var(--tr-space-3xs)` вместо `gap: 2px` в `DetailField` (миграция по значению — фаза требует все узлы на токенах, а не только новые правила)
- `DetailPanel` не рендерит фон на `.detail-panel` — явное требование плана, чтобы избежать двойной заливки с обёрткой master-detail, которую введёт D-02 в планах 27-02/27-04/27-07

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
Три примитива готовы к потреблению планами 27-02 (Акты), 27-04 (Картриджи), 27-07 (Принтеры) — они заменят bespoke `.section`/`.detail-header`/`.fields-grid` разметку в `ActDetail.svelte`, `CartridgeDetail.svelte`, `PrinterDetail.svelte` на эти компоненты. Замена содержимого (props для action-кнопок, доменных полей и т.д.) остаётся за окнами wave-2 — сами примитивы полей/структуры не диктуют.

---
*Phase: 27-core-workflow-windows*
*Completed: 2026-07-21*
