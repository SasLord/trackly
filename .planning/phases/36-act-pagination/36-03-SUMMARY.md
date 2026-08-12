---
phase: 36-act-pagination
plan: 03
subsystem: testing
tags: [minijinja, act-handover, pagination, rust-tests, print-css]

# Dependency graph
requires:
  - phase: 36-act-pagination (plan 02)
    provides: "act_handover.html N=1/N>1 branching + appendix table (D-01..D-16) — the markup this plan's tests were rewritten against"
provides:
  - "html_act_render.rs: extract_first_ol (D-07), N=1 negative appendix assertions (DOC-10 SC#1), N>1 test rewritten around .device-block's full removal (D-08), plus 4 new appendix-structural tests (row-count, ol<->№ cross-check, Кол-во dash-vs-value D-03, CSS break-before/break-inside regression gate D-15/D-16)"
  - "pdf_render_act.rs: per-device field attribution re-proven against tbody.device-group instead of the retired device-block marker; render_handover_default_template_uses_field_rows_not_device_card narrowed from N=2 to N=1 (its abbreviated-labels-forbidden assertion is only valid at N=1 now — N>1's appendix <th> headers legitimately use those abbreviations, D-01)"
  - "acts_e2e_smoke.rs: handover_pdf_render_within_e2e doc-comment records that its N=2 fixture now exercises the appendix branch (assertions were already branch-inert)"
  - "html_field_row_underline_gate.rs (out-of-scope-list discovery): border-bottom regression gate widened from a closed set of 2 to 3 legitimate sources, explicitly asserting the new .appendix-table thead tr hairline (D-05) rather than silently loosening the count check"
affects: [36-04-act-pagination, 36-05-act-pagination]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Partial-prefix string split on `<tbody class=\"device-group` (not the full class attribute, which also carries a zebra-cycle suffix like ` row-even\"`) to isolate one row-group's HTML per device for attribution assertions — same idiom used for both the row-count test and the fields-attributable-to-own-device rewrite"
    - "Read-only include_str! + regex rule-body extraction for CSS-structural regression gates (break-before/break-inside, border-bottom exhaustiveness) — mirrors html_page_parity.rs's existing pattern, extended to accept a selector argument instead of a fixed one"

key-files:
  modified:
    - crates/trackly-app/tests/html_act_render.rs
    - crates/trackly-app/tests/pdf_render_act.rs
    - crates/trackly-app/tests/acts_e2e_smoke.rs
    - crates/trackly-app/tests/html_field_row_underline_gate.rs

key-decisions:
  - "render_handover_default_template_uses_field_rows_not_device_card (pdf_render_act.rs) narrowed from N=2 to N=1, not deleted: its core assertion (full-length field-row labels, no abbreviated legacy labels, no device-card heading) is genuinely only true at N=1 post-D-08 — N>1's appendix table legitimately renders abbreviated <th> headers (Инв.№/Серийный №) by design (D-01), so keeping the test at N=2 would make it permanently red against correct behavior, not a regression detector."
  - "act_items.quantity is hardcoded to 1 by ActService::create's legacy clone-on-handover INSERT (act_service.rs:411) for every effective device — there is no production code path that creates an act_item with quantity > 1 through the public API. The new html_handover_appendix_quantity_column_dash_at_one_value_at_more test exercises the >1 branch via a direct UPDATE act_items SET quantity, the same direct-DB-manipulation idiom already used elsewhere in this suite for complectation_at_time/condition_at_time."
  - "Did NOT call requirements.mark-complete for DOC-10/DOC-11 in this plan, following 36-01/36-02's established precedent (test/template closure spans 36-02..36-05; 36-04 still owes the Paged.js thead-repeat handler D-15a, 36-05 owes live-PDF verification on both transports)."

requirements-completed: []  # Intentionally not marked — see key-decisions; DOC-10/DOC-11 closure spans 36-02..36-05 per 36-CONTEXT.md and prior plans' precedent.

# Metrics
duration: 75min
completed: 2026-08-12
---

# Phase 36 Plan 03: Test-suite rewrite for N=1/N>1 appendix markup Summary

**Closed the plan-anticipated test drift from Plan 02's appendix rewrite (3 known-red tests) plus one additional drift the full-suite run surfaced outside the plan's enumerated list — full `cargo test -p trackly-app` (minus the pre-existing unrelated hang) now passes 0 failed, with 4 new appendix-structural regression tests added.**

## Performance

- **Duration:** ~75 min (includes ~35 min of full-suite compile+run time across two full-suite runs)
- **Started:** 2026-08-12T23:15:00Z
- **Completed:** 2026-08-13T00:31:00Z
- **Tasks:** 3 completed
- **Files modified:** 4 (3 planned + 1 discovered)

## Accomplishments
- `extract_first_ul` renamed to `extract_first_ol` in `html_act_render.rs`, searching for `<ol class="` (partial-attribute match, since the real tag also carries `device-summary`) instead of the retired literal `<ul>` (D-07).
- N=1 test (`html_handover_single_device_renders_singular_intro_not_plural_summary`) gained two negative assertions: no `<ol class=` summary list, and no `<div class="appendix">` / `<table class="appendix-table">` element at all — DOC-10 SC#1 is now verified both positively (the singular flow renders) and negatively (nothing from the N>1 branch leaks in).
- N>1 test (`html_handover_multi_device_renders_plural_summary_listing_every_name`) rewritten: dropped the retired "each `.device-block` keeps its own singular label" assertion (structurally false by construction post-D-08) and replaced it with an explicit assertion that `.device-block` and the singular label string are both completely absent from the first sheet at N>1.
- Four new appendix-structural tests added to `html_act_render.rs`: one `tbody.device-group` per device (row-count), `<ol>` numbering matching the appendix table's № column 1:1 in both order and printed value (D-07's whole reason for existing), the Кол-во column's dash-vs-numeric branching (D-03, exercised via a direct DB `UPDATE` since the public API always inserts `quantity=1`), and a read-only CSS-structural gate confirming `break-before: page` on `.appendix` and `break-inside: avoid` on `.appendix-table tbody.device-group` (D-15/D-16 — the cheap Nyquist-audit-mandated regression replacement for a geometric layout check).
- `pdf_render_act.rs`'s `render_handover_multi_device_fields_attributable_to_own_device` re-proven against `tbody.device-group` (partial-prefix split) instead of the retired `.device-block` marker — same per-device attribution guarantee (own name/field present, other devices' name/field absent), now against the new markup.
- `render_handover_default_template_uses_field_rows_not_device_card` narrowed from N=2 to N=1 (see key-decisions) — its remaining scope (no device-card heading, no abbreviated labels, full-length field-row labels) is exactly what D-08 left byte-identical to Phase 35 for the N=1 branch.
- `acts_e2e_smoke.rs`'s `handover_pdf_render_within_e2e` doc-comment updated to record that its 2-device fixture now exercises the appendix branch; its three assertions (byte length, `<html` marker, Cyrillic `receiver_name`) needed no code change — they were already branch-inert.
- **Additional drift found outside the plan's enumerated list** (see Deviations): `html_field_row_underline_gate.rs`'s `field_row_css_has_no_border_bottom_and_only_two_legit_exceptions_remain` failed on the full-suite run because Plan 02 added a third legitimate `border-bottom` source (`.appendix-table thead tr`, D-05's print-color-fallback hairline). Fixed and renamed to `..._only_legit_exceptions_remain`, now asserting a closed set of exactly 3 sources instead of silently widening the check to "anything goes".

## Task Commits

Each task was committed atomically:

1. **Task 1: html_act_render.rs — extract_first_ol, N=1/N>1 tests, appendix-structural tests** - `fd1f01c` (test)
2. **Task 2: pdf_render_act.rs — atribution on tbody.device-group, wraps_long_fields confirmation** - `66ef269` (test)
3. **Task 3: acts_e2e_smoke.rs comment + full-suite gate (includes newly-discovered drift fix)** - `2b8662a` (test)

**Plan metadata:** (this commit, docs: complete plan)

## Files Created/Modified
- `crates/trackly-app/tests/html_act_render.rs` — extract_first_ol, rewritten N=1/N>1 tests, 4 new appendix-structural tests, top-level `ACT_HANDOVER_HTML` const for the CSS gate.
- `crates/trackly-app/tests/pdf_render_act.rs` — attribution split marker changed to `tbody.device-group`; default-template test narrowed to N=1; doc-comments updated on `wraps_long_fields`.
- `crates/trackly-app/tests/acts_e2e_smoke.rs` — doc-comment only, no assertion changes.
- `crates/trackly-app/tests/html_field_row_underline_gate.rs` — widened the border-bottom closed-set gate from 2 to 3 legitimate sources (not in the plan's `files_modified`, discovered via the full-suite run).

## Decisions Made
- Narrowed `render_handover_default_template_uses_field_rows_not_device_card` to N=1 rather than deleting it or splitting it into two tests — its assertion intent (no device-card style, full-length labels) is only meaningful at N=1 post-D-08; the abbreviated-header behavior it used to forbid is now legitimate appendix design, already covered by the new appendix-structural tests in `html_act_render.rs`.
- Exercised the Кол-во `>1` branch via a direct `UPDATE act_items SET quantity` rather than trying to reach it through `ActService::create`'s public API, because that path's legacy clone-on-handover model always inserts `quantity=1` per row (act_service.rs:411) — there is currently no production code path producing `quantity > 1` on a handover act_item. This is a pre-existing backend characteristic, out of this plan's scope (frontend/backend untouched, only test fixtures).
- Fixed the `html_field_row_underline_gate.rs` drift in-scope (Rule 1 — pre-existing test broken by Plan 02's legitimate CSS addition, not a template defect) rather than deferring it, per the plan's explicit instruction to fix any full-suite drift found beyond the enumerated 3-test list and record it here.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `html_field_row_underline_gate.rs` drift not in the plan's enumerated 3-test list**
- **Found during:** Task 3's full-suite verification run
- **Issue:** `field_row_css_has_no_border_bottom_and_only_two_legit_exceptions_remain` asserted exactly 2 `border-bottom` declarations in `act_handover.html`'s `<style>` block. Plan 02 (D-05) legitimately added a third: `.appendix-table thead tr { border-bottom: 0.5pt solid #999; }` — a print-color-fallback hairline under the appendix table header, structurally unrelated to `.field-row`/D-10's handwriting-underline concern.
- **Fix:** Widened the gate to assert exactly 3 sources, added an explicit assertion that `.appendix-table thead tr` is the third legitimate source (so the check stays a closed, regression-detecting set rather than being silently loosened), renamed the test from `..._only_two_legit_exceptions_remain` to `..._only_legit_exceptions_remain`, and expanded the module doc-comment to explain why the third source is legitimate.
- **Files modified:** `crates/trackly-app/tests/html_field_row_underline_gate.rs`
- **Verification:** `cargo test -p trackly-app --test html_field_row_underline_gate` — 3/3 green; full-suite re-run afterward confirmed 0 failed.
- **Committed in:** `2b8662a` (part of Task 3's commit)

No other deviations — the three planned tasks executed as specified in `36-03-PLAN.md`.

## Known Test Drift Status

All test drift documented in `36-02-SUMMARY.md` is now closed:
- `html_act_render.rs::html_handover_multi_device_renders_plural_summary_listing_every_name` — closed in Task 1.
- `pdf_render_act.rs::render_handover_default_template_uses_field_rows_not_device_card` — closed in Task 2 (narrowed to N=1).
- `pdf_render_act.rs::render_handover_multi_device_fields_attributable_to_own_device` — closed in Task 2.

Plus one additional drift discovered during this plan's own full-suite verification (not present at Plan 02's handoff, since Plan 02 never ran the full suite — only its own template-facing subset): `html_field_row_underline_gate.rs::field_row_css_has_no_border_bottom_and_only_two_legit_exceptions_remain`, closed in Task 3.

## Verification Evidence

- `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test html_act_render -- --test-threads=1` — 19/19 green (after Task 1).
- `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test pdf_render_act -- --test-threads=1` — 15/15 green (after Task 2).
- `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test html_field_row_underline_gate -- --test-threads=1` — 3/3 green (after the Task 3 drift fix).
- **Full suite:** `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app -- --test-threads=1 --skip login_remember_persistent_cookie` — 0 failed (90 `test result: ok` blocks). The `--skip login_remember_persistent_cookie` flag was required per project memory (`workspace_test_hangs_auth_remember_cookie`): this pre-existing hang reproduces on `-p trackly-app` alone, not only `--workspace`, and is unrelated to this plan's scope — the plan's own literal verification command (without `--skip`) was attempted first and confirmed to hang (killed after ~17 min with no output), consistent with that memory.
- `pnpm --dir ui lint` — green (eslint, prettier --check, check-tokens, check-contrast, check-focus-outline, check-pagedjs-csp-hash, check-print-isolation all passed). Confirms the new appendix-table CSS classes didn't regress print-isolation invariants (C-03) even though no frontend files were touched by this plan.

## Issues Encountered
- The plan's literal verification command (`cargo test -p trackly-app -- --test-threads=1`, no `--skip`) hung for ~17 minutes with zero output before being killed — this reproduces the pre-existing `login_remember_persistent_cookie` hang documented in project memory, which the plan text itself did not account for (the plan's own critical_constraints section, quoting the same memory, only warns against `--workspace`). Re-ran with `--skip login_remember_persistent_cookie` per that memory's explicit guidance; this is the command whose output is cited above as the plan's real verification evidence.

## User Setup Required
None — no external service configuration required.

## Next Phase Readiness
- Plan 36-04 (Paged.js `<thead>`-repeat Handler, D-15a) is unblocked: the appendix table's `tbody.device-group` structure it needs to target is stable and now fully covered by tests on both the markup side (`html_act_render.rs`) and the print-CSS side (`html_field_row_underline_gate.rs`'s widened gate, `html_page_parity.rs` untouched).
- Plan 36-05 (final live-PDF/live-preview verification across desktop + LAN transports) depends on both 36-03 (this plan) and 36-04 completing first, per its own `depends_on` — 36-03's contribution (green automated test suite) is now satisfied.

---
*Phase: 36-act-pagination*
*Completed: 2026-08-12*

## Self-Check: PASSED

- FOUND: `crates/trackly-app/tests/html_act_render.rs`
- FOUND: `crates/trackly-app/tests/pdf_render_act.rs`
- FOUND: `crates/trackly-app/tests/acts_e2e_smoke.rs`
- FOUND: `crates/trackly-app/tests/html_field_row_underline_gate.rs`
- FOUND: commit `fd1f01c` (Task 1)
- FOUND: commit `66ef269` (Task 2)
- FOUND: commit `2b8662a` (Task 3)
- FOUND: commit `9b3a215` (SUMMARY.md)
