---
phase: 04-cartridges
verified: 2026-06-12T00:00:00Z
status: human_needed
score: 12/12 must-haves verified
overrides_applied: 0
deferred:
  - truth: "Баннер «низкий остаток» на дашборде (CART-12 dashboard half)"
    addressed_in: "Phase 7"
    evidence: "Phase 4 Success Criterion #5 explicitly scopes banner to 'в разделе «Картриджи»'; DASH-02 (Phase 7) covers 'Виджет «Картриджи»: разбивка по статусам + alert о низком остатке'"
human_verification:
  - test: "Запустить приложение `cargo tauri dev`, открыть раздел «Картриджи» и выполнить полный lifecycle-сценарий"
    expected: "Создание модели с матрицей совместимости, создание экземпляра с авто-кодом C-000001, установка в принтер, возврат на склад, поиск, баннер low stock — всё работает без ошибок"
    why_human: "Интерактивный UI-тест: focus-open autocomplete в CompatibilityEditor, portal-меню CartridgeContextMenu, Toast-уведомления, реактивное обновление счётчиков после операций — не верифицируются grep-ом"
  - test: "Создать фотобарабан в форме ModelFormModal и убедиться, что поле «Цвет» скрыто при kind_id=2"
    expected: "Поле «Цвет» не отображается, поле «Тип» установлено в «Фотобарабан»"
    why_human: "Conditional rendering {#if kindId !== 2} правильно написан в коде, но визуальное поведение формы при выборе типа требует ручной проверки"
  - test: "Focus-open autocomplete в CompatibilityEditor при фокусе на «Бренд принтера» без ввода"
    expected: "Выпадающий список открывается сразу при фокусе и показывает ранее введённые бренды (пустой список на первом запуске)"
    why_human: "Поведение focus-open (prefix='') требует интерактивного тестирования в браузере/webview"
  - test: "Operaion Task 3 из 04-06-PLAN.md — полный сценарий human-verify checkpoint"
    expected: "Все 15 шагов сценария из PLAN 04-06 Task 3 выполняются успешно"
    why_human: "Это явный human-verify checkpoint из плана — ждёт ручного запуска приложения и подтверждения «approved»"
---

# Phase 04: Картриджи — Verification Report

**Phase Goal:** Поставить раздел «Картриджи» — модели с матрицей совместимости, экземпляры с авто-кодом `C-000001`, lifecycle с контекстными действиями, журнал перемещений и баннер низкого остатка.
**Verified:** 2026-06-12T00:00:00Z
**Status:** human_needed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths (Roadmap Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Пользователь создаёт модель с матрицей совместимых принтеров (Бренд+Модель, автокомплит), создаёт экземпляр с авто-кодом C-000001; коллизия не теряет счётчик | VERIFIED | `assign_code_in_tx` с retry-loop (`format!("C-{:06}", seq)`); `concurrent_50_unique_codes` тест зелёный; `collision_retry_does_not_lose_counter` зелёный; `upsert_compatibility_in_tx` + `CompatibilityEditor` + `suggestCompatPrinter` wire-up |
| 2 | Switch-bar показывает корректные счётчики; контекстное меню меняется по статусу | VERIFIED | `CartridgeFilters` получает `CartridgeCountsDto` от `statusCounts()` через `CartridgesPage.loadAll()`; `CartridgeContextMenu` генерирует `menuItems` через `$derived.by` по `status_id`; `use:portal` присутствует |
| 3 | Установка/возврат/заправка запрашивают нужные поля; история операций видна в карточке как хронологический список из audit_log | VERIFIED | `OperationModal` ветвится по `op`; `get_history` запрашивает `audit_log WHERE entity_type='cartridge'`; `CartridgeDetail` рендерит `history` через `{#each}`; `CartridgesPage` вызывает `getHistory(id)` при выборе картриджа |
| 4 | Поиск по картриджам находит совпадения по коду, модели, расположению | VERIFIED | `search()` в `cartridges_sqlite.rs` использует FTS UNION LIKE CTE; тесты `search_by_code`, `search_by_model_brand`, `search_by_location`, `empty_query_returns_all` — все зелёные; `CartridgesSearchAndTabs` вызывает `cartridges.search()` при debounce 250ms |
| 5 | Баннер «низкий остаток» в разделе «Картриджи» показывается когда количество ниже порога | VERIFIED | `low_stock()` читает `app_settings.low_stock_threshold`; `LowStockBanner` условно рендерится (`{#if items.length > 0}`); `CartridgesPage` передаёт `lowStockItems` из `cartridges.lowStock()`; тест `low_stock_returns_model_below_threshold` зелёный |

**Score:** 5/5 roadmap success criteria verified

### Plan must_haves (additional detail)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 04-01 | V016 миграция: cartridge_kinds (2 строки), app_settings, 3 FTS-триггера, PRAGMA 16 | VERIFIED | Файл существует, содержит все DDL, `PRAGMA user_version = 16`; триггеры `cartridges_fts_ai/ad/au` присутствуют |
| 04-02 | CartridgeRepository trait + SqliteCartridgeRepository | VERIFIED | `pub trait CartridgeRepository` в `ports/cartridges.rs`; `impl CartridgeRepository for SqliteCartridgeRepository` в `cartridges_sqlite.rs` (1223 строки) |
| 04-03 | CartridgeService.create() с validate_create + AppCtx.cartridges + все 6 тестов GREEN | VERIFIED | `validate_create` проверяет: пустой код, >32 символа, управляющие символы; `AppCtx.cartridges: Arc<CartridgeService>`; `cargo test` — 23/23 тестов зелёных |
| 04-04 | Switch-bar 5 вкладок + список с Badge + DetailPanel с историей | VERIFIED | `CartridgeFilters` со статусами; `CartridgeListRow` с Badge по `status_id`; `CartridgeDetail` рендерит `history` секцию |
| 04-05 | CartridgeContextMenu + OperationModal + CartridgeFormModal + LowStockBanner | VERIFIED | Все 4 компонента существуют и substantive; `OperationModal` вызывает `cartridges.transition()`; `CartridgeFormModal` содержит `openInstanceCounter` паттерн |
| 04-06 | Models CRUD (ModelsList + ModelFormModal + CompatibilityEditor) + финальный wire-up | VERIFIED | 4 новых компонента созданы; `ModelFormModal` передаёт `suggestCompatPrinter` в `CompatibilityEditor`; `CartridgesPage` является единственным оркестратором |

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `migrations/V016__cartridges_kind_color_settings.sql` | DDL картриджей, FTS триггеры, PRAGMA 16 | VERIFIED | 79 строк; все DDL присутствуют |
| `crates/trackly-core/src/domain/cartridges.rs` | Domain structs, CartridgeTransitionOp | VERIFIED | 342 строки; CartridgeRow, CartridgeModelRow, CartridgeTransitionOp, LowStockItem |
| `crates/trackly-core/src/ports/cartridges.rs` | CartridgeRepository trait | VERIFIED | Содержит `pub trait CartridgeRepository` |
| `crates/trackly-infra/src/repos/cartridges_sqlite.rs` | SqliteCartridgeRepository + SQL helpers | VERIFIED | 1223 строки; assign_code_in_tx, transition_in_tx, low_stock, get_history |
| `crates/trackly-app/src/services/cartridge_service.rs` | CartridgeService + validate_create | VERIFIED | 817 строк; validate_create с тремя проверками |
| `crates/trackly-app/src/dto/cartridge.rs` | CartridgeDto, CartridgeTransitionPayload, etc. | VERIFIED | Существует; bindings.ts содержит CartridgeDto, CartridgeTransitionPayload |
| `crates/trackly-app/src/tauri_cmds/cartridges.rs` | 19 Tauri commands | VERIFIED | cartridges_list, cartridges_transition, cartridges_low_stock, cartridges_get_history и др. зарегистрированы |
| `crates/trackly-app/src/http/cartridges.rs` | axum router() | VERIFIED | Существует; router построен, не bind'ится (Phase 5) |
| `crates/trackly-app/src/context.rs` | AppCtx.cartridges: Arc<CartridgeService> | VERIFIED | Строка 76: `pub cartridges: Arc<CartridgeService>` |
| `ui/src/features/cartridges/api.ts` | cartridges API объект | VERIFIED | Все методы: list, get, create, transition, statusCounts, getHistory, lowStock, etc. |
| `ui/src/features/cartridges/CartridgesPage.svelte` | Оркестратор с loadAll() | VERIFIED | `Promise.all([refresh(), refreshCounts(), refreshLowStock()])` в `loadAll()` |
| `ui/src/features/cartridges/CartridgeDetail.svelte` | Детальная панель с историей | VERIFIED | Рендерит `history` через `{#each history as entry}` |
| `ui/src/features/cartridges/CartridgeContextMenu.svelte` | Status-dependent меню с portal | VERIFIED | `use:portal`; `$derived.by` генерирует `menuItems` по `status_id` |
| `ui/src/features/cartridges/OperationModal.svelte` | Параметризованная модалка 5 операций | VERIFIED | Ветвится по `op`; вызывает `cartridges.transition()` |
| `ui/src/features/cartridges/LowStockBanner.svelte` | Баннер с условным рендером | VERIFIED | `{#if items.length > 0}` |
| `ui/src/features/cartridges/CompatibilityEditor.svelte` | Focus-open autocomplete пар | VERIFIED | `suggestBrandFn`/`suggestModelFn` props; ModelFormModal передаёт `suggestCompatPrinter` |
| `ui/src/features/cartridges/ModelFormModal.svelte` | CRUD модели с CompatibilityEditor | VERIFIED | `{#if kindId !== 2}` для цвета; `suggestCompatPrinter` x2 |
| `ui/src/features/layout/sidebar-config.ts` | Route /cartridges → CartridgesPage | VERIFIED | Строка 15: `{ kind: 'item', route: '/cartridges', label: 'Картриджи' }` |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `CartridgesPage.svelte` | `api.ts` | `import { cartridges } from './api'` + `cartridges.statusCounts()` | WIRED | Verified by grep |
| `CartridgeDetail.svelte` | history prop | `history: AuditEntryDto[]` из CartridgesPage | WIRED | `CartridgesPage` вызывает `getHistory(id)` и передаёт через prop |
| `CartridgesPage.svelte` | `CartridgeContextMenu.svelte` | `onMenuAction` callback chain через `CartridgeListRow` | WIRED | `CartridgeListRow` импортирует `CartridgeContextMenu`; callbacks wire-up verified |
| `OperationModal.svelte` | `api.ts` | `cartridges.transition(buildPayload())` | WIRED | Строка 189 |
| `ModelFormModal.svelte` | `CompatibilityEditor.svelte` | `bind:compatibility` | WIRED | suggestBrandFn/suggestModelFn props передаются |
| `CompatibilityEditor.svelte` | `api.ts` | `suggestCompatPrinter('brand'/'model', prefix)` | WIRED | Строка 428-429 ModelFormModal |
| `specta_export.rs` | `tauri_cmds/cartridges.rs` | `collect_commands!` | WIRED | cartridges_list, cartridges_transition и др. зарегистрированы |
| `context.rs` | `cartridge_service.rs` | `Arc<CartridgeService>` в AppCtx | WIRED | Строка 76 context.rs |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|--------------|--------|-------------------|--------|
| `CartridgesPage` → `CartridgeFilters` | `counts: CartridgeCountsDto` | `cartridges.statusCounts()` → `cartridges_status_counts` Tauri cmd → `CartridgeService.status_counts()` → SQL COUNT queries | Yes — 5 SELECT COUNT per-status | FLOWING |
| `CartridgesPage` → `LowStockBanner` | `lowStockItems: LowStockItemDto[]` | `cartridges.lowStock()` → `cartridges_low_stock` → `SqliteCartridgeRepository.low_stock()` → reads `app_settings.low_stock_threshold` + GROUP BY | Yes — real DB query | FLOWING |
| `CartridgeDetail` | `history: AuditEntryDto[]` | `cartridges.getHistory(id)` → `cartridges_get_history` → `get_history()` → SELECT FROM `audit_log WHERE entity_type='cartridge'` | Yes — real audit_log query | FLOWING |
| `CartridgesPage` | `items: CartridgeDto[]` | `cartridges.list()` / `cartridges.search()` → SQL JOIN query `SELECT_CARTRIDGES` | Yes | FLOWING |
| `ModelListRow` | `instanceCount: number` | Hardcoded `0` passed from `ModelsList` | No — always 0, no DB query | HOLLOW_PROP (cosmetic — not blocking, see note) |

**Note on `instanceCount=0`:** CART-01 requires "CRUD моделей: Название, Цвет, Примечание, Совместимые принтеры" — no requirement for showing instance count. The SUMMARY documents this explicitly as cosmetic: "функционал отображения есть, значение будет «0 шт.»". This does not block any CART requirement.

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| cartridges_lifecycle — 6 tests | `cargo test -p trackly-app --test cartridges_lifecycle` | 6 passed, 0 failed | PASS |
| cartridges_numbering — concurrent 50 + collision retry | `cargo test -p trackly-app --test cartridges_numbering` | 2 passed, 0 failed | PASS |
| cartridges_low_stock — threshold from app_settings | `cargo test -p trackly-app --test cartridges_low_stock` | 3 passed, 0 failed | PASS |
| cartridges_crud — validate_create + auto-code | `cargo test -p trackly-app --test cartridges_crud` | 6 passed, 0 failed | PASS |
| cartridges_search — FTS + LIKE | `cargo test -p trackly-app --test cartridges_search` | 4 passed, 0 failed | PASS |
| cartridges_history — chronological | `cargo test -p trackly-app --test cartridges_history` | 2 passed, 0 failed | PASS |
| svelte-check | `pnpm svelte-check` | 0 errors, 28 warnings (style only) | PASS |
| pnpm lint | `pnpm lint` | All files use Prettier code style | PASS |

**Total backend test score:** 23/23 pass

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| CART-01 | 04-03, 04-06 | CRUD моделей: Название, Цвет, Примечание, Совместимые принтеры | SATISFIED | `ModelFormModal` + `cartridge_service.model_create/update/delete` + Tauri commands |
| CART-02 | 04-02, 04-06 | Совместимые принтеры: массив пар Бренд+Модель с автокомплитом | SATISFIED | `upsert_compatibility_in_tx`; `CompatibilityEditor` с `suggestCompatPrinter` focus-open |
| CART-03 | 04-02, 04-03 | CRUD экземпляров: Код, Модель, Состояние заряда, Расположение, Примечания | SATISFIED | `CartridgeService.create/update/delete`; `CartridgeFormModal` |
| CART-04 | 04-01, 04-02, 04-03 | Авто-код C-000001, потокобезопасно; пользователь может ввести свой | SATISFIED | `assign_code_in_tx` с retry-loop; `concurrent_50_unique_codes` тест зелёный |
| CART-05 | 04-04 | Switch-bar со счётчиками: Все/На складе/В работе/На заправке/Списано | SATISFIED | `CartridgeFilters` со `StatusCounts`; 5 статусов; данные из `statusCounts()` |
| CART-06 | 04-04, 04-05, 04-06 | Контекстные действия по статусу | SATISFIED | `CartridgeContextMenu` с `$derived.by` по `status_id`; все 5 операций |
| CART-07 | 04-03, 04-05 | Установка в принтер: Дата, Кто выдал, Кому выдал, Расположение | SATISFIED | `OperationModal` с `op='install'`; `CartridgeTransitionOp::Install` в сервисе |
| CART-08 | 04-03, 04-05 | Возврат на склад: Состояние заряда (default Пустой), Расположение, Примечания | SATISFIED | `OperationModal` с `op='return_to_stock'`; `stateId` default=3 (Пустой) |
| CART-09 | 04-03, 04-05 | Заправка/возврат с заправки аналогично выдаче/возврату | SATISFIED | `op='to_refill'` и `op='from_refill'` в OperationModal |
| CART-10 | 04-02, 04-04 | История перемещений в карточке экземпляра (хронологически из audit_log) | SATISFIED | `get_history()` из audit_log; `CartridgeDetail` рендерит историю; `history_is_chronological` тест зелёный |
| CART-11 | 04-02, 04-04 | Поиск по коду, модели, расположению | SATISFIED | FTS+LIKE UNION CTE в `search()`; `CartridgesSearchAndTabs` с debounce 250ms |
| CART-12 | 04-01, 04-02, 04-05 | Баннер низкого остатка в разделе «Картриджи» (dashboard — Phase 7) | SATISFIED (partial) | Баннер в разделе «Картриджи» — VERIFIED; dashboard-виджет — деферировано Phase 7 |

### Deferred Items

| # | Item | Addressed In | Evidence |
|---|------|-------------|----------|
| 1 | Баннер «низкий остаток» на дашборде (CART-12 dashboard part) | Phase 7 | Phase 4 Success Criterion #5 scopes to 'в разделе «Картриджи»'; DASH-02 Phase 7: 'Виджет «Картриджи»: разбивка по статусам + alert о низком остатке' |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `ui/src/features/cartridges/ModelsList.svelte` | 44 | `instanceCount={0}` hardcoded | Info | Cosmetic — CART-01 не требует отображения count; backend CartridgeModelDto не имеет поля instance_count |

No TBD/FIXME/XXX/unresolved debt markers found in any Phase 4 files.

The 28 svelte-check warnings are Svelte 5 runes style suggestions (`state_referenced_locally` — initial value capture in let bindings). These are not errors and the patterns are consistent with existing non-Phase-4 components in the codebase. Not blockers.

### Human Verification Required

#### 1. Полный end-to-end lifecycle в разделе «Картриджи»

**Test:** Выполнить сценарий из PLAN 04-06 Task 3 (шаги 1–15): создание модели с совместимостью, создание экземпляра C-000001, установка в принтер, возврат, switch-bar фильтрация, поиск, баннер low stock.
**Expected:** Все 15 шагов выполняются без ошибок. Toast уведомления появляются. Статус картриджа меняется корректно. Счётчики обновляются.
**Why human:** Интерактивный UI: portal-меню, Toast, реактивные счётчики, focus-open autocomplete — не верифицируются статическим анализом.

#### 2. Скрытие поля «Цвет» при выборе типа «Фотобарабан»

**Test:** Открыть форму «Новая модель картриджа», переключить тип с «Картридж» на «Фотобарабан».
**Expected:** Поле «Цвет» немедленно скрывается. При возврате к «Картридж» — поле снова появляется.
**Why human:** Conditional rendering `{#if kindId !== 2}` корректен в коде, но реактивное поведение при изменении select требует визуальной проверки.

#### 3. Focus-open autocomplete в CompatibilityEditor

**Test:** В форме «Новая модель картриджа» нажать «+ Добавить принтер», кликнуть в поле «Бренд принтера» без ввода символов.
**Expected:** Dropdown открывается сразу при фокусе (показывает ранее введённые бренды или пустой список при первом запуске). Аналогично для поля «Модель принтера» после ввода бренда.
**Why human:** Поведение `prefix=''` в focus-open — визуальное взаимодействие в браузере/webview.

#### 4. Human-verify checkpoint из PLAN 04-06 Task 3 (явный blocking gate)

**Test:** Запустить `cargo test && pnpm svelte-check && pnpm lint`, затем `cargo tauri dev`. Выполнить все 15 шагов из Task 3 раздела `<how-to-verify>`.
**Expected:** Написать "approved" если раздел работает корректно.
**Why human:** Это явный `type="checkpoint:human-verify" gate="blocking"` из плана — ожидает ручного подтверждения.

---

### Gaps Summary

No code gaps found. All 12 CART requirements are implemented and all automated checks pass.

The only pending item is the explicit `gate="blocking"` human-verify checkpoint from PLAN 04-06 Task 3 — which is a developer UAT, not a code gap.

---

_Verified: 2026-06-12T00:00:00Z_
_Verifier: Claude (gsd-verifier)_
