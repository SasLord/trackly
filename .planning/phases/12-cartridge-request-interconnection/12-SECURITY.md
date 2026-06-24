---
phase: 12
slug: cartridge-request-interconnection
status: secured
threats_open: 0
threats_total: 31
threats_closed: 10
threats_accepted: 21
register_authored_at_plan_time: true
asvs_level: 1
block_on: high
accepted_risks:
  - id: WR-04
    severity: medium
    summary: "Manager can requests_delete an ad_register request, orphaning the linked users row; fix tracked in follow-up task"
created: 2026-06-24
---

# SECURITY.md — Phase 12: cartridge-request-interconnection

**Milestone:** v1.1
**ASVS Level:** 1
**Audit date:** 2026-06-24
**Block policy:** `block_on: high`
**Scope:** Whole phase (plans 12-01..12-15). Depth focus: Round 2 gap-closure
(12-10..12-15), new request-lifecycle endpoints `requests_delete` / `requests_cancel`.

---

## Verdict

All declared `mitigate`-disposition threats are CLOSED with code evidence.
All `accept`-disposition threats are logged below. No executor-reported
unregistered attack surface (both `## Threat Flags` sections explicitly declare
"no new surface").

One out-of-register finding (**WR-04**, from 12-REVIEW.md) is a real but
**MEDIUM** data-integrity gap on the new `requests_delete` endpoint. It is below
the `high` block threshold; the user **explicitly accepted it as a documented risk
(2026-06-24)** with the fix tracked in a follow-up task. `threats_open: 0` — the
phase is not blocked. WR-04 is logged in the accepted-risks section below.

---

## Threat verification — `mitigate` dispositions (code-verified)

| Threat ID | Category | Evidence (file:line) | Status |
|-----------|----------|----------------------|--------|
| T-12-02-T-12-01 | Elevation of Privilege (cartridges/requests_transition RBAC) | `request_service.rs:407` `authorize(&Action::TransitionRequests)`; RBAC Cases A/B in `role_endpoint_matrix.rs` | CLOSED |
| T-12-04-01 | Tampering (suggest_person LIKE, cartridges arm) | bound `params![]` + `escape_like()` pattern; column fixed by Rust match, no interpolation | CLOSED |
| T-12-05-01 | Tampering (set_compatible_models/devices IDs) | all IDs bound via `params![]`; FK `ON DELETE CASCADE` on `printer_cartridge_models` (V029) | CLOSED |
| T-12-05-02 | Elevation of Privilege (compatibility mutators) | gated `Action::MutatePrinters`/`MutateCartridges` (Admin\|Manager) at `auth.rs:146-155` | CLOSED |
| T-12-06-01 | Tampering (printer_device_id in transition_in_tx) | bound param; FK `cartridges.current_printer_device_id REFERENCES devices(id)` | CLOSED |
| T-12-07-02 | Tampering (checked id arrays to setCompatible*) | backend 12-05 binds ids via `params![]` + FK constraints reject invalid ids at DB | CLOSED |
| T-12-09-01 | Tampering (previous_cartridge_state_id override) | bound param; same trust level as existing ReturnToStock.state_id (no new exposure) | CLOSED (accepted parity) |
| **T-12-14-01** | **Elevation of Privilege — BOLA on requests_cancel** | `request_service.rs:663` `cancel()` calls `self.get(id, caller)`; ownership guard at `:93-97` (`dto.requested_by_user_id != caller.user_id` → Forbidden for Employee). Verified by `role_endpoint_matrix.rs` Case 39 (`:1381-1394`, manager-owned request → 403) and `request_lifecycle.rs::cancel_other_users_request_returns_forbidden` (`:271`) | **CLOSED** |
| **T-12-14-02** | **Tampering — optimistic lock on requests_delete** | `request_service.rs:597` `UPDATE ... WHERE id=?2 AND version=?3 AND deleted_at_utc IS NULL`; `affected==0` disambiguates NotFound vs OptimisticLockMismatch (`:602-623`). Verified by `request_lifecycle.rs::delete_with_wrong_version_returns_optimistic_lock_mismatch` (`:155`) | **CLOSED** |
| **T-12-14-03** | **Repudiation — delete/cancel without audit trail** | `delete()` inserts `custom:delete` audit row in same `tx` (`request_service.rs:625-637`); `cancel()` inserts `custom:cancel` audit row in same `tx` (`:676-688`) | **CLOSED** |

---

## Accepted risks log (`accept` dispositions)

These were dispositioned `accept` at plan time. Verified the stated rationale
holds against the implementation; no SECURITY.md remediation required.

| Threat ID | Category | Component | Accepted rationale (verified) |
|-----------|----------|-----------|-------------------------------|
| T-12-01-01 | Information Disclosure | `CartridgeFilter.installable_only` / `RequestDto.printer_location` | Recomposition of already-readable columns behind existing ReadData/owner gates; no new RBAC surface |
| T-12-01-02 | Tampering | hardcoded `state_id IN (1,2)` | Domain constants; `installable_only` is a `bool`, no client-supplied state_id list (no SQLi vector) |
| T-12-02-03 | Tampering | `linked_cartridge_id` snapshot in notes_json | Real existing cartridge (else `NotFound` aborts tx); same `TransitionRequests` Admin\|Manager gate already grants ReadData — operator-error inaccuracy at worst, not escalation |
| T-12-03-01 | Elevation of Privilege | UI "Установить картридж" gate | UI `isSpecialist` is cosmetic; backend RBAC (Wave 2) is the real boundary |
| T-12-03-04 | Tampering | client-side selectedCartridge state | optimistic-lock (`OptimisticLockMismatch`) catches any tampered cartridge_id/version |
| T-12-04-02 | Information Disclosure | cartridges.holder_name via suggestions | holder_name already visible to any ReadData caller; no new auth requirement |
| T-12-05-03 | Denial of Service | cartridges_list compatibility subquery | indexed lookup against low-cardinality link table; no pagination bypass |
| T-12-05-SC | Tampering | no new package installs | Rust/SQL/internal only; Package Legitimacy Gate N/A |
| T-12-06-02 | Repudiation | auto-return previous cartridge actor field | D-17: same actor as new-install given_by_name; correlatable via created_at_utc + tx adjacency |
| T-12-06-03 | Denial of Service | "other cartridge in printer" subquery | single indexed-equality lookup; no unbounded scan |
| T-12-07-01 | Elevation of Privilege | CompatibleModels/DevicesEditor render | mounted in Admin/Manager-gated screens; backend independently enforces MutatePrinters/MutateCartridges |
| T-12-08-01 | Information Disclosure | getCompatibleModels from install picker | same ReadData gate; 403 fail-safe falls back to UX hint only |
| T-12-09-02 | Tampering | previous_cartridge_location free text | bound param (not interpolated); same trust as existing location fields |
| T-12-10-01 | Tampering | DROP TABLE printers / rebuild (V030) | refinery one-file = one transaction (`set_grouped(false)`); full rollback on any step failure. VERIFIED in `V030__printers_drop_connectivity_check.sql` |
| T-12-10-02 | Denial of Service | PRAGMA foreign_keys=OFF window (V030) | OFF/ON pair scoped to single migration file run once at startup before traffic; FK=ON restored at file end (`V030:59`). VERIFIED |
| T-12-11-01 | Information Disclosure | WsEvent field casing | casing-only change; `is_visible_to()` data composition unchanged |
| T-12-12-01 | Information Disclosure | printer IP shown in install form | same value already visible to Admin/Manager via printer card (ReadPrinters) |
| T-12-13-01 | Tampering | `json_extract` on payload_json | `payload_json` server-built via `serde_json::json!()`; `json_extract` is read-only. VERIFIED in `audit_log_sqlite.rs:75` |
| T-12-13-02 | Information Disclosure | audit_log names in autocomplete | same exposure as existing acts.giver_name / cartridges.holder_name name sources |
| T-12-14-04 | Denial of Service | employee create/cancel loop | cancel limited to own + open status; no costlier than CreateRequest (already unrate-limited for all roles) |
| T-12-15-01 | Spoofing | employee forges DOM to show Cancel on others' requests | UI `isOwnRequest` cosmetic only; real guard is server BOLA in `cancel()` (T-12-14-01, Case 39) |

---

## Accepted out-of-register findings

### WR-04 — Manager can delete an `ad_register` request, orphaning the linked user row

**Disposition: ACCEPTED (documented risk) — user decision 2026-06-24.** Below the
`high` block threshold; fix tracked in a follow-up task. Does not block Phase 12.

- **Source:** 12-REVIEW.md (code review), NOT in any plan-time `<threat_model>` register.
- **Category:** Tampering / data-integrity (secondary Elevation-of-Privilege flavor).
- **Severity:** **MEDIUM** (below `high` block threshold → does not block phase).
- **Endpoint:** `POST /api/v1/requests_delete` (and Tauri `requests_delete`).
- **Detail:** `RequestService::delete()` (`request_service.rs:582-643`) authorizes on
  `Action::DeleteRequests` = **Admin | Manager** (`auth.rs:153`) and soft-deletes a
  request in ANY status of ANY type — with **no `request_type == "ad_register"`
  branch**. The `ad_register` lifecycle is special: its
  reject/approve paths (`reject_ad_register` `:851`, `approve_ad_register` `:720`)
  reconcile the linked `users` row (activate / soft-delete / revive) and are
  **Admin-only** (`Action::ManageUsers`). `delete()` bypasses that reconciliation
  entirely. A Manager (who cannot approve/reject ad_register) CAN delete the
  governing request, potentially leaving:
    - an auto-created **active** `users` row with no pending request to govern it
      (auto-accept path), or
    - a pending/blocked user row stranded with no approvable request.
- **Why not HIGH:** requires an already-trusted Manager role (not an Employee/external
  escalation); impact is integrity inconsistency on the linked user row, not direct
  takeover. No password/secret exposure.
- **Recommended fix (NOT applied — implementation is read-only here):** either
  (a) reject `delete()` when `request_type == "ad_register"` (force the
  Admin-only reject/approve path), or (b) require `Action::ManageUsers` for
  deleting `ad_register` requests, or (c) cascade the same user-row reconciliation
  inside `delete()`'s transaction.
- **Status:** ACCEPTED (documented risk, 2026-06-24). Follow-up fix task exists.
  Re-run `/gsd-secure-phase 12` after the fix lands to move WR-04 to CLOSED.

---

## Unregistered flags (executor-reported new attack surface)

None. `12-06-SUMMARY.md` and `12-09-SUMMARY.md` `## Threat Flags` sections both
explicitly state no new surface beyond the plan's own `<threat_model>`. All other
plan summaries carry no `## Threat Flags` section (no new surface declared).

---

## Notes

- Two pre-existing AD-mock test failures (`restore_request_visibility_http`,
  `settings_ad::ad_test_connection_admin_succeeds_in_mock_mode`) are environmental
  (no AD reachable from macOS dev box) — **not security findings**, out of scope.
- Cancel path optimistic lock additionally confirmed: `transition_in_tx`
  (`requests_sqlite.rs:120-168`) checks `current.version != version` (`:134`) AND
  uses `WHERE ... version = ?7` (`:160`), so `cancel()` is version-guarded too.
- Migration V031 (`requests.status` CHECK widened to include `'cancelled'`) follows
  the same single-file-transaction + scoped FK OFF/ON pattern as V030 — verified.
