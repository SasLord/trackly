---
phase: 17-html-krilla
plan: 03
subsystem: reports-templates-frontend
tags: [svelte, html-print, reports, template-editor, frontend]

# Dependency graph
requires:
  - phase: 17-html-krilla
    plan: 01
    provides: ReportService::export_pdf HTML render (reports_export_pdf returns string)
  - phase: 17-html-krilla
    plan: 02
    provides: TemplateService file-backed editor (templates_list_for_editor/templates_validate_preview returning string)
  - phase: 17-html-krilla
    plan: 04
    provides: HTML-render regression test suite confirming backend contract stability
provides:
  - PdfPreviewModal.svelte mode='report' (self-fetch reports_export_pdf via apiCall<string>)
  - ReportsPage.svelte export/print wired onto the modal (old blob/tauri-plugin-fs download path deleted)
  - TemplateEditor.svelte file-backed HTML editor UI (report kind selectable, srcdoc preview, per-kind variables panel)
affects: [phase-17-closure, verifier]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "PdfPreviewModal mode union extended additively (handover/acceptance/report) — each mode's renderCall() branch and ready $derived branch added without touching print machinery (printViaSystemBrowser/printViaTopLevel/handlePrint untouched)"
    - "ReportsPage export+print unified onto a single reportModalOpen boolean opening PdfPreviewModal mode='report' — modal self-fetches on open via its own $effect, caller only supplies reportParams"
    - "TemplateEditor per-kind variables panel driven by a Record<string, VariableEntry[]> constant + currentVariables $derived keyed on selectedKind, replacing the old static two-column hardcoded markup"

key-files:
  created:
    - .planning/phases/17-html-krilla/17-03-SUMMARY.md
  modified:
    - ui/src/features/acts/PdfPreviewModal.svelte
    - ui/src/features/reports/ReportsPage.svelte
    - ui/src/features/settings/TemplateEditor.svelte

key-decisions:
  - "D-09/D-10 implemented exactly as specified: PdfPreviewModal extended with mode='report' as a pure additive change; ReportsPage's exportPdf()/onPrint both now open the same modal instance, printReport() deleted entirely, exportCsv() untouched"
  - "D-11/D-12 implemented exactly as specified: TemplateEditor's preview state renamed blobUrl -> previewHtml (plain string, no URL.createObjectURL/revokeObjectURL lifecycle), rendered via iframe srcdoc; variables panel driven by a new VARIABLES_BY_KIND per-kind constant instead of one static hardcoded block"
  - "Reworded an in-code comment in ReportsPage.svelte's exportPdf() (mentioned 'tauri-plugin-fs' as historical context) to avoid a grep false-positive against the plan's own acceptance-criteria check for 'blob/download path fully removed' — no functional change, comment-only edit"
  - "Minor UX-accuracy fix (out of plan's must_haves but Rule 1 in spirit): TemplateEditor's 'Проверить (превью PDF)' button label changed to 'Проверить (превью)' since the preview is now HTML, not PDF"

requirements-completed: [Req-4, Req-5]

# Metrics
duration: 25min
completed: 2026-07-07
---

# Phase 17 Plan 03: Отчёты и Шаблоны — фронтенд на HTML-контракте Summary

**Wired the frontend onto the HTML-returning backend contracts from Plans 17-01/17-02: PdfPreviewModal gained a third mode='report' (pure additive extension), ReportsPage's export/print buttons now open that modal instead of the old blob/tauri-plugin-fs download path, and TemplateEditor.svelte previews via HTML srcdoc with a report kind and per-kind variables panel.**

## Performance

- **Duration:** ~25 min
- **Started:** 2026-07-07 (session start)
- **Completed:** 2026-07-07
- **Tasks:** 3/3
- **Files modified:** 3 (PdfPreviewModal.svelte, ReportsPage.svelte, TemplateEditor.svelte) + bindings.ts regenerated in place (gitignored, not committed)

## Accomplishments

- `ui/src/bindings.ts` regenerated via `cargo test -p trackly-app --test export_bindings` — confirmed `reports_export_pdf` and `templates_validate_preview` both return `Promise<Result<string, AppError>>` (not `number[]`); file confirmed gitignored (no `git status` output for it).
- `PdfPreviewModal.svelte` extended with `mode='report'`: new exported `ReportParams` interface (`reportType`, `filter`, optional `period`), `reportParams` prop (default `null`), a `renderCall()` branch calling `apiCall<string>('reports_export_pdf', {...})`, and a 3-way `ready` `$derived`. Zero changes to `printViaSystemBrowser`/`printViaTopLevel`/`handlePrint`/the iframe markup — the extension is purely additive per D-09.
- `ReportsPage.svelte`: imported `PdfPreviewModal`, added `reportModalOpen` state. `exportPdf()` reduced to a single `reportModalOpen = true` statement (D-10); `printReport()` deleted entirely; `ReportFilters`' `onPrint` now points at the same `exportPdf` trigger. Rendered a `PdfPreviewModal` instance at the bottom of the template with `mode="report"`, `actId={null}`, `title="Печать отчёта"`, `reportParams={{ reportType: reportTypeKey(), filter, period: isSnapshot() ? undefined : period }}`, and `onClose` resetting `reportModalOpen`. `exportCsv()` and its wiring left completely untouched.
- `TemplateEditor.svelte`: added a `report: 'Отчёт'` entry to `KIND_LABELS` (kind-select now offers all 3 kinds once `templates_list_for_editor` returns 3 file-backed items). Replaced `blobUrl`/`URL.createObjectURL`/`URL.revokeObjectURL` state and cleanup with a plain `previewHtml: string | null` state; `validateAndPreview()` now does a single `previewHtml = await apiCall<string>('templates_validate_preview', {...})` assignment; the kind-change `$effect` simply nulls `previewHtml` (no revoke needed); the preview markup renders `<iframe srcdoc={previewHtml}>` instead of a blob-URL `src`. Replaced the static two-column hardcoded variables grid with a new `VARIABLES_BY_KIND` constant (3 entries: `act_handover`, `act_acceptance`, `report`, each sourced from the corresponding `templates/*.html` doc-comment) and a `currentVariables` `$derived`, rendered via an each-block.

## Task Commits

Each task was committed atomically:

1. **Task 1: Regenerate bindings.ts against the Plan 17-01/17-02 backend changes** - no commit (bindings.ts is gitignored, regenerated in place, not tracked — confirmed via `git status`)
2. **Task 2: Extend PdfPreviewModal with mode='report' and rewire ReportsPage export/print to the modal** - `34f45d2` (feat)
3. **Task 3: Retarget TemplateEditor.svelte to the file-backed HTML editor contract** - `74f4e10` (feat)

## Files Created/Modified

- `ui/src/features/acts/PdfPreviewModal.svelte` - Added `ReportParams` interface, `mode='report'` union member, `reportParams` prop, `renderCall()` branch (self-fetch `reports_export_pdf` via `apiCall<string>`), 3-way `ready` `$derived`. Print machinery (`printViaSystemBrowser`/`printViaTopLevel`/`handlePrint`) and iframe markup untouched.
- `ui/src/features/reports/ReportsPage.svelte` - Imported `PdfPreviewModal`; added `reportModalOpen` state; `exportPdf()` reduced to opening the modal; `printReport()` deleted; `ReportFilters`' `onPrint` repointed at `exportPdf`; rendered `PdfPreviewModal mode="report"` instance at template bottom. `exportCsv()` untouched.
- `ui/src/features/settings/TemplateEditor.svelte` - Added `report` kind to `KIND_LABELS`; replaced `blobUrl` state/lifecycle with `previewHtml` string state; `validateAndPreview()` retargeted to HTML string assignment; preview iframe uses `srcdoc`; added `VARIABLES_BY_KIND` constant + `currentVariables` `$derived` replacing the static hardcoded variables grid; minor button-label accuracy fix ("превью PDF" → "превью").

## Decisions Made

- Followed D-09 through D-12 from `17-CONTEXT.md`/plan frontmatter exactly as specified: `PdfPreviewModal` extension is purely additive (no rewrite of existing print machinery); `ReportsPage`'s export and print buttons converge on one modal-opening trigger; `TemplateEditor`'s preview state and markup fully decoupled from blob/PDF-object-URL lifecycle; variables panel is per-kind data-driven, not one static block.
- Reworded one in-code comment in `ReportsPage.svelte` (mentioned "tauri-plugin-fs" as historical context describing what was removed) after it tripped the plan's own acceptance-criteria grep (`grep -c "tauri-plugin-fs\|writeFile\|@tauri-apps/plugin-dialog"` expecting 0) — this is a comment-only edit, no functional code was affected; the actual import/call-site removal was already complete.
- Minor accuracy fix to `TemplateEditor.svelte`'s "Проверить (превью PDF)" button label → "Проверить (превью)" since the preview surface is now HTML, not PDF — small enough in scope to fold into Task 3 rather than flag as a separate deviation, but noted here for traceability.

## Deviations from Plan

None requiring the formal Rule 1-4 process — the one comment-reword above was a self-inflicted grep false-positive against this plan's own acceptance criteria (the code itself was already correct; only a descriptive comment needed adjusting), caught and fixed during Task 2's own verification step before committing.

## Issues Encountered

None. `cargo test -p trackly-app --test export_bindings` completed quickly (bindings.ts regeneration is a lightweight, targeted test, unlike the full workspace suite noted as slow in Plans 17-01/17-02/17-04). `pnpm --dir ui exec svelte-check` and `pnpm --dir ui build` both completed cleanly with 0 errors (pre-existing warnings in unrelated files only).

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Phase 17's full stack (backend HTML render in 17-01/17-02, frontend consumer in this plan, verification suite in 17-04) is now complete end-to-end: Reports export/print and the Templates editor both consume the HTML contract, with zero blob/PDF-object-URL/tauri-plugin-fs download machinery remaining on either surface.
- `ui/dist` rebuilt (`pnpm --dir ui build`) so LAN-browser/server-mode testing serves the current frontend.
- Manual verification (open Reports page, click "Экспорт PDF"/"Печать"; open Settings > Шаблоны, select "Отчёт") is recommended per the plan's `<verification>` section but was not performed interactively in this session — code-level verification (svelte-check, build, bindings regeneration, acceptance-criteria greps) all passed.
- No blockers identified for phase closure / milestone review.

---
*Phase: 17-html-krilla*
*Completed: 2026-07-07*

## Self-Check: PASSED

- FOUND: ui/src/features/acts/PdfPreviewModal.svelte
- FOUND: ui/src/features/reports/ReportsPage.svelte
- FOUND: ui/src/features/settings/TemplateEditor.svelte
- FOUND: .planning/phases/17-html-krilla/17-03-SUMMARY.md
- FOUND commit: 34f45d2 (Task 2)
- FOUND commit: 74f4e10 (Task 3)
