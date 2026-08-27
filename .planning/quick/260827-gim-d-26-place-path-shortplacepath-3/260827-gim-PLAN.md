---
quick_id: 260827-gim
slug: d-26-place-path-shortplacepath-3
phase: 260827-gim
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - crates/trackly-app/src/services/report_service.rs
  - crates/trackly-app/src/tauri_cmds/reports.rs
  - crates/trackly-app/tests/report_place_subtree.rs
  - ui/src/features/reports/ReportTable.svelte
  - ui/src/features/reports/ReportsPage.svelte
autonomous: false
requirements: [PLC-04]
must_haves:
  truths:
    - "Отчёт «Заявки» (все 4 таба: requests_all/open/in_progress/completed) показывает имя принтера в колонке «Место» даже когда путь размещения принтера состоит из 3+ сегментов — D-26-сокращение применяется только к самому пути, имя принтера никогда не пропадает из ячейки"
    - "Полный путь размещения принтера остаётся в title-атрибуте ячейки при наведении, как и раньше (D-26 не отменяется, только чинится для составной колонки)"
    - "CSV- и PDF-экспорт отчёта «Заявки» продолжают выдавать составную ячейку «<принтер>, <путь>» без изменений — фикс касается только экранной сокращённой отрисовки, не транспорта экспорта"
    - "Печатные/экспортные заголовки колонки «Место» на всех отчётных доменах (устройства/картриджи/заявки) совпадают с экранными подписями ReportsPage.svelte COLUMNS_MAP — везде «Место», без остаточных «Локация»/«Расположение»/«Принтер / Локация» (W2)"
    - "Домены «Устройства» и «Картриджи» — некомпозитные ячейки place_path продолжают сокращаться до двух последних сегментов ровно как раньше (регресс не допущен)"
  artifacts:
    - path: "crates/trackly-app/src/services/report_service.rs"
      provides: "query_requests_inner пишет printer_name в device_name и printer_place в place_path РАЗДЕЛЬНО (не склеивает их в одну строку на уровне запроса); row_field получает новый ключ printer_place, который склеивает device_name+place_path через существующую combine_printer_and_place — используется только CSV/PDF export"
      contains: "\"printer_place\" =>"
    - path: "crates/trackly-app/src/tauri_cmds/reports.rs"
      provides: "columns_for для requests_* доменов возвращает printer_place вместо place_path последним ключом; column_labels_for выдаёт «Место» для ВСЕХ мест-колонок на всех доменах (W2)"
      contains: "\"printer_place\""
    - path: "crates/trackly-app/tests/report_place_subtree.rs"
      provides: "Регрессионный тест на 3-сегментном пути (переиспользует tree.room_a = «Здание А / 2 этаж / Кабинет 214»), доказывающий, что device_name и place_path приходят раздельно и НЕ теряют данные друг друга"
      contains: "requests_report_printer_name_survives_deep_place_path"
    - path: "ui/src/features/reports/ReportTable.svelte"
      provides: "formatPlaceCell — сокращает (D-26) только place_path-часть композитной ячейки, префикс из col.compositeWith (device_name) никогда не обрезается и не парсится из строки"
      contains: "compositeWith"
    - path: "ui/src/features/reports/ReportsPage.svelte"
      provides: "REQUEST_COLUMNS помечает свою place_path-колонку явным сигналом compositeWith: 'device_name', вместо неявного угадывания состава ячейки по содержимому строки"
      contains: "compositeWith: 'device_name'"
  key_links:
    - from: "ReportsPage.svelte REQUEST_COLUMNS place_path column"
      to: "ReportTable.svelte formatPlaceCell(row, col, ...)"
      via: "явное поле Column.compositeWith, проверяемое по имени колонки, а не по содержимому строки"
      pattern: "compositeWith"
    - from: "report_service.rs query_requests_inner (r.get(6)/r.get(7) — d.name AS printer_name, pfp.full_path AS printer_place)"
      to: "ReportRow.device_name / ReportRow.place_path (раздельные поля, без склейки на уровне запроса)"
      via: "прямое присваивание вместо combine_printer_and_place(printer_name, printer_place)"
      pattern: "device_name: printer_name"
    - from: "row_field(row, \"printer_place\")"
      to: "combine_printer_and_place(row.device_name.clone(), row.place_path.clone())"
      via: "CSV/PDF export путь (export_csv/export_pdf), единственное оставшееся место вызова combine_printer_and_place"
      pattern: "combine_printer_and_place\\(row\\.device_name"
---

<objective>
Исправить дефект W1 из аудита вехи v1.4 (`.planning/v1.4-MILESTONE-AUDIT.md`): в отчёте «Заявки»
имя принтера молча пропадает из колонки «Принтер / Место», когда путь размещения принтера состоит
из 3 и более сегментов. Причина — межфазная коллизия: `report_service.rs::combine_printer_and_place`
(Фаза 12) склеивает printer_name+place в ОДНУ строку `"<принтер>, <путь>"`, а
`ReportTable.svelte::shortPlacePath` (Фаза 39, D-26) безусловно режет ЛЮБОЕ значение колонки
`place_path` по `' / '`, оставляя два последних сегмента — для составной строки это отрезает имя
принтера, если путь глубже двух уровней.

**Выбранный подход — разделение полей у источника (бэкенд), а не парсинг составной строки во
фронтенде.** Рассмотрены три варианта (см. brief в задаче):
1. Чинить только `ReportTable.svelte`, распознавая составную строку и парся её обратно на
   printer_name/path — хрупко: имя принтера в принципе может содержать `,` или `' / '`, парсинг
   развалится тихо.
2. Разделить поля у источника — шире по площади (report_service.rs + tauri_cmds/reports.rs +
   тесты + фронтенд), но устраняет саму возможность бага навсегда, а не патчит один симптом.
3. Средний путь — оставить составную строку на проводе, резать только «хвост» по явному сигналу
   колонки, не угадывая по содержимому.

**Выбран вариант 2, реализованный так, что фронтенду вариант 3 достаётся бесплатно**: бэкенд
перестаёт склеивать `printer_name`+`place` в одну строку в `ReportRow` для домена «Заявки» —
`place_path` несёт ЧИСТЫЙ путь, `device_name` (существующее, ранее неиспользуемое для этого домена
поле — принтер это Device по глоссарию проекта) несёт имя принтера. Склейка `"<принтер>, <путь>"`
остаётся ТОЛЬКО в CSV/PDF-экспорте (`row_field`, новый ключ `printer_place`), где полная нередактируемая
строка и раньше не резалась. Экранная отрисовка (`ReportTable.svelte`) больше не парсит строку
вообще — она получает `device_name` и `place_path` как два готовых поля и явно помечает эту
колонку флагом `compositeWith: 'device_name'` на уровне определения колонки (`ReportsPage.svelte`),
а не угадывает состав ячейки по её содержимому. D-26-сокращение (`shortPlacePath`) теперь
применяется ИСКЛЮЧИТЕЛЬНО к чистому `place_path`, никогда не видит имя принтера — баг структурно
не может повториться.

Заодно (дёшево — те же файлы, тот же тип правки) закрывается W2: `column_labels_for` в
`reports.rs` отдаёт устаревший словарь («Локация»/«Расположение»/«Принтер / Локация») вместо
«Место», как везде на экране (`ReportsPage.svelte` COLUMNS_MAP). Собственный inline-инвариант
`column_labels_for` требует ровно этого соответствия — сейчас нарушен, правится в этом же плане.

Purpose: устранить дефект, отгруженный (но не обнаруженный) Фазой 39 UAT, — воспроизводится только
на путях глубже 2 уровней, поэтому был невидим на мелком демо-дереве. Данные никогда не терялись
(полная строка была в title и в экспорте), это чисто отображение — но высокая severity в аудите:
теряется первичный идентификатор ячейки.

Output: раздельные поля `device_name`/`place_path` в `ReportRow` для домена «Заявки»; исправленная
экранная отрисовка без парсинга строк; выровненные экспортные заголовки (W2); регрессионный тест
на пути в 3 сегмента.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@CLAUDE.md
@.planning/STATE.md
@.planning/v1.4-MILESTONE-AUDIT.md

<interfaces>
Перечитать оба бэкенд-файла перед правкой — номера строк ниже ориентировочные (на момент
планирования).

`crates/trackly-app/src/services/report_service.rs`:

- `combine_printer_and_place(printer_name: Option<String>, printer_place: Option<String>) ->
  Option<String>` (~строка 279) — существующий хелпер, НЕ МЕНЯТЬ сигнатуру/тело/3 существующих
  unit-теста (`combine_printer_and_place_none_without_printer`,
  `combine_printer_and_place_appends_place`, `combine_printer_and_place_printer_only_when_place_missing`,
  ~строки 2278-2295) — они продолжают проверять чистую функцию склейки, которая просто переезжает
  на новый call-site.

- `query_requests_inner` (~строка 1633-1662) — SQL уже селектит `d.name AS printer_name, pfp.full_path
  AS printer_place` (алиасы 6 и 7 в `query_map`). Текущее присваивание в `ReportRow`:
  `place_path: combine_printer_and_place(printer_name, printer_place), act_type: None, device_name:
  None,` — заменить на прямое присваивание раздельных полей (см. artifacts/key_links во
  frontmatter). `printer_name`/`printer_place` — локальные `Option<String>` из `r.get(6)?`/`r.get(7)?`,
  типы совпадают напрямую, `.clone()` не нужен на этом присваивании.

- `row_field(row: &ReportRow, col: &str) -> String` (~строка 1009) — единственное место вызова
  `combine_printer_and_place` после этой правки переезжает сюда: добавить новый match-рукав
  `"printer_place"`, вызывающий `combine_printer_and_place(row.device_name.clone(),
  row.place_path.clone()).unwrap_or_default()` — это ЕДИНСТВЕННЫЙ потребитель `row_field`
  (`export_csv` ~строка 846, `export_pdf` ~строка 963), CSV/PDF получают точно ту же составную
  строку `"<принтер>, <путь>"`, что и раньше.

- Doc-комментарий над `combine_printer_and_place` (~строка 275) сегодня описывает её как
  используемую при построении `ReportRow`. Обновить одну строку, отметив, что теперь она
  вызывается из `row_field`'s `"printer_place"` рукава (CSV/PDF export), а не при сборке строки
  запроса — экран (`ReportTable.svelte`) читает `device_name`/`place_path` раздельно.

`crates/trackly-app/src/tauri_cmds/reports.rs`:

- `columns_for(report_type: &str) -> Vec<&'static str>` (~строка 20) — в рукаве
  `"requests_all" | "requests_open" | "requests_in_progress" | "requests_completed"` (~строка 41)
  последний элемент вектора сегодня `"place_path"` — заменить на `"printer_place"` (остальные 5
  ключей рукава не трогать). Остальные 5 рукавов `columns_for` (device_acts/device_returns/
  device_in_use/device_in_stock/cartridge_*) НЕ содержат составных колонок — их ключ остаётся
  `"place_path"` без изменений.

- `column_labels_for(report_type: &str) -> Vec<&'static str>` (~строка 63) — W2: заменить ВСЕ
  метки места на `"Место"`, выровняв с `ReportsPage.svelte` COLUMNS_MAP (уже везде «Место»):
  `device_acts`/`device_returns` рукав — `"Локация"` → `"Место"`; `device_in_use`/`device_in_stock`
  рукав — `"Расположение"` → `"Место"`; `cartridge_consumption`/`cartridge_refills` рукав —
  `"Локация"` → `"Место"`; `cartridge_in_use`/`cartridge_in_stock` рукав — `"Расположение"` →
  `"Место"`; `requests_*` рукав — `"Принтер / Локация"` → `"Место"` (последний элемент вектора,
  индекс-выровненный с новым ключом `"printer_place"` из `columns_for`). Длина векторов в каждом
  рукаве не меняется — существующий регрессионный тест
  `column_labels_for_is_index_aligned_with_columns_for` (~строка 616) продолжит проходить без
  изменений.

`ui/src/features/reports/ReportTable.svelte` (текущее состояние, ~строки 31-107):

- `interface Column { key: string; label: string; }` (~строка 31) — добавить опциональное поле
  `compositeWith?: string;` с комментарием: имя соседнего поля строки (`ReportRow`), значение
  которого должно ПРЕДШЕСТВОВАТЬ (через `", "`) отображаемому/сокращённому значению `place_path` —
  и НИКОГДА не участвует в D-26-сокращении.

- `formatCellValue`, `formatCellTitle`, `formatCellDisplay`, `shortPlacePath` (~строки 74-105) —
  сегодня `formatCellTitle`/`formatCellDisplay` принимают `(row, colKey: string)`. Заменить сигнатуру
  на `(row, col: Column)` в обеих; внутри проверять `col.key === 'place_path'` (как сегодня) и
  делегировать в новую функцию `formatPlaceCell(row, col, transform)`, где `transform` — либо
  `shortPlacePath` (для display), либо identity-функция `(p) => p` (для title). `formatPlaceCell`
  читает `rawPath = typeof row.place_path === 'string' ? row.place_path : ''`, применяет `transform`
  только к непустому `rawPath`, читает `prefix = col.compositeWith && typeof
  row[col.compositeWith] === 'string' ? row[col.compositeWith] : ''`; если `prefix` И путь оба
  непустые — вернуть `` `${prefix}, ${path}` ``; если только `prefix` — вернуть `prefix`; если
  только путь — вернуть (возможно сокращённый) путь; если ни то ни другое — вернуть
  `formatCellValue(row, col.key)` (существующий null→'—' fallback, поведение для домена
  устройства/картриджи не меняется).

- Шаблон (~строка 161): `<td title={formatCellTitle(row, col.key)}>{formatCellDisplay(row,
  col.key)}</td>` — заменить оба `col.key` на `col` (передаём весь объект колонки, не только ключ).

`ui/src/features/reports/ReportsPage.svelte`:

- `interface Column { key: string; label: string; }` (~строка 65) — то же поле `compositeWith?:
  string;`, что и в ReportTable.svelte (независимая копия интерфейса, оба файла уже дублируют этот
  тип без общего импорта — паттерн проекта, не менять на импорт в этой задаче).

- `REQUEST_COLUMNS` (~строка 124-131) — элемент `{ key: 'place_path', label: 'Место' }` (последний
  в массиве) заменить на `{ key: 'place_path', label: 'Место', compositeWith: 'device_name' }`.
  Остальные 8 записей `COLUMNS_MAP` с `key: 'place_path'` (acts/returns/in_use/in_stock/
  consumption/refills/cartridge_in_use/cartridge_in_stock) НЕ трогать — они не композитные.

`crates/trackly-app/tests/report_place_subtree.rs` — уже используемые в файле хелперы (не менять):
`make_ctx()`, `seed_tree(&ctx.writer) -> Tree` (даёт `tree.room_a` = «Здание А / 2 этаж / Кабинет
214» — ГОТОВЫЙ путь в 3 сегмента), `seed_device(writer, name, place_id) -> i64`,
`seed_requester(writer, login, full_name) -> i64` (~строка 275), `seed_request(writer, request_type,
status, requested_by_user_id, printer_device_id, created_at_utc) -> i64` (~строка 294),
`wide_period()`. Секция 8 (`requests_report_root_place_filter_and_excludes_sibling`, ~строка 880)
— соседний тест, показывающий точный вызов `ctx.reports.list_requests_all(...)`.
</interfaces>
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: Бэкенд — раздельные поля device_name/place_path для «Заявки» + W2-выравнивание меток + регрессия</name>
  <files>crates/trackly-app/src/services/report_service.rs, crates/trackly-app/src/tauri_cmds/reports.rs, crates/trackly-app/tests/report_place_subtree.rs</files>
  <behavior>
    - Тест 1 (report_place_subtree.rs, новый): принтер «Kyocera-01» размещён в `tree.room_a`
      («Здание А / 2 этаж / Кабинет 214» — 3 сегмента). После `list_requests_all` строка отчёта
      имеет `device_name == Some("Kyocera-01")` И `place_path ==
      Some("Здание А / 2 этаж / Кабинет 214")` — раздельно, БЕЗ склейки в одну строку, ничего не
      обрезано и не потеряно на уровне бэкенда.
    - Тест 2 (report_service.rs, unit, через `make_row`-подобную сборку `ReportRow` с
      `device_name: Some("Kyocera-01")`, `place_path: Some("Здание А / 2 этаж / Кабинет
      214")`): `row_field(&row, "printer_place")` возвращает ровно
      `"Kyocera-01, Здание А / 2 этаж / Кабинет 214"` — CSV/PDF-путь склейки не регрессирует.
    - Тест 3 (report_service.rs, unit): `row_field` с `device_name: None, place_path: None`
      (заявка без принтера) на ключе `"printer_place"` возвращает пустую строку (не панику, не
      «—») — тот же контракт, что был у `combine_printer_and_place(None, None) == None`.
  </behavior>
  <action>
Перечитать оба файла (номера строк из interfaces ориентировочные).

В `report_service.rs`: в `query_requests_inner` заменить строку присваивания `place_path:
combine_printer_and_place(printer_name, printer_place), act_type: None, device_name: None,` на
`place_path: printer_place, act_type: None, device_name: printer_name,` — раздельные поля, без
склейки на уровне сборки строки запроса. Обновить doc-комментарий над `combine_printer_and_place`
одной строкой (теперь используется из `row_field`, не из сборки `ReportRow`). В `row_field`
добавить match-рукав `"printer_place" =>
combine_printer_and_place(row.device_name.clone(), row.place_path.clone()).unwrap_or_default(),`.

В `tauri_cmds/reports.rs`: в `columns_for`, рукав `requests_*`, заменить последний ключ
`"place_path"` на `"printer_place"`. В `column_labels_for` — W2-фикс: заменить ВСЕ метки места
(«Локация», «Расположение», «Принтер / Локация») на «Место» во всех шести рукавах функции (per
D-26 UI-инвариант: печатные заголовки обязаны совпадать с `ReportsPage.svelte` COLUMNS_MAP,
которая везде использует «Место»).

В `report_place_subtree.rs`: добавить тест
`requests_report_printer_name_survives_deep_place_path` рядом с существующим тестом 8 (та же
структура: `tokio::test(flavor = "multi_thread", worker_threads = 4)`, обёрнут в
`tokio::time::timeout(Duration::from_secs(30), ...)`), переиспользуя `seed_tree`, `seed_requester`,
`seed_device`, `seed_request`, `wide_period()` — сценарий из `<behavior>` Тест 1. Добавить в
`report_service.rs`'s `#[cfg(test)] mod tests` два unit-теста из `<behavior>` Тест 2 и Тест 3,
переиспользуя существующий хелпер `make_row` (или локальную сборку `ReportRow` с явными полями
по его образцу) — только вымышленные названия («Kyocera-01», существующий путь фикстуры «Здание
А / 2 этаж / Кабинет 214»), CLAUDE.md privacy-условие.

Прогнать полный набор бэкенд-проверок из repo_constraints (один `cargo test` за раз, `-p
trackly-app` со `--skip login_remember_persistent_cookie` и `TRACKLY_AD_MOCK=1
TRACKLY_SNMP_MOCK=1`, `cargo fmt --check` перед коммитом).
  </action>
  <verify>
    <automated>TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test report_place_subtree -- --test-threads=1 2>&1 | tail -40</automated>
  </verify>
  <done>12 тестов в report_place_subtree.rs зелёные (11 существующих + 1 новый); 2 новых unit-теста row_field printer_place зелёные в report_service.rs; column_labels_for_is_index_aligned_with_columns_for остаётся зелёным без изменений своего кода; cargo fmt --check чист; cargo clippy -p trackly-app --all-targets -- -D warnings чист.</done>
</task>

<task type="auto">
  <name>Task 2: Фронтенд — composite-колонка без парсинга строк (compositeWith)</name>
  <files>ui/src/features/reports/ReportTable.svelte, ui/src/features/reports/ReportsPage.svelte</files>
  <action>
Перечитать оба файла (номера строк из interfaces ориентировочные, могли сместиться после Task 1
в бэкенде — фронтенд-файлы Task 1 не трогал, но перечитать обязательно перед правкой).

В `ReportTable.svelte`: добавить `compositeWith?: string;` в `interface Column`. Заменить
`formatCellTitle`/`formatCellDisplay`, чтобы принимать `(row: ReportRow, col: Column)` вместо
`(row: ReportRow, colKey: string)`. Реализовать новую функцию `formatPlaceCell(row, col,
transformPath)` точно по описанию в interfaces (читает `rawPath` из `row.place_path`, применяет
`transformPath` только к непустому пути, читает `prefix` из `row[col.compositeWith]` только если
`col.compositeWith` задан и целевое поле — строка, комбинирует через `", "` при наличии обоих,
иначе возвращает то, что есть, иначе — существующий `formatCellValue` fallback). `formatCellDisplay`
вызывает `formatPlaceCell(row, col, shortPlacePath)` при `col.key === 'place_path'`;
`formatCellTitle` — `formatPlaceCell(row, col, (p) => p)` (identity, без сокращения — полный путь
в title, как и раньше). Оставить `shortPlacePath` без изменений — она по-прежнему режет ТОЛЬКО
принятую ей строку пути, теперь гарантированно никогда не получает составное значение. В шаблоне
заменить оба вызова `formatCellTitle(row, col.key)`/`formatCellDisplay(row, col.key)` на
`formatCellTitle(row, col)`/`formatCellDisplay(row, col)`.

В `ReportsPage.svelte`: добавить `compositeWith?: string;` в свой `interface Column`. В
`REQUEST_COLUMNS` заменить `{ key: 'place_path', label: 'Место' }` на `{ key: 'place_path', label:
'Место', compositeWith: 'device_name' }` — единственное изменение состава колонок в этом файле.

Пересобрать фронтенд-бандл после правки (обязательно для последующей LAN-браузер проверки —
`cargo tauri dev` HMR-ит только десктоп-вебвью, не собранный `ui/dist`).
  </action>
  <verify>
    <automated>pnpm --dir ui exec svelte-check --tsconfig ./tsconfig.json 2>&1 | tail -40 && pnpm --dir ui lint 2>&1 | tail -40 && pnpm --dir ui build 2>&1 | tail -40</automated>
  </verify>
  <done>svelte-check/lint/build все зелёные; ReportTable.svelte больше не содержит парсинга составной строки (никакого split по «, » внутри компонента); ReportsPage.svelte REQUEST_COLUMNS явно помечает place_path-колонку через compositeWith.</done>
</task>

<task type="checkpoint:human-verify" gate="blocking">
  <name>Task 3: Human UAT — имя принтера в отчёте «Заявки» на пути 3+ сегментов</name>
  <action>Блокирующая ручная проверка: Task 1/2 автоматизировали всё, что можно (backend split + frontend compositeWith), но svelte-check/eslint/build не доказывают поведение рун Svelte 5 в рантайме (см. CLAUDE.md-память "Compile gates miss Svelte runtime") — нужно визуально подтвердить, что имя принтера больше не пропадает из ячейки «Место» в живом приложении.</action>
  <what-built>
Раздельные бэкенд-поля device_name/place_path для домена «Заявки» + фронтенд, который склеивает их
явно по compositeWith (без парсинга строк) + сокращает D-26 только путь. Заголовки экспорта
выровнены со экраном («Место» везде, W2). Компиляционные гейты (svelte-check/eslint/build) не
доказывают поведение рун Svelte 5 в рантайме — нужна живая проверка.
  </what-built>
  <how-to-verify>
1. Убедиться, что `ui/dist` пересобран после Task 2 (`pnpm --dir ui build`), затем запустить
   `cargo tauri dev` (или открыть режим сервера в LAN-браузере).
2. В дереве мест создать (или использовать существующее) место глубиной 3+ сегмента, например
   «Здание А / 2 этаж / Кабинет 214».
3. Создать/использовать устройство-принтер (тип «Принтер») в этом месте, например «Kyocera-01».
4. Создать заявку с этим принтером (любой тип, например «free_form»), либо использовать
   существующую тестовую заявку на этот принтер.
5. Открыть «Отчёты» → домен «Заявки» → любая из 4 вкладок (Все/Открытые/В работе/Выполненные) →
   период, покрывающий дату заявки.
6. В колонке «Место» для строки этой заявки ожидается текст вида
   «Kyocera-01, 2 этаж / Кабинет 214» (имя принтера ВСЕГДА видно, путь сокращён до 2 последних
   сегментов) — имя принтера НЕ должно пропадать.
7. Навести курсор на ячейку — title-подсказка должна показывать полный путь:
   «Kyocera-01, Здание А / 2 этаж / Кабинет 214».
8. Экспортировать этот же отчёт в CSV и в PDF-предпросмотр — ячейка/колонка «Место» там должна
   содержать ПОЛНУЮ несокращённую строку «Kyocera-01, Здание А / 2 этаж / Кабинет 214» (экспорт
   не сокращает, как и раньше), и заголовок колонки — «Место» (не «Принтер / Локация»).
9. Для контроля регресса — открыть отчёт «Устройства» (любая вкладка) и убедиться, что колонка
   «Место» для устройства с путём в 3+ сегмента по-прежнему сокращается до двух последних сегментов
   с полным путём в title (как до этой задачи — некомпозитные колонки не должны были измениться).
  </how-to-verify>
  <resume-signal>Напишите "approved" или опишите, что увидели не так</resume-signal>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|--------------|
| Нет новой границы доверия | Задача не меняет источник данных (ReportFilter/SQL) и не вводит новый ввод от клиента — только переносит уже вычисленные бэкендом строки (printer_name/place) из одного строкового поля в два, и меняет чистую строковую отрисовку на фронтенде |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-260827-gim-01 | Information Disclosure | `row_field(row, "printer_place")` — склейка device_name+place_path для CSV/PDF | accept | Точно тот же контент, что и раньше отдавала `combine_printer_and_place` на этапе сборки `ReportRow` — только call-site переехал, набор раскрываемых данных не изменился, авторизация (`Action::ReadData`) не затронута |
| T-260827-gim-02 | Tampering | `ReportTable.svelte formatPlaceCell` — до этой задачи фронтенд парсил составную строку по разделителям (риск: имя принтера с «,»/«/» ломает парсинг); теперь строки не парсятся вовсе | mitigate | Устранено полностью, не смягчено — device_name и place_path приходят как раздельные, готовые поля из бэкенда, фронтенд их просто конкатенирует по явному флагу колонки (`compositeWith`), никакого сплита составной строки в компоненте больше нет |
</threat_model>

<verification>
1. `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test report_place_subtree -- --test-threads=1` — 12/12 зелёных.
2. `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --lib -- --test-threads=1` (или полный `-p trackly-app` regression со `--skip login_remember_persistent_cookie`, один `cargo test` за раз) — 0 упавших.
3. `cargo fmt --check` — чист.
4. `cargo clippy -p trackly-app --all-targets -- -D warnings` — чист.
5. `pnpm --dir ui exec svelte-check`, `pnpm --dir ui lint`, `pnpm --dir ui build` — все зелёные.
6. Живая проверка (Task 3) — принтер на пути 3+ сегментов виден в отчёте «Заявки» на экране, в title и в CSV/PDF-экспорте; регресс на «Устройства» отсутствует.
</verification>

<success_criteria>
- Имя принтера никогда не пропадает из колонки «Место» отчёта «Заявки» независимо от глубины пути размещения принтера.
- D-26-сокращение (два последних сегмента + полный путь в title) продолжает работать для всех некомпозитных place_path-колонок (устройства, картриджи) без изменений.
- CSV/PDF-экспорт отчёта «Заявки» не регрессирует — та же составная строка, что и раньше.
- Заголовки CSV/PDF-экспорта колонки «Место» совпадают со экранными на всех отчётных доменах (W2 закрыт).
- Фронтенд-компонент `ReportTable.svelte` больше не парсит составную строку по разделителям — устранён источник дефекта, а не один его симптом.
</success_criteria>

<output>
Create `.planning/quick/260827-gim-d-26-place-path-shortplacepath-3/260827-gim-SUMMARY.md` when done
</output>
