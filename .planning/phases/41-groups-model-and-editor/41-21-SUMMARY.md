---
phase: 41-groups-model-and-editor
plan: 21
subsystem: ui
tags: [svelte, dropdown, combobox, group-composition, invalidation-gate, groups]
requires: [41-03, 41-18]
provides:
  - "ui/src/lib/components/Dropdown.svelte: необязательный проп getGroupSection (заголовки секций, role=presentation)"
  - "ui/src/features/groups/GroupContentsTable.svelte: состав группы, вложенные группы, строка-поиск добавления устройств и групп"
  - "ui/src/features/groups/GroupAddDevicesModal.svelte: модальный мультивыбор устройств"
  - "ui/scripts/check-place-tree-invalidation.mjs: маркеры groups.move(/addDevices(/setParent( в DIRECT_CALL_MARKERS"
affects: [41-23]
tech-stack:
  added: []
  patterns: ["синтетическая member-строка для авто-раскрытия единственного результата-группы (AUTO-05)", "клиентский пост-фильтр + автоподгрузка страниц, когда сервер не умеет фильтр"]
key-files:
  created:
    - ui/src/features/groups/GroupContentsTable.svelte
    - ui/src/features/groups/GroupAddDevicesModal.svelte
  modified:
    - ui/src/lib/components/Dropdown.svelte
    - ui/src/features/showcase/sections/DropdownSection.svelte
    - ui/scripts/check-place-tree-invalidation.mjs
key-decisions:
  - "Оба пути добавления устройств (строка-поиск и модалка) вызывают один groups.addDevices; оба вызывают notifyPlaceContentChanged(changed_place_ids)"
  - "Реестр INV-7 расширен от серверных мутаций (groups_move, groups_add_devices, groups_set_parent), а не от экранов; гейт не ослаблялся"
  - "Единственный результат-группа в Dropdown раскрывается в одну синтетическую строку-член: выбор вложит именно её (иначе AUTO-05 показал бы её устройства, и выбор добавил бы устройство)"
requirements-completed: [GRP-04, GRP-05, GRP-06]
duration: ~75min
completed: 2026-10-04
---

# Phase 41 Plan 21: Group contents, add row and INV-7 markers Summary

Состав группы: таблица как «Содержимое места» с лениво раскрываемыми вложенными группами, один Dropdown-combobox с секциями «Устройства»/«Группы» для добавления и вложения, модалка мультивыбора и три новых маркера реестра INV-7.

## Tasks

| Task | Commit | Result |
|------|--------|--------|
| 1. Dropdown getGroupSection + витрина | a5f0b1fa | необязательный проп, заголовок `<li role="presentation">` вне навигации/aria-activedescendant/счёта; без пропа разметка и клавиатурный код не тронуты; демонстрация «Комбобокс с заголовками секций» («Раздел А»/«Раздел Б») |
| 2. GroupContentsTable | 075ae7f6 | 6 столбцов, раскрытие вложенных (рекурсивные snippet'ы), меню строк, пустое состояние Table, строка добавления в footer; без фильтра статуса |
| 3. GroupAddDevicesModal + INV-7 | 73603877 | мультивыбор с фильтрами, занятость одним groups.forDevices, «Выбрано: N», три маркера в реестре; кнопка «Добавить несколько…» подключена в таблицу |

## Verification (what actually ran)

- `pnpm --dir ui run lint`: exit 0 (вся цепочка, включая check-group-vocabulary, check-privacy, check-place-tree-invalidation).
- `pnpm --dir ui run svelte-check`: 0 ERRORS / 68 WARNINGS (база); в features/groups предупреждений нет.
- `pnpm --dir ui run build`: проходит (см. «Отклонения: cargo» ниже).
- `node ui/scripts/check-place-tree-invalidation.mjs`: PASS, 0 нарушений.
- **Мутационная проверка гейта** (на копии `ui/src` в scratchpad через `--src=`, реальные файлы не трогались; в каждой функции перед правкой проверено, что `notifyPlaceContentChanged(` встречается ровно один раз): снятие вызова в `submit()` модалки, `addDevice()`, `attachGroup()` и `detachGroup()` таблицы — каждый раз гейт падает с `INV-7` и названием файла и функции; на восстановленной копии exit 0.
- Acceptance greps: нет `status_id: 1`, есть `status_id: null`; нет `Placeholder`; «Состав пуст» на месте; нет `<select`/`Select.svelte`/`draggable`/`ondragstart`/`{@html}`; `getGroupSection` передан в Dropdown; «Выбрано: » и «Уже в группе» в модалке; `git diff` Dropdown не затрагивает обработчики клавиатуры, ActFormItemsTable.svelte не менялся.
- Пре-коммит гейт приватности: PASS всех трёх коммитов. Данные — только подписи, placeholder'ы и нейтральные «Раздел А/Б».

## Deviations from Plan

**1. [Нарушение ограничения: cargo] `pnpm run build` запускает cargo через хук `prebuild`**
- `ui/package.json`: `"prebuild": "cargo test -p trackly-app --test export_bindings"`. Я дважды запускал `pnpm --dir ui run build` (после задачи 2 и после задачи 3) и только после этого заметил в выводе строку `test export_bindings_to_ui_writes_health_dto_and_app_error ... ok`. То есть два раза неявно выполнен `cargo test -p trackly-app --test export_bindings` — вопреки указанию не запускать cargo, пока идёт регрессионный прогон. Оба раза он завершился успешно. Следствия для оркестратора: (а) мог кратко ждать блокировку каталога сборки у полного `cargo test --workspace` или замедлить его; (б) мог перезаписать gitignored `ui/src/bindings.ts` (содержимое должно быть тем же, генератор детерминирован, но я не сравнивал); (в) `git status` после этого чист. Тот же хук, судя по SUMMARY 41-18/41-20 («build проходит», «cargo не запускался»), срабатывал и у предыдущих исполнителей незаметно. Рекомендация: для дальнейших фронтенд-планов гонять `pnpm --dir ui exec vite build` (без хука) либо считать хук cargo-шагом.

**2. [Данные сервера: поиск в модалке] Нет серверного фильтра по месту и текст+фильтры одним запросом**
- `devices.list` фильтрует только по типу и статусу (текста нет, `place_id` в фильтре игнорируется), `devices.search` ищет по тексту без фильтров. Поэтому с текстом — `devices.search` + «Тип»/«Статус» на клиенте; без текста — `devices.list` с серверными «Тип»/«Статус»; «Место» всегда клиентское (поддерево по `full_path` выбранного места, разделитель « / » из V037, путь берётся `places_get`). Когда клиентский фильтр активен, страницы подгружаются подряд до 30 подходящих строк за вызов (потолок 1000 просмотренных), «Показать ещё» догружает дальше. Иначе узкий фильтр давал бы пустые страницы.

**3. [Отклонение от подписи UI-SPEC] «Место» в модалке — PlacePicker**
- Как и предписывал план: штатный выбор места проекта вместо Dropdown. «Тип» и «Статус» — Dropdown flat (справочники: Устройство/Принтер и 4 статуса, как в DeviceFilters/DeviceFormModal).

**4. [Rule 1 - Bug] AUTO-05 Dropdown и единственный результат-группа**
- Dropdown вызывает `onExpandGroup` для ЛЮБОГО единственного результата, не только раскрываемого. Для найденной группы возвращается одна синтетическая строка-член `{kind:'group'}`, выбор которой вызывает `setParent` — иначе показались бы устройства найденной группы и выбор добавил бы устройство.

**5. Мелочи**
- Выбор свёртки/подгруппы одинаковых несерийных устройств добавляет ОДНО устройство — первое, не помеченное занятым (план: «выбор устройства -> addDevices({device_ids:[id]})»). Количество не выбирается.
- Пометка «уже в группе «…»» в панели Dropdown выводится в мета-слот элемента (общий вид Dropdown), а не отдельным caption-стилем: добавлять стиль в общий компонент вне плана не стал.
- Добавлен необязательный проп `onOpenDevice` у GroupContentsTable: первая ячейка строки устройства (`role="button"`, `tabindex="0"`, точка входа фокуса, inset-кольцо TableRow) по Enter/клику зовёт его, если он задан. Без него клик ничего не делает (до плана 41-23 строка устройства никуда не ведёт).
- У строк вложенных групп глубже первого уровня сдвиг имени задан глобальным CSS-селектором (`tr.gc-nested`): TableRow в group-режиме не принимает `indent`.
- Меню строки вложенной группы обёрнуто в `div role="presentation"` с `stopPropagation`, чтобы клик по «⋯» не сворачивал строку (у `<td>` роль presentation запрещена a11y-правилом).

## UNVERIFIED (ничего из этого не запускалось)

`svelte-check`, `eslint` и сборка слепы к рантайму рун; Playwright/Chromium не использовался. НЕ проверено ни в Tauri (WKWebView), ни в LAN-браузере:
- **Dropdown с заголовками секций:** что заголовки видны, стрелки их перескакивают, `aria-activedescendant` не сбивается; что форма акта и пикер принтера (проп не задан) выглядят и работают как прежде; вид заголовка в витрине (`--tr-text-body-strong`).
- **Строка добавления:** поиск возвращает и устройства, и группы с секциями; drill-in свёртки; авто-раскрытие единственной группы в синтетическую строку; выбор устройства/группы добавляет без перезагрузки страницы, поле очищается и сохраняет фокус; Toast с именем группы при выборе занятого устройства; гонка ответов при быстром наборе (счётчики `searchSeq`/`loadSeq` — только код).
- **Раскрытие вложенных групп** шевроном, ленивая загрузка, повторное раскрытие после перезагрузки, рекурсия (группа в группе), клик по «⋯» не сворачивает строку, меню в portal внутри Table.
- **Отсутствие `effect_update_depth_exceeded`** в GroupContentsTable (эффект загрузки в `untrack`) и в модалке (нет `$effect`, загрузка из `onMount`).
- **Модалка:** фильтры (клиентский пост-фильтр и автоподгрузка), «Показать ещё», отметка и снятие, серое неотмечаемое занятое устройство с подсказкой, «Выбрано: N» / «Добавить (N)», инлайн-ошибка сервера (например, превышение 500), закрытие после успеха, видимость PlacePicker-панели внутри Modal.
- **Инвалидация счётчиков:** что дерево «Места» реально обновляется после добавления/вложения/вывода группы (гейт доказывает только проводку вызова, не корректность `changed_place_ids`).
- Пункты `human-check` всех трёх задач; компоненты до плана 41-23 не смонтированы ни на одной странице (кроме витрины Dropdown).

## Known Stubs
None. (Строка устройства без `onOpenDevice` не ведёт никуда — намеренно, страница в плане 41-23 может передать обработчик.)

## Threat Flags
None. T-41-21-01 (серверный отказ показывается Toast/инлайн, UI не блокирует), T-41-21-02 (контролы только при `canEdit`), T-41-21-03 (notify + мутационно проверенный INV-7), T-41-21-04 (нет `{@html}`), T-41-21-06 (проп необязателен) выполнены в коде; рантайм-часть — в UNVERIFIED.

## Self-Check: PASSED
Файлы GroupContentsTable.svelte, GroupAddDevicesModal.svelte и коммиты a5f0b1fa, 075ae7f6, 73603877 найдены.
