# Phase 16: documents-html-print - Context

**Gathered:** 2026-07-05
**Status:** Ready for planning

<domain>
## Phase Boundary

Генерация обоих актов (приёма-передачи `act_handover` и приёмки устройства `act_acceptance`) переходит с krilla/DocSpec-пайплайна на HTML-шаблоны: файлы `templates/*.html` рядом с exe (редактируются как файлы) + вшитый дефолт-fallback, рендер в self-contained HTML по образцу Word, печать/сохранение в PDF через диалог браузера в обоих режимах (desktop Tauri-webview + LAN-браузер). krilla/DocSpec/`renderer.rs`/MiniJinja-`.minijinja`-шаблоны остаются в репо **заморожёнными** (не удаляются, активно не вызываются).

</domain>

<spec_lock>
## Requirements (locked via SPEC.md)

**8 requirements are locked.** См. `16-SPEC.md` для полного списка требований, границ и критериев приёмки.

Downstream agents MUST read `16-SPEC.md` before planning or implementing. Требования здесь не дублируются.

**In scope (from SPEC.md):**
- HTML-генерация обоих актов (`act_handover` + `act_acceptance`) по образцу Word
- Мультиустройство (N позиций) с word-wrap и корректными page-break
- Хранение шаблонов в папке `templates/` рядом с exe + вшитый дефолт-fallback; редактирование файлами
- Печать/сохранение в PDF через диалог браузера в обоих режимах (desktop + LAN)
- Self-contained HTML (системные шрифты, локальный логотип, без внешних CDN)
- Print CSS (A4-книжная, поля, разрывы страниц)
- Тесты HTML-генерации обоих актов + fallback-шаблона

**Out of scope (from SPEC.md):**
- Удаление krilla/DocSpec/`renderer.rs`/MiniJinja-шаблонов (остаются замороженными, без fallback на них)
- Серверный headless-PDF (генерация канонического PDF на бэкенде) — печать только через браузер
- UI-редактор шаблонов сверх правки файлов (DOC-01)
- Миграция старых MiniJinja→DocSpec шаблонов из `document_templates`
- Любые документы кроме двух актов
- Embedding шрифтов в документ (достаточно системных)

</spec_lock>

<decisions>
## Implementation Decisions

### Движок HTML-шаблонов
- **D-01:** Шаблонизатор — **MiniJinja, рендерящий HTML напрямую из файлов** `templates/*.html` (без DocSpec-JSON round-trip, без БД). «Новым механизмом» из интервью SPEC является связка **файлы + HTML** (вместо DB `document_templates` + DocSpec), а НЕ смена крейта. Крейт MiniJinja переиспользуется — 0 новых зависимостей.
- **D-02:** Причина против нового крейта: compile-time движки (askama/maud/build.rs) **несовместимы с Req 4** (правка файла должна применяться без пересборки) — вшивают шаблон в бинарь. Нужен рантайм-движок; Tera/Handlebars дублировали бы роль MiniJinja и тяжелее. MiniJinja уже даёт autoescape HTML + safe-mode (`UndefinedBehavior::Strict`, `set_fuel`) + 5s timeout в `render_with_timeout`.
- **D-03:** Авторская модель: пользователь **сам пишет `templates/act_handover.html` / `act_acceptance.html` в VSCode** — обычный HTML со своей вёрсткой, `{{ переменные }}`, `{% for %}`/`{% if %}` (тот же синтаксис, что в текущих `.minijinja`). Стили — **инлайн `<style>` в том же файле** (self-contained, без внешнего `.css`). Ручной `wrap_text_to_width` больше не нужен — перенос длинных полей делает браузер (CSS).
- **D-04:** Контекст рендера переиспользует уже собираемый serde_json `ctx`-объект (`act_service::render_pdf`, `act_service.rs:1414`). Логотип пробрасывается новой переменной-`data:`-URI (см. D-11), а не через `logo_bytes`-впрыск в DocSpec.

### Папка templates + fallback
- **D-05:** **Авто-материализация** вшитого дефолта при старте: если `templates/act_handover.html` (или `act_acceptance.html`) рядом с exe нет — записать `include_str!`-дефолт в папку, чтобы пользователь сразу видел файл для правки.
- **D-06:** Fallback при генерации: если файла нет (удалён после старта) — использовать вшитый `include_str!`-дефолт, **генерация не падает**.
- **D-07:** Резолв пути: прод — `std::env::current_exe()?.parent()?/templates/`. Dev/тесты — **ENV-override** (напр. `TRACKLY_TEMPLATES_DIR`), чтобы не сорить в `target/debug/` и упростить правку в dev. Миррорит существующие mock-env паттерны (`TRACKLY_AD_MOCK`/`SNMP_MOCK`).
- **D-08:** Перечитка — **read-on-render** (файл читается с диска при каждой генерации). Правка применяется сразу без перезапуска; без `notify`-watch (акты генерятся редко, I/O незначим).

### Доставка + печать
- **D-09:** Единый путь desktop+LAN: команда/эндпоинт возвращает **HTML-строку** → `srcdoc` в `<iframe>` существующей `PdfPreviewModal.svelte` → `iframe.contentWindow.print()`. Максимум reuse текущего UI, одинаково работает в обоих webview.
- **D-10:** API: **заменить** текущие `Vec<u8>`-команды/эндпоинты на возврат `String` (HTML) — те же имена (`acts_render_pdf`/`devices_render_acceptance_pdf` → отдают HTML; HTTP `text/html`). `acts_open_pdf_in_system` больше не нужен (печать через браузер). Биндинги + фронт обновляются. (krilla-`renderer.rs` при этом остаётся в коде, но сервисом не вызывается — см. D-14.)
- **D-11:** Логотип — **base64 `data:`-URI прямо в `<img src>`** (self-contained, офлайн, без сетевых запросов в обоих режимах). НЕ локальный `/api/v1/org/logo`-endpoint (сломался бы в desktop-без-сервера).
- **D-12:** Print CSS — `@page { size: A4 portrait; margin: ... }` задаёт формат/поля по образцу Word. Мультиустройство: `page-break-inside: avoid` на секции устройства + браузерная A4-пагинация (Req 3). Браузерные header/footer (URL/дата/номер) CSS отключить нельзя — это галочка «Колонтитулы» в диалоге печати; вёрстка на них не полагается + короткая UAT-подсказка пользователю.

### Тесты + судьба krilla
- **D-13:** krilla-тесты — **гибрид**: быстрые единичные (`pdf_logo` и т.п.) оставить зелёными как защиту от bit-rot замороженного кода; тяжёлые/медленные (`pdf_determinism`, рендер реального PDF) — `#[ignore]`. Тесты вызывают `renderer` напрямую, сервису не мешают.
- **D-14:** Новые HTML-тесты (Req 8), обязательный набор:
  1. **Наличие блоков/полей** — HTML обоих актов содержит обязательные блоки (шапка, заголовок, номер/дата, поля устройства, подписи) + логотип (`data:`-URI в разметке).
  2. **1 vs N устройств** — акт с 1 и с N≥2 устройствами: все позиции присутствуют, длинные поля не обрезаны.
  3. **Fallback vs файл** — при отсутствии папки берётся вшитый дефолт; при наличии `templates/*.html` берётся файл; правка файла меняет вывод.
  4. **Офлайн/без-CDN** — в разметке нет `http(s)://`-ссылок в `href`/`src` (кроме `data:`) — гарантия self-contained.

### Claude's Discretion
- Точная HTML/CSS-вёрстка образца Word (порядок и стили блоков) — воспроизвести зафиксированный в Phase 15 результат; исполнитель/пользователь дорабатывают файл шаблона.
- Точное имя ENV-переменной для dev-override пути templates/ (D-07).
- Механика передачи HTML-строки во фронт (srcdoc vs blob) в деталях — важно iframe + print() (D-09).

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Спецификация фазы (locked)
- `.planning/phases/16-documents-html-print/16-SPEC.md` — Locked requirements (8 шт.), границы, констрейнты, критерии приёмки. MUST read before planning.

### Текущий (замораживаемый) пайплайн генерации
- `crates/trackly-app/src/pdf/mod.rs` — описание 3-стадийного krilla-пайплайна (минимально нужно понять, что замораживается)
- `crates/trackly-app/src/pdf/renderer.rs` — 1908 строк ручной krilla-вёрстки (`wrap_text_to_width`, `DeviceCard`, `FieldRow`, page-break Phase 15) — **замораживается**, HTML это заменяет
- `crates/trackly-app/src/pdf/minijinja_env.rs` — `render_with_timeout` (safe-mode + 5s timeout) — **переиспользуется** для HTML-рендера
- `crates/trackly-app/src/services/act_service.rs` §`render_pdf` (~1342–1476), §`render_acceptance_pdf` (~1478+) — сборка serde_json `ctx`, чтение шаблона, логотип; сюда встраивается HTML-путь

### Доставка (адаптеры под замену/расширение)
- `crates/trackly-app/src/tauri_cmds/acts.rs` — `acts_render_pdf`, `devices_render_acceptance_pdf`, `acts_open_pdf_in_system` (Vec<u8> → String)
- `crates/trackly-app/src/http/acts.rs` — axum-роуты (mirror Tauri-команд; `application/pdf` → `text/html`)
- `ui/src/features/acts/PdfPreviewModal.svelte` — iframe blob-preview + print (переиспользуется, srcdoc + print())
- `ui/src/features/acts/DocumentAcceptanceModal.svelte` — путь акта приёмки на фронте
- `crates/trackly-app/src/services/template_service.rs` — текущее БД-хранение шаблонов (замораживается для актов; новый путь = файлы)

### Существующие шаблоны-образцы (переносятся в HTML)
- `crates/trackly-app/templates/act_handover.minijinja` — образец акта приёма-передачи (Phase 15 word-fidelity) — источник вёрстки для HTML
- `crates/trackly-app/templates/act_acceptance.minijinja` — образец акта приёмки

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **`pdf::minijinja_env::render_with_timeout`** — рантайм-рендер MiniJinja с safe-mode (`UndefinedBehavior::Strict`, `set_fuel(100_000)`, no-loader) + `tokio::time::timeout(5s)` вокруг `spawn_blocking`. Переиспользуется как есть: на вход HTML-шаблон вместо DocSpec-шаблона, на выход HTML-строка вместо JSON.
- **`act_service::render_pdf` serde_json `ctx`** (`act_service.rs:1414`) — уже собирает `org` (расширенные реквизиты + logo), `act` (number/suffix/date/giver/receiver/deadline/location/items[]/parent), `return`. Тот же контекст кормит HTML-шаблон без изменений структуры (кроме `logo` → `data:`-URI).
- **`PdfPreviewModal.svelte`** — iframe + blob + print уже реализованы; переключить с `blob:`(PDF) на `srcdoc`(HTML) + `iframe.print()`.
- **Portable path-resolve pattern** — `current_exe().parent()` уже используется по проекту (CLAUDE.md constraint); templates-резолв следует ему.
- **Mock-env pattern** — `TRACKLY_AD_MOCK`/`SNMP_MOCK` env-гейты как прецедент для `TRACKLY_TEMPLATES_DIR` dev-override.

### Established Patterns
- **Dual-transport**: Tauri-команда + axum-эндпоинт — тонкие адаптеры над сервисным слоем (`act_service`). HTML-строка возвращается обоими; бизнес-логика едина.
- **Специта-биндинги**: смена возврата `Vec<u8>`→`String` требует `export_bindings` (проверять на drift в CI).
- **Seed-on-startup**: `template_service::seed_defaults_on_startup` — прецедент авто-сидирования дефолтов при старте (D-05 повторяет паттерн, но в файловую папку, не в БД).

### Integration Points
- Точка входа генерации: `AppCtx.acts` (ActService) `.render_pdf()` / `.render_acceptance_pdf()` — переключаются на HTML-рендер.
- Файловый резолв templates/ — новый модуль (напр. `pdf/html_templates.rs` или `templates/loader`), инициализируется при старте (материализация) + читается при рендере.
- Frontend `ui/src/lib/api/acts.ts` / `pdf.ts` — тип возврата number[]→string, потребители в модалках.

</code_context>

<specifics>
## Specific Ideas

- Пользователь хочет **сам верстать HTML-шаблон в VSCode** со своей таблицей стилей, расставляя `{{ переменные }}` и `{% for %}`/`{% if %}` — движок читает файл при генерации и подставляет данные. Один файл = один самодостаточный HTML (`<style>` инлайн, логотип `data:`-URI).
- Визуальная цель — **тот же образец Word, что зафиксирован в Phase 15** (не новый дизайн): шапка (логотип + реквизиты), «Акт приема-передачи», номер/дата, вводная формулировка с ФИО, поля устройства построчно «метка | подчёркнутое значение» (без «Устройство №N»-заголовков), «Сроком до», двухстрочные подписи «Выдал/Получил».

</specifics>

<deferred>
## Deferred Ideas

None — обсуждение осталось в границах фазы. (UI-редактор шаблонов сверх правки файлов, headless-PDF на бэкенде, удаление krilla — явно вне скоупа per SPEC.)

</deferred>

---

*Phase: 16-documents-html-print*
*Context gathered: 2026-07-05*
