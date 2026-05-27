---
phase: 02-ui
plan: "04"
subsystem: devices
tags:
  - vertical-slice
  - devices
  - search
  - fts5
  - autocomplete
  - grouping
  - bulk-create
  - scope-extension
dependency_graph:
  requires:
    - 02-03  # DevicesPage / DeviceFormModal base
    - 02-02  # Svelte shell + theme
    - 02-01  # Device service, repo, migration V012/V013
  provides:
    - FTS5 search endpoint (devices_search)
    - Autocomplete endpoint (devices_autocomplete) with enum-whitelisted fields
    - Grouped list endpoint (devices_list_grouped) for non-unique devices
    - Status counts endpoint (devices_status_counts)
    - List-by-ids endpoint (devices_list_by_ids) for group expansion
    - Bulk-create endpoint (devices_bulk_create) — scope extension
    - DeviceFilters.svelte — FTS search input + status switch-bar + group toggle
    - DeviceGroupRow.svelte — expandable grouped row
    - DeviceAutocompleteField.svelte — reusable autocomplete with contextual header
    - DeviceFormModal — autocomplete fields + quantity input for bulk create
  affects:
    - crates/trackly-core/src/domain/devices.rs
    - crates/trackly-core/src/ports/devices.rs
    - crates/trackly-infra/src/repos/devices_sqlite.rs
    - crates/trackly-app/src/dto/device.rs
    - crates/trackly-app/src/services/device_service.rs
    - crates/trackly-app/src/tauri_cmds/devices.rs
    - crates/trackly-app/src/http/devices.rs
    - ui/src/features/devices/DevicesPage.svelte
    - ui/src/features/devices/DeviceFormModal.svelte
tech_stack:
  added:
    - FTS5 MATCH query builder (build_fts_query — escape, tokenise, prefix-star)
    - AutocompleteField enum (injection-safe column whitelist in trackly-core domain)
    - GROUP_CONCAT(id) pattern for list_grouped aggregate
  patterns:
    - FTS5 prefix search: each whitespace-token quoted + asterisk appended
    - Enum-whitelisted field names (prevents SQL column injection)
    - DeviceGroupRow lazy-expand pattern: children fetched once on first expand, invalidated on mutation
    - Svelte autocomplete with debounce + keyboard nav + click-outside
    - Svelte props debounce via localSearch $state + $effect sync
key_files:
  created:
    - ui/src/features/devices/DeviceFilters.svelte
    - ui/src/features/devices/DeviceGroupRow.svelte
    - ui/src/features/devices/DeviceAutocompleteField.svelte
    - crates/trackly-app/tests/devices_search.rs
    - crates/trackly-app/tests/devices_autocomplete.rs
    - crates/trackly-app/tests/devices_grouping.rs
    - crates/trackly-app/tests/devices_bulk_create.rs
  modified:
    - crates/trackly-core/src/domain/devices.rs
    - crates/trackly-core/src/ports/devices.rs
    - crates/trackly-infra/src/repos/devices_sqlite.rs
    - crates/trackly-app/src/dto/device.rs
    - crates/trackly-app/src/services/device_service.rs
    - crates/trackly-app/src/tauri_cmds/devices.rs
    - crates/trackly-app/src/http/devices.rs
    - crates/trackly-app/src/specta_export.rs
    - crates/trackly-app/tests/export_bindings.rs
    - ui/src/lib/api/devices.ts
    - ui/src/features/devices/DevicesPage.svelte
    - ui/src/features/devices/DeviceList.svelte
    - ui/src/features/devices/DeviceFormModal.svelte
decisions:
  - "AutocompleteField enum in trackly-core domain — whitelist approach chosen over SQL escape; variants map to static SQL column names via sql_column() method"
  - "Separate devices_status_counts command instead of merging into devices_list response — simpler model, counts refreshed independently from list"
  - "Bulk-create Tauri param renamed new→device — new is a reserved word in TypeScript; specta generates camelCase JS bindings from snake_case Rust"
  - "FTS5 ё/е not cross-normalized in SQLite 3.51.0 with unicode61 remove_diacritics 2 — ё is indexed as-is, not collapsed to е; test reflects actual behaviour, cross-variant search deferred to Phase 4+"
  - "Bulk create inserts N rows in single transaction with snapshot timestamp — all-or-nothing semantics, one commit for entire batch"
metrics:
  duration: "~120 min"
  completed_date: "2026-05-26"
  tasks: 3
  files_created: 7
  files_modified: 13
---

# Phase 02 Plan 04: FTS5 Search, Autocomplete, Grouping + Bulk Create — Summary

**One-liner:** FTS5 search + enum-whitelisted autocomplete + expandable device grouping + 8 bulk-create integration tests wired end-to-end through Tauri commands, axum routes, and Svelte components.

## What Was Built

### Task 1 — Backend (commit 7608aa3)

**FTS5 search** (`devices_search`): `build_fts_query` splits user input on whitespace, escapes `"` as `""`, drops `\0`, wraps each token as `"token"*` and joins with space — safe for FTS5 MATCH. Query joins `devices_fts ON d.id = devices_fts.rowid WHERE d.deleted_at_utc IS NULL ORDER BY rank`. COUNT(*) runs in a separate query on the same filter for `DeviceListResponse.total`.

**Autocomplete** (`devices_autocomplete`): `AutocompleteField` enum in `trackly-core/domain/devices.rs` maps string field names ("name", "model", "specs", "kit", "state", "location") to static SQL column name strings via `sql_column()`. Column name is never taken from user input. SQL uses `DISTINCT col LIKE ?prefix% [AND name = ?ctx]` with `LIMIT 30 ORDER BY col ASC`.

**Grouping** (`devices_list_grouped`): aggregates non-unique devices (both inventory_number and serial_number null/empty) with `GROUP BY type_id, name, model, notes, complectation, condition, location_id, status_id` + `MIN(id) repr_id, COUNT(*), GROUP_CONCAT(id)`. Group ids are parsed from the concat string; parse failure returns `AppError::Internal`.

**Status counts** (`devices_status_counts`): single `SELECT status_id, COUNT(*) ... GROUP BY status_id` on non-deleted rows.

**List by ids** (`devices_list_by_ids`): dynamic `WHERE id IN (...)` with cap 1000. Used by DeviceGroupRow expand.

**Integration tests:** 9 search tests (devices_search.rs), 7 autocomplete tests (devices_autocomplete.rs), 7 grouping/list-by-ids tests (devices_grouping.rs). All pass.

### Task 2 — Tauri commands + axum routes (commit 74fc73f)

- 6 new Tauri commands registered via `specta::specta` and `collect_commands![]`
- 6 axum handlers at `/api/v1/devices_search`, `/api/v1/devices_autocomplete`, `/api/v1/devices_list_grouped`, `/api/v1/devices_status_counts`, `/api/v1/devices_list_by_ids`, `/api/v1/devices_bulk_create`
- `bindings.ts` regenerated: `DeviceGroup`, `StatusCount`, all 6 new command signatures

### Task 3 — Frontend (commit ae3e384)

**DeviceFilters.svelte:** FTS search input with 250ms debounce (localSearch internal state + $effect sync from prop), status switch-bar with 5 tabs and count badges, grouped checkbox toggle.

**DeviceGroupRow.svelte:** Expandable row renders repr device data; on first expand fetches children via `devices.listByIds(group.ids)`; children cached per component instance, invalidated on edit/delete. Shows `{count} шт.` pill in Серийный № column.

**DeviceAutocompleteField.svelte:** Debounce 200ms, keyboard nav (↑↓ Enter Esc Tab), click-outside close. Contextual header "Ранее использовалось с «{contextName}»:" shown when `contextName` set and `field !== 'name'`. `aria-autocomplete="list"` + `role="listbox"` + `role="option"`. Note: `aria-expanded` removed from `<input>` (not valid on textbox role per ARIA spec).

**DevicesPage.svelte:** Branches on `searchActive` (FTS mode), `grouped` (grouped mode), flat mode. `$effect` on `statusFilter` and `grouped` triggers `refresh()`. `refreshCounts()` called independently.

**DeviceFormModal.svelte:** Name, model, state, location fields replaced with `DeviceAutocompleteField`. `contextName={name.trim() || undefined}` wired to model/state/location fields for DEV-09 contextual autocomplete.

## Scope Extension: Quantity Bulk-Create

**Added by user request (not in original PLAN.md).** Implemented inline without architectural changes.

### Backend

`DeviceService::bulk_create(new: DeviceNew, count: u32) -> Result<Vec<DeviceDto>, AppError>`:
- Validates `count` in `1..=100`; if `count > 1`, both `inventory_number` and `serial_number` must be empty
- Calls `validate_new` for field validation
- Inserts `count` rows + `count` audit_log rows inside a single transaction (snapshot timestamp shared across all rows)
- Returns `list_by_ids(ids)` for full `DeviceDto` array

Tauri command: `devices_bulk_create(device: DeviceNew, count: u32)` — parameter named `device` (not `new` — reserved word in TypeScript).

**Integration tests (devices_bulk_create.rs):** 8 tests covering: inserts N rows + audit rows, rejects inv# set, rejects serial# set, count=0 rejected, count>100 rejected, count=1 equivalent to single create, transactionality (all-or-nothing on unique constraint failure), count=100 allowed.

### Frontend

`DeviceFormModal.svelte`:
- `let quantity = $state<number>(1)` — reset to 1 when modal opens
- `showQuantity = $derived(!isEdit && inventoryNo.trim() === '' && serialNo.trim() === '')`
- `$effect` resets quantity to 1 when inv/serial become non-empty
- Submit calls `devices.bulkCreate(newDevice, quantity)` in create mode (single create is qty=1 bulk)
- Toast: "Устройство создано" (qty=1) or "Создано {qty} устройств" (qty>1)
- `<input type="number" min=1 max=100>` shown only when `showQuantity`

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed FTS5 ё/е cross-normalization test**
- **Found during:** Task 1 test run
- **Issue:** RESEARCH.md stated `unicode61 remove_diacritics 2` normalizes ё→е at index time. Actual SQLite 3.51.0 behaviour: ё is indexed as-is (NOT collapsed to е). Test `search_normalizes_yo_ye` would have passed when it should have failed.
- **Fix:** Rewrote test to reflect actual behaviour — `ёлоч*` finds Ёлочка, `елоч*` finds Елочка (case-insensitive same-variant). Added code comment in test explaining cross-variant limitation.
- **Files modified:** `crates/trackly-app/tests/devices_search.rs`
- **Commit:** 7608aa3

**2. [Rule 2 - Missing] AutocompleteField enum — injection-safe column whitelist**
- **Found during:** Task 1 implementation
- **Issue:** Without an enum whitelist, any string passed as `field` could be used to craft malicious SQL via column name injection.
- **Fix:** Added `AutocompleteField` enum to `trackly-core/domain/devices.rs` with `from_str()` validation and `sql_column()` returning static strings. All autocomplete SQL uses enum-derived column names, never raw user strings.
- **Files modified:** `crates/trackly-core/src/domain/devices.rs`, `crates/trackly-core/src/ports/devices.rs`, `crates/trackly-infra/src/repos/devices_sqlite.rs`
- **Commit:** 7608aa3

**3. [Rule 1 - Bug] Renamed bulk_create Tauri param new→device**
- **Found during:** Task 3 svelte-check
- **Issue:** specta generated `async devicesBulkCreate(new: DeviceNew, ...)` — `new` is a reserved word in TypeScript, causing parse errors even under `// @ts-nocheck`.
- **Fix:** Renamed Rust Tauri command parameter from `new` to `device`; updated HTTP `BulkCreatePayload.new` to `.device`; updated `devices.ts` apiCall accordingly.
- **Files modified:** `crates/trackly-app/src/tauri_cmds/devices.rs`, `crates/trackly-app/src/http/devices.rs`, `ui/src/lib/api/devices.ts`
- **Commit:** ae3e384

**4. [Rule 1 - Bug] Fixed ctxName casing in devices.ts autocomplete call**
- **Found during:** Task 3 implementation review
- **Issue:** `devices.ts` passed `ctx_name` (snake_case) to `apiCall`, but Tauri converts `ctx_name` Rust param to `ctxName` (camelCase) in JS invoke.
- **Fix:** Changed `apiCall(..., { ctx_name: ... })` to `{ ctxName: ... }`.
- **Files modified:** `ui/src/lib/api/devices.ts`
- **Commit:** ae3e384

**5. [Rule 1 - Bug] Removed unused inputEl binding and aria-expanded from DeviceAutocompleteField**
- **Found during:** Task 3 svelte-check
- **Issue:** `inputEl` was declared but never read (svelte-check error). `aria-expanded` on `<input>` (implicit textbox role) is invalid per ARIA spec (svelte-check warning).
- **Fix:** Removed `let inputEl = $state<HTMLInputElement | null>(null)` and `bind:this={inputEl}`; removed `aria-expanded` from the input element.
- **Files modified:** `ui/src/features/devices/DeviceAutocompleteField.svelte`
- **Commit:** ae3e384

**6. [Rule 1 - Bug] Removed unused showItems derived in DeviceList**
- **Found during:** Task 3 svelte-check
- **Issue:** `const showItems = $derived(!showGroups)` declared but never read.
- **Fix:** Removed the declaration.
- **Files modified:** `ui/src/features/devices/DeviceList.svelte`
- **Commit:** ae3e384

## Known Stubs

None. All data sources are wired. The «Импорт CSV» and «Экспорт CSV» buttons in DevicesPage remain `disabled` — these are Plan 05 placeholders, not new stubs introduced in this plan.

## Threat Flags

No new threat surface introduced. All new endpoints follow the same AppCtx injection pattern as existing device commands. The `AutocompleteField` enum prevents column-name injection at the domain layer.

## Self-Check: PASSED

- 7608aa3 exists: `cargo test --workspace` all green (9+7+7 integration tests)
- 74fc73f exists: bindings.ts includes DeviceGroup, StatusCount, 6 new commands
- ae3e384 exists: `pnpm build` clean (0 errors, 1 benign Svelte warning for state_referenced_locally)
- 8 bulk_create tests pass: `cargo test -p trackly-app --test devices_bulk_create` → 8/8 ok
- `pnpm svelte-check` → 0 errors, 1 warning (DeviceFilters localSearch initial-capture — benign)

---

## Post-checkpoint Fixes (2026-05-26)

Five defects found during manual smoke were applied as atomic commits after the initial checkpoint.

### Fix 1 — Autocomplete принимает ctx_status_id + Расположение с context (commit 32dc75c)

**Дефект:** Поле `Расположение` не фильтровало подсказки по выбранному Статусу.

**Изменения backend:**
- `DeviceRepository::autocomplete()` получил новый параметр `ctx_status_id: Option<i64>`
- SQL WHERE динамически добавляет `AND status_id = ?` когда ctx_status_id задан
- `ctx_name` и `ctx_status_id` комбинируются через AND — оба могут быть активны одновременно
- `DeviceService::autocomplete()`, `build_devices_autocomplete()`, HTTP `AutocompletePayload` прокидывают новый параметр
- Tauri command принимает `Option<i32>` (Specta запрещает i64 в публичных командах — `BigIntForbidden`), конвертирует в i64 внутри

**Изменения frontend:**
- `DeviceAutocompleteField.svelte`: добавлен prop `contextStatusId?: number | null`; $effect перезапускается при его изменении
- Заголовок дропдауна: если оба контекста заданы → «Ранее использовалось с «{name}» в статусе #{id}:»; если только status → «Ранее использовалось в статусе #{id}:»
- `ui/src/lib/api/devices.ts`: `autocomplete()` принимает `ctxStatusId?: number | null`
- `DeviceFormModal.svelte`: поле Расположение → `contextStatusId={parseInt(statusId, 10) || null}`

**Новые тесты:**
- `autocomplete_location_filtered_by_status`: status_id=1 → только «Склад A», status_id=2 → только «Офис 305», без фильтра → оба
- `autocomplete_location_combines_with_name`: ctx_name AND ctx_status_id → AND-семантика

**Все 9 тестов autocomplete зелёные.**

### Fix 2 — Порядок полей формы (коммит включён в 32dc75c)

**Дефект:** Статус шёл после Расположения — contextStatusId не был активен когда пользователь добирался до Расположения.

**Новый порядок:** Наименование → Статус → Расположение → Модель → Инв.№ → Серийный № → Состояние → Комплектация → Технические характеристики → Количество

**Файл:** `ui/src/features/devices/DeviceFormModal.svelte`

### Fix 3 — Группировка по Наименованию (включает устройства с inv/serial) (commit d88e8d8)

**Дефект:** `list_grouped` фильтровал устройства с `inventory_number` или `serial_number` — они исчезали из grouped view.

**Изменение:** Удалён WHERE-фильтр `AND (inventory_number IS NULL OR inventory_number = '') AND (serial_number IS NULL OR serial_number = '')` из SQL запроса `list_grouped`. Группировочный ключ не изменился.

**Тесты:**
- `grouping_keeps_unique_separate` заменён на `grouping_groups_devices_with_same_name_even_if_inventory_set` — 3 устройства «Ноутбук Lenovo X1» с разными inventory_no → 1 группа count=3
- Добавлен `grouping_singleton_included` — одиночное устройство с inventory_no → группа count=1

**Все 8 тестов groups зелёные.**

### Fix 4 — Grouped view показывает синглтоны (commit 9c5a86f)

**Дефект:** В grouped режиме `DeviceList.svelte` рендерил только `DeviceGroupRow` — устройства с count=1 пропадали из вида.

**Изменение:** Логика рендеринга:
- `group.count > 1` → `DeviceGroupRow` (chevron + count badge)
- `group.count == 1` → `DeviceListRow` (обычная строка, без chevron, без «1 шт.»)

**Файл:** `ui/src/features/devices/DeviceList.svelte`

### Fix 5 — Визуальные фиксы DeviceGroupRow (commit a5f2e9e)

**Дефекты:** top-aligned ячейки, truncation имени, пустые колонки Инв.№ / Серийный / Модель.

**Изменения:**
- Базовый `.cell` сохранил `vertical-align: middle` (уже был в исходном коде), убраны конкурирующие стили `.cell-expand` с `display: flex` (который нарушал вертикальное выравнивание как `<td>`)
- `<td colspan="4">` объединяет Наименование + Инв.№ + Серийный + Модель; chevron и текст имени внутри этой ячейки
- Убраны `text-overflow: ellipsis`, `max-width: 0`, `overflow: hidden` — truncation устранён
- Count badge перенесён в колонку Действий (ячейки: colspan=4 + Расположение + Статус + Действия = 7, сходится с thead)

**Файл:** `ui/src/features/devices/DeviceGroupRow.svelte`

### Итоговая верификация post-checkpoint (round 1)

- `cargo build --workspace` — зелёный
- `cargo test --workspace --no-fail-fast` — все тесты прошли (9 autocomplete + 8 grouping + все остальные)
- `pnpm svelte-check --threshold error` — 0 ошибок
- `pnpm build` — зелёный (0 ошибок, 1 существующее предупреждение state_referenced_locally в DeviceFilters)

---

## Post-checkpoint Fixes — Round 2 (2026-05-27)

Четыре дефекта найдены при ручном смоке после round 1. Применены атомарными коммитами.

### Fix 1 — Autocomplete: suppression после выбора + no auto-open on focus (commit e767cf9)

**Дефекты:**
- (a) После выбора подсказки дропдаун немедленно переоткрывался с той же строкой
- (b) Повторный фокус на заполненном поле переоткрывал дропдаун

**Реализация:**
- Добавлены флаги `lastSelected: string | null` и `suppressDropdown = $state(false)`
- При выборе: `suppressDropdown = true`, дропдаун закрыт
- `onfocus` удалён — дропдаун открывается только через `oninput` или `ArrowDown`
- `handleInput`: снимает suppression когда новое значение отличается от `lastSelected`
- ArrowDown на непустом поле явно переоткрывает дропдаун (escape hatch пользователя)
- mousedown на click-outside уже использовался (Дефект 2 уже был исправлен в round 1)

**Файл:** `ui/src/features/devices/DeviceAutocompleteField.svelte`

### Fix 2 — Порядок полей формы + горизонтальный ряд Инв/Серийный/Количество (commit 14d465b)

**Дефект:** Поля были в неправильном порядке; Инв.№ / Серийный № / Количество отображались вертикально отдельными строками.

**Правильный порядок:** Наименование → Инв.№/Серийный/Количество (flex row) → Модель → Тех.хар-ки → Комплектация → Состояние → Статус → Расположение

**Изменения:**
- Три поля (Инв.№, Серийный №, Количество) помещены в `.field-row` (flex, gap, `flex: 1 1 0` для равномерного распределения)
- При скрытии Количество остальные два растягиваются на всю ширину
- Технические характеристики и Комплектация переключены с Textarea на DeviceAutocompleteField (контекстные подсказки), импорт Textarea удалён

**Файл:** `ui/src/features/devices/DeviceFormModal.svelte`

### Fix 3 — Редактирование показывает корректное Расположение из DTO (часть commit fe899e0)

**Дефект:** `location = target.specs ?? ''` — заглушка из Plan 03, показывала технические характеристики в поле Расположение.

**Исправление:** `location = target.location ?? ''` — берётся из нового поля `DeviceDto.location` (строка из таблицы `locations`).

### Fix 4 — Round-trip location через таблицу locations (commit fe899e0 + e84bf2b)

**Дефект:** `devices.location_id` был всегда NULL — расположение нигде не сохранялось. Автодополнение поля Location возвращало числовые ID.

**Backend изменения:**
- `DeviceRow` / `DeviceDto`: добавлено поле `location: Option<String>`
- `DeviceNew` / `DevicePatch`: добавлено поле `location: Option<String>` (строковый ввод)
- `SqliteDeviceRepository::resolve_location_id_in_tx`: `INSERT OR IGNORE INTO locations` + `SELECT id` в одной транзакции
- `create` / `bulk_create`: разрешают строку расположения в `location_id` через хелпер
- `update`: разрешает `patch.location = Some(Some(name))` → `location_id`
- `SELECT_DEVICES`: LEFT JOIN locations l ON d.location_id = l.id (колонка 15 = location_name)
- `list_grouped`: scalar subquery для location_name у representative row
- `search_fts`: добавлен LEFT JOIN locations для location_name
- `AutocompleteField::Location`: отдельная SQL-ветка — `JOIN devices d ON d.location_id = l.id WHERE l.name LIKE ?` с фильтрами по ctx_status_id и ctx_name

**Frontend изменения:**
- `DeviceFormModal.svelte`: create и edit отправляют `location` строкой (не location_id)
- `DeviceListRow.svelte`: `device.location ?? '—'`
- `DeviceGroupRow.svelte`: `group.repr.location ?? '—'`

**Тесты (6 новых в devices_location_roundtrip.rs):**
- `create_with_location_persists_round_trip`
- `create_with_same_location_reuses_id`
- `update_changes_location_creates_new_locations_row`
- `create_with_empty_location_keeps_null`
- `autocomplete_location_returns_from_locations_table`
- `autocomplete_location_filtered_by_ctx_status_id_via_locations_table`

### Итоговая верификация post-checkpoint (round 2)

- `cargo build --workspace` — зелёный
- `cargo test --workspace --no-fail-fast` — все тесты зелёные (6 location roundtrip + все предыдущие)
- `pnpm svelte-check --threshold error` — 0 ошибок (1 существующее предупреждение DeviceFilters)
- `pnpm build` — зелёный (0 ошибок)

---

## Post-checkpoint Fixes — Round 3 (2026-05-27)

Четыре дефекта найдены при ручном смоке после round 2. Применены атомарными коммитами.

### Fix 1 — Тест serial_no round-trip (commit a3cbc9d)

**Дефект:** Пользователь сообщил, что серийный номер не сохраняется при создании устройства.

**Диагностика:**
- Backend (`create_in_tx`) корректно принимает `serial_no` и биндит на колонку `serial_number`
- Frontend payload: `serial_no: serialNo.trim() || null` — корректно
- `Input` компонент с `oninput={(v) => (serialNo = v)}` — корректно

**Результат:** Backend и frontend работают корректно. Тест отсутствовал — это и создавало неопределённость. Добавлены 2 теста для документирования и регрессии.

**Новые тесты:**
- `create_persists_serial_number`: round-trip `serial_no="SN-XYZ-001"` через create → get
- `create_persists_inventory_and_serial_together`: оба номера одновременно

**Файл:** `crates/trackly-app/tests/devices_crud.rs`

### Fix 2 — Autocomplete: начальное значение = уже-выбранное (commit 0bb9f71)

**Дефект:** При открытии устройства в режиме редактирования каждый автокомплит-дропдаун мгновенно открывался, показывая текущее значение как единственную подсказку.

**Корневая причина:** `lastSelected` и `suppressDropdown` инициализировались как пустые независимо от пропа `value`. При монтировании в edit-режиме `$effect` запускал debounced fetch, подгружал подсказки и открывал дропдаун, так как suppression не была активна.

**Исправление:**
- Добавлен `$effect.pre`: при первом рендере с `value.length > 0` → `lastSelected = value`, `suppressDropdown = true`
- Добавлен watcher `_prevValue`: при изменении `value` извне (смена target) re-arm suppression
- Документирован инвариант «initial value = already-selected» в комментарии
- Исправлено 4 предупреждения `state_referenced_locally` (убраны `$state(value.x)`)

**Файл:** `ui/src/features/devices/DeviceAutocompleteField.svelte`

### Fix 3 — DeviceContextMenu портализован в `<body>` (commit 870d77d)

**Дефект:** Меню ⋮ обрезалось контейнером списка с `overflow: auto`, добавляло скролл-бар.

**Исправление:**
- Создан `ui/src/lib/utils/portal.ts` — Svelte use-action, перемещает DOM-узел в `<body>`
- `DeviceContextMenu.svelte` переработан: dropdown рендерится через `use:portal`
- Координаты вычисляются из `triggerEl.getBoundingClientRect()` при каждом открытии
- `position: fixed; z-index: 2000` — видим поверх всех контейнеров
- Закрывается: mousedown вне меню, Esc, выбор пункта, scroll/resize
- `:global()` стили для портализованного div
- Добавлен `tabindex="-1"` на `role="menu"` (a11y)

**Файлы:**
- `ui/src/lib/utils/portal.ts` (создан)
- `ui/src/features/devices/DeviceContextMenu.svelte`

### Fix 4 — Финальный порядок полей + Количество всегда visible disabled (commit 0582122)

**Дефект:** Поля расположены не в том порядке; Количество исчезало и появлялось при вводе номеров («прыжки» макета).

**Финальный порядок полей:**
1. Наименование (required, autocomplete)
2. Инв.№ / Серийный № / Количество — один flex ряд (всегда 3 поля, нет добавления/удаления)
3. Модель (autocomplete, ctx_name)
4. Технические характеристики (autocomplete, ctx_name)
5. Комплектация (autocomplete, ctx_name)
6. Статус (required, Select)
7. Расположение (required, autocomplete, ctx_name + ctx_status_id)
8. Состояние (autocomplete, ctx_name) — ПОСЛЕДНЕЕ

**Поведение Количества:**
- Всегда отображается — нет `{#if}`, макет стабилен
- `disabled` + значение `1` когда: edit-режим ИЛИ инв./серийный заполнен
- Подсказка «Доступно только для устройств без инвентарного и серийного номера.»
- Submit: `quantityDisabled ? 1 : quantity`
- Backend-валидация `count>1 с inv/serial` сохранена как defence-in-depth

**Файл:** `ui/src/features/devices/DeviceFormModal.svelte`

### Итоговая верификация post-checkpoint (round 3)

- `cargo build --workspace` — зелёный
- `cargo test --workspace --no-fail-fast` — все тесты зелёные (+2 новых devices_crud)
- `pnpm svelte-check --threshold error` — 0 ошибок (1 старое предупреждение DeviceFilters)
- `pnpm build` — зелёный (0 ошибок)
