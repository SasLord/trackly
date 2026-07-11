---
phase: 18-autocomplete-dropdowns
verified: 2026-07-11T00:00:00Z
status: passed
score: 5/5 must-haves verified
overrides_applied: 0
re_verification:
  previous_status: human_needed
  previous_score: 5/5
  gaps_closed:
    - "WR-01: removeRow() теперь реиндексирует все 8 index-keyed transient-мап через shiftRowState() + чистит debounceTimers[idx] (9b0125a)"
    - "WR-02: Enter в member/drill-in ветке handleRowKeydown теперь preventDefault/stopPropagation — нет native form submit (6954e49)"
    - "WR-03: namespace портированных дропдаун-CSS (dropdown--person/location/device/items), :global-коллизии устранены (b2b480f)"
    - "WR-04: COUNT(DISTINCT COALESCE(d.condition,' ')) во всех 3 ветках list_grouped + новый регресс-тест (760a273)"
    - "WR-05: onDestroy-очистка debounce-таймеров в 4 компонентах (228a79e)"
  gaps_remaining: []
  regressions: []
human_verification: []
---

# Phase 18: Автокомплит и дропдауны — Verification Report

**Phase Goal:** Автокомплиты по всему приложению рендерят свой выпадающий список через portal в `body` (не обрезаются и не ломают вёрстку внутри модалок), а выбор устройства в форме акта работает полноценно: раскрывается по фокусу, фильтруется вводом, группирует одинаковые устройства с раскрытием деталей экземпляра и схлопывает единственную оставшуюся группу до плоского списка.

**Verified:** 2026-07-11
**Status:** passed (все обязательные truths подтверждены кодом/тестами; 5 WARNING-находок code review исправлены и перепроверены)
**Re-verification:** Yes — после закрытия 5 WARNING-находок (WR-01..05) из code review

## Re-verification Summary

Первичная верификация выставила `human_needed` из-за двух edge-case находок code review (WR-01, WR-02) в `ActFormItemsTable.svelte`, не покрытых сценарием финального human-checkpoint. Координатор сообщил, что все 5 WARNING-находок исправлены и закоммичены на main. Verifier независимо перепроверил каждый фикс в исходном коде (не по описанию коммитов):

| Находка | Коммит | Проверка в коде | Статус |
|---------|--------|-----------------|--------|
| WR-01 (removeRow не реиндексирует state-мапы) | 9b0125a | `ActFormItemsTable.svelte:118` `shiftRowState<T>()` хелпер; `removeRow()` (строки 135-142) вызывает его для всех 8 мап (`suggestionsByRow`/`loadingByRow`/`openByRow`/`viewModeByRow`/`drillGroupByRow`/`membersByRow`/`activeIndexByRow`/`showBackByRow`) + `delete debounceTimers[idx]` (строка 134) | ✓ ЗАКРЫТ |
| WR-02 (Enter в member-view → form submit) | 6954e49 | `ActFormItemsTable.svelte:364-374` — ветка `viewModeByRow[idx]==='members'` теперь при `e.key==='Enter'` вызывает `preventDefault()`+`stopPropagation()` перед `return` | ✓ ЗАКРЫТ |
| WR-03 (коллизия `:global(.dropdown)` CSS) | b2b480f | Namespaced классы подтверждены: `dropdown--location` (LocationAutocomplete:152), `dropdown--person` (PersonAutocomplete:218), `dropdown--device` (DeviceAutocompleteField:324), `dropdown--items` (ActFormItemsTable:530); `:global()`-правила скоупятся под namespace | ✓ ЗАКРЫТ |
| WR-04 (COUNT DISTINCT игнорирует NULL) | 760a273 | `devices_sqlite.rs:1014,1048,1084` — `COUNT(DISTINCT COALESCE(d.condition, ' '))` во всех 3 SQL-ветках; новый тест `grouping_condition_distinct_count_counts_null_as_distinct` (devices_grouping.rs:755) — PASS | ✓ ЗАКРЫТ |
| WR-05 (debounce-таймеры не отменяются на unmount) | 228a79e | `onDestroy` подтверждён в 4 компонентах: LocationAutocomplete:52, PersonAutocomplete:69, DeviceAutocompleteField:107, ActFormItemsTable:83 | ✓ ЗАКРЫТ |

**Проверки после фиксов (запущены verifier'ом независимо):**
- `cargo test -p trackly-app --test devices_grouping` → **24/24 passed** (было 23; +1 новый WR-04 тест)
- `pnpm --dir ui run svelte-check` → **0 errors** (242 файла, 38 baseline warnings)
- `pnpm --dir ui run build` → **success**
- `git status --short` → чисто (кроме нового файла верификации)

Регрессий не обнаружено — основная цепочка AUTO-01..05 остаётся полностью реализованной.

## Goal Achievement

### Observable Truths (Roadmap Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Любой автокомплит внутри модалки разворачивает список через portal в `body`, без обрезки/лишнего скролла/искажения вёрстки | ✓ VERIFIED | `ui/src/lib/utils/portal.ts` (перенос узла в `body`) + `ui/src/lib/utils/dropdownAnchor.ts` (fixed-позиционирование от `getBoundingClientRect()`, капча-фаза scroll, флип вверх) применены во всех 4 кастомных дропдаунах: `LocationAutocomplete.svelte`, `PersonAutocomplete.svelte`, `DeviceAutocompleteField.svelte`, `ActFormItemsTable.svelte` (все с namespaced CSS после WR-03). 4 native-`<select>` компонента задокументированы AUTO-01-комментарием и подтверждены отсутствием `role="listbox"`/`class="dropdown"`. Финальный human-checkpoint (Plan 18-05 Task 3) подтвердил визуально — **approved**. |
| 2 | В форме акта поле выбора устройства раскрывает список сразу при фокусе, без ввода | ✓ VERIFIED | `ActFormItemsTable.svelte` — `handleFocus(idx)` вызывает `fetchGroups(idx, query)` с delay 0; `<input onfocus>`; ранний return `v.trim().length < 1` удалён; backend возвращает top-20 по `count DESC` при пустом `name_prefix`. Подтверждено human-checkpoint (approved). |
| 3 | Ввод текста фильтрует список по наименованию в реальном времени | ✓ VERIFIED | Frontend debounce 250мс → `devices.listGrouped({..., name_prefix, group_by_condition: true})`. Backend `sql_grouped_by_model_with_query` фильтрует по `devices_fts MATCH ?4` (name+inv#+SN+model), санитизировано `build_fts_query`. Тесты `grouping_true_branch_filters_by_name_text`/`_by_inventory_and_serial`/`_query_sanitizes_special_chars` — PASS (24/24). |
| 4 | Одинаковые по наименованию устройства объединены в раскрываемую группу; раскрыв, пользователь видит и выбирает конкретный экземпляр (инв.№/SN/модель/состояние) | ✓ VERIFIED | Backend `list_grouped(group_by_condition=true)` группирует по `(type_id, name, model)` (D-05); `condition_distinct_count` теперь корректно считает NULL как отдельное значение (WR-04 fix) — сигнал drill-in надёжен. Frontend `isExpandable`/`drillInto`/`memberRows`/`pickDevice` + кнопка «← Назад». Подтверждено human-checkpoint (approved). |
| 5 | Единственная группа после фильтрации не отображается как группа — сразу плоский список её устройств | ✓ VERIFIED | `fetchGroups()`: `if (filtered.length === 1) { await drillInto(idx, filtered[0], false); }` — `showBack=false` подавляет «← Назад». Подтверждено human-checkpoint (approved). |

**Score:** 5/5 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/trackly-infra/src/repos/devices_sqlite.rs` | `list_grouped()` triple-mode; false-branch untouched; NULL-safe condition count | ✓ VERIFIED | 3 static SQL constants; `GROUP BY d.type_id, d.name, d.model` / `ORDER BY cnt DESC, d.name ASC` / `devices_fts MATCH ?4`; `COUNT(DISTINCT COALESCE(d.condition,' '))` (WR-04). `sql_without_condition` unchanged; `DevicesPage.svelte` still `group_by_condition: false`. |
| `crates/trackly-app/tests/devices_grouping.rs` | Regression + new tests incl. WR-04 | ✓ VERIFIED | 24/24 pass, включая `grouping_condition_distinct_count_counts_null_as_distinct`. |
| `ui/src/lib/utils/dropdownAnchor.ts` | Reusable use-action: fixed position, capture-scroll/resize, upward flip | ✓ VERIFIED | Exports `dropdownAnchor`/`DropdownAnchorParams`; capture-phase scroll listener; `destroy()` removes both. |
| `ui/src/lib/components/LocationAutocomplete.svelte` | portal+anchor, namespaced CSS, onDestroy cleanup | ✓ VERIFIED | `dropdown--location`, dual-ref click-outside, `onDestroy` таймер-cleanup. |
| `ui/src/lib/components/PersonAutocomplete.svelte` | portal+anchor migration | ✓ VERIFIED | `dropdown--person`, `onDestroy` cleanup. |
| `ui/src/features/devices/DeviceAutocompleteField.svelte` | portal+anchor, header untouched | ✓ VERIFIED | `dropdown--device`, `maxHeight: 200`, `onDestroy` cleanup. |
| `Select/CartridgeSelect/GroupedPrinterSelect/PrinterSelect.svelte` | AUTO-01 doc-comment, no hidden overlay | ✓ VERIFIED | `grep -l "AUTO-01"` → все 4; overlay-разметка отсутствует. |
| `ui/src/features/acts/ActFormItemsTable.svelte` | Portal+anchor per-row, focus-open, filter, D-05, drill-in, auto-flatten, state-hygiene | ✓ VERIFIED | Все поведения прослежены в коде; `removeRow` реиндексация (WR-01), Enter-подавление (WR-02), `dropdown--items` namespace (WR-03), `onDestroy` (WR-05); svelte-check/build чистые. |

### Key Link Verification

| From | To | Via | Status |
|------|-----|-----|--------|
| `ActFormItemsTable.svelte` | `dropdownAnchor.ts` | `use:dropdownAnchor={{ anchorEl: rowInputEls[idx] }}` | ✓ WIRED |
| `ActFormItemsTable.svelte` | `devices.listGrouped` | `group_by_condition: true, name_prefix: query` | ✓ WIRED |
| `ActFormItemsTable.svelte` | `devices.listByIds` | `devices.listByIds(ids)` в `drillInto()` | ✓ WIRED |
| `LocationAutocomplete/PersonAutocomplete/DeviceAutocompleteField` | `dropdownAnchor.ts` / `portal.ts` | `use:dropdownAnchor` / `use:portal` | ✓ WIRED |
| `list_grouped (true-branch, text)` | `devices_fts` | `JOIN devices_fts ... AND devices_fts MATCH ?4` | ✓ WIRED |
| `list_grouped` | `build_fts_query` | direct call, sanitizer reuse, bound param | ✓ WIRED |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Backend grouping/sort/filter/NULL-condition regression suite | `cargo test -p trackly-app --test devices_grouping` | 24 passed; 0 failed | ✓ PASS |
| Frontend type-check | `pnpm --dir ui run svelte-check` | 242 files, 0 errors, 38 pre-existing warnings | ✓ PASS |
| Frontend build | `pnpm --dir ui run build` | Success | ✓ PASS |
| `DevicesPage.svelte` false-branch contract | `grep group_by_condition` | 2 call sites, both `false` | ✓ PASS |
| Working tree clean | `git status --short` | only new VERIFICATION.md | ✓ PASS |

### Requirements Coverage

| Requirement | Source Plan | Status | Evidence |
|-------------|------------|--------|----------|
| AUTO-01 | 18-02/03/04 | ✓ SATISFIED | portal+anchor во всех 4 кастомных дропдаунах (namespaced после WR-03) + 4 native-select задокументированы; human-checkpoint approved. |
| AUTO-02 | 18-04 | ✓ SATISFIED | `handleFocus`/`fetchGroups` delay-0; ранний return убран. |
| AUTO-03 | 18-01/04 | ✓ SATISFIED | Backend FTS5 фильтр + frontend `name_prefix`; тесты зелёные. |
| AUTO-04 | 18-04/05 | ✓ SATISFIED | Группировка name+model + drill-in; NULL-safe `condition_distinct_count` (WR-04); human-checkpoint approved. |
| AUTO-05 | 18-05 | ✓ SATISFIED | Single-group auto-flatten; human-checkpoint approved. |

Все 5 requirement ID (`AUTO-01..05`) присутствуют в REQUIREMENTS.md с `[x]`; orphaned requirements не обнаружены.

### Anti-Patterns Found

Все 5 WARNING-находок из `18-REVIEW.md` (WR-01..05) исправлены и перепроверены в коде (см. таблицу Re-verification Summary выше). Остаётся 5 Info-находок (advisory, не блокирующие): IN-01 (`dropdownAnchor` не репозиционируется на async-изменение высоты контента), IN-02 (dead CSS `.hint-warn`), IN-03 (`pickGroup` label опускает SN при наличии inv+SN у singleton), IN-04 (модуло клавнавигации может дать NaN при пустом open-списке — сейчас недостижимо), IN-05 (`list_grouped` repr смешивает MIN(id)+MAX() агрегаты). Все Info-находки — awareness-only; ни одна не влияет на достижение цели фазы. TBD/FIXME/XXX-маркеров без ссылки на issue в изменённых файлах не найдено.

## Gaps Summary

Гэпов не осталось. Основная цепочка AUTO-01..05 полностью реализована, прослежена в коде от backend SQL-диспетчера (`list_grouped` triple-mode) до frontend drill-in/auto-flatten UI, финальный human-checkpoint пройден и одобрен пользователем. Пять WARNING-находок code review, из-за которых первичная верификация выставляла `human_needed`, исправлены отдельными атомарными коммитами (9b0125a, 6954e49, b2b480f, 760a273, 228a79e) и независимо перепроверены verifier'ом: код каждого фикса подтверждён grep'ом/чтением, регресс-тесты зелёные (24/24, +1 новый), svelte-check/build чистые, дерево чистое. Регрессий нет. Статус фазы — **passed**.

---

_Verified: 2026-07-11 (re-verification after WR-01..05 closure)_
_Verifier: Claude (gsd-verifier)_
