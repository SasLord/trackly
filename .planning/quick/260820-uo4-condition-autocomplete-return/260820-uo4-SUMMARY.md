---
phase: 260820-uo4-condition-autocomplete-return
plan: 01
subsystem: ui
tags: [svelte, autocomplete, acts, devices, dropdown]

# Dependency graph
requires: []
provides:
  - "DeviceAutocompleteField.svelte: field=\"state\" всегда мержит 6 стандартных вариантов состояния (STANDARD_STATES) в дропдаун, с префикс-фильтром и case/whitespace-insensitive де-дупом против backend suggestions"
  - "DeviceAutocompleteField.svelte: сквозной disabled-проп (input/textarea + CSS), тот же паттерн что в LocationAutocomplete.svelte"
  - "ReturnModal.svelte bulk-«Состояние» и ReturnItemsTable.svelte per-row-«Состояние» используют DeviceAutocompleteField вместо plain Input"
affects: [acts, devices, printers]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Стандартные дефолты дропдауна вставляются ТОЛЬКО на фронтенде, мержем в существующий $derived allItems — тот же паттерн, что уже применён для field=\"location\" (allLocationSuggestions)"
    - "Per-row disabled-семантика через условный рендер (DeviceAutocompleteField когда редактируемо / disabled Input-плейсхолдер иначе), а не через disabled-проп — зеркалит существующий паттерн LocationAutocomplete в ReturnItemsTable"

key-files:
  created: []
  modified:
    - ui/src/features/devices/DeviceAutocompleteField.svelte
    - ui/src/features/acts/ReturnModal.svelte
    - ui/src/features/acts/ReturnItemsTable.svelte

key-decisions:
  - "Стандартные варианты (Новое, Б/У, Хорошее, Среднее, Плохое, На списание) — статичный литеральный массив в DeviceAutocompleteField.svelte, backend не трогается"
  - "Open-гейтинг унифицирован на allItems.length > 0 (было: suggestions.length > 0 || allLocationSuggestions.length > 0) — иначе дефолты для field=\"state\" не открывали бы дропдаун без backend-подсказок"

patterns-established:
  - "Field-specific derived-merge в DeviceAutocompleteField (standardSuggestions для state, allLocationSuggestions для location) — модель для будущих field-специфичных дефолтов в этом компоненте"

requirements-completed: [UO4-01, UO4-02]

# Metrics
duration: 12min
completed: 2026-08-20
---

# Quick Task 260820-uo4: Автокомплит «Состояния» со стандартными дефолтами Summary

**Дропдаун «Состояние» в DeviceAutocompleteField теперь всегда показывает 6 стандартных вариантов (Новое/Б/У/Хорошее/Среднее/Плохое/На списание) с де-дупом против ранее введённых значений; Акты → Возврат (bulk и per-row) переведены на этот компонент вместо plain Input.**

## Performance

- **Duration:** ~12 min
- **Started:** 2026-08-20T15:05:00Z
- **Completed:** 2026-08-20T15:17:00Z
- **Tasks:** 2/2
- **Files modified:** 3

## Accomplishments

- `DeviceAutocompleteField.svelte` для `field="state"` мержит статичный `STANDARD_STATES` (6 значений) в `allItems`, с регистро/пробело-независимым де-дупом против backend `suggestions` и префикс-фильтром — эффект виден одновременно в трёх местах (Акты→Возврат bulk+per-row, попапы «Добавить/Редактировать устройство/принтер»), т.к. все три используют один компонент.
- Добавлен сквозной `disabled`-проп компонента (native `disabled` на `<input>`/`<textarea>` + CSS `:disabled`), по образцу `LocationAutocomplete.svelte`.
- Open-гейтинг (когда дропдаун реально открывается) унифицирован на единый источник `allItems.length > 0` в обоих местах (debounced fetch-эффект и `handleFocus()`), вместо дублировавшегося `suggestions.length > 0 || allLocationSuggestions.length > 0`.
- `ReturnModal.svelte` bulk-«Состояние»: `<Input>` → `<DeviceAutocompleteField field="state" disabled={!applyToAll} .../>`.
- `ReturnItemsTable.svelte` per-row-«Состояние»: условный рендер зеркальный уже существующему для «Расположение» — `DeviceAutocompleteField` когда `row.checked && !applyToAll`, иначе disabled-плейсхолдер `<Input disabled>`.

## Task Commits

Each task was committed atomically:

1. **Task 1: Стандартные дефолты «Состояния» + disabled-проп в DeviceAutocompleteField** - `8cd7dfb4` (feat)
2. **Task 2: Подключить DeviceAutocompleteField к «Состояние» в модале «Возврат»** - `428f98b9` (feat)

_Note: docs/state metadata commit created separately by orchestrator per constraints._

## Files Created/Modified

- `ui/src/features/devices/DeviceAutocompleteField.svelte` — `STANDARD_STATES` + `standardSuggestions` derived (field="state" only), `disabled` prop, унифицированный `open` gating, новый template-блок «Стандартные варианты:»
- `ui/src/features/acts/ReturnModal.svelte` — bulk-«Состояние» на `DeviceAutocompleteField`; убран ставший неиспользуемым импорт `Input`
- `ui/src/features/acts/ReturnItemsTable.svelte` — per-row-«Состояние» на условный рендер `DeviceAutocompleteField`/disabled `Input`; убран ставший неиспользуемым `{@const perRowDisabled}`

## Decisions Made

- Дефолты состояния — фронтенд-only статичный список внутри переиспользуемого компонента, без записи в БД/backend-изменений (зафиксировано ещё на этапе планирования, см. `<objective>` плана).
- `perRowDisabled` в `ReturnItemsTable.svelte` удалён целиком (был единственным читателем условия рендера, которое теперь выражено напрямую через `row.checked && !applyToAll`) — избегает мёртвого кода.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Удалён ставший неиспользуемым импорт `Input` в ReturnModal.svelte**
- **Found during:** Task 2 (верификация `svelte-check` после замены bulk-поля «Состояние»)
- **Issue:** План предполагал, что `Input` в `ReturnModal.svelte` используется где-то ещё («Кто возвращает»/«Кто принимает» через `PersonAutocomplete`), но фактически в файле не было других мест использования `Input` — после замены единственного использования на `DeviceAutocompleteField` импорт стал мёртвым, `svelte-check` падал с `'Input' is declared but its value is never read` (1 ERROR).
- **Fix:** Убрана строка `import Input from '$lib/components/Input.svelte';`.
- **Files modified:** `ui/src/features/acts/ReturnModal.svelte`
- **Verification:** `pnpm --dir ui run svelte-check` → 0 ERRORS (было 1); `pnpm --dir ui build` проходит.
- **Committed in:** `428f98b9` (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (1 bug — unused import removal)
**Impact on plan:** Минимальное — план чуть ошибся в описании интерфейса файла, фикс тривиален и не меняет поведение. No scope creep.

## Issues Encountered

None beyond deviation above.

## Runtime Verification Status

**UNVERIFIED (compile gates only)** — `svelte-check` (0 ошибок), `lint` (0 ошибок), `build` (успешно) все прошли чисто после обеих задач, но per project memory «Compile gates miss Svelte runtime» эти гейты НЕ доказывают, что дропдаун реально открывается/фильтруется в рантайме (реактивность `$derived`/`$effect` внутри debounce-колбэков не проверяется статически). У этого executor-агента нет инструментов браузерной/UI-автоматизации (нет Playwright/MCP browser tool) для клика по полям в реально запущенном `cargo tauri dev` (обнаружен уже запущенным процессом на машине пользователя, PID отдельно от этой сессии — не управляется этим агентом).

Требуется ручная UAT-проверка пользователем в живом приложении по чеклисту из `<verification>` плана:
- Акты → активный акт → «Возврат»: bulk-«Состояние» открывает дропдаун с 6 стандартными вариантами на пустом поле; префикс «б» сужает до вариантов на «Б»/«б».
- Выключение/включение «Применить ко всем» переключает per-row-«Состояние» между редактируемым автокомплитом и серым disabled-плейсхолдером — как «Расположение».
- Попап «Добавить устройство»/«Редактировать»: поле «Состояние» показывает 6 стандартных вариантов даже без истории ввода.
- Ранее введённое «Б/У» не дублируется в дропдауне.
- Кнопка «Сохранить» в «Возврате» по-прежнему недоступна без непустого bulk-«Состояние» при `applyToAll` (regression-check `canSubmit`).

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Функциональность полностью фронтенд-only и точечная — не блокирует и не требует дальнейших фаз. Единственное открытое — ручная UAT-проверка рантайма пользователем (см. выше), т.к. compile-гейты не покрывают Svelte 5 rune-реактивность.

---
*Phase: 260820-uo4-condition-autocomplete-return*
*Completed: 2026-08-20*

## Self-Check: PASSED

All created/modified files verified present on disk; both task commits (`8cd7dfb4`, `428f98b9`) verified present in git log.
