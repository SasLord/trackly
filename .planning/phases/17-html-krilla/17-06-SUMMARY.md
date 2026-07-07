---
phase: 17-html-krilla
plan: 06
subsystem: templates
tags: [rust, minijinja, svelte, security, iframe-sandbox, validation]

# Dependency graph
requires:
  - phase: 17-html-krilla (plan 02)
    provides: TemplateService.validate_preview (build_safe_html_env + demo_context_for_kind pipeline), file-backed update_body
  - phase: 17-html-krilla (plan 03)
    provides: PdfPreviewModal.svelte srcdoc preview, TemplateEditor.svelte file-backed HTML editor
provides:
  - update_body validates via validate_preview (strict undefined + autoescape), same render path as real document — not a lenient bare minijinja::Environment::new()
  - Bare deny-all sandbox on both preview iframes (PdfPreviewModal.svelte, TemplateEditor.svelte)
affects: [17-VERIFICATION, 17-07]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Save-time template validation reuses the exact render pipeline used at print time (validate_preview) so «Сохранить» cannot silently accept a template that will fail on real render"
    - "Deny-all iframe sandbox (sandbox=\"\") for srcdoc content built from user-editable template bodies"

key-files:
  created: []
  modified:
    - crates/trackly-app/src/services/template_service.rs
    - crates/trackly-app/tests/template_edit.rs
    - ui/src/features/acts/PdfPreviewModal.svelte
    - ui/src/features/settings/TemplateEditor.svelte

key-decisions:
  - "update_body reorders allowlist check BEFORE render validation — unrecognized kind still returns NotFound and never triggers a render (T-17-02-01 preserved)"
  - "validate_preview's error field ('template') remapped to 'body' inside update_body — preserves the existing UI/test contract for AppError::Validation.field"
  - "sandbox implemented as sandbox=\"\" not a bare boolean attribute — svelte-check's lib.dom.d.ts types HTMLIFrameElement.sandbox as string; bare form fails type-checking. Functionally identical deny-all (no allow-* tokens)"
  - "update_body_writes_file_to_disk fixture corrected act_number -> act.number to match the real nested demo context; the old fixture only passed under the removed lenient validator"

patterns-established:
  - "When closing a validation gap, point the write path at the same function the read/render path already uses rather than duplicating a second (drifting) validation environment"

requirements-completed: [Req-4, Req-5]

metrics:
  duration: ~15 min
  completed: 2026-07-07
  tasks: 3
  files: 4
---

# Phase 17 Plan 06: WR-01 template-save validation + WR-03 iframe sandbox Summary

Closed two WARNING defects from 17-VERIFICATION.md/17-REVIEW.md: `update_body` now validates saved templates through the same strict `build_safe_html_env` + demo-context render pipeline used at real print time (WR-01), and both preview iframes render user-editable template HTML in a deny-all sandbox (WR-03).

## What Was Built

### Task 1 — WR-01: strict save-time validation (commit `7aa3268`)
`TemplateService::update_body` previously validated MiniJinja syntax through a separate lenient `minijinja::Environment::new()` (no autoescape, no `UndefinedBehavior::Strict`) that caught only parse errors. A template referencing an undeclared top-level variable saved successfully and only failed at real render/print. It now:
- Moves the `DEFAULT_HTML_TEMPLATES` allowlist check ahead of validation (unrecognized `kind` still short-circuits to `NotFound`, never rendering — T-17-02-01 preserved).
- Calls `self.validate_preview(kind, &body)`, reusing the exact render path (`build_safe_html_env` + `demo_context_for_kind`) the real document uses.
- Remaps `AppError::Validation.field` from `"template"` (what `validate_preview` emits) to `"body"` to keep the existing UI/test contract, passing other `AppError` variants through unchanged.

Test changes: fixed the `update_body_writes_file_to_disk` fixture (`{{ act_number }}` → `{{ act.number }}`, matching the real nested demo context) and added `update_body_rejects_undefined_top_level_variable` asserting the new rejection path and the file-unchanged invariant.

### Task 2 — WR-03: sandboxed preview iframes (commit `a8d57e3`)
Added `sandbox=""` (deny-all: no-scripts, no-same-origin, no-forms, no-popups, no-top-navigation) to the `srcdoc` iframe in `PdfPreviewModal.svelte` (line 288) and `TemplateEditor.svelte` (line 267). A `ManageSettings`-gated admin can no longer have an embedded `<script>` in a template body execute in the app origin. Print logic works off the `htmlContent` string directly and never reaches into the iframe DOM/`contentWindow`, so sandbox does not affect printing.

### Task 3 — Human-verify checkpoint (approved)
User verified all four previews (Отчёты «Экспорт PDF», Акты print, Настройки → Шаблоны «Проверить») and confirmed no visual regression — "Всё замечательно".

## Verification

- `cargo test -p trackly-app --test template_edit` — 6/6 passed (incl. new undefined-variable test)
- `cargo test -p trackly-app --lib template_service` — 11/11 passed
- `pnpm --dir ui build` — succeeded
- `pnpm --dir ui run svelte-check` — 0 errors; both target files clean
- `grep -c "minijinja::Environment::new()" ...template_service.rs` — 1 hit, but it is prose in a doc-comment explaining the removed path, not a code call site (bare-env validation fully removed from `update_body`)
- `grep -n "iframe sandbox" ...` — found in both `PdfPreviewModal.svelte` and `TemplateEditor.svelte`

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] sandbox typed as string, not boolean**
- **Found during:** Task 2 (svelte-check verification)
- **Issue:** The plan specified a bare `sandbox` boolean attribute. `svelte-check`'s `lib.dom.d.ts` types `HTMLIFrameElement.sandbox` as `string`, so bare `sandbox` produced `Type 'boolean' is not assignable to type 'string'` errors in both files.
- **Fix:** Used `sandbox=""` — functionally identical deny-all sandbox (no `allow-*` tokens), type-checks clean.
- **Files modified:** `ui/src/features/acts/PdfPreviewModal.svelte`, `ui/src/features/settings/TemplateEditor.svelte`
- **Commit:** `a8d57e3`

## Out of Scope (deferred)

- Reports UI cleanup: two redundant buttons «Экспорт PDF» + «Печать» in `ReportsPage.svelte`. Explicitly out of scope for 17-06; deliberately untouched, to be handled separately by the orchestrator.

## Self-Check: PASSED

- `crates/trackly-app/src/services/template_service.rs` — FOUND (modified, committed `7aa3268`)
- `crates/trackly-app/tests/template_edit.rs` — FOUND (modified, committed `7aa3268`)
- `ui/src/features/acts/PdfPreviewModal.svelte` — FOUND (modified, committed `a8d57e3`)
- `ui/src/features/settings/TemplateEditor.svelte` — FOUND (modified, committed `a8d57e3`)
- Commit `7aa3268` — FOUND
- Commit `a8d57e3` — FOUND
