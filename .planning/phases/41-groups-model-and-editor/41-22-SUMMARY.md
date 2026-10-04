---
phase: 41-groups-model-and-editor
plan: 22
subsystem: ui
tags: [svelte, groups, properties-form, chips, printers, explicit-save]
requires: [41-12, 41-18]
provides:
  - "ui/src/features/groups/GroupUsersField.svelte: чипсы пользователей с основным и плоский Dropdown добавления (источник только groups.userOptions)"
  - "ui/src/features/groups/GroupPrintersList.svelte: единый список подключённых принтеров USB + ссылки"
  - "ui/src/features/groups/GroupPropertiesForm.svelte: форма свойств группы с явным сохранением и серверными ошибками по полю"
affects: [41-23]
tech-stack:
  added: []
  patterns:
    - "эффект инициализации читает только card, пишет состояние внутри untrack; несохранённые правки не затираются, пока версия той же группы не сменилась"
    - "aria-required на контроле Input ставится действием контейнера (общий Input не пробрасывает атрибут, он не менялся)"
key-files:
  created:
    - ui/src/features/groups/GroupUsersField.svelte
    - ui/src/features/groups/GroupPrintersList.svelte
    - ui/src/features/groups/GroupPropertiesForm.svelte
  modified: []
key-decisions:
  - "Нормализация ip/mac/number/text только на сервере: форма отправляет строку как введена и перезаполняется из возвращённой карточки"
  - "Поиск принтера для ссылки — devices.listGrouped с type_id = PRINTER_TYPE_ID (серверный фильтр типа плюс текст), наборы одинаковых устройств раскрываются через listByIds"
requirements-completed: [GRP-03, GRP-08]
duration: ~45min
completed: 2026-10-04
---

# Phase 41 Plan 22: Вкладка «Свойства» группы Summary

Три компонента вкладки «Свойства»: чипсы пользователей с одним основным, единый дедуплицированный список подключённых принтеров (USB без меню, ссылки с «Убрать ссылку») и типизированная форма с явным «Сохранить»/«Отмена», серверными ошибками под полем и показом приведённых сервером значений.

## Tasks

| Task | Commit | Result |
|------|--------|--------|
| 1. GroupUsersField | 4ccfb6f4 | чипс = звёздочка (`aria-pressed`) + имя + «×» (`aria-label` «Убрать {ФИО}»); повторное нажатие на основного снимает отметку; Dropdown select flat, источник только `groups.userOptions`, уже выбранные скрыты, защита от гонки ответов |
| 2. GroupPrintersList | 24162b31 | Table на 4 колонки; USB-строки без ⋯, ссылки и «USB+ссылка» с ⋯ «Убрать ссылку»; строка добавления только при `canEdit && canLink` |
| 3. GroupPropertiesForm | 4e1d1853 | поля по `data_type`, закреплённый ряд кнопок, разбор `values.<id>`, конфликт версий — Toast, пустое состояние со ссылкой «Настроить свойства типа» |

## Verification (что реально запускалось)

- `pnpm --dir ui run lint`: exit 0 (вся цепочка, включая check-group-vocabulary, check-privacy, check-place-tree-invalidation).
- `pnpm --dir ui run svelte-check`: 0 ERRORS / 68 WARNINGS (база), в features/groups предупреждений нет.
- `pnpm --dir ui exec vite build`: проходит. Оговорка: компоненты пока нигде не подключены (их монтирует план 41-23), поэтому в бандл они не попадают; их компиляцию проверил только `svelte-check`.
- Acceptance greps: нет `normalize`/`isIP`/`split(':')`, автосейва, `<select`/`Select.svelte`, `{@html}`; `untrack`, `values.`, «Сохранить»/«Отмена», `aria-pressed`, «Убрать», `userOptions`, «Принтеров нет», `PRINTER_TYPE_ID = 2` на месте.
- Пре-коммит гейт приватности PASS на трёх коммитах; данных организации и людей нет (в коде только подписи и плейсхолдеры IP/MAC).
- **cargo:** `pnpm --dir ui run build` (хук `prebuild` -> `cargo test -p trackly-app --test export_bindings`) запущен ОДИН раз, в самом конце, после всех коммитов; прошёл. Остальные проверки шли через `vite build` без хука.

## Deviations from Plan

**1. [Организационное] Props GroupPrintersList отличаются от черновика плана**
- Вместо `usbAndLinked` принимается `printers` (весь `card.printers`) и дополнительно `id`/`errorText`; USB-строки берутся из `origin === 'usb'`, сведения о ссылках — из `card.printers` либо `linkedInfo`. Контракт для страницы (GroupPropertiesForm props) соблюдён как в плане.

**2. [Организационное] Поиск принтеров**
- `devices.list` не фильтрует по тексту, `devices.search` по типу. Взят `devices.listGrouped` с `type_id`, `name_prefix` и `group_by_condition: true` (как строка добавления состава в 41-21), лимит 20.

**3. [Организационное] aria-required через действие**
- `Input` не пробрасывает `aria-required`; общий компонент не менялся (правило «только свои файлы»), атрибут выставляет действие на контейнере полей по id `gpf-<id>`.

**4. Мелочи**
- Ссылка на не-принтер остаётся в значении свойства и уходит при сохранении, но строкой не показывается (так же ведёт себя сервер).
- Для device_refs-свойства, не являющегося `link_property_id`, USB-строки не показываются (только ссылки).

## UNVERIFIED (ничего из этого не запускалось)

`svelte-check`, `eslint` и сборка слепы к рантайму рун. Не проверено ни в Tauri (WKWebView), ни в LAN-браузере; Playwright/Chromium не использовался. Компоненты не смонтированы ни на одной странице до 41-23.
- **Отсутствие `effect_update_depth_exceeded`** в GroupPropertiesForm (эффект инициализации в `untrack`) и корректность условия «не затирать несохранённые правки».
- **Чипсы:** звёздочка переносит отметку на одного, «×» убирает, Tab/Enter/Space; Dropdown ищет по ФИО и логину без учёта регистра (в том числе кириллицей) под ролью manager; визуальный вид чипса.
- **Принтеры:** живой сценарий USB (нужен ручной `UPDATE printers SET usb_host_device_id`), дедупликация USB+ссылка в одну строку с ⋯, Badge «Ссылка»; поведение Dropdown combobox flat (подсказки, очистка поля после выбора).
- **Форма:** показ нормализованных значений («2001:db8::1», «aa:bb:cc:dd:ee:ff») после сохранения, ошибка под полем для невалидного IP и пустого обязательного свойства, неактивность «Сохранить» без изменений, «Отмена», Toast при конфликте версий, расстановка `aria-required` действием, закреплённый ряд кнопок и прокрутка полей в оболочке `.content`.
- Пункты `human-check` всех трёх задач (SC7 «из коробки» на чистой dev-БД).

## Known Stubs
None.

## Threat Flags
None. T-41-22-01 (формулы ip/mac на клиенте нет), -02 (источник только `groups.userOptions`, показываются ФИО и логин), -04 (нет `{@html}`), -05 (CAS по `version`, сообщение с просьбой обновить), -06 (запись состояния в `untrack`; рантайм — в UNVERIFIED) выполнены в коде.

## Self-Check: PASSED
Файлы GroupUsersField.svelte, GroupPrintersList.svelte, GroupPropertiesForm.svelte и коммиты 4ccfb6f4, 24162b31, 4e1d1853 найдены.
