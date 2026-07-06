# Phase 17: Отчёты и Шаблоны через HTML-печать (убрать krilla) — Specification

**Created:** 2026-07-06
**Ambiguity score:** 0.15 (gate: ≤ 0.20)
**Requirements:** 7 locked

## Goal

Экспорт Отчётов и редактор Шаблонов переходят с krilla/DocSpec на HTML-печать по паттерну Phase 16 (акты): `export_pdf` возвращает HTML-строку, печать/сохранение идёт через диалог браузера в превью-модалке (desktop + LAN), редактор Шаблонов правит HTML-файлы в `templates/`, и `krilla`/`DocSpec` полностью выведены из активного пути (заморожены, не удалены).

## Background

После Phase 16 оба акта рендерятся из HTML-файлов (`templates/act_handover.html`, `templates/act_acceptance.html`, file-first + embedded fallback) и печатаются через `PdfPreviewModal.svelte` (srcdoc + `window.print()`) в обоих режимах. Но два потребителя krilla остались:

- **Отчёты** — `ReportService::export_pdf` ([report_service.rs:512](../../../crates/trackly-app/src/services/report_service.rs)) строит `DocSpec` (`HeaderBlock` + `Section::ItemsTable`/`Heading`) и вызывает `PdfRenderer::render_docspec` → krilla PDF-байты. Фронтенд ([ReportsPage.svelte:380](../../../ui/src/features/reports/ReportsPage.svelte)) скачивает байты как `application/pdf` blob. STATE фиксирует живой баг **16-HUMAN-UAT 2b**: `reports_export_pdf` падает с «Ошибка при создании PDF».
- **Редактор Шаблонов** — `TemplateService::validate_preview` ([template_service.rs:295](../../../crates/trackly-app/src/services/template_service.rs)) рендерит DocSpec-JSON MiniJinja → krilla PDF для превью; `TemplateEditor.svelte` правит **DB-backed** `document_templates` (MiniJinja→DocSpec). Это мёртвый путь: акты рендерятся из HTML-файлов, а редактор всё ещё правит DocSpec-шаблоны в БД, которые больше ничего не печатают.

Оба факта означают: пока Отчёты и редактор Шаблонов не переведены на HTML, krilla остаётся в активном пути, а редактор Настроек редактирует не то, что печатается. Phase 17 закрывает отложенные пункты **16-HUMAN-UAT 2a** (миграция Отчётов) и **2b** (баг экспорта Отчётов).

## Requirements

1. **Отчёты → HTML-рендер**: `ReportService::export_pdf` возвращает HTML-строку вместо krilla PDF-байтов.
   - Current: `export_pdf` строит `DocSpec` и вызывает `self.pdf.render_docspec(&spec)` → `Vec<u8>` (krilla)
   - Target: метод рендерит self-contained HTML (шапка организации с логотипом data-URI, таблица с помесячной группировкой, «Нет данных за указанный период» при пустом наборе) и возвращает `String`; krilla/DocSpec из этого пути убраны
   - Acceptance: `export_pdf` имеет сигнатуру, возвращающую HTML-строку; в теле нет обращений к `DocSpec`/`render_docspec`; юнит/интеграционный тест проверяет, что HTML содержит помесячные заголовки (например «Сентябрь 2026») и строки данных для непустого отчёта

2. **Шаблон отчёта — редактируемый файл**: Отчёты рендерятся из `templates/report.html` (file-first + embedded fallback), по паттерну актов.
   - Current: HTML-разметки отчёта не существует; layout захардкожен в `DocSpec`-секциях в `export_pdf`
   - Target: добавлен `templates/report.html` в `DEFAULT_HTML_TEMPLATES` (embedded default), материализуется на диск при старте через тот же механизм, что и акты; `export_pdf` загружает его через `html_templates::load_template` (файл-first, embedded fallback) и рендерит через `build_safe_html_env` (autoescape ON)
   - Acceptance: файл `templates/report.html` присутствует в embedded-дефолтах и материализуется в `templates/` при старте; удаление файла на диске не ломает экспорт (embedded fallback); правка файла меняет вывод экспорта

3. **Адаптеры Отчётов возвращают HTML**: Tauri-команда и HTTP-хэндлер экспорта Отчёта отдают HTML-строку с `text/html`, не PDF-байты.
   - Current: `reports_export_pdf` (Tauri, [reports.rs:345](../../../crates/trackly-app/src/tauri_cmds/reports.rs)) и `handler_export_pdf` (HTTP, [reports.rs:203](../../../crates/trackly-app/src/http/reports.rs)) возвращают `Vec<u8>` PDF
   - Target: оба адаптера возвращают HTML-строку; HTTP-ответ имеет `Content-Type: text/html`; `bindings.ts` перегенерирован под новый тип возврата
   - Acceptance: HTTP-эндпойнт отвечает `text/html`; сгенерированный `bindings.ts` объявляет строковый тип возврата для `reports_export_pdf`; `specta_roundtrip`/`export_bindings` тесты зелёные

4. **Печать Отчётов как у актов**: Фронтенд открывает HTML отчёта в превью-модалке с `window.print()` в обоих режимах (desktop + LAN).
   - Current: `ReportsPage.exportPdf()` собирает `application/pdf` blob и скачивает файл (или открывает через Tauri-плагины)
   - Target: `exportPdf()` получает HTML-строку и открывает её в превью-модалке (srcdoc + печать), по паттерну `PdfPreviewModal.svelte`; кнопка скачивания PDF-файла удалена/заменена печатью; CSV-экспорт не меняется
   - Acceptance: нажатие «Экспорт PDF» в Отчётах открывает превью и печать без ошибки «Ошибка при создании PDF» (баг 2b закрыт) и работает и в desktop, и в LAN-браузере

5. **Редактор Шаблонов правит HTML-файлы**: `TemplateEditor.svelte` и `TemplateService` работают с HTML-файлами в `templates/`, а не с DB DocSpec-MiniJinja.
   - Current: редактор правит `document_templates` (DB, DocSpec-JSON MiniJinja); «Сохранить» пишет в БД, «Проверить» = krilla-PDF превью; список переменных описывает DocSpec-поля
   - Target: редактор загружает содержимое HTML-файлов (`act_handover.html`, `act_acceptance.html`, `report.html`); «Сохранить» пишет на диск в `templates/*.html`; «Сбросить до умолчания» восстанавливает embedded default; превью = HTML-print (не krilla PDF); панель переменных обновлена под HTML-контекст
   - Acceptance: «Сохранить» в редакторе изменяет файл `templates/act_handover.html` на диске; последующий рендер акта отражает правку; «Сбросить» возвращает embedded-дефолт; превью открывается как HTML, не как krilla PDF

6. **krilla выведена из активного пути (заморожена)**: Ни Отчёты, ни Шаблоны, ни health-проверки не вызывают krilla при обычной работе; `PdfRenderer`/`DocSpec`/`pdf/{renderer,docspec,fonts}.rs` остаются скомпилированными, но неиспользуемыми, `krilla` остаётся в `Cargo.toml`.
   - Current: `report_service`, `template_service`, `health` (Tauri+HTTP) конструируют `PdfRenderer` и вызывают `render_docspec`
   - Target: активные пути Отчётов/Шаблонов/health не создают `PdfRenderer` и не вызывают `render_docspec`; krilla-код помечен как замороженный (не удалён), как акты в Phase 16 (D-13 hygiene: krilla-тесты `#[ignore]`)
   - Acceptance: `grep` по `render_docspec`/`PdfRenderer::new` в активных (не `#[ignore]`, не заморожённых) путях `report_service`/`template_service`/`http`/`tauri_cmds` даёт 0 совпадений; проект собирается; krilla-тесты помечены `#[ignore]`

7. **Тесты мигрированы**: Тесты Отчётов и Шаблонов проверяют HTML-вывод, а не PDF-байты; krilla-специфичные тесты заморожены.
   - Current: тесты экспорта/превью опираются на PDF-байты (`pdf_*`, `report_*`, `template preview`)
   - Target: тесты Отчётов/Шаблонов ассертят на HTML-строку (наличие полей, помесячные заголовки, экранирование); krilla PDF-тесты помечены `#[ignore]` (заморожены, как в Phase 16 D-13)
   - Acceptance: `cargo test -p trackly-app` зелёный; есть тест HTML-рендера отчёта (1 и N строк, помесячная группировка, пустой набор) и тест HTML-рендера/сохранения шаблона; `clippy -D warnings` и `fmt --check` зелёные

## Boundaries

**In scope:**
- Миграция `ReportService::export_pdf` на HTML-строку
- Новый редактируемый `templates/report.html` (embedded default + материализация + fallback)
- Обновление Tauri/HTTP-адаптеров экспорта Отчёта на `text/html` + перегенерация `bindings.ts`
- Печать Отчётов через превью-модалку (srcdoc + `window.print()`) в desktop и LAN
- Перенацеливание редактора Шаблонов на HTML-файлы `templates/*.html` (загрузка/сохранение на диск/сброс/HTML-превью), включая `report.html`
- Обновление панели «Доступные переменные» под HTML-контекст
- Заморозка krilla/DocSpec в активном пути (Отчёты, Шаблоны, health); `#[ignore]` для krilla-тестов
- Миграция тестов Отчётов/Шаблонов с PDF-байтов на HTML-ассерты
- Закрытие отложенных пунктов 16-HUMAN-UAT 2a и 2b

**Out of scope:**
- Полное удаление crate `krilla` из `Cargo.toml` и файлов `pdf/{renderer,docspec,fonts}.rs` — по решению заморозить (как акты в Phase 16); удаление — отдельная будущая уборка
- CSV-экспорт Отчётов — не трогаем, работает и не зависит от krilla
- Новые виды отчётов или новые поля/колонки — только миграция движка рендера
- Богатый WYSIWYG-редактор шаблонов (DOC-01) — вне milestone
- Импорт/экспорт .docx шаблонов (DOC-02) — вне milestone
- Прочие виды печатных документов помимо актов/отчёта (DOC-03) — вне milestone
- Изменение layout/фиделити актов — сделано в Phase 15/16, не пересматривается

## Constraints

- **Паттерн-эталон:** Phase 16 (акты) — file-first + embedded fallback через `html_templates::load_template`, `build_safe_html_env` (autoescape ON, T-07-04-02: тело шаблона не eval'ится в браузере), data-URI логотип, `PdfPreviewModal` (srcdoc + `window.print()`), работа в обоих режимах (desktop WebView2/WKWebView + LAN-браузер).
- **Portable-режим:** запись отредактированных шаблонов и материализация дефолтов — в `templates/` рядом с исполняемым файлом (через `Paths::templates_dir()`), не в `%APPDATA%`.
- **Безопасность:** значения фильтров/данных отчёта проходят через autoescape HTML-env; CSV formula-injection guard (`csv_safe`) и параметризованные SQL-запросы сохраняются без изменений.
- **krilla заморожен, не удалён:** код `pdf/{renderer,docspec,fonts}.rs` и dep остаются; активные пути его не вызывают.

## Acceptance Criteria

- [ ] `ReportService::export_pdf` возвращает HTML-строку; в активном теле нет `DocSpec`/`render_docspec`
- [ ] `templates/report.html` присутствует в embedded-дефолтах, материализуется в `templates/` при старте, поддерживает file-first + fallback
- [ ] Tauri `reports_export_pdf` и HTTP `handler_export_pdf` возвращают HTML-строку; HTTP отдаёт `Content-Type: text/html`; `bindings.ts` перегенерирован
- [ ] «Экспорт PDF» в Отчётах открывает превью+печать без ошибки «Ошибка при создании PDF» и работает в desktop и LAN (закрывает 2a + 2b)
- [ ] Редактор Шаблонов загружает/сохраняет/сбрасывает HTML-файлы `templates/*.html` (включая `report.html`); «Сохранить» пишет на диск; превью = HTML-print
- [ ] Панель «Доступные переменные» обновлена под HTML-контекст
- [ ] Активные пути Отчётов/Шаблонов/health не создают `PdfRenderer` и не вызывают `render_docspec` (grep = 0); krilla-тесты помечены `#[ignore]`
- [ ] Есть HTML-рендер-тесты Отчёта (1/N строк, помесячная группировка, пустой набор) и HTML-рендер/сохранение шаблона
- [ ] `cargo test -p trackly-app`, `clippy -D warnings`, `fmt --check` — зелёные

## Ambiguity Report

| Dimension          | Score | Min  | Status | Notes                                                        |
|--------------------|-------|------|--------|--------------------------------------------------------------|
| Goal Clarity       | 0.90  | 0.75 | ✓      | Три оси (Отчёты/Шаблоны/krilla) зафиксированы через паттерн 16 |
| Boundary Clarity   | 0.85  | 0.70 | ✓      | krilla заморожен (не удалён), CSV/акты вне скоупа             |
| Constraint Clarity | 0.80  | 0.65 | ✓      | Phase 16 pattern + portable write path                       |
| Acceptance Criteria| 0.80  | 0.70 | ✓      | 9 pass/fail критериев, закрывает 2a/2b                        |
| **Ambiguity**      | 0.15  | ≤0.20| ✓      |                                                              |

Status: ✓ = met minimum, ⚠ = below minimum (planner treats as assumption)

## Interview Log

| Round | Perspective     | Question summary                          | Decision locked                                             |
|-------|-----------------|-------------------------------------------|-------------------------------------------------------------|
| 1     | Researcher      | Судьба редактора Шаблонов после Phase 16  | Перенацелить редактор на HTML-файлы `templates/`            |
| 1     | Boundary Keeper | Глубина удаления krilla                    | Убрать из активного пути (заморозить), не удалять crate/код |
| 1     | Simplifier      | UX печати Отчётов                          | Как акты: превью-модалка (srcdoc + `window.print()`)        |
| 2     | Boundary Keeper | Куда «Сохранить» пишет шаблон              | В файл `templates/*.html` на диске (не в DB)                |
| 2     | Boundary Keeper | Рендер HTML отчёта — файл или код          | Отдельный редактируемый файл `templates/report.html`        |

---

*Phase: 17-html-krilla*
*Spec created: 2026-07-06*
*Next step: /gsd-discuss-phase 17 — implementation decisions (структура report.html, точки материализации, список HTML-переменных, форма превью-модалки Отчётов)*
