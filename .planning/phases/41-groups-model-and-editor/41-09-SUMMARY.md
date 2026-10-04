---
phase: 41-groups-model-and-editor
plan: 09
subsystem: api
tags: [rust, tauri, axum, transport, groups, group-types, authorization, specta]
requires:
  - phase: 41-07
    provides: "GroupTypeService, dto::group_types, AppCtx.group_types"
provides:
  - "tauri_cmds::group_types: десять build_group_types_* / build_group_type_properties_* и десять #[tauri::command]"
  - "http::group_types: router() с десятью POST /api/v1/<имя_команды>"
  - "Регистрация десяти команд в specta_export.rs (блок Phase 41 — Group types)"
  - "Cases 76-80 матрицы прав: 3 роли x 2 транспорта, неизменяемость, правила защиты, полнота маршрутов"
affects: [41-13 транспорты групп, 41-10.. UI редактора типов]
tech-stack:
  added: []
  patterns:
    - "таблица-драйвер матрицы (имя, GateKind, payload) + двусторонний тест полноты по include_str! исходника роутера"
    - "admin-ветка проверки отказа исключает 401/403/415/422, чтобы тест отказа не был вакуумным"
key-files:
  created:
    - crates/trackly-app/src/tauri_cmds/group_types.rs
    - crates/trackly-app/src/http/group_types.rs
  modified:
    - crates/trackly-app/src/tauri_cmds/mod.rs
    - crates/trackly-app/src/http/mod.rs
    - crates/trackly-app/src/specta_export.rs
    - crates/trackly-app/tests/role_endpoint_matrix.rs
key-decisions:
  - "Десять операций на обоих транспортах; оба адаптера вызывают одни build_*, гейт (ManageGroupTypes / ReadGroups) живёт в build_* и дублирует гейт сервиса"
  - "Тесты матрицы вынесены в отдельные функции, существующий role_endpoint_matrix_test не менялся"
requirements-completed: [GRP-01, GRP-02, GRP-03, GRP-09]
completed: 2026-10-04
---

# Phase 41 Plan 09: Транспорты типов групп и матрица прав Summary

Сервис типов групп и свойств открыт парой Tauri-команда + axum-роут для всех десяти операций; права (Admin мутирует, Admin|Manager читает, Employee отрезан) и неизменяемость `code`/`behavior` доказаны на обоих транспортах невакуумными тестами.

## Tasks

| Task | Name | Commit |
|------|------|--------|
| 1 | Tauri-команды, HTTP-роуты и регистрация для типов и свойств | a7e54446 |
| 2 | Матрица прав 3 роли x 2 транспорта, HTTP-правила, полнота маршрутов | cbf06302 |

## Verification

- `cargo check -p trackly-app` — чисто; `cargo test -p trackly-app --test export_bindings` — passed (`group_type_properties_reorder` в `ui/src/bindings.ts`; файл в .gitignore, в коммит не входит)
- `cargo test -p trackly-app --test role_endpoint_matrix` целиком (с `--skip login_remember_persistent_cookie`) — 6 passed (5 новых + старый `role_endpoint_matrix_test`)
- `cargo clippy -p trackly-app --all-targets -- -D warnings` — чисто
- grep-критерии: `"/api/v1/` в http/group_types.rs = 10; `#[tauri::command]` = 10; `group_types::` в specta_export = 10; `Action::ManageGroupTypes` 9 вхождений (8 мутаций + комментарий), `Action::ReadGroups` 3 (2 чтения + комментарий)
- Мутация полноты: в router() добавлен `.route(\n "/api/v1/group_types_probe", ...)` с переносом строки, как у rustfmt — `group_types_http_route_completeness` краснеет (`left: ["group_types_probe"]`); откат через `git checkout -- <файл>`, якорь вставки уникален (assert count == 1)
- check-privacy в pre-commit — PASS (оба коммита); в тестах только вымышленные имена

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] clippy `unneeded_struct_pattern` в тестах**
- **Found during:** финальный clippy после Task 2
- **Issue:** `AppError::Forbidden { .. }` — Forbidden unit-вариант
- **Fix:** паттерны заменены на `AppError::Forbidden` (4 места, только в блоке Phase 41 Plan 09)
- **Files modified:** crates/trackly-app/tests/role_endpoint_matrix.rs
- **Commit:** cbf06302

Иных отклонений нет. Организационно: `cargo check` ушёл в фон по таймауту инструмента (холодная сборка 4 мин); управление не возвращалось, дождался завершения.

## Known Stubs

None.

## Threat Flags

None. T-41-09-01..05 закрыты: оба транспорта вызывают одни build_* с authorize; manager -> 403 на каждой из 8 мутаций; employee -> 403 на всех 10 (HTTP и прямой вызов); неизменяемость проверена через HTTP со сравнением БД до/после; полнота маршрутов страхуется тестом по исходнику.

## Self-Check: PASSED

- tauri_cmds/group_types.rs, http/group_types.rs, 41-09-SUMMARY.md — на месте
- Коммиты a7e54446, cbf06302 — существуют
