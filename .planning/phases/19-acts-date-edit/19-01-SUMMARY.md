---
phase: 19-acts-date-edit
plan: 01
subsystem: acts
tags: [rust, rusqlite, specta, svelte, minijinja, act-pdf]

# Dependency graph
requires: []
provides:
  - ActDto.handover_date_utc wire field (Rust struct + regenerated TS interface)
  - acts_sqlite.rs list()/search_acts() sort by handover_date_utc DESC
  - render_pdf act+parent date fields sourced from handover_date_utc
  - ActListRow/ActDetail display handover_date_utc as the act's "Дата"
  - regression test suite proving the read-side date source (acts_date_source.rs +
    2 new html_act_render.rs tests)
affects: [19-02, 19-03, 19-04, 19-05]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Read-side date-source fix: ActRow already carried handover_date_utc since
      V015 (write side was already correct) — only ActDto mapping + 2 ORDER BY
      clauses + render_pdf's date-format call sites needed to catch up (Pitfall 1
      from RESEARCH.md)."

key-files:
  created:
    - crates/trackly-app/tests/acts_date_source.rs
  modified:
    - crates/trackly-app/src/dto/act.rs
    - crates/trackly-infra/src/repos/acts_sqlite.rs
    - crates/trackly-app/src/services/act_service.rs
    - crates/trackly-app/tests/html_act_render.rs
    - ui/src/features/acts/ActListRow.svelte
    - ui/src/features/acts/ActDetail.svelte
    - ui/src/bindings.ts (gitignored — regenerated, not committed)

key-decisions:
  - "ui/src/bindings.ts is gitignored (.gitignore:20) — Task 1's regeneration step
    was run and verified (grep -c handover_date_utc >= 1) but produces no git diff
    to commit; only the Rust ActDto struct change is committed."
  - "Fixed test assertions to check act.date_human (RU format, what the
    act_handover.html template actually renders in the subtitle/parent-block) rather
    than the unused ISO act.date field — discovered during RED-phase verification
    when both new html_act_render.rs tests failed on the ISO-date assertion even
    after the production fix was applied, because the template never emits the ISO
    format at all."
  - "render_acceptance_pdf's explicit date_utc parameter was left untouched per plan
    scope — confirmed via grep that no act.created_at_utc/parent.created_at_utc
    references remain in render_pdf's date-formatting call sites."

patterns-established:
  - "TDD gate sequence (RED commit -> GREEN commit) verified via git-apply-based
    temporary revert/reapply of the production diff rather than git stash (stash
    is prohibited in this workflow) — apply -R to confirm failing tests, commit
    tests, apply forward to confirm passing tests, commit fix."

requirements-completed: [ACT-01]

# Metrics
duration: 14min
completed: 2026-07-11
---

# Phase 19 Plan 01: Act Date Source Fix (ACT-01) Summary

**The acts list, detail card, and PDF/HTML print now sort and display by `handover_date_utc` ("Когда отдали") instead of `created_at_utc` (row-insertion timestamp) — closes the "act always shows today's date" bug end-to-end on the read side.**

## Performance

- **Duration:** ~14 min (2026-07-11T21:13:18+07:00 → 2026-07-11T21:27:04+07:00)
- **Started:** 2026-07-11T21:13:18+07:00
- **Completed:** 2026-07-11T21:27:04+07:00
- **Tasks:** 3/3 completed
- **Files modified:** 7 (1 new test file, 6 modified; `ui/src/bindings.ts` regenerated but gitignored)

## Accomplishments

- `ActDto` now carries `handover_date_utc` on the wire (Rust struct + `act_dto_from_row` mapping + regenerated TS interface) — the field existed in the domain (`ActRow`) and database (V015) but was never exposed to the frontend until now.
- `list()`/`search_acts()` in `acts_sqlite.rs` sort `DESC` by `handover_date_utc` at both call sites, proven by a dedicated regression test (`acts_date_source.rs`) using a fixture where creation order is the reverse of handover-date order.
- `render_pdf`'s act block and parent block (return-act's linked handover) now derive their `date`/`date_human` fields from `handover_date_utc`; `render_acceptance_pdf` (a distinct document type with an explicit `date_utc` parameter) was correctly left untouched.
- `ActListRow.svelte` and `ActDetail.svelte` display `act.handover_date_utc`; `created_at_utc` is no longer shown to the user as the act's date anywhere in these two components.
- Full TDD RED→GREEN gate sequence followed for Task 2 (backend date-source switch): tests committed first proving failure against the old `created_at_utc`-based code, then the fix committed proving all tests pass.

## Task Commits

Each task was committed atomically:

1. **Task 1: Add handover_date_utc to ActDto wire contract** - `004af34` (feat)
2. **Task 2 (RED): failing regression tests for handover_date_utc sort+render source** - `8af89fe` (test)
2. **Task 2 (GREEN): sort acts and render PDF dates by handover_date_utc** - `cd57544` (feat)
3. **Task 3: display handover_date_utc as act date in list row and detail** - `cc6d15b` (feat)

_Task 2 (`tdd="true"`) produced two commits per the RED/GREEN gate protocol._

## Files Created/Modified

- `crates/trackly-app/src/dto/act.rs` - `ActDto.handover_date_utc` field + `act_dto_from_row` mapping
- `crates/trackly-infra/src/repos/acts_sqlite.rs` - `list()`/`search_acts()` `ORDER BY a.handover_date_utc DESC, a.id DESC` (both call sites)
- `crates/trackly-app/src/services/act_service.rs` - `render_pdf`'s act block + parent block date/date_human read `handover_date_utc`
- `crates/trackly-app/tests/acts_date_source.rs` (new) - regression tests proving `list()`/`search()` sort by `handover_date_utc`, not creation order
- `crates/trackly-app/tests/html_act_render.rs` - 2 new tests proving `render_pdf`'s act-date and parent-block-date reflect `handover_date_utc`
- `ui/src/features/acts/ActListRow.svelte` - `dateLabel` derives from `act.handover_date_utc`
- `ui/src/features/acts/ActDetail.svelte` - `headerDate` derives from `act.handover_date_utc`
- `ui/src/bindings.ts` - regenerated via `cargo test --test export_bindings` (gitignored, not committed — verified `grep -c handover_date_utc` returns 4)

## Decisions Made

- `ui/src/bindings.ts` is gitignored (confirmed via `git check-ignore -v`) — the regeneration step in Task 1 was executed and verified but produces no commit; this is expected project behavior, not a gap.
- Corrected the two new `html_act_render.rs` test assertions mid-execution: the `act_handover.html` template only renders `date_human` (RU format) in its subtitle and parent-block markup, never the raw ISO `act.date` field. The tests originally asserted on the ISO string, which failed even with the fix correctly applied (a false-negative RED that would have persisted as a false-negative GREEN). Switched both tests to assert on `format_ru_date(...)` output instead, matching what the template actually emits.
- `render_acceptance_pdf`'s `date_utc` parameter (a distinct document type, explicitly out of scope per the plan's `<interfaces>` note) was left untouched; confirmed via grep that no `act.created_at_utc`/`parent.created_at_utc` references remain inside `render_pdf`'s date-formatting lines.

## Deviations from Plan

**None beyond the test-assertion correction documented above (in-flight test authoring, not a deviation from production code scope).** All three tasks were executed exactly as specified in the plan's `<action>` blocks; no Rule 1-4 auto-fixes were required in the production code.

## Issues Encountered

- The RED-phase verification (required by Task 2's `tdd="true"` attribute) surfaced that my first draft of the two new `html_act_render.rs` tests asserted on the wrong template output (ISO date instead of RU date_human). This was caught and corrected before the GREEN commit, so no incorrect test assertions were committed. Resolved by inspecting `crates/trackly-app/templates/act_handover.html` directly to confirm which context field the template interpolates.
- Confirmed `ui/src/bindings.ts` is gitignored via `git check-ignore -v` before assuming a missing commit was an error — avoided incorrectly force-adding a gitignored generated file.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- ACT-01 (this plan) is complete: sort order, detail card, and PDF/HTML print all reflect `handover_date_utc`; `created_at_utc` remains internal-only.
- `ActDto.handover_date_utc` is now available on the wire for Plan 19-02 (act editing) to read/patch as part of ACT-02's "Редактировать" fix, and for Plan 19-04 (bindings assertion) to add an explicit presence check in `export_bindings.rs`.
- No blockers identified for subsequent plans in Phase 19.

---
*Phase: 19-acts-date-edit*
*Completed: 2026-07-11*

## Self-Check: PASSED

All 7 claimed files found on disk; all 4 claimed commit hashes (`004af34`, `8af89fe`, `cd57544`, `cc6d15b`) found in git log.
