---
phase: 41-groups-model-and-editor
plan: 14
subsystem: api
tags: [rust, sqlite, groups, write-sites, devices, guard, transactions]
requires:
  - phase: 41-06
    provides: "SqliteGroupRepository::locked_group_for_device_in_tx (D-21: только группы с place_id IS NOT NULL)"
  - phase: 41-11
    provides: "services::group_membership::release_device_in_tx"
provides:
  - "DeviceService::update: guard S1 (GRP-07) — реальная смена place_id члена группы с местом -> Validation{field:'place_id'}"
  - "DeviceService::delete_soft: освобождение членства в той же транзакции (S8)"
  - "cartridges_sqlite.rs: S9 зафиксирован комментарием и тестом (сознательно без guard'а)"
  - "tests/group_write_sites.rs: таблица-драйвер S1/S8/S9 + HTTP/Tauri-путь, расширяется планом 16"
affects: [41-15 массовый перенос места (S2), 41-16 акты (S3-S7), 41-24 блок в форме устройства]
tech-stack:
  added: []
  patterns:
    - "guard сравнивает с ТЕКУЩИМ значением внутри writer-замыкания: повтор проходит, смена блокируется"
    - "таблица-драйвер: сценарий = строка данных, фикстуры расходятся до/после"
key-files:
  created:
    - crates/trackly-app/tests/group_write_sites.rs
  modified:
    - crates/trackly-app/src/services/device_service.rs
    - crates/trackly-infra/src/repos/cartridges_sqlite.rs
key-decisions:
  - "Сравнение new_place != before_place_id делается по значению, прочитанному в том же замыкании (форма шлёт place_id при каждом сохранении)"
  - "Очистка места (Some(None)) у члена группы с местом считается реальной сменой и отклоняется"
  - "S9 без guard'а: условие WHERE place_id IS NULL у принтера-члена группы с местом не срабатывает само"
requirements-completed: [GRP-07, GRP-05]
completed: 2026-10-04
---

# Phase 41 Plan 14: Write-site'ы place_id, часть 1 Summary

Член группы с местом больше нельзя переместить по одному через `devices_update` (ни HTTP, ни Tauri-путь, ни для менеджера), повтор текущего места и правка других полей проходят, а мягкое удаление устройства чистит членство в той же транзакции.

## Tasks

| Task | Name | Commit |
|------|------|--------|
| 1 | Guard S1 в update, release S8 в delete_soft, комментарий S9, таблица-драйвер | d009d329 |
| 2 | HTTP-вариант S1/S8 и Tauri-путь | d18cea82 |

## Инвентарь write-site'ов devices.place_id (от серверных мутаций)

Сверка командой `grep -rnE "update_status_and_place_in_tx\(|update_full_in_tx\(|restore_from_snapshot_in_tx\(|UPDATE devices SET" crates/*/src` по всему серверному коду. Каждое попадание учтено:

| Сайт | Место в коде | Статус |
|------|--------------|--------|
| S1 `DeviceService::update` -> `update_in_tx` | device_service.rs | ЗАКРЫТ этим планом (guard) |
| S8 `DeviceService::delete_soft` | device_service.rs | ЗАКРЫТ этим планом (release) |
| S9 backfill места принтера при установке картриджа | cartridges_sqlite.rs ~646 | ЗАКРЫТ как «сознательно без guard'а»: комментарий + тест |
| S2 `PlaceService::move_subtree_contents` -> `update_status_and_place_in_tx` | place_service.rs ~717 | ОСТАВЛЕН, план 41-15 |
| S3-S7 `ActService`: `update_status_and_place_in_tx` (act_service.rs ~596, ~948), `update_full_in_tx` (~1756, ~2355, ~2443), `restore_from_snapshot_in_tx` (~1133, ~2285, ~3645) | act_service.rs | ОСТАВЛЕНЫ, план 41-16 |
| `group_place.rs::apply_group_place_to_device_in_tx` -> `update_status_and_place_in_tx` | group_place.rs:85 | свой путь группы (план 10), guard не нужен: это и есть «место задаёт группа» |
| `DeviceRepository::update` (devices_sqlite.rs ~728, не-tx вариант порта) | devices_sqlite.rs | в рабочем коде не вызывается (сервис идёт через `update_in_tx`); не трогали |
| S10 create / bulk / CSV | device_service.rs | без изменений: новое устройство членом группы быть не может |
| `UPDATE devices SET deleted_at_utc` (devices_sqlite.rs ~571/~803, report_service.rs тестовая фикстура) | — | место не меняют |

## Что сделано

- **S1.** В writer-замыкании `DeviceService::update` после чтения `before` и до `update_in_tx`: если патч несёт `place_id` и оно отличается от текущего, ищется `locked_group_for_device_in_tx`; при найденной группе — `Validation{field:"place_id"}` «Место задаётся группой «…». Выведите устройство из состава, чтобы переместить его отдельно.». Группа без места не находится запросом (D-21), поэтому запрет «спит» без отдельной ветки.
- **S8.** `delete_soft` после `delete_soft_in_tx` вызывает `release_device_in_tx` (пользователь `None`, как и прежний audit удаления). Место не меняется.
- **S9.** Русский комментарий над `UPDATE devices SET place_id ... WHERE place_id IS NULL` объясняет, почему guard не нужен; тест `s9_...` проверяет три случая: член группы с местом (место и `version` принтера не тронуты), контроль вне групп (backfill реально работает), D-21 (группа без места — принтер заполняется).
- **Долг плана 41-11 закрыт.** Ветка «членства не было» в `release_device_in_tx` покрыта тестом `s8_delete_soft_without_membership_writes_no_release` (удаление устройства вне групп: без ошибки, без `custom:group_member_released`). Ветка «членство было» покрыта `s8_delete_soft_releases_membership` и `s8_http_delete_releases_membership`.

## Verification

- `group_write_sites` 9 passed: 5 сервисных (таблица из 9 сценариев S1, manager, S8 x2, S9) и 4 второго транспорта (HTTP update, HTTP после вывода, Tauri-путь, HTTP delete). Существующие цели зелёные: `devices_crud` 19, `place_movements_write_sites_devices` 6, `cartridges_lifecycle` 36.
- `cargo clippy -p trackly-app --all-targets -- -D warnings` — чисто; `rustfmt --check` по трём изменённым файлам чисто. check-privacy в pre-commit — PASS (оба коммита); имена вымышленные.
- Мутации (якорь `new_place != before_place_id` встречается ровно 1 раз, файл восстановлен копией и сверен):
  1. условие -> `false`: красные `s1_table_service_update_guard`, `s1_table_manager_is_also_guarded`, `s1_tauri_path_...`, `s1_http_..._rejects_...`, `s1_http_..._after_release...` (5 из 9);
  2. условие -> `true` (блокирует всегда): красный `s1_table_service_update_guard` на сценарии «повтор текущего места вместе с переименованием» — различение «повтор / реальная смена» держится на фикстуре;
  3. вызов `release_device_in_tx` в `delete_soft` убран: красный `s8_delete_soft_releases_membership`.
- Невакуумность: у каждого отказа есть контрольный запрос того же маршрута, дающий 200 (устройство вне группы), и сравнение `place_id`/`version` до и после; в таблице есть сценарий, где место устройства расходится с местом группы, так что «повтор» и «смена на место группы» дают разные исходы.

## Deviations from Plan

**1. [Организационное] Откат при мутационной проверке.** Первая мутация была откачена `git checkout -- device_service.rs`, что стёрло и ещё не закоммиченные правки плана; файл восстановлен из копии, сделанной перед мутацией, и сверен (`grep`: якорь 1 раз, `release_device_in_tx` 1 раз). Результата не изменило, остальные мутации откатывались копией и `cmp`.

Прочих отклонений нет: план выполнен как написан.

## Для следующих планов

- План 41-15 (S2) и 41-16 (S3-S7): расширять `tests/group_write_sites.rs`; хелперы `seed_group`, `seed_device`, `http_env` уже там.
- Для актов важен вывод: guard S1 стоит только в `DeviceService::update`; прямые вызовы `update_*_in_tx` из актов и `place_service` guard НЕ проходят.
- План 41-24 (UI): серверный запрет и его сообщение готовы; блокировка PlacePicker в форме — только подсказка.
- Принтер-член группы с NULL-местом при установке картриджа получит место картриджа (S9 срабатывает по `place_id IS NULL`); при инварианте «место члена = место группы» такого нет.

## Known Stubs

None.

## Threat Flags

None. T-41-14-01 (обход через devices_update): guard в сервисе, тесты HTTP и Tauri-пути. T-41-14-02: guard читает членство в том же замыкании writer'а. T-41-14-03: release в delete_soft, тест состава. T-41-14-04: сравнение с текущим значением, тест различает.

## Self-Check: PASSED

- group_write_sites.rs, device_service.rs, cartridges_sqlite.rs на месте
- коммиты d009d329, d18cea82 существуют
