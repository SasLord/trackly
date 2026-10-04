---
phase: 41-groups-model-and-editor
plan: 07
subsystem: api
tags: [rust, service, groups, group-types, seed, authorization]
requires:
  - phase: 41-02
    provides: "domain::groups, Action::ManageGroupTypes / ReadGroups"
  - phase: 41-04
    provides: "GroupTypeRepository / SqliteGroupTypeRepository"
provides:
  - "GroupTypeService: типы, свойства, засев, правила защиты (оба транспорта — тонкие адаптеры, план 09)"
  - "dto::group_types: GroupTypeDto, GroupTypePropertyDto, входные DTO, PropertyDeleteOutcomeDto, GroupRefDto"
  - "seed_builtin_types_on_startup: три встроенных типа и одноразовый засев свойств АРМ (D-31)"
  - "SqliteGroupTypeRepository::seed_default_properties_in_tx и DefaultProperty"
affects: [41-09 транспорты, редактор типов, карточка группы]
tech-stack:
  added: []
  patterns:
    - "одноразовый засев по маркеру: UPDATE-guard и вставка в одной транзакции writer"
    - "уникальность имени без учёта регистра кириллицы — в Rust, не в SQLite lower()"
    - "update-DTO несёт неизменяемые поля, чтобы serde не проглатывал их молча"
key-files:
  created:
    - crates/trackly-app/src/dto/group_types.rs
    - crates/trackly-app/src/services/group_type_service.rs
    - crates/trackly-app/tests/groups_types_service.rs
  modified:
    - crates/trackly-infra/src/repos/group_types_sqlite.rs
    - crates/trackly-app/src/context.rs
    - crates/trackly-app/src/dto/mod.rs
    - crates/trackly-app/src/services/mod.rs
key-decisions:
  - "Свойство, созданное сразу как обязательное при уже существующих пустых группах типа, отклоняется тем же правилом D-14 (создаётся необязательным, проверяется, при нарушителях удаляется внутри одного замыкания writer)"
  - "Потолок 50 свойств считает и скрытые: иначе скрытие позволило бы раздуть схему"
  - "Скрытое свойство не участвует в расчёте нарушителей обязательности, empty_groups_for_property для него отдаёт пустой список"
  - "У system_unit и teardown свойств по умолчанию нет, маркер остаётся 0"
requirements-completed: [GRP-01, GRP-02, GRP-03, GRP-09]
completed: 2026-10-04
---

# Phase 41 Plan 07: Сервис типов групп и свойств Summary

`GroupTypeService` реализует правила SPEC 1-4: неизменяемость `code`/`behavior` с проверкой через DTO, защиту встроенных типов и типов с группами, скрытие заполненных свойств вместо удаления, запрет смены `data_type` заполненного свойства, проверку обязательности с именами групп-нарушителей и однократный засев свойств АРМ при старте.

## Tasks

| Task | Name | Commit |
|------|------|--------|
| 1 | DTO типов, GroupTypeService (типы + засев), регистрация в AppCtx | 4353124a |
| 2 | Свойства типа: тесты properties_/protect_/rights_ | ee820631 |

## Verification

- `cargo test -p trackly-app --test groups_types_service` — 26 passed
- `cargo test -p trackly-app --test export_bindings` — passed
- `cargo clippy -p trackly-app -p trackly-infra --all-targets -- -D warnings` — чисто
- grep: `ON CONFLICT(code) DO NOTHING` суммарно 1 (только репозиторий); `default_props_seeded = 0` в репозитории 1; `INSERT INTO group_type_properties` в сервисе нет
- Мутация одноразовости: убрано `AND default_props_seeded = 0` (якорь уникален) — краснеют seed_props_rename, seed_props_hidden, seed_props_deleted (а также seed_props_custom и restart-тест), seed_props_clean остаётся зелёным; откат
- Мутация неизменяемости: условие смены code заменено на `false` (якорь уникален) — краснеет immutable_code_rejected_and_row_unchanged; откат
- Мутация защиты data_type: `let filled = repo.filled_group_count(conn, id)?;` -> `let filled = 0;` (якорь уникален) — краснеет protect_b; откат
- check-privacy в pre-commit — PASS (оба коммита)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Литералы `AppCtx { .. }` в тестовых фикстурах**
- **Found during:** Task 1 (cargo check --tests)
- **Issue:** новое поле `group_types` сломало сборку восьми мест, собирающих `AppCtx` вручную
- **Fix:** в каждое добавлено построение `GroupTypeService` и поле
- **Files modified:** src/http/health.rs, src/tauri_cmds/health.rs, tests/templates_status.rs, report_movements.rs, report_requests.rs, reports_period_required.rs, specta_roundtrip.rs
- **Commit:** 4353124a

**2. [Организационное] Граница коммитов**
Методы свойств нужны тестам засева (rename/hidden/deleted), поэтому код `create_property`/`update_property`/`delete_property`/`unarchive_property`/`reorder_properties`/`empty_groups_for_property` вошёл в коммит Task 1, а коммит Task 2 содержит тесты properties_/protect_/rights_ к нему.

## Known Stubs

None.

## Threat Flags

None. T-41-07-01..06 закрыты: authorize первой строкой каждого метода (тесты manager/employee -> Forbidden), неизменяемость code/behavior невакуумно проверена, засев с DO NOTHING и маркером, значения не теряются при правке схемы, потолок 50 свойств, аудит каждой мутации типа и свойства.

## Self-Check: PASSED

- group_types.rs (dto), group_type_service.rs, groups_types_service.rs — на месте
- Коммиты 4353124a, ee820631 — существуют
