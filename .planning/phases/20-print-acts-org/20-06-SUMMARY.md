---
phase: 20-print-acts-org
plan: 06
subsystem: html-template-lifecycle
tags: [templates, startup, migration, d-12, upgrade-in-place, fail-closed]
requires:
  - phase: 20-print-acts-org
    provides: "Plan 20-03 header parity + address_line2 in bundled act_handover/act_acceptance/report.html"
provides:
  - "KNOWN_LEGACY_DEFAULTS registry (byte-for-byte pre-Phase-20 snapshots)"
  - "upgrade_untouched_defaults_on_startup — overwrites untouched on-disk templates with current bundled body, fail-closed on customized files"
  - "context.rs startup wiring (runs after materialize_defaults_on_startup)"
  - "3 regression tests (legacy-upgrade / customized-untouched / already-current no-op)"
affects:
  - "All existing installs — Phase 20 template changes (PRN-01/ORG-02) now reach on-disk templates, not just fresh installs"
tech-stack:
  added: []
  patterns:
    - "include_str! compile-time embed of checked-in legacy snapshot files (mirrors DEFAULT_HTML_TEMPLATES)"
    - "byte-identity untouched-detection (structural, no is_default metadata column — the file-based analog of template_service's DB is_default flag)"
key-files:
  created:
    - crates/trackly-app/templates/_legacy_defaults/v20/act_handover.html
    - crates/trackly-app/templates/_legacy_defaults/v20/act_acceptance.html
    - crates/trackly-app/templates/_legacy_defaults/v20/report.html
  modified:
    - crates/trackly-app/src/pdf/html_templates.rs
    - crates/trackly-app/src/context.rs
key-decisions:
  - "Tasks 1 and 3 both edit html_templates.rs → committed together as one feat commit (registry+function+tests); Task 2 (context.rs wiring) is a separate commit. Executed inline by orchestrator after the sonnet subagent's session-quota limit."
requirements-completed: [PRN-01, ORG-02]
duration: 20min
completed: 2026-07-14
---

# Phase 20 Plan 06: Auto-upgrade untouched HTML template defaults (D-12) Summary

Closed the exact recurrence of the `[[db_backed_templates_upgrade_trap]]` for file-based templates: `materialize_defaults_on_startup` is insert-only and `load_template` prefers the on-disk copy, so Plan 20-03's header-parity / `address_line2` bundle changes would only reach fresh installs. Added `upgrade_untouched_defaults_on_startup` — it overwrites an on-disk template with the current bundled body ONLY when the on-disk content is byte-identical to a known prior default (provably untouched), and leaves anything else alone (fail closed). Wired into the single `AppCtx::build` startup path shared by desktop and server mode.

## Performance

- **Duration:** ~20 min
- **Tasks:** 3 completed
- **Files:** 3 created (legacy snapshots), 2 modified

## Accomplishments
- **Task 1:** `KNOWN_LEGACY_DEFAULTS: &[(&str, &[&str])]` registry + three byte-for-byte pre-Phase-20 snapshot files under `templates/_legacy_defaults/v20/` (captured from pinned commit `8f82339`, verified byte-identical via `git hash-object`). `upgrade_untouched_defaults_on_startup(&Path) -> Result<(), AppError>` implements the no-op / safe-upgrade / fail-closed decision tree, mirroring `materialize_defaults_on_startup`'s error-wrapping style.
- **Task 2:** Wired into `context.rs` immediately after `materialize_defaults_on_startup(&html_templates_dir)?`, reusing the same `html_templates_dir` binding — no new variable, no new resolution call.
- **Task 3:** Three regression tests in the existing `#[cfg(test)] mod tests`: (1) `upgrade_replaces_untouched_legacy_default_with_current_bundled_body` pre-writes OLD legacy content for all three files and asserts they become the current body — realistic pre-existing-install disk state, not a fresh TempDir; (2) `upgrade_leaves_user_customized_file_untouched` proves a non-default/non-legacy file is never overwritten; (3) `upgrade_is_noop_when_file_already_current`.
- **Verification:** `cargo check -p trackly-app --all-targets` exits 0; `cargo test -p trackly-app --lib pdf::html_templates::tests` — 8/8 passed (3 new).

## Task Commits

1. **Task 1 + Task 3 (shared file html_templates.rs): registry + function + regression tests + 3 snapshots** - `fcf8409` (feat)
2. **Task 2: context.rs startup wiring** - `abf9a4e` (feat)

## Files Created/Modified
- `crates/trackly-app/templates/_legacy_defaults/v20/{act_handover,act_acceptance,report}.html` - checked-in pre-Phase-20 default snapshots (compile-time `include_str!` sources, not runtime-materialized)
- `crates/trackly-app/src/pdf/html_templates.rs` - `KNOWN_LEGACY_DEFAULTS`, `upgrade_untouched_defaults_on_startup`, 3 tests
- `crates/trackly-app/src/context.rs` - startup call after materialize

## Decisions Made
- **Untouched detection is structural (byte-identity), not metadata-based.** File templates have no `is_default` companion column like `template_service.rs`'s DB rows; the registry of every previously-shipped default body is the file-based equivalent. Documented extension point in the const's doc-comment instructs future phases to append a new `_legacy_defaults/vNN/` snapshot when a bundled body changes again.
- **Fail closed on ambiguity.** Any content matching neither the current default nor a known legacy default is treated as user-customized and never overwritten (T-20-06-01 mitigation), locked by Test 2.

## Deviations from Plan

### Commit granularity (not a code defect)

**1. Tasks 1 and 3 committed together**
- **Reason:** Both tasks edit `html_templates.rs` (Task 1 adds the registry+function, Task 3 adds tests to the same file's `mod tests`). With inline (non-`-p`) editing they land in one working-tree state, so they were committed as a single atomic `feat` commit rather than split mid-file. Task 2 (context.rs) is its own commit.
- **Impact:** None on deliverables; all three tasks' acceptance criteria met and verified.

### Execution-mode deviation (not a code defect)

**2. Plan executed inline by the orchestrator**
- **Reason:** The Phase 20 sonnet executor hit its account session limit during Plan 20-05; to avoid another failed spawn, the orchestrator (Opus) executed 20-06 inline per the user's continuation directive. Same code, same patterns as specified.

---

**Total deviations:** 2 process notes (no code auto-fixes; Rules 1-3 did not apply)
**Impact on plan:** None on functionality.

## Issues Encountered
Initial `git show` snapshot capture failed once due to zsh parsing `$PIN:c…` as a modifier; fixed by quoting the ref (`"${PIN}:path"`). Snapshots then verified byte-identical to the pinned git objects.

## User Setup Required
None. On next app startup, any existing install whose HTML templates are still on their pre-Phase-20 bundled default will be auto-upgraded silently.

## Next Phase Readiness
- Phase 20 is functionally complete: PRN-01 (acceptance-PDF requisite parity), ORG-01 (SVG-logo img-only invariant, regression-tested), and ORG-02 (`address_line2` across data layer, all three templates, UI, and existing-install delivery) are all delivered and test-locked.
- No blockers.

---
*Phase: 20-print-acts-org*
*Completed: 2026-07-14*

## Self-Check: PASSED

- FOUND: crates/trackly-app/templates/_legacy_defaults/v20/act_handover.html (+ act_acceptance.html, report.html)
- FOUND: pub fn upgrade_untouched_defaults_on_startup in html_templates.rs
- FOUND: KNOWN_LEGACY_DEFAULTS in html_templates.rs
- FOUND: upgrade_untouched_defaults_on_startup call in context.rs
- FOUND commit fcf8409 (Task 1+3), abf9a4e (Task 2)
- TESTS: pdf::html_templates::tests 8/8 pass
