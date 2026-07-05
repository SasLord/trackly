# Phase 16 — Deferred Items

Items discovered during execution that are out of scope for the current plan's
`files_modified` boundary and were not auto-fixed (SCOPE BOUNDARY rule).

## 16-04: `templates_render_preview` HTTP content-type / frontend type mismatch

- **Found during:** Plan 16-04, Task 2 (auditing `client.ts` for stale binary-response
  comments referencing the now-deleted `fetchPdfBlob`).
- **Issue:** `build_templates_render_preview` (Plan 16-02) already returns
  `Result<String, AppError>` (HTML), but:
  - `crates/trackly-app/src/http/templates.rs::handler_render_preview` still responds
    with `Content-Type: application/pdf` (stale — should be `text/html; charset=utf-8`
    to match `http/acts.rs`'s handlers).
  - `ui/src/lib/api/templates.ts::renderPreview` is still typed
    `Promise<number[]>`/`apiCall<number[]>` (stale — should be `Promise<string>`).
- **Why not fixed here:** Both files are outside 16-04's `files_modified`
  (`ui/src/lib/api/acts.ts`, `ui/src/lib/api/pdf.ts`,
  `ui/src/features/acts/PdfPreviewModal.svelte`). `renderPreview` has zero callers
  anywhere in the UI (`grep -rln "renderPreview" ui/src` → only the declaration
  itself) — dead code, not a live-breaking bug exercised by any current user flow.
  Originated in Plan 16-02/16-03, not caused by this plan's edits.
- **Recommended fix (future plan or quick task):** Update
  `handler_render_preview`'s content-type to `text/html; charset=utf-8` and
  `templates.ts::renderPreview`'s return type to `Promise<string>`/`apiCall<string>`,
  matching the `acts.ts` pattern from this plan. Low priority since no UI feature
  currently calls it.
