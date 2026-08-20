# Quick Task 260820-vad: Домен «Заявки» в разделе Отчёты - Context

**Gathered:** 2026-08-20
**Status:** Ready for planning

<domain>
## Task Boundary

Добавить третий домен «Заявки» в раздел «Отчёты» (сейчас там только «Устройства» и «Картриджи»)
с просмотром на экране, экспортом CSV и печатью/PDF — по тому же паттерну, что уже работает
для существующих доменов.

**В границах:**
- Бэкенд: новые `list_*` / `count_*` методы по таблице `requests` в `ReportService`,
  новые build_*-хелперы + Tauri-команды + зеркальные HTTP-роуты, регистрация в specta.
- DTO: расширение `ReportRow` новыми `Option`-полями под заявки.
- Фронтенд: третий домен в `ReportSubNav`, конфиг вкладок/колонок в `ReportsPage.svelte`.
- Тесты по образцу существующих `report_*.rs`.

**Вне границ:**
- Любые изменения поведения отчётов по устройствам и картриджам (регресс недопустим).
- Новый код печати — печать/PDF уже идёт через `PdfPreviewModal mode="report"` →
  `reports_export_pdf`, ей достаточно новых report_type-ключей.
- Раздел «Заявки» как таковой (список, формы, переходы статусов) — не трогаем.

</domain>

<decisions>
## Implementation Decisions

### Состав вкладок домена «Заявки»
- Четыре вкладки, разрез по статусу: **Все / Открытые / В работе / Выполненные**.
- «Все» — без фильтра по статусу, то есть включает и `rejected` (отдельной вкладки
  «Отклонённые» не делаем).
- Маппинг на `requests.status`: Открытые → `open`, В работе → `in_progress`,
  Выполненные → `completed`.
- Ключи report_type для CSV/PDF: `requests_all`, `requests_open`, `requests_in_progress`,
  `requests_completed` (единообразно с `device_acts` / `cartridge_consumption`).

### Периодичность отчётов
- **Все четыре вкладки — периодические** (period-based), отбор по дате создания заявки
  (`requests.created_at_utc`). Селектор периода активен всегда, снимков (snapshot) в этом
  домене нет.
- Следствие: все четыре ключа добавляются в `PERIOD_BASED_REPORT_TYPES`, а на фронте
  `isSnapshot()` для домена `requests` всегда false (сейчас функция смотрит только на
  `activeReport ∈ {in_use, in_stock}` — проверить, что домен учтён).

### Набор колонок (экран + CSV + печать)
- `№` (id заявки), `Дата` (создания), `Тип`, `Статус`, `Заявитель`, `Принтер / Локация`.
- Описание, категорию, исполнителя и дату закрытия в таблицу НЕ выносим — свободный текст
  ломает вёрстку печатной формы.
- Колонка «Принтер / Локация» — это `printer_name` + `printer_location` (для заявок без
  принтера — пусто, а не «—»-заглушка на бэкенде; отображение решает фронт).
- Набор колонок одинаков для всех четырёх вкладок.

### Локализация значений
- Тип и статус переводятся на русский **везде: экран, CSV, печать**.
  - Тип: `cartridge_replace` → «Замена картриджа», `free_form` → «Произвольная»,
    `ad_register` → «Учётная запись AD».
  - Статус: `open` → «Открыта», `in_progress` → «В работе», `completed` → «Выполнена»,
    `rejected` → «Отклонена», `cancelled` → «Отменена» (пятый статус из CHECK-констрейнта
    `requests.status`, V031 — реально используется `RequestService::cancel()`/самоотменой;
    уже подписан как «Отменена» в `RequestListRow.svelte`/`RequestDetail.svelte` и есть
    отдельная вкладка-фильтр `cancelled` в `RequestsSearchAndTabs.svelte` — отчёт обязан
    показывать тот же перевод, а не английский raw-ключ).
- Перевод выполняется на бэкенде (в SQL/маппинге строк), чтобы экран, CSV и печать
  гарантированно совпадали и не разъезжались, как это было с заголовками колонок (D-03/CR-01).
- Неизвестное (не входящее в текущую CHECK-схему) значение статуса/типа не должно приводить к
  пустой ячейке — выводить исходный ключ как fallback.

### Claude's Discretion
- Точная форма расширения `ReportRow` (какие именно новые `Option`-поля и их имена).
- Нужен ли `month_key`-группировщик для заявок (как в отчётах по картриджам) — на усмотрение
  исполнителя, исходя из единообразия печатной формы.
- Структура SQL-запроса (один параметризованный `query_requests_inner` со статус-фильтром
  против четырёх отдельных функций) — предпочтительно один общий, по образцу
  `query_acts_inner(..., act_type)`.
- Формулировки `report_display_name()` для заголовка печатной формы.

</decisions>

<specifics>
## Specific Ideas

Существующие точки, которые нужно расширить (проверены в коде на момент планирования):

- `crates/trackly-app/src/services/report_service.rs` — `list_*` (spawn_blocking + `query_*_inner`),
  `count_*_inner`, `get_report_counts(domain)` (сейчас знает только `"devices"` / `"cartridges"`,
  для прочих доменов возвращает пустой `Vec` → бейджи вкладок будут нулевые, если не добавить ветку),
  `export_csv`, `export_pdf`.
- `crates/trackly-app/src/tauri_cmds/reports.rs` — `columns_for()`, `column_labels_for()`
  (index-aligned с `columns_for`, ЛОМАЕТСЯ при рассинхроне), `report_display_name()`,
  `fetch_report()`, `PERIOD_BASED_REPORT_TYPES`.
- `crates/trackly-app/src/http/reports.rs` — зеркальные `/api/v1/reports_list_*` роуты
  (LAN-режим обязан работать наравне с десктопом).
- `crates/trackly-app/src/specta_export.rs` — регистрация новых команд; после этого
  перегенерировать bindings для UI.
- `crates/trackly-app/src/dto/reports.rs` — `ReportRow` (разреженная структура, добавляем
  `Option`-поля).
- `ui/src/features/reports/ReportsPage.svelte` — `DEVICE_REPORTS`/`CARTRIDGE_REPORTS`,
  `COLUMNS_MAP`, `isSnapshot()`, `currentCmd()`, `reportTypeKey()`, `currentColumns()`,
  `loadStatusCounts()`.
- `ui/src/features/reports/ReportSubNav.svelte` — `DOMAINS` + списки отчётов
  (дублируют конфиг из ReportsPage — обновлять оба, иначе вкладки разъедутся).
- Данные: `crates/trackly-core/src/domain/requests.rs`, `crates/trackly-app/src/dto/request.rs`
  (статусы `open|in_progress|completed|rejected`, типы `cartridge_replace|free_form|ad_register`,
  джойны `requester_name`, `printer_name`, `printer_location`, `category_name`).

Тесты по образцу: `crates/trackly-app/tests/report_acts.rs`, `report_cartridges.rs`,
`report_csv_export.rs`, `html_report_render.rs`, `reports_period_required.rs`.

Проверка после изменений фронтенда: для LAN/серверного режима нужен `pnpm --dir ui build`
(десктопный `cargo tauri dev` HMR-ит только свою webview).

</specifics>

<canonical_refs>
## Canonical References

- `CLAUDE.md` → **Приватность данных (жёсткое условие)**: репозиторий публичный. В коде, тестах,
  фикстурах и артефактах `.planning/` — только вымышленные данные («Иванов И.И.», «Петров П.П.»),
  никаких реальных ФИО, названия организации и её реквизитов. Проверять ДО коммита.
- `CLAUDE.md` → UI и печатные формы в v1 — только русский язык.
- `CLAUDE.md` → dual access path: Tauri-команды и axum-хендлеры — тонкие адаптеры над общим
  сервисным слоем; бизнес-логика не дублируется.

</canonical_refs>
