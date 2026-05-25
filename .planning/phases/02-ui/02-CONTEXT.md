# Phase 2: Устройства и базовый UI - Context

**Gathered:** 2026-05-25
**Status:** Ready for planning
**Source:** /gsd-discuss-phase 2 --auto (gray areas auto-resolved with best-practice defaults; every D-* below is overridable in PLAN.md via `--power` or a CONTEXT-update)

<domain>
## Phase Boundary

Сделать первый end-to-end вертикальный срез приложения по разделу «Устройства» (DEV-01..13) и поставить навигационный каркас + тему + русскоязычный UI (UI-01..06). Phase 1 положила инфраструктуру (БД, миграции, writer-channel, AppCtx, tauri-specta, tracing); Phase 2 строит **поверх неё** первую реальную сущность с CRUD/поиском/автокомплитами/CSV — на десктопе через Tauri-invoke; HTTP-транспорт (axum + browser) появится в Phase 5, но Phase 2 уже закладывает **runtime-детекцию транспорта** (UI-05) чтобы тот же Svelte-бандл работал в обоих режимах.

**Не в scope этой фазы:**
- DEV-14, DEV-15 (документ приёма + история) — Phase 3.
- Login / AD-аутентификация — Phase 4 (пока работаем без auth, AppCtx::current_user возвращает «system»).
- Серверный режим (axum слушает порт) — Phase 5; в Phase 2 axum router строится и держится в AppCtx, но не bind'ится.
- Картриджи / акты / заявки — последующие фазы.

**Mode:** mvp — каждый план фазы должен делать тонкий вертикальный срез (UI → command → service → repo → DB), а не горизонтальный слой.

</domain>

<decisions>
## Implementation Decisions

### Backend layer

#### D-Repo-01: гексагональная раскладка для devices — port в trackly-core, repo в trackly-infra, service в trackly-app
- **`trackly-core::ports::devices`** — trait `DeviceRepository` с методами `create`, `update`, `delete_soft`, `get`, `list`, `search_fts`, `autocomplete_for_field`, `autocomplete_contextual` + domain types (`DeviceNew`, `DevicePatch`, `DeviceFilter`).
- **`trackly-infra::repos::devices_sqlite`** — `SqliteDeviceRepository` implementing the port; uses `&Connection` borrows (writer-conn для write-методов, reader-conn для read-методов).
- **`trackly-app::services::device_service`** — `DeviceService { writer: WriterHandle, readers: Arc<ReaderPool>, clock: Arc<dyn Clock> }`. Service оборачивает repo вызовы в `writer.execute(...)` для writes и `readers.acquire()` для reads. Здесь же — бизнес-валидация (обязательные поля, unique-constraints).
- **Tauri command + axum handler — thin adapter'ы**: вызывают `state.devices.create(dto)`, сериализуют результат. Никакой бизнес-логики.
- **Rationale:** Phase 1 SUMMARY 01-04 + ARCHITECTURE.md прямо требует этот раскладок. Service в `trackly-app` (НЕ `trackly-infra`) потому что он держит `WriterHandle` (тип из infra) И ничего не предоставляет наружу — это композиционный слой.

#### D-Schema-Phase2-01: схема devices УЖЕ создана в Phase 1 V003 — Phase 2 НЕ пишет новых миграций для базового CRUD
- `migrations/V003__devices.sql` уже создал таблицу `devices` со всеми колонками (per ROADMAP REQ-DEV-01: type, name, inventory_no, serial_no, model, specs, kit, state, location, status + standard4 created_at_utc/updated_at_utc/deleted_at_utc/version).
- `V012__indexes_and_fts.sql` создал FTS5-таблицу `devices_fts` НО **без триггеров синхронизации** (триггеры были вынесены за пределы Phase 1 per Plan 03 SUMMARY: "FTS5 triggers NOT created in Phase 1 — Phase 2/3/4 own devices_fts sync triggers when the corresponding write paths land").
- **Phase 2 пишет `migrations/V013__devices_fts_triggers.sql`** — `AFTER INSERT/UPDATE/DELETE ON devices` синхронизирующие триггеры на `devices_fts(name, inventory_no, serial_no, model)`. PRAGMA `user_version = 13` на конце.
- **Rationale:** Triggers — стандартный SQLite-паттерн для держать FTS5-индекс в sync. Plan 03 explicitly deferred this к фазам, которые добавляют CRUD-paths.

#### D-Autocomplete-01: DISTINCT-запросы с индексами, БЕЗ pre-computed таблицы
- `DEV-08` (per-field autocomplete): `SELECT DISTINCT <field> FROM devices WHERE deleted_at_utc IS NULL AND <field> LIKE ?1 ORDER BY <field> LIMIT 30`.
- `DEV-09` (contextual): когда выбран `name`, остальные дропдауны фильтруют `WHERE name = ?ctx AND <field> LIKE ?prefix`.
- **Индексы:** `V013` дополнительно добавляет partial индексы `(name, model)`, `(name, location)`, `(name, state)`, `(name, kit)`, `(name, specs)` — поддержка contextual lookup'а без full-scan.
- **НЕ заводим** отдельную `device_field_values`-таблицу или materialized view в v1 — для ~10k устройств DISTINCT с индексом отрабатывает <50ms. Если станет узким местом — отдельная фаза оптимизации.
- **Rationale:** Premature optimization избегаем. SUMMARY.md Phase 1 implications: «start simple, profile later». DISTINCT — родной SQLite паттерн, читается прозрачно в коде.

#### D-Group-01: критерии группировки не-уникальных устройств (DEV-03 / DEV-11)
- «Уникальное» = у устройства есть `inventory_no` ИЛИ `serial_no` (любой из них непуст).
- «Не-уникальное» = оба `inventory_no` и `serial_no` пусты (NULL).
- **Ключ группировки** для не-уникальных в табличном представлении: `(type, name, model, specs, kit, state, location, status)` — всё кроме (id, inventory_no, serial_no, *_at_utc, version).
- В UI: группированная строка показывает `count` + первые/typical значения; разворот показывает все исходные `id`.
- **Backend API:** `DeviceService::list_grouped(filter) -> Vec<DeviceGroup>` где `DeviceGroup { repr: DeviceDto, ids: Vec<i64>, count: i64 }`. Группировка — `GROUP BY` на SQL-уровне.
- **Rationale:** REQ-DEV-03 явно говорит «количество», DEV-11 — «развернуть». Группировка серверным `GROUP BY` дешевле, чем клиентская агрегация.

#### D-Search-01: FTS5 trigger-synced, query через `MATCH` оператор
- Триггеры в `V013` (см. D-Schema-Phase2-01).
- Query pattern: `SELECT d.* FROM devices d JOIN devices_fts f ON d.id = f.rowid WHERE f MATCH ?1 AND d.deleted_at_utc IS NULL`.
- Префикс-search: `name*` (`'lenovo*'`); пользователь вводит free-form, мы добавляем `*` к каждому терму через простой sanitizer (escape `'`, `"`, NUL).
- **Sanitizer:** `query.split_whitespace().map(escape).map(|t| format!("{t}*")).collect::<Vec<_>>().join(" ")`.
- **Rationale:** FTS5 нативный SQLite, без внешних деп. Trigger-sync гарантирует консистентность даже при ручных UPDATE.

#### D-CSV-01: импорт CSV с детектом кодировки и делимитера
- **Кодировка**: проверка BOM первой; если нет BOM — `chardetng = "0.1"` (port Mozilla UCD на Rust, чисто-Rust, без C-deps). Decoder — `encoding_rs = "0.8"` (де-факто стандарт в экосистеме). Поддерживаемые: UTF-8 (с/без BOM), CP1251.
- **Делимитер**: считаем `,` и `;` в первой непустой строке (вне кавычек) — побеждает большинство.
- **Парсер**: `csv = "1.3"` (workspace dep — уже есть transitively через rusqlite/tower).
- **Preview**: backend возвращает `CsvImportPreview { encoding: String, delimiter: char, headers: Vec<String>, rows: Vec<Vec<String>> /* первые 5 */, total_rows_estimated: Option<u64> }`.
- **Commit**: отдельный invoke `import_csv(file_id, mapping)` — `mapping: HashMap<column_name, device_field>` от UI; backend вставляет по одной строке через writer-channel; ошибки на каждую строку аккумулируются в `Vec<(row_index, AppError)>` и возвращаются в `CsvImportReport { inserted: u64, failed: Vec<RowError> }`.
- **Rationale:** Это устоявшийся pattern для русскоязычных приложений (БД в CP1251 у людей, Excel в UTF-8 у тех, кто помоложе). `chardetng` + `encoding_rs` — связка из rust-lang.

#### D-CSV-02: экспорт CSV — UTF-8 BOM + `;`-делимитер (RU Excel-friendly)
- BOM `\xEF\xBB\xBF` в начале → Russian Excel правильно распознаёт UTF-8.
- Делимитер `;` — default для RU-locale Excel; `,` ломается на русских разрядных запятых в числах.
- Header line на русском: `Тип;Наименование;Инвентарный №;...`.
- Tauri command: `export_devices_csv(filter) -> String` (содержимое файла) → frontend сохраняет через `tauri-plugin-dialog` save-dialog. HTTP route в Phase 5: `GET /api/v1/devices/export.csv?...`.
- **Rationale:** Эмпирический выбор для российской локали; задокументирован в обсуждениях Tauri-сообщества и `csv` README.

#### D-AutocompleteEndpoint-01: один endpoint `devices_autocomplete(field, prefix, ctx_name=None) -> Vec<String>`
- Универсальный API для DEV-08 и DEV-09 (контекст передаётся опционально).
- 30 результатов max, sorted ASC.
- **Rationale:** Один endpoint — один UI-hook на фронте.

#### D-DeviceHints-01: «подсказки» Состояние (DEV-10) — статический массив в DTO + UI, не БД
- 6 пресетов из REQ-DEV-10 не меняются юзером — они служат «quick-pick» рядом с autocomplete-полем «Состояние».
- Храним в `crates/trackly-app/src/dto/device.rs` как `pub const STATE_HINTS: &[&str] = &[...]`; UI получает через `bindings.ts` (specta-type для константного массива не получится, поэтому отдельный `#[tauri::command] fn device_state_hints() -> Vec<&'static str>`).
- **НЕ путать** с автокомплитом — это hint-кнопки ниже поля; кликнул — заполнилось.
- **Rationale:** Это UI-affordance, а не данные; в БД им делать нечего. Лежит ближе к коду, а не к схеме.

### Frontend layer

#### D-UI-Structure-01: feature-folders + shared `lib/`
```
ui/src/
├── App.svelte                  # router shell + theme apply
├── main.ts                     # bootstrap
├── lib/
│   ├── api/                    # apiClient — transport-detect + typed methods
│   │   ├── client.ts           # `apiCall<R>(name, args)` — dispatch invoke vs fetch
│   │   ├── devices.ts          # apiClient.devices.list/create/...
│   │   └── index.ts
│   ├── stores/                 # module-level $state runes
│   │   ├── theme.svelte.ts
│   │   ├── toast.svelte.ts
│   │   └── transport.svelte.ts
│   ├── components/             # cross-feature primitives
│   │   ├── Button.svelte
│   │   ├── Input.svelte
│   │   ├── Modal.svelte
│   │   ├── Toast.svelte
│   │   ├── ToastHost.svelte
│   │   ├── ThemeSwitcher.svelte
│   │   └── ...
│   └── utils/
├── features/
│   ├── layout/                 # sidebar + header + main area
│   │   ├── Layout.svelte
│   │   ├── Sidebar.svelte
│   │   └── sidebar-config.ts
│   └── devices/                # entire devices feature
│       ├── DevicesPage.svelte
│       ├── DeviceList.svelte
│       ├── DeviceFormModal.svelte
│       ├── DeviceFilters.svelte
│       ├── DeviceAutocompleteField.svelte
│       ├── DeviceImportCsvModal.svelte
│       └── api.ts              # feature-scoped wrappers around lib/api
└── styles/
    ├── _tokens.scss             # design tokens (Phase 1 scaffold — extend in Phase 2)
    └── global.scss
```
- **Rationale:** Feature-folders масштабируются на 8 фаз (Acts, Cartridges, Requests, Reports...) без перетасовок. `lib/` — горизонтальные примитивы.

#### D-UI-Router-01: `svelte-spa-router` (hash routing) — НЕ svelte-routing
- Plan 1 STACK.md упоминал оба; выбираем hash routing.
- **Hash (`#/devices/123`)** работает идентично в Tauri webview И в browser без серверных rewrites — в Phase 5 axum'у не нужно знать о маршрутах фронта.
- `svelte-spa-router` стабильный, Svelte-5-совместимый, ~6KB.
- **Rationale:** Один и тот же бандл, ноль конфига на server-side — критично для UI-05.

#### D-UI-State-01: Svelte 5 runes — module-level для app-state, локальные внутри компонентов
- `lib/stores/theme.svelte.ts` — `export const themeStore = $state({ value: 'system', resolved: 'light' })`, плюс mutator-функции.
- Аналогично `toast.svelte.ts`, `transport.svelte.ts`.
- Внутри компонентов — `let count = $state(0); let doubled = $derived(count * 2)`.
- **НЕ используем** легаси Svelte stores (`writable`/`readable`).
- **Rationale:** Runes — современный Svelte-5 идиом; module-level state via `$state` поддерживается официально (см. Svelte 5 docs «Sharing state»).

#### D-UI-Theme-01: data-theme attr + inline-script-в-`<head>` для no-flash
- `<html data-theme="dark">` — switching через JS `document.documentElement.dataset.theme = ...`.
- В `index.html` `<head>` — небольшой inline script (~15 строк), читает `localStorage.getItem('theme')` и `matchMedia('(prefers-color-scheme: dark)')`, выставляет `data-theme` ДО hydration → нет вспышки светлой темы при загрузке.
- `_tokens.scss`: `:root { --color-bg: white; ... } [data-theme="dark"] { --color-bg: #1a1a1a; ... }`.
- ThemeSwitcher.svelte — 3 radio: «Светлая / Тёмная / Системная»; при «Системная» подписывается на `matchMedia` change-event.
- **Rationale:** Стандартный pattern (Tailwind UI, Svelte tutorial), zero JS-deps, работает в SSR (хотя мы SSR не используем — но pattern проверенный).

#### D-UI-Transport-01: runtime-детекция через `'__TAURI_INTERNALS__' in window`
- Tauri 2 injects `window.__TAURI_INTERNALS__` в webview (в Tauri 1 был `__TAURI__`; в Tauri 2 — точное имя `__TAURI_INTERNALS__`).
- `lib/api/client.ts`:
  ```ts
  const isTauri = '__TAURI_INTERNALS__' in window;
  export async function apiCall<R>(name: string, args: object = {}): Promise<R> {
    if (isTauri) {
      const { invoke } = await import('@tauri-apps/api/core');
      return invoke<R>(name, args);
    }
    const res = await fetch(`/api/v1/${name}`, { method: 'POST', body: JSON.stringify(args), headers: { 'content-type': 'application/json' } });
    if (!res.ok) throw await parseAppError(res);
    return res.json();
  }
  ```
- Lazy dynamic import `@tauri-apps/api/core` чтобы он НЕ попадал в browser-only бандл, если Vite tree-shake'ает. Альтернатива: всегда import + проверка флага — Vite оба варианта окей.
- **Closes deferred-item Phase 1:** `ui/package.json` добавляет `@tauri-apps/api = "^2"` в dependencies. svelte-check перестанет падать на bindings.ts; `continue-on-error: true` на svelte-check в CI **удаляется** в этой фазе (см. D-Cleanup-01).

#### D-UI-Errors-01: toast-хост + парсинг AppError
- `lib/components/ToastHost.svelte` — единый хост, отображает stack из `toastStore`.
- `lib/stores/toast.svelte.ts`: `$state` массив `{ id, kind: 'error'|'success'|'info', message, ttl? }`.
- `apiClient` оборачивает каждый вызов try/catch; ловит `AppError`, парсит через `lib/api/errors.ts` (импортируется тип `AppError` из `bindings.ts`), показывает `toastStore.error(err.message)`.
- **Message:** `AppError.message` уже на русском (формируется на бэке); фронт просто показывает.
- **Без библиотеки:** свой `Toast.svelte` + хост ~80 LoC. `svelte-french-toast` (popular) добавляет ~10KB и conventions, которые нам не нужны.
- **Rationale:** UI-06. Своими руками — меньше зависимостей и больше контроля над i18n/стилями.

#### D-UI-Validation-01: ручная валидация форм через runes, без формовой библиотеки
- 4 обязательных поля (DEV-02): валидация — простой `$derived` flag per поле + `$derived` aggregate `canSubmit`.
- **НЕ используем** `formsnap`/`superforms` — они SvelteKit-привязаны и overkill для 4 полей.
- Сервер всё равно валидирует через `AppError::Validation { field, message }` — frontend показывает inline + toast.
- **Rationale:** Простота > обобщённость; в Phase 3+ (acts) добавим формы посложнее, тогда оценим необходимость библиотеки.

#### D-UI-Pagination-01: server-side пагинация, 50 строк/страница
- `DeviceService::list(filter, Pagination { offset, limit }) -> (Vec<DeviceDto>, total: u64)`.
- UI: простой `< 1 2 3 ... 12 >` + offset-jump; виртуализация (`svelte-virtual-list`) — deferred.
- 50 — emperical sweet spot для 1280×720 экрана (10-15 видимых + scroll).
- **Rationale:** Простой UX, легко тестировать. Виртуализация для 100k+ — задача отдельной фазы.

#### D-UI-Responsive-01: фиксированный sidebar 240px, контент-overflow
- min-target 1280×720 (UI-04). Sidebar — `width: 240px; flex-shrink: 0`. Контент — `flex: 1; overflow: auto`.
- На <1280: горизонтальный скролл всей страницы — приемлемо для desktop-приложения учёта.
- Адаптивных breakpoint'ов в v1 НЕ закладываем (это не SaaS-marketing-сайт).
- **Rationale:** Скоуп — desktop + LAN browser, не mobile. Не тратим время на mobile-first CSS.

#### D-UI-i18n-01: hard-coded русские строки в .svelte файлах; Paraglide deferred
- v1 — only Russian (CLAUDE.md). Hard-code строки прямо в компонентах: `<button>Создать устройство</button>`.
- **НЕ настраиваем** Paraglide-JS / `svelte-i18n` сейчас — добавляет boilerplate без value (одна локаль). Если в будущем понадобится — отдельная фаза «add i18n», ретрофит дешёвый (Paraglide compile-time, миграция через codemod).
- **Backend ошибки** уже на русском (AppError.message); они просто пробрасываются в UI.
- **Rationale:** YAGNI; v1 — Russian-only.

#### D-UI-Sidebar-01: точная структура (UI-01)
```
- Дашборд           (заглушка в v2 — пустая страница «Скоро»)
- Карта             (заглушка в v2 — «В разработке»)
---
- Устройства        ★ активный раздел в Phase 2
- Акты              (заглушка — Phase 3)
---
- Принтеры          (заглушка — Phase 5+ /монолог Phase 5 или 6)
- Картриджи         (заглушка — Phase 4)
- Заявки            (заглушка — Phase 5+)
---
- Отчёты            (заглушка)
- Пользователи      (заглушка — Phase 7)
---
- Настройки         (заглушка — Phase 7)
```
- Конфиг в `features/layout/sidebar-config.ts` — массив `SidebarItem | SidebarDivider`. Разделители `---` рендерятся отдельным компонентом.
- Все заглушки — `<div class="placeholder">Раздел в разработке</div>`. Они нужны чтобы layout/router работали и UI-01 был выполнен.
- **Rationale:** UI-01 явно перечисляет — пунктуально воспроизводим.

### Composition + Tests

#### D-AppCtx-Extension-01: AppCtx расширяется полем `devices: Arc<DeviceService>`
- Plan 04's `AppCtx { writer, readers, paths, config, clock, shutdown, log_guard, schema_version }` — Phase 2 добавляет 9-е поле.
- `AppCtx::build` вызывает `DeviceService::new(writer.clone(), readers.clone(), clock.clone())` после reader-pool inicialization.
- Cloneability: `AppCtx: Clone` уже декларирован (для Tauri State + axum State), `Arc<DeviceService>` дёшево клонируется.
- **Rationale:** Прямое продолжение паттерна Phase 1 — никаких структурных изменений.

#### D-Test-Phase2-01: интеграционные тесты — extend `test_writer_and_readers` через сервисы
- Plan 04 fixture `test_writer_and_readers() -> (WriterHandle, ReaderPool, TempDir)` — расширяем `test_device_service() -> (Arc<DeviceService>, TempDir)` ради читаемости.
- Тесты на каждый command (`devices_create`, `devices_list`, `devices_search`, `devices_autocomplete`, `devices_import_csv`, `devices_export_csv`) — отдельные integration-тесты в `crates/trackly-app/tests/devices_*.rs`.
- **CSV-тесты**: реальные файлы (UTF-8, UTF-8 BOM, CP1251 + `,`, CP1251 + `;`) в `crates/trackly-app/tests/fixtures/devices/`. Используем строку `"Сидоров-Петроградский Иван Александрович (ё) №42"` из CONTEXT Phase 1 specifics — фикстурная.
- **Tauri тесты**: для `#[tauri::command]`-функций тестируем `build_*` helper (как Plan 05's `build_health` pattern); полный `tauri::Builder` не запускаем.
- **Rationale:** Тот же тест-паттерн, что Phase 1, расширенный. CSV-фикстуры — реальные файлы, не строки в коде; ловит реальные encoding-баги.

#### D-Bindings-01: расширение `specta_export::builder` всеми Phase 2 commands
- `collect_commands![ health, devices_list, devices_get, devices_create, devices_update, devices_delete, devices_search, devices_autocomplete, devices_state_hints, devices_import_csv_preview, devices_import_csv_commit, devices_export_csv, devices_list_grouped ]`.
- `cargo test -p trackly-app --test export_bindings` — UPDATE assertions to include `DeviceDto`, `DeviceNew`, `DevicePatch`, `DeviceFilter`, `CsvImportPreview`, `CsvImportReport`, `DeviceGroup` substrings.
- Этот тест — gate против drift (как и для HealthDto).

#### D-Cleanup-01: закрываем Phase 1 deferred-items
- **`ui/package.json`**: добавить `@tauri-apps/api = "^2"` в `dependencies` (НЕ devDependencies — runtime для UI).
- **`.github/workflows/ci-fast.yml` + `ci-full.yml`**: убрать `continue-on-error: true` со step `pnpm svelte-check` (теперь bindings.ts резолвится — svelte-check должен быть зелёным и блокирующим). Обновить комментарий.
- **`.planning/phases/01-foundation/deferred-items.md`**: дописать «✅ Resolved in Phase 2: ...» к соответствующей записи (sed inline в одном из Phase 2 commits).
- **НЕ закрываем** `tests/export_bindings.rs` skip-on-windows (это требует upgrade specta-стека на stable Rust — пока не возможно; см. deferred-items.md).
- **Rationale:** Гигиена. Phase 2 — естественный момент закрыть эти долги.

### Claude's Discretion

Перечисленное ниже планировщик решает самостоятельно по плану реализации — пользователь не возражает:
- Точные имена fields в DTO (`DeviceDto`, `DeviceNew` и т.д.) — соблюдать snake_case в JSON (Phase 1 invariant).
- Структуру файлов под devices feature: можно дробить на больше/меньше .svelte файлов чем перечислено, главное — feature-folder остаётся.
- Конкретные tokens в `_tokens.scss` (цвета, типографика, spacing scale). База — Tailwind/Radix-inspired.
- Внутреннее API DeviceService::list_grouped (методы группировки можно вынести в `DeviceQuery` builder или оставить плоско).
- Конкретные индексы V013 (помимо обязательных name-prefix для контекста) — планировщик решает по EXPLAIN на тестовом dataset.
- Где хранить mapping для CSV import (in-memory только на время preview→commit, или persisted) — рекомендую in-memory + token (UUID), expires через 5 минут.
- Имена `data-theme` атрибута, theme presets — следовать индустрии (`light`/`dark`/`system`).
- Использовать ли `tauri-plugin-dialog` для file-pick (CSV import) и save (CSV export) — да, это стандарт; уже в plugins-workspace.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Project-Level (must-read)
- `CLAUDE.md` — стек, что НЕ использовать, version compatibility (Tauri 2, Svelte 5, rusqlite 0.38, refinery 0.9, axum 0.8, MSRV 1.88).
- `.planning/PROJECT.md` — vision, core value.
- `.planning/REQUIREMENTS.md` — DEV-01..13 + UI-01..06 точные формулировки.
- `.planning/ROADMAP.md` §«Phase 2: Устройства и базовый UI» — 5 success criteria.

### Phase 1 carry-forward (Phase 2 надстраивает поверх)
- `.planning/phases/01-foundation/01-CONTEXT.md` — 17 D-* решений Phase 1 (D-Schema-01..05, D-Workspace-01..02, D-AppError-01, D-WriterChannel-01, D-Logging-01, D-Test-01 и пр.).
- `.planning/phases/01-foundation/01-VERIFICATION.md` — что было доказано в Phase 1 (опираемся на это, не переделываем).
- `.planning/phases/01-foundation/01-04-SUMMARY.md` — публичный API: `WriterHandle::execute<F,R>`, `ReaderPool::acquire()`, `AppError` 9 variants, `AppCtx::build`, `error_conversions::map_*`.
- `.planning/phases/01-foundation/01-05-SUMMARY.md` — pattern `build_*` helper + Tauri command + axum handler thin adapters + `specta_export::builder()`; HealthDto как эталон DTO.
- `.planning/phases/01-foundation/01-03-SUMMARY.md` — `db::migrations::run`, `embed_migrations!("../../migrations")` path, V003 + V012 уже задеплоены.
- `.planning/phases/01-foundation/deferred-items.md` — закрываем в этой фазе `@tauri-apps/api` runtime; НЕ закрываем `export_bindings` Windows skip.

### Research (общая для проекта)
- `.planning/research/ARCHITECTURE.md` — hexagonal layout, dual-transport pattern, единый `build_*` helper для двух транспортов.
- `.planning/research/STACK.md` — pinned versions (Svelte 5.55+, Vite 6, svelte-spa-router как опция).
- `.planning/research/PITFALLS.md` — top 15 pitfalls; для Phase 2 особо актуальны #6 (cyrillic paths в CSV), #5 (auth gap — для DEV-* пока пропускаем user_id, ставим NULL).
- `.planning/research/SUMMARY.md` — общие resolved decisions.
- `.planning/research/FEATURES.md` — denormalized assigned_to и пр.; для devices актуально как контекст.

### External (для researcher-агента, если копать глубже)
- Tauri 2 invoke + State: https://v2.tauri.app/develop/calling-rust/
- tauri-specta v2 RC docs: https://github.com/specta-rs/tauri-specta
- Svelte 5 runes (sharing state): https://svelte.dev/docs/svelte/$state#Sharing-state
- svelte-spa-router: https://github.com/ItalyPaleAle/svelte-spa-router
- SQLite FTS5 triggers pattern: https://sqlite.org/fts5.html#external_content_tables
- `chardetng`: https://docs.rs/chardetng/
- `encoding_rs`: https://docs.rs/encoding_rs/
- `csv` crate: https://docs.rs/csv/
- axum 0.8 State extractor: https://docs.rs/axum/0.8/axum/extract/struct.State.html

</canonical_refs>

<code_context>
## Existing Code Insights (from Phase 1)

### Reusable Assets
- `crates/trackly-infra/src/db/writer_worker.rs::WriterHandle::execute<F,R>(&self, F) -> Result<R, AppError>` — ЕДИНСТВЕННЫЙ путь для writes (D-WriterChannel-01).
- `crates/trackly-infra/src/db/pools.rs::ReaderPool::acquire() -> ReaderHandle` (Deref→Connection, RAII) — все reads.
- `crates/trackly-infra/src/db/migrations.rs::max_known_version() -> u32` — current = 12; V013 в Phase 2 ⇒ 13.
- `crates/trackly-infra/src/test_support/test_app_ctx::test_writer_and_readers()` — fixture для тестов.
- `crates/trackly-infra/src/error_conversions::{map_rusqlite, map_refinery, map_send_timeout, map_oneshot_recv}` — конверторы (orphan-rule workaround); call site-side для каждого `.map_err()`.
- `crates/trackly-app/src/context.rs::AppCtx` — composition root; extend with `devices` field.
- `crates/trackly-app/src/dto/health.rs::HealthDto` — pattern для DTO (snake_case, manual derives, `specta::Type`).
- `crates/trackly-app/src/tauri_cmds/health.rs::build_health` — pattern для `build_*` shared helper.
- `crates/trackly-app/src/http/health.rs::router()` — pattern для axum роутера.
- `crates/trackly-app/src/specta_export.rs::builder()` — `collect_commands![...]` — добавляем все Phase 2 commands.
- `crates/trackly-core/src/error.rs::AppError` + `AppErrorRepr` — 9 variants + manual specta sibling.
- `crates/trackly-core/src/primitives::{Secret, Clock}` — discipline-types.
- `migrations/V003__devices.sql` — schema готова.
- `migrations/V012__indexes_and_fts.sql` — FTS5 table готов, triggers — НЕТ (расширяем в V013).
- `ui/src/App.svelte` — placeholder, будем заменять router shell.
- `ui/src/styles/_tokens.scss` — scaffold, расширяем design tokens.
- `ui/src/bindings.ts` — auto-generated, gitignored, регенерится через `pnpm prebuild`.
- `ui/package.json` — scripts уже настроены (`dev`, `build`, `prebuild`, `svelte-check`, `lint`); добавляем `@tauri-apps/api` dep.

### Established Patterns (locked, Phase 2 наследует)
- **Hexagonal:** trackly-core (no I/O), trackly-infra (adapters), trackly-app (composition).
- **Single-writer mpsc + spawn_blocking** — только `WriterHandle::execute` для writes.
- **DTO в trackly-app** (NOT trackly-core), `build_*` helpers shared between Tauri + axum.
- **snake_case в JSON** (НЕ camelCase) — consistent в обоих транспортах.
- **`AppError` unified shape** `{ code, message, details }` через `Serialize` + `AppErrorRepr` для specta.
- **`AppCtx: Clone`** — каждое поле `Arc<...>` или `Copy`.
- **UTC unix seconds** для timestamps (`*_at_utc INTEGER NOT NULL`); форматирование на UI.
- **`Secret<T>` discipline** — не появится в Phase 2 (auth — Phase 4), но pattern зафиксирован.
- **Все пути через `Paths::resolve`** — никаких `dirs::*_dir()` (clippy enforce).
- **tauri-specta export pinned** to `=2.0.0-rc.21` / `=2.0.0-rc.22` / `0.0.9` — НЕ обновляем (требует nightly, см. deferred-items.md).

### Integration Points
- **`AppCtx::build`** — добавляется конструкция `DeviceService` после reader-pool init.
- **`specta_export::builder()`** — `collect_commands![...]` расширяется до ~13 функций (health + 12 device-related).
- **axum `Router`** — `http::devices::router().merge(http::health::router())` в `AppCtx::build` (Phase 5 будет bind'ить порт).
- **Migrations**: новый `V013__devices_fts_triggers_and_autocomplete_indexes.sql`; `max_known_version()` → 13.
- **`tests/export_bindings.rs`** assertions расширяются (DeviceDto, etc).

### Not-yet-existing (нужно создать в Phase 2)
- `crates/trackly-core/src/ports/mod.rs` — впервые появится модуль portов.
- `crates/trackly-core/src/ports/devices.rs` — `DeviceRepository` trait.
- `crates/trackly-core/src/domain/mod.rs` — впервые появится модуль domain.
- `crates/trackly-core/src/domain/devices.rs` — domain types (`DeviceNew`, `DevicePatch`, фильтры).
- `crates/trackly-infra/src/repos/mod.rs` — впервые появится repos.
- `crates/trackly-infra/src/repos/devices_sqlite.rs` — SqliteDeviceRepository.
- `crates/trackly-app/src/services/mod.rs` — впервые появится services.
- `crates/trackly-app/src/services/device_service.rs` — DeviceService.
- `crates/trackly-app/src/dto/device.rs` — DeviceDto, DeviceNew, etc.
- `crates/trackly-app/src/tauri_cmds/devices.rs` — все devices commands.
- `crates/trackly-app/src/http/devices.rs` — axum router для devices.
- `crates/trackly-app/src/csv/` — csv encoding sniff + parsing helpers.
- Весь `ui/src/lib/`, `ui/src/features/layout/`, `ui/src/features/devices/`.

</code_context>

<specifics>
## Specific Ideas

- **Fixture string** для CSV/PDF тестов (наследуется из Phase 1): `«Сидоров-Петроградский Иван Александрович (ё) №42»` — присутствует и в CP1251, и в UTF-8 csv test files.
- **Sidebar order** строго по UI-01 — не менять; разделители `---` рисуем явно.
- **Theme storage key**: `localStorage.getItem('trackly:theme')` — namespaced, чтобы не конфликтовать с потенциальным embed'дингом.
- **Inline no-flash script** в `index.html` — минимальный, без зависимостей, ~15 LoC; ставится в `<head>` ДО `<script type="module">`.
- **Sidebar config — компилируется один раз**, не reactive (статичные секции). Активный раздел — derived из `$page` (свой store, обновляется роутером).
- **CSV preview limit**: ровно 5 строк (REQ-DEV-12 точно говорит), но parser держит файл целиком в памяти на время preview→commit окна (~5 мин TTL).
- **Контекстный autocomplete (DEV-09):** срабатывает ТОЛЬКО когда `name` уже выбран (не свободный текст). При свободном вводе name — обычный autocomplete по name.
- **Группировка не-уникальных:** UI-toggle «Группировать похожие» (вкл/выкл) — пользователь решает per-сессии. По умолчанию ВКЛ.

</specifics>

<deferred>
## Deferred Ideas

- **DEV-14, DEV-15** (документ приёма + история) — Phase 3.
- **CSV import column-mapping UI** (drag-and-drop сопоставление столбцов CSV → полей DEV) — Phase 2 включает simple mapping (autodetect по совпадению header'ов); ручной маппинг через расширенный UI — отдельная мини-фаза, если пользователи попросят.
- **Виртуализация списка устройств** (>1000 строк) — отдельная perf-фаза при необходимости.
- **i18n / multi-language** — отдельная фаза «add i18n via Paraglide», deferred (v1 — Russian-only).
- **Mobile-first responsive** — out-of-scope (CLAUDE.md: «не mobile-app»).
- **`featch`-keepalive / abort-on-route-change** — Phase 2 простой fetch без abort; добавим если будут гонки.
- **Dashboard, Карта, Принтеры, Картриджи, Заявки, Отчёты, Пользователи, Настройки разделы** — заглушки в Phase 2; реальная реализация — соответствующие фазы.
- **Authorization для DEV-* commands** — Phase 4 (RBAC); сейчас все commands открытые, audit_log пишет `user_id = NULL`.
- **HTTPS / cert generation** — Phase 5 (серверный режим).
- **Печатные формы / PDF** — Phase 3+ (Acts), Phase 6 (Reports).
- **Toast library** (`svelte-french-toast` и т.п.) — если домашний `Toast.svelte` окажется недостаточным, заменим.
- **Form library** (`formsnap`/`superforms`) — если формы в Phase 3+ станут сложнее, переоценим.
- **Specta upgrade** (rc.24+) — заблокировано stable-Rust (nightly only); ждём stable релиза `debug_closure_helpers`.

</deferred>

---

*Phase: 2-ui*
*Context gathered: 2026-05-25 via /gsd-discuss-phase --auto*
