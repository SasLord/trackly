---
phase: 41-groups-model-and-editor
plan: 24
subsystem: ui
tags: [svelte, groups, device-form, movement-timeline, vocabulary-gate]
requires: [41-03, 41-05, 41-14, 41-18]
provides:
  - "DeviceFormBody: PlacePicker заблокирован у члена группы С МЕСТОМ, подпись «Место задаётся группой «…»» со ссылкой (D-19); у группы без места не блокируется (D-21)"
  - "MovementTimeline: основания «в составе группы «имя»» (ссылка через onNavigateToGroup) и «перенос группы» (D-28)"
  - "allowlist словарного гейта: узкий маркер «перенос группы» только для MovementTimeline.svelte"
affects: [41-23, 41-25]
tech-stack:
  added: []
  patterns:
    - "членство читается одним groups.forDevices([id]) в onMount формы правки; DeviceDto не расширяется; ошибка запроса = «не член»"
    - "новый необязательный проп onNavigateToGroup повторяет onNavigateToAct: push затем закрытие окна (модалки) либо только push (страницы)"
key-files:
  created: []
  modified:
    - ui/src/features/devices/DeviceFormBody.svelte
    - ui/src/features/devices/DeviceFormModal.svelte
    - ui/src/lib/components/MovementTimeline.svelte
    - ui/src/features/places/PlaceEntityViewModal.svelte
    - ui/src/features/printers/PrinterDetail.svelte
    - ui/src/features/cartridges/CartridgeDetail.svelte
    - ui/scripts/check-group-vocabulary.mjs
key-decisions:
  - "Блокировка места в UI только подсказка: форма по-прежнему шлёт place_id, серверный guard (41-14) остаётся источником истины"
  - "У заблокированного члена группы скрыт чекбокс «Перевести в статус На складе», и эффект неявной смены статуса тоже отключён (иначе статус менялся бы молча)"
requirements-completed: [GRP-07, GRP-06]
completed: 2026-10-04
---

# Phase 41 Plan 24: форма устройства и журнал перемещений о группах Summary

Форма правки устройства блокирует выбор места у члена группы с местом и ведёт в раздел «Группы», а таймлайн перемещений показывает «в составе группы «АРМ #3»» со ссылкой и «перенос группы» для строки самой группы.

## Tasks

| Task | Name | Commit |
|------|------|--------|
| 1 | Форма устройства: блокировка PlacePicker и подпись (D-19) | 89f686f5 |
| 2 | MovementTimeline: основания, потребители, allowlist гейта | 0536c35c |

## Что сделано

- **DeviceFormBody:** проп `onNavigateToGroup?`; один `groups.forDevices([target.id])` в `onMount` (только правка, не readonly); `lockedByGroup = membership !== null && group_has_place`; `PlacePicker disabled={readonly || lockedByGroup}`; подпись с `Button variant="link"` (без обработчика — обычный текст). Логика отправки `place_id` не менялась.
- **DeviceFormModal:** `handleNavigateToGroup` — `await push('#/groups?id=N')`, затем `onClose()` (порядок как в PlaceEntityViewModal).
- **MovementTimeline:** проп `onNavigateToGroup?`; ветка `source === 'group'`; ссылка строится из `group_id` (number), имя только текстом. Если `group_id = null`, нет обработчика или пустой снимок имени, остаётся обычный текст. Анатомия строки и путь-в-tooltip не тронуты, JS-зеркала формулы сокращения пути нет.
- **Потребители:** PlaceEntityViewModal (push + onClose), PrinterDetail и CartridgeDetail (страницы — только push, как их onNavigateToAct).
- **Гейт GRD-06:** `markers: ['в составе группы', 'перенос группы']` для MovementTimeline; в selftest 3 фикстуры (позитив; негатив в другом файле; негатив на прочее слово «группа» в том же файле). Selftest 12/12.

## Verification (что реально запускалось)

- `pnpm --dir ui run lint`: exit 0 (в цепочке check-group-vocabulary selftest + живой, check-privacy).
- `pnpm --dir ui run svelte-check`: 0 ERRORS / 68 WARNINGS (база).
- `pnpm --dir ui exec vite build`: проходит.
- Мутация гейта (якорь `markers: ['в составе группы', 'перенос группы']` встречается ровно 1 раз, файл восстановлен копией и сверен `cmp`): маркер убран -> красны и selftest (1 FAIL), и живой прогон (`MovementTimeline.svelte:78`).
- **cargo:** `pnpm --dir ui run build` (хук prebuild с cargo) НЕ запускался; бандл проверен через `vite build`.

## Deviations from Plan

**1. [Rule 2 - корректность] Эффект «статус На складе» отключён для заблокированного члена.** План требовал лишь скрыть чекбокс. Эффект при этом продолжал бы молча ставить статус «На складе» для устройства в складском месте. Добавлено `!lockedByGroup` в условие эффекта. Коммит 89f686f5.

Прочих отклонений нет.

## UNVERIFIED (ничего из этого не запускалось)

`svelte-check`, `eslint` и сборка слепы к рантайму рун. Не проверено ни в Tauri (WKWebView), ни в LAN-браузере; Playwright не использовался.
- Реальная блокировка PlacePicker у члена группы с местом, подпись и клик по имени группы; нет ли `effect_update_depth_exceeded` в DeviceFormBody (запись `membership` идёт после await в `.then`, не из эффекта).
- Сохранение прочих полей члена группы без серверного отказа (форма шлёт текущий `place_id`).
- Устройство из группы без места и вне групп: место редактируется как раньше.
- Вид строки таймлайна «в составе группы «…»» и «перенос группы», переход по ссылке и фокус на группе (страница `#/groups?id=` принадлежит плану 41-23: до его появления переход не покажет группу).
- Прежние строки таймлайна («вручную», «актом №…») выглядят как раньше; витрина (showcase) не менялась.
- Пункты human-check обеих задач не пройдены.

## Known Stubs
None.

## Threat Flags
None. T-41-24-01: серверный guard 41-14 не тронут. T-41-24-02: нет `{@html}`, ссылка из number. T-41-24-03: один запрос на открытие, ошибка не блокирует форму.

## Self-Check: PASSED
Файлы изменены, коммиты 89f686f5 и 0536c35c существуют.
