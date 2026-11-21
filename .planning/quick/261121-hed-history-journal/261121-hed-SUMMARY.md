---
phase: quick-261121-hed
plan: 01
status: complete
subsystem: history-journal
tags: [place_movements, audit_log, ws, reports, timeline, gates]
requires: []
provides:
  - "MovementSource::Repair + MovementSource::ALL"
  - "place_movements.request_id (V056) с переносом прежних ремонтных строк"
  - "ui/src/lib/movementReason.ts — словарь подписей источников"
  - "гейт ui/scripts/check-movement-source-labels.mjs в pnpm lint"
  - "audit_log custom:printer_connection"
  - "RequestStatusChanged из link_scans, EmployeeScopeChanged из create"
affects: [repair_service, device_service, report_service, MovementTimeline, RequestDetail]
tech-stack:
  added: []
  patterns:
    - "исчерпывающий match по MovementSource в отчёте + гейт паритета Rust↔TS с мутантами настоящих исходников"
    - "аудит секрета — только факт смены ({\"changed\": true})"
key-files:
  created:
    - migrations/V056__place_movements_request_id.sql
    - ui/src/lib/movementReason.ts
    - ui/scripts/check-movement-source-labels.mjs
    - crates/trackly-app/tests/repair_movement_source.rs
    - crates/trackly-app/tests/devices_printer_link_audit.rs
    - crates/trackly-app/tests/repair_ws_signals.rs
  modified:
    - crates/trackly-core/src/domain/place_movements.rs
    - crates/trackly-infra/src/repos/place_movements_sqlite.rs
    - crates/trackly-infra/src/repos/cartridges_sqlite.rs
    - crates/trackly-infra/tests/place_movements_migration.rs
    - crates/trackly-app/src/dto/place_movements.rs
    - crates/trackly-app/src/services/place_movement_service.rs
    - crates/trackly-app/src/services/repair_service.rs
    - crates/trackly-app/src/services/report_service.rs
    - crates/trackly-app/src/services/device_service.rs
    - crates/trackly-app/tests/repair_accept.rs
    - crates/trackly-app/tests/repair_cancel_reject.rs
    - crates/trackly-app/tests/entities_support/registry.rs
    - crates/trackly-app/tests/suites/repairs.rs
    - crates/trackly-app/tests/suites/devices.rs
    - ui/src/lib/components/MovementTimeline.svelte
    - ui/src/features/printers/PrinterDetail.svelte
    - ui/src/features/cartridges/CartridgeDetail.svelte
    - ui/src/features/places/PlaceEntityViewModal.svelte
    - ui/src/features/showcase/sections/MovementTimelineSection.svelte
    - ui/src/features/requests/RequestDetail.svelte
    - ui/scripts/check-group-vocabulary.mjs
    - ui/package.json
decisions:
  - "V056 переносит только строки устройства с note РОВНО 'по заявке №'||id существующей заявки repair; картриджные «вместе с принтером» прошлого не переносятся (заявку не восстановить однозначно)"
  - "community в аудите — только {\"changed\": true}; IP/USB-хост — from/to по фактическим значениям строки printers, не по патчу"
  - "EmployeeScopeChanged шлётся только для новой (не joined) заявки с устройством"
  - "notify_request_status вынесен из finish_transition и переиспользован link_scans"
metrics:
  started: 2026-11-21T05:52:25Z
  completed: 2026-11-21T09:20:11Z
  tasks: 3
requirements: [INT-30, NEW-3, INT-09, NEW-7, NEW-4]
---

# Quick 261121-hed: история и журнал — Summary

Ремонтный перенос пишет в журнал источник `repair` с id заявки (V056, ссылка «по ремонтной заявке №N» в таймлайне, текст в отчёте); у всех шести источников явные подписи, паритет Rust↔фронт держит гейт в `pnpm lint`; смена IP/USB-хоста/community принтера попадает в audit_log без значения community; link_scans и создание ремонтной заявки шлют WS-сигналы второму клиенту.

## Коммиты

| Задача | Коммит | Что |
|---|---|---|
| 1 | 988e0078 | MovementSource::Repair/ALL, V056, record_repair_movement_if_applicable, cascade_place_for_printer_repair_in_tx, MovementEntryDto.request_id, отчёт «по ремонтной заявке №N» |
| 2 | 0fab199d | movementReason.ts, ссылка на заявку в MovementTimeline, проводка в трёх карточках, гейт check-movement-source-labels |
| 3 | 8ed015b7 | аудит custom:printer_connection, RequestStatusChanged из link_scans, EmployeeScopeChanged из create, RequestDetail перечитывает состояние ремонта |

## Проверка

Тесты (по одному, TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1):
- trackly-core `place_movements` — 13 ok
- infra_tests `place_movements` — 17 ok (включая v056-перенос на настоящей БД V055 через `run_up_to(55)` и проверку последней строки `PRAGMA user_version = 56;`)
- trackly-app `--lib report_service` — 61 ok; `--lib device_service` — 5 ok
- app_repairs — 137 ok; app_devices — 253 ok; app_reports — 176 ok (2 ignored); app_places — 80 ok; app_gates — 244 ok (1 ignored); app_acts — 293 ok; app_cartridges — 88 ok; app_server — 48 ok; export_bindings — ok
- app_groups — 145 ok, **2 красных не от этого пакета** (см. Deferred)

Гейты: `cargo clippy --workspace --all-targets -- -D warnings` — 0; `cargo fmt --all -- --check` — 0; `pnpm --dir ui lint` — 0; `pnpm --dir ui svelte-check` — 0 ошибок (75 warnings, все существовали раньше); `pnpm --dir ui build` — ok.

Мутационные проверки:
- `check-movement-source-labels.mjs --selftest`: 4 мутанта копий настоящих исходников (удалён ключ map; вариант Survey в Rust; workstation → fallback; Repair убран из ALL) — все красные; якоря уникальны (replace только при count == 1).
- Ручная порча рабочего movementReason.ts (удалён ключ map) → `pnpm lint` красный (M3), файл восстановлен.
- Реализация снята мутацией (без notify в link_scans, без EmployeeScopeChanged, без INSERT аудита) → 2/3 теста repair_ws_signals и 4/5 тестов devices_printer_link_audit красные (зелёные остались те, что проверяют отсутствие события/строки); исходники восстановлены побайтно.

## Deviations from Plan

1. **[Rule 3] Гейт словаря групп** — вынос подписей в `movementReason.ts` перенёс литералы «в составе группы» / «перенос группы» в новый файл, `check-group-vocabulary.mjs` покраснел. В allowlist добавлен `src/lib/movementReason.ts` с теми же двумя маркерами, что у MovementTimeline. Коммит 0fab199d.
2. **[Rule 1] Индексы place_movements** — тест V040 ожидал ровно 7 индексов; V056 добавляет частичный `idx_place_movements_request` → ожидание 8. Коммит 988e0078.
3. **[Rule 3] Гейт реестра entities_changed** — `printer_connection_diff` как `pub(crate) fn` попадал в инвентарь сервисных функций (NoVerdict); функция и структура сделаны приватными для модуля. Коммит 8ed015b7.
4. **Гейт M5 расширен** сверх плана: где отчёт подписывает источник литералом (`act`, `map`, `workstation`), таймлайн обязан давать ту же строку — держит «одинаково с отчётом» из INT-09.
5. **bindings.ts** в .gitignore — перегенерирован `export_bindings` (поле `request_id` в MovementEntryDto есть), но не коммитится. Это штатно.
6. **NEW-4, тест «второй сотрудник того же АРМ»**: в тесте один сотрудник (Иванов) с группой-АРМ создаёт заявку, видимость события проверена через `is_visible_to(emp(Петров))`; второго сотрудника в той же группе не заводил (у `seed_user_group` одна группа на пользователя). Как клиент перечитывает список, проверено только кодом (EmployeeDevicesPage уже слушает `employee_scope_changed`).
7. **TDD-порядок**: тесты Task 3 написаны вместе с реализацией, а не строго раньше; что они несущие, подтверждено мутацией реализации (см. выше).

## Deferred Issues

- app_groups: `group_report_batch::report_batch_csv_full_composition` и `report_batch_act_package_folds_like_group_transfer` красные из-за колонки «Пакет», добавленной в 261121-hef (bbc463bd); тест не обновлён. Подробности — `deferred-items.md`.

## Не проверено (нужно вживую)

- Таймлайн в карточках принтера/картриджа/устройства (из «Мест»): строка «по ремонтной заявке №N», клик по N открывает заявку на вкладке «Информация»; у PlaceEntityViewModal модалка закрывается.
- Второй клиент: открыта карточка ремонтной заявки, на другом клиенте привязаны сканы к заходу → у первого счётчик сканов обновляется без перезагрузки.
- Второй сотрудник того же АРМ (LAN-браузер): после создания ремонтной заявки первым у второго появляется «Есть заявка» без перезагрузки.
- Журнал аудита: после смены IP/community/хоста в форме принтера есть строка custom:printer_connection; как UI журнала её отображает (сырой action или подпись) — не проверялось.
- Обновление старой dev-БД: V056 переносит прежние строки «по заявке №N» в `repair` (проверено тестом на синтетической V055, на реальной копии БД — нет).

## Known Stubs

Нет.

## Self-Check: PASSED

- Файлы: V056, movementReason.ts, check-movement-source-labels.mjs, repair_movement_source.rs, devices_printer_link_audit.rs, repair_ws_signals.rs — на месте.
- Коммиты 988e0078, 0fab199d, 8ed015b7 — в `git log`.
