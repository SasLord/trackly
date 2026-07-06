# Phase 17: html-krilla - Context

**Gathered:** 2026-07-06
**Status:** Ready for planning

<domain>
## Phase Boundary

Экспорт Отчётов и редактор Шаблонов переходят с krilla/DocSpec на HTML-печать по паттерну Phase 16 (акты): `ReportService::export_pdf` возвращает HTML-строку, печать/сохранение идёт через диалог браузера в превью-модалке (desktop + LAN), редактор Шаблонов правит HTML-файлы в `templates/`, а `krilla`/`DocSpec` полностью выведены из активного пути (заморожены, не удалены). Закрывает отложенные пункты 16-HUMAN-UAT 2a (миграция Отчётов) и 2b (баг `reports_export_pdf`).

Обсуждение уточняло **только КАК** реализовать — требования зафиксированы SPEC.md.

</domain>

<spec_lock>
## Requirements (locked via SPEC.md)

**7 requirements are locked.** См. `17-SPEC.md` для полного текста требований, границ и критериев приёмки.

Downstream-агенты ОБЯЗАНЫ прочитать `17-SPEC.md` перед планированием/реализацией. Требования здесь не дублируются.

**In scope (from SPEC.md):**
- Миграция `ReportService::export_pdf` на HTML-строку
- Новый редактируемый `templates/report.html` (embedded default + материализация + fallback)
- Обновление Tauri/HTTP-адаптеров экспорта Отчёта на `text/html` + перегенерация `bindings.ts`
- Печать Отчётов через превью-модалку (srcdoc + `window.print()`) в desktop и LAN
- Перенацеливание редактора Шаблонов на HTML-файлы `templates/*.html` (загрузка/сохранение на диск/сброс/HTML-превью), включая `report.html`
- Обновление панели «Доступные переменные» под HTML-контекст
- Заморозка krilla/DocSpec в активном пути (Отчёты, Шаблоны, health); `#[ignore]` для krilla-тестов
- Миграция тестов Отчётов/Шаблонов с PDF-байтов на HTML-ассерты
- Закрытие отложенных пунктов 16-HUMAN-UAT 2a и 2b

**Out of scope (from SPEC.md):**
- Полное удаление crate `krilla` и файлов `pdf/{renderer,docspec,fonts}.rs` — заморозить, не удалять
- CSV-экспорт Отчётов — не трогаем
- Новые виды отчётов или новые поля/колонки — только миграция движка рендера
- Богатый WYSIWYG-редактор (DOC-01), импорт/экспорт .docx (DOC-02), прочие печатные документы (DOC-03) — вне milestone
- Изменение layout/фиделити актов — сделано в Phase 15/16

</spec_lock>

<decisions>
## Implementation Decisions

### Структура report.html (Requirement 1, 2)
- **D-01:** Layout = **новый чистый дизайн таблицы**, НЕ прямой порт DocSpec-вида. Чистый современный вид: zebra-строки, лёгкие границы, выделенный `<thead>`, помесячные заголовки-разделители; аккуратная печать на A4. (Это стилистическая переработка того же набора данных — новых колонок/полей НЕ добавляется, границы SPEC соблюдены.)
- **D-02:** Шапка организации **переиспользуется как в актах** — тот же 2-колоночный блок (логотип data-URI + полные реквизиты: name/inn/kpp/address/phone/fax/email/okpo/ogrn) из `act_handover.html`. Единообразие документов.
- **D-03:** Русские подписи колонок **передаёт Rust** как данные (готовые метки в списке `columns`), шаблон генеричен и просто итерирует колонки. Один `report.html` обслуживает все типы отчётов; пользователь правит вёрстку/стили, но НЕ метки колонок.

### Контекст / переменные report.html (Requirement 2)
- **D-04:** Группировка по месяцам — **на стороне Rust**. `export_pdf` строит структуру `groups = [{ month_label: "Сентябрь 2026", rows: [...] }]`; шаблон итерирует группы и строки. Логика группировки тестируема в Rust, шаблон простой.
- **D-05:** Строки передаются как **списки ячеек по порядку колонок** (row = list of готовых строк-значений, выровненных по `columns`); шаблон делает `{% for cell in row %}`. Генерично для любого набора колонок.
- **D-06:** **Формат дат НЕ трогаем** — сохраняется текущее поведение `row_field` (то же, что использует CSV-экспорт сегодня). Не расширяем скоуп форматированием дат.
- **D-07:** Пустой набор → «Нет данных за указанный период.» (уже в SPEC Req 1, сохранить).
- **D-08:** Autoescape ON через `build_safe_html_env`; исключение `| safe` только для `org.logo_data_uri` (как в актах, D-11 Phase 16).

### Превью-модалка Отчётов (Requirement 4)
- **D-09:** **Расширить существующий `PdfPreviewModal`** новым `mode='report'`. Модалка сама вызывает `reports_export_pdf` по переданным параметрам (`reportType` / `filter` / `period`) и владеет состояниями loading/error — зеркалит текущий self-fetch паттерн `mode='handover'`/`'acceptance'`. НЕ отдельный компонент.
- **D-10:** Кнопку «Скачать PDF» в Отчётах **полностью заменить печатью**. Кнопка открывает превью+печать; сохранение в PDF — через нативный диалог печати браузера («Сохранить как PDF»). CSV-экспорт остаётся без изменений. Удаляется старый blob/tauri-plugin-fs download-путь в `ReportsPage.exportPdf()`.

### Миграция редактора Шаблонов (Requirement 5)
- **D-11:** Превью в редакторе рендерится на **встроенных sample-данных** (демо-акт / демо-отчёт, фиксированные значения). Детерминированно, работает на пустой БД, без выбора реальной записи.
- **D-12:** Панель «Доступные переменные» показывает переменные **только текущего выбранного шаблона** (`act_handover` / `act_acceptance` / `report`) — каждый со своим контекстом. Без общего смешанного списка.
- **D-13:** Старый DB-путь шаблонов (`document_templates` + `seed_defaults_on_startup` + DocSpec-MiniJinja `DEFAULT_TEMPLATES`) **заморозить, не трогать** — таблица и seed остаются (безвредны), редактор просто больше на них не указывает. Минимум изменений, зеркалит krilla-заморозку. НЕ удалять seed и НЕ отключать его.

### Claude's Discretion (технические детали — planner/executor решает)
- Имена команд: переиспользовать существующие `templates_*` (list_for_editor / update_body / reset_to_default / validate_preview), перенацелив внутренности на файлы `templates/*.html`, — минимизирует churn `bindings.ts`. Возврат `validate_preview` меняется на HTML-строку; `update_body`/`reset_to_default` принимают идентификатор шаблона (kind → filename) и пишут/восстанавливают файл на диске. Точные сигнатуры — за planner.
- Маппинг kind → файл: `act_handover.html` / `act_acceptance.html` / `report.html`.
- `report.html` добавить в `DEFAULT_HTML_TEMPLATES` (`html_templates.rs`) и материализовать на старте тем же механизмом, что акты (`materialize_defaults_on_startup`).
- Точная структура sample-данных для editor-превью — за executor (должна покрывать все переменные каждого шаблона, включая мультиустройство для актов и многомесячный отчёт).
- Конкретный CSS нового дизайна таблицы (zebra/границы/шрифты, `@page A4`) — за executor в рамках «чистого современного вида».

</decisions>

<canonical_refs>
## Canonical References

**Downstream-агенты ОБЯЗАНЫ прочитать эти файлы перед планированием/реализацией.**

### Locked requirements
- `.planning/phases/17-html-krilla/17-SPEC.md` — 7 зафиксированных требований, границы, критерии приёмки. MUST read before planning.

### Эталонный паттерн Phase 16 (акты HTML-печать)
- `crates/trackly-app/src/pdf/html_templates.rs` — file-first + embedded fallback loader: `DEFAULT_HTML_TEMPLATES`, `resolve_templates_dir` (`TRACKLY_TEMPLATES_DIR` override), `materialize_defaults_on_startup` (idempotent insert-only), `load_template` (read-on-render + fallback). `report.html` добавляется сюда.
- `crates/trackly-app/templates/act_handover.html` — образец self-contained HTML-шаблона: inline `<style>`, `@page A4`, 2-колоночная шапка организации с `org.logo_data_uri | safe`, autoescape-safe `{{ var }}`. Шапка отчёта копируется отсюда (D-02).
- `crates/trackly-app/templates/act_acceptance.html` — второй act-шаблон (тоже редактируется новым редактором).
- `ui/src/features/acts/PdfPreviewModal.svelte` — превью-модалка (srcdoc + `window.print()`), `mode='handover'|'acceptance'`, `renderCall()` self-fetch, `printViaSystemBrowser` (desktop/LAN) + `printViaTopLevel`. Расширяется `mode='report'` (D-09).
- `ui/src/features/acts/ActsPage.svelte` §252 — пример вызова `PdfPreviewModal`.

### Файлы для миграции (Отчёты)
- `crates/trackly-app/src/services/report_service.rs` §512 `export_pdf` — текущий DocSpec+krilla путь, мигрируется на HTML-строку (D-01/D-03/D-04/D-05); `row_field` helper — источник значений ячеек (D-06).
- `crates/trackly-app/src/tauri_cmds/reports.rs` §345 `reports_export_pdf` — Tauri-адаптер, возврат → HTML-строка.
- `crates/trackly-app/src/http/reports.rs` §203 `handler_export_pdf` — HTTP-адаптер, `Content-Type: text/html`.
- `ui/src/features/reports/ReportsPage.svelte` §380 `exportPdf()` — убрать blob/download, открыть превью-модалку (D-10); CSV-путь не трогать.

### Файлы для миграции (редактор Шаблонов)
- `crates/trackly-app/src/services/template_service.rs` — `validate_preview` §295 (krilla-превью → HTML), `list_all_for_editor` §145, `update_body` §179, `reset_to_default` §222; `DEFAULT_TEMPLATES`/`seed_defaults_on_startup` §43/§88 — ЗАМОРОЗИТЬ, не трогать (D-13).
- `ui/src/features/settings/TemplateEditor.svelte` — kind-select §164, панель переменных §173, save/reset/preview §114/§136/§90; перенацелить на HTML-файлы (D-11/D-12).

### krilla-заморозка (эталон Phase 16 D-13)
- `crates/trackly-app/src/pdf/{renderer,docspec,fonts}.rs` — остаются скомпилированными, но неиспользуемыми; krilla-тесты помечаются `#[ignore]`.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `html_templates::{load_template, materialize_defaults_on_startup, resolve_templates_dir, DEFAULT_HTML_TEMPLATES}` — весь механизм report.html переиспользует эту инфраструктуру Phase 16 без изменений (только добавить кортеж `("report.html", include_str!(...))`).
- `build_safe_html_env` — autoescape-окружение MiniJinja, используется актами; report.html и editor-превью рендерятся через него.
- `PdfPreviewModal.svelte` — вся печатная логика (`printViaSystemBrowser` для desktop/LAN, `printViaTopLevel`, srcdoc-iframe) переиспользуется; добавляется третья ветка `mode='report'` в `renderCall()`.
- Шапка организации из `act_handover.html` (блок `.header` + `.requisites` + логотип) копируется в `report.html`.
- `OrganizationService::read_logo_bytes` (Phase 16, 16-02) — источник байтов логотипа для data-URI.
- `row_field` в `report_service.rs` — маппинг ключ-колонки → значение строки, сохраняется как источник ячеек.

### Established Patterns
- **file-first + embedded fallback + materialize-on-startup** (Phase 16 D-05..D-08): файл читается свежим с диска на каждый рендер, отсутствие файла → embedded default, генерация никогда не падает из-за удалённого файла.
- **portable write path**: материализация и запись правок — через `Paths::templates_dir()` (`<exe_dir>/templates`), не `%APPDATA%`; dev/test override `TRACKLY_TEMPLATES_DIR`.
- **self-fetch modal**: `PdfPreviewModal` сам вызывает render-команду по `mode`+props, владеет loading/error — новый `mode='report'` следует ему.
- **krilla freeze**: `#[ignore]` для krilla-тестов, код не удаляется, dep остаётся в `Cargo.toml` (Phase 16 D-13).
- **bindings.ts gitignored** — регенерируется через `cargo test --test export_bindings`, не коммитится (Phase 16, 16-03).

### Integration Points
- `report.html` регистрируется в `DEFAULT_HTML_TEMPLATES` и материализуется тем же startup-хуком, что и акты.
- Редактор Шаблонов подключается к тем же файлам `templates/*.html`, что рендерят акты (правка через редактор → следующий рендер акта/отчёта отражает изменение).
- Tauri- и HTTP-адаптеры Отчётов — тонкие обёртки над `export_pdf`, меняют тип возврата на HTML-строку синхронно.

</code_context>

<specifics>
## Specific Ideas

- Новый дизайн таблицы отчёта: «чистый современный вид» — zebra-строки, лёгкие/тонкие границы, выделенный заголовок таблицы `<thead>`, помесячные заголовки как разделители секций, корректная печать на A4. НЕ компактно-плотный, НЕ прямой визуальный порт старого DocSpec-PDF.
- Шапка отчёта — визуально идентична шапке акта приёма-передачи (единый фирменный блок).
- Editor-превью должно работать даже на пустой БД (демо-данные вшиты), чтобы пользователь всегда видел результат правки.

</specifics>

<deferred>
## Deferred Ideas

- Полное удаление krilla/DocSpec/`pdf/*.rs` из репозитория — отдельная будущая уборка (SPEC out-of-scope; сейчас заморозка).
- Форматирование/локализация дат отчёта (МСК dd.mm.yyyy) — сознательно отложено, чтобы не расширять скоуп миграции движка (D-06). Кандидат в будущий фикс.
- Богатый WYSIWYG-редактор шаблонов (DOC-01), импорт/экспорт .docx (DOC-02), прочие печатные документы (DOC-03) — вне milestone.

None beyond the above — обсуждение осталось в границах фазы.

</deferred>

---

*Phase: 17-html-krilla*
*Context gathered: 2026-07-06*
