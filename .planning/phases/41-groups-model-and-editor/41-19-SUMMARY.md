---
phase: 41-groups-model-and-editor
plan: 19
subsystem: ui
tags: [svelte, tree, aria, modals, dropdown, groups]
requires: [41-18]
provides:
  - "ui/src/features/groups/GroupTree.svelte: дерево типов и групп (поиск, клавиатура, меню узлов, перезагрузка по refreshToken и placeContentEventsStore)"
  - "ui/src/features/groups/GroupTreeNode.svelte: строка узла 32px, экспортирует TreeItem, TreeNodeActions, rowDomId"
  - "ui/src/features/groups/GroupFormModal.svelte, GroupTypeFormModal.svelte, GroupDeleteModal.svelte"
affects: [41-23]
tech-stack:
  added: []
  patterns: ["сиблинг PlaceTree: копия анатомии, не обобщение", "загрузка в untrack внутри $effect с токенами инвалидации", "tabbableKey как $derived вместо записи activeId в эффекте"]
key-files:
  created:
    - ui/src/features/groups/GroupTree.svelte
    - ui/src/features/groups/GroupTreeNode.svelte
    - ui/src/features/groups/GroupFormModal.svelte
    - ui/src/features/groups/GroupTypeFormModal.svelte
    - ui/src/features/groups/GroupDeleteModal.svelte
  modified: []
key-decisions:
  - "Выбранный узел хранит страница (initialSelected), дерево хранит только раскрытые ключи (trackly:groups:expanded) — одно место записи на одно состояние"
  - "Раскрытие в режиме поиска — отдельный набор searchExpanded, сохранённое раскрытие дерева поиском не затирается"
  - "GroupFormModal сам грузит типы и группы (подсказка N), страница типы в него не передаёт"
requirements-completed: [GRP-04, GRP-01]
duration: ~50min
completed: 2026-10-04
---

# Phase 41 Plan 19: Group tree and modals Summary

Левая панель раздела «Группы» (дерево, где корни — типы, внутри — группы; вложенная группа только под родителем) и три модалки: создание/переименование типа, создание/переименование группы, подтверждение удаления.

## Tasks

| Task | Commit | Result |
|------|--------|--------|
| 1. GroupTreeNode + GroupTree | 08146f17 | дерево, поиск с подсветкой и aria-live, клавиатура по плоскому visibleNodes, меню узлов по UI-SPEC 8.3 |
| 2. Три модалки | e7cc9fa7 | тип (поведение только при создании), группа (тип в Dropdown, пустое имя уходит null), удаление (destructive) |

## Контракт для плана 41-23

- `GroupTree` Props: `initialSelected`, `refreshToken`, `focusRequest`, `canEditTypes`, `onSelect`, `onCreateType`, `onCreateGroup(typeId)`, `onRename(node)`, `onMove(groupId)`, `onRemoveFromParent(groupId)`, `onDelete(node)`, `onLoaded(types, groups)`. Тип `GroupTreeNodeRef = {kind, id}` экспортируется из `<script module>` GroupTree.svelte.
- `onSelect` на первой загрузке НЕ вызывается, если `initialSelected` найден (страница и так его знает); вызывается с `null`, если сохранённого узла больше нет, и при исчезновении выбранного узла после перезагрузки. `focusRequest` вызывает `onSelect` с найденным узлом.
- Права на меню групп дерево берёт из `authStore` (admin, manager); меню типов — из `canEditTypes`.
- `GroupFormModal`: `mode`, `group?`, `presetTypeId?`, `onClose`, `onSaved(dto)`. `GroupTypeFormModal`: `mode`, `type?`, `onClose`, `onSaved(dto)`. `GroupDeleteModal`: `kind`, `id`, `name`, `directDeviceCount?`, `onClose`, `onDeleted()`. Все монтируются через `{#if}`. Тосты успеха показывают сами модалки.
- Удаление группы не меняет места устройств, поэтому `notifyPlaceContentChanged` модалка не зовёт; счётчики дерева обновит `refreshToken`. Вывод из состава (`groups.setParent`) вызывает страница и обязана сама вызвать `notifyPlaceContentChanged(result.changed_place_ids)`.

## Verification

- `pnpm --dir ui run lint`: exit 0 (вся цепочка, включая check-group-vocabulary и check-reorder).
- `pnpm --dir ui run svelte-check`: 0 ERRORS / 68 WARNINGS, в `features/groups` предупреждений нет. Ошибка из deferred-items 41-18 (MovementTimelineSection) уже была устранена до этого плана.
- `pnpm --dir ui run build`: проходит.
- Приёмочные grep: `role="tree"` с `aria-label="Типы и группы"`, один `role="treeitem"`, `untrack` и `placeContentEventsStore` в GroupTree, нет `<select`/`Select.svelte`/`draggable`/`{@html}`/`variant="danger"`, нет строки «Создать вложенную группу», `освободятся из состава` присутствует, `variant="select"` в GroupTypeFormModal, `variant="destructive"` в GroupDeleteModal.
- Пре-коммит гейт приватности: PASS обоих коммитов.

## UNVERIFIED (ничего из этого не запускалось)

Компиляционные гейты (`svelte-check`, `eslint`, `pnpm build`) слепы к ошибкам рантайма рун. Не проверено в запущенном приложении ни в Tauri (WKWebView), ни в LAN-браузере:
- отсутствие `effect_update_depth_exceeded` при перезагрузке дерева по `refreshToken` / `placeContentEventsStore.seq`;
- клавиатурная навигация (стрелки, Home/End, Enter/Space), roving tabindex, фокус после `focusRequest`;
- поиск: раскрытие веток, подсветка фрагмента, озвучивание aria-live;
- открытие ActionMenu с клавиатуры из строки и закрытие меню, видимость кнопки «⋯» по hover/focus/selected;
- вид Dropdown в модалках, предзаполненный и заблокированный тип, подсказка «Если оставить пустым — «… #N»»;
- реальный ответ сервера на переименование типа без ключей `code`/`behavior` (см. отклонение 1);
- что в живом дереве видны три встроенных типа и счётчики/бейдж «Без места».
Эти проверки (`human-check` обеих задач) нужно собрать верификатору в HUMAN-UAT; сами компоненты до плана 23 не смонтированы ни на одной странице, то есть проверять их можно только после сборки страницы.

## Deviations from Plan

**1. [Rule 1 - Bug] Запрос переименования типа без ключей code/behavior**
- Сгенерированный `GroupTypeUpdateDto` требует ключи `code`/`behavior` (`string | null`), а приёмка требует, чтобы вызов их не содержал. Тело запроса — `{ name } as GroupTypeUpdateDto`: серверный serde читает отсутствующий `Option` как `None` («не менять»). Этот путь не запускался (cargo был занят, рантайм не проверен) — см. UNVERIFIED.
- Файл: GroupTypeFormModal.svelte. Commit: e7cc9fa7.

**2. [Rule 1 - Bug] Узел раскрытия/действий: события из ActionMenu**
- ActionMenu не останавливает всплытие Enter/Space/стрелок. Без защиты строка и контейнер дерева гасили бы `preventDefault` на триггере меню и двигали фокус по дереву. Добавлены: проверка `e.target !== e.currentTarget` в обработчике строки и пропуск событий изнутри `.row-actions` в обработчике дерева. PlaceTree этой защиты не имеет (не трогался).
- Файлы: GroupTreeNode.svelte, GroupTree.svelte. Commit: 08146f17.

**3. [Rule 2 - Missing] Тексты удаления группы для 0 и 1 устройства**
- Для 0 устройств вместо «0 устройств освободятся» показывается «Устройств в составе нет.»; для 1, 21, 31 — «освободится». Для остальных чисел текст дословно по UI-SPEC 17.5.
- Файл: GroupDeleteModal.svelte. Commit: e7cc9fa7.

**4. Выбранный узел в localStorage — не в дереве**
- План и UI-SPEC 7.4 говорят «раскрытые и выбранный». Дерево хранит только раскрытые ключи (`trackly:groups:expanded`); выбранный узел приходит как `initialSelected` и хранится страницей (`trackly:groups:selected`, план 23 это прямо поручает странице). Иначе было бы два места записи одного состояния.

**5. `svelte-ignore state_referenced_locally` на трёх строках начального значения полей форм**
- Начальное значение читается один раз при монтировании по контракту `{#if}`; без пометки +7 предупреждений к базе 68. Пометка стоит с пояснением. Файлы: GroupFormModal.svelte, GroupTypeFormModal.svelte.

**6. Пропущено по указанию оркестратора:** предусловие `cargo test -p trackly-app --test export_bindings` — отложено оркестратору, cargo был занят регрессионным прогоном фазы. Типы взяты из `ui/src/bindings.ts` на диске.

## Known Limitations

- В режиме поиска набор раскрытых узлов отдельный и сбрасывается заново при каждом вводе; ручное схлопывание держится только до следующего символа.
- Значение `title` типа записано с заглавной буквы («тип группы · Контейнер») по тексту плана; в UI-SPEC 8.2 пример со строчной.

## Known Stubs
None.

## Threat Flags
None. T-41-19-02 (форма не отправляет code/behavior) и T-41-19-03 (нет `{@html}`) выполнены; T-41-19-04 закрыт структурно (загрузка в untrack), рантайм-проверка консоли отложена.

## Self-Check: PASSED
Все 5 файлов и коммиты 08146f17, e7cc9fa7 найдены.
