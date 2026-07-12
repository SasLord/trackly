# Phase 22: Правка возвратов (return-act-edit) - Research

**Researched:** 2026-07-12
**Domain:** Rust/axum/rusqlite backend service layer (return-act lifecycle delta-recompute) + Svelte 5 frontend form reuse
**Confidence:** HIGH — every finding below is grounded in a direct read of this repo's current code this session (not training-data assumption). This phase is 100% internal, no new dependencies, and is a direct structural sibling of Phase 19's `ActService::update` (handover edit), whose delta-recompute pattern, CAS mechanism, and audit-trail restore mechanism are reused nearly verbatim.

## Summary

ACT-03 asks for the exact capability Phase 19 built for handover acts (`ActService::update`), but for **return** acts, with three genuinely new problems Phase 19 did not have to solve: (1) a return's own "Дата возврата" needs to become a real, editable, independently-stored field instead of a parent-inherited copy; (2) "un-returning" a device inside an edit (not a full act delete) needs a **new** safety check — D-11 — because the device may have been re-issued or manually relocated *after* the return happened, and blindly restoring it would corrupt whatever touched it since; (3) the return act's `giver_name`/`receiver_name` columns are **currently write-only garbage from the client's perspective** — the create-time `do_return` path silently ignores whatever the UI's «Кто возвращает»/«Кто принимает» fields contain and hard-copies the **parent's own unswapped** `giver_name`/`receiver_name` into the return row (`act_service.rs:1220-1221`). This is a pre-existing, undocumented gap that ACT-03's D-12 requirement makes visible and must fix on the write side, not just the edit side.

Every other hard part — CAS/optimistic-lock, per-device audit-snapshot restore, delta diffing between old/new device sets, `recompute_parent_archived` sequencing — already exists in this codebase in three sibling functions (`ActService::update` for handover-edit, `ActService::do_return` for the "add" side, `ActService::delete_soft`'s `ActType::Return` branch for the "undo" side) and is directly reusable. The `update_act_header_in_tx` repo helper (built generically, not handover-specific) can be reused **unchanged** for the return's header write (giver/receiver/handover_date_utc/location_id), because it operates on the `acts` table with no `act_type` branching at all.

**Primary recommendation:** Add `ActService::update_return(payload: ActUpdateReturnDto)` that: (a) diffs the return's current `act_items` device set against the payload's full replacement set (added/removed/retained, exactly like `update()`'s D-06 delta), (b) reuses `do_return`'s per-device "add" loop body for newly-added outstanding devices, (c) reuses `delete_soft`'s "un-return" restore mechanism (`select_latest_device_mutation` + `restore_from_snapshot_in_tx`) for removed devices, gated by a **new** D-11 safety check comparing the device's current `(status_id, location_id, state)` against the snapshot this return itself set (via a new `select_latest_device_mutation_pair` audit-log query returning both `before_json` and `after_json`), (d) calls the existing `update_act_header_in_tx` for giver/receiver/«Дата возврата», (e) always calls `recompute_parent_archived` when the item count changed. Add `handover_date_utc`/`giver_name`/`receiver_name` to `ActReturnDto` too (fixing the create-path silent-drop bug as a prerequisite for D-12/D-05), switch `do_return`'s write site (`act_service.rs:1232`) from `parent.handover_date_utc` to the payload's entered date, and add a V034 backfill migration (`UPDATE acts SET handover_date_utc = created_at_utc WHERE act_type = 'return'`).

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Return-edit delta reconciliation (un-return / re-edit / add-more) | API / Backend (`ActService::update_return`) | Database (transactional guarantee) | Same single-writer-transaction discipline as `create`/`do_return`/`delete_soft`/`update` — CLAUDE.md single-writer rule. |
| «Дата возврата» storage + display switch | API / Backend (`acts.handover_date_utc` column, write-site in `do_return`/new `update_return`) | Browser/Client (`ActDetail`/`ActListRow`/`ReturnModal` read/prefill) | Backend is source of truth; frontend read-sites already key off `handover_date_utc` post-Phase-19 — no new frontend plumbing for *display*, only for the *editable input*. |
| D-11 "device unchanged since this return" safety check | API / Backend (`ActService::update_return`, `audit_log` snapshot compare) | — | Must be authoritative server-side (mirrors Phase 19 Pitfall 3 — a UI-only guard is bypassable via raw HTTP). |
| Return giver/receiver ФИО (D-12) | API / Backend (new `ActReturnDto`/`ActUpdateReturnDto` fields + `do_return`/`update_return` write) | Browser/Client (`ReturnModal` — fields already exist in local state, just never wired to the payload) | This is a **write-path bug fix**, not purely a display fix — the persisted value has never reflected user input. |
| Archival-date derived concept (D-07) | Database (compute-on-read query) | API / Backend (optional `ActDto` field if UI ever needs it) | `archived` is a bare `INTEGER 0/1` with no timestamp semantics (V004) — recommend **not** adding a column; ACT-03's actual success criteria do not require a UI display of this date. |
| Edit-form UI (prefill both sides: return's own items + parent's outstanding) | Browser/Client (`ReturnModal.svelte` in edit mode, `ActsPage.svelte` orchestration) | API / Backend (extend `ActItemDto` with location fields for prefill) | Same vertical-slice pattern as Phase 19's `ActFormBody` edit-mode reuse. |

## Standard Stack

No new packages — 100% internal Rust service/DTO/repo + Svelte component work, identical dependency footprint to Phase 19.

| Component | Version (workspace-pinned) | Role in this phase |
|-----------|----------------------------|---------------------|
| `rusqlite` | `0.39` (bundled) | New CAS `UPDATE ... WHERE version=?` reuse via existing `update_act_header_in_tx`; new `select_latest_device_mutation_pair` query |
| `refinery` | `0.9` | **One new migration needed**: `V034__return_handover_date_backfill.sql` (D-08) |
| `tauri-specta` / `specta` | pinned per workspace | New `acts_update_return` Tauri command auto-exports to `bindings.ts` |
| `axum` | `0.8` | New `/api/v1/acts_update_return` route |
| `time` | `0.3` | Unaffected — `format_ru_date`/`format_iso_date` already operate on `handover_date_utc` post-Phase-19 |
| Svelte | `5.x` (runes) | Edit-mode variant of `ReturnModal.svelte` |

**Migration required:** `V034__return_handover_date_backfill.sql` — one `UPDATE` statement, no schema change (column already exists since V015). Confirmed via `grep -l handover_date_utc migrations/*.sql` — only `V015__acts_clone_on_handover.sql` touches this column; last migration is `V033__org_settings_requisites.sql`, so **V034** is the next free version number.

```sql
-- V034__return_handover_date_backfill.sql (D-08)
-- Existing return rows currently hold handover_date_utc copied from their
-- parent handover act (do_return's old write-site, act_service.rs:1232).
-- After this phase, handover_date_utc on a return row means "Дата возврата"
-- (when devices were actually returned) — the only available historical
-- signal for that is the return row's own created_at_utc.
UPDATE acts SET handover_date_utc = created_at_utc WHERE act_type = 'return';
```
Safe to run once (refinery never re-runs applied migrations); runs before any user interaction at next startup, so no race with the code-side write-site switch.

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| New `ActUpdateReturnDto` (mirrors `ActUpdateDto`'s shape, reuses `ActReturnItemDto` for items) | Extend `ActReturnDto` + separate `id`/`expected_version` params (split-args like legacy `do_return`) | `ActReturnItemDto`'s `device_ids[]`/`condition_override`/`location_name_override` shape is *already* exactly what a return-row full-replacement-set needs, and the frontend's existing `buildReturnItems()` helper in `returnPayload.ts` can build it **unchanged** for both create and edit. A single-DTO shape (matching `build_acts_update`'s convention, not `build_acts_return`'s split-args convention) keeps `id`+`expected_version` co-located with the payload, avoiding the split-args style CONTEXT.md's Phase-19 comment explicitly called out as inconsistent (`tauri_cmds/acts.rs:86-88`). |
| `select_latest_device_mutation_pair` (new: returns `(before_json, after_json)`) | Two separate queries (existing `select_latest_device_mutation` for before_json + a new after_json-only query) | One query round-trip instead of two inside the same writer transaction; also naturally gives D-11's comparison basis (`after_json`) and the un-return restore basis (`before_json`) from a single fetch per device. |
| Compute-on-read "Дата архивации" (D-07) | New `acts.archived_at_utc` column | ACT-03's success criteria only require `archived` flag *consistency*, not a displayed archival date. A stored column would need its own recompute/clear logic every time `recompute_parent_archived` flips the flag (including flip-backs from un-return), and could drift if a return's own date is edited *after* archival — compute-on-read (`MAX(handover_date_utc)` over non-deleted child returns) can never drift because it has no independent state to go stale. |
| Reuse `update_act_header_in_tx` unchanged for the return header write | A new `update_return_header_in_tx` helper | The function has zero `act_type` branching — it just runs `UPDATE acts SET giver_name=?, receiver_name=?, location_id=?, notes=?, deadline_utc=?, handover_date_utc=COALESCE(?, ...), number=COALESCE(?, ...), version=version+1 ... WHERE id=? AND version=?` (`acts_sqlite.rs:377-426`). Passing `number: None` (return numbers never change per D-10/out-of-scope) and the return-specific giver/receiver/date values works with **no code changes** to this helper. |

## Package Legitimacy Audit

Not applicable — zero new external packages (Rust crates or npm packages) introduced by this phase.

## Architecture Patterns

### System Architecture Diagram (return-edit update flow)

```
Svelte UI (ActDetail "Редактировать" → ActsPage.handleEdit routes on act_type)
   │
   │  act.act_type === 'return':
   │    parent = await acts.get(act.parent_act_id)   ← for addable outstanding rows
   │    returnAct = act (already selectedAct, fresh via acts.get(id))
   │    ReturnModal opens in mode="edit", prefilled from BOTH:
   │      - returnAct.items[]  → checked rows (condition_at_time, device fields)
   │      - parent.items[].outstanding_device_ids → unchecked addable rows
   │      - returnAct.giver_name/receiver_name → NOT swapped (already return's own values)
   │      - returnAct.handover_date_utc → "Дата возврата" DatePicker prefill
   │    user edits: un-check (un-return) / change condition+location / check new outstanding rows
   │    submit → ActUpdateReturnDto { id, expected_version, giver_name, receiver_name,
   │                                   handover_date_utc, bulk_condition, bulk_location_name,
   │                                   apply_to_all, items: ActReturnItemDto[] (FULL set) }
   ▼
Tauri invoke `acts_update_return` ──┐
                                     ├──► build_acts_update_return(ctx, caller, payload)
axum POST /api/v1/acts_update_return┘        authorize(caller, &Action::MutateActs)
                                              ▼
                                   ActService::update_return(payload)
                                              │
                       WriterHandle::execute(closure)  ← SINGLE writer tx (BEGIN IMMEDIATE)
                                              │
       ┌──────────────────────────────────────┼──────────────────────────────────────┐
       │ 1. fetch_full_in_tx(return_id) → act │ 2. act_type must be Return            │
       │    (NotFound if missing/deleted)     │    (guard mirrors do_return's own     │
       │                                      │     act_type check, inverted)         │
       └──────────────────────────────────────┼──────────────────────────────────────┘
                                              ▼
                       3. CAS pre-check: act.version == payload.expected_version
                                              ▼
              4. Load parent = fetch_full_in_tx(act.parent_act_id)
                 d_old = current return's act_items device_ids
                 d_new = flatten(payload.items[].device_ids)  (effective_device_ids)
                 removed = d_old − d_new     added = d_new − d_old    retained = d_old ∩ d_new
                                              ▼
       5. VALIDATE (before any mutation — validate-then-commit):
          - payload.items non-empty (D-10: reject "0 positions" — use Удалить instead)
          - `added`: must belong to parent.act_items (existence check, mirrors
            do_return's :1143-1163) AND devices_repo.get_in_tx(dev).status_id ==
            in_work_status_id ("в_работе") — same guard as do_return :1286
          - `removed` ∪ (`retained` with a value-change requested): D-11 guard —
            select_latest_device_mutation_pair(tx, return_id, dev_id) → (before_json,
            after_json); current = devices_repo.get_in_tx(tx, dev_id); reject with
            AppError::Conflict if current.(status_id, location_id, state) !=
            after_json.(status_id, location_id, state)
                                              ▼
       6. For `removed`: restore_from_snapshot_in_tx(before_json) (un-return) +
          audit 'custom:update_remove' + DELETE act_items row
       7. For `added`: update_full_in_tx(на_складе, condition, location) + INSERT
          act_items(condition_at_time=effective_condition, complectation_at_time=
          before.kit) + audit 'update' {"act_id":return_id,"kind":"return"}
       8. For `retained` with changed condition/location: update_full_in_tx + UPDATE
          act_items.condition_at_time + audit
                                              ▼
       9. update_act_header_in_tx(tx, return_id, ActPatch{giver_name, receiver_name,
          location_id: bulk_location, notes: None, deadline_utc: None,
          handover_date_utc: Some(payload.handover_date_utc), number: None,
          expected_version}, now)   ← REUSED UNCHANGED from Phase 19
                                              ▼
       10. recompute_parent_archived(&tx, parent_act_id, now)   ← ALWAYS when
           added/removed non-empty (item-count-driven, symmetric to update()'s gate)
                                              ▼
       11. Final audit row (entity_type='act', action='update') + tx.commit()
                                              ▼
                          return self.get(return_id).await  (fresh ActDto)
```

### Recommended Project Structure (files touched, no new modules)

```
crates/trackly-infra/src/repos/audit_log_sqlite.rs   # new: select_latest_device_mutation_pair
crates/trackly-infra/src/repos/acts_sqlite.rs        # no changes needed — update_act_header_in_tx reused as-is
crates/trackly-app/src/dto/act.rs                    # ActReturnDto += giver_name/receiver_name/handover_date_utc
                                                      #   (fixes create-path silent-drop, needed for D-05/D-12);
                                                      # ActItemDto += device_location_id/device_location fields
                                                      #   (needed for return-edit row prefill);
                                                      # new ActUpdateReturnDto (mirrors ActUpdateDto shape,
                                                      #   reuses ActReturnItemDto for items)
crates/trackly-app/src/services/act_service.rs       # do_return: write-site switch line 1232 (parent.handover_date_utc
                                                      #   → payload.handover_date_utc, with a documented back-compat
                                                      #   default when the field is absent); giver_name/receiver_name
                                                      #   write-site switch lines 1220-1221 (payload values, default
                                                      #   to parent-swap when absent, back-compat);
                                                      # new ActService::update_return(); load_items_for_act's SQL
                                                      #   gains a LEFT JOIN locations for the new ActItemDto fields
crates/trackly-app/src/tauri_cmds/acts.rs            # build_acts_update_return + #[tauri::command] acts_update_return
crates/trackly-app/src/http/acts.rs                  # UpdateReturnPayload + handler_update_return + router() entry
ui/src/lib/api/acts.ts                               # acts.updateReturn(...)
ui/src/features/acts/ReturnModal.svelte              # mode: 'create' | 'edit' prop; editTarget/parentAct props;
                                                      #   Дата возврата DatePicker (reuse ActFormBody's
                                                      #   unixToIso/isoToUnix/todayISO pattern verbatim);
                                                      #   drop the giver/receiver auto-swap when mode==='edit'
                                                      #   (prefill directly from editTarget's own values, D-12)
ui/src/features/acts/ReturnItemsTable.svelte         # no structural change — rows already carry condition/location;
                                                      #   edit mode just seeds `checked` per row from whether the
                                                      #   device is already in returnAct.items
ui/src/features/acts/returnPayload.ts                # buildReturnItems() reused UNCHANGED for edit submission
crates/trackly-app/src/dto/act.rs (or new module)     # migrations/V034__return_handover_date_backfill.sql
ui/src/features/acts/ActDetail.svelte                # line 70 edit-gate: add `|| act.act_type === 'return'`
ui/src/features/acts/ActsPage.svelte                 # handleEdit: branch on act.act_type — return acts need
                                                      #   a second acts.get(act.parent_act_id) fetch before
                                                      #   opening ReturnModal in edit mode
```

### Pattern 1: Optimistic-lock CAS + header write — reuse `update_act_header_in_tx` verbatim

**What:** The exact same generic helper Phase 19 built for handover header edits. It has no `act_type` branching:
```rust
// Source: crates/trackly-infra/src/repos/acts_sqlite.rs:377-426 (update_act_header_in_tx)
"UPDATE acts SET giver_name = ?1, receiver_name = ?2, \
 location_id = ?3, notes = ?4, deadline_utc = ?5, \
 handover_date_utc = COALESCE(?6, handover_date_utc), \
 number = COALESCE(?7, number), \
 version = version + 1, updated_at_utc = ?8 \
 WHERE id = ?9 AND version = ?10 AND deleted_at_utc IS NULL"
```
**When to use:** `update_return`'s header write. Build `ActPatch { giver_name: Some(payload.giver_name), receiver_name: Some(payload.receiver_name), location_id: Some(resolved_bulk_location_id), notes: Some(None), deadline_utc: Some(None), handover_date_utc: Some(payload.handover_date_utc), number: None, expected_version: payload.expected_version }` — `number: None` is critical: return numbers are explicitly out of scope (CONTEXT.md "Не в scope: изменение нумерации возвратов"), so passing `None` guarantees the `COALESCE` keeps the current `sub_number`-derived display unchanged.

### Pattern 2: Device delta reconciliation for returns — three sub-cases, all reusing existing primitives

**"Un-return" (removed device) — reuse `delete_soft`'s undo mechanism, scoped to ONE device:**
The `ActType::Return` branch of `delete_soft` (`act_service.rs:1811-1839`) already calls `undo_device_mutations_for_act(&tx, ..., id, ...)` which iterates `select_device_mutations_for_act(tx, act_id)` in reverse and restores every device the return touched. For a **single-device** removal during an *edit* (not full delete), use the already-built single-device helper `select_latest_device_mutation` (`audit_log_sqlite.rs:104-122`, added in Phase 19 specifically with the comment "No caller yet — Plan 19-03 is the first" — it is generic on `entity_type='device'` + `act_id` + `device_id` and works identically whether `act_id` refers to a handover or a return act):
```rust
// Source: crates/trackly-infra/src/repos/audit_log_sqlite.rs:104-122
pub fn select_latest_device_mutation(&self, tx, act_id, device_id) -> Result<Option<String>, AppError>
// ORDER BY created_at_utc DESC, id DESC LIMIT 1 — restores to the state
// immediately BEFORE this return act's own do_return mutation, which is
// exactly "put it back в работу with its pre-return location/condition".
```
Then `devices_repo.restore_from_snapshot_in_tx(tx, device_id, &snapshot, now)` (unchanged) + a new audit row (`action: "custom:update_remove"`, same convention as handover's D-06).

**"Add outstanding device" — copy `do_return`'s per-device loop body:**
```rust
// Source: crates/trackly-app/src/services/act_service.rs:1281-1401 (do_return's inner loop)
// Status guard (CR-02): before.status_id != in_work_status_id → Conflict
// (this is the exact guard that naturally prevents adding a device already
//  claimed by ANOTHER sibling return of the same parent — no new logic needed)
let after = devices_repo.update_full_in_tx(&tx, device_id, on_warehouse_status_id,
    effective_location, effective_condition.as_deref(), now)?;
acts_repo.insert_act_item_in_tx(&tx, return_act_id, device_id, per_device_qty,
    effective_condition.as_deref(), before.kit.as_deref())?;
```
Also reuse `do_return`'s existence check (`act_service.rs:1143-1163`: every device_id must belong to `act_items WHERE act_id = parent_id`) for any newly-added device_id in the edit payload.

**"Change condition/location of a retained returned device":** Same `update_full_in_tx` call as the add-path, but on an EXISTING act_items row — `UPDATE act_items SET condition_at_time = ?` instead of `INSERT`. Gated by the SAME D-11 check as removal (see Pattern 4).

### Pattern 3: `recompute_parent_archived` — call unconditionally on the item-count-changed path

**What:** Bare `COUNT`-based recompute, no date/status semantics of its own (`acts_sqlite.rs:502-539`):
```sql
archived = (handover_total > 0 AND handover_total <= returned_total)
```
where `returned_total = COUNT(DISTINCT rai.device_id)` across all non-deleted child returns of the parent. Since this is a pure re-derivation from current `act_items` state, calling it after **any** add/remove delta on `update_return` (mirroring `update()`'s own `if !added.is_empty() || !removed.is_empty()` gate at `act_service.rs:934`) is both correct and idempotent — it naturally flips `archived` back to `false` if an un-return drops `returned_total` below `handover_total`, and flips it to `true` if adding devices to a return pushes `returned_total` to meet `handover_total`. No new logic needed here at all — call the existing function with `parent_act_id`, exactly as `do_return` (`act_service.rs:1406`) and `delete_soft`'s Return branch (`act_service.rs:1823`) already do.

### Pattern 4: D-11 conflict detection — concrete query, not a vague heuristic

**The precise unsafe condition, derived from the codebase's own invariants:** A device can ONLY have its `status_id`/`location_id`/`state` (condition) changed through `act_service.rs`'s three mutation helpers (`update_status_and_location_in_tx`, `update_full_in_tx`, `restore_from_snapshot_in_tx`) **or** through `DeviceService::update` (the general "Устройства" edit page, `device_service.rs:216-276`, which accepts a `DevicePatch` with independent `location_id`/`state`/`status_id` fields — confirmed via `crates/trackly-core/src/domain/devices.rs:32-43`). This means a device that a return set to `(на_складе, location=X, condition="Хорошее")` can drift away from that snapshot via **either** a later handover-act re-issuance (status changes) **or** a manual device-page edit (location/condition changes while status stays `на_складе`). D-11's own wording ("устройство больше не в том состоянии/локации, которое возврат установил ... либо device_id завязан на более поздний handover-акт") covers **both** paths — so the correct check is a **3-field snapshot compare**, not a status-only check.

**Concrete new repo helper** (sibling of `select_latest_device_mutation`, one query instead of two):
```sql
-- New: SqliteAuditLogRepository::select_latest_device_mutation_pair
SELECT before_json, after_json FROM audit_log
 WHERE entity_type = 'device'
   AND entity_id = ?2
   AND json_extract(payload_json, '$.act_id') = ?1
   AND before_json IS NOT NULL
 ORDER BY created_at_utc DESC, id DESC LIMIT 1
```
Called with `act_id = return_act_id` (not the parent's id) — the `after_json` from THIS return's own most-recent mutation of this device is exactly "what this return set." Compare:
```rust
let current = devices_repo.get_in_tx(&tx, device_id)?;
let expected: serde_json::Value = serde_json::from_str(&after_json)?;
let safe = expected.get("status_id").and_then(|v| v.as_i64()) == Some(current.status_id)
    && expected.get("location_id").and_then(|v| v.as_i64()) == current.location_id
    && expected.get("state").and_then(|v| v.as_str()) == current.state.as_deref();
if !safe {
    return Err(AppError::Conflict {
        reason: format!(
            "Устройство id={} изменилось после этого возврата (другой акт или изменение \
             вручную) — редактирование строки невозможно", device_id),
    });
}
```
Run this check for every device_id in `removed` (un-return) **and** every device_id in `retained` whose payload requests a condition/location change different from what's currently stored. Devices in `retained` with **no** requested value change do NOT need this check (D-05-equivalent: an unrelated no-op resubmit must not be blocked). Devices in `added` do not need it either — they get the standard `in_work_status_id` guard instead (Pattern 2), which is a stronger, simpler, already-existing check for that direction.

**No force-override** — matches CONTEXT.md D-11 explicitly; this Conflict aborts the entire transaction (validate-then-mutate ordering, same as D-08's guard in `update()`).

### Anti-Patterns to Avoid

- **Checking only `status_id` for D-11.** A device can drift via a manual `DeviceService::update` location/condition edit without ever touching `status_id`. A status-only check would miss this (see Pattern 4).
- **Reusing `select_device_mutations_for_act` (bulk, ASC order) for the un-return restore instead of the single-device `DESC LIMIT 1` lookup.** Same Pitfall-2-class bug as Phase 19 — if a return is edited more than once (add device, later edit removes it again), the bulk-ASC query's first match would restore to the ORIGINAL pre-first-do_return state, not the state immediately before the most recent inclusion.
- **Trusting `parent.handover_date_utc` as a return's date after this phase ships.** Once `do_return`'s write-site switches to the payload's own date, any remaining code path that still reads `parent.handover_date_utc` for a *return* row's display is stale — the migration (D-08) exists precisely because historical rows conflate the two.
- **Adding a stored `archived_at_utc` column.** Creates a second source of truth that must be kept in sync with every `recompute_parent_archived` call (including flip-backs) for a UI element ACT-03 does not actually require. Compute-on-read has no drift risk.
- **Forgetting the `do_return` create-path fix (giver_name/receiver_name) is a prerequisite, not optional polish.** If the create path keeps hard-copying `parent.giver_name`/`parent.receiver_name` (`act_service.rs:1220-1221`) while the edit path lets the user set independent values, every NEWLY created return continues to persist the wrong (parent-copied) values, and the very next time it's opened for edit, the prefill will show the WRONG names (not what was actually typed at create time) — D-12 cannot be satisfied without also fixing create.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Return header CAS update (giver/receiver/date) | A new return-specific header UPDATE statement | `update_act_header_in_tx` (unchanged, `acts_sqlite.rs:377`) | Zero `act_type` branching in the existing helper — it's already generic. |
| Un-return device restore | A new query walking `devices`/history tables | `select_latest_device_mutation` + `restore_from_snapshot_in_tx` (both already exist, Phase 19) | Same audit-log-as-source-of-truth mechanism `delete_soft`'s undo already uses. |
| "Is this device untouched since the return" (D-11) | A bespoke join across `acts`/`act_items`/`devices` trying to infer "later handover" | The 3-field snapshot compare against this return's own `after_json` (Pattern 4) | Directly answers the literal question D-11 asks, catches BOTH the reissue-by-handover case AND the manual-device-edit case with one check. |
| "Is this outstanding device safe to add" | New existence/status logic | `do_return`'s existing existence check (`act_service.rs:1143-1163`) + `in_work_status_id` guard (`act_service.rs:1286`) | Byte-identical semantics to a fresh return; no new invariant to design. |
| Archival date | New `archived_at_utc` column + write-path threading | `MAX(handover_date_utc)` compute-on-read over non-deleted child returns | Zero drift risk, zero migration, satisfies D-07's literal definition. |

**Key insight:** Every hard part of ACT-03 (delta reconciliation, CAS, header write, archived recompute) is either a byte-identical reuse of a Phase-19-built primitive or a byte-identical reuse of a `do_return`/`delete_soft`-built primitive. The genuinely NEW work is: (1) the D-11 safety check (Pattern 4 — new but small, ~15 lines + 1 new SQL query), (2) wiring giver/receiver/date into the create path too (fixing the silent-drop bug), and (3) the frontend's dual-source prefill (return's own items + parent's outstanding).

## Runtime State Inventory

Not applicable — this is not a rename/refactor/migration phase in the sense that requires the 5-category audit (no renamed identifiers, no external service state). The one schema change is a straightforward backfill `UPDATE` (D-08), already covered above.

## Common Pitfalls

### Pitfall 1: The create-path giver_name/receiver_name bug means D-12's "prefill from saved values" has NO correct saved value to prefill from — for existing rows
**What goes wrong:** For every return act created **before** this phase ships, `acts.giver_name`/`acts.receiver_name` on the return row are copies of the **parent's own unswapped** values (`act_service.rs:1220-1221`: `giver_name: parent.giver_name.clone()`), NOT what the user typed into the ReturnModal's «Кто возвращает»/«Кто принимает» fields (which were captured in local Svelte state but never included in the `ActReturnDto` payload sent to `acts.doReturn` — confirmed by reading the full `ActReturnDto` struct at `dto/act.rs:126-136`, which has no giver/receiver fields at all, and `ReturnModal.svelte`'s `handleSubmit` payload construction at line 112-118, which never references `giverName`/`receiverName`). Opening an EXISTING return act for edit will therefore prefill «Кто возвращает» with the ORIGINAL giver's name (not the person who actually returned it), which is silently wrong data, not a display bug.
**Why it happens:** The swap logic exists only in the UI's *display* (lines 60-61 of `ReturnModal.svelte`) — it was never connected to a backend field because `do_return`'s return-row construction never accepted these as inputs.
**How to avoid:** This phase MUST extend `ActReturnDto` (not just a new edit-only DTO) with `giver_name: Option<String>`/`receiver_name: Option<String>`, switch `do_return`'s write-site to use them (falling back to the existing parent-swap default when `None`, for backward compatibility with any client not yet updated), and accept that for return acts created before this fix, the prefilled ФИО will show the historically-wrong (parent-copied) values — there is no way to recover the "actually returned by" name for old rows since it was never persisted. Flag this as a known, accepted limitation (no migration can fix data that was never captured).
**Warning signs:** A plan that treats D-12 as "just read `act.giver_name`/`act.receiver_name` for prefill" without also fixing `do_return`'s write path has only fixed the edit half of the bug — new returns created after this phase but with the create-form's giver/receiver still unwired will continue to produce wrong data.

### Pitfall 2: `ActItemDto` has no location field at all — return-edit prefill cannot show "Расположение" without a DTO/SQL extension
**What goes wrong:** `load_items_for_act`'s SQL (`act_service.rs`, `load_items_for_act` helper) selects `d.name, d.inventory_number, d.serial_number, d.model, d.notes` — no `location_id`/`location` column. `ActItemDto` (`dto/act.rs:94-116`) has no location field either. The return-edit form needs to prefill "Расположение" for each already-returned row (D-13), which requires the device's current location — this data is simply not on the wire today for ANY act item (handover or return).
**Why it happens:** Handover-act items never needed to display location (a handover's own header carries a single act-level `location`); returns are the first case where per-item location display/editing matters.
**How to avoid:** Extend `load_items_for_act`'s SQL with `LEFT JOIN locations dl ON d.location_id = dl.id` (mirrors the existing `LEFT JOIN locations l ON a.location_id = l.id` pattern already used in `SELECT_ACTS`, `acts_sqlite.rs:42`) and add `device_location_id: Option<i64>` / `device_location: Option<String>` to `ActItemDto`. Harmless for handover-act consumers (extra fields, ignored).
**Warning signs:** A plan that designs the edit-mode `ReturnItemsTable` prefill without first checking whether `ActItemDto` carries location will discover this gap mid-implementation, not at planning time.

### Pitfall 3: D-11's server-side guard must not be bypassable via raw HTTP (same class as Phase 19 Pitfall 3)
**What goes wrong:** If D-11 is implemented only as "hide the uncheck-checkbox for rows the UI thinks are unsafe," a raw POST to `/api/v1/acts_update_return` with a crafted payload removing a device that's actually been re-issued elsewhere bypasses it entirely.
**Why it happens:** Same structural temptation as Phase 19 — the UI doesn't proactively know about conflicts (this research recommends NOT building a proactive UI hint at all — see Pattern 4's closing note — so there is no UI-side guard to lean on in the first place, which is actually the SAFER default: no false sense of client-side protection).
**How to avoid:** Implement Pattern 4 as a hard `AppError::Conflict` inside `ActService::update_return`'s writer transaction, unconditionally. Add an HTTP-level integration test posting a raw payload that attempts to remove/re-edit a device whose current state diverges from the return's own snapshot, asserting a rejection independent of any UI state.
**Warning signs:** A plan whose only D-11 task is "add a warning icon to rows that look unsafe" with no corresponding backend validation task has under-scoped the requirement.

### Pitfall 4: `recompute_parent_archived`'s "unconditional +1 version bump" ordering trap (same class as Phase 19's CR-01 fix)
**What goes wrong:** `recompute_parent_archived` bumps the PARENT act's `version` unconditionally (`acts_sqlite.rs:532-537`, `UPDATE acts SET archived=?, ..., version = version + 1 WHERE id = ?3` — no CAS `WHERE version=?` clause on this statement). If `update_return`'s own header-write CAS check (on the RETURN act, a different row) runs AFTER a call that also touches the parent's version, there's no ordering hazard between them since they're different rows — but if a future refactor ever tries to run `recompute_parent_archived` on the SAME row a CAS-guarded UPDATE just touched (not the case here — the return and its parent are always different act rows), the same trap Phase 19 documented (`act_service.rs:920-936` comment) would apply. Not a live risk for THIS phase's `update_return` since parent ≠ return, but worth carrying forward the awareness: never call `recompute_parent_archived(&tx, X, now)` on the SAME act id a CAS UPDATE was just applied to, within the same transaction, without re-deriving `expected_version` first.
**Why it happens:** `recompute_parent_archived`'s version bump is deliberately unconditional-on-any-id (it doesn't take an `expected_version` at all) so it can be called safely from multiple different mutation paths (`create`/`do_return`/`update`/`delete_soft`/`update_return`) without each one having to track the parent's current version.
**How to avoid:** Call `recompute_parent_archived(&tx, parent_act_id, now)` — targeting the PARENT — strictly AFTER `update_act_header_in_tx(&tx, return_id, ...)` — targeting the RETURN itself — since they're different rows, ordering relative to each other doesn't matter for CAS correctness, but do keep the call inside the same transaction so both mutations are atomic.
**Warning signs:** None specific to this phase — documented for completeness since the underlying function is shared.

## Code Examples

### D-05: the exact write-site to switch in `do_return`

```rust
// Source: crates/trackly-app/src/services/act_service.rs:1214-1235 (do_return, return_row construction)
let return_row = ActRow {
    id: 0,
    number: parent.number,
    sub_number: Some(sub_number),
    parent_act_id: Some(act_id),
    act_type: ActType::Return,
    giver_name: parent.giver_name.clone(),      // ← D-12: switch to payload.giver_name.unwrap_or_else(|| parent.receiver_name.clone()) (swap-default)
    receiver_name: parent.receiver_name.clone(), // ← D-12: switch to payload.receiver_name.unwrap_or_else(|| parent.giver_name.clone())
    location_id: resolved_bulk_location_id,
    location: None,
    notes: None,
    deadline_utc: None,
    archived: false,
    created_at_utc: now,
    updated_at_utc: now,
    deleted_at_utc: None,
    version: 1,
    // Return-акты наследуют parent.handover_date_utc.
    handover_date_utc: parent.handover_date_utc, // ← D-05: switch to payload.handover_date_utc.unwrap_or(now)
    parent_number: None,
    sibling_return_count: None,
};
```
**NOT in scope for D-05's write-site fix alone:** the read-side display work is ALREADY done — Phase 19 already switched `ActListRow.svelte:36` and `ActDetail.svelte:43` to read `act.handover_date_utc`, and `acts_sqlite.rs`'s `list()`/`search_acts()` already `ORDER BY a.handover_date_utc DESC` (confirmed at `acts_sqlite.rs:295` and `:601`). **D-06 (sort by «Дата возврата») is therefore already satisfied by D-05's write-site fix alone — no separate sort-logic work needed.**

### D-11: the new repo helper

```rust
// New — sibling of select_latest_device_mutation, audit_log_sqlite.rs
/// Returns (before_json, after_json) of the most recent mutation this
/// specific act_id made to this specific device — used both to restore
/// on un-return (before_json) and to detect drift since (after_json vs
/// current device row) for Phase 22 D-11.
pub fn select_latest_device_mutation_pair(
    &self,
    tx: &Transaction<'_>,
    act_id: i64,
    device_id: i64,
) -> Result<Option<(String, String)>, AppError> {
    tx.query_row(
        "SELECT before_json, after_json FROM audit_log \
         WHERE entity_type = 'device' \
           AND entity_id = ?2 \
           AND json_extract(payload_json, '$.act_id') = ?1 \
           AND before_json IS NOT NULL \
         ORDER BY created_at_utc DESC, id DESC LIMIT 1",
        params![act_id, device_id],
        |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
    )
    .optional()
    .map_err(map_rusqlite)
}
```

### Frontend: reuse `ActFormBody`'s date-picker pattern verbatim for «Дата возврата»

```typescript
// Source: ui/src/features/acts/ActFormBody.svelte:44-61 (todayISO/unixToIso)
function todayISO(): string { /* UTC Y-M-D */ }
function unixToIso(unixSeconds: number | null | undefined): string { /* UTC Y-M-D */ }
// isoToUnix at line 120 — inverse, used at submit time.
// DatePicker component usage: ActFormBody.svelte:261
// <DatePicker id="act-handover-date" bind:value={handoverDateISO} required />
```
Apply identically in `ReturnModal.svelte`: `let returnDateISO = $state(isEditPrefill ? unixToIso(editTarget!.handover_date_utc) : todayISO());`, submit `handover_date_utc: isoToUnix(returnDateISO)`.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|---------------|--------|
| Return acts inherit `handover_date_utc` from parent; return acts are non-editable (Phase 19 D-07) | Return acts have their own «Дата возврата», independently editable; return acts ARE editable | This phase (22) | `do_return`'s write-site (`act_service.rs:1232`) and the return-row's giver/receiver copy (`:1220-1221`) both need a payload-driven switch; sort/display already work (Phase 19 already keyed off `handover_date_utc` generically) |

**Deprecated/outdated:** None — no library deprecations. The only "old → new" shift is the internal semantic redefinition of what `handover_date_utc` means for `act_type='return'` rows.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | The `ActReturnDto` create-path giver_name/receiver_name gap (Pitfall 1) is a genuine pre-existing bug (not an intentional "return always inherits parent's roles unswapped" design) and should be fixed as part of this phase's D-12 work, not deferred | Summary, Pitfall 1 | If the project owner considers this intentional behavior (e.g., "return's giver/receiver ARE supposed to equal parent's, the UI swap-fields are cosmetic-only and never meant to persist"), fixing it would be an unrequested behavior change. Low risk given D-12 explicitly requires these fields be "редактируемы" and "предзаполнены сохранёнными значениями возврата" — which is impossible to satisfy correctly without a real write path. |
| A2 | D-11's correct detection condition is a 3-field snapshot compare (status_id + location_id + state) against THIS return's own after_json, not merely a status_id-only check | Pattern 4 | If wrong (i.e., only status matters per stricter reading of D-11), the 3-field check is a strict superset — it can only be MORE conservative (block more, never less-safe), so worst case is over-blocking a legitimate edit that changed device.notes/etc. through an unrelated path without status/location/condition drift — recoverable by loosening the check later. |
| A3 | «Дата архивации» (D-07) does not need a stored column or an `ActDto`-exposed field for THIS phase, since ACT-03's actual success criteria only mention `archived` flag consistency, not a UI display of an archival date | Alternatives Considered, Architectural Responsibility Map | If a UI display IS actually wanted this phase (CONTEXT.md's D-07 wording is ambiguous on this), the compute-on-read query (`MAX(handover_date_utc) FROM acts WHERE act_type='return' AND parent_act_id=? AND deleted_at_utc IS NULL`) can be added to `ActDto` as an optional field with zero schema risk — just extra scope, not a design change. |
| A4 | The new return-update command surface should be a single-DTO shape (`ActUpdateReturnDto` carrying `id`+`expected_version`), following `build_acts_update`'s convention, rather than split-args like the legacy `build_acts_return` | Alternatives Considered | Low risk — either shape works; this is purely a code-organization preference already implicitly endorsed by Phase 19's own comment noting the split-args style as the older, less-preferred convention (`tauri_cmds/acts.rs:86-88`). |

## Open Questions

1. **Should `do_return`'s giver/receiver fix accept a required or optional payload field?**
   - What we know: `ActReturnDto` currently has no such fields at all; adding them as `Option<String>` (matching `ActCreateDto.location_name`'s optional-with-fallback style) preserves backward compatibility with any not-yet-updated client (falls back to the existing parent-swap default).
   - What's unclear: Whether the planner wants to make the frontend's `ReturnModal` create-mode submission REQUIRE these fields (since the UI already collects them, just never sent them) or keep them optional at the DTO level for defense-in-depth.
   - Recommendation: `Option<String>` at the DTO level (defense-in-depth, back-compat), but the frontend's create-mode submission should ALWAYS send the current `giverName`/`receiverName` state (no reason not to, since the UI already has them) — closing the gap end-to-end in one plan, not leaving the DTO field theoretically-optional-but-practically-always-sent.

2. **Does the return-edit form need a proactive UI hint for D-11-unsafe rows, or is server-side-only rejection acceptable for this phase's MVP scope?**
   - What we know: Phase 19's equivalent guard (D-08) was implemented server-side-only, with the UI simply surfacing the resulting error toast on a rejected submit — no proactive graying-out of "unsafe" rows.
   - What's unclear: Whether return-edit's UX bar is higher (since D-11 conflicts may be more commonly hit than D-08's, given devices flow in and out of stock more often than acts get deleted).
   - Recommendation: Match Phase 19's precedent (server-side-only, error-toast-on-reject) for MVP — a proactive hint would require exposing per-item safety state on `ActItemDto`, which is additional DTO surface not strictly required by CONTEXT.md's decisions. Flag as a possible follow-up gap-closure item if live UAT surfaces friction, same pattern as Phase 19's D-09..D-13 gap-closure round.

## Environment Availability

Skipped — no external tool/service/runtime dependency introduced by this phase (pure in-repo Rust + Svelte code change against an already-running local SQLite file), identical to Phase 19.

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | `cargo test` (rusqlite integration tests against `trackly_infra::test_support` tempfile-backed real SQLite, WAL+migrations applied) — no frontend unit-test framework in this repo (no vitest/jest); frontend correctness covered by `svelte-check` + manual/human-verify checkpoints |
| Config file | none — pattern lives in existing test files, e.g. `crates/trackly-app/tests/acts_update.rs`, `acts_returns.rs`, `acts_undo.rs` |
| Quick run command | `cargo test -p trackly-app --test acts_update_return` (new dedicated test file) |
| Full suite command | `cargo test --workspace` (project convention: never run two `cargo test` invocations concurrently — they contend on the `target/` lock) |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| ACT-03 | Return-edit happy path: change condition/location on a retained returned device | integration | `cargo test -p trackly-app --test acts_update_return -- retained_edit_changes_device_condition_location` | ❌ Wave 0 |
| ACT-03 | Un-return (remove device from return) restores device to prior в_работе state (D-06/D-09.1) | integration | `cargo test -p trackly-app --test acts_update_return -- un_return_restores_prior_state` | ❌ Wave 0 |
| ACT-03 | Add outstanding device to an existing return (D-09.3) | integration | `cargo test -p trackly-app --test acts_update_return -- add_outstanding_device_to_return` | ❌ Wave 0 |
| ACT-03 | D-10: saving an empty item set (all unchecked) is rejected | integration | `cargo test -p trackly-app --test acts_update_return -- reject_empty_item_set` | ❌ Wave 0 |
| ACT-03 | D-11: un-returning a device whose status has drifted (re-issued by a later handover) is rejected | integration | `cargo test -p trackly-app --test acts_update_return -- reject_un_return_after_reissue` | ❌ Wave 0 |
| ACT-03 | D-11: editing a retained device whose location was changed via a manual device-page edit (status unchanged) is rejected | integration | `cargo test -p trackly-app --test acts_update_return -- reject_edit_after_manual_device_relocation` | ❌ Wave 0 |
| ACT-03 | D-11: un-returning/re-editing when nothing has changed since the return succeeds normally | integration | `cargo test -p trackly-app --test acts_update_return -- allow_edit_when_device_untouched` | ❌ Wave 0 |
| ACT-03 | archived flag flips false→true when an edit adds the last outstanding device to a return | integration | `cargo test -p trackly-app --test acts_update_return -- add_last_device_archives_parent` | ❌ Wave 0 |
| ACT-03 | archived flag flips true→false when an un-return removes a device from a fully-returned parent | integration | `cargo test -p trackly-app --test acts_update_return -- un_return_unarchives_parent` | ❌ Wave 0 |
| ACT-03 | Version mismatch → `OptimisticLockMismatch` (409) | integration | `cargo test -p trackly-app --test acts_update_return -- version_mismatch_returns_conflict` | ❌ Wave 0 |
| ACT-03 | D-12: giver_name/receiver_name persist as submitted (create AND edit), not silently dropped | integration | `cargo test -p trackly-app --test acts_returns -- create_persists_giver_receiver_from_payload` (extend existing file) + `acts_update_return -- edit_persists_giver_receiver` | ❌ Wave 0 (both new) |
| ACT-03 | D-05/D-08: `handover_date_utc` write-site uses payload's date, not parent's; migration backfill sets existing rows to created_at_utc | integration | `cargo test -p trackly-app --test acts_date_source -- do_return_persists_own_date` (extend) + a migration idempotency/backfill test | ❌ Wave 0 (extend `acts_date_source.rs`) |
| ACT-03 | RBAC: `acts_update_return` gated by `Action::MutateActs` (Employee role rejected) | integration | extend `crates/trackly-app/tests/role_endpoint_matrix.rs` with new case (mirrors existing `acts_update` case ~line 1415-1430) | ❌ Wave 0 |
| ACT-03 (UI) | Edit form prefilled from BOTH the return's own items AND the parent's outstanding items, not a stale list() row | manual / human-verify | N/A — UI behavior, checkpoint-gated per `human_verify_mode: end-of-phase` | N/A |

### Sampling Rate
- **Per task commit:** `cargo test -p trackly-app --test acts_update_return` (once created) plus whichever existing file is extended (`acts_returns.rs`, `acts_date_source.rs`)
- **Per wave merge:** `cargo test --workspace` (single invocation, no concurrent `cargo test`)
- **Phase gate:** Full suite green + `pnpm --dir ui build` (LAN/browser mode serves `ui/dist`, not HMR) before `/gsd-verify-work`

### Wave 0 Gaps
- [ ] `crates/trackly-app/tests/acts_update_return.rs` — new file, covers all ACT-03 rows above
- [ ] Extend `crates/trackly-app/tests/acts_returns.rs` — add `create_persists_giver_receiver_from_payload` (Pitfall 1 fix regression guard)
- [ ] Extend `crates/trackly-app/tests/acts_date_source.rs` — add assertion that `do_return` persists the payload's own `handover_date_utc`, not the parent's
- [ ] New migration test: `migrations/V034__return_handover_date_backfill.sql` idempotency (apply-twice-is-safe, standard refinery invariant already covered by the project's general migration test suite — confirm it picks up V034 automatically)
- [ ] Extend `crates/trackly-app/tests/role_endpoint_matrix.rs` — add `acts_update_return` RBAC case
- [ ] Regenerate `ui/src/bindings.ts` (`cargo test -p trackly-app --test export_bindings`) once `ActUpdateReturnDto`/`acts_update_return`/extended `ActReturnDto`/`ActItemDto` exist — extend `export_bindings.rs`'s assertions

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|----------------|---------|-------------------|
| V2 Authentication | no (unchanged) | Session-based auth already gates every `/api/v1/acts_*` route |
| V3 Session Management | no (unchanged) | `tower-sessions`, unaffected |
| V4 Access Control | **yes** | `authorize(caller, &Action::MutateActs)` — same action as `create`/`do_return`/`delete_soft`/`update`, no new `Action` variant. D-11's device-drift guard and the return act_type guard are access-control-adjacent business-rule guards that MUST be enforced inside `ActService::update_return`, not merely in the UI (Common Pitfall 3) |
| V5 Input Validation | **yes** | New `ActUpdateReturnDto`/extended `ActReturnDto` need the same rigor as existing `validate_return`/`validate_update`: dedup device_ids, non-empty items (D-10), non-empty giver/receiver strings (mirrors the `NOT NULL` DB constraint on `acts.giver_name`/`receiver_name`, `migrations/V004...sql`) |
| V6 Cryptography | no | Not applicable |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|----------------------|
| Client submits stale `version` expecting last-write-wins | Tampering | CAS `UPDATE ... WHERE version=?` via reused `update_act_header_in_tx` |
| Client attempts to un-return / re-edit a device that has since been re-issued or manually relocated, via raw HTTP bypassing any UI hint | Tampering / business-rule bypass | Server-side D-11 guard (Pattern 4) — MUST run inside the same transaction before any device mutation |
| Client submits `acts_update_return` against a `handover`-type act id (type confusion) | Tampering | `ActService::update_return` must check `act.act_type == ActType::Return` and reject `Handover` acts, mirroring `update()`'s own inverse guard (`act_service.rs:602-607`) |
| Client submits a completely empty item set to force-clear a return without going through `delete_soft` | Tampering (business-rule bypass, D-10) | Reject `payload.items.is_empty()` in `validate_update_return`, same style as `validate_update`'s existing check (`act_service.rs:528-533`) |
| Employee-role caller attempts `acts_update_return` directly | Elevation of Privilege | `authorize(caller, &Action::MutateActs)` already excludes Employee — extend `role_endpoint_matrix.rs` with a case proving this holds for the new endpoint |

## Sources

### Primary (HIGH confidence — all direct repo reads, this session)
- `crates/trackly-app/src/services/act_service.rs` (targeted full-section reads: `update` :515-1027, `do_return` :1033-1441, `get`/`search`/`list` :1447-1721, `delete_soft` :1721-1845, helper functions :2225-2418) — every line-number citation in this document was verified against the current file state this session
- `crates/trackly-infra/src/repos/acts_sqlite.rs` (targeted reads: `insert_act_in_tx` :92-116, `insert_act_item_in_tx` :119-142, `next_sub_number_for_parent`/`recompute_parent_archived` :464-539, `update_act_header_in_tx` :377-426, `SELECT_ACTS`/ORDER BY sites :30-42, :295, :601)
- `crates/trackly-infra/src/repos/audit_log_sqlite.rs` (full file read) — `select_device_mutations_for_act`, `select_latest_device_mutation` (confirmed Phase-19-built, generic on any `act_id`)
- `crates/trackly-infra/src/repos/devices_sqlite.rs` :300-440 — `update_status_and_location_in_tx`, `update_full_in_tx`, `restore_from_snapshot_in_tx`
- `crates/trackly-app/src/services/device_service.rs` :190-276 — `DeviceService::update` (confirms the manual device-page edit path exists and can independently change location/condition — basis for Pattern 4's 3-field check, not status-only)
- `crates/trackly-core/src/domain/devices.rs` :32-43 — `DevicePatch` (confirms independent location_id/state/status_id fields)
- `crates/trackly-core/src/domain/acts.rs` (full file read) — `ActRow` (handover_date_utc at line 155, NOT line 141 as CONTEXT.md cited — drifted), `ActPatch` (struct at line 115, doc-comment starting 109 — CONTEXT.md's ":110" is close), `ActType`
- `crates/trackly-app/src/dto/act.rs` (full file read) — `ActDto` (already has `handover_date_utc`, confirmed exact match to CONTEXT.md's citation), `ActReturnDto` :126 (confirmed exact — no giver/receiver fields, Pitfall 1 basis), `ActReturnItemDto` :149 (confirmed exact), `ActUpdateDto` :228 (confirmed exact), `ActUpdateItemDto` :275 (confirmed exact), `ActItemDto` :94-116 (confirmed no location field, Pitfall 2 basis)
- `crates/trackly-app/src/tauri_cmds/acts.rs` (full command list read) — `build_acts_return`/`build_acts_update`/`build_acts_delete` patterns
- `crates/trackly-app/src/http/acts.rs` — router pattern (:293-312), confirms `/api/v1/acts_update` route to mirror
- `ui/src/lib/api/acts.ts` — client shape (`acts.update`, `acts.doReturn`)
- `ui/src/features/acts/ReturnModal.svelte` (full file read) — prefill from `outstanding_device_ids` (confirmed exact :47-48), giver/receiver swap (confirmed exact :59-64), `handleSubmit`/payload (confirmed exact :104-118, confirms payload NEVER includes giver/receiver — Pitfall 1 direct evidence)
- `ui/src/features/acts/ReturnItemsTable.svelte`, `ui/src/features/acts/returnPayload.ts` (full file reads) — row shape, `buildReturnItems` per-row-split/coalesce logic (directly reusable for edit submission)
- `ui/src/features/acts/ActDetail.svelte` (full file read) — edit-button gate at line 70 (confirmed exact match to CONTEXT.md's citation)
- `ui/src/features/acts/ActListRow.svelte` (full file read) — confirms `act.handover_date_utc` ALREADY used for date/sort display (Phase 19 already fixed this — D-06 is free once D-05's write-site changes)
- `ui/src/features/acts/ActsPage.svelte` (full file read) — `handleReturn` :136, `handleEdit` :145, `handleEditSaved` :150, `handleReturnSuccess` :169, tab filter `act_type` :44 (all confirmed exact matches to CONTEXT.md's citations)
- `ui/src/features/acts/ActFormBody.svelte` :1-100, :120 — `todayISO`/`unixToIso`/`isoToUnix` date-picker pattern (directly reusable for «Дата возврата»)
- `ui/src/features/acts/ActFormModal.svelte` — `mode`/`initialAct` prop pattern (directly reusable for `ReturnModal`'s edit mode)
- `crates/trackly-app/tests/acts_update.rs` (function list read) — existing Phase-19 test conventions to mirror in a new `acts_update_return.rs`
- `crates/trackly-app/tests/role_endpoint_matrix.rs` (grep) — existing `acts_update` RBAC case (~line 1415-1430) to mirror
- `crates/trackly-app/tests/acts_date_source.rs` (function list read) — existing Phase-19 test file already asserting sort-by-`handover_date_utc`
- `migrations/*.sql` directory listing + `grep -l handover_date_utc` — confirms only V015 touches this column, last migration V033, next free version V034
- `migrations/V004*.sql` — confirms `giver_name`/`receiver_name` NOT NULL, `archived INTEGER NOT NULL DEFAULT 0` (bare boolean, no date column — D-07 basis)
- `.planning/phases/22-return-act-edit/22-CONTEXT.md`, `.planning/phases/19-acts-date-edit/19-CONTEXT.md`, `.planning/phases/19-acts-date-edit/19-RESEARCH.md` — locked decisions, precedent pattern
- `.planning/REQUIREMENTS.md`, `.planning/STATE.md`, `.planning/config.json` — ACT-03 text, milestone status, `nyquist_validation: true`, `security_enforcement: true`

### Secondary (MEDIUM confidence)
None — no external web research necessary; every claim is grounded in a direct repository read this session.

### Tertiary (LOW confidence)
None.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — zero new dependencies; one new migration (V034) with verified version number and verified no prior migration conflict
- Architecture: HIGH — every pattern cited is copy-adaptable from working, already-tested code in this exact repo (`update`/`do_return`/`delete_soft`); the one genuinely new piece (D-11's 3-field snapshot compare) is derived directly from reading the ONLY two code paths that can mutate a device's status/location/condition (`act_service.rs` mutation helpers + `DeviceService::update`)
- Pitfalls: HIGH — Pitfall 1 (giver/receiver silent-drop) is a directly-observed code fact (not speculation) with byte-exact line citations proving the payload never carries these fields; Pitfall 2 (missing location field) is a directly-observed DTO/SQL gap
- Line-number discrepancies vs. CONTEXT.md: two minor drifts found and corrected — `ActRow.handover_date_utc` is at `domain/acts.rs:155` (CONTEXT.md cited :141); `delete_soft`'s `ActType::Return` branch starts at `act_service.rs:1811` (CONTEXT.md's ":1746+" points to the top of the enclosing `match` block, not the Return arm itself). All other citations in CONTEXT.md were verified exact.

**Research date:** 2026-07-12
**Valid until:** No expiry driver — this is a snapshot of the current repo's own code, not a third-party API; valid until the underlying files change (effectively until this phase is implemented)
