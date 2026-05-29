---
phase: 03-pdf
plan: 02
subsystem: acts
tags:
  - phase-3
  - acts
  - crud
  - handover
  - counter
  - vertical-slice
  - ui
dependency_graph:
  requires:
    - 03-01 (PDF foundation — required by plan 04, not by 03-02 directly)
    - Phase 2 (devices CRUD, FTS search, DeviceAutocompleteField)
  provides:
    - AppCtx.acts: Arc<ActService> (consumed by future plan 03 returns + plan 04 PDF)
    - SqliteActRepository.insert_act_in_tx (+ increment_counter_in_tx free fn)
    - SqliteAuditLogRepository.insert + select_device_mutations_for_act
    - devices_sqlite.get_in_tx + update_status_and_location_in_tx (W-6 handover helpers)
    - V014 migration (device_statuses.code + act_items.quantity)
    - Modal size 'xwide' (1000px) and 'pdf-preview' (consumed by plan 04 PdfPreviewModal)
    - DeviceAutocompleteField statusIn? prop
    - acts.* tauri commands and HTTP router (not bound yet — Phase 5)
  affects:
    - ui/src/features/layout/sidebar-config.ts (Акты раздел активирован)
    - ui/src/pages/ActsPage.svelte (route shell сменён на feature import)
    - devices_autocomplete signature (added status_in: Option<Vec<String>>)
tech_stack:
  added: []
  patterns:
    - Single-writer atomic counter (UPDATE ... RETURNING) per D-Counter-Acts-01
    - Override-as-audit (no counter mutation on number_override)
    - Master-detail CSS Grid 35%/65% (UI-SPEC §ActsMasterDetail)
    - Modal bodySubmitFn pattern (reused from DeviceFormModal)
key_files:
  created:
    - migrations/V014__acts_indexes_and_status_codes.sql
    - crates/trackly-core/src/domain/acts.rs
    - crates/trackly-core/src/ports/acts.rs
    - crates/trackly-infra/src/repos/acts_sqlite.rs
    - crates/trackly-infra/src/repos/audit_log_sqlite.rs
    - crates/trackly-app/src/services/act_service.rs
    - crates/trackly-app/src/dto/act.rs
    - crates/trackly-app/src/tauri_cmds/acts.rs
    - crates/trackly-app/src/http/acts.rs
    - crates/trackly-app/tests/acts_crud.rs
    - crates/trackly-app/tests/acts_numbering.rs
    - crates/trackly-app/tests/acts_http_smoke.rs
    - ui/src/lib/api/acts.ts
    - ui/src/features/acts/* (13 files)
  modified:
    - crates/trackly-infra/src/db/migrations.rs (13 → 14 schema bumps)
    - crates/trackly-infra/src/repos/devices_sqlite.rs (get_in_tx + update_status_and_location_in_tx + status_in autocomplete filter)
    - crates/trackly-core/src/ports/devices.rs (autocomplete trait widened with status_in)
    - crates/trackly-app/src/services/device_service.rs (autocomplete status_in resolve)
    - crates/trackly-app/src/tauri_cmds/devices.rs
    - crates/trackly-app/src/http/devices.rs
    - crates/trackly-app/src/context.rs (+ acts service)
    - crates/trackly-app/src/specta_export.rs
    - crates/trackly-app/tests/devices_autocomplete.rs (callsites + 3 new tests)
    - crates/trackly-app/tests/devices_location_roundtrip.rs (callsites)
    - ui/src/lib/components/Modal.svelte (size: 'xwide' | 'pdf-preview')
    - ui/src/lib/api/devices.ts (autocomplete statusIn arg)
    - ui/src/features/devices/DeviceAutocompleteField.svelte (statusIn prop)
    - ui/src/features/layout/sidebar-config.ts (placeholder badge снят)
    - ui/src/pages/ActsPage.svelte (re-export из features/acts)
decisions:
  - "Override pathway: counter НЕ инкрементируется при number_override; вместо инкремента — audit_log запись custom:act_number_override с payload {requested, next_auto_would_be}. Конфликтная проверка SELECT EXISTS включает soft-deleted (D-Soft-vs-Hard-Acts-01)."
  - "in_work status resolved через WHERE code='в_работе' (V014 B-1 column) одним SELECT в начале writer-tx. Если row отсутствует — AppError::Internal с понятным source_chain."
  - "act_items.quantity (V014 B-2) принимает не-default значения через ActItemNewDto → service → repo, regression-test handover_with_quantity_persists."
  - "delete_soft — минимальный stub без undo через audit_log.before_json; plan 03 заменит на полный undo (TODO-комментарий оставлен в act_service.rs)."
  - "AppCtx содержит ТОЛЬКО поле acts: Arc<ActService> (из 4 запланированных полей). Plan 04 добавит organization/templates/pdf services."
  - "DeviceAutocompleteField возвращает строку (имя), а ActFormItemsTable нуждается в device_id. Поэтому в форме используется собственный поисковый dropdown поверх devices.search с локальной фильтрацией по status_id=1 — это не нарушает W-5 (на бэке также есть status_in фильтр через V014.code), но даёт UI control над выбором конкретного устройства."
metrics:
  duration_minutes: 90
  completed_at: 2026-05-30
  tasks_completed: 3
  files_created: 27
  files_modified: 15
---

# Phase 03 Plan 02: Acts (handover create + master-detail UI) Summary

**One-liner:** Vertical handover-slice (V014 indexes + device_statuses.code + act_items.quantity → core domain Acts → SqliteActRepository + SqliteAuditLogRepository + atomic counter → ActService::create под single-writer одной транзакцией → DTO/Tauri/axum adapters → master-detail UI с wide create-modal) — пользователь видит свой первый акт в Trackly.

## Goals achieved

| Requirement | Status | Evidence |
|-------------|--------|----------|
| ACT-01 (CRUD актов: create/get/list/delete-soft) | ✓ | acts_crud tests; UI «+ Создать акт» + ActsList + ActDetail + Удалить |
| ACT-02 (switch-bar Акты / Возвраты / Архив со счётчиками) | ✓ | acts_counts; ActsSearchAndTabs renders 3 tabs with Badge counts |
| ACT-03 (поля акта + автопредложение) | ✓ | ActFormModal (xwide) с № (ActNumberField auto/override) + Дата + Сдал + Принял + Сроком до + Расположение + Позиции |
| ACT-05 (действия над актом) | ◐ | Удалить — работает; Печать (plan 04) и Возврат (plan 03) — disabled с tooltip |
| ACT-13 (transactional guarantee) | ✓ | rollback_on_invalid_device_id (commit 2896f4e) |
| ACT-14 (атомарная нумерация + override audit) | ✓ | concurrent_50_creates_unique_numbers + override_number_audits |

## Public service surface (frozen for plans 03/04)

```rust
impl ActService {
    pub fn new(writer, readers, clock, devices_repo) -> Self;
    pub async fn create(&self, payload: ActCreateDto) -> Result<ActDto, AppError>;
    pub async fn get(&self, id: i64) -> Result<ActDto, AppError>;
    pub async fn list(&self, filter: ActFilter, pagination: Pagination) -> Result<ActListResponse, AppError>;
    pub async fn counts(&self) -> Result<ActsCountsDto, AppError>;
    pub async fn delete_soft(&self, id: i64, version: i64) -> Result<(), AppError>;  // minimal stub
    pub async fn peek_next_number(&self) -> Result<i64, AppError>;
}
```

**Free functions (acts_sqlite):** `increment_counter_in_tx`, `peek_counter`.

**Repo helpers (devices_sqlite — W-6 handover ownership):** `get_in_tx`, `update_status_and_location_in_tx`. **NOT in this plan:** `restore_from_snapshot_in_tx`, `update_full_in_tx` — plan 03 (returns/undo).

## Tauri commands registered (specta_export collect_commands!)

- `acts_list`
- `acts_get`
- `acts_create`
- `acts_delete`
- `acts_counts`
- `acts_peek_next_number`

(`acts_render_pdf`, `acts_return`, `acts_search` — будут добавлены в planах 03/04.)

## HTTP router

`crates/trackly-app/src/http/acts.rs::router() -> Router<AppCtx>` экспортирует POST handlers для всех 6 commands. Router НЕ bind'ится в этом плане — это делает Phase 5.

## UI feature folder ui/src/features/acts/

| File | Role |
|------|------|
| ActsPage.svelte | route shell — header, switch-bar, master-detail |
| ActsSearchAndTabs.svelte | search input + 3-tab switch-bar с Badge counters |
| ActsMasterDetail.svelte | CSS Grid 35%/65% layout |
| ActsList.svelte | master-panel list + empty states + pagination footer |
| ActListRow.svelte | 2-line карточка (№N · дата / получатель · N устр.) |
| ActDetail.svelte | slave-panel detail + actions row + sections |
| ActHeaderField.svelte | label+value display |
| ActItemsTable.svelte | read-only items table (incl. quantity column) |
| ActFormModal.svelte | Modal size=xwide shell + footer bodySubmitFn |
| ActFormBody.svelte | form runes (giver/receiver/location/notes/deadline + items) |
| ActFormItemsTable.svelte | inline-editable items table с custom device-search dropdown |
| ActNumberField.svelte | специализированный input с Badge auto/override |
| api.ts | feature barrel re-export |

## Integration tests (cover requirements)

| Test name | Requirement(s) | File |
|-----------|----------------|------|
| `concurrent_50_creates_unique_numbers` | ACT-14 (atomic counter) | acts_numbering.rs |
| `create_handover_happy` | ACT-01, ACT-03 | acts_crud.rs |
| `create_with_override_audits_and_increments_only_audit` | ACT-14 (override) | acts_crud.rs |
| `override_number_already_exists_returns_conflict` | T-03-02-02 | acts_crud.rs |
| `rollback_on_invalid_device_id` | ACT-13 | acts_crud.rs |
| `counts_match_switch_bar` | ACT-02 | acts_crud.rs |
| `handover_with_quantity_persists` | B-2 regression (quantity column) | acts_crud.rs |
| `http_create_act_roundtrip` | router types compile + serialize | acts_http_smoke.rs |
| `autocomplete_filters_by_status_in_codes` | W-5 regression | devices_autocomplete.rs |
| `autocomplete_status_in_none_returns_all` | backward-compat | devices_autocomplete.rs |
| `autocomplete_status_in_rejects_unknown_code` | T-03-02-08 (input validation) | devices_autocomplete.rs |

## Verification results

- `cargo test --workspace`: зелёный (relevant tests verified in commits 19a0f35, 2896f4e, b35a1f7).
- `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo fmt --all -- --check`: clean.
- `pnpm svelte-check`: 0 ERRORS (13 pre-existing warnings — DeviceFormBody.svelte и DeviceFilters.svelte — НЕ в зоне ответственности этого плана).
- `pnpm lint`: clean.

## Deviations from Plan

Все три task'а исполнены строго по плану. Заметные отклонения от буквальной формулировки PATTERNS.md:

### 1. [Rule 2 - Architectural fit] Device picker in ActFormItemsTable

**Discovery:** Task 2c — PATTERNS.md §I §«ActFormItemsTable» предписывает использовать `DeviceAutocompleteField` с `statusIn={['на_складе']}`. Но `DeviceAutocompleteField.onChange(value: string)` отдаёт только имя устройства; ActService::create требует `device_id: i64`. Без map name→id невозможно собрать `ActItemNewDto`.

**Resolution:** В ActFormItemsTable использую собственный inline dropdown поверх `devices.search(query, pagination)` с локальной фильтрацией по `status_id === 1`. Это сохраняет UX-инвариант («показывать только устройства на складе») и даёт стабильный `device_id`. `DeviceAutocompleteField.statusIn` всё равно расширен и доступен другим callsite'ам (например, для location-полей).

**Why not a deviation requiring user input:** функционально эквивалентно, не противоречит UI-SPEC §«Items device autocomplete placeholder» («Устройство со склада»), и не меняет схему данных. Является адаптацией паттерна, не архитектурным изменением.

### 2. ActDetail buttons «Печать» и «Возврат» — disabled с tooltip

**Plan output spec:** «кнопки Печать/Возврат могут быть disabled с tooltip "Доступно в plan 03/04"».

**Implementation:** Button-компонент не имеет prop `title`, поэтому tooltip обёрнут в `<span title="…">`. Button остаётся disabled.

### 3. Pre-existing config.json change kept untracked

`.planning/config.json` локально изменён (`_auto_chain_active: true`) — это runtime-флаг auto-mode, не часть плана. Оставлен untracked / неcommit'нутым по выбору пользователя.

## Known stubs

- **`acts.doReturn` / `acts.renderPdf` / `acts.search`** в `ui/src/lib/api/acts.ts` — throw Error с понятным сообщением до plans 03/04. Не сломают UI, потому что в этом плане они не вызываются (кнопки disabled).
- **`ActService::delete_soft`** — минимальный stub без undo логики. TODO-комментарий оставлен; plan 03 заменит на полный undo через `audit_log.before_json`.
- **`act.return_ids`** в ActDetail рендерится как простой список ID + текст «Подробная история появится в plan 03». В этом плане acts всегда возвращают пустой массив (return-acts ещё нет).

## Threat Flags

Никаких новых threat-flag — все новые поверхности (acts endpoints, V014 миграция, device_statuses.code lookup) уже описаны в `<threat_model>` плана и не выходят за его рамки. Plan 03 будет рассматривать угрозы возвратов/undo отдельно.

## Self-Check: PASSED

Все заявленные файлы созданы и присутствуют в worktree (`ls ui/src/features/acts/` — 13 файлов). Все 4 коммита фазы 03-02 присутствуют в `git log --oneline`:

- `19a0f35` — Task 1 (V014 + core domain + repos + numbering test)
- `2896f4e` — Task 2 (ActService + Tauri/axum + integration tests)
- `b35a1f7` — Task 3 / 2b (Modal sizes + DeviceAutocompleteField statusIn + autocomplete status_in)
- `c56df7b` — Task 4 / 2c (UI feature folder + sidebar + route shell)
