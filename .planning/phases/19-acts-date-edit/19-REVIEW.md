---
phase: 19-acts-date-edit
reviewed: 2026-07-12T00:00:00Z
depth: standard
files_reviewed: 21
files_reviewed_list:
  - crates/trackly-app/src/dto/act.rs
  - crates/trackly-app/src/http/acts.rs
  - crates/trackly-app/src/services/act_service.rs
  - crates/trackly-app/src/specta_export.rs
  - crates/trackly-app/src/tauri_cmds/acts.rs
  - crates/trackly-app/tests/acts_date_source.rs
  - crates/trackly-app/tests/acts_update.rs
  - crates/trackly-app/tests/export_bindings.rs
  - crates/trackly-app/tests/html_act_render.rs
  - crates/trackly-app/tests/role_endpoint_matrix.rs
  - crates/trackly-core/src/domain/acts.rs
  - crates/trackly-infra/src/repos/acts_sqlite.rs
  - crates/trackly-infra/src/repos/audit_log_sqlite.rs
  - ui/src/bindings.ts
  - ui/src/features/acts/ActDetail.svelte
  - ui/src/features/acts/ActFormBody.svelte
  - ui/src/features/acts/ActFormItemsTable.svelte
  - ui/src/features/acts/ActFormModal.svelte
  - ui/src/features/acts/ActListRow.svelte
  - ui/src/features/acts/ActsPage.svelte
  - ui/src/lib/api/acts.ts
findings:
  critical: 1
  warning: 3
  info: 1
  total: 5
status: issues_found
---

# Phase 19: Code Review Report

**Reviewed:** 2026-07-12
**Depth:** standard
**Files Reviewed:** 21
**Status:** issues_found

## Summary

Phase 19 adds act editing (`ActService::update`, ACT-02) plus the read-side
handover-date sort fix (ACT-01). The RBAC gating, CAS optimistic-lock,
validate-then-mutate ordering, and the Pitfall-2 "restore most-recent snapshot"
logic are all sound and well-tested. The DTO/bindings/specta registration are
consistent across both transports.

The critical defect is that `ActService::update` mutates the act's device set
(add / remove positions) but **never recomputes the derived `archived` flag** —
unlike `create`, `do_return`, and `delete_soft`, all of which call
`recompute_parent_archived`. This lets an act's `archived` state diverge from
its actual outstanding-device count, which the current test suite does not
exercise. Three lower-severity issues follow: stale return-act numbers on
rename, a misleading edit-mode quantity/group picker, and an audit-trail gap on
retained-item комплектация edits.

## Critical Issues

### CR-01: `ActService::update` never recomputes `archived` after add/remove — leaves devices stuck `в_работе` and acts mis-categorized

**File:** `crates/trackly-app/src/services/act_service.rs:578-954`
**Issue:**
`update()` transitions devices `на_складе → в_работе` when a position is added
(step 6) and restores devices when a position is removed (step 8c), but the
function never calls `recompute_parent_archived`. Every other mutation path that
changes the handover↔return device balance does recompute it (`do_return`
`act_service.rs:1333`, `delete_soft` `act_service.rs:1750`). Two concrete,
UI-reachable failure modes:

1. **Editing an archived (fully-returned) act and adding a device.** The Edit
   button in `ActDetail.svelte:70` is enabled for *any* handover, including
   archived ones (`act.act_type === 'handover'`, no `archived` check). Adding a
   new position flips that device to `в_работе`, but `acts.archived` stays
   `true`. The Return button is disabled for archived acts
   (`ActDetail.svelte:83` — `!act.archived`), so the newly-added device can
   never be returned through the UI. The device is stranded `в_работе` with no
   return path short of deleting the whole act.

2. **Removing the last outstanding device from a non-archived act.** Handover
   with 2 devices, one already returned (not archived). Removing the remaining
   outstanding device (allowed — it is outstanding) leaves `act_items` holding
   only the already-returned device, so `handover_total (1) <= returned_total
   (1)` — the act *should* archive but stays `archived = false`, showing in the
   active «Акты» tab with zero outstanding devices. Switch-bar counts
   (`counts()` filters on `archived`) drift accordingly.

**Fix:** After the item add/remove loops and before the header UPDATE (or right
after it), recompute the flag in the same transaction:
```rust
// After step 8c (removed loop) — the item set is now final.
trackly_infra::repos::acts_sqlite::recompute_parent_archived(&tx, payload.id, now)?;
```
Note `recompute_parent_archived` also bumps `version`; account for that so the
CAS header UPDATE and the returned `ActDto` version stay consistent (call it
before `update_act_header_in_tx`, or re-fetch version for the final audit row).
Add a regression test: create a 2-device handover, return one device, remove the
other via `update`, assert the act is now `archived`.

## Warnings

### WR-01: Renaming a handover that has return acts leaves the returns' `number` column stale, permanently reserving the old number

**File:** `crates/trackly-app/src/services/act_service.rs:792-809,868-878`
**Issue:**
`update_act_header_in_tx` (`acts_sqlite.rs:377`) updates only the target act
row (`WHERE id = ?`). Return acts store a *copy* of the parent's number
(`do_return` sets `number: parent.number`, `act_service.rs:1144`). After
renaming handover 42 → 50, its return rows still hold `number = 42`. Display is
unaffected (the «в»/«в1» rule reads `parent_number` via JOIN, not the stored
column), but the number-uniqueness check
(`SELECT EXISTS(... WHERE number=?1 ...)`, `act_service.rs:796`) counts those
orphaned return rows, so 42 can never be reused by a future act — a silent,
permanent number leak. It also makes the return row's own `number`/`number_raw`
internally inconsistent with its parent.
**Fix:** When the number changes and returns exist, cascade the new number to
child return acts inside the same transaction, e.g.
`UPDATE acts SET number = ?1 WHERE parent_act_id = ?2 AND deleted_at_utc IS NULL`,
or exclude `act_type = 'return'` rows from the uniqueness check so a rename does
not strand the old number.

### WR-02: Edit-mode form shows a quantity / group picker but silently submits only one device per row

**File:** `ui/src/features/acts/ActFormBody.svelte:150-156`, `ui/src/features/acts/ActFormItemsTable.svelte:680-694`
**Issue:**
In edit mode the items table renders the same on-warehouse group picker and
quantity input as create mode (no `mode` gating on the picker or the qty
`<input>`). A user adding a new position can pick a group (`×5`) and set
quantity 5. But `ActUpdateItemDto` carries only `{ device_id,
complectation_at_time }` — no `quantity`/`device_ids` — and the edit-mode submit
maps every row to just `device_id` (`ActFormBody.svelte:151-155`). The other 4
devices of the group are silently dropped; only one device is added. The user
sees «×5» / quantity 5 but the act gains a single device with no error.
**Fix:** In edit mode either (a) hide/disable the quantity column and force
single-device picks for added rows, or (b) expand grouped picks into N separate
`ActUpdateItemDto` rows before submit so the visible quantity matches what is
persisted.

### WR-03: Retained-item комплектация edits are not written to `audit_log`

**File:** `crates/trackly-app/src/services/act_service.rs:756-770`
**Issue:**
Step 7 overwrites `act_items.complectation_at_time` for retained device rows
(D-04) with a bare `UPDATE`, but records no `audit_log` entry for that change.
The final act-level audit row (step 10) diffs only header fields
(`giver_name`, `receiver_name`, `location_id`, `notes`, `deadline_utc`,
`handover_date_utc`, `number`, `version`) — item-level комплектация changes are
invisible in the trail. Given the app's emphasis on a full audit history for
acts, an edited комплектация value cannot be traced to who/when.
**Fix:** Emit an audit row (e.g. `action = "custom:act_item_complectation_edit"`
with `before_json`/`after_json` of the affected `act_items` row) whenever a
retained row's комплектация actually changes, mirroring the device-mutation
audit pattern used elsewhere in `update`.

## Info

### IN-01: Handover-date default uses local calendar day while edit-prefill uses UTC

**File:** `ui/src/features/acts/ActFormBody.svelte:41-58,117-121`
**Issue:**
`todayISO()` (create-mode default) derives the date from local `getFullYear/
getMonth/getDate`, whereas `unixToIso()` (edit prefill) and `isoToUnix()`
(submit) use UTC. For the RU/MSK (+3) target this only shifts the date-only
value at day boundaries and is acknowledged in the comment, but the two paths
being inconsistent invites confusion and a future off-by-one when the app is
run in other offsets.
**Fix:** Make `todayISO()` use UTC accessors (`getUTCFullYear/getUTCMonth/
getUTCDate`) so create and edit share one timezone convention with the
`isoToUnix` round-trip.

---

_Reviewed: 2026-07-12_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
