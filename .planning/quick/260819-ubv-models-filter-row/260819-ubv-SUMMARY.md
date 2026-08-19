---
phase: 260819-ubv
plan: 01
subsystem: ui
tags: [svelte, filter, table-layout, a11y]

requires: []
provides:
  - "Поле фильтра моделей (id=models-search) на вкладке «Модели» — тот же Input/debounce-паттерн, что и cartridges-search"
  - "modelSearchQuery + клиентский derived filteredModels в CartridgesPage.svelte (фильтр по brand/model/notes, регистронезависимо)"
  - "Однострочная ячейка «Модель» в ModelListRow.svelte: span.kind-indicator (title/aria-label) вместо отдельного Badge типа расходника"
affects: [cartridges]

tech-stack:
  added: []
  patterns:
    - "Клиентский derived-фильтр над уже загруженным целиком списком (без сетевого запроса), зеркальный существующему debounce-паттерну поиска картриджей"
    - "a11y-индикатор (title+aria-label на декоративной полоске) вместо текстового Badge — экономит горизонтальное место, не теряя доступность"

key-files:
  created: []
  modified:
    - ui/src/features/cartridges/CartridgesSearchAndTabs.svelte
    - ui/src/features/cartridges/CartridgesPage.svelte
    - ui/src/features/cartridges/ModelListRow.svelte

key-decisions:
  - "Фильтрация моделей — клиентская (Array.filter над models, уже загруженным через refreshModels()), не бэкенд-эндпоинт — зафиксировано планом"
  - "CartridgeFilters и CartridgeFormModal продолжают получать полный нефильтрованный models (нужен для опций выбора модели); только ModelsList переключён на filteredModels"

patterns-established: []

requirements-completed: [UBV-01, UBV-02]

duration: ~15min
completed: 2026-08-19
---

# Quick 260819-ubv: Фильтр моделей + однострочная ячейка «Модель» Summary

**Добавлено клиентское текстовое поле фильтра над таблицей «Модели картриджей» (по образцу cartridges-search) и ячейка «Модель» свёрнута в одну строку с a11y-индикатором типа расходника вместо отдельного чипа**

## Performance

- **Duration:** ~15 min
- **Started:** 2026-08-19T14:45:00Z
- **Completed:** 2026-08-19T15:00:08Z
- **Tasks:** 2/2
- **Files modified:** 3

## Accomplishments
- Вкладка «Модели» раздела «Картриджи» показывает Input(id=models-search) в одной строке с переключателем вкладок, слева от него — вместо пустого `.search-spacer`
- Ввод текста регистронезависимо сужает список моделей по бренду+модели+примечанию клиентски (без обращения к бэкенду); очистка поля возвращает полный список
- Ячейка «Модель» в таблице — одна строка: полоска-индикатор типа расходника (title/aria-label «Картридж»/«Фотобарабан») → название модели (обрезается многоточием) → опциональный чип цвета; отдельного чипа типа расходника больше нет
- Инвариант FIX B3 (td.cell-name без display:flex напрямую) сохранён — раскладка по-прежнему живёт на вложенном span.cell-name-inner

## Task Commits

Each task was committed atomically:

1. **Task 1: Фильтр над таблицей моделей (вкладка «Модели»)** - `305fbac3` (feat)
2. **Task 2: Однострочная ячейка «Модель» с вертикальным индикатором типа** - `2ce425fb` (feat)

_Plan metadata commit (SUMMARY.md/STATE.md/ROADMAP.md) handled by orchestrator, not by this executor per quick-task constraints._

## Files Created/Modified
- `ui/src/features/cartridges/CartridgesSearchAndTabs.svelte` - добавлены пропы modelSearchQuery/onModelSearchChange, зеркальный debounce-стейт (localModelQuery/modelDebounceTimer/handleModelInput), Input(id=models-search) вместо .search-spacer в ветке {:else}; удалено неиспользуемое CSS-правило .search-spacer
- `ui/src/features/cartridges/CartridgesPage.svelte` - добавлен modelSearchQuery state и filteredModels derived (фильтр по brand/model/notes), проброшены новые пропы в CartridgesSearchAndTabs, ModelsList переключён на models={filteredModels}
- `ui/src/features/cartridges/ModelListRow.svelte` - добавлен kindLabel derived; разметка ячейки «Модель» свёрнута в одну строку (kind-indicator + name + опциональный Badge цвета), удалены .badges-обёртка и старый Badge типа; .cell-name-inner изменён с column на row, .name получил flex:1 1 auto, добавлено правило .kind-indicator(--drum) и .cell-name-inner :global(.badge){flex-shrink:0}

## Decisions Made
- Фильтр моделей — клиентский `Array.filter`, не отдельный бэкенд-эндпоинт (models уже загружены целиком, без пагинации) — решение зафиксировано планом, не пересматривалось
- Полоска-индикатор берёт цвета из уже существующих токенов `--tr-accent` (картридж) / `--tr-border-strong` (фотобарабан) — новых токенов не добавлялось

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Изменения чисто фронтендовые, изолированы в разделе «Картриджи» → вкладка «Модели». `pnpm --dir ui run svelte-check` и `pnpm --dir ui build` проходят без ошибок (0 ERRORS в обоих прогонах). Визуальная верификация (UAT) остаётся за пользователем в живом приложении — синтетические харнессы не считаются верификацией для Svelte/WKWebView-приложения.

---
*Quick task: 260819-ubv*
*Completed: 2026-08-19*

## Self-Check: PASSED

All modified files and task commits verified to exist:
- ui/src/features/cartridges/CartridgesSearchAndTabs.svelte — FOUND
- ui/src/features/cartridges/CartridgesPage.svelte — FOUND
- ui/src/features/cartridges/ModelListRow.svelte — FOUND
- .planning/quick/260819-ubv-models-filter-row/260819-ubv-SUMMARY.md — FOUND
- commit 305fbac3 (Task 1) — FOUND
- commit 2ce425fb (Task 2) — FOUND
