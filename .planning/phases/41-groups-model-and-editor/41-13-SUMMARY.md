---
phase: 41-groups-model-and-editor
plan: 13
subsystem: api
tags: [rust, tauri, axum, transport, groups, authorization, specta, role-matrix]
requires:
  - phase: 41-09
    provides: "образец транспорта (group_types), таблица-драйвер матрицы, хелперы gt_*"
  - phase: 41-12
    provides: "GroupService целиком: чтения, create/rename/delete, move, состав, вложенность, значения, user_options, card"
provides:
  - "tauri_cmds::groups: 15 build_groups_* и 15 #[tauri::command]"
  - "http::groups: router() с 15 POST /api/v1/groups_*"
  - "Регистрация 15 команд в specta_export.rs (блок Phase 41 — Groups)"
  - "Cases 81-86 матрицы прав: 3 роли x 15 команд x 2 транспорта, кросс-проверка с типами, серверная валидация, правила состава/вложенности, полнота маршрутов"
affects: [41-18..41-24 UI раздела «Группы»]
tech-stack:
  added: []
  patterns:
    - "build_* дублирует гейт сервиса (ReadGroups / MutateGroups) на границе транспорта"
    - "отдельный enum GroupGate вместо расширения GateKind плана 09: старые match не тронуты"
    - "тест полноты маршрутов по include_str! исходника роутера, в обе стороны"
key-files:
  created:
    - crates/trackly-app/src/tauri_cmds/groups.rs
    - crates/trackly-app/src/http/groups.rs
  modified:
    - crates/trackly-app/src/tauri_cmds/mod.rs
    - crates/trackly-app/src/http/mod.rs
    - crates/trackly-app/src/specta_export.rs
    - crates/trackly-app/tests/role_endpoint_matrix.rs
key-decisions:
  - "Пятнадцать операций на обоих транспортах; оба адаптера вызывают одни build_groups_*, правил в адаптерах нет"
  - "groups_add_devices и groups_move — единственные обработчики для обоих путей UI (D-01, D-17/D-18)"
  - "Для мутаций, принимающих DTO (set_parent, add/remove_devices, move, set_values), HTTP-payload — {dto}; groups_update — плоский {id, version, name}"
requirements-completed: [GRP-04, GRP-05, GRP-06, GRP-08, GRP-09]
completed: 2026-10-04
---

# Phase 41 Plan 13: Транспорты групп и матрица прав Summary

GroupService открыт парой Tauri-команда + axum-роут для всех пятнадцати операций; права (Admin и Manager читают и мутируют, Employee отрезан) и серверная валидация доказаны невакуумными тестами на обоих транспортах.

## Tasks

| Task | Name | Commit |
|------|------|--------|
| 1 | Tauri-команды, HTTP-роуты и регистрация 15 операций групп | 0b7ac040 |
| 2 | Матрица прав x 2 транспорта, HTTP-проверки правил, тест полноты | c0bdd4b2 |

## Verification

- `cargo check -p trackly-app` чисто; `cargo test -p trackly-app --test export_bindings` passed (`groups_move`, `groups_add_devices`, `groups_set_values` в `ui/src/bindings.ts`; файл в .gitignore, в коммит не входит)
- `cargo test -p trackly-app --test role_endpoint_matrix groups_` — 6 passed; весь файл с `--skip login_remember_persistent_cookie` — 12 passed (6 новых, 5 плана 09, старый `role_endpoint_matrix_test`)
- `cargo clippy -p trackly-app --all-targets -- -D warnings` чисто; `rustfmt --check` по трём своим файлам чисто
- grep-критерии: `"/api/v1/` в http/groups.rs = 15; `#[tauri::command]` = 15; `Action::MutateGroups` 9 (8 мутаций + комментарий), `Action::ReadGroups` 8 (7 чтений + комментарий)
- Мутация полноты: в router() вставлен `.route("/api/v1/groups_probe", ...)` (якорь уникален, count == 1 проверен) — `groups_http_route_completeness` краснеет (`left: ["groups_probe"]`); файл восстановлен из резервной копии, `cmp` чисто
- Невакуумность: каждая проверка отказа идёт после контрольного запроса admin/manager, дающего не-401/403/415/422; валидация ip сверяет `COUNT(*)` в `group_property_values` и `version` до и после; для mac перед каждой формой пишется другое значение; для занятого устройства, цикла, «Разбора» и переноса вложенной группы есть положительный контроль на том же маршруте
- check-privacy в pre-commit — PASS (оба коммита); в тестах только вымышленные имена

## Deviations from Plan

### Auto-fixed Issues

None по правилам 1-3.

### Организационное

- Вместо расширения `GateKind` значением `GroupMut` введён отдельный `GroupGate {Mut, Read}`: существующие тесты плана 09 делают исчерпывающий `match kind` по `TypeMut/Read`, и план требует их не менять.
- Тип «Разбор» для проверки вложения в teardown создаётся прямым SQL (код `dismantle_grp13`), как в `groups_membership.rs`, а не берётся из засева.
- `cargo check` и clippy шли около четырёх минут каждый (холодная сборка); выполнялись в foreground.

## Known Stubs

None.

## Threat Flags

None. T-41-13-01..06 закрыты: employee получает 403 на всех 15 командах и по HTTP, и прямым вызовом build_groups_*; manager создаёт группу (200) и получает 403 на создании типа (число типов в БД не меняется); невалидный ip отклоняется с 400 без записи; цикл, teardown и перенос вложенной группы дают 400 без изменения БД; `groups_user_options` отдаёт ровно ключи `full_name`, `id`, `login`; незарегистрированный в таблице маршрут краснит тест полноты.

## Self-Check: PASSED

- tauri_cmds/groups.rs, http/groups.rs, 41-13-SUMMARY.md — на месте
- Коммиты 0b7ac040, c0bdd4b2 — существуют
