---
phase: 21-cartridge-drum-codes
plan: 01
subsystem: database
tags: [rusqlite, format, cartridges, numbering]

# Dependency graph
requires:
  - phase: 04-cartridges
    provides: assign_code_in_tx auto-numbering with retry-loop counter mechanism
provides:
  - Shortened cartridge/photo-drum auto-code format (C-NNNN / D-NNNN, minimum 4 digits) replacing the previous 6-digit format
affects: [cartridges, drums, reports (code display)]

# Tech tracking
tech-stack:
  added: []
  patterns: []

key-files:
  created: []
  modified:
    - crates/trackly-infra/src/repos/cartridges_sqlite.rs
    - crates/trackly-app/tests/cartridges_numbering.rs

key-decisions:
  - "format!(\"{prefix}-{seq:04}\") is a minimum-width format specifier, not a fixed width — no DB migration needed since existing 6-digit codes stay valid distinct strings and seq > 9999 naturally overflows to 5+ digits without collision"
  - "concurrent_50_unique_codes assertion widened to len >= 6 (minimum 4 digits) rather than a fixed length, per plan spec, so it stays valid both today and once counters exceed 9999"

patterns-established: []

requirements-completed: [CRT-01]

# Metrics
duration: 22min
completed: 2026-07-14
---

# Phase 21 Plan 01: Cartridge/Drum Auto-Code Format Shortened to C-NNNN/D-NNNN Summary

**Cartridge and photo-drum auto-codes now generate as `C-0001`/`D-0001` (4-digit minimum width) instead of `C-000001`/`D-000001`, via a single format-string change in `assign_code_in_tx`.**

## Performance

- **Duration:** 22 min
- **Started:** 2026-07-14T21:16:04+07:00 (first commit in this session)
- **Completed:** 2026-07-14T21:38:15Z
- **Tasks:** 1 (TDD: RED + GREEN)
- **Files modified:** 2

## Accomplishments
- `assign_code_in_tx` in `crates/trackly-infra/src/repos/cartridges_sqlite.rs` now emits `format!("{prefix}-{seq:04}")` instead of `{seq:06}`, shortening new auto-codes to a 4-digit minimum width for both cartridges (`C-`) and photo drums (`D-`)
- Doc-comment and inline comment updated to reference `C-NNNN` / `D-NNNN`
- `cartridges_numbering.rs` integration test widened to assert the new minimum-width pattern (`len >= 6`, `C-` prefix, all-ASCII-digit suffix) instead of the old fixed 8-char check
- Retry-loop, `code_override` UNIQUE-validation branch, and the `kind_id == 2` → `drum_seq`/`'D'` vs `cartridge_seq`/`'C'` counter mapping are unchanged

## Task Commits

Each task was committed atomically (TDD RED/GREEN):

1. **Task 1 (RED): widen numbering test assertion** - `9177c13` (test)
2. **Task 1 (GREEN): shorten auto-code format** - `166e540` (feat)

**Plan metadata:** (final commit — see below)

_Note: this task used tdd="true"; see TDD Gate Compliance note below regarding the RED phase._

## Files Created/Modified
- `crates/trackly-infra/src/repos/cartridges_sqlite.rs` - `assign_code_in_tx`: format string `{seq:06}` → `{seq:04}`, doc/inline comments updated to `C-NNNN`/`D-NNNN`
- `crates/trackly-app/tests/cartridges_numbering.rs` - `concurrent_50_unique_codes` assertion widened to minimum-4-digit pattern; header/inline comments updated

## Decisions Made
- No DB migration added: `{:04}` is a *minimum* width specifier. Existing 6-digit codes (e.g. `C-000123`) remain valid, distinct strings from new 4-digit codes (e.g. `C-0001`) — there is no collision because the strings differ character-for-character, and once a counter exceeds 9999 the format naturally widens to 5+ digits with no special-casing required.
- Test assertion widened to `len >= 6` (a superset of both the old and new formats) rather than a fixed length, matching plan instructions and staying valid indefinitely as counters grow.

## Deviations from Plan

None — plan executed exactly as written. One documented TDD-gate nuance below (not a deviation from the plan's own instructions, since the plan explicitly specified the widened, superset-style assertion).

## TDD Gate Compliance

- `test(21-01): ...` commit exists (`9177c13`) — RED gate commit present.
- `feat(21-01): ...` commit exists after it (`166e540`) — GREEN gate commit present.
- No REFACTOR commit — none needed (single format-string change).

**Note on the RED phase:** Per the plan's explicit instructions, the updated test assertion (`code.len() >= 6 && ...`, i.e. "minimum 4 digits") is a superset of the pre-existing 6-digit output (`code.len() == 8`). Running the widened assertion against the unmodified source (`{seq:06}`) therefore passed rather than failed — this is expected and intentional (the plan's own rationale: "не фиксированную длину 8" — the assertion is deliberately forward-compatible with future counter values > 9999, not just the immediate 4-digit target). The actual narrowing to 4-digit output is verified by the source diff itself (`grep '{seq:04}'` present, `grep '{seq:06}'` absent) and confirmed functionally by both tests passing after the GREEN commit. No investigation/fix was needed — this is a known characteristic of widening-style test assertions applied to formatting refactors, not a sign the feature already existed.

## Issues Encountered
None.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- CRT-01 closed; both cartridge and photo-drum auto-codes now use the 4-digit-minimum format.
- No blockers for the rest of milestone v1.1.2 (remaining items: Phase 22 human-UAT follow-ups tracked separately in STATE.md / memory).

## Self-Check: PASSED

- FOUND: crates/trackly-infra/src/repos/cartridges_sqlite.rs
- FOUND: crates/trackly-app/tests/cartridges_numbering.rs
- FOUND commit: 9177c13
- FOUND commit: 166e540

---
*Phase: 21-cartridge-drum-codes*
*Completed: 2026-07-14*
