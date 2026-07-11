---
phase: 18-autocomplete-dropdowns
plan: 05
subsystem: ui
tags: [svelte5, dropdown, drill-in, autocomplete, act-form, device-picker]

# Dependency graph
requires:
  - phase: 18-autocomplete-dropdowns
    provides: "Plan 18-04 (ActFormItemsTable portal+anchor device-picker: focus-open, real filtering, group-row name+model+×count, per-row keyboard nav, empty-state) + Plan 18-01 (list_grouped backend: name+model grouping, condition_distinct_count signal, multi-field FTS) + existing devices.listByIds command (per-instance detail source, no backend change)"
provides:
  - "Drill-in навигация (AUTO-04/D-06/D-07): клик по раскрываемой группе вызывает devices.listByIds(ids) и заменяет список группами на per-instance члены — серийные/инвентарные отдельными строками, несерийные/безынвентарные подгруппированы по state с ×count"
  - "Sticky-заголовок группы в member-view: название группы (repr.name · repr.model) закреплено position:sticky top:0 с фоном/тенью; кнопка «← Назад» — только при ручном drill-in"
  - "Single-group auto-flatten (AUTO-05/D-09): фильтрация до ровно одной группы сразу разворачивает её в плоский member-список со sticky-заголовком, без кнопки «← Назад»"
  - "D-08 нераскрываемая группа сохранена: несерийные/безынвентарные с одним condition И единственный экземпляр (ids.length===1) — прямой clone-выбор без drill-in"
  - "Смена текста фильтра сбрасывает view-mode строки обратно к списку групп (нет 'залипшего' member-списка)"
affects: [18-06, 19]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Per-row view-mode state machine (viewModeByRow: 'groups'|'members') для drill-in внутри N независимых пикеров одной таблицы — расширяет per-row ref-map паттерн Plan 18-04"
    - "Client-side партиционирование member-списка (memberRows): серийные/инвентарные → отдельные строки, несерийные/безынвентарные → Map-подгруппы по state — один devices.listByIds на drill-in, без повторных backend-запросов"
    - "Reserved fixed-width chevron-slot во всех типах строк дропдауна — column-alignment ×count-бейджей независимо от наличия drill-in стрелки"

key-files:
  created: []
  modified:
    - ui/src/features/acts/ActFormItemsTable.svelte

key-decisions:
  - "Единая логика auto-flatten (D-09) и обычного drill-in — единственная оставшаяся группа ВСЕГДА разворачивается через drillInto(showBack=false), а не рендерится как одна строка группы (упрощает код, идентичное поведение для раскрываемых и нераскрываемых)"
  - "Количество задаётся ТОЛЬКО в колонке «Количество» таблицы позиций — spinner из дропдауна убран (checkpoint fix #2); клик по member-строке выбирает устройство, qty правится потом (зеркалит pickGroup clone-семантику)"
  - "isExpandable требует ids.length > 1 — единственный экземпляр не помечается стрелкой и при клике сразу выбирается (checkpoint fix #4), даже при наличии serial_no/inventory_no"
  - "Sticky-заголовок группы показывается всегда в member-view (в т.ч. auto-flatten), уточняя D-09 — пользователь видит контекст группы; showBackByRow различает ручной drill-in (кнопка «← Назад») и auto-flatten (без неё)"

requirements-completed: [AUTO-04, AUTO-05]

# Metrics
duration: ~40min
completed: 2026-07-11
---

# Phase 18 Plan 05: ActFormItemsTable drill-in + single-group auto-flatten Summary

**Device picker в форме акта теперь раскрывает группу name+model в per-instance члены (серийные/инвентарные отдельными строками, несерийные — подгруппами по state) через devices.listByIds с sticky-заголовком группы и «← Назад», а единственная оставшаяся после фильтрации группа сразу схлопывается в плоский список — завершая цепочку AUTO-01..05.**

## Performance

- **Duration:** ~40 min (включая раунд checkpoint-исправлений)
- **Started:** 2026-07-11
- **Completed:** 2026-07-11
- **Tasks:** 3/3 (2 auto + 1 human-verify checkpoint)
- **Files modified:** 1

## Accomplishments

- **Drill-in навигация (AUTO-04/D-06/D-07):** `isExpandable(g)` определяет раскрываемость (ids.length>1 И [смешанный condition ИЛИ serial/inventory]); клик по раскрываемой группе вызывает `drillInto()` → `devices.listByIds(g.ids)` (с DEF-2A-дедупом) и заменяет список групп member-списком. `memberRows()` партиционирует членов client-side: серийные/инвентарные — отдельные строки, несерийные/безынвентарные — Map-подгруппы по state с ×count. `pickDevice()` зеркалит присваивания `pickGroup()` в `items[idx]`.
- **D-08 нераскрываемая группа сохранена:** несерийные/безынвентарные с одним condition, а также любой единственный экземпляр (ids.length===1) — прямой clone-выбор через существующий `pickGroup()` без изменений.
- **Single-group auto-flatten (AUTO-05/D-09):** `fetchGroups()` при `filtered.length===1` сразу вызывает `drillInto(showBack=false)` — плоский member-список со sticky-заголовком группы, без кнопки «← Назад».
- **Sticky-заголовок группы:** `position:sticky; top:0` с непрозрачным фоном и тенью — виден всегда в member-view, не просвечивает при прокрутке.
- **Сброс view-mode:** смена текста фильтра (`handleQueryInput`) возвращает строку к списку групп; `backToGroups()` — ручной возврат.
- **Автопроверка:** `svelte-check` (0 errors, 38 pre-existing warnings) + `build` зелёные; `ui/dist` обновлён для LAN-проверки.

## Task Commits

Each task was committed atomically:

1. **Task 1: Drill-in навигация — раскрытие группы, per-instance рендер (D-06/D-07)** - `f8d6816` (feat)
2. **Task 2: Single-group auto-flatten (AUTO-05/D-09) + сброс view-mode на новый ввод** - `72b65f7` (feat)
3. **Task 3: Финальная сквозная проверка AUTO-01..05** - checkpoint:human-verify → **approved** после раунда исправлений
   - **Checkpoint-fix (4 замечания ручной проверки)** - `efe0f99` (fix)

**Plan metadata:** (this commit, docs: complete plan)

## Files Created/Modified

- `ui/src/features/acts/ActFormItemsTable.svelte` — добавлены `viewModeByRow`/`drillGroupByRow`/`membersByRow`/`showBackByRow` state, `isExpandable()`/`drillInto()`/`handleGroupClick()`/`backToGroups()`/`memberRows()`/`pickDevice()` хелперы; member-view разметка (sticky drill-header + instance/subgroup строки); auto-flatten ветка в `fetchGroups()`; view-mode reset в `handleQueryInput()`; guard в `handleRowKeydown()` для member-режима; CSS для `.drill-header` (sticky), `.opt-chevron` (reserved slot), `.member-subgroup-label`.

## Decisions Made

См. `key-decisions` в frontmatter. Кратко: (1) единственная группа всегда разворачивается через тот же `drillInto`, что и обычный drill-in (единый код-путь); (2) количество — только в колонке таблицы, не в дропдауне; (3) `isExpandable` требует >1 экземпляра; (4) sticky-заголовок группы виден всегда в member-view.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Guard клавиатурной навигации в member-режиме**
- **Found during:** Task 1
- **Issue:** `handleRowKeydown` вычислял ArrowUp/Down/Enter/Tab по `visibleGroups(idx)` (список групп), но в member-режиме рендерится другой список (инстансы + подгруппы) — Enter выбрал бы неверный элемент.
- **Fix:** Ранний `return` из группового keyboard-пути при `viewModeByRow[idx] === 'members'`.
- **Files modified:** `ui/src/features/acts/ActFormItemsTable.svelte`
- **Verification:** `svelte-check` 0 errors; логика проверена вручную в чекпоинте.
- **Committed in:** `f8d6816` (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (Rule 1 — bug guard в рамках вводимой той же задачей функциональности).
**Impact on plan:** Без scope creep — предотвращает баг выбора неверного элемента в новом member-режиме.

## Checkpoint Verification (Task 3)

Финальный `checkpoint:human-verify` НЕ прошёл с первого раза — пользователь при ручной проверке нашёл 4 проблемы, все исправлены в `efe0f99` и подтверждены (**approved**):

1. **Sticky-заголовок группы** — при drill-in/auto-flatten не было видно, к какой группе относятся экземпляры. Добавлен `position:sticky top:0` заголовок с названием группы (виден всегда, в т.ч. auto-flatten — уточнение D-09); «← Назад» только при ручном drill-in.
2. **Убран spinner количества из дропдауна** — member-строки под-групп больше не содержат `<input type=number>`; количество задаётся только в колонке «Количество» таблицы. Под-группы снова рендерятся валидным `<button>` (убран div-workaround для вложенного инпута); удалены `subgroupQty`/`handleMemberQtyInput`/`memberQtyByRow`.
3. **Выравнивание ×count в столбик** — chevron-slot зарезервирован фиксированной шириной (12px) во всех типах строк (пустой у нераскрываемых/member-строк).
4. **Единственный экземпляр не раскрывается** — `isExpandable` возвращает false при `ids.length===1`; такие строки без стрелки, клик сразу выбирает. D-08 и DEF-2A сохранены.

Переименован `showDrillHeaderByRow` → `showBackByRow` (семантика сместилась: заголовок теперь всегда виден, флаг управляет только кнопкой «← Назад»).

**Round 2 UAT (ещё 2 дефекта отображения номеров, исправлены в `ef94e8a`):**

5. **Строка-ГРУППА не показывает SN/инв.№ представителя** — раскрываемая группа рендерила номер одного `repr` (вводило в заблуждение, у экземпляров номера свои). Теперь SN/инв.№ показываются только при `g.ids.length === 1` (одиночное устройство); внутри группы номера видны в drill-in, у каждого экземпляра свои.
6. **Оба номера, если оба заполнены** — одиночное устройство и серийные member-строки показывают `SN … · инв. …` (middot-разделитель) вместо прежнего mutually-exclusive `{:else if}`, показывавшего только SN. Добавлены `.opt-meta-row`/`.opt-sep` стили.

## Issues Encountered

None помимо 4+2 checkpoint-замечаний выше (все закрыты в `efe0f99` и `ef94e8a`).

## User Setup Required

None — внешняя конфигурация не требуется.

## Next Phase Readiness

Цепочка AUTO-01..05 завершена и принята пользователем end-to-end (portal-дропдаун, focus-open, фильтрация, drill-in группировка, single-group flatten). Клон-qty семантика (`MAX_CLONE_QTY`, `stock_available` cap) и DEF-2A дедуп сохранены без регрессии. `ui/dist` собран. Готово к следующему плану фазы 18 / переходу к Phase 19.

---
*Phase: 18-autocomplete-dropdowns*
*Completed: 2026-07-11*

## Self-Check: PASSED

- FOUND: ui/src/features/acts/ActFormItemsTable.svelte
- FOUND: .planning/phases/18-autocomplete-dropdowns/18-05-SUMMARY.md
- FOUND commit: f8d6816 (Task 1)
- FOUND commit: 72b65f7 (Task 2)
- FOUND commit: efe0f99 (checkpoint-fix)
