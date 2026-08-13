---
phase: 36-act-pagination
plan: 06
subsystem: pdf
tags: [minijinja, act-handover, act-items, quantity-aggregation, gap-closure]

# Dependency graph
requires:
  - phase: 36-act-pagination (plan 04)
    provides: v24 legacy-defaults snapshot for act_handover.html + Paged.js thead-repeat handler this plan's v25 snapshot builds on
provides:
  - "group_items_for_print(): Rust-side aggregation of print-identical ActItemDto rows, keyed on (name, model, inventory_no, serial_no, condition, kit, specs) — mirrors devices_sqlite::list_grouped's GROUP BY concept, extended per D-17"
  - "act.items_grouped[] render-context key (act_service.rs render_pdf), consumed by both the first-sheet <ol> and the appendix table in act_handover.html — act.items (raw, one row per act_items DB row) is left untouched for the D-13 N=1/N>1 threshold and the N=1 .device-block loop"
  - "_legacy_defaults/v25/act_handover.html — pre-Plan-36-06 snapshot, delivers the fix to already-installed copies via upgrade_untouched_defaults_on_startup"
  - "html_act_render.rs regression coverage: merge via real clone-on-handover path, « × N» suffix on the first-sheet <ol>, and the quantity=5-single-position-still-uses-appendix-branch case (D-13 not reopened)"
affects: []

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Render-only aggregation in Rust, not Jinja: group_items_for_print builds a HashMap<key, index> plus a Vec preserving first-occurrence insertion order — no indexmap dependency, no iteration-order surprises in the printed output"
    - "Two parallel context lists for one entity: act.items (raw DB rows, drives thresholds/N=1 branch) vs act.items_grouped (render-ready, drives N>1 <ol>/appendix) — the raw list is never deleted or replaced, only supplemented, so no existing threshold logic needed to change"

key-files:
  created:
    - crates/trackly-app/templates/_legacy_defaults/v25/act_handover.html
  modified:
    - crates/trackly-app/src/services/act_service.rs
    - crates/trackly-app/src/pdf/html_templates.rs
    - crates/trackly-app/templates/act_handover.html
    - crates/trackly-app/tests/html_act_render.rs

key-decisions:
  - "D-17 supersedes D-03 (2026-08-13, live UAT): D-03's 'quantity column present, prints when >1' design assumed act_items.quantity carried a real per-row count. It does not — it is hardcoded to 1 at INSERT time (act_service.rs, clone-on-handover path), and multiplicity is expressed as N separate act_items rows (source + anonymous clones with NULL inventory/serial) instead. D-03 could therefore never show anything but a dash, and the raw per-row iteration duplicated the position N times in both the first-sheet list and the appendix table. D-17 keeps the user's original intent (reflect the printed count) but fixes the data-model mismatch: aggregation now happens in Rust at render time, mirroring devices_sqlite::list_grouped's existing GROUP BY (type_id, name, model) concept — extended, at Claude's discretion (user may remove), with the remaining printed fields (inventory_no, serial_no, condition, kit, specs) so two units differing on paper (e.g. distinct inventory numbers) can never collapse into one '×2' entry and lose a number from the legal document."
  - "act.items (raw) was deliberately left in the render context unchanged, alongside the new act.items_grouped — the D-13 N>1/N=1 threshold and the N=1 .device-block loop both keep reading the raw list, so a single grouped position with a raw count of 5 (all 5 rows print-identical) still routes through the N>1/appendix branch rather than falling back to the one-sheet N=1 flow. This was the plan's explicit open design point and it settles in favor of NOT reopening D-13."
  - "type_id was deliberately excluded from the merge key even though list_grouped's SQL GROUP BY includes it — ActItemDto's joined SELECT (load_items_for_act) does not carry d.type_id, and widening ActItemDto (shared with the return/update flows) purely for this render-only concern was judged out of proportion; device_name already disambiguates device kinds in this catalog in practice."

requirements-completed: [DOC-11]

# Metrics
duration: ~3h (mostly automated: ~1h40min full-suite test run + human-verification wait; hands-on implementation ~40min across 3 tasks)
completed: 2026-08-13
---

# Phase 36 Plan 06: Merge print-identical act positions (D-17 gap closure) Summary

**Rust-side `group_items_for_print()` aggregation fixes the act-print duplication bug live UAT found — positions with quantity>1 now render once with the real merged count instead of N duplicate rows with a dash in «Кол-во», without ever losing an inventory number when units genuinely differ.**

## Root Cause (record this plainly — it is the reason the plan exists)

`act_items.quantity` is hardcoded to `1` on every INSERT (`ActService::create`'s clone-on-handover path, `act_service.rs` ~line 409). When a user picks quantity N > 1 for one catalog position, the service does NOT store N in one row — it creates N separate `act_items` rows instead: the original device plus N-1 anonymous clones (`devices_sqlite::clone_device_in_tx`), each clone getting `inventory_number = NULL` and `serial_number = NULL` but otherwise identical printed fields (name, model, condition, kit, specs) to the source.

The act-read path (`load_items_for_act`, `act_service.rs` ~line 3026) never aggregated those N rows back together — it returned them as N separate `ActItemDto` entries, one per DB row. So:
- The «Кол-во» column (introduced by Phase 36's original D-03) read `item.quantity` off each individual `ActItemDto`, which was always `1` — the column could **never** show a real count, no matter what quantity the user selected at creation time. It always showed a dash.
- Both the first-sheet device list and the appendix table iterated the same raw N-row list, so a quantity-3 position appeared as **three separate, indistinguishable entries** (or three separate table rows) instead of one entry with a "×3" indication.

D-17 (2026-08-13, replacing D-03) fixes this at the correct layer: aggregation happens in Rust, at render time, over the already-loaded `ActItemDto` rows — mirroring the grouping key the app already uses elsewhere for the exact same "same catalog position, multiple physical units" concept (`devices_sqlite::list_grouped`'s `GROUP BY d.type_id, d.name, d.model`), extended with every other printed field (`inventory_no`, `serial_no`, `condition_at_time`, `complectation_at_time`, `specs`) as a deliberate anti-data-loss measure: two units that print differently on paper (most commonly, two units with different inventory numbers) must never collapse into one "×2" line and silently drop an inventory number from a legal document.

## Why no automated test caught this before live UAT

**This is the most important lesson from this plan.** Every test in `html_act_render.rs`, `pdf_render_act.rs`, and `acts_e2e_smoke.rs` written across Phases 16 through 36-03/36-04/36-05 seeded N *distinct* devices (`seed_devices(&writer, 3)` → `HTML-Ноутбук-0`, `-1`, `-2`, each its own row, its own device_id, its own name) and asserted the template renders N entries. Those tests were internally consistent and green throughout — they correctly proved "the template renders one entry per `act.items` row, in order, without truncation." What none of them ever constructed was the shape that a real quantity>1 selection actually produces in the database: N rows that are all *print-identical* clones of ONE catalog position. The test suite exercised the template's contract with its input list faithfully; it never exercised whether that input list's *construction* (from `ActService::create`'s clone-on-handover path) matched what a human selecting "quantity: 3" in the UI would expect to see printed. The gap was entirely in the untested seam between the create-side data model (N distinct rows, deliberately anonymized clones) and the render-side assumption (each row is a distinct real device) — a seam that only became visible once a real user created a real act with a real quantity selection and looked at the printed output. The fix for this class of gap is not "write more assertions of the same shape" — it is deliberately constructing the *actual* data-model shape (`ActItemNewDto { device_ids: Vec::new(), quantity: N }`, the real clone-on-handover path) in the regression tests added by this plan, which is exactly what Task 3 did (mirroring the existing `acts_clone_handover.rs` fixture pattern instead of inventing a new one).

## Performance

- **Duration:** ~3h wall clock. Hands-on implementation (Tasks 1-3): ~40 min. The remainder was automated: a full `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app -- --test-threads=1 --skip login_remember_persistent_cookie` run (90 test binaries, ~1h40min wall clock, single-threaded per project's no-concurrent-cargo-test rule) plus the wait for the user's live desktop verification at the Task 4 checkpoint.
- **Tasks:** 4 completed (3 automated + 1 human-verify checkpoint, approved)
- **Files modified:** 4 modified + 1 created

## Accomplishments
- `group_items_for_print()` (new, `act_service.rs`): pure-Rust aggregation of `ActItemDto` rows by a 7-field print-identity key, preserving first-occurrence order via a `HashMap<key, index>` + `Vec` (no `indexmap` dependency added). Covered by 4 unit tests: identical-fields merge with summed quantity, differing-inventory-no prevents merge (T-36-06-01 mitigation), single-item passthrough, first-occurrence (not alphabetical) output order.
- `render_pdf` now populates `ctx.act.items_grouped` alongside the existing, unchanged `ctx.act.items` — the raw list keeps driving the D-13 `act.items | length > 1` threshold (exactly 2 occurrences in the template, both untouched) and the N=1 `.device-block` loop; only the N>1 `<ol>` and appendix table switched to the grouped list.
- `act_handover.html`: `<ol class="device-summary">` now shows one `<li>` per merged position, with a « × N» suffix when N > 1; `table.appendix-table`'s tbody-per-position loop now reflects real merged counts in the «Кол-во» column instead of always printing a dash. Doc-comment rewritten to document both `act.items[]` and `act.items_grouped[]` and replace the stale D-03 NOTE with the D-17 explanation.
- `_legacy_defaults/v25/act_handover.html`: byte-identical pre-Task-2 snapshot, registered in `KNOWN_LEGACY_DEFAULTS` (6th element for `act_handover.html`), with a mirrored `upgrade_replaces_v25_legacy_default_with_current_bundled_body` test — confirmed RED immediately after the snapshot (Task 1) and GREEN after the template edit (Task 2), proving a real structural upgrade path for already-installed copies.
- `html_act_render.rs`: the `>1`-quantity test now exercises the real `ActItemNewDto { device_ids: Vec::new(), quantity: 3 }` clone-on-handover path instead of a direct `UPDATE act_items SET quantity` (which no longer proves anything, since that DB column doesn't drive the printed value). Two new tests lock in the `<ol>` suffix behavior (2 `<li>` instead of 4 raw rows) and confirm the D-13 threshold is not reopened by a single grouped position with a raw count of 5.
- Live desktop UAT (Task 4, user-approved "ок"): user confirmed in the running `cargo tauri dev` app that a quantity-3 position renders once with « × 3» on the first sheet and once with «Кол-во» = 3 in the appendix, while a quantity-1 position is unchanged (no suffix, dash).

## Task Commits

Each task was committed atomically:

1. **Task 1: v25 legacy snapshot + registration + group_items_for_print + Rust unit tests** - `b865736` (feat)
2. **Task 2: Template switched to act.items_grouped + doc-comment rewrite** - `757362e` (feat)
3. **Task 3: html_act_render.rs — quantity test rewritten + 2 new regression tests** - `d80faa2` (test)
4. **Task 4: Live desktop UAT (checkpoint)** - no code commit; user-verified, approved "ок"

**Plan metadata:** (this commit, docs: complete plan)

## Files Created/Modified
- `crates/trackly-app/templates/_legacy_defaults/v25/act_handover.html` (created) — pre-fix snapshot for the D-16 upgrade-detection mechanism.
- `crates/trackly-app/src/pdf/html_templates.rs` — registered v25 in `KNOWN_LEGACY_DEFAULTS`, added the mirrored v25 upgrade-regression test.
- `crates/trackly-app/src/services/act_service.rs` — `group_items_for_print()` + wiring into `render_pdf`'s context + 4 unit tests.
- `crates/trackly-app/templates/act_handover.html` — `<ol>` and appendix table switched to `act.items_grouped`; doc-comment rewritten (D-17 supersedes D-03 note).
- `crates/trackly-app/tests/html_act_render.rs` — quantity test rewritten off DB-column manipulation onto the real clone-on-handover path; 2 new regression tests.

## Decisions Made
See `key-decisions` in frontmatter — D-17 supersedes D-03; raw `act.items` intentionally kept alongside `act.items_grouped` so D-13's threshold is never reopened; `type_id` intentionally excluded from the merge key.

## Deviations from Plan

None — plan executed exactly as written. All four tasks matched their `<action>`/`<verify>`/`<acceptance_criteria>` blocks; no Rule 1-4 auto-fixes were needed.

## Issues Encountered

None beyond an expected long wall-clock wait for the full single-threaded `cargo test -p trackly-app` run (~1h40min for 90 test binaries) — not a regression, just the cost of the project's mandated one-test-at-a-time / no-concurrent-cargo-test rule applied to the full workspace-subset suite. Verified the run's exit status via `grep -c "test result: FAILED"` on the full untruncated log (not a `tail`-piped log, which would have hidden earlier failures) — 0 failures across 90/90 binaries.

## User Setup Required
None — no external service configuration required.

## Next Phase Readiness
- Phase 36 (Пагинация акта по количеству устройств) is now fully closed: all five plans (36-01 through 36-05) plus this gap-closure plan (36-06) are complete, and DOC-11 is fully satisfied — the «Кол-во» column and device listing now reflect the real quantity, not the physical `act_items` row structure.
- The pre-existing, out-of-scope duplication in `ui/src/features/acts/ActItemsTable.svelte` (the act-view screen) remains deferred per `36-CONTEXT.md`'s explicit note — not touched by this plan, tracked separately.
- No further follow-up items were surfaced by this plan.

---
*Phase: 36-act-pagination*
*Completed: 2026-08-13*

## Self-Check: PASSED

- FOUND: `crates/trackly-app/templates/_legacy_defaults/v25/act_handover.html`
- FOUND: `crates/trackly-app/src/pdf/html_templates.rs`
- FOUND: `crates/trackly-app/src/services/act_service.rs`
- FOUND: `crates/trackly-app/templates/act_handover.html`
- FOUND: `crates/trackly-app/tests/html_act_render.rs`
- FOUND: commit `b865736` (Task 1)
- FOUND: commit `757362e` (Task 2)
- FOUND: commit `d80faa2` (Task 3)
