---
phase: 41-groups-model-and-editor
plan: 06
subsystem: database
tags: [rust, rusqlite, repository, groups, recursive-cte, membership]
requires:
  - phase: 41-01
    provides: "V045 схема groups/group_devices/group_property_values"
  - phase: 41-02
    provides: "domain::groups (GroupRow, GroupNew, GroupValueRow)"
  - phase: 41-04
    provides: "образец стиля group_types_sqlite, CAS-хелперы"
provides:
  - "trait GroupRepository (trackly-core::ports::groups): простое чтение групп"
  - "SqliteGroupRepository: inherent *_in_tx примитивы (seq, членство, вложенность, состав, значения)"
  - "usb_printers_for_group / ref_devices_for_group / tree_counts / group_counts_by_type"
  - "domain::groups::PrinterRefRow"
affects: [41-08, 41-10, 41-11, 41-12 сервис групп]
tech-stack:
  added: []
  patterns:
    - "составные операции — inherent *_in_tx на &Transaction, а не метод trait'а"
    - "SQL собирается только через const + concat; динамика — список ?-заглушек"
    - "замена набора значений внутри SAVEPOINT: частичная замена откатывается"
key-files:
  created:
    - crates/trackly-core/src/ports/groups.rs
    - crates/trackly-infra/src/repos/groups_sqlite.rs
    - crates/trackly-infra/tests/groups_repo.rs
  modified:
    - crates/trackly-core/src/domain/groups.rs
    - crates/trackly-core/src/ports/mod.rs
    - crates/trackly-infra/src/repos/mod.rs
key-decisions:
  - "locked_group_for_device_in_tx фильтрует place_id IS NOT NULL: D-21 реализован запросом"
  - "цикл проверяется раньше teardown-правила (группа-сама-себе-родитель получает сообщение о цикле)"
  - "CTE предков на UNION, а не UNION ALL: испорченные данные с циклом не зациклят запрос"
  - "add_device_in_tx предпроверяет членство и отдаёт Conflict с device_id; PK остаётся бэкстопом"
  - "USB-принтеры: и принтер, и хост-устройство обязаны быть живыми"
  - "дедупликацию USB и явных ссылок репозиторий не делает — оба источника отдаются сервису (план 12)"
requirements-completed: [GRP-04, GRP-05, GRP-06, GRP-08, GRP-10]
completed: 2026-10-04
---

# Phase 41 Plan 06: Репозиторий групп Summary

Слой хранения групп: нумерация `MAX(seq)+1` по колонке, членство с PK-инвариантом «одна группа на устройство», вложенность с защитой от циклов и запретом вложения в «Разбор», рекурсивный состав, замена значений свойств в savepoint и производные USB-принтеры.

## Tasks

| Task | Name | Commit |
|------|------|--------|
| 1 | Порт GroupRepository и SQL групп: CRUD, seq, членство, вложенность, состав | 2559f1ea |
| 2 | Значения свойств, производные принтеры, данные для дерева | 78b902ad |
| - | Правка комментария PrinterRefRow (grep-критерий serde = 0) | 8457849b |

## Verification

- `cargo test -p trackly-infra --test groups_repo` — 18 passed
- `cargo clippy -p trackly-infra -p trackly-core --all-targets -- -D warnings` — чисто; `cargo fmt --check` — чисто; `cargo check --workspace` — без ошибок
- `grep -c "WITH RECURSIVE"` = 3 литерала в groups_sqlite.rs до задачи 2 (ancestors, up, общий SUBTREE_CTE для групп и устройств поддерева); после задачи 2 добавились tree_counts
- `format!(` в groups_sqlite.rs — только два сообщения об ошибках Conflict; SQL строится через `concat()`
- `grep -c serde` в domain/groups.rs = 0
- Мутации (якорь уникален, count == 1 проверен, файл откатывался и сверялся diff'ом):
  1. убран `AND g.place_id IS NOT NULL` — красный `locked_group_requires_place`
  2. `SELECT 1 FROM ancestors WHERE id = ?2` -> `SELECT 1 WHERE 0` — красный `cycle_detection_and_unchanged_parent`
  3. `== "teardown"` -> `== "never"` — красный `teardown_group_cannot_be_parent`
  4. USB-запрос расширен `IS NOT NULL OR` — красный `usb_printers_come_from_composition_including_nested_group`
  5. убран `ROLLBACK TO gpv_replace` — красный `two_primary_values_conflict_and_old_values_survive`
- check-privacy в pre-commit — PASS (все три коммита)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Вакуумный тест отката savepoint**
- **Found during:** Task 2, мутационная проверка
- **Issue:** первая версия `two_primary_values_conflict_and_old_values_survive` проходила и без `ROLLBACK TO`: частично вставленное значение совпадало с прежним (одна строка `ref=1, primary` и до, и после).
- **Fix:** набор для замены стал `[ref 2 не primary, ref 1 primary, ref 3 primary]` — две вставки проходят, третья ломает `uq_gpv_primary`; тест требует ровно одну прежнюю строку. Мутант теперь красный.
- **Files modified:** crates/trackly-infra/tests/groups_repo.rs
- **Commit:** 78b902ad

Мелкие решения сверх плана (не отклонения): добавлены `parent_behavior_in_tx` (упомянут в behavior-блоке) и `group_counts_by_type` («число групп по типу» из behavior tree_counts); `replace_property_values_in_tx` оборачивает замену в SAVEPOINT, чтобы метод был атомарным сам по себе.

## Known Stubs

None.

## Threat Flags

None. T-41-06-01..04 закрыты и доказаны тестами/мутациями: CTE цикла перед UPDATE (parent_group_id в БД не меняется при отказе), PK + Conflict, только `rusqlite::params!`, только живые devices. T-41-06-05 принят: `tree_counts` — один рекурсивный запрос.

## Self-Check: PASSED

- ports/groups.rs, groups_sqlite.rs, groups_repo.rs — на месте
- Коммиты 2559f1ea, 78b902ad, 8457849b — существуют
