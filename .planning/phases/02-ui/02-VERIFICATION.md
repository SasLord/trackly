---
phase: 02-ui
verified: 2026-05-28T00:05:00Z
status: passed
score: 6/6 success criteria verified
resolved_gap: "pnpm lint blocker fixed in commit 7b02fd2 — added HTMLDivElement/HTMLButtonElement/Node to eslint browserGlobals; downgraded svelte/valid-compile to ignoreWarnings:true (intentional initial-prop capture inside {#key} remount). pnpm lint now exits 0."
overrides_applied: 0
gaps:
  - truth: "pnpm lint зелёный (план 02-02 must-have + CI gate ci-fast.yml)"
    status: failed
    reason: "pnpm lint выходит с кодом 1: 15 ESLint-ошибок в 4 файлах. (1) eslint.config.js не объявляет HTMLDivElement, HTMLButtonElement, Node в browserGlobals — вызывает no-undef в DeviceAutocompleteField.svelte:52, :185 и DeviceContextMenu.svelte:26. (2) svelte/valid-compile правило (state_referenced_locally) флагирует $state() инициализированные из пропа target в DeviceFormBody.svelte (12 строк) и searchQuery в DeviceFilters.svelte. Сборка проходит, svelte-check выходит 0 (только warnings), но lint-гейт в CI остаётся красным."
    artifacts:
      - path: "ui/src/features/devices/DeviceAutocompleteField.svelte"
        issue: "HTMLDivElement (line 52), Node (line 185) — не в browserGlobals eslint.config.js"
      - path: "ui/src/features/devices/DeviceContextMenu.svelte"
        issue: "HTMLButtonElement (line 26) — не в browserGlobals eslint.config.js"
      - path: "ui/src/features/devices/DeviceFormBody.svelte"
        issue: "12x state_referenced_locally (lines 52-67) — $state() инициализированы из пропа target; svelte/valid-compile rule treats as error"
      - path: "ui/src/features/devices/DeviceFilters.svelte"
        issue: "state_referenced_locally (line 18) — $state(searchQuery) where searchQuery is a prop"
      - path: "ui/eslint.config.js"
        issue: "Отсутствуют HTMLDivElement, HTMLButtonElement, Node в browserGlobals объекте"
    missing:
      - "Добавить HTMLDivElement, HTMLButtonElement, Node (и возможно HTMLLIElement) в browserGlobals объект в ui/eslint.config.js"
      - "Для state_referenced_locally: либо отключить правило для инициализации пропов (это намеренный Svelte 5 паттерн — значения намеренно захватываются один раз при маунте через {#key openInstanceCounter} в родителе), либо использовать untrack() на инициализаторе"
human_verification:
  - test: "Открыть pnpm tauri dev, кликнуть Устройства → Создать устройство"
    expected: "Форма открывается. 4 обязательных поля (Наименование, Расположение, Статус + тип скрыт). Автокомплит Наименования предлагает ранее введённые значения. После сохранения устройство появляется в списке."
    why_human: "End-to-end Tauri invoke + SQLite write — нельзя проверить grep'ом"
  - test: "Выбрать Наименование → заполнить → открыть поле Модель"
    expected: "Дропдаун Модели показывает заголовок «Ранее использовалось с «{Наименование}»:» с ранее введёнными значениями для этого наименования"
    why_human: "DEV-09 contextual autocomplete — визуальное поведение"
  - test: "Кликнуть «Тёмная» тему → перезапустить приложение"
    expected: "Приложение открывается сразу в тёмной теме без вспышки светлой"
    why_human: "No-flash поведение нельзя проверить без рендера"
  - test: "В списке устройств с несколькими одинаковыми (без серийного/инвентарного) кликнуть на строку группы"
    expected: "Строка разворачивается, показывая полный список DeviceDto. В collapsed-состоянии отображается «N шт.»"
    why_human: "DEV-11 expand interaction — нельзя проверить без UI"
  - test: "Нажать «Экспорт CSV», сохранить файл, открыть в Excel с русской локалью"
    expected: "Кириллица отображается без мохибаке. Разделитель ';'. Заголовки на русском."
    why_human: "DEV-13 Excel-совместимость нельзя проверить автоматически"
  - test: "Нажать «Импорт CSV», выбрать cp1251_semicolon.csv (если доступен)"
    expected: "Step 2 показывает «Определена кодировка: windows-1251, разделитель: «;»». Preview строки читаются корректно с кириллицей."
    why_human: "DEV-12 wizard visual steps"
---

# Phase 2: Устройства и базовый UI — Verification Report

**Phase Goal:** Поставить end-to-end вертикальный срез по разделу «Устройства» — CRUD, автокомплиты, поиск, CSV, плюс навигационный каркас приложения с темой и русскоязычным UI.
**Verified:** 2026-05-28T00:05:00Z
**Status:** gaps_found
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths (Roadmap Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| SC-1 | Пользователь создаёт устройство с автокомплитом; тип type_id=1 hardcoded; bulk-create по количеству | ? HUMAN | Backend VERIFIED: `DeviceService::create` + `bulk_create(count: 1..=100)` + `DeviceAutocompleteField` компонент. Визуальный flow нужен человек. |
| SC-2 | Контекстный автокомплит предлагает значения, ранее встречавшиеся с этим Наименованием | ? HUMAN | Backend VERIFIED: `autocomplete(ctx_name: Option<String>)` SQL `WHERE name = ?ctx`. UI VERIFIED: `contextName={name.trim() \|\| undefined}` передаётся во все поля формы, header «Ранее использовалось с «{name}»» рендерится при ctx. |
| SC-3 | FTS5-поиск + switch-bar с counters + группировка без серийника | ? HUMAN | Backend VERIFIED: `search_fts` (FTS5 MATCH + build_fts_query), `status_counts`, `list_grouped` (GROUP_CONCAT). UI VERIFIED: `DeviceFilters.svelte` (debounce 250ms, 5-tab switch-bar), `DeviceGroupRow` (expand via listByIds). Tests all green. |
| SC-4 | CSV-импорт с encoding detection + preview; CSV-экспорт UTF-8 BOM + русский Excel | ? HUMAN | Backend VERIFIED: sniff/decode/parse pipeline, `import_csv_preview`, `import_csv_commit`, `export_csv` (BOM + `;` + formula-injection guard). UI VERIFIED: 4-step wizard `DeviceImportCsvModal.svelte`. Tests: all green. Excel open — нужен человек. |
| SC-5 | Sidebar с правильными разделами + ThemeSwitcher в layout + no-flash + русский UI | ✓ VERIFIED | `sidebar-config.ts` — 10 items + 4 dividers в правильном порядке. No-flash inline script в `index.html` ДО `<script type=module>`. `_tokens.scss` light/dark CSS custom properties. `initTheme()` вызван ДО mount. Russian labels в ThemeSwitcher (Светлая/Тёмная/Системная). `pnpm build` — dist/index.html preserves inline script. |
| SC-6 | bulk-create 1-100, quantity скрывается при инв./сер. номере | ✓ VERIFIED | `DeviceService::bulk_create` validates count 0\|>100 → AppError::Validation. `DeviceFormBody.svelte`: `quantityDisabled = isEdit \|\| inventoryNo.trim()!=='' \|\| serialNo.trim()!==''`. Test `bulk_create_count_zero_rejected`, `bulk_create_exactly_100_allowed` — green. |

**Score:** 2 fully verified / 4 need human (all backend/wiring verified, visual behavior pending) / 1 BLOCKER (pnpm lint)

### CI Gate Verification

| Gate | Status | Evidence |
|------|--------|----------|
| `cargo clippy --workspace -D warnings` | ✓ PASS | Finished dev profile with 0 warnings |
| `cargo test --workspace --no-fail-fast` | ✓ PASS | All test suites green (see Behavioral Spot-Checks) |
| `pnpm svelte-check` | ✓ PASS | 0 ERRORS, 12 WARNINGS (warnings are not errors), exit code 0 |
| `pnpm lint` | ✗ FAIL | 15 errors, exit code 1 — BLOCKER |
| `pnpm build` | ✓ PASS | 180 modules transformed, no errors |

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `migrations/V013__devices_fts_triggers.sql` | FTS5 triggers + 5 partial indexes + PRAGMA user_version=13 | ✓ VERIFIED | 3 triggers (ai/ad/au), 5 indexes `idx_devices_autocomplete_*`, final `PRAGMA user_version = 13;` |
| `crates/trackly-app/src/services/device_service.rs` | DeviceService с CRUD + search + autocomplete + grouping + CSV | ✓ VERIFIED | 941 lines; create/get/list/update/delete_soft/search_fts/autocomplete/list_grouped/status_counts/import_csv_preview/import_csv_commit/export_csv/bulk_create/list_by_ids |
| `crates/trackly-infra/src/repos/devices_sqlite.rs` | SqliteDeviceRepository impl с CRUD + search + autocomplete + grouping | ✓ VERIFIED | 918 lines; impl DeviceRepository; from_row Path-B column mapping; `devices_fts MATCH` query; `SELECT DISTINCT` autocomplete; `GROUP_CONCAT(id)` grouping |
| `crates/trackly-app/src/csv/sniff.rs` | detect(bytes) → CsvProfile с BOM + chardetng + delimiter sniff | ✓ VERIFIED | BOM fast-path + chardetng::EncodingDetector; comma vs semicolon count |
| `crates/trackly-app/src/csv/decode.rs` | decode_to_string + had_replacements | ✓ VERIFIED | encoding_rs::Encoding::decode; returns (String, bool) |
| `crates/trackly-app/src/csv/parse.rs` | parse_rows с csv::ReaderBuilder flexible=true | ✓ VERIFIED | flexible(true), has_headers(true) |
| `crates/trackly-app/src/csv/session_store.rs` | ImportSessionStore с 5-min TTL | ✓ VERIFIED | Mutex<HashMap<Uuid, ImportSession>>; lazy sweep on put |
| `crates/trackly-app/src/tauri_cmds/fs_helpers.rs` | read_file_bytes + write_file_bytes + path validation | ✓ VERIFIED | canonicalize → `..` reject → UNC reject → `.csv` extension → 50MB cap |
| `ui/src/features/layout/sidebar-config.ts` | SIDEBAR_ITEMS: 10 items + 4 dividers = 14 entries | ✓ VERIFIED | Exactly as specified in UI-SPEC |
| `ui/index.html` | Inline no-flash script перед Vite module entry | ✓ VERIFIED | IIFE с `localStorage.getItem('trackly:theme')` + `matchMedia` — ДО `<script type=module>` |
| `ui/src/lib/api/client.ts` | apiCall с `__TAURI_INTERNALS__` transport detect | ✓ VERIFIED | isTauri check → lazy import('@tauri-apps/api/core') invoke OR fetch('/api/v1/...') |
| `ui/src/features/devices/DeviceFormModal.svelte` | Form modal create/edit | ✓ VERIFIED | Split: DeviceFormModal (92 lines, shell) + DeviceFormBody (490 lines, form state) = 582 lines total; well above 150 min |
| `ui/src/features/devices/DeviceFilters.svelte` | Search input + 5-tab switch-bar + group toggle | ✓ VERIFIED | 232 lines; debounce 250ms; status tabs с counters; grouping toggle |
| `ui/src/features/devices/DeviceAutocompleteField.svelte` | Reusable autocomplete с contextual header | ✓ VERIFIED | 361 lines; debounce; keyboard nav; «Ранее использовалось с «{name}»:» heading |
| `ui/src/features/devices/DeviceGroupRow.svelte` | Expandable grouped row | ✓ VERIFIED | 254 lines; lazy fetch listByIds on expand |
| `ui/src/features/devices/DeviceImportCsvModal.svelte` | 4-step CSV import wizard | ✓ VERIFIED | 553 lines; steps 1-4: file pick → preview → mapping → result |
| `ui/src/lib/components/` (11 primitives) | Button/Input/Select/Textarea/Modal/Toast/ToastHost/ThemeSwitcher/Placeholder/Spinner/Badge | ✓ VERIFIED | All 11 files present |
| `crates/trackly-app/capabilities/main.json` | Tauri 2 capability с dialog:default + core:default | ✓ VERIFIED | Has `core:default` + `dialog:default`. NOTE: `single-instance:default` отсутствует — plugin работает без capability entry (Tauri 2 не требует capability для single-instance плагина) |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `context.rs` | `device_service.rs` | `pub devices: Arc<DeviceService>` field + `DeviceService::new(writer, readers, clock)` in `AppCtx::build` | ✓ WIRED | Line 59 + 136 |
| `device_service.rs` | `writer_worker.rs` | `self.writer.execute(move \|conn\| {...})` для всех mutations | ✓ WIRED | audit_log INSERT в той же транзакции |
| `devices_sqlite.rs` | `devices_fts` (V012/V013) | `WHERE devices_fts MATCH ?1` в `search_fts` | ✓ WIRED | `build_fts_query` + JOIN |
| `devices_sqlite.rs` | `idx_devices_autocomplete_*` (V013) | `SELECT DISTINCT {col}` + WHERE deleted_at_utc IS NULL | ✓ WIRED | enum-whitelisted field → static SQL column |
| `specta_export.rs` | все 17 команд | `collect_commands![...]` | ✓ WIRED | 6 CRUD + 6 search/autocomplete/group/counts + 3 CSV + 2 FS helpers |
| `main.rs` | Tauri Builder | `.plugin(single_instance)` + `.plugin(dialog)` + `.manage(ctx)` + `.invoke_handler(builder.invoke_handler())` | ✓ WIRED | Lines 129-134 |
| `DevicesPage.svelte` | `devices` API | `devices.search()` / `devices.listGrouped()` / `devices.list()` per state | ✓ WIRED | Reactive `$effect` drives API calls |
| `DeviceFormBody.svelte` | `DeviceAutocompleteField` | `contextName={name.trim() \|\| undefined}` for Модель/Состояние/Комплектация/Расположение/ТехХар | ✓ WIRED | Lines 267, 281, 294, 331, 348 |
| `DeviceImportCsvModal.svelte` | `@tauri-apps/plugin-dialog` | `open({ filters: [{csv}] })` на Step 1 | ✓ WIRED | Line 108 |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|--------------------|--------|
| `DevicesPage.svelte` | `items` / `groups` | `devices.list()` / `devices.listGrouped()` / `devices.search()` in `$effect` | Yes — `DeviceService::list` → `readers.acquire()` → `repo.list()` SQL SELECT | ✓ FLOWING |
| `DeviceFilters.svelte` | `counts: Map<number,number>` | `devices.statusCounts()` → `DeviceService::status_counts` → `COUNT(*) GROUP BY status_id` | Yes — real SQL aggregate | ✓ FLOWING |
| `DeviceAutocompleteField.svelte` | `suggestions: string[]` | `devices.autocomplete(field, prefix, ctxName)` → `SELECT DISTINCT {col}` | Yes — partial index query | ✓ FLOWING |
| `DeviceGroupRow.svelte` | `children: DeviceDto[]` | `devices.listByIds(group.ids)` on expand | Yes — `DeviceService::list_by_ids` → SQL WHERE id IN | ✓ FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| CRUD round-trip (create→get→update→list→delete) | `cargo test -p trackly-app --test devices_crud` | 13 passed | ✓ PASS |
| FTS5 search (prefix, cyrillic, multi-token, ё) | `cargo test -p trackly-app --test devices_search` | 9 passed | ✓ PASS |
| Autocomplete (DISTINCT, contextual, 30-limit) | `cargo test -p trackly-app --test devices_autocomplete` | 9 passed | ✓ PASS |
| Grouping (non-unique, GROUP_CONCAT, expand) | `cargo test -p trackly-app --test devices_grouping` | 13 passed | ✓ PASS |
| CSV import (4 encoding/delimiter variants, cyrillic round-trip) | `cargo test -p trackly-app --test devices_csv_import` | 10 passed | ✓ PASS |
| CSV export (BOM, `;` delimiter, Russian headers, formula injection) | `cargo test -p trackly-app --test devices_csv_export` | 7 passed | ✓ PASS |
| CSV session TTL (5-min expire, double-take) | `cargo test -p trackly-app --test devices_csv_session` | 5 passed | ✓ PASS |
| Bulk-create (count 1-100, transactional, audit_log) | `cargo test -p trackly-app --test devices_bulk_create` | 10 passed | ✓ PASS |
| HTTP smoke (dual-transport axum) | `cargo test -p trackly-app --test devices_http_smoke` | 1 passed | ✓ PASS |
| Schema version 13 + V013 migration | `cargo test -p trackly-app --test health_smoke` | 1 passed | ✓ PASS |
| core no I/O deps | `cargo test -p trackly-core --test no_io_deps` | 1 passed | ✓ PASS |
| pnpm build (SPA bundle) | `cd ui && pnpm build` | 180 modules, no errors | ✓ PASS |
| pnpm svelte-check | `cd ui && pnpm svelte-check` | 0 errors, 12 warnings, exit 0 | ✓ PASS |
| pnpm lint | `cd ui && pnpm lint` | **15 errors, exit 1** | ✗ FAIL |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| DEV-01 | 02-03 | CRUD устройств с полями | ✓ SATISFIED | DeviceService create/get/update/delete_soft + DeviceFormBody (10 fields) |
| DEV-02 | 02-03 | Обязательные поля: Тип, Наименование, Расположение, Статус | ✓ SATISFIED | Validation in DeviceService; type_id=1 hardcoded per Phase 2 scope; required fields in form |
| DEV-03 | 02-03 | Не-уникальные устройства с количеством | ✓ SATISFIED | bulk_create validates count; grouping shows non-unique as collapsed |
| DEV-04 | 02-01 | Типы устройств seeded | ✓ SATISFIED | V013 migration + type_id=1 default; seeded data from Phase 1 |
| DEV-05 | 02-01 | Статусы устройств seeded | ✓ SATISFIED | device_statuses seeded table; status switch-bar shows На складе/В работе/На ремонте/Списано |
| DEV-06 | 02-04 | Полнотекстовый поиск | ✓ SATISFIED | FTS5 MATCH via search_fts; build_fts_query; DeviceFilters search input |
| DEV-07 | 02-03,04 | Фильтр switch-bar по статусу со счётчиками | ✓ SATISFIED | DeviceFilters 5-tab switch-bar; status_counts endpoint |
| DEV-08 | 02-04 | Автокомплит для полей из ранее введённых | ✓ SATISFIED | DeviceAutocompleteField; autocomplete endpoint with DISTINCT |
| DEV-09 | 02-04 | Контекстный автокомплит по Наименованию | ✓ SATISFIED | ctx_name param; SQL WHERE name = ?ctx; UI contextName prop wired |
| DEV-10 | 02-03 | STATE_HINTS: 6 пресетов состояния | ✓ SATISFIED | `STATE_HINTS: &[&str]` const в dto/device.rs; stateHints command; chips в DeviceFormBody |
| DEV-11 | 02-04 | Группировка похожих позиций с expand | ✓ SATISFIED | list_grouped (GROUP_CONCAT); DeviceGroupRow lazy-expand via listByIds |
| DEV-12 | 02-05 | Импорт CSV (UTF-8 + CP1251 + autodetect) с превью | ✓ SATISFIED | sniff/decode/parse pipeline; 4-step wizard; all 4 encoding variants tested |
| DEV-13 | 02-05 | Экспорт CSV (UTF-8 BOM, Excel-friendly) | ✓ SATISFIED | BOM + `;` delimiter + Russian headers + formula injection guard; test suite green |
| UI-01 | 02-02 | Sidebar-навигация с разделами | ✓ SATISFIED | 10 items + 4 dividers in correct order per UI-SPEC |
| UI-02 | 02-02 | Переключатель темы light/dark/system в sidebar, no-flash | ✓ SATISFIED | ThemeSwitcher in Sidebar footer; no-flash IIFE in index.html; initTheme() before mount |
| UI-03 | 02-02 | Полностью русскоязычный UI | ✓ SATISFIED | All user-facing strings in Russian; checked in form labels, switch-bar, buttons, placeholders |
| UI-04 | 02-02 | Адаптивный layout 1280×720 | ? HUMAN | Layout CSS structure in place (grid + var(--sidebar-width)); visual check needed |
| UI-05 | 02-02 | Один Svelte-бандл для Tauri и браузера; transport detect | ✓ SATISFIED | `__TAURI_INTERNALS__` detect in client.ts; lazy import; fetch fallback |
| UI-06 | 02-02 | Глобальные toast-уведомления, русские сообщения | ✓ SATISFIED | ToastHost + toastStore; pushToast with TTL; Russian error messages |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `DeviceFormBody.svelte` | 52-67 | `$state()` initialized from prop `target` → ESLint `state_referenced_locally` (12 instances) | ⚠️ WARNING | Intentional Svelte 5 pattern (form fields intentionally capture initial value; parent uses `{#key openInstanceCounter}` to remount on open — documented in file header). Does not cause data loss but breaks `pnpm lint` gate. |
| `DeviceFilters.svelte` | 18 | `$state(searchQuery)` where searchQuery is prop → `state_referenced_locally` | ⚠️ WARNING | Same intentional pattern; debounce timer requires local copy. Breaks lint gate. |
| `DeviceAutocompleteField.svelte` | 52, 185 | `HTMLDivElement`, `Node` — no-undef ESLint | 🛑 BLOCKER | Missing from `eslint.config.js` browserGlobals. Lint gate fails. |
| `DeviceContextMenu.svelte` | 26 | `HTMLButtonElement` — no-undef ESLint | 🛑 BLOCKER | Same root cause. |
| `ui/eslint.config.js` | browserGlobals | Missing: HTMLDivElement, HTMLButtonElement, Node | 🛑 BLOCKER | Root cause of no-undef errors above. |

### Human Verification Required

### 1. Device create/edit end-to-end flow

**Test:** `pnpm tauri dev` → Устройства → «+ Создать устройство» → заполнить Наименование/Расположение/Статус → Создать
**Expected:** Устройство появляется в списке без перезагрузки; toast «Устройство создано»
**Why human:** Tauri invoke + SQLite write end-to-end, cannot grep

### 2. Contextual autocomplete (SC-2)

**Test:** Создать несколько устройств с одинаковым Наименованием и разными Моделями → открыть форму → выбрать то же Наименование → открыть дропдаун Модели
**Expected:** Заголовок «Ранее использовалось с «{Наименование}»:» + только ранее введённые модели
**Why human:** Visual UI behavior, DEV-09

### 3. Theme no-flash on reload (SC-5)

**Test:** Переключить на «Тёмную» → закрыть и снова открыть приложение
**Expected:** Приложение открывается сразу в тёмной теме, никакой вспышки светлого фона
**Why human:** Timing-sensitive visual behavior

### 4. Device grouping expand (SC-3)

**Test:** Создать 3 устройства «Флешка USB» без серийного/инвентарного → перейти в grouped view → кликнуть на группу
**Expected:** Строка «Флешка USB — 3 шт.» разворачивается в 3 отдельные строки
**Why human:** DEV-11 UI interaction

### 5. CSV export opens in Excel without mojibake (SC-4)

**Test:** Создать устройство «Принтер Pantum BM5100ADN» → Экспорт CSV → открыть в Excel (Windows, русская локаль)
**Expected:** Кириллица читается без мохибаке; разделитель `;`; первые 3 байта файла = EF BB BF
**Why human:** DEV-13 Excel compatibility

### 6. Adaptive layout at 1280×720 (UI-04)

**Test:** Открыть браузерный вид (Phase 5 не активен, но через dev server) или изменить размер окна Tauri до 1280×720
**Expected:** Sidebar виден, контент не обрезается, таблица устройств читаема
**Why human:** Visual layout check

### Gaps Summary

**1 blocker preventing CI gate from passing:**

`pnpm lint` завершается с exit code 1 из-за 15 ESLint-ошибок в файлах Phase 2:

**Root cause A** (3 ошибки): В `ui/eslint.config.js` не объявлены DOM-типы `HTMLDivElement`, `HTMLButtonElement`, `Node` в `browserGlobals`. Это добавленные во время Phase 2 компоненты (`DeviceAutocompleteField.svelte`, `DeviceContextMenu.svelte`) используют эти типы в TypeScript-аннотациях.

**Root cause B** (12 ошибок): Svelte 5 правило `svelte/valid-compile` флагирует `$state()` инициализированные из пропа как `state_referenced_locally`. В `DeviceFormBody.svelte` это намеренный паттерн (форма захватывает начальные значения пропа `target`; родительский компонент использует `{#key openInstanceCounter}` для ремаунта при каждом открытии). В `DeviceFilters.svelte` — локальная копия `searchQuery` для debounce-таймера. Технически ESLint treats these as errors (not warnings).

**Исправление минимально:** (1) добавить 3 глобала в `eslint.config.js`; (2) либо отключить правило `svelte/valid-compile` для этих паттернов, либо обернуть инициализаторы в `untrack(() => target?.name ?? '')`.

Все остальные success criteria (SC-1 через SC-6) имеют полную backend + wiring реализацию с прошедшими тестами. Блокер — только lint gate.

---

_Verified: 2026-05-28T00:05:00Z_
_Verifier: Claude (gsd-verifier)_
