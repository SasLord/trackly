---
phase: 260805-gdz
plan: 01
subsystem: ui
tags: [svelte, print, pagedjs, pdf-preview, diagnostics]

# Dependency graph
requires:
  - phase: 260805-edd
    provides: earlier Paged.js-stylesheets-as-object fix for printViaTopLevel (already live in build 1.3.2)
provides:
  - handlePrint's catch now binds and logs the real caught error via console.error with print-path context (printViaSystemBrowser vs printViaTopLevel), in addition to the existing toast
  - printViaTopLevel no longer hides #act-print-root via display:none during Paged.js's render/measure pass; hidden off-screen instead with an explicit @media print reset
affects: [print-preview-polish, lan-print-troubleshooting]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Off-screen positioning (position:absolute; left:-100000px) instead of display:none to hide DOM subtrees that a layout-measuring library (Paged.js) needs to read getBoundingClientRect from"

key-files:
  created: []
  modified:
    - ui/src/features/acts/PdfPreviewModal.svelte

key-decisions:
  - "Bound and logged the previously-swallowed print exception via console.error with a printPath label, permanently (not a temporary debug probe) — LAN print failures must be diagnosable from a browser console screenshot"
  - "Fixed printViaTopLevel's display:none-during-pagination defect on first principles even though it was NOT confirmed to be the root cause of the reported LAN print-dialog failure (an earlier isolated-harness experiment was inconclusive)"

patterns-established:
  - "When a container's content is measured by a layout engine (getBoundingClientRect-based), hide it off-screen (position:absolute; left:-100000px) rather than display:none, and always pair with an explicit @media print reset to static/auto positioning"

requirements-completed: [GDZ-01, GDZ-02]

# Metrics
duration: ~15min
completed: 2026-08-05
---

# Quick Task 260805-gdz: LAN print swallowed-error diagnostics + pagination geometry fix Summary

**handlePrint's catch now logs the real exception with print-path context via console.error, and printViaTopLevel hides its pagination container off-screen instead of display:none so Paged.js can measure real layout geometry**

## Performance

- **Duration:** ~15 min
- **Completed:** 2026-08-05
- **Tasks:** 2/2 completed
- **Files modified:** 1

## Accomplishments

- `handlePrint`'s previously bare `catch { ... }` now binds the error (`catch (err)`) and logs it via `console.error('[PdfPreviewModal] handlePrint failed', printPath, err)` before the existing toast fires — on both the desktop (`printViaSystemBrowser`) and LAN-browser (`printViaTopLevel`) paths, distinguished by a `printPath` label.
- `printViaTopLevel`'s injected `printStyle` no longer sets `display: none` on `#act-print-root` under `@media screen` while Paged.js's `previewer.preview()` renders/measures into that container (a first-principles pagination-geometry defect — `display:none` zeroes `getBoundingClientRect` for the whole subtree). It is now hidden off-screen via `position: absolute; left: -100000px; top: 0;` (unconditional base rule), with an explicit `position: static; left: auto;` reset inside the existing `@media print` block so printed/saved-as-PDF output is not pushed off the page.
- `printViaSystemBrowser` is byte-for-byte unchanged (confirmed via `git diff` — no lines touching that function).
- `bootstrapScript.js` (CSP-hash-locked) untouched; the lint gate's `check-pagedjs-csp-hash.mjs` step passed.

## Task Commits

Each task was committed atomically:

1. **Task 1: Bind and log the swallowed print error with path context** - `4b7f96f` (fix)
2. **Task 2: Replace display:none hiding with off-screen positioning in printViaTopLevel's pagination container** - `8a06587` (fix)

**Plan metadata:** (docs commit handled by orchestrator, not this executor)

## Files Created/Modified

- `ui/src/features/acts/PdfPreviewModal.svelte` - `handlePrint`'s catch binds and logs the caught error with print-path context; `printViaTopLevel`'s `printStyle.textContent` swaps `display:none` for off-screen positioning with an explicit print-time reset.

## Decisions Made

- Followed the plan exactly for both changes — no scope deviation. See `key-decisions` in frontmatter for the two substantive judgment calls already made by the plan (permanent logging, fix-regardless-of-confirmation).

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## Verification Performed

**Statically verified (all passed):**
- `pnpm --dir ui svelte-check` — 0 errors (48 pre-existing warnings in unrelated files, none introduced by this change).
- `pnpm --dir ui lint` — passed, including `check-pagedjs-csp-hash.mjs` (bootstrapScript.js untouched, hash unaffected) and prettier/eslint/token/contrast/focus-outline gates.
- `pnpm --dir ui build` — production build succeeded (pre-existing chunk-size warning, unrelated to this file).
- Grep-based plan verification commands for both tasks passed (`catch (err)` + `console.error` near `handlePrint`; `position: absolute` / `left: -100000px` / `position: static` present, `@media screen` block removed).
- `git diff` confirms zero lines touching `printViaSystemBrowser`'s body.

**NOT verified (requires a real LAN browser against the axum server, per `synthetic_harness_not_verification` — no frontend test framework exists and none of the above commands exercise runtime print behaviour):**
- Whether the print dialog now opens on a real LAN-browser client (e.g. `https://web.cmy.local:8443`).
- Whether the new `console.error` actually surfaces a useful exception if the failure persists.
- Whether paginated output renders correctly (no off-page content) when actually printed/saved-as-PDF.

**Per the plan's honesty note: Change 2 (off-screen positioning) is a real defect fix found by reading the code (Paged.js needs real DOM geometry to paginate, which `display:none` denies it), but it is NOT proven to be the root cause of the LAN print-dialog failure reported in the live Windows UAT. An earlier standalone-harness isolation attempt was inconclusive — the control case (a visible container) also hung, so the experiment did not isolate the variable. The next real LAN-browser test (with the new console.error now available) is the only way to confirm whether this fix resolves the reported issue, or whether the console log surfaces a different real cause.**

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Ready for the next live LAN-browser UAT round: open devtools console BEFORE pressing «Печать», then either confirm the print dialog now opens correctly (verify no content is pushed off-page), or capture the new `console.error` output (which will now include a `[PdfPreviewModal] handlePrint failed` label, `printPath`, and the real exception/stack) as the next diagnostic lead.
- No blockers for this quick task; it is diagnostic/defect-fix groundwork, not a confirmed resolution of the reported bug.

---
*Phase: 260805-gdz*
*Completed: 2026-08-05*

## Self-Check: PASSED

- FOUND: commit 4b7f96f (Task 1)
- FOUND: commit 8a06587 (Task 2)
- FOUND: ui/src/features/acts/PdfPreviewModal.svelte
- FOUND: 260805-gdz-SUMMARY.md
