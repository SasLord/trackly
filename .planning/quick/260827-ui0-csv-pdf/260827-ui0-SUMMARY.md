---
phase: 260827-ui0
plan: 01
subsystem: ui
tags: [svelte, tauri, csv-export, save-dialog, reports]

requires: []
provides:
  - Reusable `saveFile(bytes, suggestedName, mimeType)` helper for Tauri desktop (native save dialog + writeFile) and LAN browser (Blob + anchor download without revokeObjectURL race)
  - Working "Экспорт CSV" in Reports — replaces broken detached-anchor blob download
  - Extended Tauri `fs:allow-write-file` ACL scope covering `*.csv` alongside existing `*.pdf`
affects: [reports, future bytes-to-file export flows]

tech-stack:
  added: []
  patterns:
    - "saveFile.ts: shared isTauri-guard + dynamic-import pattern for delivering in-memory bytes to disk/download, reusable for any future bytes→file export"

key-files:
  created:
    - ui/src/lib/utils/saveFile.ts
  modified:
    - ui/src/features/reports/ReportsPage.svelte
    - crates/trackly-app/capabilities/main.json

key-decisions:
  - "Extracted saveFile() as a shared helper rather than a one-off CSV patch — second call site for 'hand user a file' after StorageSettings' DB-move flow, so the pattern (native dialog + writeFile in Tauri, Blob+anchor in browser) is centralized instead of re-copied a third time"
  - "Cancelled save dialog (Tauri save() returns null) is a normal user action, not an error — no toast; only a real writeFile/apiCall exception shows the error toast"
  - "CSV filename now includes reportTypeKey() + local ISO date (not toISOString(), which drifts a day around Moscow midnight) so repeated exports don't collapse into an identical 'отчёт.csv'"
  - "PDF export path (exportPdf/PdfPreviewModal/reports_export_pdf) intentionally untouched — out of scope, already works via preview+print modal"

requirements-completed: [UI0-01, UI0-02]

duration: ~20min
completed: 2026-08-27
---

# Quick Task 260827-ui0: Fix broken CSV export in Reports Summary

**Replaced the broken detached-blob-anchor CSV download in ReportsPage with a shared `saveFile` helper that uses Tauri's native save dialog + `writeFile` in the desktop webview, and a race-free Blob+anchor download in the LAN browser.**

## Performance

- **Duration:** ~20 min
- **Tasks:** 2/3 completed (Task 3 is a blocking human-verify checkpoint — see below)
- **Files modified:** 3 (1 created, 2 modified)

## Accomplishments

- `ui/src/lib/utils/saveFile.ts` created: `saveFile(bytes, suggestedName, mimeType): Promise<'saved' | 'cancelled'>`. Tauri branch uses `@tauri-apps/plugin-dialog` `save()` + `@tauri-apps/plugin-fs` `writeFile()` (same pattern as `StorageSettings.svelte`/`OrgSettings.svelte`). Browser branch appends the anchor to the DOM before `click()` and defers `revokeObjectURL` via `setTimeout` instead of calling it synchronously (that synchronous call was the original download-killing race).
- `crates/trackly-app/capabilities/main.json`: `fs:allow-write-file.allow` extended with 5 `*.csv` entries (`$TEMP`, `$HOME`, `$DOCUMENT`, `$DESKTOP`, `$DOWNLOAD`) mirroring the existing `*.pdf` entries — without this, Tauri's ACL silently returns permission-denied on `writeFile` for CSV paths, a failure mode invisible to compile-time gates.
- `ReportsPage.svelte`'s `exportCsv` rewritten as `async function`, delegates to `saveFile`, builds filename as `отчёт-${reportTypeKey()}-${YYYY-MM-DD}.csv`, shows a success toast only on `'saved'`, no toast on `'cancelled'`, error toast only on a real thrown exception.

## Task Commits

1. **Task 1: Общий хелпер saveFile + расширение Tauri ACL под .csv** - `d2d6c287` (feat)
2. **Task 2: Wiring — exportCsv использует saveFile вместо сломанного blob-якоря** - `a53ae284` (fix)
3. **Task 3: Human UAT — CSV реально сохраняется в обоих вебвью** - **PENDING** (blocking checkpoint, not yet approved by user)

## Files Created/Modified

- `ui/src/lib/utils/saveFile.ts` - new shared bytes→file helper (Tauri native dialog+writeFile / browser Blob+anchor)
- `ui/src/features/reports/ReportsPage.svelte` - `exportCsv` rewritten to use `saveFile`; new `buildCsvFilename()` helper
- `crates/trackly-app/capabilities/main.json` - `fs:allow-write-file` ACL scope extended to `*.csv`

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `new Blob([bytes], ...)` failed strict TypeScript typecheck**
- **Found during:** Task 1 verification (`tsc --noEmit`)
- **Issue:** `Uint8Array<ArrayBufferLike>` is not assignable to `BlobPart` under this project's TS 5.9 + strict `lib.dom.d.ts` combination (the generic buffer type includes `SharedArrayBuffer`, which `BlobPart`'s `ArrayBufferView<ArrayBuffer>` constraint rejects). The plan's interface spec (`new Blob([bytes], ...)`) did not anticipate this typing mismatch.
- **Fix:** Slice `bytes.buffer` into a definite `ArrayBuffer` (`bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength) as ArrayBuffer`) before constructing the `Blob`. Functionally identical output, just satisfies the stricter type.
- **Files modified:** `ui/src/lib/utils/saveFile.ts`
- **Commit:** `d2d6c287`

None else — plan executed as written otherwise.

## Out-of-Scope Pre-existing Issue (not fixed, logged per Scope Boundary rule)

`pnpm --dir ui exec tsc --noEmit` reports one pre-existing error unrelated to this task:
`src/features/acts/returnPayload.ts(15,15): error TS2614: Module '"*.svelte"' has no exported member 'ReturnRowState'.`
This file was not touched by this plan and the error predates this session (confirmed via `git show HEAD~2:...` — file already contained this import before Task 1). Left untouched per Scope Boundary rule; `svelte-check` (the actual gate used for `.svelte` files per this plan's `<verification>`) reports 0 errors, so this pre-existing plain-`tsc` finding does not block this task's verification criteria.

## Verification Results

- `pnpm --dir ui exec tsc --noEmit -p ./tsconfig.json` — 0 errors in `saveFile.ts` / `ReportsPage.svelte` (1 unrelated pre-existing error, see above).
- `python3 -c "import json; json.load(open('crates/trackly-app/capabilities/main.json'))"` — valid JSON.
- `pnpm --dir ui exec svelte-check --tsconfig ./tsconfig.json` — 0 ERRORS, 57 warnings (all pre-existing, unrelated files).
- `pnpm --dir ui lint` — clean (eslint, prettier, tokens, contrast, focus-outline, pagedjs-csp-hash, print-isolation all pass).
- `pnpm --dir ui build` — succeeded, `ui/dist` rebuilt (required for LAN/server-mode verification in Task 3).

No Rust changes in this plan — `cargo fmt --check` / `cargo test` sweep not applicable.

## Known Stubs

None.

## Threat Flags

None — this plan's `<threat_model>` already covers the only new surface (write-path ACL scope, save-dialog trust). No additional surface introduced beyond what's registered (T-ui0-01, T-ui0-02, T-ui0-SC).

## Checkpoint Pending — Task 3 (Human UAT)

This plan stops here per plan constraints (`type="checkpoint:human-verify" gate="blocking"`). Automated
verification (svelte-check/tsc/lint/build) cannot prove a CSV file actually reaches disk in the Tauri
webview or actually downloads in a LAN browser — that is exactly the class of defect the original bug
was invisible to these same gates.

### What was built

- `saveFile` helper (native save-dialog + `writeFile` in Tauri; race-free Blob+anchor download in browser)
- Extended `.csv` ACL scope
- `exportCsv` rewired to use `saveFile`, with separate cancel-vs-error handling

### How to verify

1. Ensure `ui/dist` is rebuilt (`pnpm --dir ui build` — already done in Task 2), then run `cargo tauri dev`.
2. **Desktop (Tauri):** open "Отчёты", pick any report (e.g. "Устройства" → "Акты"), click "Экспорт CSV".
   Expected: a native save dialog appears with a filename like `отчёт-device_acts-2026-08-27.csv` and a
   CSV filter. Pick a folder and save — expected: success toast "CSV-файл сохранён", and the file
   actually appears in the chosen folder (open in Finder/Explorer or `cat`/Excel — report data present,
   UTF-8, Cyrillic renders correctly).
3. Repeat step 2 but click "Cancel" in the dialog — expected: NO toast at all (neither success nor
   error), `csvExporting` returns to its initial state (button clickable again).
4. **LAN browser (server mode):** open the same screen via `http://<host>:<port>` in a regular browser
   (Chrome/Firefox/Safari), repeat the CSV export. Expected: the browser actually downloads the file
   (appears in the downloads list/folder), data is correct, success toast shows.
5. **PDF regression check:** on the same screen, click "Экспорт PDF" — the preview modal should open
   with correct report data, as before (this task did not touch the PDF path).
6. If anything above does not hold — describe exactly what you saw (which step, what happened instead
   of the expected outcome); this blocks completion.

### Resume signal

Reply "approved", or describe what you observed instead.

## Self-Check: PASSED

- FOUND: `ui/src/lib/utils/saveFile.ts`
- FOUND: `ui/src/features/reports/ReportsPage.svelte` (modified, contains `saveFile(`)
- FOUND: `crates/trackly-app/capabilities/main.json` (contains 5 new `*.csv` entries)
- FOUND commit `d2d6c287` in `git log --oneline`
- FOUND commit `a53ae284` in `git log --oneline`
