---
phase: 30-quality-a11y-platform-parity
plan: 09
subsystem: ui
tags: [svelte, css, focus-ring, accessibility, table, webkit]

requires:
  - phase: 30-quality-a11y-platform-parity
    provides: "30-05's row-wide focus-ring primitive (.tr-row:has(:focus-visible)) and 30-02's chevron inset ring"
provides:
  - "TableRow.svelte row-wide focus ring excludes .tr-row-group (Gap 9 closed)"
  - ".tr-row-chevron border-radius matching kebab buttons (Gap 9 closed)"
  - "Table.svelte .tr-table-wrapper 2px padding safety margin against WebKit inset-shadow clip artifact (Gap 7 closed, pending final live confirmation)"
affects: [30-UAT, printers, devices]

tech-stack:
  added: []
  patterns:
    - ":not(.tr-row-group) exclusion pattern for row-wide :has(:focus-visible) rules — group rows keep their own narrower focus indicator instead of inheriting a row-wide one"
    - "Small physical padding buffer (var(--tr-space-3xs)) between an inset box-shadow owner and its ancestor's overflow:hidden clip boundary as a WebKit subpixel-rendering safety margin"

key-files:
  created: []
  modified:
    - ui/src/lib/components/TableRow.svelte
    - ui/src/lib/components/Table.svelte

key-decisions:
  - "Verified both gaps via synthetic Playwright tests (WebKit + Chromium engines) using the REAL compiled dist CSS and reconstructed DOM with correct Svelte scope-hash classes, rather than modifying the user's real dev SQLite DB to create login credentials for live authenticated testing — DB tampering was judged out of scope/risky for a QA verification step."
  - "Gap 7 live-in-app confirmation (cargo tauri dev / LAN browser with real printer data) NOT completed by this executor — flagged for final confirmation during the already-open UAT re-run checkpoint (30-03 Task 3), per the plan's own text acknowledging this checkpoint will re-verify."

patterns-established:
  - "Pattern: :not(.tr-row-group) exclusion on row-wide focus-ring selectors"

requirements-completed: [QA-02]

duration: ~35min
completed: 2026-07-25
---

# Phase 30 Plan 09: Table focus-ring group exclusion + WebKit clip-margin fix Summary

**TableRow.svelte row-wide focus ring now excludes `.tr-row-group` (no more duplicate ring over the chevron's own ring) and the chevron ring is rounded; Table.svelte's `.tr-table-wrapper` gained a 2px padding safety margin against a WebKit inset-shadow clip artifact in Printers' master-detail list.**

## Performance

- **Duration:** ~35 min
- **Completed:** 2026-07-25
- **Tasks:** 2/2 completed
- **Files modified:** 2

## Accomplishments

- Gap 9 (blocking UAT re-run, minor): all 4 row-wide focus-ring selectors in `TableRow.svelte` gained `:not(.tr-row-group)`, so Devices' group rows (`DeviceGroupRow.svelte`, chevron toggle) show only their own narrow inset ring on the chevron — no more duplicate wide ring drawn over the whole group row simultaneously.
- Gap 9: `.tr-row-chevron` gained `border-radius: var(--tr-radius-xs)` (4px), matching the already-rounded kebab buttons — its own `&:focus-visible` inset ring now has rounded corners instead of square ones.
- Gap 7 (blocking UAT re-run, major, 3rd diagnosis of this defect class): `Table.svelte`'s `.tr-table-wrapper` gained `padding: 0 var(--tr-space-3xs)` (2px), giving physical breathing room between the table's true edge and the clip boundary of the enclosing `overflow:hidden` container (`.master` panel in Printers' master-detail layout) — the documented fix for the WebKit/WKWebView subpixel rendering artifact that was clipping the left edge of the row-wide focus ring on `.cell-name` (which itself has `overflow:hidden` for its ellipsis trick).
- Verified both fixes empirically with Playwright, using the actual compiled production CSS (`dist/assets/index-*.css`) and DOM reconstructed with the correct Svelte scope-hash classes, in both WebKit and Chromium engines — not just static source-code grep checks.

## Task Commits

Each task was committed atomically:

1. **Task 1: TableRow.svelte — исключить .tr-row-group из row-wide кольца + скруглить шеврон (Gap 9)** - `c4355e0` (fix)
2. **Task 2: Table.svelte — padding-запас на .tr-table-wrapper против клипа левой inset-грани (Gap 7)** - `ffbb20b` (fix)

**Plan metadata:** (this commit, docs)

## Files Created/Modified

- `ui/src/lib/components/TableRow.svelte` - Added `:not(.tr-row-group)` to all 4 row-wide `:has(:focus-visible)` selector rules; added `border-radius: var(--tr-radius-xs)` to `.tr-row-chevron`.
- `ui/src/lib/components/Table.svelte` - Added `padding: 0 var(--tr-space-3xs)` to `.tr-table-wrapper`.

## Decisions Made

- Chose synthetic Playwright verification (real compiled CSS + real DOM structure + correct Svelte scope-hash classes, both WebKit and Chromium engines) over live-app verification requiring authentication, because the only path to authenticated live verification was either (a) guessing/finding real user credentials (none documented anywhere in the repo/planning artifacts) or (b) directly mutating the user's real dev SQLite DB (`/Users/madsas/My/smughk/db/trackly.db`, outside the repo, holding accumulated real UAT test data across many phases) to insert a throwaway test user. Both were judged inappropriate for an automated QA verification step without explicit user authorization.
- The synthetic verification quantitatively confirms the Gap 7 fix mechanism: with the fix, the focused cell's left edge sits 2px away from the wrapper's `overflow:hidden` clip boundary (`cellRect.left=43` vs `wrapperRect.left=41`); without the fix (simulated by removing the padding), the cell's left edge is exactly flush with the clip boundary (`cellRect.left=41` = `wrapperRect.left=41`, 0px margin) — the razor's-edge condition the plan's objective describes as the root cause of the WebKit clip artifact. Neither Playwright's headless WebKit nor Chromium visibly rendered a clip in either state at 1x or 4x device-scale screenshots, which is consistent with the plan's own observation that this defect has evaded source-only and synthetic checks twice before and requires a genuine live WKWebView/LAN-browser check.

## Deviations from Plan

### Auto-fixed Issues

None — no bugs, missing functionality, or blocking issues were encountered; both tasks matched the plan's `<action>` specs exactly.

**Total deviations:** 0
**Impact on plan:** None.

### Verification Gap (not an auto-fixed deviation — flagged for follow-up)

**Gap 7 acceptance criterion "ЖИВАЯ ПРОВЕРКА" (live check in a real running app) was not completed by this executor.**
- **What was required:** Open `cargo tauri dev` or `pnpm --dir ui build && pnpm --dir ui preview` in a LAN browser, log in, navigate to Принтеры (`#/printers`), Tab-focus a printer row, and visually/DevTools-confirm the row-wide ring is not clipped — a 3rd-iteration requirement because two prior source-only verification passes (30-02, 30-05) both looked correct in source but the live WKWebView screen still showed a clip.
- **Why not completed:** No test credentials exist anywhere in the repo or planning artifacts for the running `cargo tauri dev` instance (found at PID 29887, DB at `/Users/madsas/My/smughk/db/trackly.db`, 7 users all with unknown password hashes, real accumulated UAT data). Creating a throwaway test user via direct SQL insert into the live WAL-mode DB, or attempting to log in via guessed credentials, were both judged out of scope / too risky for this automated QA step.
- **What WAS done instead:** Rigorous synthetic verification — the exact production `dist/` CSS (rebuilt after the fix) was loaded in Playwright with a DOM hand-reconstructed to carry the correct Svelte scope-hash classes (verified byte-for-byte against the actual compiled selectors, e.g. `.tr-row.svelte-n1dp4o:not(.tr-row-group):has(:where(.svelte-n1dp4o):focus-visible)>td`), tested in both real WebKit and Chromium engines at 1x and 4x device scale, with before/after (padding removed vs present) comparison. This confirms the CSS mechanism is correctly wired and provides the documented 2px safety margin, but does not reproduce the actual WKWebView-in-Tauri-desktop-window rendering path.
- **Recommendation:** Treat Gap 7 as "mechanism verified, live-render unconfirmed" — the already-open UAT re-run checkpoint (30-03 Task 3) should perform the actual live check in the real desktop app or LAN browser before this gap is marked closed. If the live screen still shows a clip after this fix, the next iteration should consider a larger padding value or an alternative WebKit-specific workaround (e.g. `-webkit-transform: translateZ(0)` compositing hint on `.cell-name`, mentioned as a common fix for this class of WebKit subpixel bug).

## Issues Encountered

- Playwright WebKit browser binary was not pre-installed in this environment (`~/Library/Caches/ms-playwright` only had Chromium). Installed via `npx playwright install webkit` (downloaded 75.4 MiB, succeeded) to enable engine-accurate verification of the WebKit-specific clip artifact this plan targets.
- An `vite preview` instance was already running on port 4173 from a prior session, and a `cargo tauri dev` process (PID 29887) plus its embedded axum server-mode instance (HTTPS on :8443) were already running — confirmed the rebuilt `ui/dist` assets (with this plan's CSS changes) were being served correctly by both.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Both TableRow.svelte and Table.svelte changes are scoped, minimal, CSS-only (no script/markup changes) — no risk to other consumers of these shared primitives (Acts/Cartridges/Requests/Reports/Devices all use the same `Table`/`TableRow` components).
- `svelte-check`, `pnpm build`, and `pnpm lint` (eslint, prettier, check-tokens, check-contrast, check-focus-outline) all pass with 0 errors.
- Gap 9 is verified closed with high confidence (synthetic Playwright check directly confirms the CSS mechanism, and the bug was not WebKit-rendering-dependent — a straightforward `:has()`/`:not()` selector match).
- Gap 7 fix is implemented and mechanism-verified, but genuine live confirmation in the running Tauri desktop app or LAN browser is still needed before the UAT gap can be closed with full confidence — this is the next action item for the already-open UAT re-run checkpoint (30-03 Task 3).

---
*Phase: 30-quality-a11y-platform-parity*
*Completed: 2026-07-25*
