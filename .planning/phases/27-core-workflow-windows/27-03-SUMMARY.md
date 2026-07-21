---
phase: 27-core-workflow-windows
plan: 03
subsystem: ui
tags: [svelte5, scss-tokens, design-system, acts, checkbox-primitive, datepicker-primitive]

# Dependency graph
requires:
  - phase: 24-core-components
    provides: "Input/Select/Checkbox/Button/DatePicker/Modal примитивы на токенах"
  - phase: 27-01
    provides: "27-PATTERNS.md pattern map (D-04 ре-токенизация модалок/виджетов)"
provides:
  - "8 файлов окна Актов (модалки + detail-внутренние таблицы) полностью на токенах/примитивах"
  - "Раскрытый паттерн: сырой <input type=checkbox> без bespoke-CSS всё равно заменяется на Checkbox-примитив ради консистентности с visually-hidden label"
affects: [27-core-workflow-windows, 28-support-admin-windows]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Checkbox-примитив с children-снипетом как visually-hidden aria-label (см. DeviceFilters.svelte convention) вместо aria-label пропа (которого у Checkbox нет)"
    - "DatePicker как pure value-pass-through замена сырого <input type=date> с bind:value — не требует конвертации локальной/UTC семантики, т.к. просто передаёт браузерную YYYY-MM-DD строку"

key-files:
  created: []
  modified:
    - ui/src/features/acts/ReturnModal.svelte
    - ui/src/features/acts/DocumentAcceptanceModal.svelte
    - ui/src/features/acts/ActFormBody.svelte
    - ui/src/features/acts/ActItemsTable.svelte
    - ui/src/features/acts/ReturnItemsTable.svelte

key-decisions:
  - "ActFormModal.svelte, ActFormBody.svelte (кроме .req-фикса) и ActNumberField.svelte, PdfPreviewModal.svelte были уже полностью на токенах/примитивах при аудите — оставлены без изменений (кроме одного точечного бага)"
  - "ReturnItemsTable: сырой чекбокс без bespoke-CSS всё равно заменён на Checkbox-примитив ради консистентности must_haves truth #2 (все контролы модалок Актов — примитивы); aria-label сохранён через visually-hidden span в children"

patterns-established:
  - "Локальный .visually-hidden utility-класс (копия из DeviceFilters.svelte) — используется, когда Checkbox/примитив не даёт aria-label prop, а нужен только screen-reader текст"

requirements-completed: []

# Metrics
duration: 25min
completed: 2026-07-21
---

# Phase 27 Plan 03: Ре-токенизация модалок и detail-внутренних таблиц окна Актов Summary

**Полная ре-токенизация трёх модалок Актов (ReturnModal, DocumentAcceptanceModal, PdfPreviewModal), формы акта (ActFormModal/ActFormBody/ActNumberField) и двух detail-внутренних таблиц (ActItemsTable, ReturnItemsTable) — с раскрытием, что 3 из 8 файлов уже были полностью токенизированы при предыдущих фазах.**

## Performance

- **Duration:** ~25 min
- **Started:** 2026-07-21T00:15:00Z (approx)
- **Completed:** 2026-07-21T00:43:56Z
- **Tasks:** 3
- **Files modified:** 5 (из 8 в scope плана — 3 файла уже соответствовали критериям без изменений)

## Accomplishments
- ReturnModal.svelte: сырой `<input type="checkbox">` для «Применить ко всем» заменён на `Checkbox`-примитив
- DocumentAcceptanceModal.svelte: сырой `<input type="date">` заменён на `DatePicker`-примитив, удалена дублирующая bespoke-CSS `.date-input`
- ActFormBody.svelte: точечный фикс — `.req` CSS-класс (звёздочка обязательного поля «Когда отдали») был использован в разметке, но нигде не стилизован; добавлен `color: var(--tr-danger)` по конвенции `.required` из DeviceFormBody
- ActItemsTable.svelte, ReturnItemsTable.svelte: оставшиеся raw-px значения (`gap: 2px`, `padding-top: 8px`, `margin-top: 2px`, `font-size: 13px`) переведены на точные токен-эквиваленты (`--tr-space-3xs`, `--tr-space-xs`, `--tr-font-size-label`)
- ReturnItemsTable.svelte: сырой чекбокс каждой строки заменён на `Checkbox`-примитив с `visually-hidden` aria-текстом
- Аудит подтвердил: PdfPreviewModal.svelte, ActFormModal.svelte, ActNumberField.svelte уже были полностью на токенах/примитивах — изменений не потребовалось
- `ActFormItemsTable.svelte` НЕ тронут (подтверждено — не в git diff ни одного из трёх коммитов)

## Task Commits

Each task was committed atomically:

1. **Task 1: ReturnModal + DocumentAcceptanceModal + PdfPreviewModal (D-04)** - `9e22ebf` (refactor)
2. **Task 2: ActFormModal + ActFormBody + ActNumberField (D-04)** - `80140fa` (fix)
3. **Task 3: ActItemsTable + ReturnItemsTable — detail-внутренние таблицы (D-04)** - `aebd231` (refactor)

_Note: коммит Task 3 также включает prettier-форматирование `ReturnModal.svelte` (обнаруженное при финальном `pnpm lint` прогоне после Task 1) — это форматирование того же диапазона строк, изменённых в Task 1, оставлено в Task 3 для минимизации шума._

## Files Created/Modified
- `ui/src/features/acts/ReturnModal.svelte` - Checkbox-примитив вместо сырого input; удалена bespoke `.apply-toggle` CSS-обёртка label
- `ui/src/features/acts/DocumentAcceptanceModal.svelte` - DatePicker-примитив вместо сырого `<input type=date>`; удалена bespoke `.date-input` CSS
- `ui/src/features/acts/ActFormBody.svelte` - добавлен `.req { color: var(--tr-danger); margin-left: 2px; }`
- `ui/src/features/acts/ActItemsTable.svelte` - `gap: 2px` → `var(--tr-space-3xs)`
- `ui/src/features/acts/ReturnItemsTable.svelte` - Checkbox-примитив на построчный чекбокс + токенизация оставшихся raw-px значений

## Decisions Made
- **PdfPreviewModal/ActFormModal/ActNumberField audit-only:** эти три файла (из 8 в frontmatter `files_modified` плана) при чтении оказались уже полностью на токенах/примитивах (никаких сырых `<input>/<select>/<button>`, никаких hex-цветов) — Modal/Button/Input/Badge уже использовались корректно с прошлых фаз. Изменений не вносилось, чтобы не создавать шум в диффе без функционального выигрыша.
- **Checkbox без bespoke-CSS всё равно заменён на примитив (ReturnModal, ReturnItemsTable):** оба чекбокса не имели собственной bespoke-стилизации (обычный браузерный чекбокс), поэтому формально не подпадали под узкую формулировку «сырых стилизованных `<input>`». Решено заменить на `Checkbox`-примитив для консистентности с общим must_haves truth #2 плана («Все контролы форм модалок Актов используют примитивы») и визуальным паритетом с уже мигрированными окнами (Устройства).
- **DatePicker как безопасная замена локального `<input type=date>`:** `DatePicker.svelte` — чистый pass-through враппер над `<input type="date"> bind:value`, без какой-либо TZ-конвертации внутри себя. Замена в `DocumentAcceptanceModal` не меняет `dateLocalToUtcSeconds`-логику компонента (она остаётся снаружи, работает с той же `YYYY-MM-DD` строкой) — чисто визуальная подмена (SC #4 соблюдён).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Добавлена стилизация неиспользуемого CSS-класса `.req` в ActFormBody.svelte**
- **Found during:** Task 2 (ActFormModal + ActFormBody + ActNumberField)
- **Issue:** Разметка использует `<span class="req">*</span>` для звёздочки обязательного поля «Когда отдали», но CSS-класс `.req` нигде не был определён — звёздочка наследовала цвет `.label` вместо акцентного danger-цвета (в отличие от аналогичного паттерна `.required` в `DeviceFormBody.svelte`)
- **Fix:** Добавлен `.req { color: var(--tr-danger); margin-left: 2px; }` в `<style>`-блок, по образцу `.required` из `DeviceFormBody.svelte`
- **Files modified:** `ui/src/features/acts/ActFormBody.svelte`
- **Verification:** `node ui/scripts/check-tokens.mjs` (0 нарушений), `pnpm svelte-check` (0 errors)
- **Committed in:** `80140fa` (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (1 bug fix, Rule 1)
**Impact on plan:** Минорный визуальный фикс в рамках уже редактируемого файла Task 2 (файл был в scope re-tokenization). Не расширяет объём плана, не затрагивает поля/раскладку/workflow (SC #4 сохранён).

## Issues Encountered
None.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Все 8 файлов окна Актов (модалки + detail-внутренние таблицы) подтверждены на токенах/примитивах, `check-tokens.mjs`/`svelte-check`/`lint`/`build` зелёные
- `ActFormItemsTable.svelte` не тронут — потребитель `Table`-примитива остался стабилен для последующих плановых волн (D-03)
- Готово к следующим плановым файлам фазы 27 (Картриджи/Принтеры, WIN-04/WIN-05) — паттерн Checkbox-примитива для построчных чекбоксов таблиц применим и там (напр. `DiscoveryResultsTable`)
- Human-verify требуется (в конце фазы, per `human_verify_mode: end-of-phase`): диалог возврата, документ приёма, предпросмотр PDF и форма акта — визуально консистентны, функционально не изменены

---
*Phase: 27-core-workflow-windows*
*Completed: 2026-07-21*
