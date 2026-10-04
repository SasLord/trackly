---
phase: 41-groups-model-and-editor
plan: 11
subsystem: api
tags: [rust, sqlite, groups, membership, nesting, place-propagation, transactions]
requires:
  - phase: 41-06
    provides: "SqliteGroupRepository: add_device_in_tx, remove_device_in_tx, group_of_device_in_tx, set_parent_in_tx (цикл, teardown), subtree_*_in_tx, set_subtree_place_in_tx"
  - phase: 41-08
    provides: "GroupService, GroupDto"
  - phase: 41-10
    provides: "services::group_place: apply_group_place_to_device_in_tx, move_group_in_tx, GroupMoveDeps"
provides:
  - "GroupService::add_devices / remove_devices / set_parent"
  - "services::group_membership::release_device_in_tx (pub(crate), для планов 14 и 16)"
  - "services::group_place::propagate_group_place_in_tx (ядро протаскивания, общее для переноса и вложения)"
  - "dto::groups: GroupAddDevicesDto, GroupAddDevicesResultDto, GroupRemoveDevicesDto, GroupSetParentDto"
affects: [41-12 карточка, 41-13 транспорты, 41-14 акты, 41-15 массовый перенос, 41-16 удаление устройства]
tech-stack:
  added: []
  patterns:
    - "одна conn.transaction() на операцию; примитивы group_place/group_membership без commit"
    - "ядро протаскивания не пишет аудит: у переноса и вложения разные действия"
key-files:
  created:
    - crates/trackly-app/src/services/group_membership.rs
    - crates/trackly-app/tests/groups_membership.rs
  modified:
    - crates/trackly-app/src/services/group_place.rs
    - crates/trackly-app/src/services/group_service.rs
    - crates/trackly-app/src/services/mod.rs
    - crates/trackly-app/src/dto/groups.rs
key-decisions:
  - "remove_devices идемпотентен: устройства не из этой группы игнорируются без ошибки; возвращает число выведенных"
  - "Аудит переноса остался в move_group_in_tx, ядро propagate_group_place_in_tx его не пишет (set_parent пишет свой set_parent)"
  - "Родитель без места: changed_place_ids содержит прежнее место группы (дерево Мест перечитывает группы), хотя места устройств не менялись"
  - "Устройство уже в ЭТОЙ группе при добавлении пропускается без ошибки и не считается в added"
requirements-completed: [GRP-05, GRP-06, GRP-10]
completed: 2026-10-04
---

# Phase 41 Plan 11: Сервис групп, часть 3 Summary

Состав и вложенность управляются через единый транзакционный путь: пакетное добавление устройств с понятной ошибкой «уже входит в группу», вывод без смены места, вложение группы с протаскиванием производного места и примитив `release_device_in_tx`.

## Tasks

| Task | Name | Commit |
|------|------|--------|
| 1 | release_device_in_tx, add_devices, remove_devices | 53d837b4 |
| 2 | set_parent с протаскиванием места, инвариант-тест | 1d8d28e4 |

## Что сделано

- `add_devices`: пакет 1..=500 с дедупликацией, одна транзакция; несуществующее, мягко удалённое или занятое другой группой устройство отменяет весь пакет (Validation на `device_ids`, текст с именами устройства и группы). Группа С местом присваивает место через `apply_group_place_to_device_in_tx` (журнал `source='group'`, `entity_label` и `group_id` без `batch_id`; NULL -> место только в `audit_log`); группа БЕЗ места место не трогает (D-21). Ответ: `added` и `changed_place_ids` без дублей.
- `remove_devices`: `release_device_in_tx` для членов именно этой группы, место не меняется, аудит `custom:group_member_released {group_id, act_id}`.
- `set_parent`: `set_parent_in_tx` (цикл раньше teardown) и дальше по правилам: родитель с местом -> `propagate_group_place_in_tx` на подгруппу (место групп поддерева, пакет журнала, каскад картриджей); родитель без места -> `set_subtree_place_in_tx(None)`, места устройств целы; выход в корень место не меняет. Аудит `set_parent`.
- Ядро `propagate_group_place_in_tx` вынесено из `move_group_in_tx` без изменения поведения.

## Verification

- `groups_membership` 14 passed, `groups_move` 10, `group_movements_journal` 5, `groups_service` 15, `export_bindings` 1 — зелёные.
- `cargo clippy -p trackly-app --all-targets -- -D warnings` — чисто; `rustfmt --check` по изменённым файлам чисто.
- Мутации (якорь уникален, count == 1 проверен, файл восстановлен и сверен diff'ом):
  1. D-21: `group.place_id` -> `group.place_id.or(Some(1))` — красный `membership_add_to_group_without_place_keeps_device_places`;
  2. атомарность: `COMMIT; BEGIN;` после `added.push` — красные `..._batch_is_atomic` и `..._deleted_or_missing_...`;
  3. родитель без места: вызов `set_subtree_place_in_tx(None)` убран — красные `..._without_place_clears_group_place_only` и `membership_place_invariant_walk`;
  4. целевое место вложения подменено на прежнее место группы — красные `..._with_place_propagates_with_journal_batch` и `membership_place_invariant_walk`.
- Фикстуры расходятся до/после (склады А/Б, устройства с местом и без); имена места, группы и устройства для сообщений читаются из БД. check-privacy в pre-commit — PASS; имена вымышленные.
- Инвариант-тест обходит все группы БД после вложения, переноса корня, вложения под родителя без места, выхода и удаления родителя и требует ненулевое число проверенных вложенных групп.

## Deviations from Plan

**1. [Отличие от плана] Аудит вне ядра протаскивания**
- Plan: ядро включает «audit перемещения».
- Сделано: аудит `move` остался в `move_group_in_tx` (идентичный payload), `set_parent` пишет собственный `set_parent`. Иначе вложение дало бы ложную запись о переносе.

**2. [Мелкое] Значение `remove_devices`** — возвращает `i32` (сколько выведено), отдельного DTO результата нет.

Прочих отклонений нет.

## Для следующих планов

- План 13: транспорты `groups_add_devices(GroupAddDevicesDto)`, `groups_remove_devices(GroupRemoveDevicesDto)`, `groups_set_parent(GroupSetParentDto)` (DTO готовы, в specta-реестр не добавлены); `groups_move` по-прежнему без транспорта.
- Планы 14 и 16: `release_device_in_tx(tx, &groups, &audit, device_id, act_id, user_id, now)`; сам примитив вне `remove_devices` напрямую не тестировался (интеграционные тесты не видят `pub(crate)`) — его проверят тесты актов и удаления устройства.
- Дедупликация USB-принтеров и явных ссылок по-прежнему за планом 41-12.

## Known Stubs

None.

## Threat Flags

None. T-41-11-01..07 закрыты: `authorize` первой строкой и тесты employee -> Forbidden; цикл и teardown проверены с неизменностью `parent_group_id`; пакет атомарен (мутация красная); потолок 500 и дедупликация; единый `propagate_group_place_in_tx` и инвариант-тест; аудит с `user_id`.

## Self-Check: PASSED

- group_membership.rs, groups_membership.rs на месте
- коммиты 53d837b4, 1d8d28e4 существуют
