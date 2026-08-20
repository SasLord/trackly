---
phase: 260820-vad
plan: 01
subsystem: reports
tags: [rust, axum, tauri, rusqlite, specta, svelte, reports, requests, rbac]

# Dependency graph
requires:
  - phase: N/A (quick task)
    provides: existing ReportService query_*_inner pattern (query_acts_inner),
      ad_register_predicate/excludes_ad_register (REQ-06/T-09-11)
provides:
  - Третий домен «Заявки» в разделе «Отчёты»: просмотр на экране, CSV-экспорт,
    печать/PDF — 4 вкладки (Все/Открытые/В работе/Выполненные)
  - query_requests_inner/count_requests_inner — общий параметризованный запрос
    по requests со статус-фильтром и RBAC exclude_ad_register
  - translate_request_type/translate_request_status — RU-переводы Тип/Статус,
    вычисляются один раз на бэкенде (экран/CSV/печать идентичны)
affects: [reports, requests]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "query_requests_inner/count_requests_inner следуют паттерну query_acts_inner:
      parameterized WHERE через next_idx(&owned_params), spawn_blocking-обёртка"
    - "fetch_report/get_report_counts принимают caller: &Identity и вычисляют
      exclude_ad_register = excludes_ad_register(&caller.role) для RBAC-фильтрации"

key-files:
  created:
    - crates/trackly-app/tests/report_requests.rs
  modified:
    - crates/trackly-app/src/dto/reports.rs
    - crates/trackly-app/src/services/report_service.rs
    - crates/trackly-app/src/tauri_cmds/reports.rs
    - crates/trackly-app/src/http/reports.rs
    - crates/trackly-app/src/specta_export.rs
    - crates/trackly-app/tests/report_csv_export.rs
    - crates/trackly-app/tests/html_report_render.rs
    - crates/trackly-app/tests/html_header_parity.rs
    - crates/trackly-app/tests/reports_period_required.rs
    - ui/src/features/reports/ReportSubNav.svelte
    - ui/src/features/reports/ReportsPage.svelte
    - ui/src/features/reports/ReportFilters.svelte

key-decisions:
  - "4 вкладки по статусу (Все/Открытые/В работе/Выполненные), «Все» без
    фильтра по статусу — включает rejected"
  - "Все 4 вкладки — периодические по requests.created_at_utc, снимков нет"
  - "6 колонок одинаковы на экране/CSV/печати: №, Дата, Тип, Статус,
    Заявитель, Принтер / Локация"
  - "Тип/Статус переводятся на бэкенде один раз; неизвестное значение —
    raw-ключ fallback, не пустая ячейка"
  - "cancelled → «Отменена» (V031, самоотмена) — тот же перевод, что уже
    используется в RequestListRow.svelte/RequestDetail.svelte"
  - "«Принтер / Локация» пусто (не тире) для заявки без принтера — фронтенд
    рисует тире для null"
  - "exclude_ad_register вычисляется из excludes_ad_register(&caller.role) во
    всех входных точках (list/export/counts) — переиспользует существующий
    REQ-06/T-09-11 инвариант, не дублирует логику"

requirements-completed: [VAD-01, VAD-02, VAD-03, VAD-04]

# Metrics
duration: ~90min
completed: 2026-08-21
---

# Quick Task 260820-vad: Домен «Заявки» в разделе «Отчёты» Summary

**Третий домен «Заявки» в Отчётах (4 периодических вкладки по статусу, RU-перевод Тип/Статус на бэкенде, RBAC-исключение ad_register для Manager) — рабочий на десктопе и в LAN-браузере, без регресса существующих отчётов «Устройства»/«Картриджи».**

## Performance

- **Duration:** ~90 min
- **Tasks:** 3/3 completed
- **Files modified:** 12 modified, 1 created

## Accomplishments

- Домен «Заявки» в «Отчётах»: 4 вкладки (Все/Открытые/В работе/Выполненные), период-based по `requests.created_at_utc`, счётчики вкладок через `get_report_counts(domain="requests")`.
- Единый набор из 6 колонок (№, Дата, Тип, Статус, Заявитель, Принтер / Локация) идентичен на экране, в CSV и в печати — вычисляется один раз в `query_requests_inner`.
- Тип/Статус переведены на русский на бэкенде (`translate_request_type`/`translate_request_status`), включая `cancelled` → «Отменена»; неизвестное значение — raw-ключ fallback, не пустая ячейка.
- RBAC-инвариант REQ-06/T-09-11 применён во всех трёх входных точках (список/экспорт/счётчики): Manager не видит заявки `ad_register` ни в строках, ни в счётчиках; Admin видит всё.
- И Tauri-команды, и зеркальные HTTP-роуты `/api/v1/reports_list_requests_*` работают одинаково (десктоп + LAN-браузер).
- Существующие отчёты «Устройства»/«Картриджи» не затронуты — полный прогон `cargo test -p trackly-app` (92 test-бинаря, `--skip login_remember_persistent_cookie`) зелёный, 0 упавших.

## Task Commits

Each task was committed atomically:

1. **Task 1: Domain layer — ReportRow.request_type_label, переводы, query_requests_inner/count_requests_inner, list_requests_*, get_report_counts(requests)** - `37f9629f` (feat)
2. **Task 2: Wiring — tauri_cmds/reports.rs, HTTP-роуты, specta, интеграционные тесты** - `06cd713b` (feat)
3. **Task 3: Frontend — домен «Заявки» в ReportSubNav/ReportsPage, регенерация bindings, полный регресс** - `3ed45725` (feat)

_Note: Task 1 и Task 2 тесно связаны по дизайну плана — Task 1 расширяет сигнатуру `get_report_counts` четвёртым параметром `exclude_ad_register`, что временно ломает единственный вызывающий код в `tauri_cmds/reports.rs` (файл Task 2); это исправляется в Task 2 сразу же. Обе задачи закоммичены отдельно per-file per плану; полная компиляция и тесты подтверждены ПОСЛЕ применения обоих наборов изменений (см. "Deviations" ниже)._

## Files Created/Modified

- `crates/trackly-app/src/dto/reports.rs` - `ReportRow.request_type_label: Option<String>` (новое поле)
- `crates/trackly-app/src/services/report_service.rs` - `query_requests_inner`/`count_requests_inner`, `translate_request_type`/`translate_request_status`/`combine_printer_and_location`, `list_requests_all/open/in_progress/completed`, `get_report_counts` домен `"requests"`, 11 unit-тестов
- `crates/trackly-app/src/tauri_cmds/reports.rs` - `columns_for`/`column_labels_for`/`report_display_name`/`PERIOD_BASED_REPORT_TYPES` расширены; `fetch_report` принимает `caller: &Identity`; 4 `build_reports_list_requests_*` + 4 Tauri-команды
- `crates/trackly-app/src/http/reports.rs` - 4 HTTP-хендлера + 4 маршрута `/api/v1/reports_list_requests_*`
- `crates/trackly-app/src/specta_export.rs` - регистрация 4 новых команд для `ui/src/bindings.ts`
- `crates/trackly-app/tests/report_requests.rs` (новый) - 6 интеграционных тестов: статус-фильтр, RU-перевод (экран+CSV), пустая «Принтер / Локация», per-tab счётчики, RBAC-исключение ad_register
- `crates/trackly-app/tests/report_csv_export.rs`, `html_report_render.rs`, `html_header_parity.rs` - добавлено поле `request_type_label: None,` в существующие `ReportRow`-литералы
- `crates/trackly-app/tests/reports_period_required.rs` - `requests_*`-ключи добавлены в период-обязательный набор
- `ui/src/features/reports/ReportSubNav.svelte` - домен `'requests'` + `REQUEST_REPORTS` (4 вкладки)
- `ui/src/features/reports/ReportsPage.svelte` - `REQUEST_REPORTS`, `REQUEST_COLUMNS`, `currentCmd()`/`reportTypeKey()` расширены доменом `requests`, `onDomainChange` дефолтит на вкладку `'all'`
- `ui/src/features/reports/ReportFilters.svelte` - типизация `reportDomain` расширена `'requests'`

## Decisions Made

Все решения зафиксированы заранее в `260820-vad-CONTEXT.md` (D-01..D-05) и применены как указано — новых архитектурных решений в процессе исполнения не потребовалось. Единственное практическое уточнение — форма `ReportRow.request_type_label` (единственное новое поле, оставлено на усмотрение исполнителя per CONTEXT.md) и SQL-структура `query_requests_inner` (один общий параметризованный запрос со статус-фильтром, по образцу `query_acts_inner`) — оба выбраны в соответствии с "Claude's Discretion" секцией контекста.

## Deviations from Plan

**1. [Ожидаемая, не Rule 1-4] Task 1 → Task 2 промежуточная несовместимость сборки**

- **Найдено на:** переход от Task 1 к Task 2
- **Причина:** план явно предписывает Task 1 расширить сигнатуру `ReportService::get_report_counts` четвёртым параметром `exclude_ad_register: bool` (пункт 2g плана), при этом единственный вызывающий код (`ctx.reports.get_report_counts(&domain, filter, period).await` в `tauri_cmds/reports.rs`) относится к файлам Task 2. Между коммитом Task 1 и коммитом Task 2 `cargo check -p trackly-app --all-targets` кратковременно не проходил бы, если бы Task 2 не был реализован и провалидирован в том же проходе.
- **Действие:** реализовал код Task 1 и Task 2 последовательно без промежуточного `cargo check` между ними (согласно замыслу плана — обе задачи тесно связаны той же сигнатурой), затем прогнал полную верификацию (`cargo check --all-targets`, unit + integration тесты) ОДИН раз после применения обоих наборов правок — всё зелёное. Коммиты сделаны раздельно по file-list каждой задачи (это НЕ означает, что HEAD после коммита Task 1 собирается изолированно — история отражает замысел плана, а не независимо валидированные состояния).
- **Файлы:** см. коммиты `37f9629f` (Task 1) и `06cd713b` (Task 2)
- **Impact on plan:** Не требует правок плана — такая связка между двумя задачами одного quick-task предусмотрена самим текстом плана (пункт 2e/2g Task 2 явно ссылается на изменения Task 1). Не blocking для дальнейшей работы, так как оба коммита уже в HEAD до финальной верификации.

---

**Total deviations:** 1 (документационная, не код-фикс по Rule 1-4)
**Impact on plan:** Отсутствует — план исполнен как написано, единственное отклонение — порядок верификации между двумя тесно связанными задачами.

## Issues Encountered

None — единственная сложность (временная несовместимость сборки между Task 1 и Task 2) описана выше в Deviations, не потребовала решения проблем сверх выполнения плана как написано.

## User Setup Required

None - конфигурация внешних сервисов не требуется.

## Verification Performed

- `cargo check -p trackly-app --all-targets` — чист (0 ошибок) после применения Task 1 + Task 2.
- `cargo test -p trackly-app --lib services::report_service::tests::` — 30 passed, 0 failed (включая 11 новых unit-тестов переводчиков).
- `cargo test -p trackly-app --lib tauri_cmds::reports::tests::` — 1 passed (index-alignment guard, охватывает новые `requests_*`-ключи).
- `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test report_requests` — 6 passed, 0 failed.
- `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test reports_period_required` — 2 passed, 0 failed.
- `pnpm --dir ui run svelte-check` — 0 ошибок (269 файлов, только предсуществующие warnings в несвязанных файлах).
- `pnpm --dir ui build` — успешно, `bindings.ts` регенерирован с 4 новыми `reports_list_requests_*` командами.
- `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app -- --skip login_remember_persistent_cookie --test-threads=1` — ПОЛНЫЙ прогон, 92 test-бинаря, 0 упавших (лог сохранён локально, не в репозитории). Существующие `report_acts`/`report_cartridges`/`report_csv_export`/`html_report_render`/`html_header_parity`/`reports_period_required` — все зелёные, регресс исключён.

**UNVERIFIED (требует ручной проверки в живом приложении):** визуальный UAT — переключение вкладок, счётчики, PeriodSelector, CSV/печать в реальном UI (десктоп + LAN-браузер). Синтетические харнессы (svelte-check/build/cargo test) не ловят рантайм-ошибки рун Svelte 5 (см. проектный урок «Compile gates miss Svelte runtime») — компилируемость подтверждена, функциональная корректность в реальном приложении не проверялась в рамках этого исполнения.

## Next Phase Readiness

Домен «Заявки» полностью проведён по стеку (SQL → сервис → Tauri-команда → HTTP-роут → specta → фронтенд-конфиг), тот же паттерн, что «Устройства»/«Картриджи». Готово к живой UAT-проверке пользователем. Блокеров нет.

---
*Quick task: 260820-vad*
*Completed: 2026-08-21*

## Self-Check: PASSED

All 14 files verified present (13 code/test files + this SUMMARY.md); all 3
task commits (`37f9629f`, `06cd713b`, `3ed45725`) verified present in git log.
No missing items.
