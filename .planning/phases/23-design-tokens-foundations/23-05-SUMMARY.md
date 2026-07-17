---
phase: 23-design-tokens-foundations
plan: 05
subsystem: ui
tags: [scss, design-tokens, css-custom-properties, svelte, typography]

# Dependency graph
requires:
  - phase: 23-01
    provides: "--tr-* typography token layer in ui/src/styles/_tokens.scss (9 roles + mono, composite + decomposed axes)"
  - phase: 23-02
    provides: "check-tokens.mjs permanent CI gate (3 rules)"
  - phase: 23-03
    provides: "--color-*/--shadow-* fully migrated (0 remaining) — clean separation, no overlap"
  - phase: 23-04
    provides: "--space-*/--radius-* fully migrated (0 remaining) — clean separation, no overlap"
provides:
  - "Все --font-size-*/--font-weight-*/--line-height-*/--font-family-base call-sites (99 файлов) переведены на --tr-* по роли, декомпозированные оси (D-14) — 0 нарушений в check-tokens.mjs --rules=1"
  - "3 fallback-паттерна (--font-size-page-title/-subheading/-caption) переписаны как целые выражения на var(--tr-font-size-h3/-body/-caption) — не просто внутренний токен"
  - "--font-size-sm QA-01 undefined-token баг закрыт в PersonAutocomplete.svelte:312 -> --tr-font-size-caption"
  - "class=\"tr-mono\" применён на 9 конкретных in-scope call-sites (7 файлов из плана + 1 доп. файл ReturnItemsTable.svelte) для инв./серийный №/№ акта"
  - "ReturnRowState (ReturnModal.svelte/ReturnItemsTable.svelte) декомпозирован из единой deviceLabel: string в deviceName + inventoryNo — открывает markup-шов для точечного tr-mono без @html"
affects: [23-06]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Одноразовый Node-скрипт литеральной ordered-map замены (String.split/join, не regex) — безопасен, т.к. полный набор старых типографических имён токенов не имеет substring-коллизий (--font-size-body не является префиксом другого имени и т.д.), подтверждено git grep до старта sweep"
    - "tr-mono всегда как ОТДЕЛЬНЫЙ вложенный <span class=\"tr-mono\">, а не добавление в существующий multi-class атрибут — гарантирует греп-видимость точного литерала class=\"tr-mono\" (плановая acceptance criteria использует именно этот паттерн, не substring-поиск токена tr-mono)"
    - "Декомпозиция plain-text label-строки (deviceLabel) в структурные поля (deviceName + inventoryNo) — единственный безопасный способ точечно замонопространить сегмент внутри строки, интерполируемой как plain text (не @html); альтернатива @html отвергнута как XSS-риск на пользовательских инвентарных номерах"

key-files:
  created: []
  modified:
    - "99 файлов ui/src/**/*.svelte — литеральный sweep 16-точечной карты (font-size/font-weight/line-height/font-family), 4 коммита-пакета по ~25 файлов"
    - "ui/src/lib/components/PersonAutocomplete.svelte:312 — QA-01 fix (--font-size-sm -> --tr-font-size-caption)"
    - "ui/src/features/acts/ActItemsTable.svelte, ui/src/features/devices/DeviceListRow.svelte, ui/src/features/printers/PrinterDetail.svelte, ui/src/features/acts/DocumentAcceptanceModal.svelte, ui/src/features/acts/ActFormItemsTable.svelte, ui/src/features/acts/ActDetail.svelte, ui/src/features/acts/ActListRow.svelte — class=\"tr-mono\" на 9 сайтах"
    - "ui/src/features/acts/ReturnModal.svelte, ui/src/features/acts/ReturnItemsTable.svelte — ReturnRowState рефакторинг (deviceLabel -> deviceName+inventoryNo) для tr-mono seam"

key-decisions:
  - "ReturnModal.svelte само по себе НЕ содержит буквальный class=\"tr-mono\" — реальный рендер списка позиций возврата делегирован дочернему ReturnItemsTable.svelte (импортируется и используется через <ReturnItemsTable items={rows} .../>). Плановый acceptance criterion «git grep -n 'class=\"tr-mono\"' ReturnModal.svelte даёт хотя бы 1 совпадение» не может быть выполнен буквально без архитектурного изменения (инлайнить рендер таблицы в ReturnModal или использовать risky @html) — задокументировано как deviation, функциональная цель D-13 достигнута в дочернем компоненте"
  - "9 mono-вставок реализованы как отдельный вложенный <span class=\"tr-mono\">, а не добавление tr-mono в существующий multi-class атрибут — иначе плановая точная греп-проверка 'class=\"tr-mono\"' не находит комбинированные атрибуты вида class=\"td col-inv tr-mono\""
  - "TemplateEditor.svelte (упомянут в плане по неверному пути ui/src/lib/components/) фактически лежит в ui/src/features/settings/ — путь скорректирован при верификации D-16 exclusion (0 tr-mono подтверждено по реальному пути)"

requirements-completed: [DS-03, QA-01]

duration: ~20min
completed: 2026-07-17
---

# Phase 23 Plan 05: Типографика по роли + .tr-mono на идентификаторах Summary

**Все ~350 call-sites типографики (`--font-size-*`/`--font-weight-*`/`--line-height-*`/`--font-family-base`) в 99 файлах переведены на декомпозированные `--tr-*` оси по роли (1:1, без перехода на composite shorthand), 3 fallback-выражения переписаны целиком, `--font-size-sm` QA-01-баг закрыт, и `class="tr-mono"` применён на 9 конкретных сайтах отображения инв./серийного №/№ акта.**

## Performance

- **Duration:** ~20 min
- **Completed:** 2026-07-17
- **Tasks:** 2/2 completed
- **Files modified:** 101 (99 sweep + ReturnModal.svelte + ReturnItemsTable.svelte, с пересечением в подсчёте по коммитам)

## Accomplishments
- Task 1: mechanical literal ordered-map sweep 16-точечной карты применён 4 пакетами по ~25 файлов (99 файлов), с промежуточной верификацией `check-tokens.mjs --rules=1` после каждого пакета (517 -> 374 -> 240 -> 64 -> 0)
- 3 fallback-выражения (`--font-size-page-title`/`-subheading`/`-caption`) переписаны как ЦЕЛЫЕ выражения на `var(--tr-font-size-h3/-body/-caption)` — не просто замена внутреннего токена, точно как требовал порядок карты (пункты 1-3 перед 5-16)
- `--font-size-sm` (QA-01 undefined-token баг) закрыт в `PersonAutocomplete.svelte:312` -> `--tr-font-size-caption`
- `check-tokens.mjs --rules=1` -> **PASS — 0 нарушений** (последнее из 3 типографических/цветовых/space-radius семейств, завершает всю миграцию плана 23)
- Task 2: `class="tr-mono"` применён на 9 сайтах в 7 плановых файлах + 1 доп. файл (`ReturnItemsTable.svelte`, см. Decisions) — все как отдельный вложенный `<span class="tr-mono">`, не примесь к существующему multi-class атрибуту
- D-16 grey-area исключения подтверждены чистыми: `ActFormModal.svelte`, `ActsPage.svelte`, `TemplateEditor.svelte` (реальный путь `features/settings/`, не `lib/components/` как в плане) — 0 `tr-mono`
- `pnpm svelte-check` -> 0 errors, 48 pre-existing warnings (неизменный baseline); `pnpm build` -> успешно (461ms, только pre-existing предупреждения о неиспользуемых CSS-селекторах/dynamic import, не связанные с этим планом)
- Побочный эффект: `ActFormItemsTable.svelte` (один из 7 файлов с pre-existing prettier-дрейфом, задокументированных в `deferred-items.md` плана 23-02) теперь prettier-чист — попал под `prettier --write` при форматировании Task 2 правок; остальные 6 файлов дрейфа не тронуты (вне scope)

## Task Commits

Каждая задача закоммичена атомарно (Task 1 — 4 коммита-пакета; Task 2 — 1 коммит):

1. **Task 1 (batch 1/4): acts/auth/cartridges** - `6c3ce19` (feat)
2. **Task 1 (batch 2/4): cartridges/dashboard/devices/layout/printers** - `473358a` (feat)
3. **Task 1 (batch 3/4): printers/reports/requests/settings/users** - `2564463` (feat)
4. **Task 1 (batch 4/4): users/lib/pages + QA-01 fix** - `6cc5d14` (feat)
5. **Task 2: class="tr-mono" на 9 сайтах (DS-03)** - `68c3c89` (feat)

_Плановая docs-метадата коммитится отдельно ниже (final_commit)._

## Files Created/Modified
- 99 файлов `ui/src/features/**/*.svelte` + `ui/src/lib/**/*.svelte` + `ui/src/pages/*.svelte` + `ui/src/App.svelte` — механический sweep 16-точечной карты (Task 1, коммиты `6c3ce19`..`6cc5d14`)
- `ui/src/lib/components/PersonAutocomplete.svelte` — QA-01 `--font-size-sm` -> `--tr-font-size-caption` fix (в составе batch 4/4)
- `ui/src/features/devices/DeviceImportCsvModal.svelte` — caption fallback-выражение `var(--font-size-caption, 12px)` -> `var(--tr-font-size-caption)` (batch 2/4)
- `ui/src/pages/SettingsPage.svelte` + 6 Page-файлов (`CartridgesPage`/`ActsPage`/`RequestsPage`/`PrintersPage`/`UsersPage`/`DevicesPage`) — page-title fallback-выражение (batch 1-4, по расположению файла)
- `ui/src/features/acts/ActDetail.svelte`/`ReturnModal.svelte` — subheading fallback-выражение (batch 1/4)
- `ui/src/features/acts/ActItemsTable.svelte`, `ui/src/features/devices/DeviceListRow.svelte`, `ui/src/features/printers/PrinterDetail.svelte`, `ui/src/features/acts/DocumentAcceptanceModal.svelte`, `ui/src/features/acts/ActFormItemsTable.svelte`, `ui/src/features/acts/ActDetail.svelte`, `ui/src/features/acts/ActListRow.svelte` — `class="tr-mono"` на 9 сайтах (Task 2)
- `ui/src/features/acts/ReturnModal.svelte`, `ui/src/features/acts/ReturnItemsTable.svelte` — `ReturnRowState.deviceLabel: string` разложен на `deviceName: string` + `inventoryNo: string | null`; сам tr-mono-рендер живёт в `ReturnItemsTable.svelte` (Task 2, deviation)

## Decisions Made
- Порядок ordered-карты (пункты 1-3 fallback-выражения ПЕРЕД пунктами 5-16 bare-token замен) соблюдён буквально — подтверждено, что полный набор старых типографических имён токенов в кодовой базе не имел substring-коллизий (напр. нет `--font-size-body-strong` бы сломанного generic-заменой `--font-size-body`), поэтому единый Node-скрипт с последовательным `String.split/join` по всем 16 правилам безопасен и не требует построчного вмешательства per-файл
- `class="tr-mono"` реализован ВСЕГДА как отдельный вложенный `<span class="tr-mono">`, не примешан к существующему multi-class атрибуту ячейки/строки — иначе плановая acceptance-проверка `git grep 'class="tr-mono"'` (точный литерал) не находит комбинированные атрибуты вида `class="td col-inv tr-mono"`
- `ReturnModal.svelte` архитектурно не рендерит список позиций возврата инлайн — делегирует дочернему `ReturnItemsTable.svelte`. Плановый acceptance criterion, ожидающий `class="tr-mono"` буквально в `ReturnModal.svelte`, не мог быть выполнен без либо (а) инлайнинга markup обратно в родителя (архитектурный откат Phase 3.1), либо (б) `{@html}` на пользовательском инвентарном номере (XSS-риск). Выбран путь (в): декомпозировать `deviceLabel: string` на структурные `deviceName`/`inventoryNo` поля в `ReturnRowState`, дав `ReturnItemsTable.svelte` markup-шов для точечного `<span class="tr-mono">{row.inventoryNo}</span>` — функциональная цель D-13 достигнута, но грепаемый литерал физически находится в файле-компоненте, а не в файле-контейнере
- `TemplateEditor.svelte` в плановом тексте указан по несуществующему пути `ui/src/lib/components/` — реальный путь `ui/src/features/settings/TemplateEditor.svelte`; verification D-16 exclusion выполнена по актуальному пути (0 tr-mono подтверждено)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] ReturnModal.svelte acceptance criterion не мог быть выполнен буквально из-за компонентной границы**
- **Found during:** Task 2 (применение tr-mono на ReturnModal.svelte)
- **Issue:** План ожидает `git grep -n 'class="tr-mono"' .../ReturnModal.svelte` — хотя бы 1 совпадение. Но реальный рендер списка позиций возврата (где отображается `инвентарный №`) находится в дочернем компоненте `ReturnItemsTable.svelte`, который `ReturnModal.svelte` импортирует и вызывает через `<ReturnItemsTable items={rows} .../>`. `deviceLabel` в `ReturnModal.svelte` — чистая JS-строка (`${it.device_name} (инв. ${it.inventory_no})`), интерполируемая как plain text в дочернем компоненте — оборачивание в `<span>` внутри JS-строки рендерило бы буквальный текст `<span>`, а не элемент
- **Fix:** Декомпозирован `ReturnRowState.deviceLabel: string` на `deviceName: string` + `inventoryNo: string | null` (тип экспортируется из `ReturnItemsTable.svelte`, используется в 3 местах построения rows в `ReturnModal.svelte`). `ReturnItemsTable.svelte` теперь рендерит `{row.deviceName}` + условно `(инв. <span class="tr-mono">{row.inventoryNo}</span>)` либо `#{row.deviceId}` (fallback без mono — device_id не входит в D-13 scope: инв./серийный №/№ акта)
- **Files modified:** `ui/src/features/acts/ReturnModal.svelte`, `ui/src/features/acts/ReturnItemsTable.svelte`
- **Verification:** `pnpm svelte-check` -> 0 errors (не изменился baseline); `git grep -c 'class="tr-mono"' ui/src/features/acts/ReturnItemsTable.svelte` -> 1; визуальная цель D-13 (инв. № моноширинным) достигнута функционально, хотя грепаемый файл — дочерний компонент, не сам `ReturnModal.svelte`
- **Committed in:** `68c3c89` (Task 2 commit)

**2. [Rule 1 - Bug] Комбинированные class-атрибуты не проходили бы точный греп-критерий плана**
- **Found during:** Task 2, после первого прохода правок (ActItemsTable/DeviceListRow/PrinterDetail/ActListRow изначально получили `class="td col-inv tr-mono"` стиль)
- **Issue:** Плановая acceptance criteria использует точный литерал `git grep 'class="tr-mono"'` (не `git grep 'tr-mono'` с последующим разбором) — комбинированный атрибут `class="td col-inv tr-mono"` не матчится этим паттерном, хотя стилизация работала бы идентично в браузере
- **Fix:** Все 4 сайта переписаны на вложенный `<span class="tr-mono">` внутри существующей ячейки/span, оставляя внешний класс нетронутым (`td col-inv`, `cell cell-numeric`, `meta-value`, `number`)
- **Files modified:** `ui/src/features/acts/ActItemsTable.svelte`, `ui/src/features/devices/DeviceListRow.svelte`, `ui/src/features/printers/PrinterDetail.svelte`, `ui/src/features/acts/ActListRow.svelte`
- **Verification:** `git grep -c 'class="tr-mono"' <каждый файл>` -> ≥1 для всех 4; `pnpm exec prettier --write` подтвердил каноничное форматирование (0 diff после write для 3 из 4, 1 небольшая перестройка для PrinterDetail.svelte)
- **Committed in:** `68c3c89` (Task 2 commit)

---

**Total deviations:** 2 auto-fixed (2 Rule 1 — оба обнаружены и закрыты в рамках acceptance-criteria верификации Task 2, не изменили функциональную цель D-13/DS-03)
**Impact on plan:** Оба отклонения — фикс несоответствия между плановой грепаемой acceptance-проверкой и фактической структурой компонентов/атрибутов. Функциональный результат (моноширинное отображение инв./серийного №/№ акта на всех 9 плановых сайтах) достигнут полностью; единственный побочный эффект — грепаемый литерал для позиций возврата физически находится в `ReturnItemsTable.svelte`, а не в `ReturnModal.svelte`, что задокументировано выше.

## Issues Encountered
None, помимо описанных выше 2 отклонений.

## User Setup Required
None - no external service configuration required.

## Known Stubs
None — это чисто CSS-токен sweep + точечное CSS-класс применение, новых компонентов/данных/UI-состояний не добавлено. Рефакторинг `ReturnRowState` (deviceLabel -> deviceName+inventoryNo) — чисто внутреннее структурное изменение существующих данных, без изменения видимого поведения (кроме добавления mono-стиля).

## Threat Flags
None — изменения ограничены переименованием CSS custom-property ссылок (значения сохранены дословно, кроме документированного QA-01 fix, который меняет РЕЗОЛВИНГ ранее-undefined токена на корректный, не поведение) + добавлением статического класса `tr-mono` в разметку (не пользовательский ввод, не новый XSS-вектор — подтверждено threat_model плана, T-23-05-01 mitigate закрыт точной ordered-картой, T-23-05-SC accept, 0 новых зависимостей). Декомпозиция `ReturnRowState` продолжает интерполировать `inventory_no`/`device_name` как plain text (не `{@html}`) — не вводит новую XSS-поверхность.

## Next Phase Readiness
Вся типографическая миграция (`--font-size-*`/`--font-weight-*`/`--line-height-*`/`--font-family-base`) в `ui/src` завершена — `check-tokens.mjs --rules=1` (последнее из 3 правил гейта) впервые за весь Phase 23 возвращает **PASS — 0 нарушений**. Вместе с 23-03 (цвет/тени) и 23-04 (space/radius) это закрывает ВСЮ статически проверяемую миграцию `--tr-*` слоя. `.tr-mono` применён на грепаемом, явно задокументированном подмножестве (9 сайтов + 1 архитектурная деталь в ReturnItemsTable.svelte). DS-03 (статическая часть) и финальная часть QA-01 выполнены. Plan 23-06 (финальная верификация фазы) может полагаться на: `check-tokens.mjs` (все 3 правила PASS), `pnpm svelte-check` (0 errors, 48 pre-existing warnings), `pnpm build` (успешно). Визуальная иерархия (текст на правильном уровне шкалы, UAT D-09) — вне scope этого плана, ожидает финальной проверки/милстоуна.

---
*Phase: 23-design-tokens-foundations*
*Completed: 2026-07-17*
