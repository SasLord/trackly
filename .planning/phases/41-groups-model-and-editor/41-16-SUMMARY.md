---
phase: 41-groups-model-and-editor
plan: 16
subsystem: api
tags: [rust, sqlite, groups, acts, write-sites, transactions, gate]
requires:
  - phase: 41-11
    provides: "services::group_membership::release_device_in_tx"
  - phase: 41-14
    provides: "tests/group_write_sites.rs, инвентарь S1/S8/S9"
  - phase: 41-15
    provides: "S2 закрыт, SubtreeStats с группами"
provides:
  - "group_membership::release_if_locked_device_in_tx: вывод из состава только если устройство сейчас член группы с местом (restore-пути)"
  - "ActService: явный release после каждого из 8 write-site'ов devices.place_id (S3-S7), D-22"
  - "tests/group_write_sites.rs: 11 сценариев актов + счётный гейт исходников (3 теста), всего 23"
affects: [41.2 подтверждение выноса, 41-24 форма устройства, 41-25 содержимое места]
tech-stack:
  added: []
  patterns:
    - "явный вызов helper'а на сайте, а не логика внутри devices_sqlite.rs: акты и массовый перенос хотят РАЗНОГО"
    - "счётный гейт по исходникам: реестр файл -> число вызовов, парность write-site/release в окне строк"
key-files:
  created: []
  modified:
    - crates/trackly-app/src/services/act_service.rs
    - crates/trackly-app/src/services/group_membership.rs
    - crates/trackly-app/tests/group_write_sites.rs
key-decisions:
  - "Прямые пути актов (create, update-added, do_return, update_return-added/edited) — безусловный release_device_in_tx; restore-пути (update-removed, update_return-removed, undo) — release_if_locked, иначе restore затёр бы «место принадлежит группе»"
  - "Членство при undo не восстанавливается; повторного вывода undo не пишет (членства уже нет)"
  - "release_if_locked использует locked_group_for_device_in_tx: группа без места (D-21) состав не теряет"
  - "Новое поле ActService не вводилось: SqliteGroupRepository — unit-struct, создаётся на сайте; undo-функция получает tx и audit_repo как раньше"
requirements-completed: [GRP-07]
completed: 2026-10-04
---

# Phase 41 Plan 16: Акты выводят устройство из состава, гейт write-site'ов Summary

Все 8 мест, где `ActService` пишет `devices.place_id`, теперь явно выводят устройство-член из группы в той же транзакции (с `audit_log custom:group_member_released {group_id, act_id}`), а счётный гейт по исходникам краснеет при любом новом write-site без кейса release.

## Tasks

| Task | Name | Commit |
|------|------|--------|
| 1 | release после 8 write-site'ов act_service.rs + helper release_if_locked | 1754dd36 |
| 2 | Сценарии S3-S7 (11 тестов) | 898cd695 |
| 3 | Счётный гейт исходников write-site'ов | 33158393 |

## Что сделано

- **Task 1.** Сразу после каждого из 8 вызовов `update_status_and_place_in_tx` / `update_full_in_tx` / `restore_from_snapshot_in_tx` в `act_service.rs` стоит явный вызов с комментарием «Phase 41 D-22». `act_id` — id акта в контексте: для create — только что созданный, для update/update_return — `payload.id`, для do_return — id акта возврата, для undo — id отменяемого акта (в каскаде handover->returns это id возврата). `devices_sqlite.rs` не менялся (`git diff` пуст). Undo-хелпер вызывается из трёх мест; его сигнатура не менялась.
- **Task 2.** Фикстуры различают до/после: у члена место (Склад А) отлично от места акта (Склад Б); для restore-путей устройство вступает в группу уже на месте акта, а снимок хранит третье место. Остальные члены группы остаются на месте, на каждом сценарии есть контроль «устройство вне групп — акт проходит как раньше, записей release нет». Сверх плана: сценарий спящей группы (D-21: restore состав не трогает) и undo акта возврата (ветка `ActType::Return`).
- **Task 3.** Три теста: самопроверка счётчика (комментарий, докстрока, хвостовой комментарий и определение `fn` не считаются), гейт 1 (в `act_service.rs` ровно 8 write-site'ов и 8 release-вызовов, и каждый write-site имеет СВОЙ helper нужного вида в окне 14 строк до следующего write-site'а), гейт 2 (скан `crates/*/src`: act_service 8, place_service 1, group_place 1; любой другой файл или иное число — паника «добавь кейс release в group_write_sites.rs»; скан < 50 файлов — тоже паника, чтобы гейт не позеленел вхолостую).

## Реестр write-site'ов devices.place_id (полный, от серверных мутаций)

Сверка: `grep -rnE "update_status_and_place_in_tx\(|update_full_in_tx\(|restore_from_snapshot_in_tx\(|UPDATE devices SET" crates/*/src` + гейт 2 (теперь держит три метода автоматически).

| Сайт | Место | Статус |
|------|-------|--------|
| S1 `DeviceService::update` | device_service.rs | ЗАКРЫТ планом 41-14 (guard: смена места члена группы с местом отклоняется) |
| S2 `PlaceService::move_subtree_contents` | place_service.rs | ЗАКРЫТ планом 41-15 (группа с местом едет целиком через `move_group_in_tx`, устройства без дублей журнала) |
| S3 `ActService::create` (handover) | act_service.rs | ЗАКРЫТ здесь: `release_device_in_tx`, тест `s3_handover_releases` |
| S4a `ActService::update` — добавленное устройство | act_service.rs | ЗАКРЫТ здесь: `release_device_in_tx`, `s4a_update_added_releases` |
| S4b `ActService::update` — убранное (restore) | act_service.rs | ЗАКРЫТ здесь: `release_if_locked`, `s4b_update_removed_releases_if_locked` + `s4b_..._keeps_membership_of_dormant_group` |
| S5 `ActService::do_return` | act_service.rs | ЗАКРЫТ здесь: `release_device_in_tx`, `s5_return_releases` |
| S6a `update_return` — добавленное | act_service.rs | ЗАКРЫТ здесь: `release_device_in_tx`, `s6_update_return_added_releases` |
| S6b `update_return` — изменённое (retained_with_change) | act_service.rs | ЗАКРЫТ здесь: `release_device_in_tx`, `s6_update_return_edited_releases` |
| S6c `update_return` — убранное (restore) | act_service.rs | ЗАКРЫТ здесь: `release_if_locked`, `s6_update_return_removed_releases_if_locked` |
| S7 `undo_device_mutations_for_act` (delete_soft handover/return, каскад) | act_service.rs | ЗАКРЫТ здесь: `release_if_locked`, `s7_undo_releases_if_locked`, `s7_undo_does_not_restore_membership`, `s7_undo_return_releases_if_locked` |
| S8 `DeviceService::delete_soft` | device_service.rs | ЗАКРЫТ планом 41-14 (`release_device_in_tx`) |
| S9 backfill места принтера при установке картриджа (`WHERE place_id IS NULL`) | cartridges_sqlite.rs | ЗАКРЫТ планом 41-14 как «сознательно без guard'а»: у члена группы с местом условие не срабатывает; комментарий + тест |
| S10 create / bulk / CSV устройств | device_service.rs | ВНЕ СКОУПА сознательно: новое устройство членом группы быть не может |
| `group_place.rs::apply_group_place_to_device_in_tx` | group_place.rs:85 | ВНЕ СКОУПА сознательно: это и есть «место задаёт группа» (план 10); учтён в реестре гейта (1 вызов) |
| `DeviceRepository::update` (не-tx вариант порта, devices_sqlite.rs) | devices_sqlite.rs | ВНЕ СКОУПА: в рабочем коде не вызывается, сервис идёт через `update_in_tx` |
| `UPDATE devices SET deleted_at_utc` и т.п. | devices_sqlite.rs, report_service.rs (тестовая фикстура) | ВНЕ СКОУПА: место не меняют |
| Определения трёх методов | devices_sqlite.rs | не вызовы; исключены из счёта гейта (`fn `) |

Все 8 write-site'ов актов — по одному на пункты S3-S7; каждый закреплён тестом и счётным гейтом.

## Verification

- `group_write_sites`: 23 passed (9 прежних + 11 сценариев актов + 3 гейта).
- Регресс актов, каждая цель отдельным вызовом, целиком: acts_crud 10, acts_returns 20, acts_update 17, acts_update_return 20, acts_undo 6, acts_clone_handover 12, acts_changed_place_ids 5, acts_place_snapshot 4, acts_e2e_smoke 4 — все зелёные.
- `cargo clippy -p trackly-app --all-targets -- -D warnings` чисто (после задач 1 и 3); `rustfmt --check` по трём изменённым файлам чисто; check-privacy PASS во всех трёх коммитах; данные вымышленные.
- Мутации (файлы восстановлены копией и сверены `cmp`):
  1. Якорь `update_full_in_tx(\n &tx,\n dev_id,\n` встречается ровно 1 раз (скрипт делает assert); вызов release после него снят — красным стал РОВНО `s6_update_return_edited_releases` (19 из 20 зелёные).
  2. Лишний `restore_from_snapshot_in_tx(` в блок-комментарии в act_service.rs — красные гейт 1 и гейт 2.
  3. `update_full_in_tx(` в device_service.rs — красный гейт 2 (гейт 1 зелёный).
  4. Снят release в update_return-edited — красный гейт 1 (парность), гейт 2 зелёный.
- Невакуумность: место члена до операции != месту акта, после — равно; для restore-путей место из снимка != места акта и != места группы; контрольные устройства вне групп имеют 0 записей release.

## Deviations from Plan

**1. [Организационное] Сверх плана добавлены два сценария:** спящая группа (D-21, restore не трогает состав) и undo акта возврата. Правок кода не потребовали.

**2. [Организационное] Парность в гейте.** Плановый гейт требовал счёт 8/8; добавлена проверка, что каждый write-site имеет СВОЙ release нужного вида в окне после него (иначе можно было переставить вызовы и сохранить числа).

Прочих отклонений нет: план выполнен как написан.

## Замечания для следующих планов

- Правка возврата с «изменённым» устройством (S6b) выводит из состава безусловно, как зафиксировано в плане, даже если место фактически не меняется (меняется только состояние). Для реального сценария это случай «член группы, которого уже вернули актом»; подтверждение выноса (фаза 41.2) может сузить его.
- Любой новый write-site `devices.place_id` в `crates/*/src` краснит `write_site_gate_no_unregistered_place_writers`.
- Код `release_*` не прячется в репозитории: вызывать на сайте.

## Known Stubs

None.

## Threat Flags

None. T-41-16-01 (обход запрета через акт): release на всех 8 сайтах, сценарии S3-S7. T-41-16-02: счётный гейт и скан исходников. T-41-16-03: `release_if_locked` на restore-путях, тесты S4b/S6c/S7. T-41-16-04: audit с group_id и act_id в той же транзакции, payload проверяется тестами.

## Self-Check: PASSED

- act_service.rs, group_membership.rs, group_write_sites.rs изменены; 41-16-SUMMARY.md создан
- коммиты 1754dd36, 898cd695, 33158393 существуют
