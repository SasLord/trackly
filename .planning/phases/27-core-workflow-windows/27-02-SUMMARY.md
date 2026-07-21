---
phase: 27-core-workflow-windows
plan: 02
subsystem: ui
tags: [svelte5, design-system, tr-tokens, acts, detail-panel, table]

# Dependency graph
requires:
  - phase: 27-core-workflow-windows
    plan: 01
    provides: DetailPanel.svelte/DetailSection.svelte/DetailField.svelte shared detail-panel primitives
  - phase: 26-windows-with-layout
    provides: PageHeader.svelte Snippet-slot precedent, --tr-* token layer, Table/TableRow primitives, DeviceFilters.svelte Tabs adapter pattern
provides:
  - "Акты (WIN-03) структурный слой целиком на PageHeader/Tabs/Table/TableRow/DetailPanel"
  - "Закрытие регресса D-13 Фазы 26 для окна Актов (master-detail снова 'всплывает' над контент-фоном)"
affects: [28-support-admin-windows]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "D-02: master-detail поверхность = --tr-surface-raised + border + box-shadow var(--tr-elev-1), заменяет --tr-surface/--tr-bg"
    - "D-05: строковый TabKey → Tabs напрямую (без String()/Number() адаптера, в отличие от DeviceFilters с number|null)"
    - "D-03: список без групп (плоская Table, без DeviceGroupRow-аналога), 4 колонки (№/Дата/Получатель/Позиций)"
    - "TableRow не форвардит onclick/role/tabindex на свой <tr> — row-click/keyboard-select вешается на <td>-ячейки самого потребителя (все ячейки кликабельны, первая — единственный tab-стоп)"
    - "Table.svelte не имеет action-slot в empty-состоянии — существующие empty-state действия («+ Создать акт» / «Сбросить поиск») перенесены в footer-snippet вместо правки shared-примитива"

key-files:
  created: []
  modified:
    - ui/src/features/acts/ActsPage.svelte
    - ui/src/features/acts/ActsMasterDetail.svelte
    - ui/src/features/acts/ActsSearchAndTabs.svelte
    - ui/src/features/acts/ActsList.svelte
    - ui/src/features/acts/ActListRow.svelte
    - ui/src/features/acts/ActDetail.svelte
    - ui/src/lib/components/DetailField.svelte
  deleted:
    - ui/src/features/acts/ActHeaderField.svelte

key-decisions:
  - "Table.svelte/TableRow.svelte НЕ модифицированы (явное требование плана — общие компоненты для Устройств/витрины/ActFormItemsTable); функциональные пробелы примитивов закрыты на стороне потребителя (footer-snippet для empty-action, onclick на <td> вместо onclick на TableRow)"
  - "DetailPanel.title остался string (не Snippet) — потеряна tr-mono моноширинная стилизация номера акта в заголовке детали; чисто типографская деталь, поля/действия не затронуты"
  - "ActHeaderField.svelte удалён (единственный потребитель — ActDetail — мигрировал на DetailField; 0 оставшихся импортов)"

patterns-established:
  - "Row-click-to-select на TableRow без правки примитива: onclick на каждой <td>, role=button+tabindex+onkeydown на первой ячейке как единственном keyboard tab-стопе"

requirements-completed: [WIN-03]

# Metrics
duration: 13min
completed: 2026-07-21
---

# Phase 27 Plan 02: Структурный слой окна Актов (WIN-03) Summary

**ActsPage/ActsMasterDetail/ActsSearchAndTabs/ActsList+ActListRow/ActDetail переведены на примитивы PageHeader/Tabs/Table+TableRow/DetailPanel+DetailSection+DetailField — регресс D-13 Фазы 26 (слияние master-панели с контент-фоном) закрыт, все поля/действия/workflow сохранены (SC #4).**

## Performance

- **Duration:** 13 min
- **Started:** 2026-07-21 (после Phase 27 Plan 08)
- **Completed:** 2026-07-21
- **Tasks:** 3 completed
- **Files modified:** 7 (6 изменено + 1 удалён), 1 shared-файл (`DetailField.svelte`) — только правка комментария

## Accomplishments

**Task 1 (D-04/D-02/D-05 — оболочка + поверхность + фильтр):**
- `ActsPage.svelte`: bespoke `<header class="page-header">` заменена на `PageHeader` (title + `actions` Snippet), по эталону `DevicesPage`; scoped CSS `.page-header`/`.page-title`/`.header-actions` удалён
- `ActsMasterDetail.svelte`: обе панели (`.master`, `.detail`) переведены на `--tr-surface-raised` + `border` + `box-shadow var(--tr-elev-1)`, вместо `--tr-surface`/`--tr-bg` — закрывает регресс D-13 Фазы 26. Grid `35% / 65%` и `<1100px`-fallback не тронуты
- `ActsSearchAndTabs.svelte`: самописный `<button class="tab">` + `<Badge>`-счётчики заменены на `Tabs variant="underline"` со встроенным `count`; debounce 250мс и guard `document.activeElement?.id` не менялись

**Task 2 (D-03 — список):**
- `ActsList.svelte`: bespoke `.rows`/`.loading`/`.empty`/`.pagination` заменены на `Table` (columns=4, skeleton на первичной загрузке, встроенный empty-state); `emptyConfig`-логика сохранена без изменений
- `ActListRow.svelte`: двухстрочный `<div class="row">` заменён на 4-колоночный `TableRow` (№/Дата/Получатель/Позиций); select-состояние теперь через проп `selected`
- `Table.svelte`/`TableRow.svelte` НЕ изменены — потребители (Устройства, `ActFormItemsTable`) не затронуты

**Task 3 (D-01 — деталь):**
- `ActDetail.svelte`: bespoke `.act-detail`/`.detail-header`/`.section`/`.header-grid`/фон `--tr-bg` заменены на `DetailPanel`+`DetailSection`+`DetailField` (7 полей шапки, секция позиций, секция истории возвратов); все действия (Печать/Редактировать/Возврат/Удалить) и empty/loading-состояния сохранены
- `ActHeaderField.svelte` удалён — единственный потребитель мигрировал на `DetailField`

## Task Commits

Each task was committed atomically:

1. **Task 1: ActsPage header→PageHeader + ActsMasterDetail (D-02) + ActsSearchAndTabs (D-05)** - `ad9218f` (feat)
2. **Task 2: ActsList + ActListRow → Table/TableRow (D-03)** - `8c254ea` (feat)
3. **Task 3: ActDetail + ActHeaderField → DetailPanel (D-01)** - `687075b` (feat)

## Files Created/Modified

- `ui/src/features/acts/ActsPage.svelte` - шапка на `PageHeader`
- `ui/src/features/acts/ActsMasterDetail.svelte` - поверхности панелей на `--tr-surface-raised` + `--tr-elev-1`
- `ui/src/features/acts/ActsSearchAndTabs.svelte` - фильтр-табы на `Tabs`
- `ui/src/features/acts/ActsList.svelte` - список на `Table`
- `ui/src/features/acts/ActListRow.svelte` - строка на `TableRow`
- `ui/src/features/acts/ActDetail.svelte` - деталь на `DetailPanel`/`DetailSection`/`DetailField`
- `ui/src/lib/components/DetailField.svelte` - правка устаревшего комментария (ссылка на удалённый `ActHeaderField`)
- `ui/src/features/acts/ActHeaderField.svelte` - **удалён** (заменён `DetailField`)

## Decisions Made

- **Table.svelte/TableRow.svelte не трогать** (явное требование плана) — функциональные пробелы примитивов закрыты на стороне потребителя:
  - `Table`'s empty-состояние не имеет action-slot → существующие empty-state действия («+ Создать акт» / «Сбросить поиск») перенесены в `footer`-snippet
  - `TableRow` не форвардит `onclick`/`role`/`tabindex` на свой `<tr>` → row-click/keyboard-select повешен на `<td>`-ячейки: все ячейки кликабельны мышью, первая ячейка (`№`) — единственный keyboard tab-стоп с `role="button"`/`tabindex="0"`/`onkeydown`, зеркалируя прежний единственный tab-стоп на всей строке
- `DetailPanel.title` остаётся `string` (не `Snippet`) — заголовок детали акта потерял `tr-mono`-стилизацию номера (чисто типографская деталь, не поле/действие)
- `ActHeaderField.svelte` удалён, а не оставлен тонкой обёрткой — 0 внешних потребителей на момент миграции

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 — missing critical functionality] `Table` empty-state не поддерживает action-кнопку**
- **Found during:** Task 2
- **Issue:** План предполагал перенос empty-состояния целиком на `Table` (`emptyTitle`/`emptyBody`), но `Table.svelte` не имеет слота для действия — а исходный `ActsList` имел кнопки «+ Создать акт» / «Сбросить поиск» в empty-состоянии (существующее действие, SC #4 требует сохранения)
- **Fix:** Кнопка действия перенесена в `footer`-snippet `Table` (рендерится внутри рамки, под пустым сообщением), вместо правки shared-примитива
- **Files modified:** `ui/src/features/acts/ActsList.svelte`
- **Commit:** `8c254ea`

**2. [Rule 1 — bug] `TableRow` не форвардит DOM-события на `<tr>`**
- **Found during:** Task 2
- **Issue:** План предполагал повесить `onclick`/`onkeydown`/`role`/`tabindex` прямо на `<TableRow>` («onSelect вешаем на `<TableRow>`»), но `TableRow.svelte`'s `Props` не объявляет эти атрибуты и не форвардит их на свой `<tr>` — переданные пропсы были бы молча отброшены, и клик по строке перестал бы работать (регресс существующего действия)
- **Fix:** `onclick` повешен на каждую `<td>` (клик в любом месте строки работает), `role="button"`/`tabindex="0"`/`onkeydown` — на первой ячейке (`№`) как единственном keyboard-доступном входе, зеркалируя прежний единственный `<div role="button">`
- **Files modified:** `ui/src/features/acts/ActListRow.svelte`
- **Commit:** `8c254ea`

**3. [Rule 1 — bug] Устаревшие комментарии-упоминания удалённого `ActHeaderField`**
- **Found during:** Task 3, post-edit self-check
- **Issue:** Комментарии в `ActDetail.svelte` и уже существующем `DetailField.svelte` (артефакт 27-01) упоминали класс `.detail-header`/`ActHeaderField.svelte` буквальным текстом — после удаления `ActHeaderField.svelte` эти строки стали "фантомными" ссылками на несуществующий файл
- **Fix:** Комментарии переформулированы без буквального имени удалённого файла/класса
- **Files modified:** `ui/src/features/acts/ActDetail.svelte`, `ui/src/lib/components/DetailField.svelte`
- **Commit:** `687075b`

## Issues Encountered

None помимо задокументированных выше отклонений.

## User Setup Required

None - изменения чисто фронтенд, дополнительной конфигурации не требуется.

## Human Verification Recommended

Плановые human-check пункты (обе темы, светлая/тёмная) не выполнены автономным исполнителем — рекомендуется проверить вручную при следующем визуальном ревью фазы:
- Окно Актов в тёмной теме: master и detail-панели «всплывают» над контент-фоном (`--tr-surface-raised` светлее `--tr-surface`)
- Окно Актов в светлой теме: панели разделены рамкой/тенью (в светлой теме `--tr-surface-raised == --tr-surface`, разделение только через border+shadow)
- Табы фильтра переключаются, счётчики совпадают с прежними
- Список актов: клик по строке (в любой ячейке) выделяет её; клавиатурный Tab+Enter/Space на ячейке «№» тоже выделяет
- Пустое состояние списка: кнопка «+ Создать акт» / «Сбросить поиск» видна и кликабельна под таблицей
- Деталь акта: все 7 полей шапки, позиции, история возвратов, кнопки Печать/Редактировать/Возврат/Удалить работают как раньше

## Next Phase Readiness

Акты (WIN-03) структурный слой полностью на примитивах Фаз 24-25 + shared detail-panel Фазы 27-01. Паттерн row-click-без-правки-TableRow (onclick на `<td>`, keyboard-стоп на первой ячейке) и footer-based empty-action можно переиспользовать в планах 27-04 (Картриджи) и 27-07 (Принтеры) — оба используют идентичный `Table`/`TableRow` для своих master-списков.

---
*Phase: 27-core-workflow-windows*
*Completed: 2026-07-21*
