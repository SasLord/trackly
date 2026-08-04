---
phase: 33-print-preview-polish
plan: 01
subsystem: ui
tags: [pagedjs, csp, postmessage, iframe, svelte5, pluralization]

# Dependency graph
requires: []
provides:
  - "pagedjs 0.4.3 exact-pinned dependency in ui/package.json, resolvable by Vite"
  - "PAGED_PREVIEW_INLINE_SCRIPT frozen concatenation formula (pagedjsLibraryText + ';\n' + bootstrapText) for Plan 33-02's CSP hash"
  - "buildSrcdoc(actHtml, theme) srcdoc-construction contract for Plan 33-03's on-screen preview wiring"
  - "attachBridge(iframeEl, onMsg) opaque-origin-safe postMessage bridge for Plan 33-03/33-04"
  - "pluralizeRu(n, forms) RU pluralization helper for Plan 33-03's page counter"
affects: [33-02-csp-hash, 33-03-preview-wiring, 33-04-print-paths]

# Tech tracking
tech-stack:
  added: ["pagedjs 0.4.3 (exact-pinned, MIT)"]
  patterns:
    - "Vite `?raw` raw-text import (first use in this codebase) to inline third-party UMD bundle + first-party bootstrap script text into an iframe srcdoc"
    - "'<' + 'script>' / '<' + '/script>' string-concatenation idiom (reused from PdfPreviewModal.svelte's printViaSystemBrowser) to avoid a literal </script> substring inside .ts source"
    - "postMessage sender validation via event.source identity (never event.origin, which is the literal string \"null\" for an opaque-origin sandboxed iframe)"

key-files:
  created:
    - ui/src/lib/pdfPreview/bootstrapScript.js
    - ui/src/lib/pdfPreview/pagedPreviewBootstrap.ts
    - ui/src/lib/pdfPreview/pagedPreviewBridge.ts
    - ui/src/lib/utils/pluralize.ts
  modified:
    - ui/package.json
    - ui/pnpm-lock.yaml
    - ui/eslint.config.js

key-decisions:
  - "pagedjs pinned to exact 0.4.3 (no caret) so Plan 33-02's CSP sha256 hash-source over paged.min.js's bytes cannot silently drift on a routine pnpm install"
  - "bootstrapScript.js kept as a single static, non-interpolated file (all per-render variance — the backdrop color — lives in buildSrcdoc's separate <style> chrome block, outside the CSP-hashed <script> boundary)"
  - "eslint.config.js extended (out-of-plan, Rule 3 blocking-issue fix): added parent/HTMLIFrameElement/MessageEvent to shared browser globals, plus a sourceType:'script' override scoped to bootstrapScript.js, since the default TS/module lint blocks don't apply to a plain non-module browser script"

patterns-established:
  - "New modules under ui/src/lib/pdfPreview/ for Paged.js print-preview machinery (bootstrap script, srcdoc builder, postMessage bridge) — downstream Plan 33-03/33-04 import from here, not from ui/src/features/acts/"

requirements-completed: [PRV-01, PRV-02]

duration: ~35min
completed: 2026-08-04
---

# Phase 33 Plan 01: Frozen Paged.js srcdoc/bridge contract Summary

**Pinned `pagedjs` 0.4.3 dependency plus four new `ui/src/lib/` modules (bootstrap script, srcdoc builder, opaque-origin postMessage bridge, RU pluralization helper) establishing the frozen interface that Plans 33-02/33-03/33-04 build on — nothing wired into `PdfPreviewModal.svelte` yet.**

## Performance

- **Duration:** ~35 min (dominated by a one-time `cargo test -p trackly-app --test export_bindings` prebuild step contending with a concurrent background `cargo check --workspace` for the target-dir lock)
- **Started:** 2026-08-04T21:10:42+07:00 (approx, session start)
- **Completed:** 2026-08-04T21:34:01+07:00
- **Tasks:** 3/3
- **Files modified:** 7 (4 created, 3 modified)

## Accomplishments
- Added `pagedjs` as an exact-pinned (`"0.4.3"`, no caret) direct dependency; `pnpm --dir ui build` verified unaffected
- Created `bootstrapScript.js` — the static, non-interpolated Paged.js bootstrap protocol script that Plan 33-02's CSP hash and Plan 33-04's print paths both depend on byte-for-byte
- Created `pagedPreviewBootstrap.ts` exporting `PAGED_PREVIEW_INLINE_SCRIPT`, `THEME_CHROME`, and `buildSrcdoc()` per the exact documented contract
- Created `pagedPreviewBridge.ts` exporting `attachBridge()` with opaque-origin-safe `event.source` validation (never `event.origin`)
- Created `pluralize.ts` exporting `pluralizeRu()` implementing the standard RU 1/2-4/5+ (with 11-14 exception) plural-agreement rule

## Task Commits

Each task was committed atomically:

1. **Task 1: Add pinned pagedjs dependency** - `bf2f294` (feat)
2. **Task 2: Create the srcdoc-construction contract** - `dceea5b` (feat)
3. **Task 3: Create the postMessage bridge and RU pluralization helper** - `1a8681c` (feat)

_No TDD tasks in this plan (project has no frontend test framework — see verification_reality)._

## Files Created/Modified
- `ui/package.json` - added `"pagedjs": "0.4.3"` (exact-pinned) to `dependencies`
- `ui/pnpm-lock.yaml` - updated by `pnpm --dir ui install`
- `ui/src/lib/pdfPreview/bootstrapScript.js` - static Paged.js bootstrap protocol script (IIFE, plain browser JS)
- `ui/src/lib/pdfPreview/pagedPreviewBootstrap.ts` - `PAGED_PREVIEW_INLINE_SCRIPT`, `THEME_CHROME`, `buildSrcdoc()`
- `ui/src/lib/pdfPreview/pagedPreviewBridge.ts` - `attachBridge()` postMessage bridge
- `ui/src/lib/utils/pluralize.ts` - `pluralizeRu()` RU pluralization helper
- `ui/eslint.config.js` - added `parent`/`HTMLIFrameElement`/`MessageEvent` browser globals + a `sourceType:'script'` override scoped to `bootstrapScript.js`

## Decisions Made
- Followed the plan's exact interface signatures (`buildSrcdoc(actHtml, theme)`, `PAGED_PREVIEW_INLINE_SCRIPT`, `THEME_CHROME`, `attachBridge(iframeEl, onMsg)`, `pluralizeRu(n, forms)`) verbatim — no deviations from the documented contract.
- Placed the four new files at `ui/src/lib/pdfPreview/**` and `ui/src/lib/utils/pluralize.ts` per the plan's frontmatter `files_modified` list (this supersedes 33-PATTERNS.md's earlier suggestion of `ui/src/features/acts/pagedPreviewBootstrap.ts` — the plan is the more specific, later-authored source of truth for file location).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Extended `ui/eslint.config.js` browser-globals coverage**
- **Found during:** Task 2/3 (running `pnpm --dir ui lint` as the verification substitute for the plan's `pnpm --dir ui check`, which does not exist as an npm script — see next deviation)
- **Issue:** `pnpm --dir ui lint`'s `eslint` step failed with 10 `no-undef` errors: `bootstrapScript.js` (a plain, non-module browser script) doesn't match any existing eslint flat-config `files` block that supplies browser globals, so `window`/`parent`/`document` were flagged as undefined; `pagedPreviewBridge.ts` used `HTMLIFrameElement` and `MessageEvent`, neither of which was in the shared `browserGlobals` object.
- **Fix:** Added `parent`, `HTMLIFrameElement`, `MessageEvent` to the shared `browserGlobals` map, and added a new eslint config block scoped to `src/lib/pdfPreview/bootstrapScript.js` with `sourceType: 'script'` (since it has no import/export) and the browser globals.
- **Files modified:** `ui/eslint.config.js` (split across the Task 2 and Task 3 commits — `parent` + the file-specific block in Task 2's commit since `bootstrapScript.js` needed them; `HTMLIFrameElement`/`MessageEvent` in Task 3's commit since `pagedPreviewBridge.ts` needed them)
- **Verification:** `pnpm --dir ui lint` passes with 0 errors (same 48 pre-existing warnings, unrelated to this plan's files)
- **Committed in:** `dceea5b` (Task 2), `1a8681c` (Task 3)

---

**Total deviations:** 1 auto-fixed (1 blocking-issue fix, split across two task commits)
**Impact on plan:** Necessary for the new files to pass the project's existing lint gate; no scope creep beyond the minimum eslint config needed for these five source files.

## Issues Encountered

**`pnpm --dir ui check` does not exist as an npm script.** The plan's `<verify>` blocks for Tasks 2 and 3 specify `pnpm --dir ui check` ("svelte-check + eslint + prettier + token/contrast scripts"), but `ui/package.json`'s `scripts` block has no `check` entry — only `svelte-check` and `lint` (which itself runs eslint + prettier + `check-tokens.mjs` + `check-contrast.mjs` + `check-focus-outline.mjs`). Per `verification_reality` guidance, ran the two constituent commands separately instead of weakening the check: `pnpm --dir ui svelte-check` (0 errors, 48 pre-existing warnings) and `pnpm --dir ui lint` (passes after the eslint-config fix above). Together these cover exactly what the plan's described `check` command was meant to run. Not treated as a plan deviation requiring a fix — this is a pre-existing gap in `package.json`'s script naming, out of this plan's stated file scope.

**One-time `cargo test -p trackly-app --test export_bindings` prebuild step (`ui/package.json`'s `prebuild` hook, triggered by the first `pnpm --dir ui build`) took ~15 minutes** due to contention with a concurrent, unrelated `cargo check --workspace --all-targets` process (consistent with the project's known "no concurrent cargo" lock-contention behavior). Waited it out; no code changes needed. Subsequent `pnpm build` calls in the same session (which skip the already-satisfied `prebuild` step incrementally) were fast (~2s).

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Plan 33-02 (CSP hash) can now compute its build-time SHA-256 over `PAGED_PREVIEW_INLINE_SCRIPT`'s exact formula. Plan 33-03 (on-screen preview wiring) can import `buildSrcdoc`, `attachBridge`, and `pluralizeRu` directly — none of these are wired into `PdfPreviewModal.svelte` yet, as intended (that is Plan 33-03's job). No blockers identified.

---
*Phase: 33-print-preview-polish*
*Completed: 2026-08-04*
