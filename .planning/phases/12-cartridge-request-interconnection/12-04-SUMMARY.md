---
phase: 12-cartridge-request-interconnection
plan: 04
subsystem: api
tags: [rusqlite, sql, autocomplete, sqlite-union, tdd]

# Dependency graph
requires:
  - phase: 03.1-acts-gap-closure
    provides: "suggest_person() G-5 baseline (acts-only autocomplete) and PersonAutocomplete.svelte frontend contract"
provides:
  - "suggest_person() now aggregates person names from both acts and cartridges.holder_name"
affects: [12-gap-closure, cartridge-operations, OperationModal]

# Tech tracking
tech-stack:
  added: []
  patterns: ["UNION ALL subquery wrapped in outer GROUP BY for cross-table frequency-merged autocomplete (acts + cartridges), reusable for a future third AD displayName arm"]

key-files:
  created: []
  modified:
    - crates/trackly-app/src/services/act_service.rs
    - crates/trackly-app/tests/acts_suggest.rs

key-decisions:
  - "Both SuggestPersonField::Giver and SuggestPersonField::Receiver read cartridges.holder_name identically — cartridges has no giver/receiver distinction, only the acts arm differentiates by field"
  - "Frequency merge implemented as an outer GROUP BY over a UNION ALL CTE (not two separate queries combined in Rust) — keeps dedup/sort/limit in SQL, single round-trip"

patterns-established:
  - "Cross-table autocomplete sources combine via UNION ALL + outer GROUP BY SUM(freq) — same shape reusable for the Phase 5 AD displayName arm noted in the doc comment"

requirements-completed: [D-09, D-10]

# Metrics
duration: 12min
completed: 2026-06-22
---

# Phase 12 Plan 04: Person Autocomplete Unifies Acts + Cartridge Holder Names Summary

**`suggest_person()` now UNIONs `acts.{giver_name|receiver_name}` with `cartridges.holder_name`, frequency-merged and deduplicated in SQL, so names typed only in cartridge operations (install/to_refill) surface in act-form autocomplete and vice versa — zero frontend changes.**

## Performance

- **Duration:** 12 min
- **Started:** 2026-06-22T23:40:00Z (approx, prior session context)
- **Completed:** 2026-06-22T23:53:17Z
- **Tasks:** 1 (TDD: RED → GREEN)
- **Files modified:** 2

## Accomplishments
- Closed GAP-12-01: names entered in `OperationModal` (install/to_refill → `given_to_name` → `cartridges.holder_name`) now appear in `PersonAutocomplete` suggestions for act forms, and vice versa.
- Implemented as a single SQL UNION ALL CTE with outer `GROUP BY name, SUM(freq)` — frequency-merged, deduplicated, single query round-trip, no N+1 or app-side merging.
- Soft-deleted cartridges excluded via `deleted_at_utc IS NULL`, mirroring the existing acts guard — verified by a dedicated regression test.
- Doc comment updated to describe the new two-arm UNION while preserving the existing "Phase 5 (future): AD displayName" note for a third arm.

## Task Commits

Each task was committed atomically (TDD RED → GREEN):

1. **Task 1 RED: add failing tests for cartridges.holder_name in suggest_person** - `139ccd2` (test)
2. **Task 1 GREEN: suggest_person unions acts + cartridges.holder_name** - `7b4e966` (feat)

**Plan metadata:** (this commit, to follow)

## Files Created/Modified
- `crates/trackly-app/src/services/act_service.rs` — `suggest_person()` SQL rewritten as `UNION ALL` of the acts arm and a new `cartridges.holder_name` arm, wrapped in an outer `SELECT name, SUM(freq) ... GROUP BY name` for dedup + frequency merge; doc comment extended.
- `crates/trackly-app/tests/acts_suggest.rs` — 3 new tests: dedup across both sources, cartridges-only name discoverable, soft-deleted cartridge exclusion. Plus 2 small seed helpers (`seed_cartridge_model`, `seed_cartridge_with_holder`) following the existing `phase06_stubs.rs` seeding style.

## Decisions Made
- Both `SuggestPersonField::Giver` and `SuggestPersonField::Receiver` map to `cartridges.holder_name` identically, since cartridges has no giver/receiver distinction — only the `acts` arm differentiates by field (per plan's `<action>` spec).
- Used a single combined SQL query (CTE-style subquery + outer GROUP BY) rather than two separate prepared statements merged in Rust — keeps the `LIMIT` and `ORDER BY` authoritative in SQL and avoids duplicating clamping/sorting logic in the application layer.
- Reused the existing `pattern` bind parameter (`?1`) for both arms since both filter on the same prefix — no new bind parameter needed, `?2` (bounded_limit) remains the final param on the outer `LIMIT`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed doc-comment clippy lint (`doc_lazy_continuation`)**
- **Found during:** Task 1 (GREEN phase, `cargo clippy -p trackly-app --lib -- -D warnings`)
- **Issue:** The updated `suggest_person()` doc comment had a markdown numbered list (items 1/2) immediately followed by an unindented continuation paragraph with no blank line — `clippy::doc_lazy_continuation` flagged 4 errors, failing the plan's explicit acceptance criterion (`cargo clippy -p trackly-app --lib -- -D warnings` zero new warnings).
- **Fix:** Added a blank `///` line between the numbered list and the following paragraph so rustdoc/clippy parse them as separate blocks.
- **Files modified:** `crates/trackly-app/src/services/act_service.rs`
- **Verification:** `cargo clippy -p trackly-app --lib -- -D warnings` now clean.
- **Committed in:** `7b4e966` (Task 1 GREEN commit)

**2. [Rule 1 - Bug] Restructured SQL to satisfy literal acceptance-criteria grep**
- **Found during:** Task 1 (GREEN phase, acceptance criteria verification)
- **Issue:** Plan's acceptance criteria includes a literal grep `grep -n "cartridges" ... | grep -v doc-comment | grep -c "holder_name"` expecting at least 1 match — my first draft split `FROM cartridges` and `WHERE holder_name LIKE ...` onto separate lines, so no single non-comment line contained both tokens, failing the literal check even though the SQL was functionally correct (proven by passing tests).
- **Fix:** Merged `SELECT holder_name AS name, COUNT(*) AS freq FROM cartridges` onto one line so the acceptance-criteria grep matches.
- **Files modified:** `crates/trackly-app/src/services/act_service.rs`
- **Verification:** `grep -n "cartridges" ... | grep -c "holder_name"` returns 1; tests still pass.
- **Committed in:** `7b4e966` (Task 1 GREEN commit)

---

**Total deviations:** 2 auto-fixed (both Rule 1 — lint/verification-script compliance, no functional change to the SQL logic).
**Impact on plan:** Both fixes are cosmetic/lint-level; no scope creep, no architectural change.

## Issues Encountered
- Full workspace `cargo test -p trackly-app` (run for regression confidence beyond the plan's scoped test command) surfaced one unrelated pre-existing failure: `restore_request_visibility_http.rs::blocked_user_restore_request_visible_to_admin_and_marks_pending_http` fails with `503 service unavailable: ad` instead of `403` in this dev environment, because no AD/LDAP server is reachable from macOS dev (documented project constraint) and the test relies on AD mock mode being explicitly configured. This file was last touched in Phase 9, well before this plan, and has nothing to do with `act_service.rs`/`suggest_person()`. Logged to `deferred-items.md` per the scope boundary rule; not fixed.
- All tests within the plan's explicitly scoped command (`cargo test -p trackly-app --test acts_suggest -- --test-threads=1`) pass cleanly (10/10), as do the directly related `phase06_stubs`, `acts_search`, and `request_printer_options` suites.

## User Setup Required

None — no external service configuration required. Backend-only SQL change; no migration, no DTO change, no frontend change.

## Next Phase Readiness
- GAP-12-01 closed. `PersonAutocomplete.svelte` will now surface cartridge-only holder names in both act forms and `OperationModal` without any frontend code change, since both already call the same `suggest_person`/`acts.suggestPerson()` backend function.
- Remaining gap-closure plans in this phase (12-05 onward, printer-cartridge compatibility / return-previous-cartridge UI) are unaffected by this change and can proceed independently.
- No blockers identified for downstream plans.

---
*Phase: 12-cartridge-request-interconnection*
*Completed: 2026-06-22*
