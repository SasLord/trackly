---
phase: quick-261121-hef
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - crates/trackly-app/src/services/device_service.rs
  - crates/trackly-app/src/dto/device.rs
  - crates/trackly-app/src/tauri_cmds/printers.rs
  - crates/trackly-app/src/dto/reports.rs
  - crates/trackly-app/src/services/report_service.rs
  - crates/trackly-app/src/tauri_cmds/reports.rs
  - ui/src/bindings.ts
  - ui/src/features/devices/DeviceImportCsvModal.svelte
  - ui/src/features/reports/ReportTable.svelte
  - crates/trackly-app/tests/devices_csv_import.rs
  - crates/trackly-app/tests/devices_csv_export.rs
  - crates/trackly-app/tests/printers_admit.rs
  - crates/trackly-app/tests/report_movements_truncation.rs
  - crates/trackly-app/tests/report_csv_export.rs
autonomous: true
requirements: [INT-21, N4, INT-32, INT-17, INT-13, INT-06, F18]

must_haves:
  truths:
    - "INT-21/N4: строка CSV с опечатанной подписью типа или статуса импортируется (импорт не блокируется), но итог импорта содержит предупреждение с номером строки, полем, исходным значением и применённым значением, плюс счётчик таких строк; пустая ячейка предупреждением не считается"
    - "INT-21/N4: до импорта (шаг сопоставления колонок) модалка показывает, сколько значений колонки, сопоставленной с «Тип»/«Статус», сервер не распознал, с примерами «строка N: «значение»» — распознавание делает сервер, в JS логики сопоставления подписей нет"
    - "INT-32/INT-17: CSV-экспорт устройств несёт колонки «Группа», «Роль в группе», «Происхождение», «USB-хост» после прежних 10 (их порядок и позиции не меняются)"
    - "INT-32: файл экспорта, загруженный обратно в импорт с авто-сопоставлением, импортируется без ошибок строк и без предупреждений; новые колонки игнорируются"
    - "INT-13: SNMP-приёмка принтера при запомненном шаблоне контекста «Новый принтер» присваивает устройству следующий номер по этому шаблону (тот же, что подставило бы окно создания принтера); без запомненного шаблона номер пуст, как в окне создания"
    - "INT-06/F18: CSV и печать отчёта «Перемещения» содержат колонку «Пакет»: у строк переноса группы — имя группы и число устройств пакета, у прочих — пусто"
    - "F18: если в выборку попала только часть устройств пакета (усечение или фильтр), и экран, и CSV, и печать пишут «показано K из N», а не голое N"
  artifacts:
    - path: "crates/trackly-app/src/dto/device.rs"
      provides: "CsvImportReport.warnings / coerced_rows, CsvImportPreviewResponse.label_checks"
      contains: "pub struct RowWarning"
    - path: "crates/trackly-app/src/dto/reports.rs"
      provides: "ReportRow.batch_shown"
      contains: "batch_shown"
    - path: "ui/src/bindings.ts"
      provides: "регенерированные типы (RowWarning, ColumnLabelCheck, batch_shown)"
      contains: "batch_shown"
  key_links:
    - from: "crates/trackly-app/src/tauri_cmds/printers.rs build_printers_admit"
      to: "NumberTemplateService::get_context + peek_next_for_context(TemplateContextDto::PrinterCreate)"
      via: "inventory_no в DeviceNew до create_with_printer_block"
      pattern: "PrinterCreate"
    - from: "crates/trackly-app/src/tauri_cmds/reports.rs columns_for(\"movements\")"
      to: "report_service.rs row_field \"batch\""
      via: "ключ колонки, индексно согласован с меткой «Пакет»"
      pattern: "\"batch\""
    - from: "ui/src/features/devices/DeviceImportCsvModal.svelte"
      to: "CsvImportPreviewResponse.label_checks / CsvImportReport.warnings"
      via: "шаг 3 и шаг 4 модалки"
      pattern: "label_checks"
---

<objective>
Закрыть четыре находки аудита вехи v1.4 из пакета B «CSV, печать и приёмка»:
INT-21 (+N4) молчаливое приведение типа/статуса при CSV-импорте; INT-32/INT-17 CSV-экспорт
устройств расходится с экраном; INT-13 SNMP-приёмка минует шаблоны номеров; INT-06 (+F18)
пакет переноса группы: экран против печати и CSV.

Purpose: импорт/экспорт/печать перестают молча терять или искажать данные, приёмка нумерует
принтеры так же, как ручное создание.
Output: изменённые сервисы, DTO, регенерированный bindings.ts, две Svelte-правки, тесты.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md
@./CLAUDE.md
@.planning/v1.4-MILESTONE-AUDIT.md

Параллельно в том же дне созданы quick-задачи 261121-heh-printer-identity и
261121-hei-dead-api-reports — они могут трогать printers.rs / reports.rs / report_service.rs.
Перед началом: `git log --oneline -10` и `git status`; если соседняя задача уже закоммитила
правки в этих файлах — работать поверх HEAD, не откатывать чужое.

<interfaces>
<!-- Извлечено из кода, исследовать заново не нужно. -->

device_service.rs:
- `struct StatusLookup { rows: Vec<(i64 id, String name, String code)>, default_id }`,
  `fn id_for_label(&self, Option<&str>) -> i64` (~стр. 131-153): пусто/неизвестно -> default_id;
  псевдоним «На ремонте» (LEGACY_REPAIR_LABEL_LOWER -> code STATUS_CODE_LEGACY_REPAIR);
  `fn name_for_id(i64) -> String`; `async fn load(Arc<ReaderPool>)`.
- `struct TypeLookup { rows: Vec<(i64, String)> }`, `fn id_for_label(Option<&str>) -> i64`
  (~стр. 210-235): пусто/неизвестно -> `DeviceService::DEVICE_TYPE_ID`; «Расходник»
  (LEGACY_EXPORT_PRINTER_LABEL_LOWER) -> PRINTER_TYPE_ID; `name_for_id`.
- `pub async fn import_csv_preview(&self, bytes: Vec<u8>) -> Result<CsvImportPreviewResponse>`
  — заголовки/строки в `ImportSession { headers, all_rows, .. }`, маппинга на этом шаге нет.
- `pub async fn import_csv_commit(&self, token: String, mapping: HashMap<String,String>) -> Result<CsvImportReport>`
  — построчно `Self::build_device_new_from_row(row, &header_idx, &mapping, &statuses, &types) -> Result<(DeviceNew, Option<String>), String>`;
  row_index 1-based; ошибки строк в `report.failed: Vec<RowError{row_index, error_code, error_message}>`.
- `pub async fn export_csv_with_limit(&self, filter, max_rows) -> Result<String>` (~стр. 2349):
  `repo.list_all(...)` без обогащения; заголовки 10 штук (стр. ~2407); `csv_safe(...)` на каждую ячейку; BOM + `;`.
- `fn enrich_devices<'a>(conn, impl IntoIterator<Item=&'a mut DeviceDto>) -> Result<(), AppError>`
  — заполняет `group_id/group_name`, `anchor_group_id/anchor_group_name/anchor_member_count`,
  `origin_group_id/origin_group_name` пачкой `IN (?1..?N)` (N = число id, параметры SQLite).
- `pub async fn create_with_printer_block(&self, new: DeviceNew, printer: Option<PrinterCreateBlockDto>) -> Result<DeviceSaveOutcome>`
  — при непустом `inventory_no` выполняет `reject_if_occupied` (Conflict), шлёт number-space broadcast.

dto/device.rs (~стр. 490-560): `CsvImportPreviewResponse { token, encoding, delimiter, headers, preview_rows, total_rows: u64 (#[specta(type=u32)]), had_replacements }`,
`CsvImportReport { inserted: u64, failed: Vec<RowError>, affected_place_ids: Vec<i64> (#[specta(type=Vec<i32>)]) }`,
`RowError { row_index: u64, error_code, error_message }`. Все derive `Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type`.

number_template_service.rs:
- `pub async fn get_context(&self, context: TemplateContextDto) -> Result<Option<i64>, AppError>` (стр. ~521) — запомненный шаблон контекста.
- `pub async fn peek_next_for_context(&self, template_id: i64, context: TemplateContextDto) -> Result<NextNumberDto, AppError>`.
- `NextNumberDto { rendered: String, alt_rendered: Option<String>, has_gap: bool, overflowed: bool }`.
- `DeviceService.number_templates: Arc<NumberTemplateService>` (pub(crate), доступно из tauri_cmds).

tauri_cmds/printers.rs `build_printers_admit(ctx, caller, selected_ips, community)` (~стр. 70-185):
цикл по IP -> проверка дубля IP -> probe -> `ctx.devices.create_with_printer_block(DeviceNew{ type_id: 2, inventory_no: None, place_id: None, status_id: 1, .. }, Some(PrinterCreateBlockDto{ip_address, community, usb_host_device_id: None}))`.

dto/reports.rs ReportRow: `batch_id: Option<String>`, `batch_role: Option<String>` ("header" | "member"),
`batch_size: Option<i64>` (#[specta(type = Option<i32>)], число УСТРОЙСТВ пакета по всей БД),
`batch_label: Option<String>`, `entity_type_label: Option<String>` ("Устройство"/"Картридж"/"Группа").

report_service.rs: `query_movements_inner` (~стр. 1800-1990) строит `ReportRow` для «Перемещений», LIMIT MOVEMENTS_REPORT_LIMIT=1000;
`movements_truncation_notice(shown, total)`; `fn row_field(row, col, tz, shorten) -> String` (стр. ~1202);
`export_csv(&rows, columns, column_labels)` и `export_pdf(...)` обе идут через `row_field`.
Прочие конструкторы ReportRow (стр. ~1432, 1654, 2119, 2291, 2443 + тестовые make_row) ставят `batch_*: None`.

tauri_cmds/reports.rs: `columns_for(report_type, omit_type_column)` и `column_labels_for(...)` индексно согласованы
(тест `column_labels_for_is_index_aligned_with_columns_for`); movements = 7 ключей:
handover_date_utc, device_name, entity_type_label, from_place_path, place_path, actor_name, reason.

UI: `DeviceImportCsvModal.svelte` — шаги 1..4; `autoMapHeaders` (стр. ~52) знает только 10 прежних заголовков;
шаг 3 — таблица маппинга (`mapping[header]`), шаг 4 — итог (`report.inserted`, `report.failed`).
`ReportTable.svelte` — свёртка пакета, подпись `(${n} ${pluralizeRu(n, ['устройство','устройства','устройств'])})` (стр. ~344),
`synthesizeHeader` берёт `batch_size` из первой строки.
</interfaces>
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: INT-21 + N4 — нераспознанный тип/статус при CSV-импорте виден в предпросмотре и в итоге</name>
  <files>crates/trackly-app/src/services/device_service.rs, crates/trackly-app/src/dto/device.rs, ui/src/bindings.ts, ui/src/features/devices/DeviceImportCsvModal.svelte, crates/trackly-app/tests/devices_csv_import.rs</files>
  <behavior>
    - Импорт строки с типом «Пинтер» (опечатка): строка вставлена с типом «Устройство», `report.warnings` содержит запись {row_index = номер строки, field = "type", value = "Пинтер", applied = "Устройство"}, `report.coerced_rows == 1`.
    - Импорт строки со статусом «На сскладе»: вставлена со статусом по умолчанию, предупреждение field = "status", applied = имя статуса по умолчанию из справочника.
    - Одна строка с ошибкой и в типе, и в статусе даёт два предупреждения, но `coerced_rows` считает её один раз.
    - Пустая ячейка типа/статуса и псевдонимы «Расходник» / «На ремонте» — без предупреждений.
    - Строка, отклонённая по другой причине (нет наименования, место не найдено), предупреждений не порождает (они только для вставленных строк).
    - Предпросмотр: `label_checks` для колонки со значениями «Принтер», «Пинтер», «» даёт по этой колонке unknown_type_count = 1 и образец {row_index 2, value "Пинтер"}; образцов не больше 20 на вид, счётчик — полный.
  </behavior>
  <action>
Решение (per finding INT-21): импорт НЕ блокировать, нераспознанное значение по-прежнему заменяется значением по умолчанию, но каждое такое приведение сообщается.

Rust:
1. В `TypeLookup` и `StatusLookup` заменить `id_for_label -> i64` на метод, различающий три исхода (например `resolve(&self, Option<&str>) -> LabelMatch` с вариантами `Empty(i64)`, `Matched(i64)`, `Unknown(i64)`; enum приватный в device_service.rs). Логика сопоставления (регистронезависимо через `to_lowercase()` в Rust, псевдонимы «Расходник»/«На ремонте») не меняется — меняется только то, что «неизвестно» теперь отличимо от «пусто». Если `id_for_label` вызывается ещё где-то — оставить его тонкой обёрткой над `resolve`.
2. В dto/device.rs добавить `RowWarning { row_index: u64 (#[specta(type = u32)]), field: String ("type"|"status"), value: String (исходное значение ячейки, обрезанное trim), applied: String (подпись применённого значения из справочника) }`; в `CsvImportReport` — `warnings: Vec<RowWarning>` и `coerced_rows: u64 (#[specta(type = u32)])`. Добавить `LabelSample { row_index: u64 (specta u32), value: String }` и `ColumnLabelCheck { header: String, unknown_type_count: u64 (u32), unknown_type_samples: Vec<LabelSample>, unknown_status_count: u64 (u32), unknown_status_samples: Vec<LabelSample> }`; в `CsvImportPreviewResponse` — `label_checks: Vec<ColumnLabelCheck>`. Derive как у соседей. Обновить существующий serde-тест в `mod tests` dto/device.rs, если он конструирует эти структуры.
3. `build_device_new_from_row` возвращает дополнительно список предупреждений этой строки (или пару исходных подписей + исходов) — без доступа к БД, как сейчас. `import_csv_commit` добавляет предупреждения в `report.warnings` ТОЛЬКО после успешной вставки строки (ветка `Ok(DeviceSaveOutcome::Created)`) и инкрементирует `coerced_rows` один раз на строку. `applied` — `types.name_for_id(id)` / `statuses.name_for_id(id)`.
4. `import_csv_preview`: загрузить `TypeLookup` и `StatusLookup` один раз, для КАЖДОЙ колонки по всем строкам посчитать непустые значения с исходом `Unknown` отдельно для типа и для статуса (полный счётчик + до 20 образцов, row_index 1-based как в commit). Колонки, где оба счётчика 0, всё равно включать (UI ищет по header). Текст ячеек в SQL не попадает (сопоставление в Rust, как сейчас). Учесть: 50 МБ-файл — O(строки x колонки) сравнений в памяти, без запросов в цикле.
5. Регенерировать bindings: `cargo test -p trackly-app --test export_bindings`.

UI (`DeviceImportCsvModal.svelte`, Svelte 5 runes, SCSS, существующий класс `.warning-banner`):
- Шаг 3 (маппинг): `$derived` — для заголовка, сопоставленного с 'type', взять `preview.label_checks.find(c => c.header === header)` и при `unknown_type_count > 0` показать под таблицей предупреждение «Колонка «{header}»: {N} значений типа не распознано — такие строки будут импортированы как «Устройство»» и список образцов «строка {row_index}: «{value}»»; аналогично для 'status' («…как статус по умолчанию»). Решение «распознано или нет» берётся только из ответа сервера — логику сопоставления подписей в JS не дублировать.
- Шаг 4 (итог): рядом с «Импортировано/Пропущено с ошибками» — «С заменой типа/статуса: {coerced_rows}»; при наличии — раскрываемый список по образцу списка ошибок: «Строка {row_index}: {поле «Тип»|«Статус»} «{value}» не распознан — записан «{applied}»».
- Только кастомные компоненты, уже используемые в модалке; новых нативных select не добавлять.

Тесты (devices_csv_import.rs, только вымышленные данные, по образцу `import_commit_per_row_errors` и хелпера `auto_map`): по одному тесту на каждое поведение из <behavior>; CSV собирать строкой в тесте (байты UTF-8), места — через `seed_place`.
  </action>
  <verify>
    <automated>cd /Users/madsas/Projects/trackly && pnpm --dir ui build >/dev/null && TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test app_devices -- devices_csv_import 2>&1 | tail -5 && cargo test -p trackly-app --test export_bindings 2>&1 | tail -3 && grep -c "label_checks\|coerced_rows" ui/src/bindings.ts && pnpm --dir ui check 2>&1 | tail -3</automated>
  </verify>
  <done>Все тесты devices_csv_import зелёные (новые и прежние); bindings.ts содержит RowWarning, ColumnLabelCheck, label_checks, coerced_rows; svelte-check без ошибок; модалка показывает предупреждения на шагах 3 и 4. Коммит: `feat(quick-261121-hef): CSV-импорт сообщает о нераспознанных типе и статусе (INT-21, N4)`.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: INT-32/INT-17 — CSV-экспорт с группой, ролью, происхождением и USB-хостом; INT-13 — номер по шаблону при SNMP-приёмке</name>
  <files>crates/trackly-app/src/services/device_service.rs, crates/trackly-app/src/tauri_cmds/printers.rs, crates/trackly-app/tests/devices_csv_export.rs, crates/trackly-app/tests/printers_admit.rs</files>
  <behavior>
    - Экспорт: заголовок = прежние 10 колонок в прежнем порядке + «Группа», «Роль в группе», «Происхождение», «USB-хост».
    - Устройство-якорь группы «Стенд А»: Группа = «Стенд А», Роль = «якорь»; член группы: Группа = имя группы, Роль = «в составе»; вне групп — обе пусты.
    - Устройство с origin_group_id на группу «Разбор Б»: Происхождение = «Разбор Б».
    - USB-принтер с хостом «АРМ-1» (инв. «ИНВ-7»): USB-хост = «АРМ-1 (ИНВ-7)»; сетевой принтер и не-принтер — пусто.
    - Round-trip: экспорт базы с группой/якорем/USB-принтером -> import_csv_preview -> import_csv_commit с `auto_map(headers)` в чистую БД с теми же местами: failed пуст, warnings пуст, inserted = числу экспортированных устройств.
    - Приёмка: при запомненном шаблоне контекста PrinterCreate (маска с числовым счётчиком, через `NumberTemplateService::create` + `remember_context`) два принятых IP получают два последовательных номера по шаблону; без запомненного шаблона inventory_no = None (как прежде).
  </behavior>
  <action>
INT-32/INT-17 (`export_csv_with_limit`):
- После `list_all` обогатить строки через `enrich_devices` ПАЧКАМИ по 500 id (enrich строит `IN (?1..?N)`; 50 000 параметров разом упрутся в лимит переменных SQLite) — в том же `spawn_blocking`, на одном reader-соединении.
- USB-хост: одним пачечным запросом на те же чанки `SELECT p.device_id, h.name, h.inventory_no FROM printers p JOIN devices h ON h.id = p.usb_host_device_id WHERE p.device_id IN (...)` (имена колонок сверить по схеме printers/миграциям; значения — только параметры). Ячейка: «{name} ({inventory_no})» либо «{name}» без номера.
- Дописать 4 заголовка В КОНЕЦ (после «Статус»), чтобы позиционные тесты «Место» и прежний порядок не сдвинулись. Группа = `group_name`, иначе `anchor_group_name`; Роль = «якорь» если `anchor_group_id.is_some()`, иначе «в составе» если `group_id.is_some()`, иначе пусто; Происхождение = `origin_group_name`. Все ячейки через `csv_safe`.
- Импорт не трогать: новые заголовки не попадают в `autoMapHeaders`/`auto_map`, сервер игнорирует несопоставленные колонки (T-02-05-08) — это и проверяет round-trip-тест. Обновить `export_headers_russian` (и любые тесты с точным списком заголовков) под 14 колонок.

INT-13 (`build_printers_admit` в tauri_cmds/printers.rs), «так же, как обычное создание» (окно «Новый принтер» подставляет следующий номер запомненного шаблона контекста PrinterCreate):
- Внутри цикла по IP, перед `create_with_printer_block`: `ctx.devices.number_templates.get_context(TemplateContextDto::PrinterCreate)` (или NumberTemplateService из AppCtx, если там он есть отдельным полем — смотреть context.rs); при `Some(id)` — `peek_next_for_context(id, PrinterCreate)` и `inventory_no = Some(next.rendered)`; при `overflowed == true` номер не ставить (пусто), как и при отсутствии шаблона. Peek делать на КАЖДОЙ итерации (после предыдущей вставки счётчик уже сдвинулся).
- Если `create_with_printer_block` вернул `AppError::Conflict` (номер занят гонкой) — повторить создание этого IP один раз с `inventory_no: None`, а не терять принтер. Контекстную память (`remember_context`) приёмка не меняет. Комментарий у DeviceNew про «inventory_no: None … NeedsConfirmation недостижим» обновить: create_with_printer_block скрипт-микс не проверяет, NeedsConfirmation по-прежнему недостижим.
- Место: у приёмки нет источника места (находка требует только номер) — `place_id: None` остаётся, комментарий D-07 сохранить.

Тесты: devices_csv_export.rs — по образцу существующих (`make_service`, `seed_place`); группы/якорь/происхождение заводить через существующие сервисы или прямой SQL как в соседних тестах groups (сверить с tests/devices_anchor_list.rs); printers_admit.rs — по образцу `a1_admit_creates_single_printer_row_with_network` с `minimal_ctx()` и mock-SNMP. Только вымышленные имена.
  </action>
  <verify>
    <automated>cd /Users/madsas/Projects/trackly && TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test app_devices -- devices_csv 2>&1 | tail -5 && TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test app_cartridges -- printers_admit 2>&1 | tail -5</automated>
  </verify>
  <done>Экспорт содержит 14 колонок, новые заполнены по правилам из <behavior>; round-trip-тест зелёный; приёмка нумерует по запомненному шаблону и не падает на гонке номера; прежние тесты devices_csv_* и printers_admit зелёные. Два коммита: `feat(quick-261121-hef): CSV-экспорт устройств с группой, ролью, происхождением и USB-хостом (INT-32, INT-17)` и `fix(quick-261121-hef): SNMP-приёмка присваивает номер по шаблону «Новый принтер» (INT-13)`.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 3: INT-06 + F18 — колонка «Пакет» в CSV и печати «Перемещений», честное «показано K из N»</name>
  <files>crates/trackly-app/src/dto/reports.rs, crates/trackly-app/src/services/report_service.rs, crates/trackly-app/src/tauri_cmds/reports.rs, ui/src/bindings.ts, ui/src/features/reports/ReportTable.svelte, crates/trackly-app/tests/report_movements_truncation.rs, crates/trackly-app/tests/report_csv_export.rs</files>
  <behavior>
    - `columns_for("movements", _)` оканчивается ключом "batch", `column_labels_for` — меткой «Пакет»; тест индексного согласования проходит для обоих значений omit_type_column.
    - row_field(.., "batch"): строка вне пакета -> ""; header или member пакета «Стенд А» с batch_size 3 и batch_shown 3 -> «Перенос группы «Стенд А» (3 устройства)»; при batch_shown 2 < batch_size 3 -> «Перенос группы «Стенд А» (показано 2 из 3 устройств)».
    - Русская плюрализация 1/2/5/11/21 (устройство/устройства/устройств) совпадает с формой экрана.
    - query_movements_inner: batch_shown = число строк этого batch_id с entity_type 'device' в ВОЗВРАЩЁННОМ наборе; при усечении (LIMIT режет пакет) batch_shown < batch_size; вне пакета None.
    - CSV-экспорт «Перемещений» с переносом группы: каждая строка пакета несёт непустую ячейку «Пакет» с именем группы.
  </behavior>
  <action>
Решение по F18 (из двух вариантов находки): усечение пакет не перестраивает, а ЧЕСТНО пишет «показано K из N» — одинаково на экране, в CSV и в печати.

1. dto/reports.rs: в `ReportRow` после `batch_size` добавить `batch_shown: Option<i64>` с `#[specta(type = Option<i32>)]` и doc-комментарием (число устройств пакета в видимом наборе; `< batch_size` значит, что пакет показан частично — усечение или фильтр). Во всех прочих конструкторах ReportRow (report_service.rs ~1432/1654/2119/2291/2443, тестовые make_row, tests/*) — `batch_shown: None`; компилятор перечислит все места.
2. report_service.rs `query_movements_inner`: после цикла построения `rows` одним проходом посчитать `HashMap<String, i64>` по batch_id для строк с `entity_type == 'device'` (сохранить исходный entity_type рядом либо сравнивать `entity_type_label == Some("Устройство")` — выбрать первое, entity_type уже есть в кортеже) и проставить `batch_shown = Some(count)` (0, если в пакете видны только группа/картриджи) каждой строке с batch_id.
3. report_service.rs: функция `pub(crate) fn batch_cell(row: &ReportRow) -> String` и арм `"batch" => batch_cell(row)` в `row_field`. Формат: имя = `batch_label` (пусто -> «без имени»); N = `batch_size`, K = `batch_shown`; если K < N — «Перенос группы «{label}» (показано {K} из {N} устройств)» (родительный падеж после «из» — всегда «устройств»), иначе «Перенос группы «{label}» ({N} {плюрализация})». Плюрализацию сделать маленькой приватной функцией по образцу `group_type_service::plural_groups` (правило: n%10==1 && n%100!=11 -> «устройство»; 2..4 не 12..14 -> «устройства»; иначе «устройств»). Unit-тесты в `mod tests` report_service.rs на оба формата и на 1/2/5/11/21.
4. tauri_cmds/reports.rs: в `columns_for("movements")` добавить "batch" последним, в `column_labels_for("movements")` — «Пакет» последним (retain по «Тип» не затрагивает их). Убедиться, что `column_labels_for_is_index_aligned_with_columns_for` и тесты omit_type_column (стр. ~983-1022) обновлены под 8/7 колонок. Шаблон `templates/report.html` НЕ трогать: колонки идут через уже существующий `ctx["columns"]`, новая переменная шаблона ломает редактор шаблонов (strict-undefined).
5. Регенерировать bindings (`cargo test -p trackly-app --test export_bindings`).
6. ReportTable.svelte: добавить `batch_shown?: number | null` в локальный тип строки, `synthesizeHeader` переносит `batch_shown` из первой строки; подпись заголовка пакета: при `batch_shown != null && batch_shown < batch_size` — «(показано {K} из {N} устройств)», иначе прежняя `(${n} ${pluralizeRu(...)})`. Число K берётся с сервера, формулу подсчёта в JS не повторять.
7. Тесты: report_movements_truncation.rs — сценарий, где LIMIT режет пакет (по образцу существующих тестов этого файла: засев > MOVEMENTS_REPORT_LIMIT строк, пакет на границе) -> у строк пакета `batch_shown < batch_size`; report_csv_export.rs — перенос группы даёт в CSV колонку «Пакет» с именем группы в каждой строке пакета и пустую ячейку у обычного перемещения.
  </action>
  <verify>
    <automated>cd /Users/madsas/Projects/trackly && TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --lib -- report 2>&1 | tail -4 && TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test app_reports 2>&1 | tail -4 && cargo test -p trackly-app --test export_bindings 2>&1 | tail -2 && grep -c "batch_shown" ui/src/bindings.ts</automated>
  </verify>
  <done>CSV и печать «Перемещений» содержат колонку «Пакет»; частично показанный пакет подписан «показано K из N» на экране, в CSV и печати; app_reports и lib-тесты report_* зелёные; bindings.ts содержит batch_shown. Коммит: `fix(quick-261121-hef): пакет переноса группы в CSV и печати «Перемещений», честное «показано K из N» (INT-06, F18)`.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| LAN/desktop -> import_csv_preview/commit | недоверенный CSV-файл (до 50 МБ) |
| БД -> CSV-экспорт / печать | пользовательский текст (имена групп, устройств) уходит в Excel и HTML |
| SNMP-устройство -> admit | sys_name/sys_descr от устройства в сети |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-hef-01 | Tampering | export_csv_with_limit, report export_csv | mitigate | все новые ячейки (Группа, Роль, Происхождение, USB-хост, Пакет) через `csv_safe` — формульная инъекция Excel |
| T-hef-02 | Tampering | label_checks / enrich / USB-хост запросы | mitigate | значения ячеек и id только параметрами SQL; сопоставление подписей в Rust, текст в SQL не попадает |
| T-hef-03 | Denial of Service | import_csv_preview label_checks | mitigate | прежний потолок 50 МБ; образцов ≤ 20 на колонку и вид, счётчики — числа; справочники грузятся один раз, без запросов в цикле |
| T-hef-04 | Denial of Service | экспорт enrich | mitigate | пачки по 500 id; прежний потолок EXPORT_MAX_ROWS с Validation |
| T-hef-05 | Information Disclosure | печать «Пакет» | accept | имя группы уже видно на экране отчёта тем же ролям; report.html экранирует через существующий safe-env, шаблон не меняется |
| T-hef-06 | Elevation of Privilege | build_printers_admit | accept | authorize(MutatePrinters) уже первым; приёмка не меняет контекстную память шаблонов и не принимает template_id от клиента |
</threat_model>

<verification>
Один cargo-процесс за раз. Перед первым тестовым прогоном: `pnpm --dir ui build`. При exit 69 на линковке — префикс `DEVELOPER_DIR=/Library/Developer/CommandLineTools`.
Гейты в конце (все — отдельные команды):
- `cargo fmt --all -- --check`
- `cargo clippy -p trackly-app --all-targets -- -D warnings`
- `pnpm --dir ui lint`
- `pnpm --dir ui check`
- Суиты: `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test app_devices`, затем `--test app_reports`, затем `--test app_cartridges` (полностью, не по фильтру — затронуты общие DTO), затем `--test app_gates` (реестровые гейты могут видеть новые read-site'ы).
- Приватность: `git diff --cached` перед каждым коммитом — только вымышленные имена («Иванов И.И.», «Стенд А», «АРМ-1»), никаких реальных названий организаций/ФИО.
</verification>

<success_criteria>
- Опечатка в типе/статусе видна до импорта (шаг маппинга) и после (итог с номером строки, значением, заменой, счётчиком), импорт не блокируется.
- CSV-экспорт устройств: 14 колонок, round-trip экспорт -> импорт без ошибок и предупреждений.
- SNMP-приёмка нумерует принтер по запомненному шаблону «Новый принтер».
- Печать и CSV «Перемещений» несут колонку «Пакет»; частичный пакет — «показано K из N» во всех трёх представлениях.
- Все гейты из <verification> зелёные.
</success_criteria>

<output>
Создать `.planning/quick/261121-hef-csv-import-export/261121-hef-SUMMARY.md` по завершении.
</output>
