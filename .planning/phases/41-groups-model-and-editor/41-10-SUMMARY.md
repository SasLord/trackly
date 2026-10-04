---
phase: 41-groups-model-and-editor
plan: 10
subsystem: api
tags: [rust, sqlite, groups, place-propagation, movements-journal, transactions]
requires:
  - phase: 41-05
    provides: "record_batch_movement_if_applicable (batch_id, entity_label, group_id)"
  - phase: 41-06
    provides: "SqliteGroupRepository: subtree_group_ids_in_tx, subtree_device_ids_in_tx, set_subtree_place_in_tx, root_group_id_in_tx"
  - phase: 41-08
    provides: "GroupService, GroupDto"
provides:
  - "services::group_place: GroupMoveDeps, DevicePlaceChange, apply_group_place_to_device_in_tx, move_group_in_tx (pub(crate), для планов 11 и 15)"
  - "GroupService::move_group(caller, id, version, target_place_id) -> GroupMoveResultDto"
  - "dto::groups: GroupMoveDto, GroupMoveResultDto"
  - "services::plural::ru_plural (общий хелпер склонения)"
  - "SqliteCartridgeRepository::cascade_place_for_printer_batch_in_tx"
affects: [41-11 состав, 41-13 транспорты, 41-15 массовый перенос места]
tech-stack:
  added: []
  patterns:
    - "примитивы протаскивания работают на &Transaction и не коммитят; коммит один, в сервисе"
    - "соседний batch-метод каскада картриджей, прежний метод делегирует с None"
key-files:
  created:
    - crates/trackly-app/src/services/plural.rs
    - crates/trackly-app/src/services/group_place.rs
    - crates/trackly-app/tests/groups_move.rs
    - crates/trackly-app/tests/group_movements_journal.rs
  modified:
    - crates/trackly-app/src/services/place_service.rs
    - crates/trackly-app/src/services/mod.rs
    - crates/trackly-app/src/services/group_service.rs
    - crates/trackly-app/src/dto/groups.rs
    - crates/trackly-infra/src/repos/cartridges_sqlite.rs
key-decisions:
  - "apply_group_place_to_device_in_tx возвращает DevicePlaceChange{changed, previous_place}, а не Option<i64>: прежнее место None (D-30) иначе неотличимо от «не менялось»"
  - "Каскад картриджей вызывается для любого устройства-члена без проверки таблицы printers: запрос идёт по current_printer_device_id, без картриджей это no-op (так же ведёт себя DeviceService::update)"
  - "Перенос в то же место: ничего не пишется, версия группы не растёт, changed_place_ids пуст, summary «группа»"
  - "Строка группы в журнале пишется общим гейтом (только Some->Some); при NULL->место её нет, строки устройств всё равно несут group_id/batch_id/entity_label"
requirements-completed: [GRP-06, GRP-07, GRP-05]
completed: 2026-10-04
---

# Phase 41 Plan 10: Перенос группы Summary

`groups_move` меняет место корневой группы, вложенных групп и каждого устройства состава одной транзакцией writer'а, пишет пакет `place_movements` с общим `batch_id` и отдаёт ответ для тоста и инвалидации счётчиков.

## Tasks

| Task | Name | Commit |
|------|------|--------|
| 1 | Общий ru_plural, batch-каскад картриджей, DTO переноса | e7b872bf |
| 2 | group_place.rs и GroupService::move_group | da1203f0 |
| 3 | Тесты журнала: 7 строк, D-30, картриджи, первое размещение | c8c1cc5d |

## Что сделано

- `move_group_in_tx`: CAS по версии группы (если передана), отказ для вложенной группы с именем корня в тексте, проверка существования и неархивности целевого места до любых изменений, `set_subtree_place_in_tx` до цикла устройств, строка группы, цикл устройств, аудит `group/move`.
- `apply_group_place_to_device_in_tx`: запись места, строка журнала (`source='group'`, `group_id` в каждой строке), D-30 — `audit_log` `custom:group_place_assigned` для устройства без места, каскад картриджей тем же пакетом.
- `GroupService::move_group`: `authorize(MutateGroups)` первой строкой, одно замыкание writer, один `tx.commit()`. В `group_place.rs` коммитов нет.
- `ru_plural` перенесён из `place_service.rs` без изменения формулировок; одно определение в `services/plural.rs`.

## Verification

- `groups_move` 10 passed, `group_movements_journal` 5 passed, `groups_service` 15, `place_movements_write_sites_cartridges` 3, `place_movements_write_sites_devices` 6, `place_movements_bulk_move` 5, `export_bindings` 1, lib `plural` 2 и `place_service` 10 — зелёные.
- `cargo clippy -p trackly-app -p trackly-infra --all-targets -- -D warnings` — чисто; `rustfmt --check` по новым файлам чисто; `git diff` по `device_service.rs` пуст.
- Атомарность: триггер `BEFORE UPDATE OF place_id` на 4-м устройстве; тест сверяет `groups.place_id`, `devices.place_id`, `COUNT(place_movements)`, `COUNT(audit_log)` до и после и требует, чтобы ошибка содержала «boom».
- Мутации (якорь уникален, count == 1 проверен, файл восстановлен):
  1. после обновления устройства вставлен `COMMIT; BEGIN` (автокоммит по шагам) — красный `move_atomic_failure_on_fourth_device_rolls_back_everything`;
  2. `action: "custom:group_place_assigned"` -> `"x"` — красный `journal_null_place_d30`.
- check-privacy в pre-commit — PASS (три коммита); имена вымышленные.

## Deviations from Plan

**1. [Отличие от сигнатуры плана] Тип результата apply_group_place_to_device_in_tx**
- Plan: `Result<Option<i64>, _>` («прежнее место, если менялось»). Для устройства без места (D-30) прежнее место — `None`, и такой результат не отличить от «не менялось», а `moved_devices` бы занижался.
- Fix: структура `DevicePlaceChange { changed, previous_place }`. Планы 11 и 15 используют её.

**2. [Мутация вместо замены транзакции] Подход к мутационной проверке атомарности**
- Транзакционные примитивы принимают `&Transaction`, подменить её автокоммитом без переписывания нельзя, поэтому мутация — `COMMIT; BEGIN` внутри цикла. Она эквивалентна «автокоммиту по шагам» и краснит тест.

Других отклонений нет.

## Для следующих планов

- План 11 (добавление в состав) и план 15 (D-23) переиспользуют `apply_group_place_to_device_in_tx` и `move_group_in_tx(.., None, ..)`.
- Транспорты `groups_move` (Tauri + `POST /api/v1/groups_move`) этим планом НЕ добавлены; их делает план 13 (`build_groups_move(GroupMoveDto)`), DTO уже готов.
- Семь прежних write-site'ов по-прежнему пишут NULL в batch-колонки; сюда не лезли.

## Known Stubs

None.

## Threat Flags

None. T-41-10-01..05 закрыты: `authorize` первой строкой и тест employee -> Forbidden; одна `conn.transaction()` и fault-injection тест с мутацией; Validation для вложенной группы; `places.get` до изменений; пакет журнала и аудит с `user_id` и `batch_id` в той же транзакции.

## Self-Check: PASSED

- plural.rs, group_place.rs, groups_move.rs, group_movements_journal.rs на месте
- коммиты e7b872bf, da1203f0, c8c1cc5d существуют
