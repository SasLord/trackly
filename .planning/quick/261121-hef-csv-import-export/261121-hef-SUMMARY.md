---
phase: quick-261121-hef
plan: 01
status: complete
subsystem: devices-csv, printers-admit, reports-movements
tags: [csv, import, export, snmp, number-templates, reports, batch]
requirements: [INT-21, N4, INT-32, INT-17, INT-13, INT-06, F18]
key-files:
  modified:
    - crates/trackly-app/src/dto/device.rs
    - crates/trackly-app/src/services/device_service.rs
    - crates/trackly-app/src/tauri_cmds/printers.rs
    - crates/trackly-app/src/dto/reports.rs
    - crates/trackly-app/src/services/report_service.rs
    - crates/trackly-app/src/tauri_cmds/reports.rs
    - ui/src/features/devices/DeviceImportCsvModal.svelte
    - ui/src/features/reports/ReportTable.svelte
    - crates/trackly-app/tests/devices_csv_import.rs
    - crates/trackly-app/tests/devices_csv_export.rs
    - crates/trackly-app/tests/printers_admit.rs
    - crates/trackly-app/tests/report_movements_truncation.rs
    - crates/trackly-app/tests/report_csv_export.rs
    - crates/trackly-app/tests/html_report_render.rs
    - crates/trackly-app/tests/html_header_parity.rs
decisions:
  - "INT-21: импорт не блокируется; нераспознанные тип/статус приводятся к значению по умолчанию и сообщаются (preview.label_checks по колонкам, report.warnings + coerced_rows по вставленным строкам)"
  - "INT-32: 4 новые колонки экспорта (Группа, Роль в группе, Происхождение, USB-хост) — в конце, прежние 10 не сдвинуты; авто-маппинг импорта их не знает"
  - "INT-13: приёмка берёт следующий номер запомненного шаблона PrinterCreate на каждой итерации; ошибка/переполнение/нет шаблона — пусто; Conflict — один повтор без номера"
  - "F18: усечение пакет не перестраивает — batch_shown с сервера, «показано K из N» одинаково на экране, в CSV и печати"
completed: 2026-11-21
---

# Quick 261121-hef: CSV, печать и приёмка — итог

Закрыты четыре находки пакета B аудита v1.4. CSV-импорт больше не подставляет молча значение вместо опечатки в типе или статусе. CSV-экспорт устройств показывает группу, роль, происхождение и USB-хост, как экран. SNMP-приёмка нумерует принтер по шаблону «Новый принтер». CSV и печать «Перемещений» содержат колонку «Пакет», а частично показанный пакет везде подписан «показано K из N».

## Коммиты

| Задача | Коммит | Содержание |
|---|---|---|
| 1 INT-21/N4 | e0714be2 | `LabelMatch` (Empty/Matched/Unknown), `RowWarning`/`ColumnLabelCheck`/`LabelSample`, `label_checks` в предпросмотре, `warnings`/`coerced_rows` в итоге, модалка: шаг 3 и шаг 4 |
| 2 INT-32/INT-17 | d4f2d025 | экспорт: `enrich_devices` пачками по 500, `usb_host_labels` пачкой, 14 колонок |
| 2 INT-13 | 1cb8e6ed | `next_printer_create_number` в `build_printers_admit`, повтор без номера при Conflict |
| 3 INT-06/F18 | bbc463bd | `ReportRow.batch_shown`, `batch_cell`, колонка «Пакет» в `columns_for`/`column_labels_for`, ReportTable «показано K из N» |
| 3 (гейт) | ed19b85c | склонение массивом форм — ложное срабатывание R4 в `device_type_label_gate` |

## Тесты (TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1, по одному процессу cargo за раз)

- `app_devices` полностью: 248 прошло, 0 упало (в том числе 6 новых `hef_import_*`/`hef_preview_*`, 3 новых `hef_export_*`)
- `app_cartridges` полностью: 88 прошло, 0 упало (в том числе `a4`/`a5` приёмки)
- `app_reports` полностью: 176 прошло, 0 упало, 2 пропущены (`#[ignore]`, как и раньше)
- `app_gates` полностью: 244 прошло, 0 упало, 1 пропущен (после ed19b85c; до него 5 красных в `device_type_label_gate`)
- `app_server` полностью: 48 прошло, 0 упало
- `--lib` полностью: 392 прошло, 0 упало (в том числе 5 новых `batch_cell_*`)
- `export_bindings`: 1 прошёл

Гейты: `cargo fmt --all --check` — чисто; `cargo clippy --workspace --all-targets -- -D warnings` — чисто; `pnpm --dir ui lint` — exit 0; `pnpm --dir ui svelte-check` — 0 ошибок, 75 предупреждений (столько же было до изменений, в изменённых файлах предупреждений нет).

## Отклонения от плана

1. **[Rule 3] `ui/src/bindings.ts` не коммитится.** Файл в `.gitignore` и генерируется (`prebuild` запускает `export_bindings`). Его перегенерировали, и он содержит `RowWarning`, `ColumnLabelCheck`, `label_checks`, `coerced_rows`, `batch_shown`, но в коммиты не попал.
2. **[Rule 3] Тесты вне `files_modified`.** `html_report_render.rs` и `html_header_parity.rs` строят `ReportRow` литералом, поэтому туда добавлено `batch_shown: None`. Этого требует компилятор.
3. **[Rule 1] Гейт `device_type_label_gate` R4.** `plural_devices` вида `if … != 11 { "устройство" }` гейт прочитал как карту id типа -> подпись. Формы переписаны массивом с индексом, как в экранном `pluralizeRu(n, forms)`. Освобождение в EXEMPT не добавлялось, гейт не ослаблен.
4. **Тест round-trip использует `auto_map` из `devices_csv_import.rs`.** Видимость повышена до `pub(crate)`, второй копии таблицы заголовков нет. Сторож `auto_map_mirrors_modal_aliases_and_stays_no_broader` по-прежнему её держит.
5. **INT-13: шаблон из `ctx.number_templates`,** а не из `ctx.devices.number_templates`. Это тот же сервис на той же БД, а поле `AppCtx` публичное. Ошибка чтения шаблона не роняет приёмку: номер остаётся пустым и пишется `tracing::warn`.
6. **Шаг 3 модалки** под списком образцов показывает строку «… и ещё M», если нераспознанных значений больше 20. Это небольшое дополнение к плану.

## Что не проверено

- **Живая UI-проверка не проводилась.** Ни десктоп (WKWebView), ни браузер не открывались. Предупреждения на шагах 3 и 4 модалки импорта и подпись «(показано K из N устройств)» в ReportTable проверены только `svelte-check`, eslint и prettier. Рантайм рун эти гейты не видят. Импорт CSV в модалке работает только на десктопе (выбор файла через Tauri-диалог).
- **Повтор без номера при `Conflict` в приёмке тестом не покрыт.** Гонку, при которой номер занимают между peek и вставкой, в сервисном тесте не воспроизвести без инъекции: peek считает следующий номер после максимального занятого.
- **Печать проверена только на уровне HTML.** `build_reports_export_pdf` содержит «Пакет» и «показано 1 из 3 устройств». Настоящий вывод на печать (Paged.js) не открывался.
- Сплошной `cargo test --workspace` не запускался, прогнаны только suite-бинарники, перечисленные выше.

## Known Stubs

Нет.

## Threat Flags

Нет: новых точек входа нет. Новые запросы (`usb_host_labels`, обогащение экспорта) передают значения только параметрами. Новые ячейки CSV идут через `csv_safe`, а для «Пакета» через уже существующий `export_csv` отчётов (T-hef-01..04 закрыты кодом).

## Self-Check: PASSED

- Коммиты e0714be2, d4f2d025, 1cb8e6ed, bbc463bd, ed19b85c есть в `git log`.
- `crates/trackly-app/src/dto/device.rs` содержит `pub struct RowWarning`; `crates/trackly-app/src/dto/reports.rs` содержит `batch_shown`; `printers.rs` содержит `PrinterCreate`; `tauri_cmds/reports.rs` содержит `"batch"`; модалка содержит `label_checks`.
