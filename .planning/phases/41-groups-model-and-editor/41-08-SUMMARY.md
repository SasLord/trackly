---
phase: 41-groups-model-and-editor
plan: 08
subsystem: api
tags: [rust, service, groups, numbering, composition, authorization]
requires:
  - phase: 41-02
    provides: "domain::groups, Action::MutateGroups / ReadGroups, MAX_DEVICES_PER_BATCH"
  - phase: 41-06
    provides: "SqliteGroupRepository: next_seq_in_tx, insert/rename/delete_group_in_tx, tree_counts, groups_for_devices, direct_child_groups"
  - phase: 41-07
    provides: "GroupTypeRepository, образец сервиса и AppCtx-регистрации"
provides:
  - "GroupService: list_groups, get_group, composition, search, for_devices, create_group, update_group, delete_group"
  - "dto::groups: GroupDto, GroupCreateDto, GroupCompositionDto, GroupMemberDeviceDto, GroupSearchHitDto, DeviceMembershipDto, GroupDeleteResultDto"
  - "AppCtx.groups"
  - "репозиторий: member_devices, direct_device_counts, group_place_paths; domain::groups::MemberDeviceRow"
affects: [41-09 транспорты, 41-10 место группы, 41-11 состав, 41-12 карточка]
tech-stack:
  added: []
  patterns:
    - "снимок GroupSnapshot: один набор запросов (группы, типы, счётчики, пути) вместо N+1; корень цепочки считается в памяти с ограничением обхода"
    - "мутация = одна conn.transaction() writer'а: проверки, запись, DTO для аудита и коммит внутри"
    - "поиск кандидатов и исключение поддерева — в Rust (to_lowercase().contains), без шаблонного SQL"
key-files:
  created:
    - crates/trackly-app/src/dto/groups.rs
    - crates/trackly-app/src/services/group_service.rs
    - crates/trackly-app/tests/groups_service.rs
  modified:
    - crates/trackly-core/src/domain/groups.rs
    - crates/trackly-infra/src/repos/groups_sqlite.rs
    - crates/trackly-app/src/context.rs
    - crates/trackly-app/src/dto/mod.rs
    - crates/trackly-app/src/services/mod.rs
    - crates/trackly-app/src/http/health.rs
    - crates/trackly-app/src/tauri_cmds/health.rs
key-decisions:
  - "Явное имя группы из пробелов отклоняется («Укажите название.»); имя по умолчанию только при name = None"
  - "Место при создании: несуществующее и архивное -> Validation{place_id}, а не NotFound"
  - "released_devices = только прямые живые устройства удаляемой группы; устройства вложенной группы остаются в ней"
  - "Teardown-тип остаётся кандидатом поиска вложенных: запрет вложения в него проверяет set_parent_in_tx"
requirements-completed: [GRP-04, GRP-05, GRP-08]
requirements-partial: [GRP-10]
completed: 2026-10-04
---

# Phase 41 Plan 08: Сервис групп, часть 1 Summary

`GroupService` даёт чтения (список, карточка, состав, поиск вложенных, членство пачкой) и CRUD с нумерацией `MAX(seq)+1` по колонке; удаление освобождает устройства, не меняя их мест.

## Tasks

| Task | Name | Commit |
|------|------|--------|
| 1 | DTO групп, строки состава в репозитории, чтения GroupService, регистрация в AppCtx | 3045dfcb |
| 2 | Запись групп: создание с нумерацией, переименование, удаление | 977f8126 |

## Что сделано

- Чтения под `ReadGroups` (admin|manager), через пул читателей. Список/карточка строятся из одного снимка: `device_count` включает вложенные, `direct_device_count` только прямые, `place_path` пуст у группы без места (D-21), `root_group_id/name` для вложенной указывают на корень (D-20).
- Состав: прямые устройства со столбцами `PlaceContents` и сокращённым путём из `compute_place_path_short_with_conn` (своей формулы сокращения нет), прямые вложенные группы со счётчиком, включающим их собственные вложенные (D-03/D-05).
- Поиск вложенных: исключает группу и потомков, регистронезависим для кириллицы, потолок 50, запрос ≤ 100 символов. `for_devices`: потолок 500 id, `group_has_place` отражает D-21.
- Запись под `MutateGroups`: одна транзакция на мутацию вместе с аудитом (`group` create/update/delete); имя по умолчанию строится только из `seq`, разбора имён нет.

## Verification

- `cargo test -p trackly-app --test groups_service` — 15 passed
- `cargo test -p trackly-app --test export_bindings` — passed
- `cargo clippy -p trackly-app -p trackly-infra -p trackly-core --all-targets -- -D warnings` — чисто
- `rustfmt --check` по новым файлам чисто; прежний дрейф `dto/act.rs` и ещё нескольких файлов не тронут
- grep: `LIKE` в group_service.rs — 0; `split(`/`Regex`/`parse::<i64>` — 0; `conn.transaction()` — 4; `compute_place_path_short_with_conn` используется, `to_lowercase` — фильтр поиска
- Мутации (якоря уникальны, count == 1 проверен, файл восстановлен и сверен diff'ом):
  1. убрано исключение поддерева в поиске — красный `search_is_case_insensitive_for_cyrillic_and_excludes_subtree`
  2. имя по умолчанию `seq` -> `seq + 1` — красный `numbering_default_names_follow_seq_and_rename_does_not_shift_counter`
  3. `released` из `direct_device_count` -> `device_count` — красный `delete_releases_devices_keeps_places_and_unnests_children`
  4. проверка архивного места отключена — красный `crud_place_is_optional_checked_and_writes_no_movements`
- Ожидаемые значения (тип, путь места, статус, `devices.place_id` до/после) читаются из БД, не хардкодятся; фикстура delete различает прямые (3) и вложенные (1) устройства, поэтому мутация 3 не вакуумна.
- check-privacy в pre-commit — PASS (оба коммита); данные вымышленные.

## GRP-10: закрыт ЧАСТИЧНО

В этом плане сделано: освобождение устройств при удалении группы, неизменность `devices.place_id`, вложенные группы становятся корневыми с прежним местом, подтверждение со счётчиком (`direct_device_count` в `GroupDto`, `released_devices` в ответе), аудит.

Остаток переносится в фазу 41.1: «удаление якорного устройства запрещено, пока группа существует» — требует `groups.anchor_device_id`, которого в схеме V045 нет.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Литералы `AppCtx { .. }` в фикстурах**
- **Found during:** Task 1 (cargo check --tests)
- **Issue:** новое поле `groups` сломало сборку шести мест, собирающих `AppCtx` вручную
- **Fix:** в каждое добавлено построение `GroupService` и поле
- **Files modified:** src/http/health.rs, src/tauri_cmds/health.rs, tests/specta_roundtrip.rs, templates_status.rs, report_movements.rs, report_requests.rs, reports_period_required.rs
- **Commit:** 3045dfcb

**2. [Расхождение внутри плана] Пустое явное имя**
Action плана («None/пусто -> имя по умолчанию») противоречит behavior («пустое/пробельное явное имя -> Validation»). Реализован behavior: по умолчанию только при `name = None`.

Мелкое решение сверх плана: добавлены `direct_device_counts` и `group_place_paths` в репозиторий, чтобы чтения не делали N+1 запросов.

## Known Stubs

None.

## Threat Flags

None. T-41-08-01..06 закрыты: `authorize` первой строкой каждого метода (тесты employee -> Forbidden для чтений и записей), `seq` считается внутри writer-транзакции (конкурентный тест даёт 1 и 2), существование и неархивность места проверяются на сервере, потолки 500 id и 50 результатов, аудит в той же транзакции с `user_id`.

## Self-Check: PASSED

- dto/groups.rs, services/group_service.rs, tests/groups_service.rs — на месте
- Коммиты 3045dfcb, 977f8126 — существуют
