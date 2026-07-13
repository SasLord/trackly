# Phase 20: Печать актов и организация - Context

**Gathered:** 2026-07-13
**Status:** Ready for planning

<domain>
## Phase Boundary

Печать документа приёма из раздела Устройства получает полный организационный контекст в шапке (как акт приёма-передачи и отчёты), настройки организации получают вторую строку адреса, а безопасность SVG-логотипа подтверждается тестом. Три требования: **PRN-01, ORG-01, ORG-02**.

**В границах фазы:**
- Довести шапку acceptance-документа (`act_acceptance.html`) до паритета с handover/report.
- Переключить acceptance-рендер на источник БД (`get_for_pdf()`), убрав legacy `org.json`.
- Новое поле «Адрес (2-я строка)» (`address_line2`) в настройках организации + вывод во всех печатных формах.
- Regression-тест, доказывающий, что SVG-логотип со встроенным `<script>` не исполняется.

**Вне границ:** новые типы документов, редизайн вёрстки шапки, изменения полей помимо `address_line2`, работа с отчётными данными.

</domain>

<decisions>
## Implementation Decisions

### PRN-01 — Шапка acceptance-печати
- **D-01:** Шапка acceptance-документа приводится к **полному паритету** с актом приёма-передачи: логотип + название + ИНН/КПП + адрес (+ 2-я строка) + телефон + факс + email + ОКПО + ОГРН. Соответствует «и прочие реквизиты» из PRN-01. Расширить блок `.requisites` в `crates/trackly-app/templates/act_acceptance.html`, зеркаля `act_handover.html`/`report.html` (та же `{% if %}`-логика скрытия пустых полей).
- **D-02:** Источник данных acceptance-рендера (`render_acceptance_pdf` в `act_service.rs`) переключается с legacy `pipeline.organization` (org.json) на `org_db.get_for_pdf()` — тот же путь, что уже используют `render_pdf` (handover) и `report_service`. Как следствие, логотип из БД BLOB (загруженный через Настройки) начинает появляться в acceptance-печати — это и есть корень бага PRN-01.
- **D-03:** Расширить контекст `org` в `render_acceptance_pdf` до полного набора полей (`phone`, `fax`, `email`, `okpo`, `ogrn`, `address_line2`) — сейчас передаётся только `name/inn/kpp/address/logo_data_uri`.

### ORG-02 — Вторая строка адреса
- **D-04:** Новая колонка `address_line2` в таблице `org_settings`. Тип `TEXT NOT NULL DEFAULT ''`.
- **D-05:** Метка поля в UI (`OrgSettings.svelte`) — **«Адрес (2-я строка)»**. Полноширинное поле (`form-field--full`) сразу под полем «Адрес».
- **D-06:** В печатных шаблонах `address_line2` выводится **отдельной строкой** (`<div>{{ org.address_line2 }}</div>`) сразу под строкой основного адреса, только если заполнена. Единообразно во **всех трёх** шаблонах: `act_handover.html`, `act_acceptance.html`, `report.html`.
- **D-07:** Проброс `address_line2` через весь стек: `OrgSettingsDto` + `OrgPatch` (`dto/reports.rs`), `OrgDbService::get`/`save_fields`/`get_for_pdf` SQL, UI load/save в `OrgSettings.svelte`, все три ctx-сборки (`act_service::render_pdf`, `render_acceptance_pdf`, `report_service`).

### ORG-01 — Безопасность SVG-логотипа
- **D-08:** Требование 3 (встроенный `<script>` не исполняется) **уже выполнено** текущей реализацией: логотип рендерится исключительно через `<img src="data:image/svg+xml;base64,...">` во всех шаблонах и в превью Настроек; в img-контексте скрипты и внешние ресурсы инертны по спецификации. Дополнительная серверная санитизация SVG **не вводится** (defense-in-depth отклонён как избыточный).
- **D-09:** Добавить regression-тест: загрузить/отрендерить SVG-логотип, содержащий `<script>`, и проверить, что итоговый HTML акта встраивает его как `data:`-URI внутри `<img>` (а не инлайнит `<script>` в DOM). Тест фиксирует, что путь встраивания остаётся img-only.

### Миграция и легаси
- **D-10:** Миграция: новая refinery-миграция (следующий номер Vxxx) — `ALTER TABLE org_settings ADD COLUMN address_line2 TEXT NOT NULL DEFAULT ''`. Существующие установки получают пустую строку, ничего не ломается. Следовать существующему паттерну V026.
- **D-11:** Легаси `org.json`: acceptance-путь **полностью переходит на БД** (D-02). После этого `read_logo_bytes`/`pipeline.organization.read()` в `render_acceptance_pdf` больше не используются; БД — единый источник истины после one-time V026-миграции org.json→БД. Без fallback на org.json.

### Claude's Discretion
- Точная форма рефакторинга сборки `org`-контекста (например, извлечь общий helper, строящий `org` ctx из `OrgSettingsDto`, чтобы три рендер-пути не дублировали JSON) — на усмотрение планировщика/исполнителя.
- Точный номер и имя файла миграции — по существующему паттерну нумерации.
- Обновление doc-комментариев в шаблонах (перечень context-переменных) под новое поле `address_line2` и расширенную acceptance-шапку.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Требования и роадмап
- `.planning/ROADMAP.md` §«Phase 20: Печать актов и организация» — цель, Success Criteria (4 пункта), зависимости.
- `.planning/REQUIREMENTS.md` — PRN-01 (стр. 28), ORG-01 (стр. 32), ORG-02 (стр. 33).

### Backend — рендер актов и настройки организации
- `crates/trackly-app/src/services/act_service.rs` — `render_pdf` (handover, ~2545–2670, **эталон полной org-шапки из БД**) и `render_acceptance_pdf` (~2678–2780, **дефицитный путь на legacy org.json — цель PRN-01**).
- `crates/trackly-app/src/services/org_db_service.rs` — `get`/`save_fields`/`get_for_pdf`, SQL по `org_settings`; mime-allowlist (png/jpeg/svg+xml) + лимит 512 КБ уже реализованы (~123–167).
- `crates/trackly-app/src/services/report_service.rs` — `render` (~546–650), **вторая эталонная реализация полной org-шапки из БД**.
- `crates/trackly-app/src/dto/reports.rs` — `OrgSettingsDto` / `OrgPatch` (добавить `address_line2`).
- `crates/trackly-app/src/http/settings_org.rs` + `crates/trackly-app/src/tauri_cmds/settings_org.rs` — транспортные адаптеры настроек организации.

### Печатные шаблоны (HTML, Phase 16/17)
- `crates/trackly-app/templates/act_handover.html` — эталонная полная шапка (`.requisites`, стр. 131–140).
- `crates/trackly-app/templates/act_acceptance.html` — шапка-цель PRN-01 (стр. 103–108, сейчас только name/inn/kpp/address).
- `crates/trackly-app/templates/report.html` — полная шапка (стр. 135–143).
- Все шаблоны: логотип через `<img src="{{ org.logo_data_uri | safe }}">` — паттерн безопасности ORG-01.

### Frontend — настройки организации
- `ui/src/features/settings/OrgSettings.svelte` — форма реквизитов + загрузка/превью логотипа (SVG уже в фильтрах и `accept`); добавить поле `address_line2`.
- `ui/src/features/devices/DevicesPage.svelte` — flow «Печать документа приёма» (`mode="acceptance"`, стр. 158–174, 296–312) — точка запуска PRN-01-печати.

### Миграции
- `crates/trackly-app` refinery-миграции (паттерн V026 — введение таблицы `org_settings`); новая Vxxx добавляет `address_line2`.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **`OrgDbService::get_for_pdf()`**: уже возвращает полный `OrgSettingsDto` + logo bytes/mime из БД; handover и report его используют — acceptance просто подключается к нему же.
- **`.requisites`-блок из `act_handover.html`/`report.html`**: копируется в `act_acceptance.html` целиком (с добавлением строки `address_line2`).
- **mime-allowlist + 512 КБ лимит** (`org_db_service.rs`): SVG уже принимается и хранится — по ORG-01 новой валидации логотипа не требуется.

### Established Patterns
- **HTML-печать (Phase 16/17)**: рендер-функция → строка HTML → браузерный print-диалог (desktop + LAN). Шаблоны читаются file-first из `templates/` с embedded-fallback (`html_templates::load_template`), autoescape ON (`build_safe_html_env`), `| safe` только для `logo_data_uri`.
- **Три рендер-пути дублируют сборку `org` ctx** — кандидат на общий helper (Claude's discretion).
- **refinery-миграции** встроены в бинарник; single-writer-паттерн для записи `org_settings`.
- **Seed/upgrade шаблонов**: правка бундл-шаблона `act_acceptance.html` в `templates/` НЕ трогает существующие пользовательские копии автоматически — см. `[[db_backed_templates_upgrade_trap]]`. Для HTML-шаблонов (Phase 16/17) путь file-first, но проверить поведение embedded-default vs существующего файла на диске.

### Integration Points
- `render_acceptance_pdf` ↔ `OrgDbService::get_for_pdf()` (замена legacy `pipeline.organization`).
- `OrgSettings.svelte` ↔ `settings_get_org`/`settings_save_org_fields` ↔ `OrgPatch`/`OrgSettingsDto` (новое поле сквозь весь контракт, регенерация `bindings.ts` через `export_bindings`).
- Новая миграция ↔ `org_settings` схема.

</code_context>

<specifics>
## Specific Ideas

- Терминология: пункт меню в Устройствах — «Печать документа приёма» (это acceptance-документ `act_acceptance.html`); именно он — цель PRN-01, несмотря на формулировку «акт приёма-передачи из раздела Устройства» в требовании.
- Метка поля дословно: **«Адрес (2-я строка)»**.
- Тест безопасности должен использовать SVG именно с `<script>` внутри и утверждать img-only-встраивание.

</specifics>

<deferred>
## Deferred Ideas

- Серверная санитизация SVG (вырезание `<script>`/`on*`/внешних ссылок) как defense-in-depth — рассмотрено и **отклонено** для этой фазы (img-only достаточно, требование выполнено). Может быть пересмотрено, если появится путь inline-встраивания SVG в DOM.
- Извлечение общего helper для сборки `org` ctx через три рендер-пути — техдолг/рефакторинг, допустим в рамках фазы, но не обязателен.

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 20-print-acts-org*
*Context gathered: 2026-07-13*
