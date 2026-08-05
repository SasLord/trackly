---
phase: 260805-edd
plan: 01
subsystem: ui
tags: [svelte, pagedjs, print, typescript, lan-server]

# Dependency graph
requires:
  - phase: 33-print-preview-polish
    provides: printViaTopLevel() top-level-document print path and the ambient pagedjs.d.ts type stub
provides:
  - Fixed printViaTopLevel() stylesheets argument shape for Paged.js's Polisher (object, not string)
  - Corrected pagedjs.d.ts ambient type to accept object-shaped stylesheet entries
affects: [pdf-preview, lan-print, pagedjs-typings]

# Tech tracking
tech-stack:
  added: []
  patterns: ["Paged.js Previewer.preview() stylesheets must be object-keyed CSS text (`[{ filename: cssText }]`), never a bare string — a string is treated as a URL by Polisher.add()"]

key-files:
  created: []
  modified:
    - ui/src/features/acts/PdfPreviewModal.svelte
    - ui/src/pagedjs.d.ts

key-decisions:
  - "Removed the previewer.preview() catch/retry fallback entirely — it retried with the same broken string shape and could never recover; a genuine failure now surfaces via handlePrint()'s existing toast."
  - "Widened pagedjs.d.ts's stylesheets type from string[] to (string | Record<string, string>)[] to match Paged.js's real runtime API, since the narrower type was blocking the object-shaped fix with a TS error (Rule 3 auto-fix — pre-existing incomplete type stub in this repo, not a package install)."

requirements-completed: [EDD-01]

# Metrics
duration: ~20min
completed: 2026-08-05
---

# Quick Task 260805-edd: Fix LAN print — pass stylesheets to Paged.js as object Summary

**`printViaTopLevel` now hands Paged.js's `Previewer.preview()` the preview CSS as `[{ 'act-preview.css': cssText }]` instead of a bare string, fixing the bogus network request that broke LAN-browser printing.**

## Performance

- **Duration:** ~20 min (including root-cause bisection of an unrelated TS type-declaration bug)
- **Completed:** 2026-08-05T03:29:03Z
- **Tasks:** 1 completed (plan had 1 task)
- **Files modified:** 2 (1 planned + 1 Rule-3 auto-fix)

## Accomplishments
- `printViaTopLevel` passes Paged.js's `Polisher.add()` an object-shaped stylesheets entry (`[{ 'act-preview.css': cssText }]`), so the CSS text is consumed directly instead of being fetched as a URL — eliminates the observed `/%3Cstyle%3E...` failed network request in a real LAN browser.
- Removed the redundant `try`/`catch` fallback that retried with a string of the same (broken) shape; a genuine `preview()` failure now surfaces through `handlePrint()`'s existing `catch { pushToast(...) }`.
- Fixed a pre-existing incomplete ambient TypeScript declaration (`ui/src/pagedjs.d.ts`) that only typed `stylesheets` as `string[]`, discovered because it blocked the plan's required object-shaped call with a compile-time type error.

## Task Commits

1. **Task 1: Pass stylesheets to Paged.js Polisher as an object, not a string** - `c77ab6c` (fix)

**Plan metadata:** committed separately by the docs commit step (not part of this task-level summary).

## Files Created/Modified
- `ui/src/features/acts/PdfPreviewModal.svelte` - `printViaTopLevel`: added `cssText` (bare CSS, used by both `printStyle.textContent` and the Paged.js call), replaced the string-shaped `previewer.preview(...)` try/catch with a single object-shaped call, no fallback.
- `ui/src/pagedjs.d.ts` - Widened the `Previewer.preview()` ambient type's `stylesheets` parameter from `string[]` to `(string | Record<string, string>)[]` to match Paged.js's actual runtime API (`Polisher.add()` branches on `typeof arguments[i] === 'object'`).

## Decisions Made
- Kept `styleHtml` (tag-wrapped) as-is for `printStyle.textContent`'s existing usage; introduced `cssText` (bare, tag-stripped) as the one new value, shared between `printStyle.textContent` (now reading from the local instead of recomputing inline) and the Paged.js `preview()` call.
- No fallback retry — see key-decisions above.
- Fixed the ambient type stub in place rather than using an `as` cast or `@ts-expect-error` at the call site, since the type itself was objectively wrong relative to Paged.js's documented/observed runtime behavior (confirmed by reading `pagedjs/dist/paged.js`'s `Polisher.add()`), and future callers of `Previewer.preview()` benefit from the corrected type too.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Widened `ui/src/pagedjs.d.ts`'s `stylesheets` type to accept object-shaped entries**
- **Found during:** Task 1 verification (`pnpm --dir ui svelte-check`)
- **Issue:** The plan's required edit (`previewer.preview(bodyHtml, [{ 'act-preview.css': cssText }], printRoot)`) failed to type-check: `Type '{ 'act-preview.css': string; }' is not assignable to type 'string'.` The repo's own hand-written ambient module declaration for `pagedjs` (added in Phase 33-04, since the `pagedjs` npm package ships no types) typed `stylesheets` as `string[]` only — narrower than the library's actual runtime API, which the plan's fix is exercising for the first time in this codebase.
  - Note: initial `svelte-check` runs surfaced this as a confusing cascade of `"Module '...PdfPreviewModal.svelte' has no default export"` errors in three importing files (`DevicesPage.svelte`, `ActsPage.svelte`, `ReportsPage.svelte`); bisecting the diff hunk-by-hunk (each variant temporarily copied into place, checked in isolation) isolated the real, single root-cause error above. `pnpm --dir ui build` succeeded throughout (Vite's Svelte compiler doesn't run this same strict TS check), confirming the issue was purely a type-checking gap, not a runtime/syntax problem.
- **Fix:** Changed `stylesheets?: string[]` to `stylesheets?: (string | Record<string, string>)[]` in `ui/src/pagedjs.d.ts`, with a comment documenting why (matches `Polisher.add()`'s real `typeof arguments[i] === 'object'` branch).
- **Files modified:** `ui/src/pagedjs.d.ts`
- **Verification:** `pnpm --dir ui svelte-check` → 0 errors (same as pre-change baseline); `pnpm --dir ui lint` and `pnpm --dir ui build` both pass.
- **Committed in:** `c77ab6c` (part of the Task 1 commit)

---

**Total deviations:** 1 auto-fixed (1 blocking)
**Impact on plan:** Necessary to make the plan's specified fix type-check at all. No scope creep — the fix is a minimal, targeted widening of an existing type stub already scoped to Paged.js's `Previewer.preview()` surface.

## Issues Encountered
- `svelte-check`'s cascading "has no default export" errors on unrelated importer files were misleading on first read; resolved by bisecting the diff into isolated variants (each copied into place, checked, reverted) until the single real type error was isolated. See deviation #1 above for the full trail.

## User Setup Required

None - no external service configuration required.

## Verification Status

**Statically verified only.** `pnpm --dir ui svelte-check`, `pnpm --dir ui lint` (including the CSP hash-drift gate — unaffected, `bootstrapScript.js` untouched), and `pnpm --dir ui build` all pass, and `printViaSystemBrowser` (the desktop print path) is byte-for-byte unchanged (confirmed via `git diff`).

**NOT verified: the actual LAN-browser print fix.** These commands cannot prove the fix works at runtime — that requires a real browser on another machine hitting the axum LAN server (`https://web.cmy.local:8443` or equivalent), opening a document preview, pressing «Печать», and confirming in DevTools Network that no request fires whose URL is CSS text, that the native print dialog opens, and that the printed/PDF output has the expected `@page` margins and fonts (proving the CSS actually reached Paged.js's Polisher). This manual check has NOT been performed as part of this quick task and remains the only real proof of the fix.

## Next Phase Readiness
- Code change is complete and statically verified; ready for a manual LAN-browser print check (see Verification Status above) whenever a real LAN-connected browser is available for testing.
- No blockers for other in-flight work — this is an isolated fix to `printViaTopLevel` and its supporting type stub.

---
*Phase: 260805-edd*
*Completed: 2026-08-05*

## Self-Check: PASSED

- FOUND: `ui/src/features/acts/PdfPreviewModal.svelte`
- FOUND: `ui/src/pagedjs.d.ts`
- FOUND: `.planning/quick/260805-edd-fix-lan-print-pass-stylesheets-to-paged-/260805-edd-SUMMARY.md`
- FOUND: commit `c77ab6c`
