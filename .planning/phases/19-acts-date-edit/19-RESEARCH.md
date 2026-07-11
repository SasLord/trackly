# Phase 19: Акты — дата и редактирование - Research

**Researched:** 2026-07-11
**Domain:** Rust/axum/rusqlite backend service layer (act lifecycle) + Svelte 5 frontend form reuse
**Confidence:** HIGH (all findings verified directly against this repo's code — no external library research needed; this phase is 100% internal, no new dependencies)

## Summary

This phase has no external-library risk at all — every finding below comes from reading the actual `trackly` codebase, not from training-data assumptions. ACT-01 is a narrow, low-risk display/sort/render fix: `ActDto` does not currently expose `handover_date_utc` to the frontend at all (only `created_at_utc`/`updated_at_utc`), and six call sites (2 SQL `ORDER BY`, 2 HTML-render date fields — act block + parent block, 2 Svelte date derivations) read `created_at_utc` where they must read `handover_date_utc` instead. No migration is needed — the column has existed since V015.

ACT-02 is the real work: there is no `update`/`patch` path for acts anywhere in the stack (confirmed — grep found zero `fn update`/`fn patch` on `ActService`, `ActRepository`, or `SqliteActRepository`). It must be built from scratch, but the codebase already contains every primitive needed, cleanly factored and directly reusable:
- **Optimistic-lock CAS pattern** — copy `SqliteActRepository::soft_delete_in_tx`'s `UPDATE ... WHERE id=? AND version=?` + `affected==0` → distinguish `NotFound` vs `OptimisticLockMismatch` — already used by `delete_soft` end-to-end (Tauri/HTTP/UI already handle `OptimisticLockMismatch` as HTTP 409).
- **Delta reconciliation "restore removed device to prior state"** — do NOT reinvent this. `audit_log.select_device_mutations_for_act(tx, act_id)` plus `SqliteDeviceRepository::restore_from_snapshot_in_tx` is the exact mechanism `delete_soft`'s cascade-undo already uses. For a *single-device* removal (not a full act delete), the correct query is "most recent `before_json` for this specific `(act_id, device_id)` pair" — see Architecture Patterns below for why "most recent," not "first."
- **Delta reconciliation "add device like create"** — copy the device-loop body of `ActService::create` (status-guard on `на_складе`, transition to `в_работе` + location, per-device audit row with `payload_json: {"act_id":..,"kind":"handover"}`).
- **D-08 "already returned" guard** — do NOT write new SQL for this. `populate_outstanding_device_ids`'s `EXCEPT` query (device_ids in `act_items` minus device_ids consumed by active return-acts) is exactly the "is this device still free to edit" predicate. A device NOT in that outstanding set is bound to a completed return and must be protected from removal/replacement; header edits stay unrestricted per D-05.

**Primary recommendation:** Add `handover_date_utc` to `ActDto`/`act_dto_from_row` and switch the 6 read-side call sites (ACT-01); add `ActService::update` following the exact transactional/audit/CAS conventions of `create`/`do_return`/`delete_soft` (ACT-02), reusing `populate_outstanding_device_ids`'s query for D-08 and `select_device_mutations_for_act` for D-06 restore. No new crates, no schema migration.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Date-source switch (ACT-01: list/detail/PDF/sort) | API / Backend (`ActService`, `SqliteActRepository`, DTO) | Browser/Client (Svelte date derivations) | Backend is source of truth for `handover_date_utc`; DTO must carry it before any frontend fix is possible. SQL `ORDER BY` lives in the repo layer. |
| Act update — header fields (№, dates, giver/receiver) | API / Backend (`ActService::update`) | — | Single-writer discipline (CLAUDE.md) — all mutation goes through the writer task; no direct SQLite access from any other tier. |
| Act update — position delta reconciliation (device side-effects) | API / Backend (`ActService::update`, `SqliteDeviceRepository`, `SqliteAuditLogRepository`) | Database (transactional guarantee) | Device state transitions + audit trail must be atomic with the act row's own update — this is exactly why `create`/`do_return`/`delete_soft` are single `WriterHandle::execute` closures; `update` must follow the same shape. |
| Optimistic concurrency (`version` CAS) | Database (SQL `WHERE version=?`) | API / Backend (error mapping to `OptimisticLockMismatch`) | Enforced structurally at the SQL layer (single UPDATE with a WHERE-clause guard), not via application-level read-then-write (race-prone). |
| Edit-form UI (prefill, submit) | Browser / Client (Svelte: `ActFormBody`/`ActFormModal`/`ActDetail`/`ActsPage`) | API / Backend (new Tauri command + HTTP route + `acts.ts` client) | Standard vertical-slice pattern already established for create/return/delete in this codebase (`build_*` helper shared by Tauri + axum). |
| D-08 validation (can't edit devices bound to a completed return) | API / Backend (`ActService::update`, reusing `populate_outstanding_device_ids` query) | — | Must be enforced server-side (authoritative), not just UI-disabled — same principle as every other act invariant in this codebase (status guards, quantity bounds). |

## Standard Stack

No new packages for this phase — 100% internal code change (Rust service/DTO/repo layer + Svelte components). Every library involved is already pinned in the workspace `Cargo.toml`/`ui/package.json` and unchanged by this work:

| Component | Version (workspace-pinned) | Role in this phase |
|-----------|----------------------------|---------------------|
| `rusqlite` | `0.38` (bundled) | `UPDATE ... WHERE version=?` CAS for act update; `restore_from_snapshot_in_tx` reuse |
| `refinery` | `0.9` | Not needed — no schema migration (see below) |
| `tauri-specta` | `=2.0.0-rc.21` / `specta =2.0.0-rc.22` | New `acts_update` Tauri command auto-exports to `ui/src/bindings.ts` via existing `export_bindings.rs` test |
| `axum` | `0.8` | New `/api/v1/acts_update` route, same router pattern as `acts_delete` |
| `time` | `0.3` | `format_ru_date`/`format_iso_date` — switch input from `created_at_utc` to `handover_date_utc`, no API change |
| Svelte | `5.x` (runes) | Edit-mode variant of `ActFormBody`/`ActFormModal` |

**No migration required.** `acts.handover_date_utc` (V015) and `acts.version` (base schema) already exist and are already read/written correctly by `create`/`do_return`/`delete_soft`. Confirmed via `grep` across `migrations/V0*.sql` — the last migration is `V033__org_settings_requisites.sql`, unrelated to acts.

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Reuse `ActFormBody`/`ActFormModal` in edit mode (add `mode`/`initialAct` props) | Build a separate `ActEditModal`/`ActEditFormBody` component | CONTEXT.md leaves this to planner's discretion but recommends reuse "для консистентности" (D-08.5 discretion note). Given `ActFormBody`'s device-picker already filters `status_id=1` (на_складе) — which is exactly the set new positions must be drawn from — reuse is low-risk. The only wrinkle: pre-filled existing-position rows must NOT go through the on-warehouse picker path (they're already `в_работе`, so a live re-search would never find them) — they're injected directly as `FormItemRow` state, bypassing `fetchGroups`. |
| Domain-level `ActPatch` struct (`crates/trackly-core/src/domain/acts.rs:110`, currently **unused** anywhere in the codebase) | Design a brand-new `ActUpdateDto` | `ActPatch` predates this phase (comment: "used by Phase 7 admin UI; minimal usage in Phase 3" — never actually wired up) and lacks `handover_date_utc`, `number`, and any position/items field. It's a candidate starting point for the header-only fields but must be extended (or replaced) to carry `items: Vec<...>` for D-06 delta reconciliation and `expected_version: i64` for CAS. Confirm with planner whether to extend `ActPatch` or introduce a new `ActUpdateDto` in `dto/act.rs` (existing convention: domain types stay serde-free, DTOs live in `trackly-app::dto`). |
| Full delta reconciliation (chosen, D-06) | "Пересоздание (delete+create)" — discussed and rejected per DISCUSSION-LOG.md | Rejected by user because it changes number/id and is wasteful for the common case (header-only edits). Delta reconciliation is the correct, user-approved approach. |

## Package Legitimacy Audit

Not applicable — this phase introduces zero new external packages (Rust crates or npm packages). All work is new functions/routes/components built on already-vetted, already-in-use dependencies.

## Architecture Patterns

### System Architecture Diagram (ACT-02 update flow)

```
Svelte UI (ActDetail "Редактировать" → ActsPage.onEdit → ActFormModal[mode=edit])
   │  prefill: acts.get(id) → ActDto (header fields + items[] + version)
   │  user edits header fields freely (D-05) and/or adds/removes position rows
   │  submit → ActUpdateDto { id, expected_version, header fields, items: [{device_id, ...}] }
   ▼
Tauri invoke `acts_update`  ──┐
                                ├──► build_acts_update(ctx, caller, payload)   [thin, shared]
axum POST /api/v1/acts_update ─┘        │  authorize(caller, &Action::MutateActs)
                                         ▼
                              ActService::update(id, payload)
                                         │
                    WriterHandle::execute(closure)  ← SINGLE writer tx (BEGIN IMMEDIATE)
                                         │
      ┌──────────────────────────────────┼───────────────────────────────────┐
      │ 1. fetch_full_in_tx(id) → act    │ 2. CAS: version must match        │
      │    (NotFound if missing/deleted) │    (else OptimisticLockMismatch)  │
      └──────────────────────────────────┼───────────────────────────────────┘
                                         ▼
                          guard: act_type must be Handover
                          (return-acts: reject — button already disabled client-side,
                           but server MUST enforce it too)
                                         ▼
                d_new = new device_ids (submitted)     d_old = current act_items.device_id
                removed = d_old − d_new                added = d_new − d_old
                unchanged = d_old ∩ d_new
                                         ▼
        D-08 guard: for each id in `removed` (or replaced), check against
        outstanding_device_ids (populate_outstanding_device_ids query) —
        if id is NOT outstanding (i.e. already consumed by an active return),
        reject the whole update with AppError::Conflict BEFORE any mutation.
                                         ▼
        For `removed`: find MOST RECENT audit_log row for
        (entity_type='device', entity_id=id, act_id=this) →
        restore_from_snapshot_in_tx(before_json) → audit 'custom:update_remove'
                                         ▼
        For `added`: status-guard на_складе → update_status_and_location_in_tx
        (в_работе + act's location) → INSERT act_items → audit 'update'
        with payload_json {"act_id":id,"kind":"handover"}  (same shape as create,
        so a LATER full act delete_soft's undo cascade still finds & unwinds it)
                                         ▼
        UPDATE acts SET <header fields>, version=version+1, updated_at_utc=now
                        WHERE id=? AND version=?  (CAS — same statement covers
                        the header write AND the lock check)
                                         ▼
                    audit_log: action='update', entity_type='act'
                                         ▼
                                  tx.commit()
                                         ▼
                          return self.get(id).await  (fresh ActDto)
```

### Recommended Project Structure (files touched, no new modules)

```
crates/trackly-core/src/domain/acts.rs      # extend/replace ActPatch with items + expected_version
crates/trackly-core/src/ports/acts.rs       # optionally add `update` to the ActRepository trait
                                             #   (or keep as *_in_tx helper on SqliteActRepository,
                                             #   matching how create/do_return/delete_soft are NOT
                                             #   trait methods — they're orchestrated by ActService)
crates/trackly-infra/src/repos/acts_sqlite.rs   # update_act_header_in_tx (CAS UPDATE) + SELECT_ACTS
                                                 #   ORDER BY fix (created_at_utc → handover_date_utc)
crates/trackly-infra/src/repos/audit_log_sqlite.rs  # new: select_latest_device_mutation(tx, act_id, device_id)
crates/trackly-app/src/dto/act.rs           # ActDto += handover_date_utc; new ActUpdateDto/ActUpdateItemDto
crates/trackly-app/src/services/act_service.rs  # ActService::update; render_pdf/render_acceptance_pdf
                                                 #   date-source switch; act_dto_from_row += handover_date_utc
crates/trackly-app/src/tauri_cmds/acts.rs   # build_acts_update + #[tauri::command] acts_update
crates/trackly-app/src/http/acts.rs         # UpdatePayload + handler_update + router() entry
ui/src/lib/api/acts.ts                      # acts.update(...)
ui/src/features/acts/ActFormBody.svelte     # mode: 'create' | 'edit' prop; prefill; branch submit call
ui/src/features/acts/ActFormModal.svelte    # mode/initialAct props, title switch
ui/src/features/acts/ActDetail.svelte       # headerDate from handover_date_utc; wire onEdit call-site
ui/src/features/acts/ActListRow.svelte      # dateLabel from handover_date_utc
ui/src/features/acts/ActsPage.svelte        # editModalOpen/editTargetAct state + onEdit handler
```

### Pattern 1: Optimistic-lock CAS UPDATE (copy from `soft_delete_in_tx`)

**What:** A single `UPDATE ... SET version = version + 1, ... WHERE id = ? AND version = ? AND deleted_at_utc IS NULL` statement. If `affected == 0`, a follow-up `SELECT version` distinguishes "row doesn't exist / already deleted" (→ `AppError::NotFound`) from "version mismatch" (→ `AppError::OptimisticLockMismatch`).

**When to use:** Every mutating path on `acts` that accepts a client-supplied `version`. This is the ONLY optimistic-concurrency mechanism in the codebase — no other pattern exists (no `rowversion`/ETag/timestamp-based alternative).

**Example (existing code — copy this shape for `update`):**
```rust
// Source: crates/trackly-infra/src/repos/acts_sqlite.rs:325-362 (soft_delete_in_tx)
pub fn soft_delete_in_tx(
    &self,
    tx: &Transaction<'_>,
    id: i64,
    version: i64,
    now_utc: i64,
) -> Result<(), AppError> {
    let affected = tx
        .execute(
            "UPDATE acts SET deleted_at_utc = ?1, version = version + 1, \
             updated_at_utc = ?1 \
             WHERE id = ?2 AND version = ?3 AND deleted_at_utc IS NULL",
            params![now_utc, id, version],
        )
        .map_err(map_rusqlite)?;

    if affected == 0 {
        let actual: Option<i64> = tx
            .query_row("SELECT version FROM acts WHERE id = ?1", params![id], |r| r.get(0))
            .optional()
            .map_err(map_rusqlite)?;
        return match actual {
            None => Err(AppError::NotFound { entity: "act", id }),
            Some(actual) => Err(AppError::OptimisticLockMismatch {
                entity: "act", id, expected: version, actual,
            }),
        };
    }
    tx.execute("DELETE FROM act_items WHERE act_id = ?1", params![id]).map_err(map_rusqlite)?;
    Ok(())
}
```
`AppError::OptimisticLockMismatch` already maps to HTTP 409 (`error_axum.rs:35`) end-to-end — no new error-mapping work needed.

### Pattern 2: Device delta reconciliation — reuse `select_device_mutations_for_act` + `restore_from_snapshot_in_tx`

**What:** The undo path (`delete_soft` → `undo_device_mutations_for_act`) already solves "how do I put a device back exactly how it was before this act touched it." For a single removed device during an *edit* (not a full act delete), you need the analogous single-device query.

**Critical nuance — MOST RECENT row, not FIRST:** `select_device_mutations_for_act` returns ALL `(device_id, before_json)` pairs tagged with this `act_id` in chronological order, across every past edit of this same act (create + any prior edit-adds). If a device was added, later removed (in an earlier edit), then re-added, there will be multiple rows for the same `device_id`. To restore it correctly on a NEW removal, you must take the **last** (most recent) row for that `device_id` — i.e. its state immediately before the most recent time this act touched it — not the original creation-time snapshot. `undo_device_mutations_for_act` gets this right implicitly because it iterates `.rev()` over the WHOLE list (full-act undo, LIFO across all devices); a single-device removal needs the equivalent single-device query:

```sql
-- New helper needed on SqliteAuditLogRepository — no equivalent exists yet.
SELECT before_json FROM audit_log
 WHERE entity_type = 'device' AND entity_id = ?device_id
   AND json_extract(payload_json, '$.act_id') = ?act_id
   AND before_json IS NOT NULL
 ORDER BY created_at_utc DESC, id DESC
 LIMIT 1
```

**Example (existing code to model the new helper after):**
```rust
// Source: crates/trackly-infra/src/repos/audit_log_sqlite.rs:66-90
pub fn select_device_mutations_for_act(
    &self, tx: &Transaction<'_>, act_id: i64,
) -> Result<Vec<(i64, String)>, AppError> {
    // SELECT entity_id, before_json FROM audit_log
    //  WHERE entity_type='device' AND json_extract(payload_json,'$.act_id')=?1
    //    AND before_json IS NOT NULL
    //  ORDER BY created_at_utc ASC, id ASC
}
```
Then restore via the existing `SqliteDeviceRepository::restore_from_snapshot_in_tx(tx, device_id, &snapshot, now)` (already used by undo — no changes needed there), and write a NEW audit row (`action: "custom:update_remove"` or similar — pick a name distinct from `"custom:undo"` so audit history is legible, but it must still carry `before_json`/`after_json` so a LATER full-act delete can still find and unwind it via `select_device_mutations_for_act`).

### Pattern 3: Device "add" — copy `create`'s per-device loop body

**What:** `create`'s device loop (`act_service.rs:429-471`) does: snapshot before → `update_status_and_location_in_tx(dev_id, in_work_status_id, resolved_location_id, now)` → snapshot after → `audit_repo.insert(action:"update", payload_json:{"act_id":act_id,"kind":"handover"})`. For added positions during an edit, do exactly this — same status check on `на_складе` (`item.device_ids` canonical path at `act_service.rs:344-359` — validate each device is `on_warehouse_status_id` before transitioning), same location resolution (reuse the act's own `location_id`, resolved once).

**Recommendation on legacy clone support:** `create` supports two paths — canonical `device_ids[]` (existing warehouse group, no cloning) and legacy `device_id + quantity` (clone-on-handover). For a brand-new `update` path, only the canonical `device_ids[]` path is worth supporting — clone-on-handover is a legacy compat shim for old create-time clients; `update` has no such backward-compat burden. Flag this as a scope decision for the planner (recommend: canonical-only, quantity always 1 per new position, matching the current data model where `act_items` post-G-12 is 1-row-per-device).

### Pattern 4: D-08 "already returned" guard — reuse `populate_outstanding_device_ids`'s query verbatim

**What:** `populate_outstanding_device_ids` (`act_service.rs:1846-1873`) computes exactly the predicate D-08 needs: which of this handover-act's device_ids have NOT yet been consumed by an active return-act.
```sql
-- Source: crates/trackly-app/src/services/act_service.rs:1851-1858
SELECT device_id FROM act_items WHERE act_id = ?1
  EXCEPT
SELECT rai.device_id FROM act_items rai
  JOIN acts ra ON ra.id = rai.act_id
 WHERE ra.parent_act_id = ?1 AND ra.deleted_at_utc IS NULL
```
Run this INSIDE the update transaction (as a helper taking `&Transaction` instead of `&Connection` — a `_in_tx` twin, same SQL) to get the outstanding set. For every device_id being **removed** (or effectively replaced) in the submitted diff: if it is NOT in the outstanding set (i.e. it has already been consumed by a completed/active return), reject the ENTIRE update with `AppError::Conflict` before any mutation runs (validate-then-commit, same style as `validate_return`/`validate_create` — validate outside or at the very top of the writer closure, before any `tx.execute` side effects). Header-only fields (D-05) must NOT be gated by this check — a caller submitting `items` unchanged (same device_id set) skips this guard entirely.

### Anti-Patterns to Avoid

- **Reinventing the delta-diff mechanism as a bespoke SQL query.** The `EXCEPT`-based outstanding-device query and the `select_device_mutations_for_act` audit-trail read already exist and are unit-tested (`acts_undo.rs`) — reuse them; do not write parallel logic that could drift out of sync with `delete_soft`'s semantics of "prior state."
- **Read-then-write version check in application code** (`if act.version == expected { UPDATE ... }` as two separate statements). This is a TOCTOU race under concurrent writers even with the single-writer task, because the writer task processes a *queue* — two `update` calls could both pass the read-check before either commits if the check and the write aren't the same SQL statement. Always fold the check into the `WHERE version = ?` clause of the UPDATE itself (Pattern 1).
- **Allowing act-number edits to bypass the `create`-path uniqueness check.** D-04 explicitly lists `№` as an editable header field. If the update path lets a client set an arbitrary new `number`, it MUST re-run the same uniqueness check `create` does at `act_service.rs:247-264` (`SELECT EXISTS(... WHERE number=?)`, including soft-deleted rows per D-Soft-vs-Hard-Acts-01) and audit the override the same way (`custom:act_number_override`). Skipping this reintroduces a duplicate-number bug class this codebase has already fixed once for `create`.
- **Applying the D-06 device-add loop to devices not in `на_складе` status.** Silently accepting an already-`в_работе` device into a *different* act's positions would corrupt the invariant "one device is only ever in one act's live position set at a time" (which `populate_outstanding_device_ids` and `do_return`'s own-device guards depend on).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Optimistic concurrency check | A separate `SELECT version` then compare in Rust | The single CAS `UPDATE ... WHERE version=?` statement (Pattern 1) | Structural race-freedom, not just convention — matches `delete_soft`'s existing, reviewed pattern. |
| "Restore device to prior state" | A new bespoke query walking `devices` history or a separate snapshot table | `audit_log.before_json` + `restore_from_snapshot_in_tx` (Pattern 2) | This IS the existing undo mechanism (D-Undo-01) — `audit_log` is already the append-only source of truth for device state history in this codebase; a second mechanism would create two competing sources of truth. |
| "Is this device still editable" (D-08) | A new join/subquery from scratch | `populate_outstanding_device_ids`'s `EXCEPT` query, factored into a `_in_tx` twin (Pattern 4) | Already correct and tested against the exact same act/return/act_items relationship the update path must respect. |
| Act number uniqueness on rename | Trusting client-submitted `number` blindly | `create`'s existing `SELECT EXISTS(SELECT 1 FROM acts WHERE number=?)` check (including soft-deleted, D-Soft-vs-Hard-Acts-01) | Prevents reintroducing a duplicate-act-number bug class. |

**Key insight:** Every hard part of ACT-02 (concurrency, delta reconciliation, "is this touchable" guards) already has a canonical, tested implementation elsewhere in `act_service.rs`/`acts_sqlite.rs`/`audit_log_sqlite.rs` for the sibling operations `create`/`do_return`/`delete_soft`. The planner's job is almost entirely "extract + adapt," not "design from zero."

## Runtime State Inventory

Not applicable — this is not a rename/refactor/migration phase. No renamed identifiers, no data migration.

## Common Pitfalls

### Pitfall 1: `ActDto` silently lacks `handover_date_utc` — an easy half-fix
**What goes wrong:** A developer switches `format_iso_date(act.created_at_utc)` → `format_iso_date(act.handover_date_utc)` inside `render_pdf`/`render_acceptance_pdf` (Rust-side, where `ActRow.handover_date_utc` already exists) but forgets that the **frontend-facing** `ActDto` struct (`dto/act.rs:46-81`) has no `handover_date_utc` field at all — so `ActListRow.svelte`/`ActDetail.svelte` literally cannot read it; there's no compile error because they're reading `created_at_utc`, which still exists (unchanged) on the DTO. The fix must add the field to `ActDto` + `act_dto_from_row` FIRST, then regenerate `bindings.ts` (`cargo test -p trackly-app --test export_bindings`, which is also `pnpm prebuild`), THEN switch the two Svelte derivations.
**Why it happens:** The backend (`ActRow`) and the wire DTO (`ActDto`) look similar but are separate structs by design (`dto/act.rs` module doc) — it's easy to fix one and assume the other is already in sync.
**How to avoid:** Grep `ActDto` in `dto/act.rs` for `created_at_utc` before touching anything — confirm `handover_date_utc` is present as a sibling field before writing any frontend code.
**Warning signs:** `svelte-check` won't catch this (TypeScript will just see `act.handover_date_utc` as `undefined`, not a type error, if the bindings.ts generator ran with stale types before the DTO change was compiled). Always regenerate bindings AFTER the Rust DTO change compiles.

### Pitfall 2: Restoring a removed device from the WRONG audit snapshot (first vs. most-recent)
**What goes wrong:** Copy-pasting `undo_device_mutations_for_act`'s bulk query and taking the first result for a given `device_id` restores the device to its state before the ORIGINAL act creation, not its state before the most recent edit that touched it. On a 2nd-generation edit (remove→re-add→remove again), this loses intermediate state changes (e.g., a location change made in between).
**Why it happens:** `select_device_mutations_for_act` returns rows in `created_at_utc ASC` order (oldest first) because that's what full-act LIFO undo needs (`.rev()`'d by the caller). A naive single-device lookup that just filters by `device_id` and takes `.first()` gets this backwards.
**How to avoid:** Always order `DESC` and `LIMIT 1` (or reuse the bulk query, filter by `device_id` client-side, and take the LAST matching entry) when restoring a single device during an in-place edit. See Pattern 2 above.
**Warning signs:** A test that edits an act twice (add device → later edit removes it) and checks the device lands back at its state *immediately before the second add*, not its state before the very first handover, will catch this if the order is wrong.

### Pitfall 3: Forgetting the D-08 return-binding guard is server-side, not just UI-side
**What goes wrong:** The UI's "Редактировать" button is only disabled for return-acts (D-07). Nothing in the client prevents a user from submitting a positions-diff on a handover act that removes a device already consumed by a completed return — the client has no visibility into `outstanding_device_ids` filtering logic unless it's explicitly wired into the edit form's remove-row handler. Even if the UI is wired correctly, a raw HTTP POST to `/api/v1/acts_update` bypasses any client-side guard entirely.
**Why it happens:** Every other act invariant in this codebase (status guards, quantity bounds, dedup) is enforced INSIDE the writer transaction, not just in the UI — this is a stack-wide convention (CLAUDE.md: single-writer discipline, "no direct writes from multiple processes"), so D-08 must follow it too, but it's tempting to treat it as "just disable the row's delete button" since that's visually sufficient.
**How to avoid:** Implement Pattern 4 (server-side outstanding-set check) as a hard `AppError::Conflict` regardless of what the client sends. Add an HTTP-level integration test that posts a raw payload attempting to remove an already-returned device and asserts 409, independent of any UI test.
**Warning signs:** A plan that only lists "disable remove button for returned items in ActFormItemsTable" as the D-08 task, with no corresponding backend validation task, has under-scoped this requirement.

### Pitfall 4: Ambiguous scope of "комплектация/технические характеристики" as an edit-form header field
**What goes wrong:** CONTEXT.md's D-04 lists "комплектация/технические характеристики" under "Шапка" (header) editable fields. But the schema has NO act-level (header) column for this — `Комплектация` maps to `act_items.complectation_at_time` (a PER-ITEM snapshot column) and `Технические характеристики` maps to `item.specs`, which the code comments explicitly document as "живое значение `devices.notes`, НЕ снимок" (`dto/act.rs:103-106` — i.e., it's a live device attribute read at render time, not stored on the act at all). Neither `ActFormBody` (create form) nor `ActItemsTable`/`ActFormItemsTable` currently expose ANY input for condition/kit/specs — there is no existing UI precedent to copy for "editing" these fields; they're currently set automatically from `source_before.state`/`source_before.kit` at creation time (`act_service.rs:415-416`), never user-entered.
**Why it happens:** CONTEXT.md's field list appears to be inherited verbatim from the discussion log's *rejected* "header-only" option table (`19-DISCUSSION-LOG.md` line 28), which predates the "Шапка + позиции" decision and may not have been re-scrutinized against the actual schema/UI once positions-editing was added to scope.
**How to avoid:** This needs an explicit planner decision (or a return to the user) before implementation: (a) treat "комплектация" as `act_items.complectation_at_time` — add a free-text input per RETAINED position row in edit mode (schema-consistent, no migration); (b) treat "технические характеристики" as read-only / out-of-scope for this phase since it's a live `devices.notes` field, not an act-owned property, and editing it here would mean mutating device data as an act-edit side effect (a materially bigger, security-relevant surface not covered by D-05/D-06's device-side-effect discussion, which only discusses status/location, not device.notes content).
**Warning signs:** A plan phase that adds "технические характеристики" as an editable act field without a corresponding device.notes write path, RBAC check, or explicit user sign-off has silently expanded scope beyond what D-05/D-06 analyzed.

### Pitfall 5: `list()`/`search()` never populate `outstanding_device_ids`
**What goes wrong:** Only `ActService::get(id)` calls `populate_outstanding_device_ids` — `list()` and `search()` always pass empty defaults (`act_dto_from_row(row, items, Vec::new())`, and `items` from `load_items_for_act` alone never has `outstanding_device_ids` filled either). If the edit form is ever prefilled from a `list()`/`search()` result instead of a fresh `acts.get(id)` call, D-08's client-side hints (if any are added to the UI) would be silently wrong (always empty → "everything looks removable").
**Why it happens:** This was an intentional Phase 03.1 design choice (list/search are read-heavy hot paths; only the single-act detail view needs the expensive EXCEPT computation) — but it's a trap for anyone assuming `ActDto.items[].outstanding_device_ids` is always populated.
**How to avoid:** The edit form MUST fetch via `acts.get(id)` (already what `ActsPage`'s `selectedAct` effect does) before populating the edit form's initial state — never reuse a `list()` row's `ActDto` directly for prefill.
**Warning signs:** None visible in tests unless a dedicated test asserts `outstanding_device_ids` is non-trivially populated on the edit-prefill path specifically (not just on `get()`).

## Code Examples

### ACT-01: the exact 6 call sites needing `created_at_utc` → `handover_date_utc`

```rust
// Source: crates/trackly-infra/src/repos/acts_sqlite.rs:537 (list())
ORDER BY a.created_at_utc DESC, a.id DESC   // → a.handover_date_utc DESC, a.id DESC

// Source: crates/trackly-infra/src/repos/acts_sqlite.rs:295 (search_acts())
ORDER BY a.created_at_utc DESC, a.id DESC   // → a.handover_date_utc DESC, a.id DESC
```
```rust
// Source: crates/trackly-app/src/services/act_service.rs:1450-1451 (render_pdf ctx)
"date": format_iso_date(act.created_at_utc),
"date_human": format_ru_date(act.created_at_utc),
// →
"date": format_iso_date(act.handover_date_utc),
"date_human": format_ru_date(act.handover_date_utc),

// Source: crates/trackly-app/src/services/act_service.rs:1406-1407 (parent_block)
"date_human": format_ru_date(parent.created_at_utc),
"date": format_iso_date(parent.created_at_utc),
// → parent.handover_date_utc (both lines)
```
```typescript
// Source: ui/src/features/acts/ActListRow.svelte:36
const dateLabel = $derived(formatDate(act.created_at_utc));
// → act.handover_date_utc (requires ActDto.handover_date_utc to exist first — Pitfall 1)

// Source: ui/src/features/acts/ActDetail.svelte:43
const headerDate = $derived(act ? formatDate(act.created_at_utc) : null);
// → act.handover_date_utc
```
**NOT in scope for ACT-01 (verify, don't touch):** `render_acceptance_pdf` takes `date_utc: i64` as an explicit caller-supplied parameter (not derived from any act row) — it's a different document type (device acceptance-to-warehouse receipt, unrelated to a handover act's `handover_date_utc`). Leave it untouched; D-01's scope is "везде" within the ACT entity, not the acceptance-receipt document.

### ACT-02: acts_sqlite.rs `insert_act_in_tx` shows the exact column list `update` must mirror

```rust
// Source: crates/trackly-infra/src/repos/acts_sqlite.rs:92-116
pub fn insert_act_in_tx(&self, tx: &Transaction<'_>, new: &ActRow) -> Result<i64, AppError> {
    tx.execute(
        "INSERT INTO acts \
         (number, sub_number, parent_act_id, act_type, giver_name, \
          receiver_name, location_id, notes, deadline_utc, archived, \
          created_at_utc, updated_at_utc, version, handover_date_utc) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?11, 1, ?12)",
        params![ /* ... */ ],
    )
    // header UPDATE for `update` must touch: giver_name, receiver_name, location_id,
    // notes, deadline_utc, handover_date_utc, number (with uniqueness re-check),
    // plus version+1 / updated_at_utc — NOT sub_number/parent_act_id/act_type
    // (immutable identity fields) and NOT created_at_utc (D-02: purely internal).
}
```

## State of the Art

Not applicable in the "external ecosystem changed" sense — nothing here tracks an evolving library API. The one internal "old → new" shift:

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|---------------|--------|
| `created_at_utc` treated as the act's displayed/sorted "Дата" | `handover_date_utc` is the sole source of truth for the displayed/sorted "Дата"; `created_at_utc` becomes purely internal (D-02) | This phase (19) | Every read path (list sort, detail header, PDF/HTML render) must be audited — `created_at_utc` remains in the DB/DTO for internal bookkeeping only, never user-facing as "Дата" again. |

**Deprecated/outdated:** None — no library deprecations involved.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | "комплектация/технические характеристики" in D-04 maps to `act_items.complectation_at_time` (kit) as a per-item edit, and `технические характеристики`/`item.specs` (live `devices.notes`) should likely stay read-only/out-of-scope pending explicit confirmation | Common Pitfalls #4 | If wrong, the plan under-scopes (misses a genuinely-requested device.notes edit path with its own RBAC/audit surface) or over-scopes (builds UI for a field the user didn't actually mean to expose per-position) |
| A2 | `update` should support only the canonical `device_ids[]` positions model (no legacy clone-on-handover quantity>1 path) for new positions added during an edit | Architecture Patterns, Pattern 3 | Low risk — if wrong, simply extends the add-path to also accept the legacy quantity+clone shape, mirroring `create`'s dual-path; no data-model conflict either way |
| A3 | Header-editable `№` (act number) requires re-running `create`'s uniqueness check (including soft-deleted acts) and an audit entry mirroring `custom:act_number_override` | Anti-Patterns, Code Examples | If wrong / skipped, reintroduces the duplicate-act-number bug class `create`'s existing check was built to prevent |

## Open Questions (RESOLVED)

1. **Scope of "комплектация/технические характеристики" editability (D-04)**
   - What we know: Schema has no act-header column for either; both map to per-item columns/live fields (`act_items.complectation_at_time`, `devices.notes` via `item.specs`). No existing UI precedent for editing either.
   - What's unclear: Whether the user's intent in D-04 was per-item editing (schema-consistent, low risk) or something else entirely (act-level free text — would need a new column).
   - Recommendation: Planner should either (a) explicitly scope this as "per-position `condition_at_time`/`complectation_at_time` free-text inputs added to retained rows in edit mode, `specs`/device.notes untouched," documented as a plan-time decision, or (b) flag back to `/gsd-discuss-phase 19` for a quick clarifying pass before planning proceeds. Given `mode: yolo` and `auto_advance: true` in config, recommend (a) with the schema-consistent interpretation, documented plainly in the plan's Decisions section so it's auditable.
   - **RESOLVED (2026-07-11, user confirmation during plan-phase):** Only «комплектация» is editable in Phase 19, per-item via `act_items.complectation_at_time` on retained rows. «Технические характеристики» (`devices.notes` / `item.specs`) are OUT of scope this phase and stay read-only. CONTEXT.md D-04 amended to record this narrowed scope. Encoded in Plans 19-02 (`ActUpdateItemDto`) and 19-05 (UI excludes any specs/device.notes input).

   - **RESOLVED:** Extend `ActPatch` with the missing header fields + `expected_version`; items travel as a DTO-only `ActUpdateDto` field destructured in the service layer (per recommendation). Encoded in Plan 19-02.

2. **`ActPatch` (domain/acts.rs:110) — extend or replace?**
   - What we know: It exists, is currently unused anywhere, lacks `handover_date_utc`/`number`/`items`.
   - What's unclear: Whether extending it (adding the missing fields) or introducing a fresh domain type is cleaner given trackly-core's "domain stays serde-free, DTOs carry serde" convention.
   - Recommendation: Extend `ActPatch` with the missing header fields + `expected_version: i64`, and add a separate `items: Vec<ActUpdateItemDto>`-shaped DTO-only field only in the `trackly-app::dto::act::ActUpdateDto` wrapper (domain layer doesn't need to know about items — the service layer can destructure the DTO into `(ActPatch, Vec<i64> new_device_ids)` before touching the domain type), mirroring how `ActCreateDto`/`ActNew` are already split.

3. **Trait method vs. `_in_tx` helper for `update`**
   - What we know: `create`/`do_return`/`delete_soft`(service)/`delete_soft`(repo, trait) are inconsistent — `delete_soft` IS on the `ActRepository` trait; `create`/`do_return` are NOT (they're orchestrated ad-hoc inside `ActService` using `*_in_tx` helpers on the concrete `SqliteActRepository`).
   - What's unclear: Whether `update`'s header-write belongs on the trait (like `delete_soft`) or as a private `_in_tx` helper (like `insert_act_in_tx`).
   - Recommendation: Given `update`'s device-delta logic (added/removed reconciliation, audit writes, D-08 guard) must live in `ActService` regardless (it needs `devices_repo`/`audit_repo`, not just `acts_repo`), keep the acts-table header UPDATE as a `_in_tx` helper on `SqliteActRepository` (e.g. `update_act_header_in_tx`), consistent with `insert_act_in_tx`'s precedent — do not add `update` to the trait unless a future caller needs it through the trait's `Conn`-generic abstraction (none currently does).

   - **RESOLVED:** Keep the acts-table header UPDATE as a private `update_act_header_in_tx` helper on `SqliteActRepository` (consistent with `insert_act_in_tx`); device-delta orchestration lives in `ActService::update`. Not added to the `ActRepository` trait. Encoded in Plans 19-02/19-03.

## Environment Availability

Skipped — no external tool/service/runtime dependency introduced by this phase (pure in-repo Rust + Svelte code change against an already-running local SQLite file).

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | `cargo test` (rusqlite integration tests against `trackly_infra::test_support::test_writer_and_readers` — tempfile-backed real SQLite, WAL+migrations applied) — this codebase has no frontend unit-test framework (no vitest/jest configured); frontend correctness is covered by `svelte-check` + manual/human-verify checkpoints per existing convention |
| Config file | none — pattern lives in existing test files, e.g. `crates/trackly-app/tests/acts_undo.rs`, `acts_crud.rs`, `acts_returns.rs` |
| Quick run command | `cargo test -p trackly-app --test acts_update` (new dedicated test file, once created) |
| Full suite command | `cargo test --workspace` (respect the project's "one `cargo test` at a time" constraint — do not run concurrent `cargo test` invocations, they contend on the `target/` lock) |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| ACT-01 | List/detail/PDF/sort all key off `handover_date_utc`, not `created_at_utc` | integration | `cargo test -p trackly-app --test acts_display_rule` (extend) or new `acts_date_source.rs` | ❌ Wave 0 (extend existing `acts_display_rule.rs` or add new file) |
| ACT-01 | HTML act render shows `handover_date_utc`-derived date string | integration | `cargo test -p trackly-app --test html_act_render` (extend) | ✅ existing file, extend assertions |
| ACT-02 | Update happy path: header-only edit, device state untouched (D-05) | integration | `cargo test -p trackly-app --test acts_update -- header_only_edit_does_not_touch_devices` | ❌ Wave 0 |
| ACT-02 | Update happy path: add position (device на_складе → в_работе) | integration | `cargo test -p trackly-app --test acts_update -- add_position_transitions_device` | ❌ Wave 0 |
| ACT-02 | Update happy path: remove position restores device to prior state (D-06) | integration | `cargo test -p trackly-app --test acts_update -- remove_position_restores_prior_state` | ❌ Wave 0 |
| ACT-02 | Remove-then-re-add-then-remove restores to MOST RECENT prior state, not original (Pitfall 2) | integration | `cargo test -p trackly-app --test acts_update -- double_edit_restores_most_recent_snapshot` | ❌ Wave 0 |
| ACT-02 | Version mismatch → `OptimisticLockMismatch` (409) | integration + HTTP smoke | `cargo test -p trackly-app --test acts_update -- version_mismatch_returns_conflict` | ❌ Wave 0 |
| ACT-02 | D-08: removing a device already bound to a completed return → rejected | integration | `cargo test -p trackly-app --test acts_update -- reject_removal_of_returned_device` | ❌ Wave 0 |
| ACT-02 | D-08: header edit on an act with an existing return still succeeds freely | integration | `cargo test -p trackly-app --test acts_update -- header_edit_free_even_with_existing_return` | ❌ Wave 0 |
| ACT-02 | D-07: return-act update rejected server-side (not just UI-disabled) | integration | `cargo test -p trackly-app --test acts_update -- reject_update_on_return_act` | ❌ Wave 0 |
| ACT-02 | Act-number edit re-validates uniqueness (A3) | integration | `cargo test -p trackly-app --test acts_update -- number_change_rejects_duplicate` | ❌ Wave 0 |
| ACT-02 | RBAC: `acts_update` gated by `Action::MutateActs` (Employee role rejected) | integration | extend `crates/trackly-app/tests/role_endpoint_matrix.rs` with new case | ❌ Wave 0 (add case to existing file) |
| ACT-02 (UI) | Edit form prefilled from `acts.get(id)`, not a stale `list()` row (Pitfall 5) | manual / human-verify | N/A — UI behavior, checkpoint-gated per `human_verify_mode: end-of-phase` | N/A |

### Sampling Rate
- **Per task commit:** `cargo test -p trackly-app --test acts_update` (once the file exists) plus whichever existing file is being extended for ACT-01 (e.g. `acts_display_rule.rs`, `html_act_render.rs`)
- **Per wave merge:** `cargo test --workspace` (single invocation — no concurrent `cargo test`, per project convention)
- **Phase gate:** Full suite green + `pnpm --dir ui build` (LAN/browser mode serves `ui/dist`, not HMR — per existing dev convention) before `/gsd-verify-work`

### Wave 0 Gaps
- [ ] `crates/trackly-app/tests/acts_update.rs` — new file, covers all ACT-02 rows above
- [ ] Extend `crates/trackly-app/tests/acts_display_rule.rs` or add `acts_date_source.rs` — covers ACT-01 list/sort assertions
- [ ] Extend `crates/trackly-app/tests/html_act_render.rs` — covers ACT-01 PDF/HTML date assertions
- [ ] Extend `crates/trackly-app/tests/role_endpoint_matrix.rs` — add `acts_update` RBAC case (mirrors the existing `acts_delete` case)
- [ ] Regenerate `ui/src/bindings.ts` (`cargo test -p trackly-app --test export_bindings`) once `ActDto.handover_date_utc` + `ActUpdateDto`/`acts_update` exist — extend `export_bindings.rs`'s assertions with `ActUpdateDto`/`acts_update` presence checks, following the exact pattern already used for `acts_create`/`ActCreateDto` (lines 193-242 of that file)

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|----------------|---------|-------------------|
| V2 Authentication | no (unchanged) | Session-based auth already gates every `/api/v1/acts_*` route via `session_identity` |
| V3 Session Management | no (unchanged) | `tower-sessions`, unaffected by this phase |
| V4 Access Control | **yes** | `authorize(caller, &Action::MutateActs)` — same action already used by `create`/`do_return`/`delete_soft`; no new `Action` variant needed. Server-side D-07 (return-acts non-editable) and D-08 (return-bound devices non-removable) are access-control-adjacent business-rule guards that MUST be enforced inside `ActService::update`, not merely in the UI (Common Pitfall #3) |
| V5 Input Validation | **yes** | New `ActUpdateDto`/`ActUpdateItemDto` need the same validation rigor as `ActCreateDto`/`ActReturnDto`: dedup device_ids, quantity bounds, act-number uniqueness (A3), non-empty giver/receiver — mirror `validate_create`/`validate_return`'s existing style (`act_service.rs:115-193`, `515-580`) |
| V6 Cryptography | no | Not applicable — no new crypto surface |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|----------------------|
| Client submits stale `version` expecting last-write-wins | Tampering | CAS `UPDATE ... WHERE version=?` (Pattern 1) — never trust a read-then-write from the client without a version guard baked into the write statement itself |
| Client attempts to remove/replace a device already bound to a completed return via raw HTTP (bypassing UI disable state) | Tampering / Elevation of Privilege (business-rule bypass) | Server-side D-08 guard (Pattern 4) — MUST run inside the same transaction, before any device mutation, regardless of what the UI does |
| Client submits an update to a `return`-type act (D-07: should be non-editable) via raw HTTP, bypassing the UI's disabled button | Tampering | `ActService::update` must check `act.act_type == ActType::Handover` and reject `Return` acts with `AppError::Validation`/`Conflict`, mirroring `do_return`'s own `parent.act_type != ActType::Handover` guard at `act_service.rs:603-608` |
| Client submits a duplicate/colliding act `number` on update | Tampering (data integrity) | Reuse `create`'s uniqueness check (A3) before accepting a number change |
| Employee-role caller attempts `acts_update` directly | Elevation of Privilege | `authorize(caller, &Action::MutateActs)` already excludes Employee per the existing permission matrix (`auth.rs:133` doc comment) — extend `role_endpoint_matrix.rs` with a case proving this holds for the new endpoint too |

## Sources

### Primary (HIGH confidence — all direct repo reads, this session)
- `crates/trackly-app/src/services/act_service.rs` (full file read) — `create`, `do_return`, `delete_soft`, `get`, `list`, `search`, `render_pdf`, `render_acceptance_pdf`, `undo_device_mutations_for_act`, `populate_outstanding_device_ids`
- `crates/trackly-infra/src/repos/acts_sqlite.rs` (full file read) — `SELECT_ACTS`, `insert_act_in_tx`, `soft_delete_in_tx`, `list`, `search_acts`, `from_row`
- `crates/trackly-infra/src/repos/audit_log_sqlite.rs` — `AuditEntry`, `select_device_mutations_for_act`
- `crates/trackly-infra/src/repos/devices_sqlite.rs` — `update_status_and_location_in_tx`, `update_full_in_tx`, `restore_from_snapshot_in_tx`
- `crates/trackly-core/src/domain/acts.rs`, `crates/trackly-core/src/ports/acts.rs` — `ActRow`, `ActPatch` (unused), `ActRepository` trait
- `crates/trackly-app/src/dto/act.rs` — `ActDto`, `ActCreateDto`, `ActReturnDto`, `act_dto_from_row` (confirms `handover_date_utc` missing from `ActDto`)
- `crates/trackly-app/src/tauri_cmds/acts.rs`, `crates/trackly-app/src/http/acts.rs` — `build_*` shared-helper pattern, router wiring
- `crates/trackly-app/src/error_axum.rs` — `OptimisticLockMismatch` → HTTP 409 mapping (already exists)
- `crates/trackly-core/src/auth.rs` — `Action::MutateActs` permission matrix
- `crates/trackly-app/tests/export_bindings.rs`, `acts_undo.rs`, `acts_crud.rs` — test conventions
- `ui/src/features/acts/ActFormBody.svelte`, `ActFormModal.svelte`, `ActFormItemsTable.svelte`, `ActDetail.svelte`, `ActsPage.svelte`, `ActItemsTable.svelte` (full reads) — device-picker `status_id=1` filter, `onEdit` wiring gap, `FormItemRow` shape
- `ui/src/lib/api/acts.ts` — API client shape
- `ui/src/bindings.ts` — confirms `ActDto` current (stale, pre-phase) shape
- `crates/trackly-app/templates/act_handover.html` — confirms `act.date_human`/`act.parent.date_human`/`item.kit`/`item.specs` template placeholders (no template changes needed for ACT-01)
- `crates/trackly-app/src/services/report_service.rs` — confirms reports already filter/sort/group by `a.handover_date_utc` (D-03 consistency claim verified)
- `migrations/V0*.sql` directory listing — confirms no new migration needed (`handover_date_utc` since V015, last migration V033 unrelated)
- `.planning/phases/19-acts-date-edit/19-CONTEXT.md`, `19-DISCUSSION-LOG.md` — locked decisions D-01..D-08, discretion notes
- `.planning/REQUIREMENTS.md`, `.planning/config.json` — ACT-01/ACT-02 text, `nyquist_validation: true`, `security_enforcement: true`

### Secondary (MEDIUM confidence)
None used — no external web research was necessary; every claim in this document is grounded in a direct repository read from this session.

### Tertiary (LOW confidence)
None.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — zero new dependencies; all versions confirmed against workspace `Cargo.toml`
- Architecture: HIGH — every pattern cited is copy-adaptable from working, already-tested code in this exact repo (`create`/`do_return`/`delete_soft`)
- Pitfalls: HIGH — each pitfall is derived from direct code inspection (e.g., `ActDto` field list, `select_device_mutations_for_act`'s ORDER BY, `populate_outstanding_device_ids`'s scope), not speculation
- One open scope ambiguity flagged (Pitfall 4 / Open Question 1) — deliberately NOT resolved unilaterally since it affects requirement interpretation, not implementation technique

**Research date:** 2026-07-11
**Valid until:** No expiry driver — this is a snapshot of the current repo's own code, not a third-party API; valid until the underlying files change (i.e., effectively until this phase is implemented)
