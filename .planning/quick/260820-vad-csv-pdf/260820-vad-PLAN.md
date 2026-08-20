---
quick_id: 260820-vad
slug: csv-pdf
phase: 260820-vad
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - crates/trackly-app/src/dto/reports.rs
  - crates/trackly-app/src/services/report_service.rs
  - crates/trackly-app/tests/report_csv_export.rs
  - crates/trackly-app/tests/html_report_render.rs
  - crates/trackly-app/tests/html_header_parity.rs
  - crates/trackly-app/src/tauri_cmds/reports.rs
  - crates/trackly-app/src/http/reports.rs
  - crates/trackly-app/src/specta_export.rs
  - crates/trackly-app/tests/report_requests.rs
  - crates/trackly-app/tests/reports_period_required.rs
  - ui/src/features/reports/ReportSubNav.svelte
  - ui/src/features/reports/ReportsPage.svelte
  - ui/src/features/reports/ReportFilters.svelte
autonomous: true
requirements: [VAD-01, VAD-02, VAD-03, VAD-04]
must_haves:
  truths:
    - "В разделе «Отчёты» Admin/Manager видит третий домен «Заявки» рядом с «Устройства» и «Картриджи», с четырьмя вкладками: Все / Открытые / В работе / Выполненные (VAD-01)"
    - "Вкладка «Все» показывает заявки без фильтра по статусу — включая rejected; отдельной вкладки «Отклонённые» нет (VAD-01)"
    - "Все четыре вкладки домена «Заявки» — периодические (фильтр по requests.created_at_utc), селектор периода активен всегда, снимков (snapshot) в этом домене нет (VAD-01)"
    - "На экране, в CSV-экспорте и в печатной форме отчёта «Заявки» — один и тот же набор из 6 колонок для всех четырёх вкладок: №, Дата, Тип, Статус, Заявитель, Принтер / Локация (VAD-02)"
    - "Значения «Тип» (Замена картриджа / Произвольная / Учётная запись AD) и «Статус» (Открыта / В работе / Выполнена / Отклонена / Отменена) переведены на русский одинаково на экране, в CSV и в печати — перевод вычисляется один раз на бэкенде; неизвестное (не из этого списка) значение выводится как исходный ключ, а не пустая ячейка (VAD-03)"
    - "Для заявки без выбранного принтера колонка «Принтер / Локация» — пустая (не тире); тире на экране рисует фронтенд для null-значения (VAD-02)"
    - "Пользователь с ролью Manager в отчёте «Заявки» НЕ видит заявки типа ad_register (ни в строках, ни в счётчиках вкладок) — тот же RBAC-инвариант (REQ-06/T-09-11), что уже действует в разделе «Заявки» и на дашборде; Admin видит все типы (VAD-04)"
    - "И десктопные Tauri-команды (reports_list_requests_all/open/in_progress/completed), и зеркальные HTTP-роуты /api/v1/reports_list_requests_* работают одинаково для LAN-браузера (VAD-04)"
    - "Существующие отчёты «Устройства» и «Картриджи» (список, счётчики вкладок, CSV, печать) продолжают работать без изменений — полный прогон существующего набора report_*/html_report_render/reports_period_required тестов зелёный (VAD-04)"
  artifacts:
    - path: "crates/trackly-app/src/dto/reports.rs"
      provides: "ReportRow.request_type_label — новое Option<String> поле для переведённого типа заявки"
      contains: "request_type_label"
    - path: "crates/trackly-app/src/services/report_service.rs"
      provides: "query_requests_inner/count_requests_inner (общая функция со status_filter/exclude_ad_register по образцу query_acts_inner), list_requests_all/open/in_progress/completed, translate_request_type/translate_request_status/combine_printer_and_location, get_report_counts домен requests"
      contains: "fn query_requests_inner"
    - path: "crates/trackly-app/src/tauri_cmds/reports.rs"
      provides: "columns_for/column_labels_for/report_display_name/PERIOD_BASED_REPORT_TYPES/fetch_report расширены requests_*-ключами; 4 build_reports_list_requests_*/Tauri-команды"
      contains: "requests_all"
    - path: "crates/trackly-app/src/http/reports.rs"
      provides: "4 handler_list_requests_* + 4 маршрута /api/v1/reports_list_requests_*"
      contains: "reports_list_requests_all"
    - path: "crates/trackly-app/src/specta_export.rs"
      provides: "регистрация 4 новых команд для генерации ui/src/bindings.ts"
      contains: "reports_list_requests_all"
    - path: "crates/trackly-app/tests/report_requests.rs"
      provides: "Интеграционные тесты: статус-фильтр, RU-перевод на экране+CSV, пустая «Принтер / Локация» без принтера, per-tab счётчики, RBAC-исключение ad_register для Manager"
      contains: "report_requests_manager_role_excludes_ad_register"
    - path: "ui/src/features/reports/ReportSubNav.svelte"
      provides: "домен 'requests' + 4 вкладки (Все/Открытые/В работе/Выполненные)"
      contains: "reports_list_requests_all"
    - path: "ui/src/features/reports/ReportsPage.svelte"
      provides: "REQUEST_REPORTS, COLUMNS_MAP для requests-вкладок, currentCmd/reportTypeKey/currentColumns расширены доменом 'requests'"
      contains: "requests_all"
  key_links:
    - from: "ui/src/features/reports/ReportsPage.svelte reportTypeKey()"
      to: "crates/trackly-app/src/tauri_cmds/reports.rs columns_for()/fetch_report()"
      via: "report_type-ключ (requests_all|requests_open|requests_in_progress|requests_completed), передаётся в reports_export_csv/reports_export_pdf и в currentCmd() для reports_list_requests_*"
      pattern: "requests_(all|open|in_progress|completed)"
    - from: "crates/trackly-app/src/services/report_service.rs query_requests_inner"
      to: "crates/trackly-app/src/dto/reports.rs ReportRow (request_type_label/status_name/giver_name/location_name)"
      via: "translate_request_type/translate_request_status/combine_printer_and_location вычисляются один раз при чтении строки; одни и те же поля читают экран (JSON), CSV (row_field) и печать (row_field)"
      pattern: "translate_request_(type|status)"
    - from: "crates/trackly-app/src/tauri_cmds/reports.rs build_reports_list_requests_*/fetch_report/get_report_counts"
      to: "trackly_core::auth::excludes_ad_register / trackly_infra::repos::requests_sqlite::ad_register_predicate"
      via: "exclude_ad_register: bool, вычисляется из caller.role, передаётся в ReportService и применяется в WHERE-условии query_requests_inner/count_requests_inner"
      pattern: "ad_register_predicate"
    - from: "ui/src/features/reports/ReportSubNav.svelte DOMAINS/REQUEST_REPORTS"
      to: "crates/trackly-app/src/http/reports.rs router()"
      via: "имя Tauri-команды совпадает с суффиксом HTTP-маршрута (\"/api/v1/\" + cmd) — та же связь, что уже используется для device_acts/cartridge_consumption"
      pattern: "/api/v1/reports_list_requests_"
---

<objective>
Добавить третий домен «Заявки» в раздел «Отчёты» (сейчас там только «Устройства» и «Картриджи»):
просмотр на экране, экспорт CSV, печать/PDF — по тому же паттерну, что уже работает для
существующих доменов (query_*_inner + build_reports_list_* + Tauri-команда + HTTP-роут +
columns_for/column_labels_for + specta-регистрация + фронтенд-конфиг). Все decisions
зафиксированы в `.planning/quick/260820-vad-csv-pdf/260820-vad-CONTEXT.md` (D-01..D-05):
4 вкладки по статусу (Все/Открытые/В работе/Выполненные), все периодические по
`requests.created_at_utc`, 6 колонок (№, Дата, Тип, Статус, Заявитель, Принтер / Локация),
русская локализация Тип/Статус на бэкенде.

Дополнительно (не в CONTEXT.md явно, но необходимо для «без регресса» и корректной работы
существующего RBAC-инварианта REQ-06/T-09-11): заявки типа `ad_register` не должны стать видны
роли Manager через новый отчёт — та же логика исключения (`excludes_ad_register`), что уже
применяется в `RequestService::list/counts` и в `DashboardService`.

Purpose: единая точка учёта заявок наравне с устройствами/картриджами — без выгрузки в Excel
вручную.

Output: домен «Заявки» в Отчётах (экран + CSV + печать), рабочий на десктопе и в LAN-браузере,
без регресса существующих отчётов.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@CLAUDE.md
@.planning/STATE.md
@.planning/quick/260820-vad-csv-pdf/260820-vad-CONTEXT.md

<interfaces>
requests-таблица (миграция V006 + V031, актуальный CHECK на status включает 'cancelled' — это
рабочая фича: `RequestService::cancel()` (самоотмена), метка «Отменена» в RequestListRow.svelte/
RequestDetail.svelte, отдельная вкладка-фильтр `cancelled` в RequestsSearchAndTabs.svelte.
translate_request_status ОБЯЗАН маппить 'cancelled' → «Отменена» так же, как остальные 4 статуса —
иначе в отчёте «Заявки» (вкладка «Все») и в CSV/печати всплывёт непереведённое английское слово,
расходясь с тем, как этот же статус подписан в разделе «Заявки». Единственный статус без явного
русского маппинга в CONTEXT.md/этом плане — гипотетические будущие значения схемы, которых сейчас
не существует; для них остаётся raw-ключ fallback):

  CREATE TABLE requests (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    request_type TEXT NOT NULL CHECK (request_type IN ('cartridge_replace','free_form','ad_register')),
    status TEXT NOT NULL CHECK (status IN ('open','in_progress','completed','rejected','cancelled')) DEFAULT 'open',
    requested_by_user_id INTEGER NOT NULL REFERENCES users(id),
    assigned_to_user_id INTEGER NULL REFERENCES users(id),
    printer_device_id INTEGER NULL REFERENCES devices(id),
    cartridge_model_id INTEGER NULL REFERENCES cartridge_models(id),
    description TEXT NULL, resolution_notes TEXT NULL,
    created_at_utc INTEGER NOT NULL, updated_at_utc INTEGER NOT NULL,
    deleted_at_utc INTEGER NULL, version INTEGER NOT NULL DEFAULT 1,
    category_id INTEGER NULL REFERENCES request_categories(id),
    completed_cartridge_id INTEGER NULL REFERENCES cartridges(id),
    ad_subtype TEXT NULL
  );

Канонические джойны для заявителя/принтера/локации (crates/trackly-infra/src/repos/requests_sqlite.rs,
SELECT_REQUESTS) — использовать те же алиасы/условия в новом query_requests_inner:

  FROM requests r
  LEFT JOIN users u ON u.id = r.requested_by_user_id       -- u.full_name AS requester_name
  LEFT JOIN devices d ON d.id = r.printer_device_id         -- d.name AS printer_name
  LEFT JOIN locations dl ON dl.id = d.location_id           -- dl.name AS printer_location

RBAC-инвариант REQ-06/T-09-11 (crates/trackly-infra/src/repos/requests_sqlite.rs):

  pub fn ad_register_predicate(alias: &str) -> String   // "{alias}request_type != 'ad_register'"

  trackly_core::auth::excludes_ad_register(&Role) -> bool   // true для Manager И Employee, false для Admin

  Использование-образец (crates/trackly-app/src/services/dashboard_service.rs:389):
    trackly_infra::repos::requests_sqlite::ad_register_predicate("r.")

ReportService::query_acts_inner — образец для query_requests_inner (тот же параметризованный
WHERE-паттерн, next_idx(&owned_params) для позиционных `?N`, spawn_blocking-обёртка в pub-методах,
month_key через strftime('%Y-%m', datetime(<col>,'unixepoch','+3 hours'))). Полный текст функции
уже прочитан на этапе планирования — исполнитель должен перечитать report_service.rs перед
правкой, номера строк могли сдвинуться.

Существующий row_field(row: &ReportRow, col: &str) -> String (report_service.rs) — плоский match
по имени колонки; реюзает поля ReportRow между доменами (giver_name/location_name/status_name уже
означают разное для acts/cartridges — этот план реюзает их же для заявок, а не заводит
дублирующие поля, кроме одного нового — request_type_label).

Существующий tauri_cmds/reports.rs::fetch_report(ctx: &AppCtx, report_type: &str, filter: ReportFilter,
period: Option<PeriodDto>) — диспетчер по report_type-строке, вызывается из build_reports_export_csv
и build_reports_export_pdf. Этот план меняет его сигнатуру (добавляет caller: &Identity) — см. Task 2.

minimal_ctx() — полностью собранный AppCtx для интеграционных тестов, уже реализован в
crates/trackly-app/tests/reports_period_required.rs (test_writer_and_readers() + все сервисы,
включая requests: Arc<RequestService>, reports: Arc<ReportService>). Новый report_requests.rs
копирует эту функцию 1:1 (тот же паттерн, что html_report_render.rs / specta_roundtrip.rs).

Seed-паттерн прямой SQL-вставки через writer.execute(move |conn| {...}) — образец
crates/trackly-app/tests/request_lifecycle.rs::seed_user/seed_ad_register (транзакция,
tx.last_insert_rowid(), tx.commit()).
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Domain layer — ReportRow.request_type_label, переводы, query_requests_inner/count_requests_inner, list_requests_*, get_report_counts(requests)</name>
  <files>crates/trackly-app/src/dto/reports.rs, crates/trackly-app/src/services/report_service.rs, crates/trackly-app/tests/report_csv_export.rs, crates/trackly-app/tests/html_report_render.rs, crates/trackly-app/tests/html_header_parity.rs</files>
  <action>
Перечитать crates/trackly-app/src/services/report_service.rs и crates/trackly-app/src/dto/reports.rs
перед правкой (см. interfaces выше) — номера строк могли сдвинуться после прошлых квик-тасков.

1. `crates/trackly-app/src/dto/reports.rs`: в `ReportRow` добавить новое поле
   `pub request_type_label: Option<String>,` с doc-комментарием, что это переведённый на русский
   `request_type` (VAD-03), рендерится в колонке «Тип» доменов `requests_*`; реюза существующего
   поля здесь нет — единственное новое поле в этом плане.

2. `crates/trackly-app/src/services/report_service.rs`:

   a. Добавить импорт `use trackly_infra::repos::requests_sqlite::ad_register_predicate;` рядом
      с существующими `use trackly_infra::...` в шапке файла.

   b. В `row_field(row: &ReportRow, col: &str) -> String` добавить match-ветку:
      `"request_type_label" => row.request_type_label.as_deref().unwrap_or("").to_string(),`
      — рядом с существующей веткой `"status_name" => ...`.

   c. Добавить приватные функции-переводчики (рядом с существующими module-level helper'ами,
      например возле `csv_safe`):

      - `fn translate_request_type(raw: &str) -> String` — match по `raw`:
        `"cartridge_replace"` → `"Замена картриджа"`, `"free_form"` → `"Произвольная"`,
        `"ad_register"` → `"Учётная запись AD"`; любое другое значение (в т.ч. будущие типы) —
        `other.to_string()` (fallback на исходный ключ, per decision).

      - `fn translate_request_status(raw: &str) -> String` — match по `raw`:
        `"open"` → `"Открыта"`, `"in_progress"` → `"В работе"`, `"completed"` → `"Выполнена"`,
        `"rejected"` → `"Отклонена"`, `"cancelled"` → `"Отменена"` (per прецедент существующих
        Svelte-компонентов — RequestListRow.svelte/RequestDetail.svelte уже подписывают этот
        статус как «Отменена», отчёт обязан совпадать); любое другое (действительно неизвестное,
        не входящее в текущую CHECK-схему requests.status) значение — `other.to_string()`
        fallback.

      - `fn combine_printer_and_location(printer_name: Option<String>, printer_location: Option<String>) -> Option<String>`
        — `None` если `printer_name` есть `None` (заявка без принтера — пусто, НЕ тире, per
        decision); если `printer_name` есть `Some(name)` и `printer_location` есть
        `Some(loc)` с непустой строкой — вернуть `Some(format!("{name}, {loc}"))`; если
        `printer_location` отсутствует или пустая строка — вернуть `Some(name)` (только имя
        принтера, без хвостовой запятой).

   d. Добавить приватную функцию `fn query_requests_inner(conn: &rusqlite::Connection, ts_from:
      Option<i64>, ts_to: Option<i64>, status_filter: Option<&str>, exclude_ad_register: bool) ->
      Result<ReportResponse, AppError>` — по образцу `query_acts_inner`:
      - `clauses` начинается с `"r.deleted_at_utc IS NULL".to_string()`.
      - если `status_filter` — `Some(status)`, добавить `format!("r.status = ?{}",
        next_idx(&owned_params))` + `owned_params.push(Box::new(status.to_string()))`.
      - если `exclude_ad_register` — `true`, добавить `ad_register_predicate("r.")` в `clauses`
        (без параметра — это литеральная строка `"r.request_type != 'ad_register'"`, не требует
        `?N`).
      - `ts_from`/`ts_to` фильтруют `r.created_at_utc >= ?N` / `<= ?N` — как в `query_acts_inner`.
      - SQL: `SELECT r.id, strftime('%Y-%m', datetime(r.created_at_utc, 'unixepoch', '+3 hours'))
        AS month_key, r.created_at_utc, r.request_type, r.status, u.full_name AS requester_name,
        d.name AS printer_name, dl.name AS printer_location FROM requests r LEFT JOIN users u ON
        u.id = r.requested_by_user_id LEFT JOIN devices d ON d.id = r.printer_device_id LEFT JOIN
        locations dl ON dl.id = d.location_id WHERE {where_clause} ORDER BY r.created_at_utc ASC,
        r.id ASC LIMIT 1000` (month_key присутствует, т.к. это period-based отчёт — согласовано с
        печатной формой месяц-группировкой, как у acts/cartridge_consumption, чтобы
        `export_pdf`/`ReportTable.svelte` не рисовали пустой заголовок группы).
      - Маппинг строки в `ReportRow`: прочитать `let id: i64 = r.get(0)?;` один раз и
        использовать её для `id` И для `number: Some(id.to_string())` (колонка «№» реюзает
        существующее поле `number`); `month_key: r.get(1)?`; `handover_date_utc: r.get(2)?`
        (реюз поля под «Дата создания» заявки — тот же ключ, что уже рендерится как «Дата» для
        acts/returns, `ReportTable.svelte::formatCellValue` уже форматирует его как дату, менять
        фронтенд не нужно); `request_type_label:
        Some(translate_request_type(&r.get::<_, String>(3)?))`; `status_name:
        Some(translate_request_status(&r.get::<_, String>(4)?))`; `giver_name: r.get(5)?`
        (реюз под «Заявитель»); `location_name: combine_printer_and_location(r.get(6)?,
        r.get(7)?)` (реюз под «Принтер / Локация»); остальные поля (`sub_number`, `receiver_name`,
        `act_type`, `device_name`, `quantity`, `code`, `model_label`) — `None`.

   e. Добавить приватную функцию `fn count_requests_inner(conn: &rusqlite::Connection, ts_from:
      Option<i64>, ts_to: Option<i64>, status_filter: Option<&str>, exclude_ad_register: bool) ->
      Result<i64, AppError>` — те же `clauses`, что в `query_requests_inner` (без джойнов, они не
      нужны для COUNT), SQL: `SELECT COUNT(*) FROM requests r WHERE {where_clause}`.

   f. Добавить 4 публичных async-метода на `impl ReportService`, по образцу `list_device_acts`
      (spawn_blocking + `readers.acquire()` + вызов `query_requests_inner`):
      `list_requests_all(&self, _filter: ReportFilter, period: PeriodDto, exclude_ad_register: bool)`,
      `list_requests_open(...)`, `list_requests_in_progress(...)`, `list_requests_completed(...)`
      — каждый вычисляет `(ts_from, ts_to) = compute_period_utc(&period, self.get_tz_offset())` и
      вызывает `query_requests_inner(&conn, ts_from, ts_to, None|Some("open")|Some("in_progress")|
      Some("completed"), exclude_ad_register)`. Параметр `_filter` подчёркнут — у домена «Заявки»
      пока нет собственных фильтр-полей в `ReportFilter` (см. decision — фильтрация только по
      статусу-вкладке и периоду); принимается только ради единообразной сигнатуры с
      `fetch_report()`. Добавить doc-комментарий, объясняющий это.

   g. Расширить сигнатуру `get_report_counts(&self, domain: &str, filter: ReportFilter, period:
      PeriodDto, exclude_ad_register: bool) -> Result<ReportCountsDto, AppError>` (добавить
      параметр `exclude_ad_register: bool`) и добавить внутри существующего `spawn_blocking`-блока
      (после веток `domain == "devices"` / `domain == "cartridges"`) ветку:
      `} else if domain == "requests" {` с 4 `ReportCountEntry`: `key: "all"` →
      `count_requests_inner(&conn, ts_from, ts_to, None, exclude_ad_register).unwrap_or(0)`,
      `key: "open"` → тот же вызов с `Some("open")`, `key: "in_progress"` → `Some("in_progress")`,
      `key: "completed"` → `Some("completed")`. Ключи должны буквально совпадать с ключами вкладок
      фронтенда (`all`/`open`/`in_progress`/`completed`) — Task 3.

   h. Обновить ВСЕ существующие литералы `ReportRow { ... }` в этом файле (в `query_acts_inner`,
      `query_device_snapshot`, `query_cartridge_audit`, `query_cartridge_snapshot`, и в
      test-хелпере `fn make_row(...)` внутри `#[cfg(test)] mod tests`) — добавить
      `request_type_label: None,` в каждый. Компилятор укажет точные места, если что-то пропущено
      (`cargo check` ниже).

   i. Добавить unit-тесты в существующий `#[cfg(test)] mod tests` (в конце файла):
      `translate_request_type_known_values` (все 3 известных ключа → русские строки),
      `translate_request_type_unknown_falls_back_to_raw_key` (например `"future_type"` →
      `"future_type"`), `translate_request_status_known_values` (все 5 известных ключей, включая
      `"cancelled"` → `"Отменена"`, — документирует, что все статусы CHECK-схемы
      `requests.status` переведены, ни один не проваливается в fallback),
      `translate_request_status_unknown_falls_back_to_raw_key` (взять действительно
      немаппированное значение — например `"future_status"` → `"future_status"` — `"cancelled"`
      для этого теста больше не подходит, он теперь известный статус),
      `combine_printer_and_location_none_without_printer` (`None, None` → `None`),
      `combine_printer_and_location_appends_location` (`Some("Принтер А".into()),
      Some("Каб. 305".into())` → `Some("Принтер А, Каб. 305".into())`),
      `combine_printer_and_location_printer_only_when_location_missing` (`Some("Принтер
      А".into()), None` → `Some("Принтер А".into())`).

3. `crates/trackly-app/tests/report_csv_export.rs`: добавить `request_type_label: None,` в оба
   литерала `ReportRow { ... }`.

4. `crates/trackly-app/tests/html_report_render.rs`: добавить `request_type_label: None,` в
   литерал `ReportRow { ... }` внутри тестового хелпера.

5. `crates/trackly-app/tests/html_header_parity.rs`: добавить `request_type_label: None,` в
   литерал `ReportRow { ... }`.

Приватность (CLAUDE.md): в новых unit-тестах/doc-комментариях этого файла — только вымышленные
названия («Принтер А», «Каб. 305»), без реальных названий организации/оборудования.
  </action>
  <verify>
    <automated>cd /Users/madsas/Projects/trackly && grep -q "request_type_label" crates/trackly-app/src/dto/reports.rs && echo OK_FIELD || echo FAIL_FIELD; grep -q "fn query_requests_inner" crates/trackly-app/src/services/report_service.rs && echo OK_QUERY || echo FAIL_QUERY; grep -q "fn count_requests_inner" crates/trackly-app/src/services/report_service.rs && echo OK_COUNT || echo FAIL_COUNT; grep -q "fn list_requests_all" crates/trackly-app/src/services/report_service.rs && echo OK_LIST || echo FAIL_LIST; grep -q "fn translate_request_type" crates/trackly-app/src/services/report_service.rs && echo OK_TYPE_FN || echo FAIL_TYPE_FN; grep -q "fn translate_request_status" crates/trackly-app/src/services/report_service.rs && echo OK_STATUS_FN || echo FAIL_STATUS_FN; grep -q "ad_register_predicate" crates/trackly-app/src/services/report_service.rs && echo OK_ADREG || echo FAIL_ADREG; cargo check -p trackly-app --all-targets 2>&1 | tail -150; cargo test -p trackly-app --lib services::report_service::tests:: -- --test-threads=1 2>&1 | tail -80</automated>
  </verify>
  <done>ReportRow несёт request_type_label; query_requests_inner/count_requests_inner дают общий параметризованный запрос по requests со статус-фильтром и exclude_ad_register-условием (по образцу query_acts_inner); 4 list_requests_* метода и get_report_counts(domain="requests") реализованы; переводчики Тип/Статус с fallback на raw-ключ покрыты unit-тестами; весь crate (включая tests/*) компилируется — cargo check -p trackly-app --all-targets чист.</done>
</task>

<task type="auto">
  <name>Task 2: Wiring — tauri_cmds/reports.rs, HTTP-роуты, specta, интеграционные тесты</name>
  <files>crates/trackly-app/src/tauri_cmds/reports.rs, crates/trackly-app/src/http/reports.rs, crates/trackly-app/src/specta_export.rs, crates/trackly-app/tests/report_requests.rs, crates/trackly-app/tests/reports_period_required.rs</files>
  <action>
Перечитать crates/trackly-app/src/tauri_cmds/reports.rs и crates/trackly-app/src/http/reports.rs
перед правкой.

1. `crates/trackly-app/src/tauri_cmds/reports.rs`:

   a. `columns_for(report_type: &str)`: добавить match-ветку
      `"requests_all" | "requests_open" | "requests_in_progress" | "requests_completed" => {
      vec!["number", "handover_date_utc", "request_type_label", "status_name", "giver_name",
      "location_name"] }` (порядок = порядок колонок из decision: №, Дата, Тип, Статус,
      Заявитель, Принтер / Локация).

   b. `column_labels_for(report_type: &str)`: та же группа ключей → `vec!["№", "Дата", "Тип",
      "Статус", "Заявитель", "Принтер / Локация"]` — индекс-в-индекс с шагом (a), иначе ломает
      D-03/CR-01 инвариант.

   c. `report_display_name(report_type: &str)`: добавить 4 ветки — `"requests_all" =>
      "Заявки"`, `"requests_open" => "Открытые заявки"`, `"requests_in_progress" => "Заявки в
      работе"`, `"requests_completed" => "Выполненные заявки"`.

   d. `PERIOD_BASED_REPORT_TYPES`: изменить тип с `[&str; 4]` на `[&str; 8]`, дописать
      `"requests_all", "requests_open", "requests_in_progress", "requests_completed"` в конец
      массива.

   e. `fetch_report`: добавить параметр `caller: &Identity` в сигнатуру (после `ctx: &AppCtx`).
      В теле, в самом начале, вычислить `let exclude_ad_register =
      trackly_core::auth::excludes_ad_register(&caller.role);` (используется только в новых
      ветках ниже — для остальных report_type это не проблема, Rust не ругается на переменную,
      используемую в части match-веток). Добавить 4 match-ветки:
      `"requests_all" => ctx.reports.list_requests_all(filter, require_period(report_type,
      period)?, exclude_ad_register).await,` — аналогично для `"requests_open"` →
      `list_requests_open`, `"requests_in_progress"` → `list_requests_in_progress`,
      `"requests_completed"` → `list_requests_completed`.

   f. Обновить ОБА существующих вызова `fetch_report(ctx, &report_type, filter, period.clone())` /
      `fetch_report(ctx, &report_type, filter, period)` внутри `build_reports_export_csv` и
      `build_reports_export_pdf` — добавить `caller` вторым аргументом:
      `fetch_report(ctx, caller, &report_type, filter, period)` (в обеих функциях `caller:
      &Identity` уже есть в скоупе — используется чуть выше для `authorize(...)`).

   g. `build_reports_get_report_counts`: перед вызовом `ctx.reports.get_report_counts(...)`
      вычислить `let exclude_ad_register =
      trackly_core::auth::excludes_ad_register(&caller.role);` и передать пятым аргументом:
      `ctx.reports.get_report_counts(&domain, filter, period, exclude_ad_register).await`.

   h. Добавить 4 build-хелпера (по образцу `build_reports_list_device_acts`):
      `build_reports_list_requests_all(ctx: &AppCtx, caller: &Identity, filter: ReportFilter,
      period: PeriodDto) -> Result<ReportResponse, AppError>` — `authorize(caller,
      &Action::ReadData)?;` затем `let exclude_ad_register =
      trackly_core::auth::excludes_ad_register(&caller.role); ctx.reports.list_requests_all(filter,
      period, exclude_ad_register).await` — и три аналогичных (`_open`/`_in_progress`/
      `_completed`).

   i. Добавить 4 `#[tauri::command] #[specta::specta]` враппера — по образцу
      `reports_list_device_acts`: `pub async fn reports_list_requests_all(state:
      tauri::State<'_, AppCtx>, filter: ReportFilter, period: PeriodDto) -> Result<ReportResponse,
      AppError>` — резолвит `caller` через `resolve_tauri_identity` и делегирует в
      `build_reports_list_requests_all` (и три аналогичных).

   j. В `#[cfg(test)] mod tests`, тест `column_labels_for_is_index_aligned_with_columns_for`:
      дописать в перечисляемый массив `report_type` строки `"requests_all"`, `"requests_open"`,
      `"requests_in_progress"`, `"requests_completed"`.

2. `crates/trackly-app/src/http/reports.rs`:

   a. Расширить `use crate::tauri_cmds::reports::{...}` — добавить `build_reports_list_requests_all,
      build_reports_list_requests_completed, build_reports_list_requests_in_progress,
      build_reports_list_requests_open` (алфавитный порядок, как остальной список импорта).

   b. Добавить 4 handler-функции — по образцу `handler_list_device_acts` (та же сигнатура,
      реюз существующего `ListWithPeriodPayload`, НЕ снапшот-паттерн): `handler_list_requests_all`,
      `handler_list_requests_open`, `handler_list_requests_in_progress`,
      `handler_list_requests_completed`.

   c. В `router()` добавить 4 маршрута: `.route("/api/v1/reports_list_requests_all",
      post(handler_list_requests_all))` и аналогично для `open`/`in_progress`/`completed`.

3. `crates/trackly-app/src/specta_export.rs`: в `collect_commands![...]` добавить 4 строки сразу
   после `crate::tauri_cmds::reports::reports_list_cartridge_in_stock,` (перед
   `reports_export_csv`): `crate::tauri_cmds::reports::reports_list_requests_all,` и аналогично для
   `_open`/`_in_progress`/`_completed`.

4. `crates/trackly-app/tests/reports_period_required.rs`: в тесте
   `period_based_exports_reject_missing_period`, в массиве `for report_type in [...]` дописать
   `"requests_all", "requests_open", "requests_in_progress", "requests_completed"` — эти 4 ключа
   должны так же отвергать `period: None` через `require_period`.

5. Создать НОВЫЙ файл `crates/trackly-app/tests/report_requests.rs`:

   - Скопировать 1:1 функцию `fn minimal_ctx() -> (AppCtx, TempDir)` (и все её импорты) из
     `crates/trackly-app/tests/reports_period_required.rs` — та же зависимость от `AppCtx.reports`.

   - Добавить seed-хелперы через `writer.execute(move |conn| {...})` (по образцу
     `request_lifecycle.rs::seed_user`/`seed_ad_register` — транзакция + `INSERT` +
     `tx.last_insert_rowid()` + `tx.commit()`), только с вымышленными данными (privacy-правило
     CLAUDE.md):
     - `seed_user(writer, login: &str, full_name: &str) -> i64` → `INSERT INTO users (login,
       full_name, role, ad_user, created_at_utc, updated_at_utc, version) VALUES (?1, ?2,
       'employee', 0, ?3, ?3, 1)`.
     - `seed_location(writer, name: &str) -> i64` → `INSERT INTO locations (name, created_at_utc,
       updated_at_utc, version) VALUES (?1, ?2, ?2, 1)`.
     - `seed_printer_device(writer, name: &str, location_id: i64) -> i64` → `INSERT INTO devices
       (type_id, name, location_id, status_id, created_at_utc, updated_at_utc, version) VALUES (2,
       ?1, ?2, 2, ?3, ?3, 1)` (type_id=2 = «Принтер», status_id=2 = «В работе», per
       migrations/V001__init_pragmas_and_lookups.sql lookup seed rows).
     - `seed_request(writer, request_type: &str, status: &str, requested_by: i64,
       printer_device_id: Option<i64>, created_at_utc: i64) -> i64` → `INSERT INTO requests
       (request_type, status, requested_by_user_id, printer_device_id, created_at_utc,
       updated_at_utc, version) VALUES (?1, ?2, ?3, ?4, ?5, ?5, 1)`.

   - Общая фикстура для всех тестов: seed один `Иванов И.И.` (`login: "us501"`), одну локацию
     `"Склад тест"`, один принтер `"Принтер HP LaserJet"` в этой локации, затем 4 заявки с
     ФИКСИРОВАННЫМ `created_at_utc = 1_780_300_000` (внутри уже юнит-тестируемых границ июня 2026
     Europe/Moscow — `1_780_261_200..=1_782_853_199`, см. `period_month_june_2026_moscow`):
     `cartridge_replace`/`open` (с принтером), `free_form`/`in_progress` (без принтера),
     `ad_register`/`completed` (без принтера), `cartridge_replace`/`rejected` (с принтером).
     Период для всех вызовов: `PeriodDto { mode: "month".to_string(), year: Some(2026), month:
     Some(6), date_from: None, date_to: None }`.

   - Тесты (все `#[tokio::test(flavor = "multi_thread", worker_threads = 2)]`, caller =
     `Identity::trusted_admin()`, кроме отдельного RBAC-теста):

     1. `report_requests_all_includes_every_status_translated_including_rejected` — вызвать
        `build_reports_list_requests_all(&ctx, &Identity::trusted_admin(), ReportFilter::default(),
        period.clone())`; assert `total == 4`; собрать множество `status_name` по строкам и
        assert равно `{"Открыта", "В работе", "Выполнена", "Отклонена"}`.

     2. `report_requests_open_filters_by_status_and_translates_type` — вызвать
        `build_reports_list_requests_open(...)`; assert `total == 1`; assert `rows[0].status_name
        == Some("Открыта".to_string())`, `rows[0].request_type_label == Some("Замена
        картриджа".to_string())`, `rows[0].giver_name == Some("Иванов И.И.".to_string())`,
        `rows[0].location_name == Some("Принтер HP LaserJet, Склад тест".to_string())`.

     3. `report_requests_printer_location_blank_when_no_printer` — вызвать
        `build_reports_list_requests_in_progress(...)` (это free_form-заявка без принтера);
        assert `rows[0].location_name.is_none()`.

     4. `report_requests_csv_export_uses_translated_values_not_raw_enum_keys` — вызвать
        `build_reports_export_csv(&ctx, &Identity::trusted_admin(), "requests_all".to_string(),
        ReportFilter::default(), Some(period.clone()))`; декодировать тело после 3-байтового BOM
        как UTF-8; assert содержит `"Замена картриджа"`, `"Открыта"`, `"Иванов И.И."`; assert НЕ
        содержит подстроку `"cartridge_replace"` (сырые enum-ключи не должны просачиваться в
        значения ячеек).

     5. `report_requests_status_counts_match_tab_keys` — вызвать
        `build_reports_get_report_counts(&ctx, &Identity::trusted_admin(), "requests".to_string(),
        ReportFilter::default(), period.clone())`; собрать `counts` в `HashMap<String, i64>` и
        assert `all == 4`, `open == 1`, `in_progress == 1`, `completed == 1`.

     6. `report_requests_manager_role_excludes_ad_register_admin_sees_all` — построить
        `Identity { user_id: Some(requester_id), role: trackly_core::auth::Role::Manager }`;
        вызвать `build_reports_list_requests_all` с этим caller'ом; assert `total == 3` и НИ одна
        строка не имеет `request_type_label == Some("Учётная запись AD".to_string())`; затем
        вызвать тот же метод с `Identity::trusted_admin()`; assert `total == 4` и РОВНО одна
        строка имеет `request_type_label == Some("Учётная запись AD".to_string())`.
  </action>
  <verify>
    <automated>cd /Users/madsas/Projects/trackly && grep -q "requests_all\" | \"requests_open\" | \"requests_in_progress\" | \"requests_completed" crates/trackly-app/src/tauri_cmds/reports.rs && echo OK_MATCH_ARM || echo FAIL_MATCH_ARM; grep -q "reports_list_requests_all" crates/trackly-app/src/specta_export.rs && echo OK_SPECTA || echo FAIL_SPECTA; grep -q "/api/v1/reports_list_requests_all" crates/trackly-app/src/http/reports.rs && echo OK_ROUTE || echo FAIL_ROUTE; grep -q "requests_all" crates/trackly-app/tests/reports_period_required.rs && echo OK_PERIOD_TEST || echo FAIL_PERIOD_TEST; grep -q "report_requests_manager_role_excludes_ad_register" crates/trackly-app/tests/report_requests.rs && echo OK_RBAC_TEST || echo FAIL_RBAC_TEST; cargo check -p trackly-app --all-targets 2>&1 | tail -150; cargo test -p trackly-app --lib tauri_cmds::reports::tests:: -- --test-threads=1 2>&1 | tail -60; TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test report_requests -- --test-threads=1 2>&1 | tail -150; TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test reports_period_required -- --test-threads=1 2>&1 | tail -80</automated>
  </verify>
  <done>columns_for/column_labels_for/report_display_name/PERIOD_BASED_REPORT_TYPES/fetch_report расширены 4 requests_*-ключами (индекс-выровнены); build_reports_list_requests_all/open/in_progress/completed + Tauri-команды + HTTP-хендлеры/роуты + specta-регистрация на месте; RBAC-исключение ad_register для Manager применяется во всех трёх входных точках (list/export/counts); новый report_requests.rs (6 тестов) и обновлённый reports_period_required.rs зелёные; cargo check -p trackly-app --all-targets чист.</done>
</task>

<task type="auto">
  <name>Task 3: Frontend — домен «Заявки» в ReportSubNav/ReportsPage, регенерация bindings, полный регресс</name>
  <files>ui/src/features/reports/ReportSubNav.svelte, ui/src/features/reports/ReportsPage.svelte, ui/src/features/reports/ReportFilters.svelte</files>
  <action>
Перечитать ui/src/features/reports/ReportSubNav.svelte и ui/src/features/reports/ReportsPage.svelte
перед правкой (текущее состояние уже прочитано на этапе планирования, номера строк могли
сдвинуться).

1. `ui/src/features/reports/ReportSubNav.svelte`:
   - Расширить `type DomainKey = 'devices' | 'cartridges' | 'requests';`.
   - Добавить константу `REQUEST_REPORTS: ReportConfig[]` рядом с `DEVICE_REPORTS`/
     `CARTRIDGE_REPORTS`: `{ key: 'all', label: 'Все', temporal: true, cmd:
     'reports_list_requests_all' }`, `{ key: 'open', label: 'Открытые', temporal: true, cmd:
     'reports_list_requests_open' }`, `{ key: 'in_progress', label: 'В работе', temporal: true,
     cmd: 'reports_list_requests_in_progress' }`, `{ key: 'completed', label: 'Выполненные',
     temporal: true, cmd: 'reports_list_requests_completed' }`.
   - В `DOMAINS` добавить `{ key: 'requests' as DomainKey, label: 'Заявки' }` третьим элементом.
   - `activeReports` `$derived`: заменить бинарный тернарник на `activeDomain === 'devices' ?
     DEVICE_REPORTS : activeDomain === 'cartridges' ? CARTRIDGE_REPORTS : REQUEST_REPORTS`.

2. `ui/src/features/reports/ReportsPage.svelte`:
   - Расширить локальный `type DomainKey = 'devices' | 'cartridges' | 'requests';`.
   - Добавить константу `REQUEST_REPORTS` — тот же состав, что в ReportSubNav.svelte (4 записи,
     `cmd` — `reports_list_requests_all/open/in_progress/completed`).
   - В локальный интерфейс `ReportRow` (в начале файла) добавить опциональное поле
     `request_type_label?: string | null;` — для параллели с бэкендовым ReportRow (не обязательно
     для рантайма из-за индекс-сигнатуры `[key: string]: unknown`, но сохраняет стиль остальных
     полей).
   - В `COLUMNS_MAP`: определить массив `const REQUEST_COLUMNS: Column[] = [{ key: 'number',
     label: '№' }, { key: 'handover_date_utc', label: 'Дата' }, { key: 'request_type_label',
     label: 'Тип' }, { key: 'status_name', label: 'Статус' }, { key: 'giver_name', label:
     'Заявитель' }, { key: 'location_name', label: 'Принтер / Локация' }];` и добавить в
     `COLUMNS_MAP` 4 записи, ссылающиеся на этот же массив: `all: REQUEST_COLUMNS, open:
     REQUEST_COLUMNS, in_progress: REQUEST_COLUMNS, completed: REQUEST_COLUMNS`.
   - `currentCmd()`: добавить ветку `if (activeDomain === 'requests') { const requestFound =
     REQUEST_REPORTS.find((r) => r.key === activeReport); if (requestFound) return
     requestFound.cmd; }` (рядом с существующими ветками для `devices`/`cartridges`).
   - `reportTypeKey()`: добавить `else if (activeDomain === 'requests') { switch (activeReport) {
     case 'all': return 'requests_all'; case 'open': return 'requests_open'; case 'in_progress':
     return 'requests_in_progress'; case 'completed': return 'requests_completed'; } }`.
   - `currentColumns()`: не требует изменений — существующий `return COLUMNS_MAP[activeReport] ??
     [];` уже покрывает ключи `all`/`open`/`in_progress`/`completed` без конфликта с существующими
     ключами (device/cartridge `in_use`/`in_stock` используют другие строки-ключи).
   - `isSnapshot()`: не требует изменений — оставить как есть (`['in_use',
     'in_stock'].includes(activeReport)`), но добавить однострочный комментарий: «для домена
     requests activeReport ∈ {all, open, in_progress, completed} — ни один ключ не совпадает с
     in_use/in_stock, поэтому isSnapshot() уже корректно возвращает false, PeriodSelector остаётся
     активным всегда (per decision)». Это осознанная проверка, а не пропуск.
   - В шаблоне: в `onDomainChange` callback (`<ReportSubNav ... onDomainChange={(d) => {...}}>`)
     изменить `activeReport = d === 'devices' ? 'acts' : 'consumption';` на трёхветочный
     тернарник: `activeReport = d === 'devices' ? 'acts' : d === 'cartridges' ? 'consumption' :
     'all';`.

3. `ui/src/features/reports/ReportFilters.svelte`: расширить тип пропа `reportDomain: 'devices' |
   'cartridges';` до `reportDomain: 'devices' | 'cartridges' | 'requests';` (проп принимается, но
   не используется для рендера — компонент уже показывает только кнопки экспорта; изменение нужно
   только чтобы TS-проверка типов приняла `activeDomain` в новом составе `DomainKey` без ошибки).

4. Пересобрать bindings и фронтенд: `pnpm --dir ui build` (внутренний `prebuild`-hook сам
   выполнит `cargo test -p trackly-app --test export_bindings`, перегенерирует
   `ui/src/bindings.ts` со свежими `reports_list_requests_*` командами — файл гитигнорится,
   отдельно коммитить не нужно).
  </action>
  <verify>
    <automated>cd /Users/madsas/Projects/trackly && grep -q "'requests'" ui/src/features/reports/ReportSubNav.svelte && echo OK_SUBNAV_DOMAIN || echo FAIL_SUBNAV_DOMAIN; grep -q "reports_list_requests_all" ui/src/features/reports/ReportSubNav.svelte && echo OK_SUBNAV_CMD || echo FAIL_SUBNAV_CMD; grep -q "requests_all" ui/src/features/reports/ReportsPage.svelte && echo OK_PAGE_KEY || echo FAIL_PAGE_KEY; grep -q "REQUEST_COLUMNS" ui/src/features/reports/ReportsPage.svelte && echo OK_COLUMNS || echo FAIL_COLUMNS; grep -q "'devices' | 'cartridges' | 'requests'" ui/src/features/reports/ReportFilters.svelte && echo OK_FILTERS_TYPE || echo FAIL_FILTERS_TYPE; pnpm --dir ui run svelte-check 2>&1 | tail -80; pnpm --dir ui build 2>&1 | tail -60; TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app -- --skip login_remember_persistent_cookie --test-threads=1 2>&1 | tail -250</automated>
  </verify>
  <done>Раздел «Отчёты» показывает домен «Заявки» с 4 вкладками (Все/Открытые/В работе/Выполненные), рабочим селектором периода (всегда активен), 6 колонками (№/Дата/Тип/Статус/Заявитель/Принтер-Локация) на экране; экспорт CSV и печать/PDF используют reportTypeKey() → requests_all/open/in_progress/completed; pnpm --dir ui run svelte-check и pnpm --dir ui build проходят без ошибок; полный прогон cargo test -p trackly-app (кроме известного зависающего login_remember_persistent_cookie) зелёный — существующие отчёты «Устройства»/«Картриджи» не сломаны.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|--------------|
| Admin/Manager (десктоп Tauri или LAN-браузер) → раздел «Отчёты» → домен «Заявки» | Все входные точки (`reports_list_requests_*`, `reports_export_csv`, `reports_export_pdf`, `reports_get_report_counts`) уже гейтятся `authorize(caller, &Action::ReadData)` (Admin\|Manager). Новая поверхность — 4 SQL-запроса к таблице `requests` (через `query_requests_inner`/`count_requests_inner`) и генерация CSV/HTML из ФИО заявителя + названия принтера/локации. |
| Внутренняя RBAC-граница между Admin и Manager для заявок типа `ad_register` | Существующий инвариант REQ-06/T-09-11: Manager не должен видеть заявки на регистрацию AD-аккаунта нигде в приложении. Этот план — первая точка, где заявки читаются ВНЕ `RequestService`/`DashboardService`, поэтому легко забыть про исключение. |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|------------------|
| T-vad-01 | Tampering (SQL injection) | `query_requests_inner`/`count_requests_inner` (report_service.rs) | mitigate | Все значения фильтра (`status_filter`, `ts_from`, `ts_to`) идут через параметризованные `?N`-плейсхолдеры и `Box<dyn ToSql>`, `format!` используется только для позиций плейсхолдеров — тот же паттерн, что уже верифицирован для `query_acts_inner` (T-07-03-01). |
| T-vad-02 | Information Disclosure | `reports_list_requests_*`/`reports_get_report_counts(domain="requests")` | mitigate | Доступ гейтится `authorize(ReadData)` (Admin\|Manager) — та же граница, что и у существующих доменов «Устройства»/«Картриджи»; никакого нового уровня доступа не вводится. |
| T-vad-03 | Information Disclosure (RBAC bypass на ad_register) | `query_requests_inner`/`count_requests_inner` exclude_ad_register-условие | mitigate | `exclude_ad_register: bool` вычисляется из `trackly_core::auth::excludes_ad_register(&caller.role)` в `tauri_cmds/reports.rs` (build_reports_list_requests_*, fetch_report, build_reports_get_report_counts) и применяется как `ad_register_predicate("r.")` в WHERE — та же функция, что уже используется `RequestService`/`DashboardService`, не дублируется вручную. Покрыто тестом `report_requests_manager_role_excludes_ad_register_admin_sees_all`. |
| T-vad-04 | Denial of Service | `query_requests_inner` | mitigate | SQL включает `LIMIT 1000` — тот же safeguard, что и у всех остальных report-запросов (T-07-03-04). |
| T-vad-05 | Tampering (CSV formula injection) | Значения колонок «Тип»/«Статус»/«Заявитель»/«Принтер / Локация» в CSV-экспорте | mitigate | Новый путь не вводит отдельный CSV writer — переиспользует существующий `ReportService::export_csv`/`csv_safe()` (T-07-03-05), который экранирует ячейки, начинающиеся с `=`/`+`/`-`/`@`, независимо от report_type. |
| T-vad-SC | Tampering (supply chain) | N/A | accept | Ни одна задача не добавляет новую зависимость и не запускает установку пакетов — только правки существующих `.rs`/`.svelte`-файлов и переиспользование уже присутствующих в кодовой базе функций (`ad_register_predicate`, `excludes_ad_register`, `export_csv`). Package Legitimacy Gate не применим. |
</threat_model>

<verification>
1. `cd /Users/madsas/Projects/trackly && cargo check -p trackly-app --all-targets` — 0 ошибок после
   каждой из задач 1 и 2 (весь crate, включая все `tests/*.rs`, компилируется с новым полем
   `ReportRow.request_type_label` и новыми сигнатурами `fetch_report`/`get_report_counts`).
2. `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test report_requests --
   --test-threads=1` — все 6 новых тестов зелёные (статус-фильтр, RU-перевод, пустая
   «Принтер / Локация» без принтера, per-tab счётчики, RBAC-исключение ad_register для Manager).
3. `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app -- --skip
   login_remember_persistent_cookie --test-threads=1` — полный прогон после Task 3 зелёный, в т.ч.
   без изменений: `report_acts`, `report_cartridges`, `report_csv_export`, `html_report_render`,
   `html_header_parity`, `reports_period_required` (регресс по существующим отчётам исключён).
4. `pnpm --dir ui run svelte-check` и `pnpm --dir ui build` — 0 ошибок после Task 3 (bindings.ts
   регенерируется автоматически через `prebuild`-hook).
5. Визуальная проверка — живое приложение (UAT), не синтетический харнесс (см. проектный урок
   «Synthetic harness not verification»): открыть «Отчёты» → домен «Заявки» → убедиться, что
   видны 4 вкладки с корректными счётчиками, переключение периода перезагружает список, колонки
   «Тип»/«Статус» на русском, «Принтер / Локация» пустая для заявок без принтера, CSV-экспорт и
   печать/предпросмотр показывают те же переведённые значения, что и экран; повторить то же самое
   для «Устройства»/«Картриджи», убедиться что они не изменились.
</verification>

<success_criteria>
- Домен «Заявки» в «Отчётах»: 4 вкладки (Все/Открытые/В работе/Выполненные), «Все» без фильтра по
  статусу (включая rejected), все периодические по `created_at_utc`, снимков нет.
- Одинаковый набор из 6 колонок (№, Дата, Тип, Статус, Заявитель, Принтер / Локация) на экране, в
  CSV и в печати, для всех четырёх вкладок.
- «Тип»/«Статус» переведены на русский идентично на всех трёх выходах, включая `cancelled` →
  «Отменена»; действительно неизвестное (не входящее в CHECK-схему) значение выводится как
  raw-ключ, не пустой ячейкой.
- Заявка без принтера — «Принтер / Локация» пустая, не тире.
- Manager не видит заявки `ad_register` в отчёте (ни в строках, ни в счётчиках); Admin видит все.
- И Tauri-команды, и `/api/v1/reports_list_requests_*` работают одинаково.
- Существующие отчёты «Устройства»/«Картриджи» не регрессировали — полный набор существующих
  report-тестов зелёный.
- `cargo check -p trackly-app --all-targets`, `pnpm --dir ui run svelte-check`, `pnpm --dir ui
  build` проходят чисто.
</success_criteria>

<output>
Create `.planning/quick/260820-vad-csv-pdf/260820-vad-SUMMARY.md` when done
</output>
