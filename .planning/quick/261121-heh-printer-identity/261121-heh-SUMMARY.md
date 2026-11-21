---
phase: quick-261121-heh
plan: 01
status: complete
subsystem: devices / printers (признак «это принтер»)
tags: [INT-31, NEW-5, gate, refactor]
requires: []
provides:
  - "trackly-core: DEVICE_TYPE_ID, PRINTER_TYPE_ID, is_printer_type, #[macro_export] printer_type_id_sql!"
  - "ui/src/lib/deviceTypeIds.ts: DEVICE_TYPE_ID, PRINTER_TYPE_ID, isPrinterType"
  - "гейты: printer_identity_gate (app_gates, P1–P5), check-printer-identity.mjs (pnpm lint, J1–J5)"
affects: [request_printer_options, employee_devices_list, place contents, group printers, cartridges, number templates, SNMP admit]
tech-stack:
  added: []
  patterns:
    - "SQL-константа через concat!(\"… = \", trackly_core::printer_type_id_sql!(), \" …\"); в format!-строках — именованный аргумент с PRINTER_TYPE_ID"
key-files:
  created:
    - crates/trackly-app/tests/printer_identity_gate.rs
    - ui/src/lib/deviceTypeIds.ts
    - ui/scripts/check-printer-identity.mjs
  modified:
    - crates/trackly-core/src/domain/devices.rs
    - crates/trackly-app/src/services/{device_service,request_service,employee_device_service,cartridge_service,number_template_service,group_service}.rs
    - crates/trackly-app/src/tauri_cmds/printers.rs
    - crates/trackly-infra/src/repos/{places_sqlite,groups_sqlite,cartridges_sqlite}.rs
    - crates/trackly-app/tests/{request_printer_options,employee_devices_list,places_contents,groups_values_card,group_report_batch}.rs
    - crates/trackly-app/tests/suites/gates.rs
    - crates/trackly-infra/tests/groups_repo.rs
    - ui/src/features/{devices/PrinterMark,devices/DeviceFormBody,devices/DeviceFormModal,devices/DeviceRowMenu,groups/GroupsPage,groups/GroupPrintersList,printers/PrintersPage}.svelte
    - ui/package.json
decisions:
  - "Канон признака принтера — devices.type_id = PRINTER_TYPE_ID (id сида V001), не подпись типа и не строка printers"
  - "Дрейф старых БД (строка printers при type_id≠2) принят без миграции: такое устройство больше не принтер и в списке сотрудника/группах"
  - "concat! с пользовательским macro_rules собирается (проверено rustc) — использован везде, где SQL статический; в format!-строках — именованный аргумент"
metrics:
  completed: 2026-11-21
  tasks: 3 (+ шаг 0)
  commits: 5 кодовых
---

# Quick 261121-heh: единый признак «это принтер» (INT-31, NEW-5) Summary

Признак принтера определён один раз — `PRINTER_TYPE_ID` (id сида V001) в trackly-core с SQL-макросом
`printer_type_id_sql!()` и фронтовым зеркалом `$lib/deviceTypeIds`; все 15 бэкенд-мест и 7 фронтовых
копий переведены. `is_printer` у сотрудника и варианты принтеров в заявке теперь считаются по одному
правилу (NEW-5). Переименование типа в справочнике больше ничего не ломает. Оба гейта проверены мутацией.

## Шаг 0 — app_groups после 261121-hef

`group_report_batch::report_batch_csv_full_composition` и `report_batch_act_package_folds_like_group_transfer`
ждали CSV без колонки «Пакет». Колонку добавили намеренно в bbc463bd (INT-06/F18: `batch_cell`,
«Пакет» — последняя в `columns_for`, сквозной тест `csv_export_movements_has_batch_column_for_group_transfer`),
значит устарели ожидания, а не код. Что поменял: заголовок `…;Причина;Пакет`, причину проверяю по
колонке 6 (а не `ends_with`), плюс новый ассерт — у каждой строки пакета ячейка «Пакет» начинается с
«Перенос группы «…»». app_groups 147/147. Коммит `21c0df3a`.

## Инвентарь: место, как было, как стало

| # | Место (HEAD) | Было | Стало |
|---|---|---|---|
| B1 | request_service.rs `request_printer_options` | `d.type_id = (SELECT id FROM device_types WHERE name = 'Принтер')` | `concat!(… d.type_id = ", printer_type_id_sql!(), " …)`; комментарий WR-04 переписан |
| B2 | places_sqlite.rs ветка group, `printer_member_count` | подзапрос по имени | `{printer_type_id}` (локальная `PRINTER_TYPE_ID`, format!) |
| B3 | places_sqlite.rs ветка device (`!=`) | подзапрос по имени | `{printer_type_id}` |
| B4 | places_sqlite.rs ветка printer | подзапрос по имени | `{printer_type_id}` |
| B5 | places_sqlite.rs `list_group_members_impl` CASE kind | подзапрос по имени | `concat!` + макрос; doc поправлен |
| B6 | employee_device_service.rs `is_printer` | `EXISTS(SELECT 1 FROM printers …)` | `(d.type_id = <макрос>)` — **NEW-5** |
| B7 | groups_sqlite.rs `member_printers_for_group` | `JOIN printers p` как фильтр | `AND d.type_id = <макрос>`, JOIN убран; doc переписан |
| B8 | groups_sqlite.rs `live_printer_device_ids` | `JOIN printers` как фильтр | `d.type_id = <макрос>`, JOIN убран; doc переписан |
| B9 | cartridges_sqlite.rs `compatible_model_aggregates` | `d.type_id = 2` | `concat!` + макрос; комментарий поправлен |
| B10 | cartridges_sqlite.rs count в `list` | `d.type_id = 2` | `concat!` + макрос |
| B11 | cartridges_sqlite.rs select в `list` | `d.type_id = 2` | `{printer_type_id}` (именованный аргумент format!) |
| B12 | cartridge_service.rs `suggest_compat_printer` | `type_id = 2` | `concat!` + макрос; doc поправлен |
| B13 | number_template_service.rs kind | `if type_id == 2` | `is_printer_type(type_id)` |
| B14 | tauri_cmds/printers.rs SNMP-приёмка | `type_id: 2` | `PRINTER_TYPE_ID`; комментарии поправлены |
| B15 | device_service.rs ассоц. `DEVICE_TYPE_ID`/`PRINTER_TYPE_ID` | локальная копия | удалены, импорт из trackly-core (употребления `Self::…`/`DeviceService::…` переписаны) |
| B16 (новое) | group_service.rs `const DEVICE_TYPE_ID: i64 = 1` | локальная копия (появилась после плана) | импорт из trackly-core |
| F1 | PrinterMark.svelte | `const PRINTER_TYPE_ID = 2` | `isPrinterType(typeId)`; серверный `isPrinter` по-прежнему приоритетнее |
| F2 | DeviceFormBody.svelte | локальная const | `import { PRINTER_TYPE_ID }` |
| F3 | DeviceFormModal.svelte | две локальные const | `import { DEVICE_TYPE_ID, PRINTER_TYPE_ID }` |
| F4 | DeviceRowMenu.svelte | `PRINTER_DEVICE_TYPE_ID = 2` | `import { PRINTER_TYPE_ID as PRINTER_DEVICE_TYPE_ID }` (в `<script module>`, имя держат якоря check-device-row-menu) |
| F5 | GroupsPage.svelte | локальная const | импорт |
| F6 | GroupPrintersList.svelte | локальная const | импорт; якорь `type_id: PRINTER_TYPE_ID` не тронут |
| F7 | PrintersPage.svelte | const внутри функции | импорт на уровне модуля |

После правок `grep 'type_id = 2|type_id == 2|type_id: 2|name = 'Принтер''` по crates/*/src находит
только тестовые модули (`#[cfg(test)]` в cartridges_sqlite.rs) и канон. На фронте `TYPE_ID\s*=\s*2` —
только deviceTypeIds.ts.

## Пробы TDD (RED, потом GREEN)

RED-коммит `d2e2cbda`, все три пробы до фикса падали так, как ожидалось:
- `request_printer_options::printer_options_survive_printer_type_rename`: `left: [] right: ["Принтер А", "Принтер Б"]`
- `employee_devices_list::employee_is_printer_matches_request_printer_options`: `is_printer «Принтер Б без строки»: left: false right: true`
- `places_contents::place_printer_identity_survives_printer_type_rename`: `ids_of(printer) left: [] right: [3]`

После GREEN (`0c1ba0bb`) все три зелёные.
Unit-тесты в trackly-core: `printer_type_id_sql_matches_const`, `is_printer_type_only_for_printer_id`,
`constants_match_v001_seed` (сид читается через include_str! из migrations/V001).

## Гейт printer_identity_gate (Rust, app_gates)

Правила P1–P5 и реестр `PRINTER_DATA_READS` (P3, точные числа после Task 1):
dashboard_service 3 (SNMP-счётчики/алерты), device_service 2 (USB-хост и сетевые поля),
employee_device_service 1 (тонер), groups_sqlite 1 (USB-хост группы), printers_sqlite 4 (свой
репозиторий), repairs_sqlite 2 (USB-хост ремонта). 9 тестов: по фикстуре на каждое правило, законные
формы, реестр сам себя инвалидирует (лишнее, пропавшее, чужой файл), новый файл, мутации реальных
cartridge_service.rs (P1), request_service.rs (P2), employee_device_service.rs (P3, «найдено 2»),
канона (P5). Якоря мутаций — регэкспы, у каждого проверяется ровно одно вхождение.

**Ручная мутация реального исходника** (в SQL B12 cartridge_service.rs вернул `type_id = 2`):
```
crates/trackly-app/src/services/cartridge_service.rs:1204: [P1] `type_id = 2`
test result: FAILED. 0 passed; 1 failed   (mut=101)
-> git checkout -- cartridge_service.rs
test result: ok. 9 passed; 0 failed      (back=0)
```
**Прогон гейта по дереву ДО фикса** (`git archive d2e2cbda`, `PRINTER_IDENTITY_GATE_ROOT`): красный по
всем классам. P1 cartridge_service:1204, number_template_service:794, printers.rs:185,
cartridges_sqlite:429/1896/1926; P2 request_service:305, places_sqlite:379/408/472 (P2 считается по
одному на литерал); P4 device_service:753; P3 employee_device_service (2 вместо 1), groups_sqlite
(3 вместо 1).

## Гейт check-printer-identity.mjs (JS, pnpm lint)

J1–J5 и E; `--selftest` прогоняет 12 кейсов: мутанты по реальным PrinterMark (J1), PrintersPage (J2) и
deviceTypeIds.ts (J4), отсутствие Rust-канона и другое значение в нём (J4), синтетика для J2/J3/J5/E,
законные формы. Полный прогон: 283 файла, 7 импортёров (порог 7).

**Ручная мутация** (в GroupsPage.svelte импорт заменён на `const PRINTER_TYPE_ID = 2;`):
```
[J1] src/features/groups/GroupsPage.svelte:20 — локальная копия id типа принтера («const PRINTER_TYPE_ID = 2»)
[J5] модуль $lib/deviceTypeIds импортируют 6 файлов, нужно не меньше 7
FAIL — 2 нарушений (mut=1)
-> откат
PASS — 0 нарушений (283 файлов, 7 импортёров)
```

## Правки фикстур (инвариант «принтер = type_id 2 + строка printers»)

- `crates/trackly-app/tests/groups_values_card.rs::seed_printer`: после вставки устройства
  `UPDATE devices SET type_id = 2`. До правки краснели 7 тестов `card_*` в app_groups.
- `crates/trackly-infra/tests/groups_repo.rs::insert_printer`: то же самое. До правки краснели
  `member_printers_follow_membership_including_nested_group` и `usb_printers_carry_host_name_and_inventory`.

Ни один продовый признак не ослаблен, ни один ассерт не удалён. Остальные фикстуры из раздела «Опасность
фикстур» (employee_devices_list, devices_container_printers, groups_anchor, request_printer_scope,
places_group_members, devices_group_children, repairs_repo) остались зелёными без правок.

## Следствие для старых БД (принято, без миграции)

В БД, созданных до quick 260820-rdj, возможен дрейф: строка `printers` есть, а `type_id ≠ 2`. Такое
устройство больше не считается принтером в списке сотрудника (`is_printer`), среди принтеров-членов
группы и в `live_printer_device_ids`. В заявке, картриджах и местах оно не считалось принтером и раньше.
Удалять такие строки `printers` миграцией нельзя: каскадом уйдут показания и ip. Тест-проба
`employee_is_printer_matches_request_printer_options` закрепляет это поведение.

## Тесты по бинарникам (все с TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1, по одному)

| Бинарник | Результат |
|---|---|
| trackly-core (unit + tests) | 185 + 1 + 3, ok |
| trackly-app --lib | 398 ok |
| trackly-infra (lib + infra_tests) | 176 + 272 ok (2 ignored) |
| app_server | 49 ok |
| app_repairs | 138 ok |
| app_places | 81 ok |
| app_cartridges | 88 ok |
| app_reports | 176 ok (2 ignored) |
| app_groups | 147 ok |
| app_devices | 253 ok |
| app_gates | 253 ok (1 ignored), включая suite_wiring, device_type_label_gate и новый printer_identity_gate (9) |
| app_acts | 293 ok |
| app_auth | 138 ok |
| app_attachments | 76 ok |
| app_chat | 41 ok |

## Гейты

`cargo fmt --all --check` 0; `cargo clippy --workspace --all-targets -- -D warnings` 0;
`pnpm --dir ui lint` 0 (в цепочке есть check-printer-identity: selftest и полный прогон);
`pnpm --dir ui svelte-check` 0 ошибок (75 предупреждений; в затронутых файлах только
`state_referenced_locally` на `target`/`presetPlaceId` в DeviceFormBody, к этим правкам отношения не имеют).
`pnpm --dir ui build` 0, ui/dist пересобран.

## Отклонения от плана

1. **[Rule 3] Шаг 0** — по поручению оркестратора: обновлены ожидания group_report_batch под колонку «Пакет» (`21c0df3a`).
2. **[Rule 2] B16 — group_service.rs `const DEVICE_TYPE_ID: i64 = 1`**. Локальная копия появилась уже после
   того, как писался план; заменена импортом из trackly-core (класс K).
3. **[Rule 3] B11 и B2–B4 собираются через `format!`, а не `concat!`.** Внутри `format!` нельзя
   неявно захватывать переменные, если строка формата пришла из `concat!`. Поэтому там именованный
   аргумент или локальная `printer_type_id = PRINTER_TYPE_ID`. Везде, где SQL статический, — `concat!`
   с макросом (`concat!` с пользовательским `macro_rules` собирается, проверил отдельно в rustc).
4. **Номера строк и комментарии плана** (tauri_cmds/printers.rs 75/141, device_service 664) сдвинулись
   после пакетов hef/hed. Места нашёл заново от HEAD, комментарии переписаны по месту.
5. **Мутант J1 в selftest для PrinterMark** — импорт оставлен, а константа добавлена рядом. Если
   импорт заменить, импортёров станет 6 и вместе с J1 сработает J5, а план ждёт ровно J1. Ручная
   мутация GroupsPage как раз заменяет импорт, поэтому показывает J1 и J5: это ожидаемо.
6. **DeviceRowMenu**: импорт оказался в `<script module>`, потому что это первый блок импортов файла.
   Импорты из module-блока видны в instance-блоке; svelte-check и check-device-row-menu зелёные.
7. **Процесс**: при откате ручной JS-мутации `git checkout --` вернул GroupsPage.svelte к HEAD и стёр
   несохранённую правку Task 3. Восстановил её из копии в scratchpad до коммита, итоговый дифф
   проверен. Перед мутацией нужно было сначала закоммитить или откатывать точечно.

## Что не проверено

- Живое приложение (WKWebView / Windows) не запускалось. Маркер принтера, предзаполнение принтера в
  RequestFormModal и раздел «Принтеры» после переезда констант проверены только компиляцией, гейтами
  и интеграционными тестами бэкенда (NEW-5 на уровне данных).
- Маленькие бинарники (health_smoke, export_bindings, templates_status, template_edit, phase06_stubs,
  attachments_heic) не запускались: изменения их не затрагивают (DTO не менялись).
- Сплошной ci-full не запускался; прогнаны все перечисленные suite по одному.

## Known Stubs

Нет.

## Self-Check: PASSED

- FOUND: crates/trackly-app/tests/printer_identity_gate.rs, ui/src/lib/deviceTypeIds.ts, ui/scripts/check-printer-identity.mjs
- FOUND commits: 21c0df3a, d2e2cbda, 0c1ba0bb, 3e46db49, ae277b8e
