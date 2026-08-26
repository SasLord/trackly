---
phase: 39-place-tree
reviewed: 2026-08-26T00:20:47Z
depth: standard
files_reviewed: 151
files_reviewed_list:
  - crates/trackly-app/src/context.rs
  - crates/trackly-app/src/dto/act.rs
  - crates/trackly-app/src/dto/cartridge.rs
  - crates/trackly-app/src/dto/device.rs
  - crates/trackly-app/src/dto/mod.rs
  - crates/trackly-app/src/dto/place.rs
  - crates/trackly-app/src/dto/printer.rs
  - crates/trackly-app/src/dto/reports.rs
  - crates/trackly-app/src/dto/request.rs
  - crates/trackly-app/src/http/cartridges.rs
  - crates/trackly-app/src/http/devices.rs
  - crates/trackly-app/src/http/mod.rs
  - crates/trackly-app/src/http/places.rs
  - crates/trackly-app/src/pdf/html_templates.rs
  - crates/trackly-app/src/services/act_service.rs
  - crates/trackly-app/src/services/cartridge_service.rs
  - crates/trackly-app/src/services/device_service.rs
  - crates/trackly-app/src/services/mod.rs
  - crates/trackly-app/src/services/place_service.rs
  - crates/trackly-app/src/services/report_service.rs
  - crates/trackly-app/src/services/request_service.rs
  - crates/trackly-app/src/services/template_service.rs
  - crates/trackly-app/src/specta_export.rs
  - crates/trackly-app/src/tauri_cmds/cartridges.rs
  - crates/trackly-app/src/tauri_cmds/devices.rs
  - crates/trackly-app/src/tauri_cmds/mod.rs
  - crates/trackly-app/src/tauri_cmds/places.rs
  - crates/trackly-app/src/tauri_cmds/printers.rs
  - crates/trackly-app/src/tauri_cmds/reports.rs
  - crates/trackly-app/templates/_legacy_defaults/v26/act_handover.html
  - crates/trackly-app/templates/act_handover.html
  - crates/trackly-app/templates/act_handover.minijinja
  - crates/trackly-app/tests/acts_archived_at.rs
  - crates/trackly-app/tests/acts_clone_handover.rs
  - crates/trackly-app/tests/acts_crud.rs
  - crates/trackly-app/tests/acts_date_source.rs
  - crates/trackly-app/tests/acts_e2e_smoke.rs
  - crates/trackly-app/tests/acts_http_smoke.rs
  - crates/trackly-app/tests/acts_numbering.rs
  - crates/trackly-app/tests/acts_place_snapshot.rs
  - crates/trackly-app/tests/acts_returns.rs
  - crates/trackly-app/tests/acts_search.rs
  - crates/trackly-app/tests/acts_suggest.rs
  - crates/trackly-app/tests/acts_undo.rs
  - crates/trackly-app/tests/acts_update.rs
  - crates/trackly-app/tests/acts_update_return.rs
  - crates/trackly-app/tests/devices_autocomplete.rs
  - crates/trackly-app/tests/devices_bulk_create.rs
  - crates/trackly-app/tests/devices_crud.rs
  - crates/trackly-app/tests/devices_csv_export.rs
  - crates/trackly-app/tests/devices_csv_import.rs
  - crates/trackly-app/tests/devices_grouping.rs
  - crates/trackly-app/tests/devices_http_smoke.rs
  - crates/trackly-app/tests/devices_location_roundtrip.rs
  - crates/trackly-app/tests/devices_search.rs
  - crates/trackly-app/tests/devices_type_conversion.rs
  - crates/trackly-app/tests/export_bindings.rs
  - crates/trackly-app/tests/html_act_render.rs
  - crates/trackly-app/tests/html_header_parity.rs
  - crates/trackly-app/tests/html_report_render.rs
  - crates/trackly-app/tests/pdf_column_overflow.rs
  - crates/trackly-app/tests/pdf_logo.rs
  - crates/trackly-app/tests/pdf_render_act.rs
  - crates/trackly-app/tests/phase06_stubs.rs
  - crates/trackly-app/tests/places_contents.rs
  - crates/trackly-app/tests/places_delete_blocked.rs
  - crates/trackly-app/tests/places_move_cycle.rs
  - crates/trackly-app/tests/places_search.rs
  - crates/trackly-app/tests/places_service_crud.rs
  - crates/trackly-app/tests/report_csv_export.rs
  - crates/trackly-app/tests/report_requests.rs
  - crates/trackly-app/tests/report_returns_sub_number.rs
  - crates/trackly-app/tests/request_printer_options.rs
  - crates/trackly-app/tests/role_endpoint_matrix.rs
  - crates/trackly-core/src/auth.rs
  - crates/trackly-core/src/domain/acts.rs
  - crates/trackly-core/src/domain/cartridges.rs
  - crates/trackly-core/src/domain/devices.rs
  - crates/trackly-core/src/domain/mod.rs
  - crates/trackly-core/src/domain/places.rs
  - crates/trackly-core/src/domain/printers.rs
  - crates/trackly-core/src/domain/requests.rs
  - crates/trackly-core/src/ports/mod.rs
  - crates/trackly-core/src/ports/places.rs
  - crates/trackly-infra/src/repos/acts_sqlite.rs
  - crates/trackly-infra/src/repos/cartridges_sqlite.rs
  - crates/trackly-infra/src/repos/devices_sqlite.rs
  - crates/trackly-infra/src/repos/mod.rs
  - crates/trackly-infra/src/repos/places_sqlite.rs
  - crates/trackly-infra/src/repos/printers_sqlite.rs
  - crates/trackly-infra/src/repos/requests_sqlite.rs
  - crates/trackly-infra/tests/cartridges_place_search.rs
  - crates/trackly-infra/tests/devices_place_search.rs
  - crates/trackly-infra/tests/migration_idempotency.rs
  - crates/trackly-infra/tests/per_record_invariants.rs
  - crates/trackly-infra/tests/places_crud.rs
  - migrations/V037__places.sql
  - migrations/V038__places_migrate_devices_acts_cartridges.sql
  - ui/eslint.config.js
  - ui/src/bindings-phase6.ts
  - ui/src/features/acts/ActDetail.svelte
  - ui/src/features/acts/ActFormBody.svelte
  - ui/src/features/acts/ActFormItemsTable.svelte
  - ui/src/features/acts/ReturnItemsTable.svelte
  - ui/src/features/acts/ReturnModal.svelte
  - ui/src/features/acts/returnPayload.ts
  - ui/src/features/cartridges/CartridgeDetail.svelte
  - ui/src/features/cartridges/CartridgeFormBody.svelte
  - ui/src/features/cartridges/CartridgeListRow.svelte
  - ui/src/features/cartridges/CartridgesList.svelte
  - ui/src/features/cartridges/CartridgesPage.svelte
  - ui/src/features/cartridges/CompatibilityEditor.svelte
  - ui/src/features/cartridges/ModelFormModal.svelte
  - ui/src/features/cartridges/OperationModal.svelte
  - ui/src/features/cartridges/api.ts
  - ui/src/features/devices/DeviceAutocompleteField.svelte
  - ui/src/features/devices/DeviceFormBody.svelte
  - ui/src/features/devices/DeviceGroupRow.svelte
  - ui/src/features/devices/DeviceImportCsvModal.svelte
  - ui/src/features/devices/DeviceList.svelte
  - ui/src/features/devices/DeviceListRow.svelte
  - ui/src/features/devices/DevicesPage.svelte
  - ui/src/features/layout/sidebar-config.ts
  - ui/src/features/places/PlaceContents.svelte
  - ui/src/features/places/PlaceEntityViewModal.svelte
  - ui/src/features/places/PlaceFormModal.svelte
  - ui/src/features/places/PlaceMoveModal.svelte
  - ui/src/features/places/PlaceTree.svelte
  - ui/src/features/places/PlaceTreeNode.svelte
  - ui/src/features/places/PlacesMasterDetail.svelte
  - ui/src/features/places/PlacesPage.svelte
  - ui/src/features/printers/PrinterCreateModal.svelte
  - ui/src/features/printers/PrinterDetail.svelte
  - ui/src/features/printers/PrinterListRow.svelte
  - ui/src/features/printers/PrintersList.svelte
  - ui/src/features/printers/PrintersPage.svelte
  - ui/src/features/reports/ReportFilters.svelte
  - ui/src/features/reports/ReportTable.svelte
  - ui/src/features/reports/ReportsPage.svelte
  - ui/src/features/requests/RequestDetail.svelte
  - ui/src/features/settings/TemplateEditor.svelte
  - ui/src/features/showcase/ShowcasePage.svelte
  - ui/src/features/showcase/sections/PlacePickerSection.svelte
  - ui/src/features/showcase/sections/TableSection.svelte
  - ui/src/lib/components/Badge.svelte
  - ui/src/lib/components/GroupedPrinterSelect.svelte
  - ui/src/lib/components/PersonAutocomplete.svelte
  - ui/src/lib/components/PlacePicker.svelte
  - ui/src/lib/components/PrinterSelect.svelte
  - ui/src/lib/utils/hashId.ts
  - ui/src/routes.ts
findings:
  critical: 1
  warning: 2
  info: 2
  total: 5
status: critical_resolved
---

# Phase 39: Code Review Report

**Reviewed:** 2026-08-26T00:20:47Z
**Depth:** standard
**Files Reviewed:** 151
**Status:** issues_found

## Summary

Phase 39 replaces the flat `locations` table with an adjacency-list `places`
tree and threads `place_id` through devices, cartridges, acts, reports,
requests and printers, across both the Tauri and axum transports. The bulk of
the implementation is solid: the D-20 Admin/Manager read-vs-mutate split is
enforced identically and defense-in-depth on both transports (verified
against `role_endpoint_matrix.rs`), the recursive-CTE tree SQL in
`places_sqlite.rs` (subtree stats, cycle check, storage-ancestor inheritance)
is parameterized and correct, the D-16 "frozen snapshot" columns are kept
genuinely distinct from the live-resolved `place_full_paths` join at both the
schema and query level, and the many previously-found UAT gaps (CSV
place-column mapping, move-to-root unreachable, printer deep-link kind
mismatch, drag-ghost hit-testing) are demonstrably fixed with the fix
reasoning documented in-line.

One real defect was found that reaches a documented, ship-blocking product
constraint (Russian-only UI copy) and a spec'd feature (D-14's delete-block
message): the delete-block pre-flight check that powers `places_delete`'s
friendly "нельзя удалить, содержит N устройств…" message does not account
for acts/act_items that still reference the place, even though D-16
guarantees such references outlive any device that has since moved away.
Deleting such a place bypasses the friendly pre-check, then fails on a raw
SQLite foreign-key error that is shown verbatim, in English, inside the
Russian-only "Удалить место?" dialog. Two further warnings and two info-level
findings are below.

## Critical Issues

### CR-01: D-14 delete-block pre-check ignores acts/act_items referencing the place — raw English SQLite error leaks into the Russian-only delete dialog

**File:** `crates/trackly-infra/src/repos/places_sqlite.rs:114-143` (`subtree_stats_impl`), `crates/trackly-infra/src/repos/places_sqlite.rs:427-462` (`delete_hard`), `crates/trackly-app/src/services/place_service.rs:369-423` (`PlaceService::delete_hard`), `migrations/V038__places_migrate_devices_acts_cartridges.sql:17-20`

**Issue:**
`SubtreeStats` (`crates/trackly-core/src/domain/places.rs:130-136`) — the
struct that powers both the D-14 pre-flight "can this place be deleted"
check and the friendly UI-SPEC §11.5/§14.3 blocked-message copy
(`build_delete_blocked_message` in `place_service.rs:636-668`) — only counts
`direct_children`, `nested_places`, `device_count` and `cartridge_count`. It
never counts rows in `acts` (`place_id`, `bulk_place_id`) or `act_items`
(`place_id_override`) that reference the place being deleted.

But `V038` adds exactly those three columns as
`REFERENCES places(id) ON DELETE RESTRICT` (lines 17-20), and D-16
deliberately freezes `place_path_snapshot`/`place_id` on an act so a later
change to the place tree — including moving every device *away* from that
place — never alters an already-issued act. That is precisely the situation
that makes a place with `device_count == 0`, `cartridge_count == 0`,
`nested_places == 0` still **not actually deletable**: it is a completely
ordinary, expected state (issue an act, then later move the device
elsewhere) for a place to be referenced by history while sitting empty in
the live tree.

When this happens:
1. `PlaceService::delete_hard`'s reader-pool pre-check (`place_service.rs:372-390`)
   sees an all-zero `SubtreeStats`, reports "not blocked", and proceeds to
   the writer.
2. The writer's own `SqlitePlaceRepository::delete_hard` (`places_sqlite.rs:427-462`)
   re-runs the same incomplete check, also sees zero, and executes
   `DELETE FROM places WHERE id = ?1 AND version = ?2`.
3. SQLite's foreign-key enforcement (always on per `CLAUDE.md`'s
   `PRAGMA foreign_keys = ON`) rejects the `DELETE` because `acts.place_id`
   (or `bulk_place_id`, or `act_items.place_id_override`) still points at
   this row.
4. `error_conversions::map_rusqlite` (`crates/trackly-infra/src/error_conversions.rs:31-47`)
   maps the SQLite `ConstraintViolation` into
   `AppError::Conflict { reason: <raw sqlite message> }`.
5. `AppError::Conflict`'s `Display` impl is `"conflict: {reason}"`
   (`crates/trackly-core/src/error.rs:46-49`) — this raw, English,
   `"conflict: FOREIGN KEY constraint failed"`-shaped string becomes the
   HTTP/Tauri error's `message` field verbatim (no translation layer
   exists for constraint text, unlike the D-04 duplicate-name path which
   *does* get translated by `PlaceService::duplicate_name_error`).
6. `PlaceTree.svelte`'s `confirmDelete` (`ui/src/features/places/PlaceTree.svelte:535-566`)
   treats any `code === 'CONFLICT'` as the "blocked" case and renders
   `err.message` verbatim as `deleteState.blockedMessage` inside the
   "Удалить место?" modal, offering "Показать содержимое" / "Архивировать"
   actions that make no sense for this error (there is no content to show —
   the place is empty of devices/cartridges/children).

This both violates the project's hard Russian-only-UI constraint
(`CLAUDE.md`: "UI и шаблоны документов — только русский в v1") by surfacing
raw English SQLite text to the end user, and breaks the spec'd D-14 feature
itself: the admin gets no indication that the place is still referenced by
one or more handover acts, and no actionable next step (there is nothing to
archive/move — an act's frozen `place_id` isn't reachable from any UI
mutation).

No test in `places_delete_blocked.rs`, `acts_place_snapshot.rs`, or
`places_crud.rs` exercises "place referenced only by an act, otherwise
empty" — which is why `cargo test` stays green.

**Fix:** Extend `SubtreeStats` (or add a sibling count) to include acts/act_items
referencing the subtree, and surface it through `build_delete_blocked_message`
with real Russian copy, e.g.:

```rust
// domain/places.rs
pub struct SubtreeStats {
    pub direct_children: i64,
    pub nested_places: i64,
    pub device_count: i64,
    pub cartridge_count: i64,
    pub referencing_act_count: i64, // NEW
}
```

```sql
-- places_sqlite.rs subtree_stats_impl, additional SELECT clause:
(SELECT COUNT(DISTINCT a.id) FROM acts a
   LEFT JOIN act_items ai ON ai.act_id = a.id
 WHERE (a.place_id IN (SELECT id FROM subtree)
     OR a.bulk_place_id IN (SELECT id FROM subtree)
     OR ai.place_id_override IN (SELECT id FROM subtree))
   AND a.deleted_at_utc IS NULL) AS referencing_act_count
```

Then have `build_delete_blocked_message` add a "N актов ссылается на это
место" clause (or a distinct, dedicated message, since — unlike devices —
there is no user action that clears this reference) so the pre-check
actually blocks the delete with a correct, localized message instead of
letting it fall through to the raw FK violation.

## Warnings

### WR-01: Place-tree content-count badges never invalidate, including on explicit "Обновить"

**File:** `ui/src/features/places/PlaceTree.svelte:294-313` (stats cache), `ui/src/features/places/PlaceTree.svelte:800` ("Обновить" button), `ui/src/features/places/PlaceTree.svelte:227-269` (`loadTree`)

**Issue:** `statsCache` (populated by the `$effect` at lines 297-313, one
`places_subtree_stats` call per visible node) is only ever added to, never
cleared. `loadTree()` (lines 227-269) — which re-fetches `allPlaces` on
`showArchived`/`refreshToken` change and is also what the "Обновить" button
at line 800 calls — never resets `statsCache`. Since the `$effect` at line
297 explicitly skips any id already present in `statsCache`
(`if (statsCache[id] !== undefined || statsInFlight.has(id)) continue;`),
once a node's per-node counter (device+cartridge count under that subtree)
has been fetched once, it is frozen for the remainder of the component's
mounted lifetime — even though moving a device/cartridge into or out of that
subtree (via `DeviceFormModal`, `CartridgeFormModal`, `ReturnModal`,
`OperationModal` — none of which know about `PlaceTree`) changes the true
count immediately. Clicking "Обновить" — the button whose entire purpose is
"get current data" — silently does not refresh this piece of visible data.

**Fix:** Reset `statsCache = {}` (and clear `statsInFlight`) at the top of
`loadTree()`, or at minimum whenever `refreshToken` changes, so "Обновить"
actually refreshes what it visibly shows.

### WR-02: Debounced place search has no request-ordering guard — a stale response can overwrite a fresher one

**File:** `ui/src/lib/components/PlacePicker.svelte:303-323` (`scheduleSearch`), `ui/src/features/places/PlaceTree.svelte:326-346` (`scheduleSearch`)

**Issue:** Both `scheduleSearch` implementations debounce via
`clearTimeout`, but once a debounced timer fires and its `fetchSearchResults`
(or `apiCall('places_search', …)`) promise is in flight, a subsequent
keystroke starts a *new* timer/fetch without cancelling the in-flight one.
If the newer request's network round-trip completes before the older one's,
the older (now-stale) response's `.then()` still unconditionally overwrites
`searchResults`/`statsCache`-equivalent state when it eventually resolves,
because neither implementation carries a request generation/token that the
callback checks before committing its result. In `PlacePicker.svelte`, this
can silently replace the results for what the user is currently looking at
with results for a query they've since changed or cleared; in
`PlaceTree.svelte`, the same class of race can leave `activeId` and
`liveMessage` pointing at a result set that no longer matches
`searchResults`.

**Fix:** Track a monotonically increasing request id (or use `AbortController`
if `apiCall` supports it) and ignore a resolved response whose id doesn't
match the latest dispatched request:

```ts
let searchSeq = 0;
searchDebounceTimer = setTimeout(async () => {
  const mySeq = ++searchSeq;
  const results = await fetchSearchResults(query);
  if (mySeq !== searchSeq) return; // superseded by a newer search
  searchResults = results.slice(0, 50);
}, 200);
```

## Info

### IN-01: `PlaceTreeNodeDto` is dead code whose doc comment describes behavior the shipped code does not implement

**File:** `crates/trackly-app/src/dto/place.rs:70-100`

**Issue:** `PlaceTreeNodeDto` (every `PlaceDto` field plus `content_count`)
is never constructed anywhere in the Rust codebase (no service method
returns it, no Tauri command or HTTP handler serializes it) and is not
referenced from the frontend. Its doc comment claims `content_count` is "sum
of devices/printers/cartridges/nested places under this node, INCLUDING
nested places" — but the actual, shipped per-node tree counter
(`PlaceTree.svelte:304`, `statsCache = {...statsCache, [id]: s.device_count + s.cartridge_count}`)
deliberately excludes `nested_places` and is computed client-side via N+1
`places_subtree_stats` calls, not via this DTO at all. A future maintainer
who reads this struct's doc comment and assumes it reflects current
behavior will build the wrong thing.

**Fix:** Either delete `PlaceTreeNodeDto` (and its now-inaccurate doc
comment) since it has no callers, or wire it up as the documented
single-round-trip tree hydration and have the frontend consume it instead
of the current N+1 `places_subtree_stats` polling loop.

### IN-02: D-04 sibling-name uniqueness is case-sensitive, allowing visually-duplicate names under the same parent

**File:** `migrations/V037__places.sql:21-25`

**Issue:** `idx_places_parent_name_unique` is
`ON places(COALESCE(parent_id, 0), name) WHERE deleted_at_utc IS NULL` with
no `COLLATE NOCASE`. SQLite's default `TEXT` comparison in a `UNIQUE INDEX`
is byte-for-byte (case-sensitive), so "Кабинет 214" and "кабинет 214" (or
"Room" vs "room") are treated as distinct siblings and both can coexist
under the same parent — the friendly "уже есть место «Название»" duplicate
check (`PlaceService::duplicate_name_error`) never fires for a case-only
variant. This is a judgment call rather than a proven regression (no spec
text mandates case-insensitive dedup), but it is worth flagging: the
resulting tree can silently accumulate near-duplicate siblings that read as
identical to a human but are functionally two different places (each with
its own `place_id`, its own device/cartridge assignments, and its own
`full_path`), which is exactly the kind of confusion the whole
D-17/full-path-search feature is trying to eliminate.

**Fix:** If unintentional, add `COLLATE NOCASE` to the index (and to
`PlaceKind`/name comparisons elsewhere that assume case-insensitivity); if
intentional, a one-line comment next to the index would save the next
reader from re-deriving this.

---

_Reviewed: 2026-08-26T00:20:47Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_


---

## Пост-ревью: статус исправлений (оркестратор, 2026-08-26)

- **CR-01 — ИСПРАВЛЕНО** (коммит `22e2c6c6`). `SubtreeStats` получил
  `referencing_act_count`; `subtree_stats_impl` считает DISTINCT акты по всем трём
  внешним ключам (`acts.place_id`, `acts.bulk_place_id`, `act_items.place_id_override`)
  с дедупликацией и учётом мягкого удаления; счёт включён в блокирующее условие
  `delete_hard` и в оба сообщения (репозиторий + `build_delete_blocked_message`
  с правильной формой «акт/акта/актов»).
  Добавлено 6 регрессионных тестов (2 в `places_crud.rs`, 4 в `places_delete_blocked.rs`),
  покрывающих все три пути ссылки и случай двойной ссылки.
  **Доказательство:** перед восстановлением фикса production-файлы были убраны в
  `git stash` и новый тест прогнан против старого кода — упал именно с
  `FOREIGN KEY constraint failed`, как и предсказывало ревью.
  Гейты после фикса: clippy чист · trackly-app 750/0 · trackly-infra 174/0 ·
  svelte-check 0 ошибок · pnpm lint PASS · сборка успешна.

- **WR-01, WR-02, INFO-01, INFO-02 — не исправлялись**, вынесены в долг вехи.
