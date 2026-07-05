---
phase: 16-documents-html-print
plan: 04
subsystem: ui
tags: [svelte, iframe, srcdoc, print, http-transport, act-service]

# Dependency graph
requires:
  - phase: 16-documents-html-print
    plan: 03
    provides: "acts_render_pdf/devices_render_acceptance_pdf Tauri+HTTP adapters returning String/text/html; acts_open_pdf_in_system fully removed backend-side; ui/src/bindings.ts regenerated with Promise<Result<string, AppError>>"
provides:
  - "ui/src/lib/api/acts.ts renderPdf/renderAcceptancePdf typed Promise<string>"
  - "ui/src/lib/api/pdf.ts deleted (blob-conversion helpers had zero remaining callers)"
  - "PdfPreviewModal.svelte previews HTML via iframe srcdoc, no blob/object-URL lifecycle"
  - "acts_open_pdf_in_system fully removed from the frontend (matches 16-03's backend removal)"
  - "Save-as-PDF UI action removed (printing via browser dialog already offers Save-as-PDF)"
  - "ui/src/lib/api/client.ts HTTP transport correctly routes text/html responses as strings, not byte arrays"
affects: [16-documents-html-print, 16-05-tests]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "iframe srcdoc for same-origin trusted HTML preview + print(), replacing blob:-URL object lifecycle entirely"
    - "client.ts content-type branch: text/html -> res.text() (string), application/json -> res.json(), else -> Uint8Array/number[] (binary fallback for reports_export_pdf)"

key-files:
  created: []
  modified:
    - ui/src/lib/api/acts.ts
    - ui/src/lib/api/pdf.ts (deleted)
    - ui/src/features/acts/PdfPreviewModal.svelte
    - ui/src/features/acts/ActsPage.svelte
    - ui/src/features/devices/DevicesPage.svelte
    - ui/src/lib/api/client.ts

key-decisions:
  - "Save-as-PDF button/handleSave() removed entirely (not repurposed to save raw HTML) per D-09/Req 5 — the browser's native print dialog already offers Save-as-PDF as a destination; keeping a separate action would have required either corrupting output (writing a string as Uint8Array) or introducing a new HTML-file-save feature nobody asked for"
  - "actNumberDisplay/actDateUtc props removed from PdfPreviewModal's Props interface (Rule 1 fix) — they only existed to feed the now-deleted filename-suggestion helpers (suggestedFilename/sanitizeFilename/isoDateForFilename); leaving them as unused destructured bindings failed svelte-check's declared-but-never-read check. Both callers (ActsPage.svelte, DevicesPage.svelte) updated to stop passing them."
  - "Rule 1 fix in client.ts (outside this plan's stated files_modified but directly blocking D-09's dual-transport requirement): the HTTP/LAN-browser transport's binary-response branch matched on '!contentType.includes(application/json)', which also caught the new text/html responses and wrongly converted the HTML string into a number[] byte array. Added an explicit text/html branch returning res.text() ahead of the binary fallback."
  - "templates_render_preview's stale application/pdf content-type (http/templates.rs) and its frontend wrapper's stale Promise<number[]> type (ui/src/lib/api/templates.ts) were found but NOT fixed — zero callers anywhere in ui/src (dead code), outside this plan's files_modified, originated in Plan 16-02/16-03. Logged to deferred-items.md per the scope-boundary rule."

requirements-completed: [SPEC-Req5]

# Metrics
duration: 20min
completed: 2026-07-05
---

# Phase 16 Plan 04: Frontend Delivery — srcdoc Preview + Print, System-Open Removal Summary

**PdfPreviewModal.svelte now renders the backend's HTML string directly via `<iframe srcdoc>` and calls `window.print()` — replacing the entire blob-URL/PDF-bytes pipeline, deleting the obsolete `acts_open_pdf_in_system` UI path, and fixing a transport-layer bug where the LAN-browser HTTP client was silently corrupting HTML responses into byte arrays.**

## Performance

- **Duration:** ~20 min
- **Started:** 2026-07-05T09:55:00Z (approx)
- **Completed:** 2026-07-05T10:15:13Z
- **Tasks:** 2 planned + 1 unplanned Rule-1 fix + 1 deferred-items log
- **Files modified:** 6 (1 deleted)

## Accomplishments

- `acts.ts`'s `renderPdf`/`renderAcceptancePdf` now typed `Promise<string>` / `apiCall<string>`, matching the backend's HTML-string contract completed in Plans 16-02/16-03.
- `pdf.ts` deleted entirely — `fetchPdfBlob`/`revokePdfUrl` had zero remaining callers once `PdfPreviewModal.svelte` moved off the blob-URL pipeline.
- `PdfPreviewModal.svelte` rewritten: `blobUrl`/`pdfBytes` state replaced by a single `htmlContent` string; the `$effect` block assigns the rendered HTML directly (no `Blob`/object-URL construction or revocation); the iframe uses `srcdoc={htmlContent}` instead of `src={blobUrl}`; `handlePrint()` is byte-for-byte unchanged.
- `handleOpen()` and its "Открыть в системном просмотрщике" footer button deleted (the system-viewer flow tied to `acts_open_pdf_in_system`, removed backend-side in Plan 16-03).
- `handleSave()` and "Сохранить как PDF" footer button deleted per D-09/Req 5 — printing via the browser's native dialog already offers "Save as PDF" as a destination, and no code path could safely binary-write `htmlContent` (a `string`) as PDF bytes anymore.
- Cleaned up now-dead `suggestedFilename`/`sanitizeFilename`/`isoDateForFilename` helpers and the `actNumberDisplay`/`actDateUtc` props that only existed to feed them; updated both call sites (`ActsPage.svelte`, `DevicesPage.svelte`) accordingly.
- **Rule 1 fix (client.ts):** discovered and fixed a real bug in the HTTP/LAN-browser transport — the binary-response detection (`!contentType.includes('application/json')`) also matched the new `text/html; charset=utf-8` responses from `acts_render_pdf`/`devices_render_acceptance_pdf`, converting the HTML string into a `number[]` byte array instead of returning it as text. This would have broken the LAN-browser half of D-09's "same code path in both transports" requirement. Added an explicit `text/html` branch (`res.text()`) ahead of the binary fallback.
- Confirmed via full-repo grep: `acts_open_pdf_in_system`, `fetchPdfBlob`, `revokePdfUrl` all return zero matches anywhere in `ui/src`.

## Task Commits

Each task was committed atomically:

1. **Task 1: acts.ts return types + delete pdf.ts** - `2b25a90` (feat)
2. **Task 2: PdfPreviewModal.svelte — srcdoc preview, remove system-open, resolve save-as** - `aeeceed` (feat)
3. **Rule 1 fix: client.ts text/html transport bug** - `2889439` (fix)
4. **Deferred-items log** - `55c38ae` (docs)

## Files Created/Modified

- `ui/src/lib/api/acts.ts` - `renderPdf`/`renderAcceptancePdf` return `Promise<string>`; doc comments updated
- `ui/src/lib/api/pdf.ts` - Deleted (blob-conversion helpers, zero remaining callers)
- `ui/src/features/acts/PdfPreviewModal.svelte` - `htmlContent` state + `srcdoc` iframe; `handleOpen`/`handleSave` and their footer buttons removed; dead filename helpers and unused props removed
- `ui/src/features/acts/ActsPage.svelte` - Stopped passing removed `actNumberDisplay`/`actDateUtc` props to `PdfPreviewModal`
- `ui/src/features/devices/DevicesPage.svelte` - Same prop cleanup for the acceptance-mode call site
- `ui/src/lib/api/client.ts` - Added `text/html` response branch (`res.text()`) ahead of the binary/`Uint8Array` fallback; fixed a stale comment referencing the deleted `fetchPdfBlob`

## Decisions Made

- Removed Save-as-PDF entirely rather than repurposing it to save raw HTML — matches Req 5's acceptance criterion that printing/saving happens through the browser's native print dialog; avoids introducing a new "save HTML file" feature nobody asked for.
- Removed `actNumberDisplay`/`actDateUtc` from `PdfPreviewModal`'s `Props` interface (not just left unbound) since they had zero remaining consumers post-cleanup — matches the codebase's usual practice of not carrying dead props, and was required to satisfy `svelte-check`'s unused-binding error.
- Fixed the `client.ts` HTTP-transport bug in this plan rather than deferring it, despite the file being outside the plan's stated `files_modified` — it directly blocks this plan's own success criterion (`PdfPreviewModal` must work correctly in the LAN-browser webview per D-09), so leaving it broken would have left Task 2's deliverable non-functional over HTTP even though `svelte-check`/`build` don't catch runtime content-type mismatches.
- Left `templates_render_preview`'s parallel stale content-type/type mismatch unfixed (dead code, zero UI callers, outside this plan's scope) — logged to `deferred-items.md` instead.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Unused `actNumberDisplay`/`actDateUtc` props broke svelte-check**
- **Found during:** Task 2, immediately after removing the filename-suggestion helpers
- **Issue:** `svelte-check` failed with 2 errors — `'actNumberDisplay' is declared but its value is never read.'` / same for `actDateUtc` — since their only consumer (`suggestedFilename()`) had just been deleted per the plan's own instructions.
- **Fix:** Removed both fields from the `Props` interface and the `$props()` destructure; updated `ActsPage.svelte` and `DevicesPage.svelte` to stop passing them.
- **Files modified:** `ui/src/features/acts/PdfPreviewModal.svelte`, `ui/src/features/acts/ActsPage.svelte`, `ui/src/features/devices/DevicesPage.svelte`
- **Commit:** `aeeceed`

**2. [Rule 1 - Bug] client.ts HTTP transport corrupted text/html responses**
- **Found during:** Task 2 completion sweep (grepping for stale references to deleted `fetchPdfBlob`)
- **Issue:** `client.ts`'s binary-response detection treated any non-`application/json` content-type as binary bytes, including the new `text/html; charset=utf-8` responses from `acts_render_pdf`/`devices_render_acceptance_pdf` — this would silently convert the HTML string into a `number[]` byte array on the LAN-browser/HTTP transport (Tauri desktop transport is unaffected — it uses `invoke()`, not this `fetch`-based code path).
- **Fix:** Added an explicit `contentType.includes('text/html')` branch returning `await res.text()` ahead of the binary fallback.
- **Files modified:** `ui/src/lib/api/client.ts`
- **Commit:** `2889439`

---

**Total deviations:** 2 auto-fixed (both Rule 1 — bugs directly caused by this plan's own changes)
**Impact on plan:** Both fixes were necessary for the plan's own success criteria (svelte-check passing; D-09's dual-transport requirement actually working). No scope creep — the `templates_render_preview` mismatch found during investigation was explicitly left out and deferred.

## Issues Encountered

None beyond the two auto-fixes above.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- Plan 16-05 (tests) already landed (commits `bbf3e4b`, `ae22b9b`, `dd5cc2c`, `f9e8c90`) — it did not depend on this plan's frontend changes since it covers backend HTML-generation assertions, not the UI.
- Frontend is now fully wired end-to-end: Rust `String` return → Tauri `invoke`/HTTP `text/html` → `acts.ts` `Promise<string>` → `PdfPreviewModal.svelte`'s `srcdoc` iframe → `window.print()`, working identically in both desktop and LAN-browser transports (the `client.ts` fix closes the gap that would have broken the LAN-browser half).
- `ui/dist` rebuilt via `pnpm --dir ui build` so server-mode/LAN-browser serves the updated bundle immediately (per dev-environment convention).
- One deferred item remains (`templates_render_preview` stale content-type/type — dead code, no live impact) — see `.planning/phases/16-documents-html-print/deferred-items.md`.

---
*Phase: 16-documents-html-print*
*Completed: 2026-07-05*
