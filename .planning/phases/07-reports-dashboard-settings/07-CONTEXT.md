# Phase 7: Отчёты, Дашборд и Настройки - Context

**Gathered:** 2026-06-15
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 7 поставляет три под-домена поверх уже готовых учётных доменов (Устройства, Акты, Картриджи, Принтеры, Заявки):

1. **Отчёты** — отчёты по Устройствам (Акты приёма-передачи, Возвраты, Что в работе, Что на складе) и Картриджам (Расход, Что в работе, Что на складе, История заправок) с выбором периода (месяц/год/диапазон в TZ организации), контекстными фильтрами, поиском, группировкой по месяцам, экспортом в CSV/PDF и печатью. RPT-01..08.
2. **Дашборд** — пять read-only виджетов (Устройства, Картриджи, Динамика расхода, Заявки, Принтеры) на фиксированной сетке; дашборд становится стартовой страницей. DASH-01..05.
3. **Настройки** — Организация (поля + логотип в БД), порог низкого остатка (UI к уже существующему `app_settings.low_stock_threshold`), путь БД (открыть папку / сменить расположение), ручной + авто-бэкап через `rusqlite::backup::Backup`, редактирование MiniJinja-шаблонов документов. SET-01..07, SET-09.

**Scope anchor:** фаза НЕ добавляет новых учётных сущностей и НЕ меняет lifecycle существующих доменов — она читает их данные (отчёты/дашборд) и добавляет конфигурационный слой (настройки/бэкапы/шаблоны). Активирует supervisor таблицы `scheduled_tasks` (заложена в Phase 1) — впервые в проекте.

**Out of scope (deferred):** Корзина (UI над soft-delete), настраиваемая компоновка дашборда (drag/hide), визуальный WYSIWYG-редактор шаблонов, реконструкция исторических snapshot'ов на произвольную дату.

</domain>

<decisions>
## Implementation Decisions

### Отчёты — навигация и тип
- **D-01:** Навигация — двухуровневая: sub-nav по доменам (Устройства / Картриджи) + свитч-бар конкретных отчётов внутри домена. Консистентно с существующими свитч-барами Актов (Акты/Возвраты/Архив) и Картриджей (статусы).
- **D-02:** Snapshot-отчёты «Что на складе» / «Что в работе» — срез текущего состояния («сейчас»); селектор периода для них **скрыт/неактивен**; группировка по локации/статусу, НЕ по месяцам. Месячная группировка (RPT-06, заголовок «Сентябрь 2026») применяется только к временным отчётам (Акты, Возвраты, Расход, История заправок).
- **D-03:** Период по умолчанию для временного отчёта — **текущий месяц**.

### Отчёты — фильтры, экспорт, печать
- **D-04:** Фильтры внутри отчёта (RPT-04) — **контекстные по типу отчёта**: Устройства → локация/тип/статус; Картриджи → модель/статус/цвет. Не единый набор.
- **D-05:** PDF отчёта — **один универсальный табличный шаблон** (шапка: логотип + организация + название отчёта + период; таблица колонок; месячные разделители). Колонки определяются типом отчёта. Переиспользует krilla DocSpec IR из Phase 3, НЕ отдельный шаблон на каждый отчёт.
- **D-06:** Печать (RPT-08) — **по контексту транспорта**: desktop (Tauri) генерирует PDF и открывает в системном просмотрщике/диалоге печати; browser (LAN) — `window.print()` / download PDF. Два пути.
- **D-07:** Охват экспорта (CSV/PDF) — **«что видно сейчас»**: экспорт уважает текущие фильтры + поиск + период.
- **D-08:** CSV — переиспользует девайсовый паттерн из Phase 2 (UTF-8 BOM, `;`-делимитер).

### Дашборд
- **D-09:** Дашборд — **стартовая (домашняя) страница** приложения (для ролей с доступом).
- **D-10:** Компоновка 5 виджетов — **фиксированная адаптивная сетка, read-only** (порядок фиксирован; настраиваемость — defer в backlog).
- **D-11:** График «Динамика расхода картриджей» (DASH-03) — **линейный**, с переключателем окна **3 / 6 / 12 месяцев**.
- **D-12:** Дашборд имеет **общий селектор периода**, влияющий на period-based виджеты (Заявки DASH-04 и др.). График расхода сохраняет собственный переключатель 3/6/12.
- **D-13:** Виджет «Принтеры» (DASH-05) «проблемные» = принтеры с **активными алертами `offline` + `error`** (вкл. Pantum hang detection) из существующей таблицы `printer_alerts`. Не только offline.

### Семантика отчётов по картриджам
- **D-14:** «Расход» (RPT-02) и график DASH-03 = события **Install** (картридж установлен в работу) по месяцам/моделям. «История заправок» = события **ToRefill / FromRefill**. Не WriteOff.

### Настройки — Организация
- **D-15:** Хранение организации — **всё в БД**: org-поля + логотип-BLOB. One-time импорт из существующего `org.json` при миграции; `org.json` устаревает как источник. Единый source-of-truth в `.db` («портабельность одним файлом»). Ретирует существующую path-traversal mitigation логику логотипа-как-файла (`safe_logo_canonical`).
  - **Примечание:** `organization.timezone` сейчас живёт в config.toml (`Europe/Moscow`) — research/planner определяет, переносить ли TZ в БД или оставить в конфиге; решение D-15 касается org-полей и логотипа.

### Настройки — Бэкапы
- **D-16:** Авто-бэкап (SET-07) — **папку выбирает пользователь** (нет дефолтной папки; автобэкап неактивен, пока путь не указан). Ретенция по умолчанию — **7 копий**, настраивается.
- **D-17:** Планировщик (scheduled_tasks supervisor) работает **пока процесс жив** (desktop ИЛИ server-mode), с **catch-up**: просроченный бэкап выполняется при следующем старте.
- **D-18:** Бэкап — обязательно через `rusqlite::backup::Backup` (НЕ `fs::copy` — clippy-забанен на воркспейсе с Phase 1); integrity_check на бэкапе после записи.

### Настройки — Путь БД
- **D-19:** «Сменить расположение БД» (SET-03) — **копия + перезапуск**: `rusqlite::backup` копирует БД на новый путь (проверка запрета SMB-шары + integrity_check), путь сохраняется в конфиг, приложение просит перезапуск (single-writer безопасность). Не горячее переключение. «Открыть папку» — через `tauri-plugin-shell`.

### Настройки — Шаблоны
- **D-20:** Редактор MiniJinja-шаблонов (SET-09) — **raw textarea + панель доступных переменных + кнопка валидации/превью PDF**. Админ видит результат до сохранения; валидация при сохранении. Не голый textarea, не WYSIWYG.

### Log-retention
- **D-21:** В scope только **log-retention worker** на активируемом supervisor'е (чистка старых ротированных логов; supervisor всё равно нужен под бэкапы). Корзина (UI над soft-delete) — **defer в backlog**.

### Claude's Discretion
- Выбор графической библиотеки для линейного графика расхода (D-11) — на research/planner (учесть, что фронт — vanilla Svelte 5, без SvelteKit; предпочесть лёгкое решение без тяжёлых зависимостей).
- Решение, переносить ли `organization.timezone` из config.toml в БД (см. примечание к D-15).
- Конкретная вёрстка universal табличного PDF (ориентация portrait/landscape для широких отчётов, разбивка колонок) — на planner.
- Точная схема активации supervisor и формат записей `scheduled_tasks` — на research/planner.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Roadmap & Requirements
- `.planning/ROADMAP.md` §"Phase 7: Отчёты, Дашборд и Настройки" — goal, success criteria 1–5, requirement list (RPT-01..08, DASH-01..05, SET-01..07, SET-09).
- `.planning/REQUIREMENTS.md` — полные формулировки RPT/DASH/SET требований; SET-04 (порог), SET-05/06/07 (бэкап через SQLite `.backup` API), PRN-06 (определение «зависшего/проблемного» принтера, нужно для DASH-05).

### Phase 1 — заложенная инфраструктура (deferred-to-Phase-7)
- `.planning/phases/01-foundation/01-CONTEXT.md` §"Логотип BLOB в БД", §"Backup retention + scheduled_tasks worker", §"Retention" — Phase 1 заложил таблицы `scheduled_tasks` (V011) и `document_templates`; supervisor запускается в Phase 7.
- `.planning/phases/01-foundation/01-SKELETON.md` — `scheduled_tasks` worker, Logo BLOB, Backup retention помечены как Phase 7; `std::fs::copy` clippy-забанен → бэкап обязан использовать `rusqlite::backup::Backup`.
- `.planning/phases/01-foundation/01-RESEARCH.md` — tauri-plugin-fs / -dialog / -shell отмечены как «Phase 7 surface» (file picker, open folder, save dialog).

### Phase 3 — PDF и Организация (переиспользуемая инфра)
- `crates/trackly-app/src/services/organization_service.rs` — текущее хранение org в `org.json` + логотип-как-путь + path-traversal mitigation (ретируется по D-15).
- `crates/trackly-app/src/services/template_service.rs`, `crates/trackly-app/src/http/templates.rs` — seed дефолтных MiniJinja-шаблонов; редактирование добавляется в SET-09.
- DocSpec IR + krilla 0.7 + DejaVu Sans + MiniJinja safe-mode из Phase 3 — основа universal PDF отчёта (D-05).

### Existing code touchpoints
- `crates/trackly-infra/src/repos/cartridges_sqlite.rs:671` — `low_stock` читает `app_settings.low_stock_threshold` (default 2); SET-04 = UI к этому.
- `crates/trackly-core/src/domain/cartridges.rs:97-130` — `CartridgeTransitionOp` (Install/ToRefill/FromRefill/WriteOff) — основа семантики Расхода/Истории (D-14).
- `crates/trackly-core/src/domain/printers.rs:46-56` — статусы `ok/warning/error/offline/unknown` + `PrinterAlertRow` (`offline/error`) — основа DASH-05 (D-13).
- `crates/trackly-app/src/csv/` (mod/parse/decode/sniff) + `http/devices.rs` — паттерн CSV-экспорта (UTF-8 BOM, `;`) для RPT-07 (D-08).
- `crates/trackly-infra/src/config.rs:106-112` — `organization.timezone` (`Europe/Moscow`) для TZ-границ периодов (RPT-03, success criterion 1).

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **DocSpec IR + krilla PDF (Phase 3):** прямая основа universal табличного PDF отчёта (D-05) и превью шаблонов (D-20).
- **CSV-инфра (`crates/trackly-app/src/csv/`, Phase 2):** UTF-8 BOM + `;`-делимитер уже отлажены — переиспользуются для RPT-07 (D-08).
- **`app_settings` k/v таблица:** уже хранит `low_stock_threshold` (читается в cartridges_sqlite), `desktop_lock_enabled` (auth). SET-04 — только UI; новые ключи (backup schedule/retention/folder) ложатся сюда же.
- **`scheduled_tasks` таблица (V011, Phase 1):** ждёт supervisor — впервые запускается здесь (D-17, D-21).
- **`document_templates` версионированная таблица (Phase 1):** место для редактируемых шаблонов SET-09.
- **`organization_service` + `template_service`:** существующие сервисы — расширяются (org→БД, template edit), а не пишутся с нуля.
- **Свитч-бары Актов/Картриджей:** UX-паттерн для навигации отчётов (D-01).

### Established Patterns
- **Single-writer (Phase 1):** все записи (бэкап, смена пути БД, сохранение org/шаблонов) идут через единый writer worker; смена пути БД требует перезапуска (D-19).
- **Dual-transport (Tauri + axum):** отчёты/настройки нужны в обоих транспортах; печать ветвится по транспорту (D-06).
- **UTC в БД, форматирование через TZ:** границы периодов (RPT-03) считаются в TZ организации — потребуется добавить `chrono-tz` (сейчас не зависимость).
- **`std::fs::copy` clippy-ban:** бэкап и смена пути БД ОБЯЗАНЫ использовать `rusqlite::backup::Backup` (D-18, D-19).

### Integration Points
- `ReportsPage.svelte` / `Dashboard.svelte` — сейчас плейсхолдеры (`Placeholder` компонент), заполняются здесь.
- `SettingsPage.svelte` — содержит только `NetworkSettings` (Phase 5); добавляются секции Организация / Хранилище-БД / Бэкапы / Шаблоны / Порог.
- Дашборд как стартовая страница — затрагивает роутинг (`svelte-spa-router`) и сайдбар (D-09).
- Логотип из БД (D-15) подставляется в шапку Актов и Документа приёма (success criterion 4) — точка интеграции с PDF-рендером Phase 3.

</code_context>

<specifics>
## Specific Ideas

- Месячный разделитель — буквальный заголовок вида «Сентябрь 2026» (русская локализация месяца) при смене месяца в временных отчётах (RPT-06).
- Snapshot-отчёты группируются по локации/статусу, а не по времени (D-02) — визуально отличаются от временных.
- Порог низкого остатка — default 2, уже зашит в БД-чтении; UI просто редактирует значение (SET-04).
- Универсальный PDF отчёта несёт ту же шапку организации/логотипа, что и документы (единый бренд-блок).

</specifics>

<deferred>
## Deferred Ideas

- **Корзина (UI над soft-delete)** — отложено из Phase 1; defer в backlog (или отдельная UX-фаза). Схема soft-delete готова с Phase 1. (D-21)
- **Настраиваемая компоновка дашборда** (drag/drop, скрытие виджетов) — defer в backlog; в Phase 7 фиксированная сетка. (D-10)
- **Визуальный WYSIWYG-редактор шаблонов** — defer; Phase 7 даёт raw MiniJinja + переменные + превью. (D-20)
- **Snapshot на произвольную историческую дату** («состояние на 30.09» через реконструкцию из audit_log) — defer; Phase 7 snapshot'ы только «сейчас». (D-02)
- **Авто-restart зависших Pantum-принтеров** — явно НЕ в v1 (см. PNT в REQUIREMENTS.md); Phase 7 только отображает проблемные в виджете.

None из обсуждения не вышло за границы фазы без явного отнесения в defer.

</deferred>

---

*Phase: 7-Отчёты, Дашборд и Настройки*
*Context gathered: 2026-06-15*
