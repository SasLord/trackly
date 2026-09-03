---
phase: 40-movement-history
plan: 27
subsystem: ui
tags: [svelte, pagedjs, print, lan-browser, structural-gate]

# Dependency graph
requires:
  - phase: 40-movement-history (plan 25)
    provides: "ui/package.json `lint` script chain ending in check-report-type-parity.mjs"
provides:
  - "Idempotent printViaTopLevel() in PdfPreviewModal.svelte — clears printRoot/destroys leftover Polisher at the START of the call, not only on afterprint"
  - "Re-entrancy guard (printing state) on handlePrint(), with the «Печать» button disabled while a print run is in flight"
  - "ui/scripts/check-print-idempotency.mjs — structural regression gate wired into `pnpm lint`"
affects: [movement-history reports printing, LAN-browser print/export flows]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Component-scope state (not function-local) for anything that must survive across repeated calls of an async function, so the next call can see/clean up what the previous call left behind"
    - "Structural regression gate (fs/path/url, zero deps) verifying source-code invariants by locating a function body and checking substring position relative to a marker call, following the check-place-path-short.mjs pattern"

key-files:
  created:
    - ui/scripts/check-print-idempotency.mjs
  modified:
    - ui/src/features/acts/PdfPreviewModal.svelte
    - ui/package.json

key-decisions:
  - "Hoisted injectedPolisher → activePolisher and added repeatTableHeadHandlerRegistered/printing as component-scope `let`s so state survives across print invocations, per plan's <interfaces> spec"
  - "Cleanup at start of printViaTopLevel is unconditional (activePolisher?.destroy(); printRoot.innerHTML = '') and independent of the existing afterprint listener, which is left in place for the normal-path cleanup"
  - "registerHandlers(RepeatTableHeadHandler) is now gated by a one-shot component-scope flag; the RepeatTableHeadHandler class itself is still redeclared per call (unchanged) because its constructor needs a fresh table.appendix-table snapshot each print"

requirements-completed: [HST-04]

# Metrics
duration: ~10min
completed: 2026-09-03
---

# Phase 40 Plan 27: LAN print idempotency gap closure Summary

**Fixed LAN-browser PDF print/export producing a duplicated first page by making `printViaTopLevel()`'s DOM cleanup unconditional at call-start (not only on `afterprint`) and adding a `printing` re-entrancy guard to `handlePrint()`, backed by a new structural lint gate.**

## Performance

- **Duration:** ~10 min
- **Started:** 2026-09-03T01:28:00Z (approx, per STATE.md session)
- **Completed:** 2026-09-03T01:31:20+07:00 (last commit)
- **Tasks:** 2/2
- **Files modified:** 3 (1 created, 2 modified)

## Accomplishments

- `printViaTopLevel()` now destroys any leftover `activePolisher` and clears `printRoot.innerHTML` at the **start** of every call, before Paged.js re-paginates — closes the gap where a second print click landed before the previous run's `afterprint` cleanup fired, which caused pagedjs's `Chunker.setup()` to `appendChild` a duplicate `.pagedjs_pages` set on top of the prior render.
- `registerHandlers(RepeatTableHeadHandler)` is now called once per component lifetime (guarded by `repeatTableHeadHandlerRegistered`), not once per print click, avoiding an ever-growing pagedjs handler registry.
- `handlePrint()` has a `printing` re-entrancy guard: a second click while a print run is in flight is a no-op, and the «Печать» button is now visually disabled for that duration (closing the "seconds without feedback" UX gap noted in the diagnosis).
- New structural gate `ui/scripts/check-print-idempotency.mjs` enforces all three invariants (cleanup-before-pagination, gated handler registration, `printing` guard present) and is wired into the end of `pnpm lint`'s existing chain, after 40-25's `check-report-type-parity.mjs`.

## Task Commits

Each task was committed atomically:

1. **Task 1: Идемпотентный printViaTopLevel + re-entrancy guard в handlePrint** - `d411a1f` (fix)
2. **Task 2: Структурный регрессионный гейт идемпотентности печати** - `3e4c496` (test)

**Plan metadata:** committed alongside this summary (see final commit hash in orchestrator output)

## Files Created/Modified

- `ui/src/features/acts/PdfPreviewModal.svelte` - Hoisted `activePolisher`/`repeatTableHeadHandlerRegistered`/`printing` to component scope; `printViaTopLevel` clears `printRoot`/destroys leftover Polisher at call-start; `registerHandlers` gated one-shot; `handlePrint` re-entrancy-guarded with `try/finally`; «Печать» button `disabled` now also depends on `printing`.
- `ui/scripts/check-print-idempotency.mjs` - New zero-dependency structural gate checking the three invariants above against `PdfPreviewModal.svelte`; self-tested against a scratch copy (each invariant mutated individually trips the gate with a matching message; unmutated file passes).
- `ui/package.json` - Appended `&& node scripts/check-print-idempotency.mjs` to the end of the existing `lint` script (after 40-25's `check-report-type-parity.mjs`, which was preserved verbatim).

## Decisions Made

- Followed the plan's `<interfaces>` section exactly for variable naming/placement (`activePolisher`, `repeatTableHeadHandlerRegistered`, `printing`) — no naming deviation.
- Kept the existing `afterprint` listener/`cleanup()` function as the normal-path cleanup; the new call-start cleanup is an *additional* safety net for the case where `afterprint` never fired (the actual bug mechanism), not a replacement.

## Deviations from Plan

None - plan executed exactly as written. `pnpm --dir ui svelte-check` produced 0 errors (only pre-existing warnings, including the pre-existing "Avoid declaring classes below top level scope" warning for `RepeatTableHeadHandler`, unchanged by this plan). The new gate script initially failed `prettier --check` on creation (formatting-only, not a logic issue) — reformatted with `prettier --write` and re-verified both the gate's own correctness (mutation self-test) and the full `pnpm lint` chain before committing; not counted as a plan deviation since no auto-fix rule applied to substantive code, only formatting.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- HST-04 requirement closed for this plan; `pnpm lint` chain now carries both 40-25's (`check-report-type-parity.mjs`) and this plan's (`check-print-idempotency.mjs`) gates without collision.
- Manual verification still required per plan's `<verification>` section (post-merge, LAN browser: Отчёты → Перемещения → Печать/Экспорт PDF — page count must match the preview). This was **not** performed in this session — flagging as the remaining open item for UAT-40 gap `lan-print-duplicate-first-page` before considering it fully closed end-to-end. Marked UNVERIFIED (structural/static checks only; no live LAN-browser print run performed in this session — Playwright/Chromium harnesses are not accepted as verification for this app per project convention).

---
*Phase: 40-movement-history*
*Completed: 2026-09-03*

## Self-Check: PASSED
