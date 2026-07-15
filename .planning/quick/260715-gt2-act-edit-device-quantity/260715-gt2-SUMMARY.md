---
quick_id: 260715-gt2
slug: act-edit-device-quantity
subsystem: ui
tags: [svelte, acts, act-service, rusqlite, forms]

# Dependency graph
requires:
  - phase: 19 (Plan 19-08/19-09)
    provides: ActFormItemsTable retained-vs-new discriminator (complectation_at_time),
      ActUpdateDto full-replacement items contract, ActService::update added/removed diff loop
provides:
  - Editable, group-bounded quantity input for freshly-added non-serial positions
    when editing an existing act (previously hard-locked to qty=1)
  - Backend regression test proving ActService::update's added-device loop is
    N-safe (not just N=1-safe)
  - ActFormBody edit-submit flatMap expansion of group_ids for multi-qty fresh rows
affects: [acts, act-editing]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Retained-vs-fresh row discriminator (complectation_at_time !== undefined) now
       gates BOTH the device-cell readonly render and the qty-cell editable/static
       render, keeping both UI restrictions in sync with one marker"

key-files:
  created: []
  modified:
    - crates/trackly-app/tests/acts_update.rs
    - ui/src/features/acts/ActFormItemsTable.svelte
    - ui/src/features/acts/ActFormBody.svelte

key-decisions:
  - "No backend DTO or service change needed — ActService::update's added: Vec<i64>
     loop was already N-safe; confirmed by a dedicated N=3 regression test rather
     than assumed"
  - "Gate qty-editability precisely on complectation_at_time !== undefined (retained)
     OR has_serial, not on mode==='edit' — this scopes the fix to exactly the case
     described in the problem statement without touching retained/serial positions"

patterns-established:
  - "Submit-side flatMap expansion (retained row -> 1 entry, fresh row with
     quantity>1 -> group_ids.slice(0,quantity) entries) mirrors the existing
     create-branch expansion pattern, reused verbatim rather than reinvented"

requirements-completed: []

# Metrics
duration: 20min
completed: 2026-07-15
---

# Quick Task 260715-gt2: Allow qty>1 for newly-added positions when editing an act

**Editing an act's device list now lets a user add e.g. 3 keyboards at once (qty-editable, stock-bounded input), instead of forcing three separate one-at-a-time rows — backend was already N-safe, this was purely a UI gap closed via one discriminator flag plus a submit-side flatMap expansion.**

## Performance

- **Duration:** ~20 min
- **Tasks:** 3 completed
- **Files modified:** 3 (1 backend test file, 2 frontend components)

## Accomplishments

- Proved (via a new N=3 regression test) that `ActService::update`'s `added: Vec<i64>` loop already correctly transitions and destocks multiple newly-added devices in one call — no backend fix needed.
- Made the quantity field editable and group-bounded for freshly-added, non-serial positions when editing an act, matching create-mode behavior exactly, while retained and serialised positions remain fixed at quantity 1.
- Wired the edit-mode submit payload to expand a multi-qty fresh row into N `ActUpdateItemDto` entries via the same `group_ids.slice(0, quantity)` pattern the create branch already uses.

## Task Commits

1. **Task 1: Backend regression test for multi-device add** — `e3ab329` (test)
2. **Task 2: Make qty editable for fresh non-serial rows in edit mode** — `ae996bc` (feat)
3. **Task 3: Expand multi-qty fresh positions into N entries on edit submit** — `644278a` (feat)
4. **Fmt fixup for Task 1's new assertions** — `a5c31bc` (style, Rule 1)

## Files Created/Modified

- `crates/trackly-app/tests/acts_update.rs` — new `add_multiple_positions_transitions_all_devices` test (seeds 1 original + 3 new devices, asserts all 4 attach, all 3 new transition to `в_работе`/act location, and each gets exactly one audit row).
- `ui/src/features/acts/ActFormItemsTable.svelte` — `pickDevice`/`pickGroup` no longer force `quantity: 1` purely because `mode === 'edit'`; qty-cell render gate narrowed from `mode === 'edit'` to `mode === 'edit' && (row.complectation_at_time !== undefined || row.has_serial)`; updated the WR-02 comments and the `FormItemRow.complectation_at_time` doc comment to describe the GT2 supersession while preserving the retained-position marker semantics.
- `ui/src/features/acts/ActFormBody.svelte` — edit-branch `handleSubmit` now builds `updateItems` via `.flatMap()`: retained rows (`complectation_at_time !== undefined`) still emit exactly one entry; fresh rows expand `group_ids.slice(0, quantity)` (falling back to `[device_id]` if `group_ids` is empty) into N entries with `complectation_at_time: null`. Rebuilt `ui/dist` via `pnpm --dir ui build`.

## Decisions Made

- Confirmed via the new backend test that no `ActUpdateDto`/`ActService::update` change was required — the existing `added: Vec<i64>` loop already handles N>1 correctly. This matches the plan's stated expectation and avoided any backend churn.
- The qty-cell and device-cell (readonly) render gates both key off the same `complectation_at_time !== undefined` marker, keeping the retained-vs-fresh distinction in exactly one place conceptually (plus `has_serial` for the W-5 serial-always-qty-1 rule).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug/Lint] Fixed rustfmt violations in the new test's assertions**
- **Found during:** Final gate check (`cargo fmt --check`)
- **Issue:** Two multi-arg `assert_eq!` calls in the new `add_multiple_positions_transitions_all_devices` test exceeded rustfmt's line-width preference and needed to be wrapped onto multiple lines (rustfmt 1.8.0 / toolchain 1.92.0, matching the pinned `rust-toolchain.toml`).
- **Fix:** Manually wrapped the two `assert_eq!` calls (lines ~297 and ~311) to match rustfmt's expected output; verified via `cargo fmt --check -- crates/trackly-app/tests/acts_update.rs` that no NEW diff locations remain in the file beyond pre-existing ones (see note below).
- **Files modified:** `crates/trackly-app/tests/acts_update.rs`
- **Verification:** Diffed `cargo fmt --check` output before vs. after the fix — the fix eliminated the 2 diff locations introduced by the new test; the file's line-shifted pre-existing diff count matched the pre-task baseline exactly (12 before, 12 after, just at different line numbers).
- **Committed in:** `a5c31bc`

---

**Total deviations:** 1 auto-fixed (Rule 1, formatting)
**Impact on plan:** Cosmetic only; no functional change. No scope creep.

## Issues Encountered

**Pre-existing `cargo fmt --check` drift (out of scope, not fixed).** Running `cargo fmt --check` against the pre-task baseline (commit `efd69b6`) already shows 38 diff locations across files this quick task never touches (`crates/trackly-app/src/dto/act.rs`, `crates/trackly-app/src/services/act_service.rs`, `crates/trackly-app/tests/acts_update_return.rs`, `crates/trackly-app/tests/acts_date_source.rs`, `crates/trackly-app/tests/acts_archived_at.rs`, `crates/trackly-app/tests/html_act_render.rs`, `crates/trackly-infra/src/repos/audit_log_sqlite.rs`, and pre-existing lines within `acts_update.rs` itself). This predates this quick task entirely — confirmed by checking out `efd69b6` in a throwaway worktree and running the same command. Per the SCOPE BOUNDARY rule ("only auto-fix issues DIRECTLY caused by the current task's changes... pre-existing... failures in unrelated files are out of scope"), these were left untouched and are NOT part of this task's commits. `cargo fmt --check` as an absolute repo-wide gate will not pass until this pre-existing drift is addressed in a separate cleanup task — this quick task's own new/modified code is fmt-clean (verified: the only 2 new diff locations introduced by the new test were fixed in commit `a5c31bc`; all other diff locations in `acts_update.rs` after the fix match the pre-task baseline exactly, just shifted by +3 lines from the insertion).

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Feature is complete and self-contained: backend proven N-safe, frontend UI + submit payload wired end-to-end.
- `pnpm --dir ui build` succeeded, so `ui/dist` reflects the change for server-mode/LAN-browser users immediately.
- No blockers for future work. The pre-existing repo-wide `cargo fmt --check` drift noted above is worth a dedicated cleanup task at some point but does not block this quick task.

---
*Quick task: 260715-gt2*
*Completed: 2026-07-15*

## Self-Check: PASSED

All created/modified files confirmed present; all 4 commit hashes (e3ab329, ae996bc, 644278a, a5c31bc) confirmed in git log.
