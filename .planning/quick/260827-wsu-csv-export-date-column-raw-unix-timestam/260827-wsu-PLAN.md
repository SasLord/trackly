---
quick_id: 260827-wsu
slug: csv-export-date-column-raw-unix-timestam
phase: 260827-wsu
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - crates/trackly-app/src/services/report_service.rs
  - ui/src/features/reports/ReportTable.svelte
autonomous: false
requirements: [WSU-01, WSU-02]
must_haves:
  truths:
    - "CSV-экспорт отчётов «Заявки» (все 4 таба) показывает в колонке «Дата» читаемую строку «дд.мм.гг, чч:мм» в таймзоне организации, а не сырой unix-timestamp"
    - "HTML/печатный экспорт того же отчёта показывает ТУ ЖЕ читаемую строку для той же колонки — оба export-пути идут через один и тот же row_field, подтверждено чтением кода (export_csv ~851, export_pdf ~968), не только CSV, как заметил владелец продукта"
    - "Экранная таблица «Отчёты» → «Заявки» тоже показывает дд.мм.гг, чч:мм (дата+время) вместо прежнего date-only в браузерной таймзоне — экран и экспорт больше не расходятся по формату"
    - "Отсутствующий handover_date_utc (NULL) по-прежнему даёт пустую ячейку в экспорте — не изобретён плейсхолдер, конвенция combine_printer_and_place сохранена"
    - "Существующие CSV-тесты (report_requests_csv_export_uses_translated_values_not_raw_enum_keys, report_requests_category_filter_reflected_in_csv_export, csv_export_guards_formula_injection, csv_export_has_utf8_bom_and_semicolon) остаются зелёными БЕЗ изменений своего кода — ни один не проверяет сырое числовое значение этой колонки"
  artifacts:
    - path: "crates/trackly-app/src/services/report_service.rs"
      provides: "format_handover_date(unix_seconds, tz) — новый форматтер «дд.мм.гг, чч:мм»; row_field получает третий параметр tz: UtcOffset и вызывает format_handover_date в рукаве \"handover_date_utc\" вместо ts.to_string()"
      contains: "fn format_handover_date"
    - path: "ui/src/features/reports/ReportTable.svelte"
      provides: "formatCellValue — рукав handover_date_utc возвращает дд.мм.гг, чч:мм вместо d.toLocaleDateString('ru-RU') (date-only)"
      contains: "getHours"
  key_links:
    - from: "ReportService::export_csv / export_pdf"
      to: "row_field(row, col, tz)"
      via: "tz = self.get_tz_offset() считается один раз в начале каждого export-метода и передаётся в row_field на каждой строке"
      pattern: "row_field\\(row, col, tz\\)"
    - from: "row_field \"handover_date_utc\" рукав"
      to: "format_handover_date(ts, tz)"
      via: "прямой вызов внутри match-рукава"
      pattern: "format_handover_date\\(ts, tz\\)"
---

<objective>
Владелец продукта после живого UAT CSV-экспорта: колонка «Дата» в отчётах «Заявки» отдаёт сырой
unix-timestamp вместо читаемой даты/времени. Просьба — не полагаться на ручную конвертацию в
Excel, а сразу экспортировать строку вида **дд.мм.гг, чч:мм** (например, `27.08.26, 14:35`).

Чтение кода подтвердило: баг НЕ ограничен CSV. `row_field()` в `report_service.rs` (~строка 1014)
— единственный источник значения этой колонки, и у него ДВА вызывающих: `export_csv` (~851, CSV)
и `export_pdf` (~968, HTML/печатный отчёт). Оба сейчас отдают `ts.to_string()` — сырое число.
Значит печатный/HTML-отчёт несёт тот же дефект, просто владелец продукта его пока не заметил (или
не сообщил). Фиксим оба пути одним изменением сигнатуры `row_field`, а не патчим только CSV.

**Таймзона:** сервис уже имеет `ReportService::get_tz_offset()` (~строка 399) — маппит
`organization.timezone` в `UtcOffset` (`Europe/Moscow` → UTC+3, иначе UTC). `row_field` сегодня —
свободная функция без доступа к `&self`, поэтому смещение придётся прокинуть параметром: меняем
сигнатуру на `row_field(row: &ReportRow, col: &str, tz: UtcOffset) -> String`, оба call site'а
(оба — методы `&self`) вычисляют `let tz = self.get_tz_offset();` один раз и передают дальше.

**Два знака года:** владелец продукта явно написал «дд.мм.гг» — следуем буквально (двузначный
год), конкретной причины отклоняться нет; отчёты «Заявки» покрывают текущий/недавний период,
неоднозначность года (20хх vs 19хх) не встаёт.

**Отсутствующий timestamp:** остаётся пустой строкой в экспорте (как и сегодня через
`.unwrap_or_default()`) — это осознанная конвенция проекта (см. doc-комментарий
`combine_printer_and_place`), плейсхолдер не изобретаем.

**Запятая в CSV:** новое значение содержит `,` (`27.08.26, 14:35`), но CSV-экспорт использует `;`
как delimiter (`csv::WriterBuilder::new().delimiter(b';')`, ~строка 856) — `csv` crate квотирует
ячейку только если она содержит сам delimiter, кавычку или `\r`/`\n`; запятая не является
delimiter'ом здесь, значит квотирование не требуется и код `csv_safe`/сама библиотека `csv` менять
не нужно. `csv_export_guards_formula_injection`-тест не трогает эту колонку (columns в тесте —
`device_name`/`model_label`), поведение не меняется.

**Экранная несогласованность (осознанное решение — включаем в скоуп):** `ReportTable.svelte`'s
`formatCellValue` сегодня рендерит ту же колонку через `new Date(val * 1000).toLocaleDateString
('ru-RU')` — только дата без времени, и в таймзоне БРАУЗЕРА, а не организации. Решение: выровнять
экранную ячейку на тот же формат `дд.мм.гг, чч:мм`, используя локальные `Date`-геттеры браузера —
это тот же принцип, что уже документирован в проекте (`DocumentAcceptanceModal.svelte`'s
«W-9»-комментарий: приложение single-tz, локальные часы клиентской машины трактуются как таймзона
организации — тестового/AD окружения по другой таймзоне не предполагается). Не делаем это отдельной
задачей позже: правка дешёвая (одна функция, без бэкенд-роундтрипа), а иначе после этого фикса
владелец продукта увидит на экране «27.08.26» без времени рядом с «27.08.26, 14:35» в экспорте —
новая, но предсказуемая несогласованность, которую разумнее закрыть сейчас же.

Purpose: убрать сырой unix-timestamp из обоих export-путей и синхронизировать с ним экранную
таблицу — колонка «Дата» читаема везде одинаково.

Output: `format_handover_date` в `report_service.rs`, обновлённый `row_field` (оба call site'а),
3 новых unit-теста (Москва/UTC/отсутствующий timestamp) + 2 существующих `row_field`-теста
подогнаны под новую сигнатуру; обновлённый `formatCellValue` в `ReportTable.svelte`.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@CLAUDE.md
@.planning/STATE.md

<interfaces>
Перечитать оба файла перед правкой — номера строк ниже ориентировочные (на момент планирования).

`crates/trackly-app/src/services/report_service.rs`:

- `use time::{Date, Month, PrimitiveDateTime, Time, UtcOffset};` (~строка 15) — `OffsetDateTime` НЕ
  импортирован; использовать полный путь `time::OffsetDateTime::from_unix_timestamp(...)`, как уже
  делает `act_service.rs::format_ru_date`/`format_iso_date` (~строка 2938) — тот же паттерн в этом
  проекте, новый `use` не добавлять.

- `ReportService::get_tz_offset(&self) -> UtcOffset` (~строка 399) — уже существует, ничего не
  менять; оба export-метода вызывают его через `self.get_tz_offset()`.

- `pub async fn export_csv(&self, rows: &ReportResponse, columns: &[&str]) -> Result<Vec<u8>,
  AppError>` (~строка 833) — внутри цикла по `rows.rows` (~строка 848-853):
  `columns.iter().map(|col| { let raw = row_field(row, col); csv_safe(&raw) })`. Добавить `let tz =
  self.get_tz_offset();` в начале функции (до `csv::WriterBuilder`), заменить вызов на
  `row_field(row, col, tz)`.

- `#[allow(clippy::too_many_arguments)] pub async fn export_pdf(&self, rows: &ReportResponse,
  report_name: &str, period_label: &str, org: &OrgSettingsDto, logo_bytes: Option<Vec<u8>>,
  logo_mime: Option<String>, columns: &[&str], column_labels: &[&str]) -> Result<String,
  AppError>` (~строка 892) — внутри функции, до цикла `for row in &rows.rows` (~строка 916), есть
  `let organization = self.organization.as_ref().ok_or_else(...)?;` (~строка 903). Добавить `let tz
  = self.get_tz_offset();` сразу после этого блока. Строка `table_rows.push(columns.iter().map(|col|
  row_field(row, col)).collect());` (~строка 968) → `row_field(row, col, tz)`.

- `fn row_field(row: &ReportRow, col: &str) -> String` (~строка 1014) — сигнатура → `fn row_field(row:
  &ReportRow, col: &str, tz: UtcOffset) -> String`. Рукав:
  ```
  "handover_date_utc" => row.handover_date_utc.map(|ts| ts.to_string()).unwrap_or_default(),
  ```
  заменить на
  ```
  "handover_date_utc" => row.handover_date_utc.map(|ts| format_handover_date(ts, tz)).unwrap_or_default(),
  ```
  Остальные рукава (`number`, `sub_number`, `printer_place` и т.д.) не трогать — `tz` им не нужен, но
  параметр находится в области видимости функции, использовать его не обязаны.

- Новая функция (сразу после `row_field`, перед секцией `// Internal query helpers`):
  `fn format_handover_date(unix_seconds: i64, tz: UtcOffset) -> String` — `time::OffsetDateTime::
  from_unix_timestamp(unix_seconds)`, при `Ok(odt)` — `odt.to_offset(tz)`, затем `format!("{:02}.{:02}.
  {:02}, {:02}:{:02}", local.day(), local.month() as u8, local.year().rem_euclid(100), local.hour(),
  local.minute())` (месяц как `u8` — тот же паттерн, что `act_service.rs::format_iso_date`); при
  `Err(_)` — `String::new()`.

- `#[cfg(test)] mod tests` (нижняя часть файла) — два существующих теста вызывают `row_field` с
  двумя аргументами и упадут на компиляции после смены сигнатуры:
  `row_field_printer_place_combines_device_name_and_place_path` (~строка 2307,
  `row_field(&row, "printer_place")`) и `row_field_printer_place_empty_when_no_printer_and_no_place`
  (~строка 2317, `row_field(&row, "printer_place")`) — добавить третий аргумент `UtcOffset::UTC`
  (значение не важно для колонки `printer_place`, но параметр обязателен).

- `fn moscow() -> UtcOffset` (~строка 2109, `UtcOffset::from_hms(3, 0, 0).unwrap()`) — уже существует
  в `mod tests`, переиспользовать для нового теста «Москва», не дублировать.

- `fn make_row(month_key: &str, device_name: &str, giver: &str) -> ReportRow` (~строка 2513) — уже
  задаёт `handover_date_utc: Some(1_780_000_000)`; для новых тестов взять `let mut row =
  make_row(...); row.handover_date_utc = Some(...);` — тот же паттерн мутации после `make_row`, что
  уже используют `row_field_printer_place_*`-тесты.

`ui/src/features/reports/ReportTable.svelte` (~строка 78, функция `formatCellValue`):
```
function formatCellValue(row: ReportRow, colKey: string): string {
  const val = row[colKey];
  if (val === null || val === undefined) return '—';
  // Format UTC timestamps as date string
  if (colKey === 'handover_date_utc' && typeof val === 'number') {
    const d = new Date(val * 1000);
    return d.toLocaleDateString('ru-RU');
  }
  return String(val);
}
```
</interfaces>
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: Бэкенд — читаемая дата в row_field (CSV + HTML/PDF export)</name>
  <files>crates/trackly-app/src/services/report_service.rs</files>
  <behavior>
    - Тест 1 (Москва, UTC+3, day rollover): epoch `1_768_515_300` = `2026-01-15 22:15:00 UTC` →
      `format_handover_date(1_768_515_300, moscow())` == `"16.01.26, 01:15"` (дата сдвигается на
      следующий день под смещением — проверяет, что tz реально применяется, а не игнорируется).
    - Тест 2 (UTC): тот же epoch, `format_handover_date(1_768_515_300, UtcOffset::UTC)` ==
      `"15.01.26, 22:15"`.
    - Тест 3 (отсутствующий timestamp): `row_field(&row, "handover_date_utc", moscow())` с
      `row.handover_date_utc = None` возвращает `""` (не паникует, не «—», сохраняет текущую
      конвенцию пустой ячейки).
  </behavior>
  <action>
Перечитать файл перед правкой (номера строк из interfaces ориентировочные).

Добавить функцию `format_handover_date(unix_seconds: i64, tz: UtcOffset) -> String` сразу после
`row_field` — реализация и формула форматирования описаны в `<interfaces>` выше дословно.

Сменить сигнатуру `row_field` на `fn row_field(row: &ReportRow, col: &str, tz: UtcOffset) ->
String`, обновить рукав `"handover_date_utc"`, чтобы вызывать `format_handover_date(ts, tz)`
вместо `ts.to_string()`. Остальные match-рукава не трогать.

В `export_csv`: добавить `let tz = self.get_tz_offset();` в начало функции, заменить вызов
`row_field(row, col)` на `row_field(row, col, tz)` в замыкании внутри `.map`.

В `export_pdf`: добавить `let tz = self.get_tz_offset();` сразу после блока `let organization =
self.organization.as_ref().ok_or_else(...)?;`, заменить `row_field(row, col)` на `row_field(row,
col, tz)` в строке `table_rows.push(...)`.

В `#[cfg(test)] mod tests`: обновить оба существующих вызова `row_field(&row, "printer_place")` →
`row_field(&row, "printer_place", UtcOffset::UTC)` (третий аргумент требуется компилятором, значение
для этой колонки роли не играет). Добавить 3 новых unit-теста из `<behavior>`, переиспользуя
`make_row`/`moscow()` — конкретные имена: `row_field_handover_date_formats_readable_moscow`,
`row_field_handover_date_formats_readable_utc`, `row_field_handover_date_absent_is_empty`. Только
вымышленные имена (`Kyocera-01`, «Иванов И.И.» — уже в `make_row`), CLAUDE.md privacy-условие.

Прогнать полный набор бэкенд-проверок из repo_constraints: один `cargo test` за раз, `-p
trackly-app` со `--skip login_remember_persistent_cookie` и `TRACKLY_AD_MOCK=1
TRACKLY_SNMP_MOCK=1`; ЗАКРЫВАЮЩИЙ прогон должен явно включать все report-related бинарники
(`report_requests`, `report_csv_export`, `report_acts`, `report_cartridges`,
`report_place_subtree`, `html_report_render`, `reports_period_required`, `report_period_bounds`,
`report_returns_sub_number`) плюс `--lib`, а не только файлы, которые правились в этой задаче — по
памяти проекта необновлённый бинарник может незаметно сломаться. `cargo fmt --check` перед
коммитом.
  </action>
  <verify>
    <automated>TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --lib report_service:: -- --test-threads=1 2>&1 | tail -60</automated>
  </verify>
  <done>3 новых unit-теста зелёные (Москва/UTC/отсутствующий timestamp); 2 существующих row_field-теста компилируются и остаются зелёными с новым третьим аргументом; закрывающий прогон всех report-* интеграционных бинарников + --lib зелёный (включая report_requests_csv_export_uses_translated_values_not_raw_enum_keys, report_requests_category_filter_reflected_in_csv_export, csv_export_guards_formula_injection, csv_export_has_utf8_bom_and_semicolon — без изменения их кода); cargo fmt --check чист; cargo clippy -p trackly-app --all-targets -- -D warnings чист.</done>
</task>

<task type="auto">
  <name>Task 2: Фронтенд — выровнять экранную ячейку «Дата» с новым экспортным форматом</name>
  <files>ui/src/features/reports/ReportTable.svelte</files>
  <action>
Перечитать файл перед правкой (номер строки из interfaces ориентировочный, Task 1 этот файл не
трогал).

В `formatCellValue` заменить рукав `handover_date_utc`: вместо `new Date(val * 1000)
.toLocaleDateString('ru-RU')` собрать строку `дд.мм.гг, чч:мм` из локальных `Date`-геттеров
(`getDate`, `getMonth`, `getFullYear`, `getHours`, `getMinutes`) с ручным `padStart(2, '0')` —
никаких новых зависимостей, никакого `Intl`/`toLocaleString` (чтобы формат был детерминированным
и не зависел от локали браузера). Прокомментировать одной строкой, почему используются локальные
(не UTC) геттеры — тот же W-9-принцип single-tz-приложения, что уже задокументирован в
`DocumentAcceptanceModal.svelte` (браузер = организация, отдельного тестового окружения с другой
таймзоной не предполагается).

Пересобрать фронтенд-бандл после правки (`pnpm --dir ui build`) — обязательно для последующей
LAN-браузер/десктоп проверки, `cargo tauri dev` HMR-ит только десктоп-вебвью.
  </action>
  <verify>
    <automated>pnpm --dir ui exec svelte-check --tsconfig ./tsconfig.json 2>&1 | tail -40 && pnpm --dir ui lint 2>&1 | tail -40 && pnpm --dir ui build 2>&1 | tail -40</automated>
  </verify>
  <done>svelte-check/lint/build все зелёные; formatCellValue больше не вызывает toLocaleDateString для handover_date_utc; ui/dist пересобран.</done>
</task>

<task type="checkpoint:human-verify" gate="blocking">
  <name>Task 3: Human UAT — колонка «Дата» читаема на всех трёх поверхностях</name>
  <action>Блокирующая ручная проверка: Task 1/2 автоматизировали всё, что можно (backend-форматирование
+ frontend-выравнивание), но компиляционные/lint-гейты не доказывают ни визуальный результат в живом
приложении, ни содержимое реального PDF/print-предпросмотра — нужно подтвердить глазами, что экран,
CSV и HTML/PDF показывают один и тот же читаемый формат «дд.мм.гг, чч:мм».</action>
  <what-built>
Task 1 автоматизировал бэкенд (CSV- и HTML/PDF-экспорт «Заявки» показывают дд.мм.гг, чч:мм вместо
сырого unix-timestamp, таймзона организации). Task 2 выровнял экранную таблицу на тот же формат.
Компиляционные/lint-гейты не доказывают визуальное поведение в реальном приложении и не проверяют
PDF/print-предпросмотр глазами — нужна живая проверка на всех трёх поверхностях сразу, потому что
именно их рассинхрон был исходной жалобой владельца продукта.
  </what-built>
  <how-to-verify>
1. Убедиться, что `ui/dist` пересобран после Task 2 (`pnpm --dir ui build`), затем запустить
   `cargo tauri dev` (или открыть режим сервера в LAN-браузере).
2. Открыть «Отчёты» → домен «Заявки» → любая вкладка (Все/Открытые/В работе/Выполненные) с периодом,
   покрывающим хотя бы одну существующую заявку.
3. Экранная таблица: колонка «Дата» показывает `дд.мм.гг, чч:мм` (например `27.08.26, 14:35`) —
   дата И время, не только дата.
4. Экспортировать CSV — открыть файл (текстом или в Excel с delimiter `;`): колонка «Дата»
   содержит ту же строку `дд.мм.гг, чч:мм`, не число; запятая внутри значения не ломает разбор
   столбцов (delimiter — точка с запятой).
5. Открыть HTML/PDF-предпросмотр того же отчёта: колонка «Дата» показывает тот же формат, не сырое
   число — это самостоятельная проверка, отдельная от CSV (см. objective — исходный баг был не
   только в CSV).
6. Значение из шага 3 (экран), 4 (CSV) и 5 (PDF/HTML) для одной и той же заявки должно совпадать
   строка-в-строку (с точностью до таймзоны — обе стороны трактуют локальные часы машины как
   таймзону организации, см. objective).
7. Если в фикстурах есть заявка без даты (либо создать через UI без завершения) — убедиться, что
   ячейка «Дата» в CSV/PDF пустая, а не «—»/«NaN»/«Invalid Date» (экран может показывать «—» —
   это существующий null-fallback formatCellValue, не regressии).
  </how-to-verify>
  <resume-signal>Напишите "approved" или опишите, что увидели не так</resume-signal>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|--------------|
| Нет новой границы доверия | Задача только форматирует уже вычисленное сервером значение (`handover_date_utc`, existing column на уже авторизованном `Action::ReadData` export-пути) — новый внешний ввод не появляется, источник данных не меняется |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-260827-wsu-01 | Information Disclosure | `row_field` рукав `"handover_date_utc"` | accept | Те же данные (handover_date_utc), что и раньше отдавались в экспорте как сырое число — меняется только представление, не состав раскрываемых полей; `Action::ReadData`-авторизация на export-командах не затронута |
| T-260827-wsu-02 | Tampering | `format_handover_date(ts, tz)` | accept | Чистая функция форматирования над серверным `OffsetDateTime`, построенным из хранимого `i64`; формат-строка — константа компиляции (`format!` с литералом), не собирается из пользовательского ввода — инъекция формата структурно невозможна |
</threat_model>

<verification>
1. `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --lib report_service:: -- --test-threads=1` — все зелёные (3 новых + 2 обновлённых).
2. Закрывающий прогон report-* интеграционных бинарников из repo_constraints (по одному `cargo test` за раз, `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1`, `-- --skip login_remember_persistent_cookie` где применимо) — 0 упавших.
3. `cargo fmt --check` — чист.
4. `cargo clippy -p trackly-app --all-targets -- -D warnings` — чист.
5. `pnpm --dir ui exec svelte-check`, `pnpm --dir ui lint`, `pnpm --dir ui build` — все зелёные.
6. Живая проверка (Task 3) — экран/CSV/PDF показывают одинаковый читаемый формат дд.мм.гг, чч:мм.
</verification>

<success_criteria>
- CSV-экспорт «Заявки» показывает читаемую дату/время вместо сырого unix-timestamp.
- HTML/PDF-экспорт того же отчёта показывает тот же читаемый формат (баг закрыт на обоих путях, не только на замеченном владельцем продукта CSV).
- Экранная таблица «Отчёты» → «Заявки» согласована по формату с обоими экспортами.
- Отсутствующий timestamp остаётся пустой ячейкой в экспорте, без изобретённого плейсхолдера.
- Существующие CSV-регрессионные тесты (translated values, category filter, formula injection, BOM/delimiter) остаются зелёными без изменения своего кода.
</success_criteria>

<output>
Create `.planning/quick/260827-wsu-csv-export-date-column-raw-unix-timestam/260827-wsu-SUMMARY.md` when done
</output>
</content>
