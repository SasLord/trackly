---
phase: 41-groups-model-and-editor
plan: 05
subsystem: movement-journal
tags: [rust, sqlite, place_movements, groups, timeline, dto]
requires: ["41-01", "41-02"]
provides:
  - "NewMovement/MovementRow: batch_id, entity_label, group_id (колонки V046)"
  - "SqlitePlaceMovementsRepository::record_batch_movement_if_applicable (соседний метод, общий record_inner)"
  - "MovementEntryDto: batch_id, group_id, group_label"
affects: [41-06, group services, movements report, timeline UI]
tech-stack:
  added: []
  patterns:
    - "соседний метод вместо смены сигнатуры 7 write-site'ов; единая точка INSERT и единое правило is_reportable_place_change"
    - "ссылка на группу читается из колонки каждой строки (аналог act_id), без JOIN"
key-files:
  created:
    - crates/trackly-infra/tests/place_movements_batch_repo.rs
    - crates/trackly-app/tests/place_movements_group_fields.rs
  modified:
    - crates/trackly-infra/src/repos/place_movements_sqlite.rs
    - crates/trackly-app/src/dto/place_movements.rs
    - crates/trackly-app/src/services/place_movement_service.rs
key-decisions:
  - "record_movement_if_applicable не менял сигнатуру: делегирует record_inner с batch_id/entity_label/group_id = None"
  - "group_id и group_label в DTO берутся прямо из place_movements.group_id / entity_label, а не из строки группы в пакете"
  - "Авторизация get_timeline не менялась: ReadPlaces-гейт покрывает entity_type='group'"
requirements-completed: [GRP-06]
duration: ~45 min (в основном компиляция)
completed: 2026-10-04
---

# Phase 41 Plan 05: Пакет группового переноса в журнале и DTO таймлайна Summary

Журнал перемещений пишет и читает batch_id, снимок имени группы и id группы через соседний метод `record_batch_movement_if_applicable`; DTO таймлайна отдаёт `batch_id`, `group_id`, `group_label` прямо из колонок V046.

## Tasks

| Task | Name | Commit |
|------|------|--------|
| 1 | Репозиторий журнала: batch_id, entity_label, group_id, соседний метод записи | 01dad4a3 |
| 2 | MovementEntryDto: batch_id, group_id, group_label; заполнение в get_timeline | 994d568e |

## Verification

- `cargo test -p trackly-infra --test place_movements_batch_repo --test place_movements_repo --test place_movements_migration` — 6 + 3 + 6 passed
- `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test place_movements_group_fields` — 5 passed; `--test place_movements_timeline` — 6 passed
- `cargo test -p trackly-app --lib place_movements` — 3 passed (DTO unit tests)
- `cargo test -p trackly-app --test export_bindings` — зелёный, `group_label` присутствует в ui/src/bindings.ts
- `cargo clippy -p trackly-infra --all-targets -- -D warnings` и `-p trackly-app` — чисто
- check-privacy в pre-commit — PASS (оба коммита)
- Сигнатура `record_movement_if_applicable` не изменена (11 аргументов), `grep -rn group_id_for_batch crates` пусто

## Deviations from Plan

None по сути. Мелкое замечание по acceptance: `grep -c "INSERT INTO place_movements"` даёт 2, а не 1, потому что одно совпадение — давний doc-комментарий в шапке файла (строка 8); исполняемый INSERT ровно один (`insert_in_tx`).

`cargo test -p trackly-infra place_movements` фильтрует по имени теста, а не по файлу, поэтому реальный прогон выполнен по целям `--test place_movements_*` (список выше).

## Known Stubs

None. Заполнение batch_id/entity_label/group_id реальными значениями делают сервисы групп в следующих планах; здесь только репозиторий и чтение.

## Threat Flags

None. T-41-05-01 закрыт тестом (employee -> Forbidden на entity_type='group'), T-41-05-02/03 — параметризованным SQL и единой точкой INSERT.

## Self-Check: PASSED

- Файлы создания/изменения на месте, коммиты 01dad4a3 и 994d568e существуют
