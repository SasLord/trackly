---
phase: 20-print-acts-org
plan: 05
subsystem: pdf-html-render-tests
tags: [tests, regression, nyquist, security, xss, org-requisites]
requires:
  - phase: 20-print-acts-org
    provides: "render_acceptance_pdf org-parity via org_db.get_for_pdf (Plan 20-02)"
  - phase: 20-print-acts-org
    provides: "OrgPatch.address_line2 / OrgSettingsDto.address_line2 (Plan 20-01)"
  - phase: 20-print-acts-org
    provides: "act_acceptance.html header parity + address_line2 in all three templates (Plan 20-03)"
provides:
  - "html_acceptance_full_org_parity_with_handover — PRN-01 parity regression test"
  - "html_svg_logo_with_script_embeds_img_only_no_inline_script — ORG-01/D-09 XSS regression test"
  - "html_report_org_header_shows_address_line2 — ORG-02 report-template coverage"
  - "logo_test_with_script.svg — malicious SVG fixture"
affects:
  - "Phase 20 verification — every requirement (PRN-01/ORG-01/ORG-02) now has a concrete automated test"
tech-stack:
  added: []
  patterns:
    - "OrgDbService::save_fields / save_logo production write-path used in tests (no hand-built ctx)"
    - "negative + non-vacuous-positive assertion pairing for security invariants (mirrors html_is_offline_safe_no_external_links)"
key-files:
  created:
    - crates/trackly-app/tests/fixtures/logo_test_with_script.svg
  modified:
    - crates/trackly-app/tests/html_act_render.rs
    - crates/trackly-app/tests/html_report_render.rs
key-decisions:
  - "Task 1 was committed by the sonnet executor before a session-quota interruption; Tasks 2-3 completed inline by the orchestrator (Opus) after the subagent hit its session limit — same code, same patterns, all committed atomically"
requirements-completed: [PRN-01, ORG-01, ORG-02]
duration: 40min
completed: 2026-07-14
---

# Phase 20 Plan 05: PRN-01/ORG-01/ORG-02 regression test suite Summary

Locked in the three central invariants of Phase 20 with automated, re-runnable tests: (1) PRN-01 — `render_acceptance_pdf` now carries the full organizational requisite set at parity with `render_pdf`; (2) ORG-01/D-09 — an SVG logo containing an embedded `<script>` is embedded EXCLUSIVELY as a `data:` URI inside `<img>`, never as inline executable markup; (3) ORG-02 — `address_line2` renders in `report.html` (acts covered by the Task 1 parity test). Nyquist validation for the phase: every requirement gets a concrete test, not just code review.

## Performance

- **Duration:** ~40 min wall (spanning a session-quota interruption + inline completion)
- **Tasks:** 3 completed
- **Files:** 1 created (fixture), 2 modified (test files)

## Accomplishments
- **Task 1 (PRN-01):** `html_acceptance_full_org_parity_with_handover` — populates every org requisite (inn/kpp/address/phone/fax/email/okpo/ogrn/**address_line2**) via the production `OrgDbService::save_fields` path, renders BOTH `render_pdf` (handover) and `render_acceptance_pdf`, and asserts all 7 distinguishing values appear in BOTH HTML strings.
- **Task 2 (ORG-01/D-09):** `html_svg_logo_with_script_embeds_img_only_no_inline_script` — saves a malicious `<script>`-bearing SVG logo via `OrgDbService::save_logo`, renders, and asserts three conditions: (a) `!html.contains("<script>")`, (b) `data:image/svg+xml;base64,` present (non-vacuous), (c) `<img src="data:image/svg+xml;base64,` present (img-only embedding). New fixture `logo_test_with_script.svg`.
- **Task 3 (ORG-02):** `html_report_org_header_shows_address_line2` — populates `address_line2` on the report org and asserts it renders in `report.html`, completing ORG-02 coverage across all three printed document types.
- **Verification:** `cargo test -p trackly-app --test html_act_render --test html_report_render` — 10/10 + 8/8 passed, including all three new tests.

## Task Commits

1. **Task 1: PRN-01 parity test** - `dee9c1c` (test) — committed by sonnet executor before quota interruption
2. **Task 2: ORG-01/D-09 SVG-script img-only regression test + fixture** - `474e4e8` (test)
3. **Task 3: ORG-02 address_line2 in report.html** - `48bc4b0` (test)

## Files Created/Modified
- `crates/trackly-app/tests/fixtures/logo_test_with_script.svg` - malicious SVG fixture (`<script>alert('xss')</script>` + `<rect>`)
- `crates/trackly-app/tests/html_act_render.rs` - added `LOGO_SVG_WITH_SCRIPT` const + Task 1 and Task 2 tests
- `crates/trackly-app/tests/html_report_render.rs` - added Task 3 test

## Decisions Made
- All three tests use the production write path (`OrgDbService::save_fields` / `save_logo`), never a hand-constructed ctx or direct DB write — mirrors the plan's `<interfaces>` guidance and the existing `html_handover_contains_required_blocks_and_logo` test.
- Task 3 relies on the positive-case assertion only: the `{% if org.address_line2 %}` guard is exercised identically to the already-tested phone/fax/email guards, so no new negative assertion was added (per the plan's own action note).

## Deviations from Plan

### Execution-mode deviation (not a code defect)

**1. Plan split across two executors due to a session-quota limit**
- **Found during:** Task 2 execution
- **Issue:** The assigned sonnet executor committed Task 1 (`dee9c1c`), created the Task 2 fixture, then hit the account session limit (resets 11am Asia/Krasnoyarsk) and terminated before writing the Task 2/3 tests or the SUMMARY.
- **Resolution:** With the working tree intact (fixture untracked, Task 1 committed) and the user directing continuation, the orchestrator (Opus) completed Tasks 2 and 3 inline — same file patterns, same production write-paths, tests verified green (10/10 + 8/8) before committing atomically. No code differs from what the plan specified.
- **Files modified:** as planned (no extra files)
- **Committed in:** `474e4e8`, `48bc4b0`

---

**Total deviations:** 1 execution-mode note (no code auto-fixes; Rules 1-3 did not apply)
**Impact on plan:** None on deliverables. All three tasks delivered exactly as specified and pass.

## Issues Encountered
Session-quota interruption of the sonnet executor mid-plan (documented above). Resolved by inline completion.

## User Setup Required
None.

## Next Phase Readiness
- PRN-01, ORG-01, ORG-02 all have concrete automated regression tests — the phase's Nyquist validation is satisfied.
- Only Plan 20-06 (D-12 template auto-upgrade for existing installs) remains in Phase 20.
- No blockers.

---
*Phase: 20-print-acts-org*
*Completed: 2026-07-14*

## Self-Check: PASSED

- FOUND: crates/trackly-app/tests/fixtures/logo_test_with_script.svg
- FOUND: html_svg_logo_with_script_embeds_img_only_no_inline_script in html_act_render.rs
- FOUND: html_report_org_header_shows_address_line2 in html_report_render.rs
- FOUND commit dee9c1c (Task 1), 474e4e8 (Task 2), 48bc4b0 (Task 3)
- TESTS: html_act_render 10/10 pass, html_report_render 8/8 pass
