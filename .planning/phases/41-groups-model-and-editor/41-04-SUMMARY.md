---
phase: 41-groups-model-and-editor
plan: 04
subsystem: database
tags: [rust, rusqlite, repository, groups, group-types, seed]
requires:
  - phase: 41-01
    provides: "V045 схема group_types/group_type_properties/groups/group_property_values + триггер неизменяемости"
  - phase: 41-02
    provides: "domain::groups (GroupTypeRow/New/Patch, PropertyRow/New/Patch, GroupBehavior, PropertyDataType)"
provides:
  - "trait GroupTypeRepository (trackly-core::ports::group_types): типы и свойства"
  - "SqliteGroupTypeRepository + inherent seed_builtin_types_in_tx (ON CONFLICT(code) DO NOTHING)"
  - "filled_group_count и groups_missing_required — основа D-14/D-16"
  - "reorder_properties с проверкой множества id в одной транзакции"
affects: [41-07 сервис типов, групповые репозитории, редактор типов]
tech-stack:
  added: []
  patterns:
    - "CAS по version с различением NotFound/OptimisticLockMismatch (resolve_*_cas_failure)"
    - "Option<Option<T>> через CASE-флаг, не COALESCE"
    - "составной засев — inherent *_in_tx, а не метод trait"
key-files:
  created:
    - crates/trackly-core/src/ports/group_types.rs
    - crates/trackly-infra/src/repos/group_types_sqlite.rs
    - crates/trackly-infra/tests/group_types_repo.rs
  modified:
    - crates/trackly-core/src/ports/mod.rs
    - crates/trackly-infra/src/repos/mod.rs
key-decisions:
  - "Репозиторий не имеет метода смены code/behavior; триггер БД ловит прямой UPDATE, map_rusqlite даёт Conflict"
  - "reorder_properties не увеличивает version свойств (только updated_at_utc): иначе перестановка рвала бы CAS открытого редактора свойства"
  - "«Заполнено» = есть value_ref либо непустой value_text, одинаково в filled_group_count и groups_missing_required"
  - "count_groups_of_type и списки считают только живые строки (deleted_at_utc IS NULL)"
requirements-completed: [GRP-01, GRP-02, GRP-03]
completed: 2026-10-04
---

# Phase 41 Plan 04: Репозиторий типов групп и свойств Summary

Порт `GroupTypeRepository` и `SqliteGroupTypeRepository`: CRUD типов и свойств с CAS, идемпотентный засев `ON CONFLICT(code) DO NOTHING`, счётчик заполненности свойства, поиск групп-нарушителей обязательности (`NOT EXISTS`) и атомарная перестановка свойств.

## Tasks

| Task | Name | Commit |
|------|------|--------|
| 1 | Порт GroupTypeRepository, типы, засев | 1874037c |
| 2 | Методы свойств: CRUD, скрытие, порядок, заполненность, нарушители | 28a31c9d |

## Verification

- `cargo test -p trackly-infra --test group_types_repo` — 14 passed
- `cargo clippy -p trackly-infra -p trackly-core --all-targets -- -D warnings` — чисто
- `cargo check --workspace` — без ошибок
- `grep -c "ON CONFLICT(code) DO NOTHING"` в group_types_sqlite.rs = 1; `DO UPDATE` / `INSERT OR REPLACE` не встречаются
- Мутация 1: `ON CONFLICT(code) DO NOTHING` -> `DO UPDATE SET name = excluded.name` (якорь уникален, count == 1) краснит оба теста засева; файл откачен, count снова 1
- Мутация 2: `AND NOT EXISTS (` -> `AND EXISTS (` (якорь уникален) краснит `groups_missing_required_finds_only_groups_without_value`; файл откачен
- Невакуумность фикстур: переименование в «Рабочее место» (≠ сидового «АРМ»); 3 строки значений у 2 групп (count 2, не 3); триггер проверен и ошибкой, и чтением прежнего значения; reorder с чужим id проверен чтением sort_order
- check-privacy в pre-commit — PASS (оба коммита)

## Deviations from Plan

None - plan executed as written. Мелкие решения (не отклонения): `archive_property`/`unarchive_property` идемпотентны (повтор — не ошибка, отсутствие строки — NotFound); счётчики «заполнено» применяют один предикат с запросом нарушителей.

## Known Stubs

None.

## Threat Flags

None. T-41-04-01..04 закрыты: только `rusqlite::params!`, нет метода смены code/behavior, засев с DO NOTHING (мутационно проверен), reorder со сверкой множества id внутри транзакции.

## Self-Check: PASSED

- group_types.rs (port), group_types_sqlite.rs, group_types_repo.rs — на месте
- Коммиты 1874037c, 28a31c9d — существуют
