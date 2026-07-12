---
phase: 22-return-act-edit
reviewed: 2026-07-12T23:40:24Z
depth: standard
files_reviewed: 19
files_reviewed_list:
  - crates/trackly-app/src/dto/act.rs
  - crates/trackly-app/src/http/acts.rs
  - crates/trackly-app/src/services/act_service.rs
  - crates/trackly-app/src/specta_export.rs
  - crates/trackly-app/src/tauri_cmds/acts.rs
  - crates/trackly-app/tests/acts_archived_at.rs
  - crates/trackly-app/tests/acts_date_source.rs
  - crates/trackly-app/tests/acts_returns.rs
  - crates/trackly-app/tests/acts_update_return.rs
  - crates/trackly-app/tests/export_bindings.rs
  - crates/trackly-app/tests/html_act_render.rs
  - crates/trackly-app/tests/role_endpoint_matrix.rs
  - crates/trackly-infra/src/repos/audit_log_sqlite.rs
  - migrations/V034__return_handover_date_backfill.sql
  - ui/src/features/acts/ActDetail.svelte
  - ui/src/features/acts/ActsPage.svelte
  - ui/src/features/acts/ReturnModal.svelte
  - ui/src/lib/api/acts.ts
findings:
  critical: 2
  warning: 4
  info: 1
  total: 7
status: issues_found
---

# Phase 22: Code Review Report

**Reviewed:** 2026-07-12T23:40:24Z
**Depth:** standard
**Files Reviewed:** 19
**Status:** issues_found

## Summary

Phase 22 adds `ActService::update_return()` (delta reconciliation for editing an
existing return act), its Tauri + axum transports, DTOs, a `select_latest_device_mutation_pair`
audit helper, the D-07 compute-on-read «Дата архивации», the V034 backfill
migration, and the `ReturnModal` edit-mode UI.

The transport/RBAC wiring is solid: `build_acts_update_return` gates on
`Action::MutateActs` (same as the sibling act mutations), the HTTP handler runs
`session_identity` first, the command is registered in `specta_export.rs`, and
`role_endpoint_matrix.rs` Case 43 proves an Employee gets 403. CAS, the D-10
empty-set reject, and the D-11 drift guard are all wired and tested.

However, two correctness defects in the delta engine survive the test suite
because every test always supplies a bulk location and never chains an edit
followed by an un-return:

1. **CR-01** — editing a return with `apply_to_all` + an empty bulk location
   NULLs the stored warehouse location of every retained device (data loss;
   contradicts the UI's own stated intent).
2. **CR-02** — un-returning a device that was previously condition/location-edited
   restores it to the wrong (post-return) snapshot, leaving it `на_складе` while
   the parent handover simultaneously treats it as outstanding/in-work.

Both are reachable through the shipped `ReturnModal`. Details and fixes below.

## Critical Issues

### CR-01: Editing a return with an empty bulk location wipes retained devices' warehouse location

**File:** `crates/trackly-app/src/services/act_service.rs:1862-1906` (step 11, `retained_with_change` loop); enabled by `:1614-1620` (`effective_location` resolution) and `:1466-1477` (`validate_update_return`)

**Issue:**
For a `retained` device whose condition changed but whose location the user did
not touch, `effective_location` resolves to `None` when `apply_to_all == true`
and the bulk location field is empty:

```rust
let effective_location: Option<i64> = per_row_loc_id.or({
    if payload.apply_to_all { resolved_bulk_location_id } else { None }
});
```

`location_changed` is then `false` (`eff_location.map(...).unwrap_or(false)`),
so the device is still added to `retained_with_change` on the strength of the
condition change alone. Step 11 then calls:

```rust
let after = devices_repo.update_full_in_tx(
    &tx, dev_id, on_warehouse_status_id, location /* = None */, condition.as_deref(), now,
)?;
```

Per the DEF-3 contract documented in `do_return` (`act_service.rs:1289-1291`),
`update_full_in_tx` with `location = None` writes `location_id = NULL`. So a
condition-only edit **erases the device's existing shelf location.**

This is reachable through the shipped UI: `ReturnModal.canSubmit` only requires a
non-empty `bulkCondition` when `applyToAll` is true (`ReturnModal.svelte:200-202`)
— the bulk location may be left blank — and the modal's own comment
(`ReturnModal.svelte:187-190`) explicitly claims "location может остаться на
текущем расположении", which the backend violates. No test catches it because
`update_return_dto_from` always passes `bulk_location_id: Some(location_id)`
(`acts_update_return.rs:215`).

**Fix:**
When the user is only changing the condition, do not overwrite the device's
location with `NULL`. Resolve the effective location against the device's CURRENT
`location_id` when no new location was supplied, e.g. inside the retained loop:

```rust
// step 11: only override location if the payload actually carried one;
// otherwise preserve the device's current location.
let before = devices_repo.get_in_tx(&tx, dev_id)?;
let (_, condition, location_opt) = effective_by_device.get(&dev_id).cloned().unwrap_or((1, None, None));
let effective_location = location_opt.or(before.location_id); // preserve, don't NULL
let after = devices_repo.update_full_in_tx(
    &tx, dev_id, on_warehouse_status_id, effective_location, condition.as_deref(), now,
)?;
```

(Consider the same preservation semantics for the `added` path if a return
without a target location should keep the device's current shelf.)

### CR-02: Un-returning a previously-edited device restores the wrong snapshot (device stuck `на_складе` while counted as outstanding)

**File:** `crates/trackly-app/src/services/act_service.rs:1757-1803` (step 9 un-return) using `crates/trackly-infra/src/repos/audit_log_sqlite.rs:104-122` (`select_latest_device_mutation`, `ORDER BY ... DESC LIMIT 1`); caused by the step-11 device audit rows at `act_service.rs:1884-1899`

**Issue:**
The un-return path restores a removed device from
`select_latest_device_mutation(payload.id, removed_id)`, which returns the
`before_json` of the **most recent** device audit row for this return
(`DESC LIMIT 1`). But `update_return`'s retained condition/location edit (step 11)
ALSO writes a `device`/`action='update'` audit row tagged with the same
`act_id = payload.id` (`:1884-1899`). Its `before_json` captures a
**post-return `на_складе`** snapshot, not the pre-return `в_работе` state.

Reproduction (two separate `update_return` calls, both reachable from the edit
modal):

1. `do_return(D)` → audit row A: `before = {status: в_работе, loc_a, Новое}`.
2. `update_return` changes D's condition → step 11 audit row B:
   `before = {status: на_складе, loc_b, Хорошее}`.
3. `update_return` unchecks D (removed) → step 9 reads the **newest** row (B) and
   restores D to `{на_складе, loc_b, Хорошее}` instead of `{в_работе, loc_a, Новое}`.

The step-8b D-11 guard does not catch this: it compares the device's current
state against row B's `after_json`, which matches, so it reports "safe". After
step 9 deletes the `act_items` row and step 13 recomputes the parent, device D is
`на_складе` yet the parent handover's `outstanding` set counts it as in-work.
Net result: a device that is physically "returned to warehouse" in its row but
"still handed out" per the act graph — a corrupt, unrecoverable-without-audit
state.

Note the analogous handover `update()` path is NOT affected, because its retained
edit only touches `act_items` (no device audit row), so the removed-restore
always finds the original handover mutation.

**Fix:**
The un-return restore must target the return's **original** device mutation
(the `do_return` row whose `after_json.status_id == на_складе` and
`before_json.status_id == в_работе`), not the latest same-act edit. Options:
- Filter `select_latest_device_mutation` for the un-return case to the row whose
  `before_json` status is `в_работе` (the true pre-return snapshot), or
- Tag the step-11 retained-edit audit rows with a distinct `action`
  (e.g. `custom:return_item_edit`) and exclude that action from the un-return
  lookup, or
- Persist the original pre-return snapshot on the return's `act_item` at
  `do_return` time and restore from that instead of walking the audit log.

Add a regression test that chains: `do_return` → `update_return` (edit condition)
→ `update_return` (remove same device), asserting the device returns to
`status_id = 2 (в_работе)` at its pre-return location.

## Warnings

### WR-01: `validate_update_return` has no parity with `validate_return` — allows malformed payloads into the mutation engine

**File:** `crates/trackly-app/src/services/act_service.rs:1466-1477`

**Issue:**
`validate_update_return` only checks that `items` is non-empty. `validate_return`
(`:1033-1098`) additionally enforces: per-item `condition_override` +
`location_*_override` required when `apply_to_all == false`; intra-payload
`device_id` dedup; and non-empty `device_ids` per item. None of these run for the
edit path. Consequences:
- The missing `apply_to_all == false` override requirement is part of what makes
  CR-01 reachable server-side (a client can submit a condition change with no
  location).
- Duplicate `device_id`s across two items silently collapse in
  `effective_by_device.insert(...)` (`:1622-1626`) — last-write-wins, no error.
- The `added` path (`:1808-1854`) omits the `already_returned + qty <= handover_qty`
  bound that `do_return` enforces (`:1338-1347`), so a re-issued device could be
  double-covered by two return rows.

**Fix:** Mirror `validate_return`'s checks in `validate_update_return` (dedup,
device_ids-non-empty, and the per-item override requirement when
`apply_to_all == false`), and replicate the quantity/already-returned bound in the
`added` loop.

### WR-02: `.expect()` on `parent_act_id` panics inside the single-writer worker thread

**File:** `crates/trackly-app/src/services/act_service.rs:1541-1543`

**Issue:**
```rust
let parent_act_id = act.parent_act_id.expect("return act always has parent_act_id");
```
runs inside the `writer.execute(move |conn| ...)` closure. If a return row ever
has a NULL `parent_act_id` (data corruption, a bad import, a future migration
bug), this panics on the dedicated single-writer task. Per the project's
single-writer architecture (CLAUDE.md), that task owns the only write connection
— a panic there can poison/tear down the entire write path for the process, not
just this request.

**Fix:** Return a domain error instead of panicking:
```rust
let parent_act_id = act.parent_act_id.ok_or_else(|| AppError::Internal {
    source_chain: format!("return act {} has NULL parent_act_id", payload.id),
})?;
```

### WR-03: `added` path in `update_return` skips the over-return quantity guard that `do_return` enforces

**File:** `crates/trackly-app/src/services/act_service.rs:1808-1854`

**Issue:**
When adding a still-outstanding device to a return, the only checks are
"belongs to parent's `act_items`" and "currently `в_работе`" (`:1654-1681`). Unlike
`do_return` (`:1318-1347`), there is no `per_device_qty + already_returned <=
handover_qty` bound. For legacy/PRE-V015 qty>1 handover rows, or a device that was
returned then re-issued without deleting the original return, this permits a
device to be returned beyond what was handed out (a second `act_items` return row
under the same parent). In the common G-12 one-device-one-item case this is
benign, but the guard asymmetry is a latent correctness gap.

**Fix:** Port the `already_returned`/`handover_qty` SUM check from `do_return`'s
per-device loop into the `added` loop before inserting the return `act_item`.

### WR-04: V034 migration comment overstates idempotency

**File:** `migrations/V034__return_handover_date_backfill.sql:16-21`

**Issue:**
The header claims the UPDATE is "naturally idempotent (re-running it is a no-op
once handover_date_utc already equals created_at_utc for return rows)". That is
only true if no return date was ever edited. After Phase 22, `update_return` lets
`handover_date_utc` diverge from `created_at_utc`; re-running
`UPDATE acts SET handover_date_utc = created_at_utc WHERE act_type = 'return'`
would **clobber every user-entered return date**. This is not a live bug
(refinery never re-runs applied migrations), but the comment invites a dangerous
manual re-run.

**Fix:** Reword the comment to state the UPDATE is a one-time historical backfill
that is safe ONLY because refinery does not re-run it, and is NOT safe to run
manually post-Phase-22.

## Info

### IN-01: Cross-basis comparison in the `retained_with_change` detection is inconsistent but currently harmless

**File:** `crates/trackly-app/src/services/act_service.rs:1699-1717`

**Issue:** `condition_changed` compares the payload's effective condition against
the stored `act_items.condition_at_time`, while `location_changed` compares the
effective location against the **device's live** `location_id` (there is no
location column on `act_items`). The two "did the user change this?" signals use
different baselines. Also, a `None` effective condition/location is treated as
"no change" (`unwrap_or(false)`), so clearing a field is impossible via this path.
Both are acceptable given the current schema, but the asymmetry is worth a comment
so a future maintainer does not assume a single baseline.

**Fix:** Add a short comment documenting the two baselines and the
"None = no change (cannot clear)" semantics.

---

_Reviewed: 2026-07-12T23:40:24Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
