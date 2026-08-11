---
phase: 35-act-handover-body
plan: 03
subsystem: templates
tags: [minijinja, html-templates, act-acceptance, print-css, signature-block]

# Dependency graph
requires:
  - phase: 35-act-handover-body
    plan: 01
    provides: "_legacy_defaults/v22/act_acceptance.html snapshot + KNOWN_LEGACY_DEFAULTS registration, prerequisite for safely changing this template's body"
  - phase: 35-act-handover-body
    plan: 02
    provides: "The exact .signatures/.signature-row/.signature-field/.signature-line/.signature-sublabel/.signature-name CSS+markup pattern this plan replicates byte-for-byte"
provides:
  - "act_acceptance.html brought to signature-block parity with act_handover.html per D-09/D-06/D-07/D-08: duplicate 'Кто передал'/'Кто принял' table rows removed, horizontal one-line-per-signer signature block with printed document.giver_name/document.receiver_name"
affects: [35-04, 35-05]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Signature block class names (.signatures/.signature-row/.signature-label/.signature-field/.signature-line/.signature-sublabel/.signature-name) now shared identically across both act templates — intentional not-DRY duplication (each template is self-contained with inline <style>, no shared stylesheet) but structurally byte-identical"

key-files:
  modified:
    - crates/trackly-app/templates/act_acceptance.html

key-decisions:
  - "Reworded the doc-comment sentence introduced in Task 1 (which literally spelled out 'document.giver_name/document.receiver_name') to avoid inflating Task 2's grep-count-1 acceptance check to 2 — same wording-collision class of fix as Plan 02's 'Сроком до' doc-comment adjustment"
  - "Task 1's literal acceptance criterion ('grep -c \"Кто передал|Кто принял\" outputs 0') could not be true in isolation, since the pre-existing signature block (untouched until Task 2) also contained those exact strings as div text content ('Кто передал: {{ document.giver_name }}'); treated Task 1's actual done-criterion (table rows removed) as authoritative and let the file-wide grep reach 0 only after Task 2 completed the signature-block rework, per the plan's own two-step sequencing"

patterns-established: []

requirements-completed: [DOC-08]

# Metrics
duration: ~10min
completed: 2026-08-11
---

# Phase 35 Plan 03: act_acceptance.html signature-block parity Summary

**Deduplicated giver/receiver ФИО out of act_acceptance.html's key-value table and replaced its two-line `.signature` div with a horizontal `.signature-row`-per-signer block, byte-identical in CSS/markup shape to act_handover.html's Phase 35 signature block (D-09/D-06/D-07/D-08).**

## Performance

- **Duration:** ~10 min
- **Started:** 2026-08-11T12:35:29Z
- **Completed:** 2026-08-11T12:42:05Z
- **Tasks:** 2
- **Files modified:** 1 (`crates/trackly-app/templates/act_acceptance.html`)

## Accomplishments

- Removed the "Кто передал" / "Кто принял" `<tr>` rows from `table.kv`, leaving only the "Дата" row — giver/receiver names now appear exactly once in the file, in the signature block (D-09)
- Updated the file's leading doc-comment to describe the new block order (table has only a date row; signature block carries the printed names) and to list D-06/D-07/D-09 alongside the pre-existing Phase 16 references
- Replaced `.signature`/`.signature .line` CSS (which had no visible underline — text was inlined directly into a margin-only div) with the full `.signatures`/`.signature-row`/`.signature-label`/`.signature-field`/`.signature-line`/`.signature-sublabel`/`.signature-name` rule set, copied verbatim from `act_handover.html`
- Replaced the two-line `div.signature` markup with two horizontal `div.signature-row` blocks ("Выдал:" / "Получил:"), each with an empty underlined `.signature-line`, a "Подпись" sublabel, and the printed `{{ document.giver_name }}` / `{{ document.receiver_name }}` — no "ФИО" sublabel, no signing-date field
- Confirmed via automated tests that giver/receiver names remain present in rendered HTML after the move from table to signature block

## Task Commits

Each task was committed atomically:

1. **Task 1: Remove duplicate ФИО rows from table** - `81b3d39` (feat)
2. **Task 2: Horizontal signature block (D-06/D-07/D-08)** - `bbfed54` (feat)

**Plan metadata:** this SUMMARY + STATE/ROADMAP updates (see final commit below)

## Files Created/Modified

- `crates/trackly-app/templates/act_acceptance.html` - table deduplicated, signature block reworked to horizontal one-line-per-signer layout matching `act_handover.html`; production render path (`act_service::render_acceptance_pdf`) unchanged, no backend code touched

## Decisions Made

- Signature-block CSS/markup class names are intentionally identical (not DRY'd into a shared partial) between `act_acceptance.html` and `act_handover.html` — each print template stays self-contained with inline `<style>` per the established architecture (no external CSS/CDN); this mirrors the plan's `<interfaces>` block, which explicitly says "не переименовывать"
- Reworded a doc-comment sentence that would otherwise have doubled the file-wide occurrence count of the literal string "document.giver_name", to satisfy Task 2's grep-based acceptance check — the same class of fix documented in Plan 02's SUMMARY for "Сроком до"

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Task 1 acceptance criterion "grep -c outputs 0" was unsatisfiable in isolation**
- **Found during:** Task 1 verification
- **Issue:** Task 1's acceptance_criteria stated `grep -c "Кто передал\|Кто принял"` should output 0 after removing the two table rows. But the pre-existing signature block (`<div class="line">Кто передал: {{ document.giver_name }}</div>` etc., not yet touched until Task 2) also contains those exact literal strings as div text content — so the file-wide grep necessarily still matched 2 lines immediately after Task 1 alone.
- **Fix:** Verified Task 1's actual `done` criterion (table contains only "Дата"; table rows removed) directly rather than the broader file-wide grep, since the plan's own Task 2 explicitly owns rewriting the signature block's text. Confirmed the file-wide grep reached 0 only after Task 2 completed, matching the plan's overall `<success_criteria>`.
- **Files modified:** `crates/trackly-app/templates/act_acceptance.html` (no extra fix needed — sequencing resolved it)
- **Commit:** `81b3d39` (Task 1), completed by `bbfed54` (Task 2)

**2. [Rule 1 - Bug] Doc-comment wording collision inflated a grep-count acceptance check**
- **Found during:** Task 2 verification
- **Issue:** The Task 1 doc-comment update I wrote included the literal phrase "document.giver_name/document.receiver_name" to describe the new block order. Once Task 2 added the single intended occurrence of `{{ document.giver_name }}` in the signature block, the file-wide grep count became 2 instead of the required 1.
- **Fix:** Reworded the doc-comment sentence to "giver/receiver names printed once, no longer duplicated in the table" — same meaning, no literal string collision. Re-verified all six of Task 2's grep-based acceptance criteria pass.
- **Files modified:** `crates/trackly-app/templates/act_acceptance.html`
- **Commit:** `bbfed54` (Task 2)

**3. [Rule 1 - Bug] Task 2's "grep -c signature-row outputs 2" criterion also counts CSS selector lines**
- **Found during:** Task 2 verification
- **Issue:** A literal `grep -c "signature-row"` matches 5 lines, not 2 — three CSS selector declarations (`.signature-row {`, `.signature-row .signature-label {`, `.signature-row .signature-name {`) also contain the substring, in addition to the two intended `<div class="signature-row">` markup elements.
- **Fix:** No code change needed — verified the underlying structural intent directly (`grep -c '<div class="signature-row">'` = 2, i.e. exactly one row per signer) rather than the imprecise literal grep. This mirrors the identical class-name reuse in `act_handover.html`'s own CSS, so the discrepancy is inherent to the pattern, not a defect in this file.
- **Files modified:** none (verification-only)
- **Commit:** n/a

No other deviations — both tasks executed as specified in the plan and `<interfaces>` block; production render path was not touched.

## Issues Encountered

None blocking. First `cargo test -p trackly-app --test html_act_render` compile took ~2m20s (cold); the follow-up `acts_e2e_smoke` run was ~15s (warm incremental) — both were backgrounded per project convention (`executors-background-cargo-and-stall` memory) and polled to completion rather than left running unattended.

## User Setup Required

None — no external service configuration required. Production render path (`act_service::render_acceptance_pdf`) was not touched; the change is confined to the template file, consistent with the plan's stated scope. Backend context (`document.giver_name`/`document.receiver_name`) was already present before this plan (Phase 20).

## Verification Results

- `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test html_act_render html_acceptance_contains_required_blocks -- --test-threads=1` — 1 passed
- `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test acts_e2e_smoke acceptance_pdf_render_smoke -- --test-threads=1` — 1 passed
- Grep-based structural checks (final file state): `Кто передал|Кто принял` = 0; `document.giver_name` = 1; `document.receiver_name` = 1; `<div class="signature-row">` = 2; `ФИО` = 0; `border-bottom` = 1

## Privacy Check

`git diff` of this plan's changes contains no real organization data, ФИО, or reused fixture strings from a live database — reviewed against `#000` (CSS color hex, not a real value) as the only numeric-looking match. No new demo/test data was introduced by this plan (backend context and demo fixtures were not touched).

## Next Phase Readiness

Plan 04 (test updates for the C-03 expected drift — `signature_renders_two_line_labels`, `html_handover_contains_required_blocks_and_logo`, and related comment updates) and Plan 05 can proceed. `act_acceptance.html`'s signature block now structurally matches `act_handover.html`'s exactly, closing D-09's cross-document consistency requirement for DOC-08. No blockers. Note for Plan 04/05: `act_acceptance.html` was not in the original C-03 test list (that list covered only `act_handover.html`'s tests) and `html_acceptance_contains_required_blocks`/`acceptance_pdf_render_smoke` already pass with the new markup — no further test changes are needed for this file specifically.

---
*Phase: 35-act-handover-body*
*Completed: 2026-08-11*

## Self-Check: PASSED

All created/modified files and commit hashes verified present.
