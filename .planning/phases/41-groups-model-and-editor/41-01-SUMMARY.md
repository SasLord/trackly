---
phase: 41-groups-model-and-editor
plan: 01
subsystem: database
tags: [sqlite, migrations, refinery, groups, place_movements]
requires: []
provides:
  - "V045: group_types, group_type_properties, groups, group_devices, group_property_values + trigger неизменяемости code/behavior"
  - "V046: place_movements.batch_id, entity_label, group_id (additive ALTER)"
affects: [41-02, 41-03, group repositories, group services, movement journal]
tech-stack:
  added: []
  patterns:
    - "аддитивные миграции без перестройки таблиц (урок V042)"
    - "токены behavior/data_type без SQL CHECK, валидация в Rust"
key-files:
  created:
    - migrations/V045__groups.sql
    - migrations/V046__place_movements_batch.sql
    - crates/trackly-infra/tests/groups_migration.rs
  modified:
    - crates/trackly-infra/tests/per_record_invariants.rs
    - crates/trackly-infra/tests/place_movements_migration.rs
key-decisions:
  - "group_id в place_movements хранится снимком без FK, пишется в каждую строку групповой записи"
  - "default_props_seeded входит прямо в CREATE TABLE group_types; триггер объявлен на UPDATE OF code, behavior и маркер не блокирует"
  - "Членство 'одна группа' = PRIMARY KEY group_devices.device_id; UNIQUE(type_id, seq) как бэкстоп нумерации"
requirements-completed: [GRP-01, GRP-02, GRP-03, GRP-05, GRP-06, GRP-10]
duration: ~1h (в основном компиляция)
completed: 2026-10-04
---

# Phase 41 Plan 01: Схема групп и колонки пакета в журнале Summary

Аддитивные миграции V045/V046: пять таблиц групп с триггером БД на неизменяемость `code`/`behavior` типа и три колонки корреляции пакета в `place_movements`, доказанные тестом апгрейда с V044 без потери данных.

## Tasks

| Task | Name | Commit |
|------|------|--------|
| 1 | Миграции V045 (схема групп) и V046 (batch_id, entity_label, group_id) | 0b7239b8 |
| 2 | Тест миграции groups_migration.rs и обновление per_record_invariants.rs | c6e4dc94 |

## Verification

- `cargo test -p trackly-infra --test migration_idempotency --test place_movements_migration` — зелёные
- `cargo test -p trackly-infra --test groups_migration` — 4 passed
- `cargo test -p trackly-infra --test per_record_invariants` — 3 passed
- Мутация: подстрока условия триггера встречается в V045 ровно 1 раз; замена на `WHEN 0` краснит `group_types_code_behavior_immutable_trigger`, после отката файл идентичен закоммиченному
- `node scripts/check-privacy.mjs --hashes scripts/privacy-tokens.sha256` — PASS (pre-commit, оба коммита)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Тест place_movements_migration ожидал ровно 5 индексов**
- **Found during:** Task 1 verify
- **Issue:** V046 добавляет idx_place_movements_batch и idx_place_movements_group, число индексов стало 7; существующий тест `place_movements_v040_creates_table_and_indexes` стал бы красным.
- **Fix:** ожидание обновлено 5 -> 7 с пояснением в сообщении утверждения.
- **Files modified:** crates/trackly-infra/tests/place_movements_migration.rs
- **Commit:** 0b7239b8

**2. [Rule 3 - Blocking] Шапка V045 ломала acceptance-счётчик**
- Фраза «CREATE TABLE» в комментарии давала `grep -c "CREATE TABLE"` = 6 вместо 5; комментарий переформулирован.

## Notes

- `embed_migrations!` не отслеживает добавление новых файлов: после добавления/правки SQL крейт trackly-infra нужно пересобрать (`touch crates/trackly-infra/src/db/migrations.rs`), иначе тесты видят старую схему. Проявилось при первом прогоне (5 индексов вместо 7).
- Тест `devices_inventory_dedup_migration::user_version_is_44_after_migration` применяет V044 вручную к синтетической схеме и от V045/V046 не зависит; не менялся и в скоуп прогонов плана не входил.

## Known Stubs

None.

## Threat Flags

None. Новая поверхность — только схема БД, описанная в threat_model плана (T-41-01-01..05 закрыты триггером, аддитивным DDL, PK, UNIQUE, вымышленными тестовыми данными).

## Self-Check: PASSED

- V045__groups.sql, V046__place_movements_batch.sql, groups_migration.rs — на месте
- Коммиты 0b7239b8, c6e4dc94 — существуют
