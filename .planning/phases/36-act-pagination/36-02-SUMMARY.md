---
phase: 36-act-pagination
plan: 02
subsystem: templates
tags: [minijinja, act-handover, pagination, print-css, pagedjs]

# Dependency graph
requires:
  - phase: 36-act-pagination (plan 01)
    provides: v24 legacy-defaults snapshot + KNOWN_LEGACY_DEFAULTS registry entry + upgrade_replaces_v24_... regression test (was RED by design, now GREEN)
provides:
  - "act_handover.html N=1/N>1 branching: N=1 keeps the exact Phase-35 .device-block flow; N>1 replaces it with ol.device-summary + appendix referral on the first sheet"
  - "table.appendix-table: 7-column thead, one tbody.device-group per device (main row + optional kit/specs sub-row), zebra via loop.cycle keyed per-tbody, print-color-adjust: exact (+ webkit), dash-for-empty via | default(\"—\", true), quantity column (D-03), break-inside: avoid on tbody, .appendix { break-before: page }"
  - "Rewritten doc-comment describing both branches and act.items[].quantity consumption (C-02)"
affects: [36-03-act-pagination, 36-04-act-pagination, 36-05-act-pagination]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "break-inside: avoid on <tbody> (not bare <tr>) for Paged.js keep-together — TBODY/THEAD is the only tag Paged.js's overflow-finder honors for this property (RESEARCH.md Pattern 4)"
    - "Zebra striping keyed per logical row-group (tbody), not per physical <tr> — both the main row and its optional sub-row share one background so a device's two rows never split color"

key-files:
  modified:
    - crates/trackly-app/templates/act_handover.html

key-decisions:
  - "N=1 branch code moved verbatim into an else-clause of the existing act.items | length > 1 if — no text/attribute inside .device-block changed, only its conditional wrapping (D-08)"
  - "Appendix markup and CSS placed after .signatures and before </body>, gated by the same act.items | length > 1 condition already used for the first-sheet summary (D-13 — identical threshold reused for both branch points)"
  - "@page block left untouched; html_page_parity.rs verified green after both tasks (C-06)"
  - "Did NOT call requirements.mark-complete for DOC-10/DOC-11 in this plan — following 36-01's established pattern (SUMMARY: \"requirements-completed: [] — delivered by the pagination rewrite in 36-02\"), but 36-04 also lists DOC-11 (Paged.js thead-repeat handler, D-15a) and 36-05 lists both for final live-PDF verification. Marking these fully complete now would misrepresent phase state since the thead-repeat behavior and updated test suite are still pending."

requirements-completed: []  # Intentionally not marked here — see key-decisions; full DOC-10/DOC-11 closure spans 36-02..36-05 per 36-CONTEXT.md and 36-01-SUMMARY.md precedent.

# Metrics
duration: 22min
completed: 2026-08-12
---

# Phase 36 Plan 02: act_handover.html pagination rewrite (N=1/N>1 branch + appendix table) Summary

**Rewrote `act_handover.html` so a single-device act renders byte-identically to the Phase-35 flow, while a multi-device act now shows only a numbered device list on the first sheet and moves the full per-device description to a forced-break appendix table with tbody-per-device grouping, zebra striping, dash-for-empty cells, and a quantity column — closing the template half of DOC-10/DOC-11 per the locked D-01..D-16 decisions.**

## Performance

- **Duration:** 22 min
- **Started:** 2026-08-12T15:58:00Z
- **Completed:** 2026-08-12T16:20:00Z
- **Tasks:** 2 completed
- **Files modified:** 1

## Accomplishments
- Doc-comment header rewritten to describe both branches and note `act.items[].quantity` is now consumed by the template (C-02), not merely present in context.
- N=1 flow (the entire `.device-block` loop, including every optional field-row) moved unchanged into an `else` branch — verified via passing `html_handover_single_device_renders_singular_intro_not_plural_summary` and the byte-identical block content (no wording, no attribute changed).
- N>1 first sheet now renders `<ol class="device-summary">` (numbers = `loop.index`, matching the appendix table's № column, D-07) followed by a referral line to «Приложение №1» — `.device-block` no longer renders at all when `act.items | length > 1` (D-08).
- New `.appendix` section added after `.signatures`, gated by the same `act.items | length > 1` condition (D-13): right-aligned two-line appendix mark (D-10), centered title, and `table.appendix-table` with a 7-column thead and one `tbody.device-group` per device.
- Appendix table implements: zebra striping via `loop.cycle('row-even', 'row-odd')` on the `<tbody>` element with `print-color-adjust: exact` + `-webkit-print-color-adjust: exact` (D-04); a hairline `border-top` on each device group and `border-bottom` under the thead as a print-color fallback (D-05); `break-inside: avoid` on `tbody.device-group` (D-15, tbody-scoped per RESEARCH.md Pattern 4, not on a bare `<tr>`); `break-before: page` on `.appendix` (D-16); dash-for-empty cells via the exact `| default("—", true)` idiom copied from `act_acceptance.html` (D-02); a Кол-во column that always renders but only prints a value when `item.quantity > 1` (D-03).
- `@page` block left completely untouched — `html_page_parity::all_three_templates_share_identical_page_block` verified green after both tasks (C-06/D-12).
- The v24 legacy-defaults regression test added in 36-01 (`upgrade_replaces_v24_legacy_default_with_current_bundled_body`), which was RED by design pending this plan's template edit, is now GREEN — confirmed via `cargo test -p trackly-app --lib pdf::html_templates` (15/15 passing).

## Task Commits

Each task was committed atomically:

1. **Task 1: Doc-комментарий + ветвление первого листа** - `cb7c53f` (feat)
2. **Task 2: Appendix-таблица (thead/tbody-группировка/зебра/прочерки/quantity) + CSS** - `fcb6297` (feat)

**Plan metadata:** (this commit, docs: complete plan)

## Files Created/Modified
- `crates/trackly-app/templates/act_handover.html` - N=1/N>1 branching on the first sheet (ol.device-summary vs. .device-block), new appendix section (table.appendix-table + supporting CSS), rewritten doc-comment. `@page` block unchanged.

## Decisions Made
- Kept `act.items | length > 1` as the single reused threshold for both the first-sheet branch and the appendix-section gate, per D-13's explicit requirement that both branch points use the identical condition.
- Placed the appendix markup as the very last block before `</body>` (after `.signatures`), matching the plan's `<interfaces>` spec and D-06 (signatures never duplicate into the appendix).
- Did not add a `requirements.mark-complete` call for DOC-10/DOC-11 in this plan — see `key-decisions` in frontmatter; state update below stops short of the requirements-traceability step deliberately.

## Deviations from Plan

None — plan executed exactly as written. No Rule 1-4 auto-fixes were needed; the template edit matched the `<interfaces>` spec and `36-PATTERNS.md` code excerpts closely enough that no blocking issues, bugs, or missing-functionality gaps surfaced during implementation.

## Known Test Drift (expected, NOT a regression — input for Plan 36-03)

Per `36-CONTEXT.md` C-05/Pitfall 6 and `36-PATTERNS.md`, the following pre-existing tests are now RED because they assert the old single-level N>1 flow (`.device-block` rendered unconditionally, `<ul>` instead of `<ol>`). This is the explicitly planned drift that Plan 36-03 is scoped to fix — recorded here as the accurate starting list:

**`crates/trackly-app/tests/html_act_render.rs`** (1 failing / 15 total):
- `html_handover_multi_device_renders_plural_summary_listing_every_name` — asserts a bare `<ul>`; the template now emits `<ol class="device-summary">` (D-07). Failure: `rendered HTML must contain a <ul>`.

**`crates/trackly-app/tests/pdf_render_act.rs`** (2 failing / 15 total):
- `render_handover_default_template_uses_field_rows_not_device_card` — seeds 2 devices and asserts full-length field-row labels (`Инвентарный номер:`, etc.) appear on the first sheet, and that abbreviated labels (`Инв.№`, `Серийный №`) do NOT appear anywhere. Both assertions now fail: N>1 no longer renders `.device-block` labels on the first sheet at all (D-08), and the new appendix table's `<th>` headers legitimately contain the abbreviated forms (`Инв.№`, `Серийный №`) that this test forbids globally.
- `render_handover_multi_device_fields_attributable_to_own_device` — splits the rendered HTML on `"<div class=\"device-block\">"` and expects 4 parts (1 preamble + 3 device-blocks) for a 3-item act. Now finds only 1 part (no `.device-block` renders at all for N>1) — `left: 1, right: 4`. The underlying intent (fields must be attributable to their own device, not bleed into a neighbor) is still valid but must be re-proven against the new `tbody.device-group` structure, per `36-PATTERNS.md`'s explicit guidance for this exact test.

**`crates/trackly-app/tests/acts_e2e_smoke.rs`** (0 failing / 4 total): No drift. `handover_pdf_render_within_e2e` (2 seeded devices, exercises the new N>1 branch) still passes — its assertions (Cyrillic content present, `html.len() > 1000`) are satisfied by the new appendix-table shape without modification.

**Verified stable, not flaky:** each suite run once; failure messages are deterministic assertion mismatches (missing/extra markup), not timing- or ordering-dependent.

**Total: 3 pre-existing tests red across the whole template-facing test surface** (1 in `html_act_render.rs`, 2 in `pdf_render_act.rs`), all structurally expected per `36-CONTEXT.md` C-05 and explicitly scoped to `36-03-PLAN.md`'s `files_modified` list.

## Self-Check Evidence (see below)
- `cargo check -p trackly-app` — clean, both after Task 1 and Task 2.
- `html_page_parity::all_three_templates_share_identical_page_block` — green after Task 2 (verifies `@page` untouched).
- `pdf::html_templates` unit-test module — 15/15 green, including `upgrade_replaces_v24_legacy_default_with_current_bundled_body` flipping from RED (36-01) to GREEN (this plan).
- Plan-mandated post-Task-2 verification (`html_page_parity`, `pdf_render_act::render_handover_act_produces_cyrillic_pdf`, `pdf_render_act::render_handover_act_contains_d09_intro_phrase`) — all green.

## Issues Encountered
None beyond the documented, plan-anticipated test drift above.

## User Setup Required
None — no external service configuration required.

## Next Phase Readiness
- Plan 36-03 (test suite rewrite for `html_act_render.rs`, `pdf_render_act.rs`, `acts_e2e_smoke.rs`) can proceed immediately with the exact 3-test red list documented above as its starting point.
- Plan 36-04 (Paged.js thead-repeat Handler in `bootstrapScript.js` + `PdfPreviewModal.svelte`, plus CSP hash regeneration) is unblocked — the appendix table's `tbody.device-group` structure it needs to target already exists in the live template.
- Plan 36-05 (final live-PDF/live-preview verification across desktop + LAN transports) depends on 36-03 and 36-04 completing first, per its own `depends_on`.

---
*Phase: 36-act-pagination*
*Completed: 2026-08-12*

## Self-Check: PASSED

- FOUND: `crates/trackly-app/templates/act_handover.html`
- FOUND: `.planning/phases/36-act-pagination/36-02-SUMMARY.md`
- FOUND: commit `cb7c53f` (Task 1)
- FOUND: commit `fcb6297` (Task 2)
- FOUND: commit `d60813d` (SUMMARY.md)
