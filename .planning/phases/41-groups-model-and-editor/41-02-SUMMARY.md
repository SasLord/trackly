---
phase: 41-groups-model-and-editor
plan: 02
subsystem: domain
tags: [rust, trackly-core, groups, authorization, normalization, movements]
requires: []
provides:
  - "domain::groups: GroupBehavior, PropertyDataType, строки/New/Patch типов, свойств, групп, значений, лимиты, validate_name"
  - "domain::group_values: normalize_ip/mac/number/text/scalar, ValueError"
  - "Action::{ManageGroupTypes, MutateGroups, ReadGroups}"
  - "MovementEntityKind::Group, MovementSource::Group"
affects: [41-03, group repositories, group services, group DTO, movement journal]
tech-stack:
  added: []
  patterns:
    - "строгий from_str на записи + from_str_lenient на чтении (токены в строках как String)"
    - "серверная чистая нормализация значений, один код для обоих транспортов"
key-files:
  created:
    - crates/trackly-core/src/domain/groups.rs
    - crates/trackly-core/src/domain/group_values.rs
  modified:
    - crates/trackly-core/src/domain/mod.rs
    - crates/trackly-core/src/auth.rs
    - crates/trackly-core/src/domain/place_movements.rs
    - crates/trackly-app/src/services/report_service.rs
key-decisions:
  - "ManageGroupTypes только Admin; MutateGroups/ReadGroups Admin|Manager; MutatePlaces для групп не используется"
  - "normalize_scalar для пустой строки возвращает Ok(None) раньше диспетчеризации по типу; Users/DeviceRefs отвергаются (связи нормализует сервис)"
  - "Причина перемещения для source=group в отчёте — «группой»; уточнение названием группы отложено сервису групп (D-26/D-28)"
requirements-completed: [GRP-01, GRP-02, GRP-03, GRP-09]
duration: ~40 min (в основном компиляция workspace)
completed: 2026-10-04
---

# Phase 41 Plan 02: Домен групп, нормализация значений и права Summary

Чистый домен групп в trackly-core (закрытые наборы behavior/data_type, row/new/patch-структуры, лимиты), серверная нормализация IP/MAC/числа/текста, три новых Action с матрицей прав и токены Group в журнале перемещений.

## Tasks

| Task | Name | Commit |
|------|------|--------|
| 1 | domain/groups.rs и domain/group_values.rs с табличными тестами | dc27f854 |
| 2 | Action-права групп и варианты Group в токенах журнала | 5cded462 |

## Verification

- `cargo test -p trackly-core group_values` — 9 passed (ip: 6 валидных / 8 невалидных, mac: 5+ валидных / 10 невалидных)
- `cargo test -p trackly-core groups` — зелёные, включая authorize_groups_admin_and_manager
- `cargo test -p trackly-core auth` — 26 passed (authorize_group_types_admin_only, authorize_groups_admin_and_manager)
- `cargo test -p trackly-core place_movements` — 11 passed (group_tokens_parse_and_round_trip)
- `cargo test -p trackly-core --test no_io_deps` — passed
- `cargo check --workspace --tests` — exit 0
- grep serde|specta в groups.rs и group_values.rs — 0; MAX_DEVICES_PER_BATCH = 500
- check-privacy в pre-commit — PASS (оба коммита)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Исчерпывающий match в report_service не компилировался**
- **Found during:** Task 2 (`cargo check --workspace`)
- **Issue:** `movement_reason` в trackly-app матчил `MovementSource` без ветки `Group`.
- **Fix:** добавлена ветка `Some(MovementSource::Group) => "группой"` с комментарием про уточнение в сервисе групп.
- **Files modified:** crates/trackly-app/src/services/report_service.rs
- **Commit:** 5cded462

Иных отклонений нет. Тесты и код писались в одном коммите на задачу (без отдельного RED-коммита; `tdd_mode: false` в конфиге).

## Known Stubs

None.

## Threat Flags

None. Новых сетевых/файловых/схемных поверхностей нет; T-41-02-01..03 закрыты (отдельный Admin-only Action с тестом матрицы, серверная нормализация с лимитами, экспортированные лимиты пакетов).

## Self-Check: PASSED

- groups.rs, group_values.rs, 41-02-SUMMARY.md — на месте
- Коммиты dc27f854, 5cded462 — существуют
