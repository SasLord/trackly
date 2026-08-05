---
phase: 260805-har
plan: 01
subsystem: ui
tags: [svelte, print-css, pagedjs, acts]

# Dependency graph
requires:
  - phase: 260805-gdz
    provides: printViaTopLevel's @media print block (display:none app-chrome rule, position:static/left:auto reset for #act-print-root)
provides:
  - LAN-browser print path (printViaTopLevel) forces a literal white sheet background, matching desktop print (printViaSystemBrowser)
affects: [print-preview-polish, acts]

# Tech tracking
tech-stack:
  added: []
  patterns: [literal #fff (not --tr-* token) inside @media print for paper-is-always-white per D-08]

key-files:
  created: []
  modified: [ui/src/features/acts/PdfPreviewModal.svelte]

key-decisions:
  - "Used literal #fff !important, not a --tr-* custom property, because --tr-bg resolves to #0e1218 in dark theme — a token reference would reproduce the exact defect being fixed (D-08: paper is theme-independent)"

patterns-established: []

requirements-completed: [HAR-01]

# Metrics
duration: 8min
completed: 2026-08-05
---

# Quick Task 260805-har: LAN Print Neutralize App Body Background Summary

**Two literal `background: #fff !important` rules added to printViaTopLevel's `@media print` block, neutralizing the app's `--tr-bg` body background so LAN-browser print matches the already-correct desktop print path**

## Performance

- **Duration:** 8 min
- **Started:** 2026-08-05T05:23:00Z (approx)
- **Completed:** 2026-08-05T05:32:00Z
- **Tasks:** 1
- **Files modified:** 1

## Accomplishments
- `printViaTopLevel`'s injected `printStyle` now sets `html, body { background: #fff !important; }` as the first rule inside `@media print`, neutralizing `global.scss`'s `body { background: var(--tr-bg) }` (which resolves to `#eef1f6` light / `#0e1218` dark) so it cannot bleed into the printed sheet
- Added `.pagedjs_page { background: #fff !important; }` mirroring what `pagedPreviewBootstrap.ts`'s `buildSrcdoc` already does for the on-screen preview iframe, per locked decision D-08 ("the sheet is ALWAYS white — it is paper")
- Pre-existing load-bearing rules preserved unchanged: `body > :not(#act-print-root) { display: none !important; }` and `#act-print-root { display: block !important; position: static; left: auto; }`

## Task Commits

Each task was committed atomically:

1. **Task 1: Neutralize app body background and force white sheet in LAN print path** - `2f296b2` (fix)

**Plan metadata:** (pending — orchestrator handles docs commit)

## Files Created/Modified
- `ui/src/features/acts/PdfPreviewModal.svelte` - Added two `background: #fff !important` declarations inside `printViaTopLevel`'s existing `@media print` block (`html, body` selector and `.pagedjs_page` selector)

## Decisions Made
- Literal `#fff` (not a `--tr-*` token) is required inside `@media print` — the dark-theme `--tr-bg` token resolves to near-black (`#0e1218`), which would reproduce the exact grey/dark bleed-through defect this task fixes. This matches the plan's explicit instruction and the precedent set by `pagedPreviewBootstrap.ts`'s `buildSrcdoc`.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Verification Performed

Static checks completed and passing:
1. `pnpm --dir ui svelte-check` — 0 errors (48 pre-existing warnings in unrelated files, no new warnings introduced)
2. `pnpm --dir ui lint` — clean, including the CSP-hash gate (`check-pagedjs-csp-hash.mjs` — confirms `bootstrapScript.js` was not touched, so its CSP hash could not drift)
3. `pnpm --dir ui build` — succeeds (642 modules transformed; pre-existing unused-CSS-selector warning in an unrelated file, pre-existing large-chunk warning, neither related to this change)
4. `grep` assertions confirmed: exactly two `background: #fff !important` rules present, the `position: static;` reset appears exactly once (unchanged), and `printViaSystemBrowser` is present and untouched

**NOT verified — requires manual LAN check (per plan's verification_reality and precedent from 260805-edd/260805-gdz):** No frontend test framework and no automated way to assert printed-output colour exists in this codebase. The actual white-sheet-vs-grey-sheet outcome in a real browser print dialog has NOT been confirmed by this execution. This needs a human on a Windows LAN client to open an act preview via `web.cmy.local:8443` (or equivalent LAN URL), click "Печать", and confirm in the print preview/dialog that the sheet background is white in both light and dark app theme. This step is flagged as a **pending follow-up UAT** — not claimed as done.

## Next Phase Readiness

- Change is isolated to `printViaTopLevel`'s injected print stylesheet; no other files touched.
- `printViaSystemBrowser` (desktop print, reference behaviour) and `ui/src/lib/pdfPreview/bootstrapScript.js` (CSP-hash-locked) remain byte-for-byte unchanged.
- Blocker/follow-up: LAN/Windows manual print-dialog verification (see above) should be performed before this defect chain (260805-edd → 260805-gdz → 260805-har) is considered fully closed.

---
*Phase: 260805-har*
*Completed: 2026-08-05*

## Self-Check: PASSED

- FOUND: ui/src/features/acts/PdfPreviewModal.svelte
- FOUND: .planning/quick/260805-har-lan-print-neutralize-app-body-background/260805-har-SUMMARY.md
- FOUND: commit 2f296b2
