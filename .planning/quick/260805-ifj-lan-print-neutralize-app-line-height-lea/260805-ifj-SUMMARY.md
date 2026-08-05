---
phase: 260805-ifj
plan: 01
subsystem: ui
tags: [svelte, print, css, pagedjs, act-preview]

# Dependency graph
requires:
  - phase: 260805-gdz
    provides: "Off-screen positioning + display:none pagination fix for #act-print-root, and the @media print reset block this plan adds a second block alongside"
  - phase: 260805-har
    provides: "background: #fff !important neutralization inside the same @media print block, precedent for the pattern of resetting app-inherited styles under @media print"
provides:
  - "printViaTopLevel's injected printStyle now resets body { line-height/letter-spacing/word-spacing: normal } under @media print, ahead of the template's own cssText"
affects: [acts, pdf-preview, lan-print]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Print-only resets for app-inherited body CSS go in their own @media print { body { ... } } block, placed BEFORE ${cssText} in printStyle.textContent's source order, so a user-customized template's own declarations still win the cascade."

key-files:
  created: []
  modified:
    - ui/src/features/acts/PdfPreviewModal.svelte

key-decisions:
  - "Added the reset as a SEPARATE @media print block (not merged into the pre-existing @media print block from 260805-gdz/har) — simpler to reason about, avoids reordering rules with known load-bearing dependencies (e.g. position:static must come after display:none for the print-root)."
  - "Left the separate cssText-unscoped-onto-app-body defect (template body rules leaking onto on-screen app typography after first LAN print) untouched — explicitly out of scope per plan; documented as a known follow-up below."

patterns-established: []

requirements-completed: [IFJ-01]

# Metrics
duration: ~15min
completed: 2026-08-05
---

# Quick Task 260805-ifj: Neutralize app line-height leak in LAN print Summary

**Prepended a print-only `body { line-height: normal }` reset ahead of the act template's own stylesheet inside `printViaTopLevel`, so LAN-browser printing no longer inherits the app's on-screen `line-height: 1.5` and instead falls back to the UA default that both desktop print and all three act templates already rely on.**

## Performance

- **Duration:** ~15 min
- **Completed:** 2026-08-05T06:21:37Z
- **Tasks:** 1/1 completed
- **Files modified:** 1

## Accomplishments

- Root-caused (already confirmed by the plan, verified again while reading the code) that `printViaTopLevel` renders Paged.js output into the app's own `document.body`, so `global.scss`'s `body { line-height: var(--tr-line-height-body) }` (1.5) leaks into printed act content — the one `body` property none of the three act templates redeclare.
- Added a new `@media print { body { line-height: normal; letter-spacing: normal; word-spacing: normal; } }` block to `printStyle.textContent`, placed BEFORE the existing `${cssText}` interpolation and scoped only to `@media print`.
- Verified via source-order gate script that the reset is (a) inside `@media print` and (b) precedes `${cssText}`, satisfying both load-bearing placement constraints from the plan.
- Confirmed all four pre-existing `@media print` rules from 260805-gdz/260805-har (background resets x2, display:none chrome, position:static/left:auto) are byte-for-byte unchanged, in their own untouched block below the new one.

## Task Commits

Each task was committed atomically:

1. **Task 1: Prepend a print-only line-height reset ahead of the template's own cssText** - `3162320` (fix)

**Plan metadata:** (this SUMMARY + STATE.md update handled by orchestrator, not committed by this executor per instructions)

## Files Created/Modified

- `ui/src/features/acts/PdfPreviewModal.svelte` — `printViaTopLevel`'s `printStyle.textContent` template literal now opens with a commented `@media print { body { line-height: normal; letter-spacing: normal; word-spacing: normal; } }` block before `${cssText}`.

## Decisions Made

- Kept the new reset as a distinct `@media print { ... }` block rather than merging it into the existing one lower in the same template literal — avoids any risk of reordering the existing rules (whose relative order is itself load-bearing per the 260805-gdz comment above them), and two `@media print` blocks in one stylesheet is valid CSS with no cascade difference from one merged block.
- Inline code comment placed directly above the new block (not just in this SUMMARY) per plan requirement, explaining both the `@media print` scoping and the before-`cssText` ordering so a future editor sees the rationale at the point of insertion.

## Deviations from Plan

None — plan executed exactly as written. One thing worth noting for future executors: the first draft of the inline comment referenced the literal string `${cssText}` inside JS-string-literal escaping to describe the source-order requirement; this produced a literal `${cssText}` substring appearing in the compiled output BEFORE the real interpolation, which would have failed the plan's own automated ordering gate. Caught and fixed before commit by rewording the comment to avoid embedding that literal token — the committed code has no such issue.

## Verification Performed

Static checks only, per this plan's explicit scope:

1. `node -e ...` source-order gate (from the plan's own `<verify>` block) — **PASSED**: `line-height: normal` exists inside `@media print` and precedes `${cssText}`.
2. `grep -c "background: #fff !important"` — **PASSED**: still exactly 2 occurrences (both pre-existing rules intact).
3. `grep "position: static;"` — **PASSED**: pre-existing reset intact.
4. `grep "async function printViaSystemBrowser"` — **PASSED**: present, and diff confirms it and `ui/src/lib/pdfPreview/bootstrapScript.js` were not touched by this change.
5. `pnpm --dir ui svelte-check` — **PASSED**: 0 errors, 48 pre-existing warnings unrelated to this file/change.
6. `pnpm --dir ui lint` — **PASSED**: eslint, prettier, token/contrast/focus-outline checks, and the CSP-hash gate (`check-pagedjs-csp-hash.mjs`) all clean — confirms `bootstrapScript.js` is byte-for-byte unchanged.
7. `pnpm --dir ui build` — **PASSED**: build succeeds (pre-existing unused-CSS-selector warning in an unrelated file, pre-existing chunk-size warnings).

**NOT verified (explicitly out of reach of automation, per the plan's verification_reality):** actual printed line spacing on paper. No frontend test framework exists to assert visual print output, and none of the commands above can measure it. This is the same class of gap as the prior print-fix quick tasks in this chain (260805-edd/gdz/har).

## Pending Follow-up UAT

From a Windows LAN client: open an act preview via the LAN server URL, click "Печать", and compare the printed sheet's line spacing against a desktop-app printout of the SAME act. They should now match (both falling back to UA-default `line-height: normal`). This plan was executed without LAN/Windows access, so this step is a pending manual verification, not yet performed.

## Known Follow-up (explicitly out of scope for this plan)

A separate, pre-existing defect in the same function: `${cssText}` is injected unscoped (not wrapped in `@media print` or scoped to `#act-print-root`), so a template's `body { font-family; font-size; color }` rules also leak onto the app's own on-screen body after a LAN print. Additionally, `printStyle` itself is never removed after printing (only `printRoot.innerHTML` is cleared on `afterprint`), so the app's on-screen typography can change after the first LAN print of a session. Fixing this requires either scoping `cssText` under `@media print` or dropping the manual style injection in favor of Paged.js's Polisher (which already consumes `cssText` via its `preview()` argument) — both carry pagination risk and deserve their own change with dedicated UAT. Not acted on here per the plan's explicit scope boundary.

## Self-Check: PASSED

- FOUND: ui/src/features/acts/PdfPreviewModal.svelte (modified, verified via Read)
- FOUND: commit 3162320 in `git log --oneline -1` (fix(260805-ifj): neutralize app line-height leak in LAN print output)
