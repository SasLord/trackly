---
phase: 12-cartridge-request-interconnection
plan: 02
subsystem: api
tags: [rusqlite, axum, rbac, sqlite, history-audit, specta]

# Dependency graph
requires:
  - phase: 12-cartridge-request-interconnection (Wave 1, Plan 01)
    provides: "CartridgeFilter.installable_only, RequestDto.printer_location (D-05) — DTO/domain layer stable, no new SQL migrations needed by this plan"
provides:
  - "RequestService.cartridge_repo — read-only cartridge lookup wired into transition() for history enrichment"
  - "completed_cartridge_id persistence (D-06) confirmed by a real async test (was #[ignore] stub)"
  - "History audit notes enriched with human-readable 'Установлен C-NNNNNN (Brand Model)' line after Complete{linked_cartridge_id} (D-07)"
  - "RBAC test coverage closing T-12-01: Employee 403 on cartridges_transition and requests_transition (Cases 31/32)"
  - "ui/src/bindings.ts regenerated (gitignored) carrying CartridgeFilter.installable_only"
  - "ui/src/bindings-phase6.ts manually updated with RequestDto.printerLocation"
affects: [12-03, frontend-cartridge-picker, request-history-ui]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Pre-write read-then-enrich: transition() resolves a foreign read (cartridge snapshot) via spawn_blocking BEFORE entering the writer transaction, then folds the result into the existing notes_json 'notes' key instead of adding a new JSON shape — keeps get_history() parsing untouched"
    - "Dual-service-on-shared-writer test pattern: CartridgeService and RequestService both constructed on the same (writer, readers) pair from test_writer_and_readers() in a single test — safe because both go through the same single-writer WriterHandle"

key-files:
  created: []
  modified:
    - crates/trackly-app/src/services/request_service.rs
    - crates/trackly-app/tests/phase06_stubs.rs
    - crates/trackly-app/tests/role_endpoint_matrix.rs
    - ui/src/bindings.ts
    - ui/src/bindings-phase6.ts

key-decisions:
  - "Cardridge-snapshot text folds into the existing notes_json {\"notes\": ...} key (combined with any operator notes via '; ' separator) rather than adding a new JSON key — keeps get_history() parsing/RequestHistoryEntryDto untouched, per RESEARCH.md Open Question 2 recommendation"
  - "New RBAC test cases numbered 31/32 (not 25/26 as the plan's stale snapshot suggested) — the actual file already had Cases up to 30 from later untracked work; followed the plan's INTENT (continue from true max) over its literal stale numbers"
  - "T-12-03 (Tampering: unvalidated linked_cartridge_id provenance in history snapshot) accepted as-is per plan's threat model — TransitionRequests is Admin|Manager only, who already have full ReadData access, so worst case is an inaccurate history note from operator error, not privilege escalation"

requirements-completed: [D-06, D-07]

duration: 35min
completed: 2026-06-22
---

# Phase 12 Plan 02: Cartridge-Request History Enrichment + RBAC Closure Summary

**RequestService.transition() now reads the linked cartridge via a pre-write spawn_blocking lookup and folds "Установлен C-NNNNNN (Brand Model)" into the audit history notes; closed an RBAC test-coverage gap (Employee 403 on cartridges_transition/requests_transition) and synced both bindings files with Wave 1's DTO additions.**

## Performance

- **Duration:** 35 min
- **Started:** 2026-06-22T04:30:00Z (approx, continuation from prior session)
- **Completed:** 2026-06-22T05:05:01Z
- **Tasks:** 2 completed
- **Files modified:** 5 (request_service.rs, phase06_stubs.rs, role_endpoint_matrix.rs, bindings.ts [gitignored, not committed], bindings-phase6.ts)

## Accomplishments
- `RequestService` gained a `cartridge_repo: Arc<SqliteCartridgeRepository>` field (zero-sized, same pattern as `request_repo`/`audit_repo`), with no change to `RequestService::new()`'s signature — callers in `context.rs`/transports untouched.
- `transition()` reads the linked cartridge BEFORE the write transaction (spawn_blocking + reader pool, matching the existing `get()`/`get_history()` pattern), then builds `"Установлен {code} ({brand} {name})"` and merges it into the existing `notes_json` "notes" key — combined with operator-supplied notes via `"; "` when both are present.
- `completed_cartridge_id` persistence (D-06) — already correct at the SQL layer (`COALESCE(?4, completed_cartridge_id)`) — is now proven by a real `#[tokio::test]` instead of an `#[ignore]` stub.
- Two new regression tests confirm history enrichment behavior with and without a linked cartridge (D-07).
- Closed RBAC test-coverage gap T-12-01: Cases 31/32 in `role_endpoint_matrix.rs` prove Employee gets 403 Forbidden on both `cartridges_transition` and `requests_transition` (even on their own request) — no new authorization code, pure coverage.
- `ui/src/bindings.ts` (gitignored, regenerated via the project's existing `export_bindings` integration test) now carries `installable_only` on `CartridgeFilter`.
- `ui/src/bindings-phase6.ts` (hand-maintained per project convention) manually updated with `RequestDto.printerLocation: string | null`.

## Task Commits

Each task was committed atomically:

1. **Task 1: transition() — cartridge_repo wiring + снапшот истории (D-06/D-07)** - `7012f7a` (feat)
2. **Task 2: RBAC Case 31/32 + bindings regen** - `f8714fe` (test)

**Plan metadata:** _pending — final docs commit below_

## Files Created/Modified
- `crates/trackly-app/src/services/request_service.rs` - Added `cartridge_repo` field + import; `transition()` reads linked cartridge pre-write and enriches `notes_json`
- `crates/trackly-app/tests/phase06_stubs.rs` - De-stubbed `test_req_cart_link`; added `history_shows_cartridge_snapshot_after_complete` and `history_complete_without_cartridge_keeps_plain_notes`; added 3 seed helpers (printer device, cartridge model, cartridge)
- `crates/trackly-app/tests/role_endpoint_matrix.rs` - Added Case 31 (cartridges_transition employee-deny), Case 32 (requests_transition employee-deny on own request); updated doc-comment header
- `ui/src/bindings.ts` - Regenerated (gitignored, not committed) — carries `installable_only`
- `ui/src/bindings-phase6.ts` - Added `printerLocation: string | null` to `RequestDto`

## Decisions Made
- History enrichment folds into the existing `notes` JSON key (no new key, no `get_history()`/`RequestHistoryEntryDto` changes) — minimizes blast radius and matches RESEARCH.md's recommended approach.
- RBAC cases numbered 31/32 instead of the plan's suggested 25/26 — see Deviations below.
- T-12-03 (unvalidated `linked_cartridge_id` provenance) accepted per plan's threat model, no mitigation code added — documented disposition, not a gap.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Seeded a printer device for cartridge_replace requests in new tests**
- **Found during:** Task 1 (test_req_cart_link implementation)
- **Issue:** First test run failed with `Validation { field: "printer_device_id", message: "printer_device_id is required for cartridge_replace" }` — the plan's behavior spec didn't mention seeding a printer device, but `RequestService::create()`'s WR-02 validation requires `printer_device_id` for `cartridge_replace` request type.
- **Fix:** Added `seed_printer_device_for_link_tests()` helper (INSERT INTO devices, type_id=2) and passed `printer_device_id: Some(...)` in both new tests' `RequestCreateDto`.
- **Files modified:** `crates/trackly-app/tests/phase06_stubs.rs`
- **Verification:** `test_req_cart_link` and `history_shows_cartridge_snapshot_after_complete` both pass.
- **Committed in:** `7012f7a` (Task 1 commit)

**2. [Rule 1 - Bug in plan's literal instructions] Renumbered new RBAC cases 31/32 instead of plan's suggested 25/26**
- **Found during:** Task 2 (before inserting new Case blocks)
- **Issue:** The plan's authoring-time snapshot assumed the file's last case was Case 24 ("Cases 20-24 уже зарезервированы... новые кейсы — например 25/26"), but the actual file already contained Cases up to 30, added by later untracked work not reflected in the plan text.
- **Fix:** Ran `grep -n "Case [0-9]*:" crates/trackly-app/tests/role_endpoint_matrix.rs | tail -20` to find the true max (Case 30), then numbered the new cases 31 and 32, following the plan's INTENT (continue numbering from the actual max) rather than its stale literal suggestion.
- **Files modified:** `crates/trackly-app/tests/role_endpoint_matrix.rs`
- **Verification:** `cargo test -p trackly-app --test role_endpoint_matrix` — all 32 cases (30 pre-existing + 2 new) pass.
- **Committed in:** `f8714fe` (Task 2 commit)

---

**Total deviations:** 2 auto-fixed (1 blocking — missing test fixture data, 1 bug fix — plan's stale case numbering)
**Impact on plan:** Both deviations were necessary to make the plan's specified tests/cases actually compile and pass. No scope creep — no new production behavior beyond what the plan specified.

## Issues Encountered
- Pre-existing `cargo clippy -p trackly-app --tests -- -D warnings` failures (2x `len_zero` in `template_service.rs`, 1x `disallowed_methods` in `tests/backup_service.rs`) are unrelated to this plan's touched files — confirmed via targeted grep that none originate in `request_service.rs`, `phase06_stubs.rs`, or `role_endpoint_matrix.rs`. Already documented in `.planning/phases/09-ad/deferred-items.md` and `.planning/phases/10-employee-employee-ui-role-gating-read/deferred-items.md`. Left untouched per deviation rules scope boundary.
- Pre-existing `cargo fmt --check -p trackly-app` diffs in `request_printer_options.rs` and `ws_http_single_broadcast.rs` confirmed unrelated to this plan's touched files (targeted grep returned empty for our files). Left untouched.
- The doc-comment header in `role_endpoint_matrix.rs` had an existing undocumented gap (Cases 25-30 were never added to the header by whatever prior work introduced them). Rather than retroactively backfilling that gap (out of scope for this plan), added a one-line note acknowledging it alongside the new Cases 31-32 documentation.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- D-06 (completed_cartridge_id persistence) and D-07 (history enrichment) are both implemented and test-covered — Wave 3 (frontend cartridge picker) can rely on `linkedCartridgeId` round-tripping correctly into both `completedCartridgeId` and a human-readable history entry.
- T-12-01 (RBAC gap on cartridge/request transition endpoints) is closed — no remaining action items for that threat.
- `ui/src/bindings-phase6.ts` is now in sync with `RequestDto.printerLocation` from Wave 1 — the follow-up flagged in `12-01-SUMMARY.md` is resolved; Wave 3 does not need to touch this file for that field.
- No blockers for Wave 3.

---
*Phase: 12-cartridge-request-interconnection*
*Completed: 2026-06-22*

## Self-Check: PASSED

All created/modified files verified present on disk:
- crates/trackly-app/src/services/request_service.rs — FOUND
- crates/trackly-app/tests/phase06_stubs.rs — FOUND
- crates/trackly-app/tests/role_endpoint_matrix.rs — FOUND
- ui/src/bindings.ts — FOUND
- ui/src/bindings-phase6.ts — FOUND
- .planning/phases/12-cartridge-request-interconnection/12-02-SUMMARY.md — FOUND

All commit hashes verified present in git log:
- 7012f7a — FOUND
- f8714fe — FOUND
