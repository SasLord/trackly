---
phase: 36-act-pagination
plan: 01
subsystem: infra
tags: [rusqlite-free, file-templates, legacy-defaults, upgrade-mechanism, jinja]

# Dependency graph
requires:
  - phase: 35-act-body
    provides: KNOWN_LEGACY_DEFAULTS v20..v23 registry + upgrade_untouched_defaults_on_startup mechanism (Phase 16/20/34/35)
provides:
  - "_legacy_defaults/v24/act_handover.html — byte-identical snapshot of act_handover.html at Phase-36 start (post-Phase-35 body, pre-pagination)"
  - "KNOWN_LEGACY_DEFAULTS act_handover.html slice extended to 5 elements (v20..v24)"
  - "upgrade_replaces_v24_legacy_default_with_current_bundled_body regression test (currently RED by design, turns GREEN once 36-02 lands)"
affects: [36-02-act-pagination, 36-03-act-pagination, 36-04-act-pagination, 36-05-act-pagination]

# Tech tracking
tech-stack:
  added: []
  patterns: ["version-bump ritual for KNOWN_LEGACY_DEFAULTS: snapshot BEFORE live-file edit, register as new slice element, add mirrored assert_ne!-guarded regression test"]

key-files:
  created:
    - crates/trackly-app/templates/_legacy_defaults/v24/act_handover.html
  modified:
    - crates/trackly-app/src/pdf/html_templates.rs

key-decisions:
  - "v24 snapshot taken from current HEAD (post-Phase-35 body) BEFORE any pagination edit — enforced as byte-identical via diff in Task 1 verification, per Pitfall 7/C-01 (this exact ordering trap has hit the project 3 times before: Phase 34 D-15, Phase 35 C-01/WR-01, quick 260704-uw3)"
  - "act_acceptance.html slice intentionally left untouched (4 elements, v20..v23) — that template is not modified this phase, so no v24 snapshot is needed for it"

patterns-established: []

requirements-completed: []  # DOC-10/DOC-11 are delivered by the pagination rewrite in 36-02; this plan only lays the upgrade-path groundwork and is not itself requirement-complete

# Metrics
duration: 8min
completed: 2026-08-12
---

# Phase 36 Plan 01: v24 legacy-defaults snapshot + registration Summary

**Snapshotted `act_handover.html` into `_legacy_defaults/v24/` before any pagination edit, registered it as the fifth `KNOWN_LEGACY_DEFAULTS` element, and added the mirrored regression test — structurally guaranteeing already-installed copies receive Plan 02's pagination rewrite instead of only fresh installs.**

## Performance

- **Duration:** 8 min
- **Started:** 2026-08-12T15:44:00Z
- **Completed:** 2026-08-12T15:49:36Z
- **Tasks:** 2 completed
- **Files modified:** 2 (1 created, 1 modified)

## Accomplishments
- `_legacy_defaults/v24/act_handover.html` created as a byte-identical copy of the live template at the current HEAD (confirmed via `diff`, no output)
- `KNOWN_LEGACY_DEFAULTS`'s `"act_handover.html"` slice extended from 4 to 5 elements (v20, v21, v22, v23, v24); `"act_acceptance.html"` slice left at 4 elements as required
- New test `upgrade_replaces_v24_legacy_default_with_current_bundled_body` added, mirroring `upgrade_replaces_v23_legacy_default_with_current_bundled_body` 1:1 (index 4, `assert_ne!(v24_body, current, ...)` precondition guard included)

## Task Commits

Each task was committed atomically:

1. **Task 1: Снять срез v24 и зарегистрировать в KNOWN_LEGACY_DEFAULTS** - `2c0df06` (feat)
2. **Task 2: Регрессионный тест upgrade_replaces_v24_...** - `9b783a7` (test)

**Plan metadata:** (this commit, docs: complete plan)

## Files Created/Modified
- `crates/trackly-app/templates/_legacy_defaults/v24/act_handover.html` - byte-identical snapshot of the live template at Phase-36 start
- `crates/trackly-app/src/pdf/html_templates.rs` - fifth `KNOWN_LEGACY_DEFAULTS` slice element (v24) + new mirrored regression test

## Decisions Made
- Snapshot taken as the very first action of the phase, strictly before touching the live template — confirmed empty `diff` between the live file and the new snapshot at the moment of creation (Pitfall 7/C-01 mitigation).
- Did not create a `v24/act_acceptance.html` sibling: that file is out of scope for Phase 36 (confirmed against CONTEXT.md/RESEARCH.md), so no snapshot or registry change was needed for it.

## Deviations from Plan

### Known Issue (not a bug — structural, expected, self-resolving)

**1. `upgrade_replaces_v24_legacy_default_with_current_bundled_body` currently fails (RED), by design**
- **Found during:** Task 2 verification (`cargo test -p trackly-app --lib pdf::html_templates -- --test-threads=1`)
- **What happens:** The new test's `assert_ne!(v24_body, current, ...)` precondition guard trips, because at this point in the phase the v24 snapshot (Task 1) is still byte-identical to the live `act_handover.html` — the live template has not been edited yet. That edit is Plan 36-02's job (`wave: 2`, `depends_on: ["36-01"]`).
- **Why this is correct, not a defect:** This is the intentional cross-plan sequencing spelled out in `36-RESEARCH.md` Pitfall 7 and `36-PATTERNS.md`'s "Legacy-defaults version-bump ritual": the v24 snapshot MUST be taken before the pagination edit lands (otherwise the upgrade mechanism silently breaks for real installs — the exact failure mode this project has hit three times before: Phase 34 D-15, Phase 35 C-01/WR-01, quick `260704-uw3`). Because the snapshot-then-test pair (this plan) and the live-file edit (36-02) are split across two separate plans instead of one, the `assert_ne!` guard is *structurally guaranteed* to fail until 36-02 executes — proving the guard itself works correctly (it is designed to catch exactly "snapshot taken after/without a real content divergence").
- **Verified stable, not flaky:** ran the full `pdf::html_templates` module twice; consistently 14 passed / 1 failed (the new v24 test) with an identical assertion message both times.
- **No code change required to fix:** once Plan 36-02 edits the live `act_handover.html` for pagination, `current` will diverge from `v24_body` and this test will pass without any further modification.
- **Files involved:** `crates/trackly-app/src/pdf/html_templates.rs`
- **Committed in:** `9b783a7` (Task 2 commit) — commit message documents the expected-RED state explicitly.

---

**Total deviations:** 0 auto-fixed. 1 documented known-issue (structural, cross-plan, self-resolving — not a code defect).
**Impact on plan:** All other acceptance criteria for both tasks are met (diff empty, grep count 1, `act_acceptance.html` slice unchanged at 4 elements, `cargo check` clean, 14/15 tests in the module green). The one exception (the new v24 test itself) is expected to be green only after 36-02 executes; this is inherent to the phase's own documented sequencing (RESEARCH.md Pitfall 7), not a scope or quality gap in this plan.

## Issues Encountered
None beyond the documented known-issue above.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Plan 36-02 (pagination rewrite of `act_handover.html`) can proceed immediately: the v24 snapshot is in place and the registry is wired, so once 36-02 edits the live template, the new `upgrade_replaces_v24_...` test will pass and prove the upgrade path works for already-installed copies.
- **Blocker for phase-level (not plan-level) verification:** `cargo test -p trackly-app --lib pdf::html_templates` will show 1 failing test until 36-02 lands. This is expected and self-resolving — no action needed beyond executing 36-02 next.

---
*Phase: 36-act-pagination*
*Completed: 2026-08-12*
