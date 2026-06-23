---
phase: 12-cartridge-request-interconnection
reviewed: 2026-06-24T00:00:00Z
depth: standard
files_reviewed: 19
files_reviewed_list:
  - crates/trackly-app/src/dto/printer.rs
  - crates/trackly-app/src/http/requests.rs
  - crates/trackly-app/src/services/act_service.rs
  - crates/trackly-app/src/services/request_service.rs
  - crates/trackly-app/src/specta_export.rs
  - crates/trackly-app/src/tauri_cmds/requests.rs
  - crates/trackly-app/tests/acts_suggest.rs
  - crates/trackly-app/tests/request_lifecycle.rs
  - crates/trackly-app/tests/role_endpoint_matrix.rs
  - crates/trackly-core/src/auth.rs
  - crates/trackly-core/src/domain/printers.rs
  - crates/trackly-infra/src/repos/printers_sqlite.rs
  - crates/trackly-infra/src/repos/requests_sqlite.rs
  - crates/trackly-infra/src/test_support/test_db.rs
  - migrations/V030__printers_drop_connectivity_check.sql
  - migrations/V031__requests_status_add_cancelled.sql
  - ui/src/features/cartridges/OperationModal.svelte
  - ui/src/features/requests/RequestDetail.svelte
  - ui/src/features/requests/api.ts
findings:
  critical: 1
  warning: 6
  info: 3
  total: 10
status: issues_found
---

# Phase 12 (Round 2): Code Review Report

**Reviewed:** 2026-06-24
**Depth:** standard
**Files Reviewed:** 19
**Status:** issues_found

## Summary

Round-2 gap-closure batch (GAP-12-04..08): printer connectivity CHECK removal (V030),
request lifecycle with a new `cancelled` status (V031 + `RequestTransitionOp::Cancel`),
soft-delete + employee self-cancel endpoints, name-autocomplete aggregation, and
per-variant `WsEvent` serialization.

The **authorization and BOLA story is solid**. `requests_delete` (`Action::DeleteRequests`,
Admin|Manager) and `requests_cancel` (`Action::CancelOwnRequest` + service-layer ownership
re-check via `Self::get()`) are correctly gated; the role×endpoint matrix (Cases 36-39)
and `request_lifecycle.rs` exercise the deny/allow/BOLA paths directly. The thin Tauri
`requests_delete`/`requests_cancel` wrappers do not call `authorize()` themselves, but this
is not a gap — `RequestService::delete`/`cancel` self-authorize as their first statement.
The `WsEvent` per-variant camelCase serialization is correct and well-tested, and the
`suggest_person` SQL uses an enum-whitelisted column with parameterized `ESCAPE '\\'` LIKE,
so the autocomplete aggregation has no injection surface.

The one BLOCKER is a **test that V031 breaks**: `test_db.rs` hardcodes
`assert_eq!(user_version, 30)`, now false (schema is at 31) — a red, CI-gating unit test
shipped with the batch. Several WARNINGs concern the new `cancelled` status not being
threaded through the UI labels, status counts, and history-action map, plus a
privilege-asymmetry where a Manager can soft-delete an Admin-only `ad_register` request and
orphan its `users` row.

## Critical Issues

### CR-01: V031 breaks the hardcoded `user_version == 30` assertion in `test_db`

**File:** `crates/trackly-infra/src/test_support/test_db.rs:41`
**Issue:** This batch adds `migrations/V031__requests_status_add_cancelled.sql`, advancing
the schema to `user_version = 31`. The canonical test-DB fixture test still asserts the old
value:
```rust
assert_eq!(user_version, 30);
```
On a fresh DB the runner now sets `user_version = 31`, so
`test_db_returns_fully_migrated_connection` fails. The module doc comment
(`test_db.rs:4`, "currently V001..V030") is also stale. Unlike
`migrations.rs::run_applies_all_known_migrations_on_fresh_db`, which computes
`expected = max_known_version()` dynamically and stays green, this assertion is pinned to a
literal. CI runs the infra unit tests, so the batch ships a red build.
**Fix:** Track the runner instead of a literal, mirroring `migrations.rs`:
```rust
let expected = crate::db::migrations::max_known_version() as i64;
assert_eq!(user_version, expected, "schema must be fully migrated");
```
and update the doc comment to drop the hardcoded "V001..V030".

## Warnings

### WR-01: `cancelled` status renders as "Отклонена" (Rejected) in the UI

**File:** `ui/src/features/requests/RequestDetail.svelte:98-108` (sibling
`ui/src/features/requests/RequestListRow.svelte:28-36` is identical, outside the explicit
review set)
**Issue:** V031 + `RequestTransitionOp::Cancel` introduce a new terminal status
`cancelled`, and `requests_cancel` returns a DTO with `status: "cancelled"`. The
`statusLabel`/`statusVariant` `$derived` chains have no `cancelled` arm, so the final
`else` maps it to `'Отклонена'` (Rejected). A user who cancels their own request sees it
labelled as if a specialist rejected it — semantically wrong, and cancel-vs-reject is the
whole point of GAP-12-07/A4.
**Fix:** Add an explicit `cancelled` arm before the catch-all `else` in both `statusLabel`
(e.g. `'Отменена'`) and `statusVariant` (e.g. `'default'`):
```ts
: request.status === 'rejected'
  ? 'Отклонена'
  : request.status === 'cancelled'
    ? 'Отменена'
    : '—',
```

### WR-02: `actionLabel` map lacks `cancel`/`custom:cancel` — history shows the raw action string

**File:** `ui/src/features/requests/RequestDetail.svelte:157-169`
**Issue:** `RequestService::cancel` writes an audit row with
`action = op.audit_action() = "custom:cancel"` (`domain/printers.rs:210`). The
`actionLabel` lookup has `create/accept/complete/reject` and their `custom:` variants but
no `cancel`/`custom:cancel`. The `?? action` fallback then renders the raw
`"custom:cancel"` string in the History list of a cancelled request.
**Fix:** Add `cancel: 'Отменена'` and `'custom:cancel': 'Отменена'` to the `labels` record.

### WR-03: `RequestCounts`/`counts()` has no `cancelled` bucket — switch-bar totals drift

**File:** `crates/trackly-infra/src/repos/requests_sqlite.rs:299-361`,
`crates/trackly-app/src/services/request_service.rs:151-178`
**Issue:** `counts()` returns `all, open, in_progress, completed, rejected`. The `all`
query has no status filter, so it now includes `cancelled` rows, but there is no
`cancelled` bucket and `cancelled` is folded into none of the existing ones. After a
self-cancel, `all` increments by 1 while `open+in_progress+completed+rejected` no longer
sums to `all`. Any switch-bar/dashboard widget reconciling "all = sum of statuses" will
show a discrepancy, and cancelled requests are uncountable in the status bar.
**Fix:** Add a `cancelled` field to `RequestCounts` + `RequestCountsDto` and a matching
`WHERE status = 'cancelled'` count query so consumers can reconcile.

### WR-04: Manager can soft-delete an Admin-only `ad_register` request, orphaning the user row

**File:** `crates/trackly-app/src/services/request_service.rs:577-642`
**Issue:** `delete()` is gated on `Action::DeleteRequests` = Admin **or Manager**
(`auth.rs:146-155`) and is owner/type-agnostic — it soft-deletes any request in any status.
But `ad_register` requests are otherwise strictly Admin-only: `approve_ad_register` and
`reject_ad_register` both require `Action::ManageUsers` (Admin) and own the linked `users`
row reconciliation (activate on approve; soft-delete the auto-created user on reject). A
Manager calling `requests_delete` on an open `ad_register` request bypasses that Admin-only
lifecycle: the request is soft-deleted but the pending/auto-created `users` row is never
reconciled, leaving an orphaned inactive (pending) or still-active `is_active=1`
(auto-accept) user with no governing request — a privilege-boundary asymmetry on a
security-sensitive entity.
**Fix:** In `delete()`, read the request type first (reuse `self.get(id, caller)`) and
either (a) reject `request_type == "ad_register"` with `AppError::Forbidden`/`Validation`,
or (b) require `Action::ManageUsers` for `ad_register` deletions and run the same user-row
reconciliation as `reject_ad_register`. Option (a) is the smaller, safer change.

### WR-05: `printers_sqlite::list` status filter is a silent no-op

**File:** `crates/trackly-infra/src/repos/printers_sqlite.rs:317-341`
**Issue:** The `list` WHERE clause is `(?1 IS NULL OR p.last_seen_utc IS NOT NULL)` with
`?1` bound to `filter.status`. When a status is supplied this does NOT filter by the status
value — it merely requires `last_seen_utc IS NOT NULL` ("ever polled"), discarding the
actual `filter.status` string ("ok"/"error"/"offline"/…). The `total` count query shares
the shape, so paginated totals are also wrong relative to the selected status. This file is
touched by the batch (V030 reshapes `printers`); even if pre-existing, it is a correctness
defect the connectivity-CHECK removal interacts with.
**Fix:** Implement real status filtering (join the latest `printer_readings.status` and
compare to `?1`), or, if status filtering is genuinely deferred, drop the misleading bind
and document that `PrinterFilter.status` is currently ignored.

### WR-06: `transition_in_tx` `affected == 0` collapses lock-mismatch into `NotFound`

**File:** `crates/trackly-infra/src/repos/requests_sqlite.rs:153-189`
**Issue:** `transition_in_tx` fetches the row, version-checks → `OptimisticLockMismatch`,
validates status, then UPDATEs `WHERE id=? AND version=? AND deleted_at_utc IS NULL`; the
`affected == 0` branch unconditionally returns `NotFound`. `cancel()`/`delete()` pass the
*payload* version (not the version observed by the BOLA-`get()` reader read), so a
legitimate stale client version racing a concurrent transition surfaces here as `NotFound`
rather than the more accurate `OptimisticLockMismatch`. The UI special-cases
`OptimisticLockMismatch` to "reload and retry"; a `NotFound` is instead shown as "request
gone," misleading the operator.
**Fix:** Disambiguate in the `affected == 0` branch (mirror `delete()`'s pattern at
`request_service.rs:601-621`): `SELECT version, deleted_at_utc FROM requests WHERE id=?` —
return `NotFound` only when truly absent, `OptimisticLockMismatch` when it exists but the
version moved, and a deleted-specific error when `deleted_at_utc IS NOT NULL`.

## Info

### IN-01: `PrinterDto::from` hardcodes `community_configured: true` with a contradictory comment

**File:** `crates/trackly-app/src/dto/printer.rs:62-65`
**Issue:** The field comment says "the service layer sets it to true when community !=
default," but the `From` impl unconditionally sets `true` and no service override is in
evidence. Code and comment disagree; the indicator is effectively a constant and conveys no
information. Not a leak (community itself is never serialized), but misleading. Pre-existing,
surfaced by reading the DTO.
**Fix:** Compute `community_configured` from the stored community in the service read path,
or drop the field and its comment.

### IN-02: V030/V031 `PRAGMA foreign_keys = OFF` relies on the refinery one-file-per-tx invariant

**File:** `migrations/V030__printers_drop_connectivity_check.sql:28-59`,
`migrations/V031__requests_status_add_cancelled.sql:22-60`
**Issue:** Both rebuild migrations toggle `PRAGMA foreign_keys = OFF/ON` inside the file,
with a comment asserting refinery runs one file per transaction (`set_grouped(false)`).
`foreign_keys` is a connection-level (not transaction-level) PRAGMA — if a future change
flips refinery to grouped mode, the OFF window would leak across migrations and silently
disable FK enforcement for subsequent files in the same run. The invariant is load-bearing
and only protected by a comment.
**Fix:** Add a guard test asserting `PRAGMA foreign_keys` is `ON` after `migrations::run()`
on a fresh DB, so a future grouping change fails loudly.

### IN-03: `delete()` audit row captures no `before_json` snapshot

**File:** `crates/trackly-app/src/services/request_service.rs:624-636`
**Issue:** The soft-delete audit entry records only `action: "custom:delete"` with
`before_json: None`. Other destructive paths (e.g. `act_service.rs` device mutations)
capture `before_json` snapshots for forensics/undo. A deleted request leaves no record of
its status/owner/fields at deletion time, weakening the audit trail for an action the
confirm modal calls irreversible ("без возможности восстановления через интерфейс").
**Fix:** Capture the request row (or key fields) into `before_json` before the soft-delete
UPDATE, consistent with the project's snapshot-on-mutation pattern.

---

_Reviewed: 2026-06-24_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
