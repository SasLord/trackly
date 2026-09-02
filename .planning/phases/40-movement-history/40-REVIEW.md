---
phase: 40-movement-history
reviewed: 2026-09-02T12:44:21Z
depth: standard
files_reviewed: 89
files_reviewed_list:
  - crates/trackly-app/src/context.rs
  - crates/trackly-app/src/dto/mod.rs
  - crates/trackly-app/src/dto/place_movements.rs
  - crates/trackly-app/src/dto/reports.rs
  - crates/trackly-app/src/http/health.rs
  - crates/trackly-app/src/http/mod.rs
  - crates/trackly-app/src/http/place_movements.rs
  - crates/trackly-app/src/http/places.rs
  - crates/trackly-app/src/http/reports.rs
  - crates/trackly-app/src/services/act_service.rs
  - crates/trackly-app/src/services/cartridge_service.rs
  - crates/trackly-app/src/services/device_service.rs
  - crates/trackly-app/src/services/mod.rs
  - crates/trackly-app/src/services/place_movement_service.rs
  - crates/trackly-app/src/services/place_path_display.rs
  - crates/trackly-app/src/services/place_service.rs
  - crates/trackly-app/src/services/report_service.rs
  - crates/trackly-app/src/specta_export.rs
  - crates/trackly-app/src/tauri_cmds/acts.rs
  - crates/trackly-app/src/tauri_cmds/cartridges.rs
  - crates/trackly-app/src/tauri_cmds/devices.rs
  - crates/trackly-app/src/tauri_cmds/health.rs
  - crates/trackly-app/src/tauri_cmds/mod.rs
  - crates/trackly-app/src/tauri_cmds/place_movements.rs
  - crates/trackly-app/src/tauri_cmds/places.rs
  - crates/trackly-app/src/tauri_cmds/reports.rs
  - crates/trackly-app/tests/acts_archived_at.rs
  - crates/trackly-app/tests/acts_clone_handover.rs
  - crates/trackly-app/tests/acts_crud.rs
  - crates/trackly-app/tests/acts_date_source.rs
  - crates/trackly-app/tests/acts_e2e_smoke.rs
  - crates/trackly-app/tests/acts_http_smoke.rs
  - crates/trackly-app/tests/acts_place_path_short.rs
  - crates/trackly-app/tests/acts_place_snapshot.rs
  - crates/trackly-app/tests/acts_returns.rs
  - crates/trackly-app/tests/acts_search.rs
  - crates/trackly-app/tests/acts_suggest.rs
  - crates/trackly-app/tests/acts_undo.rs
  - crates/trackly-app/tests/acts_update.rs
  - crates/trackly-app/tests/acts_update_return.rs
  - crates/trackly-app/tests/cartridges_crud.rs
  - crates/trackly-app/tests/cartridges_history.rs
  - crates/trackly-app/tests/cartridges_lifecycle.rs
  - crates/trackly-app/tests/devices_crud.rs
  - crates/trackly-app/tests/devices_location_roundtrip.rs
  - crates/trackly-app/tests/devices_type_conversion.rs
  - crates/trackly-app/tests/html_act_render.rs
  - crates/trackly-app/tests/html_header_parity.rs
  - crates/trackly-app/tests/html_report_render.rs
  - crates/trackly-app/tests/pdf_column_overflow.rs
  - crates/trackly-app/tests/pdf_logo.rs
  - crates/trackly-app/tests/pdf_render_act.rs
  - crates/trackly-app/tests/place_movements_act_link.rs
  - crates/trackly-app/tests/place_movements_bulk_move.rs
  - crates/trackly-app/tests/place_movements_timeline.rs
  - crates/trackly-app/tests/place_movements_write_sites_cartridges.rs
  - crates/trackly-app/tests/place_movements_write_sites_devices.rs
  - crates/trackly-app/tests/report_csv_export.rs
  - crates/trackly-app/tests/report_movements.rs
  - crates/trackly-app/tests/report_place_path_short.rs
  - crates/trackly-app/tests/report_place_subtree.rs
  - crates/trackly-app/tests/report_requests.rs
  - crates/trackly-app/tests/report_returns_sub_number.rs
  - crates/trackly-app/tests/reports_period_required.rs
  - crates/trackly-app/tests/role_endpoint_matrix.rs
  - crates/trackly-app/tests/specta_roundtrip.rs
  - crates/trackly-app/tests/templates_status.rs
  - crates/trackly-core/src/domain/mod.rs
  - crates/trackly-core/src/domain/place_movements.rs
  - crates/trackly-infra/src/repos/cartridges_sqlite.rs
  - crates/trackly-infra/src/repos/mod.rs
  - crates/trackly-infra/src/repos/place_movements_sqlite.rs
  - crates/trackly-infra/tests/place_movements_migration.rs
  - crates/trackly-infra/tests/place_movements_repo.rs
  - migrations/V040__place_movements.sql
  - ui/src/features/acts/ActsPage.svelte
  - ui/src/features/cartridges/CartridgeDetail.svelte
  - ui/src/features/devices/DeviceContextMenu.svelte
  - ui/src/features/devices/DeviceListRow.svelte
  - ui/src/features/places/PlaceContents.svelte
  - ui/src/features/places/PlaceEntityViewModal.svelte
  - ui/src/features/printers/PrinterDetail.svelte
  - ui/src/features/reports/ReportFilters.svelte
  - ui/src/features/reports/ReportSubNav.svelte
  - ui/src/features/reports/ReportTable.svelte
  - ui/src/features/reports/ReportsPage.svelte
  - ui/src/features/showcase/ShowcasePage.svelte
  - ui/src/features/showcase/sections/MovementTimelineSection.svelte
  - ui/src/lib/components/MovementTimeline.svelte
findings:
  critical: 2
  warning: 2
  info: 1
  total: 5
status: issues_found
---

# Phase 40: Code Review Report

**Reviewed:** 2026-09-02T12:44:21Z
**Depth:** standard
**Files Reviewed:** 89 (43 test files sampled for coverage evidence, 46 source files read in full)
**Status:** issues_found

## Summary

The core write path is well built: `SqlitePlaceMovementsRepository::record_movement_if_applicable`
is genuinely the single funnel every write site goes through, the D-04/D-06 skip guard
(`is_reportable_place_change`) lives in exactly one place, the cartridge `transition_in_tx` nested
auto-return correctly attributes its movement row to `prev_id` (not the newly-installed cartridge),
`delete_soft`'s undo scoping deletes each act's own `place_movements` rows at its own point in the
LIFO cascade (not one blanket delete), both transports gate the new timeline/report/bulk-move
endpoints on the correct actions, `role_endpoint_matrix.rs` has real Manager/Employee coverage for
every new endpoint on both transports, the SQL report builder binds every filter value as a
parameter (no interpolation) and its two subtree CTEs correctly combine with AND, and no JS mirror
of the path-shortening formula was introduced.

Two real defects were found, both silent (no crash, no test failure) — which is exactly why they
survived to submission:

1. Editing an already-created act to drop one device (`ActService::update`) or to un-return one
   device from an existing return act (`ActService::update_return`) restores that device's
   `place_id` to its pre-act value without ever touching `place_movements` — the entity's timeline
   goes stale/incorrect at that point, silently.
2. The movement timeline (`PlaceMovementService::get_timeline`, HST-02's own read path) resolves
   `act_number` by reading the `acts.number` INTEGER column as a Rust `String`, which fails every
   single time and is silently swallowed by `.ok()` — every act-linked row in every device/cartridge
   timeline shows "актом" with no number, and D-19's "номер акта кликабелен" never renders. The
   sibling report path (`report_service.rs::query_movements_inner`) reads the same column correctly
   as `i64` — the type mismatch is isolated to this one query.

## Critical Issues

### CR-01: Editing an act to remove one device silently drops its place-history entry

**File:** `crates/trackly-app/src/services/act_service.rs:909-941` (`ActService::update`, step "8c.
Removed devices") and `crates/trackly-app/src/services/act_service.rs:1953-1988`
(`ActService::update_return`, step "9. MUTATE `removed`")

**Issue:** Both of these blocks call `devices_repo.restore_from_snapshot_in_tx(&tx, removed_id,
&snapshot, now)`, which unconditionally writes `place_id` back to whatever it was before the device
was added to (or returned by) this act — a genuine `place_id` mutation, confirmed by
`devices_sqlite.rs:436` (`place_id = ?10`, not `COALESCE`). Every other place-mutating branch in
this same file (the "added" loop in `create`/`update`, and the "added"/"retained_with_change" loops
in `do_return`/`update_return`) calls `place_movements_repo.record_movement_if_applicable(...)`
immediately after its own `devices_repo` mutation. These two "removed"/"un-return" branches do not
— there is no call to `record_movement_if_applicable`, and no call to
`place_movements_repo.delete_by_act_id_in_tx` either (which would at least remove the now-stale
row instead of leaving it).

Concretely: `create()` (or a prior `update()`) records `place_movements(from=P1, to=P2, act_id=A)`
when a device is added to handover act A. If the act is later edited to drop that device,
`restore_from_snapshot_in_tx` puts the device back at P1 — but the `place_movements` row still says
P1→P2, and no new row records the P2→P1 reversion. The device's actual current place (P1) no
longer matches the `to_place_id` of its most recent movement row (P2). HST-01's own promise
("каждая смена места ... записывается в историю") is violated for this specific, already-tested,
already-reachable code path.

This is not a hypothetical: `crates/trackly-app/tests/acts_update.rs::remove_position_restores_prior_state`
(pre-existing, not touched by this phase) exercises exactly this sequence — create a handover that
moves a device from `Склад-A` to `Кабинет-B`, then edit the act to drop that device, and assert it
is restored to `Склад-A`. No Phase 40 test (`place_movements_act_link.rs`,
`place_movements_write_sites_devices.rs`) covers this scenario against `place_movements`, so the
gap shipped untested.

The identical pattern exists a second time in `update_return`'s un-return branch (line 1973) for
the return-act-edit flow.

**Fix:** Add a `record_movement_if_applicable` call right after each `restore_from_snapshot_in_tx`
call, using the snapshot's pre-restore `place_id` as `before` and the restored row's `place_id` as
`after`, with `source: MovementSource::Act` and `act_id: Some(payload.id)` — mirroring the "added"
loop exactly:
```rust
let restored = devices_repo.restore_from_snapshot_in_tx(&tx, removed_id, &snapshot, now)?;
// snapshot.place_id is what the device had WHILE part of this act (the "from" side of the
// reversion); restored.place_id is where it landed after restore (the "to" side).
let snapshot_place_id: Option<i64> = snapshot.get("place_id").and_then(|v| v.as_i64());
place_movements_repo.record_movement_if_applicable(
    &tx,
    places_repo.as_ref(),
    MovementEntityKind::Device,
    removed_id,
    snapshot_place_id,
    restored.place_id,
    MovementSource::Act,
    None,
    Some(payload.id),
    user_id_opt,
    now,
)?;
```
Add a regression test asserting a `place_movements` row (or the correct net effect) exists after
removing a device from an act edit, for both `update()` and `update_return()`.

### CR-02: Movement timeline never shows the act number — `acts.number` read as the wrong SQL type, error silently swallowed

**File:** `crates/trackly-app/src/services/place_movement_service.rs:83-94`

**Issue:**
```rust
let act_number: Option<String> = row.act_id.and_then(|act_id| {
    conn.query_row(
        "SELECT number FROM acts WHERE id = ?1",
        params![act_id],
        |r| r.get::<_, String>(0),
    )
    .optional()
    .ok()
    .flatten()
});
```
`acts.number` is declared `INTEGER NOT NULL` (`migrations/V004__acts.sql:12`). rusqlite's
`FromSql for String` only accepts SQLite's `Text` storage class (`ValueRef::as_str()` returns
`Err(FromSqlError::InvalidType)` for `ValueRef::Integer`), so `r.get::<_, String>(0)` fails on
every single row, every time — not just on a missing act. `.optional()` only translates
`QueryReturnedNoRows` into `Ok(None)`; every other error variant (including this one) still comes
back as `Err(...)`, which the trailing `.ok()` then silently discards, so `act_number` is `None`
100% of the time an act-linked movement is rendered.

This is a real, deterministic regression against D-19 ("номер акта → карточка акта" — clickable)
and the phase's own documented timeline format
(`01.09 — ... → ... · Иванов И.И. · актом №123`): `MovementTimeline.svelte`'s template
(`{#if entry.source === 'act' && entry.act_id !== null && entry.act_number}`) will never take the
clickable-number branch, and `reasonText()`'s fallback for `source === 'act'` returns the bare
string `"актом"` with no number, for every act-linked row, in both consumers (`PlaceEntityViewModal`
and `CartridgeDetail`/`PrinterDetail`).

Confirmed by contrast with the correct precedent already present in this exact codebase for the
exact same column: `report_service.rs:1242` (`CAST(a.number AS TEXT) as number`) and
`acts_sqlite.rs:264,294` both explicitly `CAST` `acts.number` to TEXT before treating it as a
string — `report_service.rs::query_movements_inner` (the sibling HST-04 report path) avoids this
bug entirely by reading the same column as `i64` (`let act_number: Option<i64> = r.get(11)?;`,
line ~1462) instead. No test in `place_movements_timeline.rs` or `place_movements_act_link.rs`
asserts on `act_number`, which is how this slipped through.

**Fix:** Read the column as its actual type and format afterward:
```rust
let act_number: Option<String> = row.act_id.and_then(|act_id| {
    conn.query_row(
        "SELECT number FROM acts WHERE id = ?1",
        params![act_id],
        |r| r.get::<_, i64>(0),
    )
    .optional()
    .ok()
    .flatten()
    .map(|n| n.to_string())
});
```
Add a test in `place_movements_timeline.rs` (or extend `place_movements_act_link.rs`) that seeds a
movement with a real `act_id`, calls `PlaceMovementService::get_timeline`, and asserts
`act_number == Some("<the real number>")` — this exact assertion would have caught the bug.

## Warnings

### WR-01: D-25's "удалено" marker is UI-table-only — CSV/PDF exports of the movements report lose it

**File:** `crates/trackly-app/src/services/report_service.rs:1108` (`row_field`'s `"device_name"`
match arm, used by both `export_csv` and `export_pdf`)

**Issue:** D-25 requires that a soft-deleted item's movement row "остаётся, рядом пометка
«удалено»" (stays, with an "удалено" marker next to it) — and D-26 requires CSV/PDF export parity
with the other 12 reports. The row itself does stay (confirmed: `is_deleted` is correctly computed
and threaded onto `ReportRow`), but the marker is only rendered by
`ui/src/features/reports/ReportTable.svelte:174` (`showDeletedBadge`), a live-table-only CSS badge.
`row_field`'s `"device_name"` arm (used by CSV via `export_csv`'s `row_field(row, col, tz, false)`
and by PDF/HTML via the same function) returns `row.device_name.as_deref().unwrap_or("")` with no
reference to `row.is_deleted` at all. An exported CSV or printed PDF of the movements report is
indistinguishable, row-for-row, between an item that is still in inventory and one that was
written off yesterday — exactly the ambiguity D-25 exists to prevent for a "report for a past
period [that] never changes shape."

`report_movements_export_csv_has_d23_headers`/`export_pdf_has_d23_headers`
(`tests/report_movements.rs`) only assert the column *headers* are present — neither test seeds a
soft-deleted item and checks the exported *body* for a marker, so this gap has no regression
coverage either.

**Fix:** In `row_field`'s `"device_name"` arm, append a suffix when `row.is_deleted == Some(true)`,
e.g. `format!("{name} (удалено)")` for CSV, and the equivalent for the PDF template (or a
conditional cell class in `report.html` if it renders visually rather than through `row_field`).
Add a test seeding a soft-deleted device's movement and asserting the exported CSV/PDF body
contains the marker text.

### WR-02: Movements report export gate diverges from movements report list gate (currently harmless, but fragile)

**File:** `crates/trackly-app/src/tauri_cmds/reports.rs:274-303`
(`build_reports_export_csv`/`build_reports_export_pdf`)

**Issue:** `build_reports_list_movements` deliberately gates on `Action::ReadPlaces` per D-12 (this
is documented and correctly implemented — see the doc-comment at line 255). However,
`build_reports_export_csv`/`build_reports_export_pdf` gate uniformly on `Action::ReadData` for
*every* `report_type` string, including `"movements"` — they never branch to `ReadPlaces` for that
one type. Today this is not exploitable: `auth.rs`'s permission matrix currently grants `ReadData`
and `ReadPlaces` to the exact same two roles (Admin | Manager, Employee excluded), and this is
explicitly tested (`role_endpoint_matrix.rs` Cases 56/57 assert Employee gets 403 on movements
export via the `ReadData` gate). But the semantic intent stated in D-12 ("Доступ — Admin + Manager
... Гейт на бэкенде, на обоих транспортах" for *both* the timeline read and the report) is only
actually enforced via `ReadPlaces` for the list path; the export path rides on a *coincidentally*
identical role set via a different `Action`. If `ReadData`'s role set is ever widened independently
of `ReadPlaces` (e.g. a future "read-only auditor" role granted `ReadData` but not `ReadPlaces`),
CSV/PDF export of the movements report would silently gain that wider audience while the on-screen
list stays correctly restricted — a divergence that would be easy to miss in review because both
gates currently "just work."

**Fix:** Either special-case `"movements"` inside `build_reports_export_csv`/`_export_pdf` to gate
on `Action::ReadPlaces` (matching the list path exactly), or accept the current design explicitly
and add a comment/test that pins `Action::ReadData`'s role set to equal `Action::ReadPlaces`'s role
set so a future divergence fails a test instead of shipping silently.

## Info

### IN-01: Verified-clean phase-specific checks

The following checks from the review brief were explicitly verified and found clean — noting them
so it's clear what was checked, not just what failed:

- **Transaction discipline:** every `record_movement_if_applicable`/`delete_by_act_id_in_tx` call
  found (device/cartridge manual update, cartridge transition + nested auto-return, all four act
  write sites, `delete_soft`'s three delete points, `move_subtree_contents`) runs against the
  caller's already-open `&Transaction<'_>` — no site opens a second transaction or a bare
  `INSERT INTO place_movements`.
- **Skip-rule single ownership:** `is_reportable_place_change` (D-04/D-06) lives only in
  `trackly-core::domain::place_movements` and is called only from
  `record_movement_if_applicable`; no call site re-derives the guard.
- **Nested auto-return entity attribution:** `cartridges_sqlite.rs:674-687` correctly records the
  auto-returned cartridge's movement against `prev_id`/`prev_current.place_id`, not the newly
  installed cartridge's id — confirmed by dedicated tests
  (`transition_in_tx_stores_caller_user_id_on_auto_return_and_main`).
- **Undo scoping (D-03/Pitfall 5):** `delete_soft`'s handover-cascade calls
  `delete_by_act_id_in_tx(&tx, ret.id)` inside the per-return LIFO loop and again for the handover's
  own id afterward — not one blanket delete — matching the research's prescribed shape exactly.
- **Authorization on both transports:** timeline read, movements report list, and bulk-move all
  gate identically on Tauri and HTTP (`build_*` helper + thin adapter pattern), and
  `role_endpoint_matrix.rs` Cases 52-59 give real Manager/Employee coverage for all three on both
  transports.
- **`source` is server-set:** every write site passes a hardcoded `MovementSource::*` variant;
  clients only ever supply free-text `note`. The one client-facing read of `source`
  (`MovementEntryDto.source: String`) is passed through raw and degrades softly in both consumers
  (`report_service::movement_reason`'s `None` arm, `MovementTimeline.svelte`'s `reasonText`
  fallback) — an unrecognized token never panics or crashes a screen.
- **No duplicated path-shortening:** `act_service.rs`, `place_movement_service.rs`, and
  `report_service.rs` all import the single promoted `place_path_display::compute_place_path_short`.
  `report_service.rs`'s own pre-existing private `compute_place_path_short` (different signature,
  pre-resolved variant) predates this phase and is a distinct function, not a new copy — correctly
  not a phase-40 defect, per the review brief's own note. No JS mirror of the formula was found in
  `ui/`; `ReportTable.svelte`/`MovementTimeline.svelte` both display only backend-supplied
  `*_short` fields with the full value in `title=`.
- **SQL injection surface:** `query_movements_inner`'s dynamic WHERE/CTE builder binds every filter
  value via `?N` placeholders through `owned_params`/`param_refs` — no filter value is ever
  interpolated into the SQL string; only static, hardcoded SQL fragments are conditionally
  concatenated. CSV export also runs every cell through the existing `csv_safe` formula-injection
  guard.
- **Report filter semantics (D-24):** the two subtree CTEs (`from_subtree`/`to_subtree`) are both
  genuinely `WITH RECURSIVE` descendant walks, and their `WHERE` clauses are joined with `AND`
  (never `OR`) — confirmed by `report_movements_place_filters`, which seeds three movements and
  asserts exactly the one row satisfying both filters simultaneously survives.
- **Svelte 5 rune usage:** `MovementTimeline.svelte` is a pure, prop-driven presentational
  component with no local derived/effect entanglement. The four consumer `$effect`s
  (`PlaceEntityViewModal`, `CartridgeDetail`, `PrinterDetail`) each read a single prop
  (`row`/`cartridge`/`printer`) and write to disjoint `$state` variables they don't also read —
  no effect reads state it also writes, no `$derived` misused where an effect belongs, and the one
  cancellable async effect (`PlaceContents.svelte`'s bulk-move count fetch) correctly guards against
  a stale response via its `cancelled` closure variable.
