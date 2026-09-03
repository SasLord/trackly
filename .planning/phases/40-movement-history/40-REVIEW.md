---
phase: 40-movement-history
reviewed: 2026-09-03T00:00:00Z
depth: standard
files_reviewed: 20
files_reviewed_list:
  - crates/trackly-app/src/dto/device.rs
  - crates/trackly-app/src/services/device_service.rs
  - crates/trackly-app/src/services/place_movement_service.rs
  - crates/trackly-app/tests/cartridges_lifecycle.rs
  - crates/trackly-app/tests/devices_grouping.rs
  - crates/trackly-app/tests/place_movements_timeline.rs
  - crates/trackly-app/tests/place_movements_write_sites_devices.rs
  - crates/trackly-core/src/domain/devices.rs
  - crates/trackly-infra/src/repos/cartridges_sqlite.rs
  - crates/trackly-infra/src/repos/devices_sqlite.rs
  - ui/package.json
  - ui/scripts/check-print-idempotency.mjs
  - ui/scripts/check-report-type-parity.mjs
  - ui/src/features/acts/ActsPage.svelte
  - ui/src/features/acts/PdfPreviewModal.svelte
  - ui/src/features/cartridges/OperationModal.svelte
  - ui/src/features/devices/DeviceGroupRow.svelte
  - ui/src/features/reports/ReportsPage.svelte
  - ui/src/lib/components/MovementTimeline.svelte
findings:
  critical: 4
  warning: 14
  info: 5
  total: 23
status: issues_found
---

# Phase 40 (gap closure 40-21..40-27): Code Review Report

**Reviewed:** 2026-09-03
**Depth:** standard
**Files Reviewed:** 20 (diff base `9b64b04f`..`HEAD`)
**Status:** issues_found

## Summary

Reviewed the seven gap-closure plans of phase 40: the printer→cartridge place cascade
(40-21), the auto-return storage-place fallback (40-22), the optional-place install
(40-23), the timeline act-number/deep-link fix (40-24), the report-type parity fix
(40-25), the grouped-device place inversion fix (40-26), and the LAN print idempotency
fix (40-27), plus the two new structural gates.

The transactional composition asked about in the review brief is *mostly* sound: the
40-21 cascade does run inside the device's own transaction, the 40-21 backfill and the
40-22 fallback cannot double-write a `place_movements` row for the same entity, and the
`COUNT(DISTINCT COALESCE(d.place_id, -1))` column is identical and correctly positioned
in all three `list_grouped` SQL branches (the `-1` sentinel cannot collide because
`places.id` is `INTEGER PRIMARY KEY AUTOINCREMENT`).

What the review found instead:

- a **reader-pool deadlock** in the timeline read path (nested `acquire()` while holding
  a pooled connection, on a fixed-size pool that blocks on a `Condvar` with no timeout);
- the **40-22 fallback does not cover the scenario it was written for** — and its
  regression test hides that by seeding the DB state the real flow never produces;
- the **40-21 cascade silently wipes** every attached cartridge's place, unlogged, when
  a printer's place is cleared;
- `cargo fmt --check` is **red on this phase's own new test code**, which (per the
  project's own CI history) skips every later gate in the sequential `ci-fast` job;
- **both new structural gates can be satisfied by a comment** — proven by mutation, with
  the exact pre-fix defect shape passing all three print-idempotency invariants.

No secrets, injection vectors, or real organization/personal data were found in the
changed files; all test fixtures use invented names.

## Critical Issues

### CR-01: Reader-pool deadlock — nested `acquire()` while holding a pooled connection

**File:** `crates/trackly-app/src/services/place_movement_service.rs:64,145-154`
(and `crates/trackly-infra/src/db/pools.rs:76-95`, `crates/trackly-app/src/services/place_path_display.rs:48`)

**Issue:** `get_timeline` acquires a reader (`let conn = readers.acquire();`, line 64) and
holds it for the entire row loop. Inside that loop it calls
`compute_place_path_short(&readers, ...)` **twice per row** (lines 145 and 150), and that
function opens a *second* connection from the *same* pool (`place_path_display.rs:48`).

`ReaderPool::acquire()` **blocks on a `Condvar` with no timeout** when the pool is
exhausted (documented in `pools.rs` as "queue-on-exhaust"). Production pool size is 8
(`context.rs:196`). If 8 tasks each hold their outer connection and then each try to take
a second one, every one of them parks forever — a permanent, unrecoverable hang of all DB
reads, not a slow request. On a 20-user LAN deployment that is reachable.

Even short of a deadlock, a 20-row timeline performs 41 pool acquisitions instead of 1
and holds 2 of 8 connections for its whole duration, starving every other reader.

The same pattern exists in the movements *report* (`report_service.rs:1495-1559`, up to
`LIMIT 1000` rows → up to 2000 nested acquisitions per render) — same class, same pool.

**Fix:** never acquire from the pool while holding one. Either pass the already-held
connection down, or hoist the two settings reads out of the loop:

```rust
// place_movement_service.rs — inside spawn_blocking, ONE connection total
let conn = readers.acquire();
let rows = repo.get_history(&conn, &entity_type, entity_id)?;
// read variant/separator settings ONCE from the connection we already hold
let (sep_ends, sep_last_two) = read_path_display_separators(&conn);
let org_default = read_org_default_variant_token(&conn);
for row in rows {
    // resolve effective_variant via `conn`, then call the pure
    // `shorten_place_path(...)` — no second `readers.acquire()` anywhere
}
```

i.e. add a `&Connection`-taking sibling of `compute_place_path_short` in
`place_path_display.rs` (keeping the formula single-owner) and make the `&ReaderPool`
variant a thin wrapper for callers that hold no connection.

---

### CR-02: 40-22 storage-place fallback misses the primary scenario; its test masks the miss

**File:** `crates/trackly-infra/src/repos/cartridges_sqlite.rs:928-953` (used at line 694)
**Test:** `crates/trackly-app/tests/cartridges_lifecycle.rs::install_auto_return_falls_back_to_last_known_storage_place`

**Issue:** `last_known_storage_place_in_tx` derives the fallback exclusively from
`place_movements.to_place_id` where `places.is_storage = 1`. But phase 40's own D-06 rule
(`is_reportable_place_change`, `place_movements.rs:100-102`) means a first assignment
`NULL -> place` **never produces a movement row**. Trace the normal life of a cartridge:

1. Cartridge is created at the warehouse (`place_id = S` set on INSERT) — D-06: no
   movement row.
2. It is installed into a printer at room `Q` — one movement row is written, `S -> Q`
   (`to_place_id = Q`, not a storage place).
3. Another cartridge is installed into the same printer → auto-return of ours with the
   place field left empty → `last_known_storage_place_in_tx` finds **no row whose
   `to_place_id` is a storage place** → returns `None` → `place_id` is set to `NULL`.

That is exactly the UAT defect "return-to-stock-empty-place-field" the plan claims to
close, and it still happens on the first (most common) install/auto-return cycle.

The new test does not catch this because it does not exercise the flow: it hand-writes a
`place_movements` row *into* the storage place via raw SQL
(`cartridges_lifecycle.rs`, "Imitate A having previously sat in the storage place") —
DB state the real code path never creates on its own. The test therefore passes while the
user-visible defect survives.

**Fix:** stop treating "arrived at a storage place" as the only evidence. Prefer, in
order: (1) the explicit override, (2) `from_place_id` of the movement that took the
cartridge *out* of a storage place, (3) the cartridge's own `place_id` if it is a storage
place — and only then `NULL`:

```sql
SELECT COALESCE(
  (SELECT pm.to_place_id FROM place_movements pm
     JOIN places p ON p.id = pm.to_place_id
    WHERE pm.entity_type='cartridge' AND pm.entity_id=?1
      AND p.is_storage=1 AND p.archived_at_utc IS NULL AND p.deleted_at_utc IS NULL
    ORDER BY pm.created_at_utc DESC, pm.id DESC LIMIT 1),
  (SELECT pm.from_place_id FROM place_movements pm
     JOIN places p ON p.id = pm.from_place_id
    WHERE pm.entity_type='cartridge' AND pm.entity_id=?1
      AND p.is_storage=1 AND p.archived_at_utc IS NULL AND p.deleted_at_utc IS NULL
    ORDER BY pm.created_at_utc DESC, pm.id DESC LIMIT 1)
)
```

and add a test that drives the whole flow through `CartridgeService` (create at storage →
install → install second → assert the auto-returned place) with **no hand-seeded
`place_movements` row**.

---

### CR-03: Clearing a printer's place silently wipes every attached cartridge's place, unlogged

**File:** `crates/trackly-app/src/services/device_service.rs:338-348`,
`crates/trackly-infra/src/repos/cartridges_sqlite.rs:956-1030`

**Issue:** the cascade fires on `before_place_id != after.place_id`, which includes
`Some(P) -> None`. `cascade_place_for_printer_in_tx` then runs
`UPDATE cartridges SET place_id = NULL, version = version + 1` for **every** attached
cartridge, and the paired `record_movement_if_applicable(old_place, None)` is skipped by
D-06 (`Some -> None` is not reportable). The cascade writes no `audit_log` row either.

Net result: an operator who clears one field on a printer destroys the recorded location
of every cartridge in it, with **no `place_movements` row, no `audit_log` row, and no
confirmation** — the information is unrecoverable from the app. The cartridge versions
are bumped too, so any open cartridge editor fails its next save with an optimistic-lock
error whose cause is invisible.

Neither of the two new cascade tests covers `Some -> None`
(`place_movements_write_sites_devices.rs` tests only `A -> B` and "no change").

**Fix:** do not cascade a clear — a printer with an unknown place says nothing about
where its cartridges are:

```rust
// device_service.rs
if before_place_id != after.place_id {
    if let Some(target) = after.place_id {
        cartridge_repo.cascade_place_for_printer_in_tx(
            &tx, id, Some(target), MovementSource::Manual,
            "вместе с принтером", user_id_opt, now,
        )?;
    }
    // Some(P) -> None: leave attached cartridges where they are.
}
```

and add a regression test asserting the cartridge keeps `place_a` and `version == 1` when
the printer's place is cleared.

---

### CR-04: `cargo fmt --check` is red on this phase's own new code — first CI gate fails

**File:** `crates/trackly-app/tests/place_movements_timeline.rs:394` (and trailing blank
line at EOF)

**Issue:** verified locally with the pinned toolchain (`rustfmt 1.8.0-stable`, `rustfmt.toml`
= defaults):

```
Diff in .../tests/place_movements_timeline.rs:394:
-async fn seed_return_act(writer: &WriterHandle, parent_act_id: i64, number: i64, sub_number: i64) -> i64 {
+async fn seed_return_act(
+    writer: &WriterHandle,
...
Diff in .../tests/place_movements_timeline.rs:485:   (trailing newline)
```

`cargo fmt` is a declared CI gate (`-D warnings` class), and this project's `ci-fast` is a
single sequential job in which the first red step **skips every later gate** — so a
formatting failure here silently disables the clippy/test/gate steps that would otherwise
protect the rest of this phase.

`crates/trackly-app/tests/cartridges_lifecycle.rs:763,780` is also unformatted; that drift
is pre-existing (blames to `feat(12-06)`), but commit `592268dc` claimed to "absorb
pre-existing cargo fmt drift in movement test files" while this phase edited that very
file and left it red.

**Fix:** run `cargo fmt --all` and commit; add the check to the plan's own self-check
step so it cannot be re-introduced.

## Warnings

### WR-01: `check-print-idempotency` INV-1 is anchored on a code comment — proven bypass

**File:** `ui/scripts/check-print-idempotency.mjs:81-101`

**Issue:** INV-1 splits the function body at `body.indexOf('await previewer.preview(')`.
That literal appears in a **comment** inside `printViaTopLevel`
(`PdfPreviewModal.svelte:~410`, "…the `await previewer.preview(...)` call immediately
below needs real geometry…") ~4000 characters before the real call, and `indexOf` finds
the comment first (measured: comment at body offset 1892, real clear at 6021).

Proven by mutation in a scratch copy: (1) delete the start-of-run clear, (2) put
`printRoot.innerHTML = ''` back inside the `afterprint` `cleanup` closure — i.e. exactly
the pre-fix defect shape — and (3) reword that one comment. Result:
`[check-print-idempotency] PASS — 0 нарушений`.

The gate's verdict therefore depends on comment prose, not on code behaviour.

**Fix:** anchor on the real call, and require the clear to be a top-level statement of the
function rather than merely textually earlier:

```js
// strip comments before scanning, and take the LAST occurrence of the call
const stripped = body.replace(/\/\*[\s\S]*?\*\//g, '').replace(/\/\/[^\n]*/g, '');
const previewIdx = stripped.lastIndexOf('await previewer.preview(');
const beforePreview = stripped.slice(0, previewIdx);
// and assert the clear is NOT inside a nested function/arrow body
```

---

### WR-02: `check-print-idempotency` INV-2 is satisfied by a comment — proven bypass

**File:** `ui/scripts/check-print-idempotency.mjs:104-137`

**Issue:** INV-2 checks only that the **last non-empty line before** the
`registerHandlers(RepeatTableHeadHandler)` call contains the substrings `if` and
`repeatTableHeadHandlerRegistered`. A comment satisfies both.

Proven by mutation: replacing the guard with

```js
// if repeatTableHeadHandlerRegistered — guard removed
registerHandlers(RepeatTableHeadHandler);
```

yields `PASS — 0 нарушений` with the guard fully deleted.

**Fix:** strip comments before the preceding-line lookup, and assert the call sits inside
an `if (…repeatTableHeadHandlerRegistered…) { … }` block by brace-matching, the same way
`functionBody()` already does.

---

### WR-03: `check-report-type-parity` INV-1 is satisfied by a comment — proven bypass

**File:** `ui/scripts/check-report-type-parity.mjs:170-197`

**Issue:** INV-1 rejects only the exact string `activeReport` (after `trim()`), then
accepts anything whose text `includes('reportTypeKey()')`. A comment defeats both checks.

Proven by mutation: changing the prop to
`reportType={activeReport /* was reportTypeKey() */}` — the literal D-25 regression —
yields `PASS — 0 нарушений`.

Two further holes in the same file: `reportTypePropExpression` scans only the **first**
`<ReportTable` occurrence (`src.match(/<ReportTable[\s>]/)`), and INV-2 only requires the
`showDeletedBadge` literal to appear *somewhere* in `reportTypeKey`'s body — comparing
against `'device_acts'` (a real return value, wrong semantics) would pass.

**Fix:** strip comments from the extracted expression before comparing; require the
trimmed expression to be exactly `reportTypeKey()`; iterate over all `<ReportTable`
occurrences.

---

### WR-04: Stale `afterprint` listener from the previous print run is never removed

**File:** `ui/src/features/acts/PdfPreviewModal.svelte:485-492` (registration),
`366-395` (start-of-run cleanup)

**Issue:** the start-of-run cleanup destroys the previous `activePolisher` and clears
`printRoot`, but the previous run's `cleanup` closure is still registered on
`window`'s `afterprint` (it only removes itself when it fires). `printing` is released in
`handlePrint`'s `finally`, i.e. as soon as `window.print()` returns — which in several
engines is *before* `afterprint` fires.

Sequence: run 1 prints → `printing = false` → user clicks again → run 2 starts and is
`await`ing `import('pagedjs')` / `previewer.preview(...)` → run 1's delayed `afterprint`
fires → `cleanup1` runs `printRoot.innerHTML = ''` and `activePolisher.destroy()` on
**run 2's** in-flight render → blank or half-paginated print.

**Fix:** track the listener at component scope and detach it at the start of every run:

```ts
let activeAfterPrint: (() => void) | null = null;
// …at the start of printViaTopLevel, next to the existing cleanup:
if (activeAfterPrint) window.removeEventListener('afterprint', activeAfterPrint);
activeAfterPrint = null;
// …after defining cleanup:
activeAfterPrint = cleanup;
window.addEventListener('afterprint', cleanup, { once: true });
```

Also consider releasing `printing` on `afterprint` rather than in `finally`.

---

### WR-05: `repeatTableHeadHandlerRegistered` is per-component-instance; pagedjs's registry is global

**File:** `ui/src/features/acts/PdfPreviewModal.svelte:292,519-531`

**Issue:** `registerHandlers` writes to pagedjs's **module-level** registry, which lives as
long as the dynamically imported module (i.e. the page session). The new flag is a
component-instance variable, so it resets whenever `PdfPreviewModal` is re-created — e.g.
navigating away from Акты and back — and there are two independent instances
(`ActsPage.svelte:377` and `ReportsPage.svelte:612`), each with its own flag. Registrations
therefore still accumulate across navigations and across the two instances.

The comment claiming "the handler class itself is still redeclared/reconstructed per call
(it needs a fresh `savedThead` snapshot each print)" is also misleading: after the first
registration the freshly declared class is never registered and is dead work; freshness is
provided by pagedjs constructing a new *instance* of the already-registered class per
chunker (its constructor re-queries the DOM), not by the redeclaration.

**Fix:** hoist the flag to module scope in a small `pagedjsHandlers.ts` helper (shared by
every component that prints), and drop the per-call class declaration in favour of a
module-level class.

---

### WR-06: Printer place backfill writes a device mutation with no movement row and no audit row

**File:** `crates/trackly-infra/src/repos/cartridges_sqlite.rs:600-650`

**Issue:** the 40-21 backfill (step 5a) updates `devices.place_id` and bumps
`devices.version`, then calls
`record_movement_if_applicable(..., None, Some(explicit), ..., Some("заполнено по месту установленного картриджа"), ...)`.
That call is **unconditionally dead**: `is_reportable_place_change(None, Some(_))` is
`false` by D-06 (`place_movements.rs:100-102`), so the note string can never reach the
database and the printer's timeline never shows the backfill.

The mutation is therefore completely untraceable: no `place_movements` row, and (unlike
`device_service::update`) no `audit_log` row either. The version bump will also break any
concurrently open printer editor with an optimistic-lock error nobody can explain, and
the UI explicitly promises this write-back to the operator (see WR-09).

**Fix:** either delete the dead call and add an `audit_log` entry for the backfill (with
`before_json`/`after_json` like the device update path), or, if this event should appear
in the timeline, call `insert_in_tx` directly with an explicit `from_place_id` semantics
decision — do **not** leave a call site that reads as if it logs something it never logs.

---

### WR-07: Auto-return fallback can place a cartridge into an archived place

**File:** `crates/trackly-infra/src/repos/cartridges_sqlite.rs:938-950`

**Issue:** `last_known_storage_place_in_tx` filters only on `p.is_storage = 1`. `places`
carries `archived_at_utc` (V037, D-15: archived places are hidden from `PlacePicker` —
`places_sqlite.rs:383` filters them out of `list_all`). The fallback can therefore assign
a cartridge to a place the operator can no longer select or see in the picker, silently.

**Fix:** add `AND p.archived_at_utc IS NULL AND p.deleted_at_utc IS NULL` to the query
(see the SQL in CR-02's fix, which already includes it).

---

### WR-08: "cartridge follows printer" holds only on the manual device-edit path

**File:** `crates/trackly-app/src/services/device_service.rs:338-348` (only call site)

**Issue:** `cascade_place_for_printer_in_tx` has exactly one caller. Every other write site
that moves a device's `place_id` skips it:

- `place_service.rs:722` (`move_subtree_contents`, D-28 bulk move)
- `act_service.rs:501,784,955,1533,2025,2112,2198` (handover/return/edit/undo)

So handing a printer over by act, or bulk-moving a room's contents, moves the printer and
leaves its cartridges behind — the exact UAT defect 40-21 set out to fix, on the paths
users are most likely to use for a real relocation. (`move_subtree_contents` accidentally
covers *some* cases because a co-located cartridge is itself in the subtree, but not a
cartridge whose `place_id` is `NULL`.)

**Fix:** call the cascade from every device-place write site, or move the cascade into a
shared `move_device_place_in_tx` helper that all four call sites go through.

---

### WR-09: Install hint claims a printer write-back that the server will refuse

**File:** `ui/src/features/cartridges/OperationModal.svelte:789-803` (hint),
`crates/trackly-infra/src/repos/cartridges_sqlite.rs:627-634` (`WHERE place_id IS NULL`)

**Issue:** the hint «Необязательно: у принтера пока не указано место. Если укажете здесь —
оно будет проставлено и принтеру» renders whenever
`effectivePrinterId !== undefined && placeId === null`. Two of those states are wrong:

1. **While the `printers.getByDeviceId` round-trip is in flight** (`OperationModal.svelte:230-296`)
   `placeId` is still `null` even for a printer that *does* have a place — the hint
   transiently asserts the opposite.
2. **After the operator clears or overrides the auto-filled place**, the hint reappears and
   promises the value will be written to the printer. It will not: the backfill runs only
   `if printer_place.is_none()`, guarded again by `WHERE ... AND place_id IS NULL`. The
   operator ends up with the cartridge at `Q` and the printer still at `P`, i.e. the
   40-21 invariant violated at the moment of install, with no reconciliation.

**Fix:** gate the hint on the resolved printer context rather than on `placeId`:

```svelte
{#if printerContext !== null && printerContext.devicePlaceId === null}
  <span class="field-hint">Необязательно: у принтера пока не указано место…</span>
{:else}
  <span class="field-hint">Укажите рабочее место или кабинет (не склад)</span>
{/if}
```

and decide explicitly what an override means when the printer already has a place (either
move the printer too, or warn about the divergence).

---

### WR-10: Movements report still renders return acts with the bare parent number

**File:** `crates/trackly-app/src/services/report_service.rs:1472,1505` (`a.number AS act_number`,
read as `Option<i64>`), vs. the fix in
`crates/trackly-app/src/services/place_movement_service.rs:104-140`

**Issue:** 40-24 correctly routed the **timeline** through `format_act_number`
(D-Numbering-01's single owner), but its sibling surface — the «Перемещения» report, which
this same phase also touched (40-25) — still selects `a.number` raw. A return act now
shows as `20в` in the timeline and `20` in the report/CSV/PDF, i.e. the phase created the
screen-vs-export divergence class it added a structural gate against for a different
column.

**Fix:** reuse the same query shape and `format_act_number` call in
`report_service.rs`'s movements SQL/mapping, or extract the shared "resolve display act
number for an `act_id`" helper so there is one owner rather than two copies.

---

### WR-11: `list_grouped`'s place-path subqueries ignore the FTS filter that defines the group

**File:** `crates/trackly-infra/src/repos/devices_sqlite.rs:1120-1170`
(`sql_grouped_by_model_with_query`)

**Issue:** in the FTS branch the outer query is filtered by `devices_fts MATCH ?4`, but the
two correlated subqueries that pick the joined path
(`LEFT JOIN place_full_paths pfp ON pfp.place_id = (SELECT MAX(d2.place_id) FROM devices d2 WHERE …)`)
are **not** — they scan every non-deleted device with the same `(type_id, name, model)`.
`MAX(d.place_id)` in the SELECT list *is* filtered. So `repr.place_id` and
`repr.full_path`/`place_path_short` can describe different places, and
`place_distinct_count` (also filtered) can be `1` while the displayed path belongs to a
device outside the group. That is precisely the value Фикс B relies on to decide whether
the displayed place is trustworthy.

**Fix:** add the same `MATCH`/status predicates to both correlated subqueries, or replace
the subquery join with a `LEFT JOIN place_full_paths pfp ON pfp.place_id = MAX(d.place_id)`
computed from the grouped set.

---

### WR-12: `list_grouped` silently ignores `place_id`, `state` and `include_deleted` filters

**File:** `crates/trackly-infra/src/repos/devices_sqlite.rs:1026-1250`

**Issue:** only `status_id` (`?1`), `limit`, `offset` and the optional `match_expr` are
bound; `filter.place_id`, `filter.state` and `filter.include_deleted` never reach any of the
three SQL branches (`deleted_at_utc IS NULL` is hardcoded). `DeviceService::list_grouped`
faithfully copies all three into `domain_filter` (`device_service.rs:532-540`), so a caller
has no way to know they are dropped. This is pre-existing, but phase 40 has just made the
grouped view place-aware, which makes "filter by place" the obvious next user expectation.

**Fix:** either bind the missing predicates, or narrow `DeviceFilter` at this call site so
the type makes the unsupported fields unrepresentable.

---

### WR-13: Place hard-delete pre-check does not count `place_movements` references

**File:** `crates/trackly-infra/src/repos/places_sqlite.rs:548-573` (`subtree_stats` gate),
`migrations/V040__place_movements.sql` (`from_place_id`/`to_place_id … ON DELETE RESTRICT`)

**Issue:** V040 added two `ON DELETE RESTRICT` FKs from `place_movements` to `places`, but
the D-14 pre-check counts only nested places, devices, cartridges and referencing acts. A
place that is now empty but has movement history passes the pre-check and then fails on the
raw `DELETE`, leaking an English SQLite FK error into the Russian-only «Удалить место?»
dialog — the identical defect already fixed as CR-01 for acts (see the comment at
`places_sqlite.rs:552-557`). Phase 40's own `last_known_storage_place_in_tx` increases the
number of places carrying movement history.

**Fix:** add a `referencing_movement_count` to `SubtreeStats` and include it in both the
service pre-check and `build_delete_blocked_message`.

---

### WR-14: `check-print-idempotency` INV-3 regex breaks on any parenthesised condition

**File:** `ui/scripts/check-print-idempotency.mjs:150`

**Issue:** `body.match(/if\s*\(([^)]*)\)\s*return;/)` cannot match a condition containing
parentheses, and only inspects the **first** `if (…) return;` in `handlePrint`. A
legitimate refactor such as
`if (!ready || (htmlContent === null && !isReport) || printing) return;` makes the gate
report a false violation and fail the build; conversely a meaningless
`if (printing === undefined) return;` satisfies it.

**Fix:** balance parentheses the way `functionBody` balances braces, scan **all** early
returns in the function, and additionally require `printing = true;` to be assigned before
the first `await` in the same body.

## Info

### IN-01: Timeline act-number SQL collapsed onto one line

**File:** `crates/trackly-app/src/services/place_movement_service.rs:106`
**Issue:** the query is a single string literal with long runs of spaces where the newlines
used to be, so it no longer visually mirrors `SELECT_ACTS` (`acts_sqlite.rs:34-49`) that the
comment above it says it must match, and any future diff of the two is unreadable.
**Fix:** restore it as a multi-line `\`-continued literal (or a shared `const`) in the same
shape as `SELECT_ACTS`.

### IN-02: Duplicate `use DeviceRepository` inside `export_csv`

**File:** `crates/trackly-app/src/services/device_service.rs:30,906`
**Issue:** `trackly_core::ports::devices::DeviceRepository` is imported at module scope and
re-imported inside `export_csv`.
**Fix:** drop the inner `use`.

### IN-03: `COALESCE(d.place_id, -1)` sentinel is undocumented

**File:** `crates/trackly-infra/src/repos/devices_sqlite.rs:1088,1133,1180`
**Issue:** the sibling `COALESCE(d.condition, ' ')` sentinel has a WR-04 comment proving it
cannot collide with a real value; the new `-1` has none. (It is in fact safe —
`places.id` is `INTEGER PRIMARY KEY AUTOINCREMENT` — but that invariant is unwritten.)
**Fix:** one comment line stating why `-1` cannot be a real `place_id`.

### IN-04: `last_known_storage_place_in_tx` takes `&self` it never uses

**File:** `crates/trackly-infra/src/repos/cartridges_sqlite.rs:928-932`
**Issue:** the repository is a zero-sized unit struct and the method ignores `self`;
`model_kind_in_tx`/`assign_code_in_tx` in the same file are associated functions.
**Fix:** make it an associated `fn` for consistency.

### IN-05: `MovementTimeline` duplicates a paragraph and reuses an "empty" class for a non-empty footer

**File:** `ui/src/lib/components/MovementTimeline.svelte:87-90,134-137`
**Issue:** the same two-line note is pasted into both the empty and non-empty branches, and
in the non-empty branch it is rendered with `class="timeline-empty-body"` outside the
`.timeline-empty` container. The rule has `margin: 0`, so the footnote sits flush against
the last timeline row (whose `border-bottom` is removed by `:last-child`).
**Fix:** hoist the text to a single `const` (or a small snippet) and give the footer variant
its own class with a top margin.

---

_Reviewed: 2026-09-03_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
