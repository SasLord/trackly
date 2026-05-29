---
phase: 03-pdf
plan: 03
subsystem: acts
tags:
  - phase-3
  - acts
  - returns
  - undo
  - archive
  - audit-replay
  - vertical-slice
  - ui
dependency_graph:
  requires:
    - 03-02 (ActService::create + V014 schema + master-detail UI)
  provides:
    - ActService::do_return (sub_number sequencing + bulk/per-row override + auto-archive)
    - ActService::delete_soft (full undo via audit_log.before_json — replaces plan-02 stub)
    - acts_sqlite::next_sub_number_for_parent (atomic MAX+1 helper)
    - acts_sqlite::recompute_parent_archived (B-2 SUM(quantity) semantics)
    - acts_sqlite::list_returns_for_parent / list_returns_for_parent_in_tx
    - devices_sqlite::update_full_in_tx (W-6 return-path ownership)
    - devices_sqlite::restore_from_snapshot_in_tx (W-6 undo-path ownership)
    - ActReturnDto + ActReturnItemDto (camelCase args; snake_case JSON; specta i32)
    - acts_return Tauri command + POST /api/v1/acts_return
    - ReturnModal + ReturnItemsTable UI (bulk + per-row override; default applyToAll ON)
    - ActDetail кнопка «Возврат» активна для handover/!archived
  affects:
    - ActService::create (расширен device_snapshot_json — теперь пишет ПОЛНЫЙ
      snapshot всех 13 полей DeviceRow в audit_log.before_json для undo path).
      Регрессия проверена acts_crud (8 тестов остаются зелёными).
    - ActsPage.svelte (добавлен ReturnModal lifecycle + destructive confirm copy).
    - acts.delete copy: handover vs return variants.
tech_stack:
  added: []
  patterns:
    - Atomic sub_number sequencing (MAX+1 в той же writer-tx; single-writer guarantee)
    - Recompute derived archived flag (D-Archive-01) — нет ручного флага
    - audit_log.before_json replay для undo (D-Undo-01) — JSON Value snapshot вместо serde DeviceRow
    - Snapshot-семантика bulk + per-row override (per-row побеждает; bulk fallback только при apply_to_all)
    - UX-friendly name → id resolve в сервисе (через resolve_location_id_in_tx)
key_files:
  created:
    - crates/trackly-app/tests/acts_returns.rs (8 tests)
    - crates/trackly-app/tests/acts_display_rule.rs (4 tests)
    - crates/trackly-app/tests/acts_undo.rs (5 tests)
    - ui/src/features/acts/ReturnModal.svelte
    - ui/src/features/acts/ReturnItemsTable.svelte
  modified:
    - crates/trackly-core/src/domain/acts.rs (+ActReturnNew, ActReturnItem)
    - crates/trackly-app/src/dto/act.rs (+ActReturnDto, +ActReturnItemDto, +format_act_number retroactive test, +Default derives)
    - crates/trackly-app/src/services/act_service.rs (+do_return, +validate_return, +полная замена delete_soft; helpers undo_device_mutations_for_act + device_snapshot_json; расширен create — пишет полный snapshot)
    - crates/trackly-app/src/tauri_cmds/acts.rs (+acts_return команда)
    - crates/trackly-app/src/http/acts.rs (+POST /api/v1/acts_return)
    - crates/trackly-app/src/specta_export.rs (+acts_return регистрация)
    - crates/trackly-infra/src/repos/acts_sqlite.rs (+next_sub_number_for_parent, +recompute_parent_archived free fns; +list_returns_for_parent + tx-variant)
    - crates/trackly-infra/src/repos/devices_sqlite.rs (+update_full_in_tx, +restore_from_snapshot_in_tx)
    - crates/trackly-app/tests/acts_http_smoke.rs (+http_acts_return_smoke)
    - ui/src/lib/api/acts.ts (doReturn активирован; renderPdf/search остаются stubs)
    - ui/src/features/acts/ActDetail.svelte (onReturn callback; кнопка «Возврат» активна)
    - ui/src/features/acts/ActsPage.svelte (ReturnModal lifecycle + destructive copy)
decisions:
  - "Return-act giver/receiver: наследуются от parent handover-акта (discretion-зона
    Interfaces). Plan 04+ может расширить, если UI попросит per-return giver/receiver
    — это потребует расширения ActReturnDto. В Phase 3 нет UX-сигнала о необходимости."
  - "Cascade-delete handover: полный LIFO undo (для returns) + handover undo в одной
    writer-tx. Без отдельного запроса разрешения у пользователя — D-Undo-01 диктует
    что delete handover откатывает всё, включая returns."
  - "DeviceRow Serialize/Deserialize НЕ добавляется в core (trackly-core остаётся
    serde-free). Вместо этого: device_snapshot_json helper пишет canonical 13-field
    JSON Value object; restore_from_snapshot_in_tx парсит JSON Value напрямую и
    делает UPDATE с COALESCE(snapshot field, db field) для optional полей."
  - "ActReturnDto принимает БОТЬ bulk_location_id и bulk_location_name (и аналогично
    per-row). Имя имеет приоритет (UX-friendly: UI передаёт name из autocomplete,
    backend резолвит через resolve_location_id_in_tx). Backward compat с тестами,
    использующими id напрямую."
  - "Validation apply_to_all=false: принимает либо location_id_override, либо
    location_name_override (но не оба None). Это симметрично с condition_override."
  - "version+1 в restore_from_snapshot_in_tx: после undo версия растёт (а не
    откатывается к snapshot.version). Это сохраняет optimistic-lock семантику для
    будущих мутаций — undo трактуется как новая ревизия с восстановленным contentом."
metrics:
  duration_minutes: 60
  completed_at: 2026-05-30
  tasks_completed: 2
  files_created: 5
  files_modified: 13
---

# Phase 03 Plan 03: Returns + Auto-archive + Undo Summary

**One-liner:** Vertical returns-slice — `ActService::do_return` (atomic sub_number + bulk/per-row override + auto-archive в одной writer-tx) + полный `delete_soft` через `audit_log.before_json` replay (handover cascade + return un-archive) + ReturnModal с галочкой «Применить ко всем» (default ON) и per-row override + ActDetail активирует кнопку «Возврат». Lifecycle round-trip green: create → partial return «42в» → second return «42в1»/«42в2» → full return (archived) → delete return (unarchive) → delete handover (cascaded undo).

## Goals achieved

| Requirement | Status | Evidence |
|-------------|--------|----------|
| ACT-06 (delete handover restores) | ✓ | `acts_undo::delete_handover_restores_devices_to_pre_handover` + `delete_handover_with_partial_return_cascades_undo` |
| ACT-07 (suffix «в»/«в1/2» + retroactive) | ✓ | `acts_display_rule::format_retroactive_promotion` + `acts_returns::second_partial_return_assigns_sub_number_2_and_promotes_suffix` |
| ACT-08 (bulk + per-row override) | ✓ | `acts_returns::bulk_apply_with_per_row_override` + `return_with_apply_to_all_false_and_full_per_row_succeeds` + UI ReturnModal/ReturnItemsTable |
| ACT-09 (auto-archive 100%) | ✓ | `acts_returns::full_return_archives_handover` |
| ACT-10 (delete return restores + unarchive) | ✓ | `acts_undo::delete_return_restores_to_handover_state_unarchives_parent` |

## Public service surface (frozen for plan 04)

```rust
impl ActService {
    pub fn new(writer, readers, clock) -> Self;
    pub async fn create(&self, payload: ActCreateDto) -> Result<ActDto, AppError>;
    pub async fn do_return(&self, act_id: i64, payload: ActReturnDto) -> Result<ActDto, AppError>;
    pub async fn get(&self, id: i64) -> Result<ActDto, AppError>;
    pub async fn list(&self, filter: ActFilter, pagination: Pagination) -> Result<ActListResponse, AppError>;
    pub async fn counts(&self) -> Result<ActsCountsDto, AppError>;
    pub async fn delete_soft(&self, id: i64, version: i64) -> Result<(), AppError>;  // FULL undo path
    pub async fn peek_next_number(&self) -> Result<i64, AppError>;
}
```

**Free functions (acts_sqlite):** `increment_counter_in_tx`, `peek_counter`, `peek_counter_in_tx`, `next_sub_number_for_parent`, `recompute_parent_archived`.

**Repo helpers (devices_sqlite) — W-6 ownership:**
- Plan 02 owns: `get_in_tx`, `update_status_and_location_in_tx` (handover write-path).
- Plan 03 owns: `update_full_in_tx` (return write-path), `restore_from_snapshot_in_tx` (undo path).

## Tauri commands registered

- `acts_list`, `acts_get`, `acts_create`, `acts_delete`, `acts_counts`, `acts_peek_next_number` — plan 02.
- **`acts_return` — plan 03 (this one).**
- `acts_render_pdf`, `acts_search` — plan 04+.

## HTTP router

`crates/trackly-app/src/http/acts.rs::router()` теперь содержит:
- `POST /api/v1/acts_list`, `acts_get`, `acts_create`, `acts_delete`, `acts_counts`, `acts_peek_next_number` (plan 02)
- **`POST /api/v1/acts_return`** (plan 03)

Router всё ещё НЕ bind'ится — Phase 5.

## DTOs (frontend bindings)

```typescript
// Новые типы в ui/src/bindings.ts:
export type ActReturnDto = {
  bulk_condition: string | null;
  bulk_location_id: number | null;
  bulk_location_name: string | null;
  apply_to_all: boolean;
  items: ActReturnItemDto[];
};
export type ActReturnItemDto = {
  act_item_id: number;
  device_id: number;
  quantity: number;
  condition_override: string | null;
  location_id_override: number | null;
  location_name_override: string | null;
};
```

## Integration tests (cover requirements)

| Test name | Requirement(s) | File |
|-----------|----------------|------|
| `format_handover` / `format_single_return` / `format_multi_returns` / `format_retroactive_promotion` | ACT-07 | acts_display_rule.rs |
| `partial_return_keeps_handover_active` | ACT-07 | acts_returns.rs |
| `full_return_archives_handover` | ACT-09 | acts_returns.rs |
| `second_partial_return_assigns_sub_number_2_and_promotes_suffix` | ACT-07 retroactive | acts_returns.rs |
| `bulk_apply_with_per_row_override` | ACT-08 | acts_returns.rs |
| `return_when_apply_to_all_false_requires_per_row_values` | ACT-08 validation | acts_returns.rs |
| `return_concurrent_two_returns_correct_sub_numbers` | T-03-03-01 (atomic sub_number) | acts_returns.rs |
| `return_does_not_increment_act_counter` | W-7 (counter discipline) | acts_returns.rs |
| `return_with_apply_to_all_false_and_full_per_row_succeeds` | W-8 (positive per-row path) | acts_returns.rs |
| `delete_handover_restores_devices_to_pre_handover` | ACT-06, T-03-03-02 | acts_undo.rs |
| `delete_handover_with_partial_return_cascades_undo` | ACT-06 cascade, T-03-03-04 | acts_undo.rs |
| `delete_return_restores_to_handover_state_unarchives_parent` | ACT-10 | acts_undo.rs |
| `delete_act_optimistic_lock_mismatch` | optimistic-lock | acts_undo.rs |
| `delete_act_audits_undo_entries` | T-03-03-05 (audit trail) | acts_undo.rs |
| `http_acts_return_smoke` | HTTP-transport roundtrip | acts_http_smoke.rs |

## Verification results

- `cargo test --workspace`: зелёный (full sweep пройден).
- `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo fmt --all -- --check`: clean.
- `pnpm svelte-check`: 0 ERRORS (13 pre-existing warnings из plan 02 — DeviceFormBody/DeviceFilters/ActsSearchAndTabs).
- `pnpm lint`: clean.
- `tauri dev`: не запускался автоматически (требует GUI); UI-flow покрыт в VALIDATION Manual-Only.

## Deviations from Plan

### 1. [Rule 3 - Architectural fit] DeviceRow serde — НЕ добавляется в core

**Plan §«Implementation»:** «DeviceRow обязан иметь Serialize+Deserialize — проверить и при необходимости добавить derive».

**Resolution:** core остаётся I/O-free (FOUND-01 ограничение). Вместо derive — helper `device_snapshot_json(&row) -> Result<String, serde_json::Error>` в `act_service.rs` пишет canonical 13-field JSON object; `restore_from_snapshot_in_tx` принимает `&serde_json::Value` и достаёт поля через `.get(...)/.as_i64()/.as_str()`. Это **функционально эквивалентно** (любой restore path работает), но сохраняет архитектурный invariant.

### 2. [Rule 2 - UX] `bulk_location_name` + `location_name_override` в DTO

**Discovery:** ReturnModal использует `DeviceAutocompleteField` для location-поля (per UI-SPEC §ReturnModal); autocomplete возвращает строку (имя), а DTO принимал только `bulk_location_id: Option<i64>`. Без UX-резолва UI не мог бы передать reasonable location в backend (нет API GET locations).

**Resolution:** Расширил DTO двумя дополнительными полями: `bulk_location_name: Option<String>` и `location_name_override: Option<String>`. Backend в `do_return` резолвит name через `resolve_location_id_in_tx` (INSERT OR IGNORE → SELECT). Имя имеет приоритет над id. Backward compat: тесты, передающие id напрямую, продолжают работать.

### 3. [Discretion-zone] Return-act giver/receiver наследуются от parent

**Plan §«ActService::do_return»:** «giver/receiver — берём те же из parent или из payload? Per D-Acts-Return-01 — giver/receiver на return могут отличаться; в этом плане **используем те же** из parent, **upgrade в plan 04** если UI спросит per-return giver/receiver».

**Resolution:** Выполнено по умолчанию плана — наследование от parent. UI не предлагает поля giver/receiver в ReturnModal в этом плане. Plan 04 может расширить ActReturnDto если потребуется.

### 4. [Discretion-zone] Cascade-delete handover делает LIFO undo

**Plan §«Implementation»:** «делать undo последовательно в обратном порядке — return undo первым, handover undo вторым».

**Resolution:** `delete_soft(handover_id)` собирает `list_returns_for_parent_in_tx(parent_id)` → итерирует через `.iter().rev()` (LIFO от sub_number DESC). Каждый return: undo own device-mutations → soft-delete return → audit. После всех returns — undo handover → soft-delete handover → audit. Если sequence handover → return №1 → return №2, undo идёт: ret2 undo → ret2 delete → ret1 undo → ret1 delete → handover undo → handover delete. Это обеспечивает корректное восстановление devices (последний return мог изменить condition после первого).

### 5. [Rule 2 - destructive copy refinement] Confirm-modal copy

**Plan output spec:** Use existing simple confirm() в ActsPage; UI-SPEC §Destructive actions предписывает richer copy.

**Resolution:** Заменил generic copy «Удалить акт №N? Действие можно отменить...» на per-variant copy:
- handover delete: «Все устройства... вернутся на склад в исходные Состояние и Расположение. Связанные возвраты также будут отменены.»
- return delete: «Состояние и Расположение устройств вернутся к значениям на момент выдачи. Если parent был в Архиве — выйдет из архива.»

Использован `window.confirm` (а не Modal-обёртка) для простоты — UI-SPEC §Destructive actions табличку допускает любой confirm-pattern.

## Known stubs

- **`acts.renderPdf` / `acts.search`** в `ui/src/lib/api/acts.ts` — остаются throw stubs до plan 04 (PDF) и post-plan-03 (search).
- **ActDetail «Печать» button** — остаётся disabled с tooltip «Доступно в plan 04».
- **`act.return_ids` в ActDetail** — рендерится как простой список IDs «Акт возврата #{id}». UI не показывает табличку возвратов в этом плане; pretty-history будет в plan 04 или post (с навигацией на return-акт). Текущее поведение не блокирует round-trip (UI updates после возврата через refresh + counts).

## Threat Flags

Никаких новых threat-flag — все новые поверхности (acts_return endpoint, undo path) уже описаны в `<threat_model>` плана: T-03-03-01..T-03-03-08. Все mitigations реализованы и покрыты тестами (T-03-03-01 → return_concurrent; T-03-03-02 → delete_handover_restores; T-03-03-04 → cascade test; T-03-03-05 → delete_act_audits_undo_entries; T-03-03-08 → recompute_parent_archived в single-writer tx).

## Self-Check: PASSED

Все заявленные файлы созданы и присутствуют в worktree:
- `crates/trackly-app/tests/acts_returns.rs` ✓
- `crates/trackly-app/tests/acts_display_rule.rs` ✓
- `crates/trackly-app/tests/acts_undo.rs` ✓
- `ui/src/features/acts/ReturnModal.svelte` ✓
- `ui/src/features/acts/ReturnItemsTable.svelte` ✓

Все 2 коммита plan 03-03 присутствуют в git log:
- `bfc700c` feat(03-03): do_return + sub_number sequencing + auto-archive + display-rule (Task 1)
- `c5ea284` feat(03-03): undo via audit_log replay + ReturnModal UI + ActDetail wiring (Task 2)
