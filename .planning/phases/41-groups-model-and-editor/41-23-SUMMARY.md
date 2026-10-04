---
phase: 41-groups-model-and-editor
plan: 23
subsystem: ui
tags: [svelte, groups, routing, sidebar, move-modal, movement-timeline, structural-gate]
requires: [41-19, 41-20, 41-21, 41-22, 41-24]
provides:
  - "ui/src/features/groups/GroupsPage.svelte: страница раздела «Группы» (PageHeader, PlacesMasterDetail, дерево, панели типа/группы, все модалки раздела)"
  - "ui/src/features/groups/GroupPanel.svelte: панель группы — шапка и три вкладки Состав / Свойства / История"
  - "ui/src/features/groups/GroupMoveModal.svelte: подтверждение переноса группы (groups.move + notifyPlaceContentChanged в одной функции)"
  - "маршрут '/groups' (не в employeeRoutes) и запись сайдбара между «Устройства» и «Акты» (admin|manager)"
  - "ui/scripts/check-groups-section.mjs (+ --selftest) в конце цепочки pnpm lint"
affects: [41-25, 41-26]
tech-stack:
  added: []
  patterns:
    - "страница хранит выбор и вкладку (localStorage trackly:groups:*), хеш #/groups?id=|type= побеждает сохранённое; запись хеша через history.replaceState"
    - "один обработчик openMove на две точки входа (меню узла, кнопка панели)"
    - "тихая перезагрузка карточки: спиннер только пока card === null, иначе вкладка «Состав» размонтировалась бы посреди работы"
key-files:
  created:
    - ui/src/features/groups/GroupsPage.svelte
    - ui/src/features/groups/GroupPanel.svelte
    - ui/src/features/groups/GroupMoveModal.svelte
    - ui/scripts/check-groups-section.mjs
  modified:
    - ui/src/routes.ts
    - ui/src/features/layout/sidebar-config.ts
    - ui/src/lib/components/MovementTimeline.svelte
    - ui/package.json
key-decisions:
  - "Переход по клику на устройство в составе ведёт в раздел по типу устройства: devices.get -> type_id == 2 ? #/printers : #/devices (у GroupContentsTable только deviceId)"
  - "hashchange-слушатель на странице: переход-фокус работает и когда #/groups?id=N вставлен при уже открытой странице (PlacesPage читает хеш один раз)"
  - "Новая группа после «Создать группу» сразу выбирается (focusNode), как и ожидает пользователь после создания"
requirements-completed: [GRP-04, GRP-06, GRP-09]
duration: ~70min
completed: 2026-10-04
---

# Phase 41 Plan 23: Раздел «Группы» собран и подключён Summary

Раздел «Группы» смонтирован: модалка переноса, панель группы с вкладками, страница-сборщик с деревом и панелями, маршрут `/groups`, пункт сайдбара между «Устройства» и «Акты» и структурный гейт. Впервые ни один из компонентов 41-19…41-22 и 41-24 не лежит неиспользованным: страница реально выполняет их рантайм.

## Tasks

| Task | Commit | Result |
|------|--------|--------|
| 1. GroupMoveModal + GroupPanel | ea1ff2b5 | модалка переноса (числа серверные, тост `Перенесено: ${summary}`), панель группы (D-18/D-20/D-21/D-29) |
| 2. GroupsPage, маршрут, сайдбар | f09cf779 | страница, `'/groups': GroupsPage` только в `routes`, запись сайдбара, PINNED 13+4=17; + `showInitialPlacementNote` в MovementTimeline |
| 3. Гейт check-groups-section | fe214260 | 5 правил, `--selftest` на 6 фикстурах, в конце `pnpm lint` |

## Contract notes for neighbours

- `GroupPanel` props: `groupId, canEdit, refreshToken, activeTab, onTabChange, onNavigateToGroup, onOpenType, onMoveRequest, onRenameRequest, onOpenDevice?, onChanged?`. Вкладка — проп + колбэк (не `$bindable`): так она живёт в странице и переживает `{#key}`-ремаунт.
- Страница поставляет то, чего ждали компоненты-соседи: `initialSelected` и `trackly:groups:selected` (41-19), `onOpenDevice` (41-21), `#/groups?id=N` (41-24, 41-19 `focusRequest`).
- `GroupMoveModal` вызывается страницей через `{#if moveGroup}`; свежая `GroupDto` приходит из `groups.get` в `openMove`.
- Плану 41-25 ничего не передаётся: свёрнутая строка пакета в отчёте «Перемещения» не затронута.

## Verification (what actually ran)

- `pnpm --dir ui run lint`: exit 0 (вся цепочка, включая check-groups-section selftest + живой, check-place-tree-invalidation, check-group-vocabulary, check-privacy).
- `pnpm --dir ui run svelte-check`: 0 ERRORS / 68 WARNINGS (база), 309 файлов; в `features/groups` предупреждений нет.
- `pnpm --dir ui exec vite build`: проходит.
- **cargo:** `pnpm --dir ui run build` (хук `prebuild` -> `cargo test -p trackly-app --test export_bindings`) НЕ запускался вовсе — ни разу: полный `cargo test --workspace` мог держать каталог сборки, DTO/Rust не менялись. Бандл проверен через `vite build`. Пункт приёмки «`pnpm --dir ui build` код 0» поэтому подтверждён только в части сборки бандла.
- Мутационная проверка гейта на реальном файле: запись `/groups` перенесена ПОСЛЕ `/acts` (якорь встречался ровно 1 раз) -> гейт exit 1 с «должна стоять МЕЖДУ»; файл восстановлен копией, `git diff` пуст, гейт exit 0.
- Приёмочные grep: `groups.move(` + `notifyPlaceContentChanged(` в одной функции (гейт INV-7 зелёный); `Перенесено: ` — тост строится из `result.summary`; в GroupPanel `entityType: 'group'`, собственный пустой блок «Группу ещё не переносили.» вне ветки MovementTimeline, «Перенести…» только при `canEdit && isRoot`; нет `<select`/`Select.svelte`/`{@html}`; импорт `PlacesMasterDetail` из `../places/`; `openMove` встречается 4 раза; `untrack` есть; `'/groups'` не в `employeeRoutes`. Замечание: `grep -c "check-groups-section" ui/package.json` даёт 1, а не 2 (обе вставки в одной строке `lint`); `grep -o ... | wc -l` даёт 2.
- Пре-коммит гейт приватности: PASS всех трёх коммитов. В коде и фикстурах только вымышленные подписи («АРМ #1», «Здание А / 2 этаж / 214», «Иванов И.И.»).

### Рантайм-проверка: что сделано и чем это является

Страница смонтирована и прогнана **в Playwright WebKit** (webkit-2272, не WKWebView) против **Vite dev-сервера** реального `App.svelte`, с `fetch('/api/v1/*')`, подменённым выброшенным фейк-бэкендом (вне репозитория, в scratchpad; сервер остановлен, следов в git нет). Фейк знает типы/группы/места/состав/перенос/таймлайн и CRUD групп. Что это доказывает: **семантику рантайма Svelte** (этот класс не зависит от движка). Прогнано, ни одной `console.error`/`warning`/`pageerror`, в том числе без `effect_update_depth_exceeded`:
- admin: сайдбар (порядок «Устройства», «Группы», «Акты»), дерево, раскрытие, выбор типа (панель типа) и группы (панель группы), три вкладки, «История» пустая (собственный текст, текста компонента нет) и после переноса (строка «перенос группы»), «Перенести…» из кнопки панели и из меню узла (один и тот же модал), выбор места в PlacePicker, подтверждение -> Toast «Перенесено: группа и 3 устройства» и обновление «Место:» в шапке; вложенная группа (нет «Перенести…», «задано группой «…»», переход на корень), группа без места (бейдж «Без места», «Место: не задано»), перезагрузка страницы (выбор восстановлен), `#/groups?id=2` побеждает сохранённое, `hashchange` при открытой странице, «Создать группу» (тип, имя, новая группа выбирается), переименование и удаление через меню, «Вывести из состава» вложенной, «Создать тип».
- manager: раздел виден, «Создать группу» есть, «Создать тип» нет, в панели типа нет ни одной кнопки.
- employee: в сайдбаре нет ни одного пункта, `#/groups` -> «Нет доступа».

## Deviations from Plan

**1. [Rule 2 - корректность] MovementTimeline: проп `showInitialPlacementNote`**
- **Found during:** живая проверка вкладки «История».
- **Issue:** при непустой истории под списком компонент рисует сноску «Первичное размещение при поступлении в историю не попадает…» — про устройства, для группы неверно (та же причина, по которой план запрещает его пустой блок).
- **Fix:** необязательный проп, по умолчанию `true` (прочие потребители не меняются); GroupPanel передаёт `false`. Компонент не форкался. Файл вне `files_modified` плана.
- **Commit:** f09cf779.

**2. [Мелочи текста callout]** склонение глагола («переедет 1 устройство» / «переедут 5 устройств») через общий `pluralizeRu` вместо жёсткого «переедут»; при вложенных группах без устройств — «Вложенная группа переедет вместе с ней.» вместо «включая 0 из …». Числа по-прежнему серверные поля, клиент только суммирует показанное.

**3. [Дополнено сверх плана]** клик по устройству в составе (`onOpenDevice`): `devices.get` и выбор раздела по `type_id` (принтер -> `#/printers`, иначе `#/devices`); `hashchange`-слушатель; выбор только что созданной группы. Все три в `GroupsPage.svelte`.

**4. [Организационное]** вкладка панели группы — проп `activeTab` + `onTabChange`, не `$bindable` (допускалось планом).

## Known Stubs
None.

## UNVERIFIED

Не проверено в запущенном приложении — ни в Tauri (WKWebView), ни в реальном LAN-браузере с настоящим сервером:
- **Вёрстка и скролл-регионы** (35/65, единственный скролл-регион в панели, `height: 100%` дочерних компонентов внутри flex-контейнера `.tab-body`, закреплённый ряд кнопок формы свойств в оболочке `.content`): скриншоты Playwright WebKit выглядят правильно, но это не WKWebView; правило проекта — движок-зависимое считается непроверенным.
- **Реальный сервер:** настоящие ответы `groups_move` (текст `summary`, `changed_place_ids`, ошибка `Validation` под полем места, `OptimisticLockMismatch`), реальные права `manager`/`employee` на сервере (403), `groups.setParent` при выводе из состава. Фейк-бэкенд их лишь имитирует.
- **Инвалидация счётчиков дерева «Места»:** дерево мест в прогоне не монтировалось; гейт INV-7 доказывает проводку вызова, не корректность счётчиков.
- **Транспорт Tauri `invoke`** (проверялся только HTTP-путь `fetch`).
- **Не прогнано в рантайме:** строка добавления в составе и модалка «Добавить несколько…» (только отрисованы), сохранение формы свойств и серверные ошибки по полю, правка свойств типа админом (панель типа админа отрисована, не редактировалась), клавиатурная навигация дерева, удаление типа, hover/focus-видимость «⋯».
- **Пункты `human-check` обеих задач 2 пройдены не мной:** «раздел на двух транспортах» и «роли manager/employee» — для HUMAN-UAT верификатора. Роли в прогоне задавались ответом фейк-`auth_status`, не реальной сессией/AD.
- `pnpm --dir ui run build` (с cargo-хуком) не запускался.

## Threat Flags
None. T-41-23-01 (маршрут не в `employeeRoutes` + гейт `check-groups-section`; серверный 403 — граница), T-41-23-02 (контролы типов не рендерятся для manager — подтверждено прогоном), T-41-23-03 (в форме свойств нет места, перенос — отдельное подтверждаемое действие), T-41-23-04 (notify в обрамляющей функции `confirmMove`/`removeFromParent`, гейт INV-7 зелёный), T-41-23-05 (нет `{@html}`) выполнены.

## Self-Check: PASSED
Файлы GroupsPage.svelte, GroupPanel.svelte, GroupMoveModal.svelte, check-groups-section.mjs существуют; коммиты ea1ff2b5, f09cf779, fe214260 найдены.
