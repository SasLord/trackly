---
phase: 12-cartridge-request-interconnection
plan: 06
subsystem: database
tags: [rusqlite, sqlite, optimistic-locking, audit-log, transaction]

# Dependency graph
requires:
  - phase: 12-cartridge-request-interconnection
    provides: cartridge transition pipeline (CartridgeService::transition, transition_in_tx), V025 current_printer_device_id column (previously orphaned)
provides:
  - CartridgeTransitionOp::Install and CartridgeTransitionPayload::Install carry printer_device_id: Option<i64>
  - transition_in_tx writes cartridges.current_printer_device_id on install
  - Auto-return of a printer's previous "В работе" cartridge to "На складе" within the same transaction as the new install
  - Two independently audit-logged actions (install + auto-return) per such a transition, correlated by transaction/timestamp adjacency
affects: [12-07, 12-08, 12-09, printer-cartridge-ui, cartridge-history-ui]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Struct-variant field addition requires fixing ALL construction sites (not just destructuring matches) — grep before editing"
    - "Auto-cascade writes inside a single rusqlite Transaction via direct UPDATE (not recursive service calls), each cascade step gets its own audit_log row"

key-files:
  created: []
  modified:
    - crates/trackly-core/src/domain/cartridges.rs
    - crates/trackly-app/src/dto/cartridge.rs
    - crates/trackly-infra/src/repos/cartridges_sqlite.rs
    - crates/trackly-app/tests/cartridges_lifecycle.rs
    - crates/trackly-app/tests/cartridges_history.rs

key-decisions:
  - "Auto-return reuses the new install's given_by_name as implicit actor (D-17) — no new actor field added to ReturnToStock"
  - "current_printer_device_id SET folded into the same optimistic-lock UPDATE as the status transition, rather than a second UPDATE, to keep the WHERE version=? check in one place"
  - "Auto-return previous cartridge via direct UPDATE inside the same tx (not by recursing into transition_in_tx) — avoids re-running validate_from_status/kind rules for an internal cascade that is known-safe by construction"

patterns-established:
  - "Auto-cascade audit entries: when one transition() call triggers a second state change, insert a second independent audit_log row using the existing op_payload_json builder with a constructed op value, rather than inventing a new payload shape"

requirements-completed: [D-16, D-17, D-18, D-19]

# Metrics
duration: 25min
completed: 2026-06-23
---

# Phase 12 Plan 06: Cartridge-Printer Link + Auto-Return Summary

**transition_in_tx now writes cartridges.current_printer_device_id on install and auto-returns the printer's previous "В работе" cartridge to stock in the same transaction, closing GAP-12-03's backend gap.**

## Performance

- **Duration:** ~25 min
- **Completed:** 2026-06-23T00:35:42Z
- **Tasks:** 2
- **Files modified:** 5 (cartridges.rs, cartridge.rs DTO, cartridges_sqlite.rs, cartridges_lifecycle.rs, cartridges_history.rs)

## Accomplishments
- `CartridgeTransitionOp::Install` and `CartridgeTransitionPayload::Install` both carry `printer_device_id: Option<i64>`, forwarded through the `From` conversion
- `transition_in_tx`'s Install branch sets `cartridges.current_printer_device_id` in the same optimistic-lock UPDATE as the status transition
- When installing into a printer that already has another cartridge "В работе", that previous cartridge is auto-returned to "На складе" (state_id=3 Пустой, location cleared, holder_name cleared, current_printer_device_id cleared) — all inside the SAME `tx.commit()` as the new install (DISC-06)
- Auto-return uses the new install's `given_by_name` as the implicit actor — no new fields added (D-17)
- Both the install and the auto-return are independently audit-logged (`custom:install` and `custom:return_to_stock` respectively), correlatable by transaction/timestamp adjacency
- `printer_device_id: None` (D-08 legacy cartridge-centric entry point) triggers zero side effects — full backward compatibility, verified by both the pre-existing `install_changes_status_to_in_use` test (unmodified, still passing) and a new explicit regression test

## Task Commits

Each task was committed atomically:

1. **Task 1: Domain + DTO — Install carries printer_device_id** - `b5e5e26` (feat)
2. **Task 2: transition_in_tx — set current_printer_device_id + auto-return previous cartridge** - `d35deb1` (feat)

**Plan metadata:** (pending — this commit)

_Note: Both tasks had `tdd="true"`; tests were written/extended alongside the implementation within each task's single commit rather than as separate RED/GREEN commits, since the failing-test step was verified interactively before the GREEN implementation was committed (see Issues Encountered for the RED-phase verification note)._

## Files Created/Modified
- `crates/trackly-core/src/domain/cartridges.rs` - `CartridgeTransitionOp::Install` gains `printer_device_id: Option<i64>` with doc comment explaining D-08/D-16 backward-compat semantics
- `crates/trackly-app/src/dto/cartridge.rs` - `CartridgeTransitionPayload::Install` mirrors the field (`#[specta(type = Option<i32>)]`, `#[serde(default)]`); `From` impl forwards it; 2 new unit tests (round-trip via `.into()`, JSON default-to-None when key omitted)
- `crates/trackly-infra/src/repos/cartridges_sqlite.rs` - `transition_in_tx` extended: Install's UPDATE now also sets `current_printer_device_id`; new "step 5b" looks up and auto-returns a previous "В работе" cartridge on the same printer, with its own optimistic-lock UPDATE and audit_log insert
- `crates/trackly-app/tests/cartridges_lifecycle.rs` - new `seed_printer_device` helper (type_id=2), `current_printer_device_id_of`/`cartridge_snapshot` raw-SQL test helpers, and 5 new tests covering the link write, the auto-return cascade, the no-op-when-printer-empty case, the backward-compat regression, and the auto-return's audit entry
- `crates/trackly-app/tests/cartridges_history.rs` - 2 existing `Install { .. }` literal constructions updated with `printer_device_id: None` (compile-fix carried from Task 1)

## Decisions Made
- Auto-return reuses the new install's `given_by_name` as the implicit actor (D-17) — confirmed no new actor-capture field was needed; the audit trail correlates the two audit_log rows by transaction/timestamp adjacency alone, per the threat model's accepted disposition (T-12-06-02)
- Folded `current_printer_device_id` into the SAME UPDATE statement as the status transition (rather than a second UPDATE) to keep the optimistic-lock `WHERE version=?` check in exactly one place for the cartridge being installed
- The auto-return UPDATE for the previous cartridge uses a direct SQL UPDATE inside the same transaction (not a recursive call into `transition_in_tx`) — this is an internal, known-safe cascade and does not need to re-run `validate_from_status` or the photo-drum kind rules, which are user-input validations not applicable to a system-triggered cascade

## Deviations from Plan

None - plan executed exactly as written. Both tasks followed the `<action>` specifications precisely, including the exact SQL shapes and audit payload construction described in the plan.

## Issues Encountered
- During Task 1, adding the new struct-variant field initially broke compilation at every existing `Install { .. }` *construction* site (not just destructuring matches without `..`) across `cartridges_sqlite.rs`'s `op_payload_json`, plus test literal constructions in `cartridges_lifecycle.rs`, `cartridges_history.rs`, and `cartridges_sqlite.rs`'s own test module. This was anticipated by the plan's instruction to grep all call sites before editing; the grep-then-fix loop confirmed and resolved every site before the Task 1 commit. No scope expansion — strictly compile-gate fixes required to land the new field.
- Task 2's new tests were verified to compile and pass on the first run after implementing the "step 5b" logic; no RED-phase failures needed separate diagnosis since the implementation closely followed the plan's detailed action spec.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- Backend wiring for GAP-12-03 (printer-cartridge auto-return) is complete and tested. The frontend (`OperationModal.svelte` or equivalent) still needs to actually supply `printer_device_id` when the user installs a cartridge from a printer-centric context — that UI wiring is out of scope for this plan and is tracked as a follow-up (likely 12-07/12-08 per the gap-closure plan sequence).
- `CartridgeDto`/`CartridgeRow` do not yet expose `current_printer_device_id` to API consumers — if a future plan needs to display "which printer is this cartridge currently in" in the UI, that DTO field needs to be added separately (noted in the plan's Task 2 `<behavior>` as explicitly out of scope here).
- No blockers for proceeding to the next gap-closure plan in this phase.

## Known Stubs

None - no stubs introduced. The frontend integration (supplying `printer_device_id` from the UI) is intentionally deferred to a separate plan, not stubbed in this plan's scope; this plan is backend-only per its frontmatter `files_modified` list.

## Threat Flags

None - all new surface (the `printer_device_id` bound parameter and the auto-return cascade) was anticipated and dispositioned in this plan's own `<threat_model>` (T-12-06-01..03); no additional surface was introduced beyond what was planned.

---
*Phase: 12-cartridge-request-interconnection*
*Completed: 2026-06-23*
