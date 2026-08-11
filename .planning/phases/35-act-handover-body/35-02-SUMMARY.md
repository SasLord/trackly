---
phase: 35-act-handover-body
plan: 02
subsystem: templates
tags: [minijinja, html-templates, act-handover, print-css]

# Dependency graph
requires:
  - phase: 35-act-handover-body
    plan: 01
    provides: "_legacy_defaults/v22/act_handover.html snapshot + KNOWN_LEGACY_DEFAULTS registration + act.giver_name in demo_context_for_kind, both prerequisites for safely changing the template body and reading act.giver_name in it"
provides:
  - "act_handover.html body reworked per D-01..D-12: plain-text field rows (no underline except two allowed spots), plural device-list summary for N>1, unconditional 'Сроком до' with blank-underline fallback, horizontal one-line-per-signer signature block with printed act.giver_name/act.receiver_name"
affects: [35-03, 35-04, 35-05]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "field-row as a single plain-text-run div instead of two flex spans (label|value) — D-11"
    - "MiniJinja length filter for plural branching ({% if act.items | length > 1 %}) — reused report.html's existing pattern"
    - "value-blank span as the sole remaining border-bottom placeholder outside the signature block, for handwritten fallback when a value is empty"

key-files:
  modified:
    - crates/trackly-app/templates/act_handover.html

key-decisions:
  - "Kept doc-comment wording as 'deadline field_row' instead of the literal phrase 'Сроком до' to avoid a duplicate match that would have broken Task 2's grep-count-1 acceptance check now that the body itself carries the phrase once"
  - "Used a throwaway #[ignore] integration test (crates/trackly-app/tests/_scratch_dump_handover_html.rs, deleted before commit) to render a real 3-device act to HTML and visually confirm the plural list, blank deadline underline, and horizontal signature rows — per project memory act-pdf-word-fidelity/synthetic-harness-not-verification, text-extraction assertions alone don't prove markup is structurally correct"

patterns-established: []

requirements-completed: [DOC-07, DOC-08, DOC-09]

# Metrics
duration: ~13min
completed: 2026-08-11
---

# Phase 35 Plan 02: act_handover.html body rework Summary

**Reworked act_handover.html's body per D-01..D-12: removed underlines from auto-filled field values, converted label+value to plain running text, added N>1 plural device-list handling, made "Сроком до" render unconditionally with a blank-underline fallback, and replaced the two-line signature grid with a horizontal one-line-per-signer block showing printed act.giver_name/act.receiver_name.**

## Performance

- **Duration:** ~13 min
- **Started:** 2026-08-11T12:21:08Z
- **Completed:** 2026-08-11T12:33:15Z
- **Tasks:** 3
- **Files modified:** 1 (`crates/trackly-app/templates/act_handover.html`)

## Accomplishments

- Rewrote the doc-comment header to describe the new plain-text field-row structure and the horizontal signature block, and added `act.giver_name` to the listed context keys (C-02)
- Removed `border-bottom` from the base `.field-row .value` selector (which itself no longer exists — `.field-row` lost its flex/label/value split entirely per D-11); added `.value-blank` as the sole remaining underline placeholder for an empty deadline (D-10/D-03)
- Replaced the `.signatures` CSS grid with `.signature-row`/`.signature-label`/`.signature-field`/`.signature-line`/`.signature-sublabel`/`.signature-name` classes matching the plan's `<interfaces>` spec exactly
- Intro paragraph text preserved verbatim ("Настоящим актом утверждаю, что мною: …") but now rendered as one plain-text div instead of two flex spans (D-01/D-11)
- Deleted the empty `&nbsp;` placeholder field-row (D-12)
- Added `{% if act.items | length > 1 %}` branch rendering "были получены устройства:" plus a plain `<ul>` of device names; the per-item "было получено устройство: {name}" label now only renders when `act.items | length == 1` (D-02)
- All six per-device fields (inventory_no/serial_no/model/kit/specs/condition) converted from two-span label|value markup to a single plain-text field-row (D-11)
- "Сроком до" now renders unconditionally; empty `deadline_human`/`deadline` falls back to `<span class="value-blank">` instead of hiding the row (D-03)
- Signature block rewritten as two `.signature-row` divs ("Выдал:"/"Получил:"), each with one `.signature-line` + "Подпись" sublabel + printed `{{ act.giver_name }}`/`{{ act.receiver_name }}` — no "ФИО" sublabel, no signing-date field (D-06/D-07/D-08)
- Verified with a real rendered HTML output (throwaway `#[ignore]` test, deleted before commit) for a 3-device act: plural list, blank deadline underline, and horizontal signature rows all confirmed structurally correct, not just via text-extraction assertions

## Task Commits

Each task was committed atomically:

1. **Task 1: Doc-comment and CSS — underline removal, signature/value-blank classes** - `c74a579` (feat)
2. **Task 2: Body — plain-text fields, plural device list, unconditional deadline** - `3904da9` (feat)
3. **Task 3: Horizontal signature block with printed names** - `d337c7d` (feat)

**Plan metadata:** this SUMMARY + STATE/ROADMAP updates (see final commit below)

## Files Created/Modified

- `crates/trackly-app/templates/act_handover.html` - doc-comment, CSS, and body all reworked per D-01..D-12; production render path (`act_service::render_pdf`) unchanged, no backend code touched

## Decisions Made

- Doc-comment avoids the literal quoted phrase "Сроком до" (uses "deadline field_row" instead) so the file-wide grep count for that phrase stays at exactly 1 after Task 2 (the body's single unconditional occurrence) — a plan-authoring nuance since the pre-Phase-35 doc-comment already contained that literal phrase once
- Visual/structural verification for this plan used a throwaway `#[ignore]` integration test that rendered a real 3-device act and dumped the HTML to the scratchpad directory for inspection, then deleted the test file before committing — consistent with project memory that text-extraction-only test assertions can't prove markup/layout correctness

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Doc-comment wording adjusted to avoid breaking Task 2's grep-count acceptance check**
- **Found during:** Task 1 (surfaced when verifying Task 2)
- **Issue:** Task 1's action instructed replacing the doc-comment's structural description but did not explicitly forbid the literal phrase "Сроком до"; the original doc-comment already contained that phrase once. Task 2's acceptance criteria required `grep -c "Сроком до"` to equal exactly 1 in the whole file after Task 2, which would have been 2 (doc-comment + body) had the doc-comment kept the literal phrase.
- **Fix:** Reworded the doc-comment to say "deadline field_row" instead of quoting "Сроком до" literally, keeping the description accurate while leaving exactly one literal occurrence (in the body) for the grep-based acceptance check to find.
- **Files modified:** `crates/trackly-app/templates/act_handover.html`
- **Commit:** `3904da9` (folded into the Task 2 commit, since the fix was discovered while verifying Task 2 and applied to a Task 1 line)

No other deviations — Tasks 1-3 executed as specified in the plan and `<interfaces>` block.

## Issues Encountered

None blocking. `cargo test -p trackly-app` compiles are slow (~2 min for the first `pdf_render_act` run, ~20s for warm incremental runs) — all runs were backgrounded per project convention.

## Expected Test Drift (documented in plan `<verification>`, not a regression)

Per the plan's own verification section and CONTEXT.md C-03, two existing tests now fail because they assert the removed "ФИО" sublabel — this is planned and will be fixed in Plan 04:

- `crates/trackly-app/tests/pdf_render_act.rs::signature_renders_two_line_labels` — asserts `"ФИО"` is present; D-07 removed it
- `crates/trackly-app/tests/html_act_render.rs::html_handover_contains_required_blocks_and_logo` — same assertion, same root cause

All other tests in both files pass, including `render_handover_act_contains_d09_intro_phrase`, `render_handover_act_produces_cyrillic_pdf`, `render_handover_multi_device_wraps_long_fields`, and `html_handover_multi_device_all_items_present_no_truncation` — confirming D-11's plain-text conversion did not break long-field wrapping or multi-device rendering.

## User Setup Required

None — no external service configuration required. Production render path (`act_service::render_pdf`) was not touched; the change is confined to the template file, consistent with the plan's stated scope.

## Next Phase Readiness

Plan 03 (act_acceptance.html signature-block parity, D-09) and Plan 04 (test updates for the C-03 expected drift) can proceed. `act_handover.html`'s body now matches the `<interfaces>` signature-block spec exactly, giving Plan 03 a concrete pattern to replicate. No blockers.

---
*Phase: 35-act-handover-body*
*Completed: 2026-08-11*
