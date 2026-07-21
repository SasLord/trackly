---
phase: 27-core-workflow-windows
plan: 09
subsystem: ui
tags: [svelte5, scss-tokens, design-system, both-theme-uat, acts, cartridges, printers]

# Dependency graph
requires:
  - phase: 27-core-workflow-windows
    provides: 27-01..27-08 — структурный слой всех трёх окон (Акты/Картриджи/Принтеры) на Table/TableRow/DetailPanel/Tabs/PageHeader
provides:
  - Финальный build-гейт Фазы 27 (check-tokens/svelte-check/lint/build) — все зелёные
  - Живой both-theme визуальный UAT (D-01..D-05) по трём окнам — approved пользователем
  - 24 fix-коммита (батчи B–G) устраняющие визуальные дефекты, найденные при живом UAT
  - Фаза 27 полностью исполнена (9/9 планов), WIN-03/04/05 закрыты
affects: [Phase 28 (Заявки/Отчёты/Настройки/Пользователи — унаследует Dropdown-миграцию нативных select), milestone v1.2 finish-up]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Master-detail панели: fillHeight-проп + внутренний скролл вместо растяжения всей страницы — окно не даёт странице скроллиться, скроллится только список/деталь"
    - "Выделенная строка таблицы: inset box-shadow вместо border — не сдвигает текст ячейки при выборе (был сдвиг на 1px при добавлении border)"
    - "Sticky-шапка панели деталей (DetailPanel) через position: sticky на заголовке, а не position: fixed — корректно скроллится вместе с внутренним контентом панели"
    - "Единый фон form-контролов через var(--tr-surface-raised) на Input/Select/PersonAutocomplete/LocationAutocomplete/qty-полях — устранён разнобой фонов внутри одной формы"
    - "Картриджи: нативный <select> заменён на кастомный Dropdown-примитив (Фаза 25) — устраняет визуальный разнобой с остальными полями формы; аналогичная миграция для Фазы 28 отложена как отдельный пункт"

key-files:
  created: []
  modified:
    - ui/src/features/acts/ActDetail.svelte
    - ui/src/features/acts/ActFormBody.svelte
    - ui/src/features/acts/ActFormItemsTable.svelte
    - ui/src/features/acts/ActsList.svelte
    - ui/src/features/acts/ActsMasterDetail.svelte
    - ui/src/features/acts/ActsPage.svelte
    - ui/src/features/cartridges/CartridgeFilters.svelte
    - ui/src/features/cartridges/CartridgeFormBody.svelte
    - ui/src/features/cartridges/CartridgeListRow.svelte
    - ui/src/features/cartridges/CartridgesList.svelte
    - ui/src/features/cartridges/CartridgesMasterDetail.svelte
    - ui/src/features/cartridges/CartridgesPage.svelte
    - ui/src/features/cartridges/CompatibilityEditor.svelte
    - ui/src/features/cartridges/ModelFormModal.svelte
    - ui/src/features/cartridges/ModelListRow.svelte
    - ui/src/features/cartridges/ModelsList.svelte
    - ui/src/features/printers/PrinterListRow.svelte
    - ui/src/features/printers/PrintersList.svelte
    - ui/src/features/printers/PrintersMasterDetail.svelte
    - ui/src/features/printers/PrintersPage.svelte
    - ui/src/features/printers/PrintersSearchAndTabs.svelte
    - ui/src/lib/components/DatePicker.svelte
    - ui/src/lib/components/DetailPanel.svelte
    - ui/src/lib/components/Dropdown.svelte
    - ui/src/lib/components/Input.svelte
    - ui/src/lib/components/LocationAutocomplete.svelte
    - ui/src/lib/components/PersonAutocomplete.svelte
    - ui/src/lib/components/Select.svelte
    - ui/src/lib/components/Table.svelte
    - ui/src/lib/components/TableRow.svelte
    - ui/src/styles/_tokens.scss

key-decisions:
  - "Task 2 (both-theme визуальный UAT) НЕ auto-approved — реальный человек провёл живой UAT через cargo tauri dev в обеих темах по всем трём окнам и явно одобрил после раунда правок B–G"
  - "Между исходным чекпоинтом и одобрением применено 24 fix-коммита (батчи B–G) по замечаниям живого UAT — все не меняют backend/workflow (SC #4 сохранён), только визуальный слой"
  - "Картриджи: нативный <select> в CompatibilityEditor/ModelFormModal заменён на Dropdown-примитив Фазы 25 — устраняет визуальный разнобой; аналогичная миграция для окон Фазы 28 (Настройки/Дашборд/Отчёты/Пользователи) явно вынесена за границы этого плана и отложена (см. Deferred to milestone end)"

patterns-established:
  - "fillHeight + internal-scroll master-detail: окно не скроллит всю страницу, список/деталь скроллятся внутри себя с pinned footer — образец для будущих master-detail окон Фазы 28+"
  - "inset box-shadow для selected-row вместо border — предотвращает text-reflow при выборе строки"

requirements-completed: [WIN-03, WIN-04, WIN-05]

# Metrics
duration: ~4h57min (включая живой both-theme UAT пользователя и итеративные раунды правок B–G)
completed: 2026-07-21
---

# Phase 27 Plan 09: Финальный build-гейт + both-theme визуальный UAT Summary

**Три окна основного рабочего процесса (Акты/Картриджи/Принтеры) прошли финальный build-гейт и живой both-theme визуальный UAT пользователя; по итогам UAT применено 24 точечных визуальных fix-коммита (батчи B–G: fill-height master-detail, единый фон полей, кастомный Dropdown в Картриджах, sticky-шапка деталей, inset-selected-row и др.), после чего пользователь подтвердил: «Пока что хорошо, оставляем так».**

## Performance

- **Duration:** ~4h57min (2026-07-21T11:42:33Z → 2026-07-21T16:39:03Z; основная часть времени — живой both-theme UAT пользователя через `cargo tauri dev` + 6 итеративных раундов правок B–G между пользователем и агентом, не пассивное ожидание)
- **Started:** 2026-07-21T11:42:33Z
- **Completed:** 2026-07-21T16:39:03Z
- **Tasks:** 2 (Task 1 автоматический build-гейт; Task 2 блокирующий both-theme UAT, approved после раунда правок)
- **Files modified:** 31 (батчи B–G, ~24 коммита) + 3 planning-файла в финальном docs-коммите

## Accomplishments
- Task 1 (build-гейт) прошёл дважды — до начала UAT и повторно после батчей B–G: `check-tokens.mjs` 0 нарушений, `svelte-check` 0 ошибок (48 pre-existing warnings вне scope), `lint` (eslint+prettier+check-tokens) чисто, `pnpm --dir ui build` успешен (prebuild `cargo test -p trackly-app --test export_bindings` 1/1 passed), свежий `ui/dist` собран
- Живой both-theme UAT (не auto-approve): пользователь лично проверил через `cargo tauri dev` обе темы по всем трём окнам (Акты/Картриджи/Принтеры) — D-01 (детали), D-02 (master/detail на raised-фоне, критичный пункт после регресса D-13/D-17 Фазы 26), D-03 (списки-таблицы), D-04 (модалки/виджеты), D-05 (Tabs-фильтры), SC #4 (workflow не сломан)
- По итогам живого UAT применено 24 fix-коммита в 6 батчах (B–G):
  - **B** — master-detail fill-height + internal scroll + pinned footer; убрана двойная граница; тёмный row-hover был невидим (совпадал с фоном панели); selected-row text-shift; sticky vs scroll-away шапка деталей; колонки картриджей/принтеров не перекрывались
  - **C** — единый required-marker на №/Сдал/Принял; унификация фона полей формы акта под `--tr-surface-raised`; placeholder-цвет в LocationAutocomplete; вертикальное центрирование даты в DatePicker
  - **D** — selected-row accent через inset box-shadow (не border, чтобы не сдвигать текст); sticky-шапка панели деталей приклеена к верху скролла
  - **E** — placeholder device-picker унифицирован с остальными hint-цветами; единый фон/бордер/radius across Input/Person/Location/Select/qty-полей; вертикальное центрирование кнопки удаления позиции (×); «Найти принтеры» перенесена в шапку страницы Принтеры; Имя+IP принтера объединены в двухстрочную колонку; убран заголовок «Действия» в списке картриджей
  - **F/G** — читаемый порядок «бренд+название» в списке Моделей (бейджи вторично); нативный `<select>` заменён на кастомный `Dropdown` в окне Картриджей (CompatibilityEditor/ModelFormModal); унифицированы off-token фоны полей/дропдаунов в модалках картриджей
- Финальный build-гейт повторно подтверждён зелёным после всех правок (check-tokens/svelte-check/build), рабочее дерево чистое
- WIN-03 (Акты) отмечен complete в REQUIREMENTS.md (WIN-04/05 уже были complete из предыдущих планов)

## Task Commits

Each task was committed atomically (Task 2 включает fix-батчи, применённые между чекпоинтом и approval):

1. **Task 1: Полная сборка + токен/тип/lint гейты** — без диффа (только запуск проверок, все зелёные); коммита нет
2. **Task 2: Both-theme визуальный UAT (D-01/D-02/D-03/D-05, SC #4)** — 24 коммита `fix(27):`/`style(27):` между чекпоинтом и approval:
   `4f9be3c` `2038433` `a6faf2a` `a38a28e` `a3dfd93` `666dc37` `e386911` `bb9647d` `dd63357` `a8e4b56` `9158da3` `76dd52b` `7ae542e` `59f0760` `8364f52` `2d4d773` `322eebf` `b88d23e` `bc8c65b` `3d16abc` `0c43d68` `6130aae` `80d0b41` `fc61a06`

**Plan metadata:** см. финальный docs-коммит после этого summary

_Note: Task 1 — audit/verification-only, без диффа. Task 2 изначально дошёл до блокирующего чекпоинта (не auto-approved); между чекпоинтом и человеческим approval применены 24 коммита правок по прямым замечаниям живого UAT — это ожидаемая часть цикла "показать → получить замечания → исправить → повторно показать", не отклонение от плана._

## Files Created/Modified
- 31 файлов (список — см. `key-files.modified` в frontmatter): master-detail shell, list-rows, form-bodies трёх окон + shared-примитивы (`DetailPanel`, `Dropdown`, `Input`, `Select`, `Table`, `TableRow`, `DatePicker`, `PersonAutocomplete`, `LocationAutocomplete`) + `_tokens.scss`
- `.planning/REQUIREMENTS.md` — WIN-03 отмечен complete
- `.planning/phases/27-core-workflow-windows/27-09-SUMMARY.md` — этот файл

## Decisions Made
- Both-theme UAT не auto-approved несмотря на `workflow.auto_advance: true` в конфиге — визуальная проверка D-02 (raised-vs-surface в обеих темах) требует реальных человеческих глаз, чекпоинт `gate="blocking"` соблюдён буквально
- Fix-батчи B–G применялись напрямую по замечаниям живого UAT (не через повторное планирование) — это стандартный executor-цикл авто-фикса (Rule 1: визуальные баги/несоответствия дизайн-системе), задокументированный как единая цепочка коммитов в рамках Task 2
- Нативный `<select>` в окне Картриджей заменён на `Dropdown` (Фаза 25); аналогичная замена для окон Фазы 28 (Настройки/Дашборд/Отчёты/Пользователи) явно вынесена за границы этого плана — отдельный deferred-пункт (см. ниже)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] 24 визуальных дефекта, найденных живым both-theme UAT, исправлены до approval**
- **Found during:** Task 2 (both-theme визуальный UAT)
- **Issue:** Живой UAT выявил ряд визуальных несоответствий дизайн-системе: master-detail не заполнял высоту окна и скроллил всю страницу вместо внутреннего списка; двойная граница панелей; невидимый hover строки в тёмной теме; text-shift выделенной строки; шапка деталей то залипала, то нет непоследовательно; колонки картриджей/принтеров перекрывались; разнобой фонов form-контролов; нечитаемый список Моделей; нативный `<select>` визуально выбивался из остальных полей; и др. (полный список — см. «Accomplishments» батчи B–G)
- **Fix:** 24 точечных коммита `fix(27):`/`style(27):`, сгруппированных в 6 батчей (B–G) по категориям дефекта; ни один не тронул backend/бизнес-логику/workflow — только SCSS/токены/структуру шаблона
- **Files modified:** 31 файл (см. key-files.modified)
- **Verification:** После каждого батча — повторный ручной просмотр пользователем в живом `cargo tauri dev`; после последнего батча (G) — полное повторное прохождение build-гейта (`check-tokens`/`svelte-check`/`build`) агентом + финальный визуальный approval пользователя в обеих темах
- **Committed in:** `4f9be3c`..`fc61a06` (24 коммита, см. Task Commits выше)

---

**Total deviations:** 1 группа auto-fixed (24 коммита, Rule 1 — визуальные баги найдены живым UAT, не архитектурные изменения)
**Impact on plan:** Все правки строго визуальные (SCSS/токены/разметка), backend и workflow не тронуты — SC #4 сохранён. Соответствует духу плана: Task 2 существует именно для того, чтобы поймать такие несоответствия перед закрытием фазы.

## Issues Encountered
None вне задокументированных выше auto-fix-правок.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- Фаза 27 (core-workflow-windows) полностью исполнена — 9/9 планов, все три окна основного рабочего процесса (Акты WIN-03, Картриджи WIN-04, Принтеры WIN-05) на новой дизайн-системе, both-theme UAT approved пользователем
- Build-гейт зелёный: `check-tokens`/`svelte-check`/`lint`/`build` — 0 нарушений/ошибок
- REQUIREMENTS.md: WIN-03/04/05 все complete
- Milestone v1.2 продолжается Фазой 28 (Заявки/Отчёты/Настройки/Пользователи) — унаследует паттерн миграции нативных `<select>` → `Dropdown`, применённый в этом плане для Картриджей

## Deferred to milestone end

Пункты, явно отложенные пользователем/координатором до финальной стадии milestone v1.2 (конкретика будет собрана позже, не блокируют закрытие Фазы 27):

1. **Дополнительные мелкие визуальные правки по трём окнам Фазы 27** — у пользователя есть ещё несколько небольших визуальных замечаний по Актам/Картриджам/Принтерам, которые он планирует донести и доработать в конце milestone v1.2 (после того как все окна будут переведены на дизайн-систему и будет видна полная картина).
2. **Миграция нативных `<select>` → кастомный `Dropdown`-примитив в окнах Фазы 28** (Настройки/Дашборд/Отчёты/Пользователи) — по аналогии с миграцией, выполненной в этом плане для окна Картриджей (батч F/G, коммит `80d0b41`); явно вне границ Фазы 27, будет решаться в контексте Фазы 28 или отдельным quick-task.
3. **Слияние `PersonAutocomplete` + `LocationAutocomplete` в единый переиспользуемый компонент** — сейчас это две отдельные реализации, визуально приведённые к идентичности (см. батч E, коммит `322eebf` и др.), но не объединённые архитектурно; кандидат на техдолг-задачу после закрытия milestone v1.2.

---
*Phase: 27-core-workflow-windows*
*Completed: 2026-07-21*

## Self-Check: PASSED

- FOUND: .planning/phases/27-core-workflow-windows/27-09-SUMMARY.md
- FOUND commits: 4f9be3c, 2038433, a6faf2a, a38a28e, a3dfd93, 666dc37, e386911, bb9647d, dd63357, a8e4b56, 9158da3, 76dd52b, 7ae542e, 59f0760, 8364f52, 2d4d773, 322eebf, b88d23e, bc8c65b, 3d16abc, 0c43d68, 6130aae, 80d0b41, fc61a06 (all 24 present in git log)
