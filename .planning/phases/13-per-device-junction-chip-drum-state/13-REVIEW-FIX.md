---
phase: 13-per-device-junction-chip-drum-state
fixed_at: 2026-06-26T12:00:00Z
review_path: .planning/phases/13-per-device-junction-chip-drum-state/13-REVIEW.md
iteration: 1
findings_in_scope: 12
fixed: 10
skipped: 2
status: partial
---

# Phase 13: Code Review Fix Report

**Fixed at:** 2026-06-26
**Source review:** .planning/phases/13-per-device-junction-chip-drum-state/13-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 12 (fix_scope = all — includes Info)
- Fixed: 10
- Skipped: 2

All fixes were applied in an isolated git worktree on a temporary branch and
fast-forwarded back onto `main`. Backend changes verified with
`cargo build --workspace`, targeted `cargo test` suites (one at a time), and
`cargo clippy` (lib-scoped, clean). Frontend changes verified with
`pnpm --dir ui exec svelte-check` (0 errors) and `pnpm --dir ui build`.

The two pre-existing environmental test failures (`restore_request_visibility_http.rs`,
`settings_ad.rs`, both `503 ... ad` from an unreachable AD on dev macOS) were NOT
touched and are unrelated to these fixes. Pre-existing clippy issues in
`template_service.rs` / `backup_service.rs` (files outside this change set) were
likewise left untouched.

## Fixed Issues

### CR-01: V032 migration drops compatibility data when concatenated printer name collapses to empty

**Files modified:** `migrations/V032__cartridge_model_compatibility_printer_name.sql`, `crates/trackly-infra/src/db/migrations.rs`
**Commit:** 9213ab1
**Applied fix:** Added `WHERE TRIM(printer_brand || ' ' || printer_model) <> ''`
to the V005→V032 data-transform INSERT so empty/whitespace-only legacy rows are
dropped rather than migrated to a functionally-dead empty `printer_name` (which
would silently suppress the D-05 "no compatibility => compatible with any
printer" pass-through). Added a self-contained migration data-transform test
(`v032_data_transform_drops_empty_and_preserves_populated`) that stands up the
V005-shaped tables, seeds populated + brand-only + empty rows, runs the real
embedded V032 SQL, and asserts populated rows survive with concatenated names
while the empty row is dropped (model ends with zero rows). This also closes
IN-05 (data-transform coverage gap).

### WR-01: Aggregate + filter queries match `devices.name` without scoping to printer-type / non-deleted devices

**Files modified:** `crates/trackly-infra/src/repos/cartridges_sqlite.rs`, `crates/trackly-core/src/domain/cartridges.rs`
**Commit:** 81a58f9
**Applied fix:** Added `AND d.type_id = 2 AND d.deleted_at_utc IS NULL` to the
device join in both the `compatible_model_aggregates` EXISTS subquery and the
two `list()` compatibility EXISTS subqueries (COUNT and SELECT). A non-printer
or soft-deleted device id whose name collides with a compatibility entry can no
longer produce a false-positive match. (Committed together with IN-01 and WR-03
doc since they touch the same query/method region.)

### WR-02: `build_printers_get_compatible_aggregates` performs no existence/type check on `device_id`

**Files modified:** `crates/trackly-app/src/tauri_cmds/printers.rs`
**Commit:** c30b3da
**Applied fix:** The command now calls `ctx.printers.get_by_device_id(device_id).await?`
first to assert the device exists and is a printer (returns `NotFound` otherwise),
before computing aggregates — matching every other printer read path. A bogus
device_id (e.g. a printers.id passed where a device_id is expected) now surfaces
as 404 instead of a silent HTTP 200 with an empty `models` list.

### WR-03: `compatible_model_aggregates` ignores `installable_only`/state semantics

**Files modified:** `crates/trackly-infra/src/repos/cartridges_sqlite.rs`, `crates/trackly-core/src/domain/cartridges.rs`
**Commit:** 81a58f9
**Applied fix:** Chose the documentation option (the reviewer's first option),
because the `in_stock` figure backs the literal "На складе" UI label, which
reflects storage status (status_id=1) regardless of `state_id` — making the
count "installable-only" would make that label inaccurate. Documented on both
`CompatibleModelAggregate` (domain) and the repository method that these are RAW
status counts, NOT installable counts, with the explicit rationale that a
status=1 state=6 (Отработанный) drum is counted even though it cannot be
installed.

### WR-04: `PrinterDto::community_configured` hardcoded `true` regardless of actual community state

**Files modified:** `crates/trackly-core/src/domain/printers.rs`, `crates/trackly-infra/src/repos/printers_sqlite.rs`, `crates/trackly-app/src/dto/printer.rs`
**Commit:** 6e8bbae
**Applied fix:** Added a `community_configured: bool` field to `PrinterRow`,
derived in the `SELECT_PRINTERS` query as `(p.community <> 'public')` (so the raw
secret community value never leaves the repository — only the safe boolean is
carried), mapped at row index 13, and propagated through `From<PrinterRow>` for
`PrinterDto` in place of the hardcoded `true`. The DTO contract (a boolean
indicating a non-default community was set) is now honoured.

### WR-05: Stale-write race in `OperationModal` async compatibility effects

**Files modified:** `ui/src/features/cartridges/OperationModal.svelte`
**Commit:** 214ac82
**Applied fix:** Added a `let cancelled = false;` token + `return () => { cancelled = true; };`
cleanup to all three async `$effect`s (the printer-context `getByDeviceId` chain,
the `getCompatibleAggregates` flag effect, and the `Promise.all([printers.list,
cartridges.modelsGet])` effect), guarding every `.then`/`.catch` assignment with
`if (cancelled) return;`. Late-resolving promises for a printer/cartridge the
operator already changed away from can no longer overwrite current state.

### WR-06: `compatibility` prop mutated as a fresh array reference but `CompatibilityEditor` only reads it once

**Files modified:** `ui/src/features/cartridges/CompatibilityEditor.svelte`
**Commit:** ca79580
**Applied fix:** Took the cheapest correct option — documented the mount-time-only
prop contract explicitly: `rows` is seeded from `compatibility` once at mount and
never re-synced; data flows out via `onChange`; resets are the caller's
responsibility via the existing `{#key openInstanceCounter}` remount in
`ModelFormModal` (verified present). Added a warning that any future change which
resets `compatibility` without bumping `openInstanceCounter` must preserve the
remount. (The svelte-check `state_referenced_locally` warning on this line is the
expected, intentional signal of this documented design — not a regression.)

### IN-01: `compatible_model_aggregates` cross-joins `devices` even though only `d.name` is used

**Files modified:** `crates/trackly-infra/src/repos/cartridges_sqlite.rs`
**Commit:** 81a58f9
**Applied fix:** Removed the outer `JOIN devices d ON d.id = ?1` and folded the
device-name lookup entirely into the `EXISTS` subquery (`SELECT 1 FROM
cartridge_model_compatibility cmc JOIN devices d ON d.id = ?1 AND ... WHERE ...`),
mirroring the `list()` query shape. Resolved together with WR-01 (the scope guard
lives in the same subquery).

### IN-03: Comment in domain enum is stale — `ReturnToStock` default documented as flat "3=Пустой"

**Files modified:** `crates/trackly-core/src/domain/cartridges.rs`
**Commit:** ffce59a
**Applied fix:** Updated the `Install.previous_cartridge_state_id` doc to state
the kind-aware default applied at the repository layer: 3 (Пустой) for cartridges
(kind_id=1), 5 (Изношенный) for drums (kind_id=2) — verified state id 5 = "Изношенный"
against V017 seed data (the reviewer's "Под замену" label was corrected to the
canonical name).

### IN-05: Data-transform path of V032 has no test coverage

**Files modified:** `crates/trackly-infra/src/db/migrations.rs`
**Commit:** 9213ab1
**Applied fix:** Covered by the migration data-transform test added under CR-01
(`v032_data_transform_drops_empty_and_preserves_populated`), which exercises the
V005→V032 transform on pre-existing rows end-to-end via the embedded V032 SQL.

## Skipped Issues

### IN-02: `current_cartridge_for_printer` picks arbitrarily among multiple linked cartridges

**File:** `crates/trackly-infra/src/repos/printers_sqlite.rs:431-441`
**Reason:** skipped — out of scope for a mechanical review-fix; requires human
decision. The reviewer's recommendation is phrased as "Consider a partial unique
index" (Info-level, explicitly "not a Phase 13 regression"). Enforcing the
invariant requires a NEW migration (`CREATE UNIQUE INDEX ... WHERE
current_printer_device_id IS NOT NULL AND deleted_at_utc IS NULL AND status_id =
2`). On the portable production-upgrade path this index creation would FAIL if any
existing DB already holds a duplicate, breaking the upgrade — so it needs a
data-audit / backfill decision and product sign-off before shipping, not a blind
application.
**Original issue:** If an invariant break ever leaves two cartridges pointing at
the same printer, the query silently returns the most-recently-updated one; a
partial unique index would make the invariant structural.

### IN-04: `printers/api.ts` references endpoints that are not in `specta_export.rs`

**File:** `ui/src/features/printers/api.ts:32,44`
**Reason:** skipped — confirmed the wiring inconsistency is real
(`printers_get_readings` and `printers_delete` are defined nowhere in the
backend: not in `specta_export.rs`, not as Tauri command fns, not in the HTTP
router), and `PrinterDetail.svelte:40` actively calls `printers.getReadings(p.id)`.
But the two valid resolutions — (a) implement the missing `printers_get_readings`
backend command (service + repo query over `printer_readings` + DTO + dual-transport
registration) or (b) remove the readings widget and its dead call — are both
beyond a mechanical fix: (a) is net-new feature work and (b) deletes user-facing
functionality. Either needs a product decision, so this is flagged for human
follow-up rather than guessed at. (The `.catch(() => { readings = []; })` on the
call means today it degrades to an empty readings list rather than crashing.)
**Original issue:** `printers.getReadings`/`printers.delete` invoke commands that
are not registered; a missing command surfaces as a runtime invoke error, not a
compile error.

---

_Fixed: 2026-06-26_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
