---
phase: 41-groups-model-and-editor
plan: 15
subsystem: api
tags: [rust, sqlite, groups, places, bulk-move, write-sites, transactions]
requires:
  - phase: 41-10
    provides: "services::group_place::move_group_in_tx / GroupMoveDeps"
  - phase: 41-11
    provides: "propagate_group_place_in_tx, release_device_in_tx"
  - phase: 41-14
    provides: "S1/S8/S9 закрыты, инвентарь write-site'ов devices.place_id"
provides:
  - "PlaceService::move_subtree_contents (S2): группа с местом едет целиком, без дублей журнала"
  - "SubtreeStats / SubtreeStatsDto: referencing_group_count и moving_group_count (считает сервер)"
  - "build_delete_blocked_message и delete_hard: группы на месте блокируют удаление понятным русским текстом"
  - "tests/places_move_groups.rs (6 тестов), +2 теста в tests/places_delete_blocked.rs"
affects: [41-16 акты (S3-S7), 41-25 UI содержимого места (модалка «переедут N групп»)]
tech-stack:
  added: []
  patterns:
    - "устройства группы исключаются из одиночного цикла, группа едет после цикла в той же tx"
    - "счётчик «реально переедут» считается восходящим рекурсивным CTE до корня с дедупом по корню"
key-files:
  created:
    - crates/trackly-app/tests/places_move_groups.rs
  modified:
    - crates/trackly-core/src/domain/places.rs
    - crates/trackly-infra/src/repos/places_sqlite.rs
    - crates/trackly-app/src/dto/place.rs
    - crates/trackly-app/src/services/place_service.rs
    - crates/trackly-app/tests/places_delete_blocked.rs
key-decisions:
  - "referencing_group_count считает все строки groups с place_id в поддереве без фильтра deleted_at_utc: FK ON DELETE RESTRICT не различает мягко удалённые"
  - "moving_group_count = различные КОРНЕВЫЕ группы с place_id IS NOT NULL и живым членом с devices.place_id в поддереве; пустая и спящая группы не входят"
  - "Целевое место для группы проверяется move_group_in_tx (существует, не в архиве); для одиночных устройств прежнее поведение, ошибка группы откатывает всю tx"
requirements-completed: [GRP-07, GRP-06, GRP-10]
completed: 2026-10-04
---

# Phase 41 Plan 15: Массовый перенос места и счётчики групп Summary

Массовый перенос содержимого места теперь двигает группы с местом целиком через `move_group_in_tx` в той же транзакции, без двойных строк журнала и без обхода запрета; удаление места с группой отклоняется русским сообщением с числом групп.

## Tasks

| Task | Name | Commit |
|------|------|--------|
| 1 | referencing_group_count / moving_group_count, сообщение блокировки удаления | ff93a468 |
| 2 | D-23 в move_subtree_contents: группа едет целиком | 1d6636bb |

## Что сделано

- **Счётчики (Task 1).** `SubtreeStats` и `SubtreeStatsDto` получили `referencing_group_count` и `moving_group_count`. Первый считает все группы с `place_id` в поддереве (пустая группа тоже блокирует удаление, `groups.place_id` ON DELETE RESTRICT). Второй считает различные корневые группы с местом, у которых есть живое устройство-член с `devices.place_id` в поддереве: восходящий рекурсивный CTE от группы члена до корня. `delete_hard` и `build_delete_blocked_message` учитывают группы («N группа/группы/групп»), сырого FK-текста нет.
- **S2 (Task 2).** Перед циклом для устройств и принтеров содержимого ищется группа члена, затем корень, затем место корня. Если у корня место есть, устройство помечается «едет с группой» и пропускается в одиночном цикле (счётчик `moved` по-прежнему растёт на каждый элемент, тип и смысл `usize` не менялись). После цикла каждый уникальный корень переносится `move_group_in_tx(.., None, target, ..)` (без CAS). Корень без места (D-21) устройство не запирает. Коммит один, в конце.

## Инвентарь write-site'ов devices.place_id

| Сайт | Статус |
|------|--------|
| S1 `DeviceService::update`, S8 `delete_soft`, S9 backfill принтера | закрыты планом 41-14 |
| S2 `PlaceService::move_subtree_contents` | ЗАКРЫТ этим планом |
| S3-S7 `ActService` (`update_status_and_place_in_tx` ~596/~948, `update_full_in_tx` ~1756/~2355/~2443, `restore_from_snapshot_in_tx` ~1133/~2285/~3645) | ОСТАВЛЕНЫ, план 41-16 |
| `group_place.rs::apply_group_place_to_device_in_tx` | свой путь группы, guard не нужен |

## Verification

- `places_move_groups` 6 passed; `places_delete_blocked` 11 passed (2 новых); `place_movements_bulk_move` 5, `places_contents` 5, `places_service_crud` 4, `groups_move` 10, `group_write_sites` 9; lib `place_service` 10; `export_bindings` 1 (в `ui/src/bindings.ts` поле `moving_group_count` есть; файл в .gitignore).
- `cargo clippy -p trackly-app -p trackly-infra -p trackly-core --all-targets -- -D warnings` чисто; `rustfmt --check` по изменённым файлам чисто; check-privacy PASS на обоих коммитах.
- Мутации (якорь уникален, файл восстановлен копией и сверен через `cmp`):
  1. `COUNT(DISTINCT root_id)` -> `COUNT(root_id)`: красный `group_counters_count_roots_with_live_members_only` (дедуп по корню держится на фикстуре, где участники есть и у корня, и у вложенной группы);
  2. `if devices_riding_with_group.contains(&item.id)` -> `if false && ...` (якорь встречался 1 раз): красные `subtree_group_moves_whole_group_without_duplicate_journal_rows` и `subtree_group_pulls_member_from_another_place_along`.
- Невакуумность: место группы отличается от места устройства вне содержимого; есть спящая группа и группа с участником вне поддерева; счётчики проверены на пяти различных составах; два теста атомарности (сбой на одиночном устройстве и сбой на члене группы ПОСЛЕ обновления одиночных устройств).

## Deviations from Plan

None - plan executed exactly as written. Замечание: `note` массового переноса в `move_group_in_tx` не передаётся (у примитива нет такого параметра), поэтому в строках журнала группы заметки нет; у одиночных устройств она сохраняется как прежде.

## Для следующих планов

- 41-16 (S3-S7, акты): прямые вызовы `update_*_in_tx` из `ActService` guard S1 не проходят; хелперы `seed_group`/`seed_device` лежат в `tests/group_write_sites.rs`.
- 41-25 (UI): `places_subtree_stats` отдаёт `moving_group_count` (число для модалки и тоста) и `referencing_group_count`; клиент их не считает.
- Картридж, установленный в принтер группы, едет в одиночном цикле (manual-строка), каскад группы затем его не дублирует (место уже целевое).

## Known Stubs

None.

## Threat Flags

None. T-41-15-01..04 закрыты (устройства групп едут только через `move_group_in_tx`; одна tx и два fault-injection теста; исключение из одиночного цикла и тест COUNT по `entity_id`; русский текст блокировки и тест без «FOREIGN KEY»).

## Self-Check: PASSED

- places_move_groups.rs на месте; коммиты ff93a468, 1d6636bb существуют
