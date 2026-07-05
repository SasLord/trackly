---
phase: 16-documents-html-print
plan: 02
subsystem: pdf
tags: [act-service, html-render, minijinja, base64, data-uri, org-settings]

# Dependency graph
requires:
  - phase: 16-documents-html-print
    plan: 01
    provides: "html_templates::{resolve_templates_dir, materialize_defaults_on_startup, load_template, DEFAULT_HTML_TEMPLATES}, minijinja_env::build_safe_html_env, act_handover.html/act_acceptance.html files"
provides:
  - "ActService::render_pdf / render_acceptance_pdf return Result<String, AppError> (HTML), not Vec<u8>"
  - "OrganizationService::read_logo_bytes — reads legacy org.json logo file bytes + MIME for data: URI embedding"
  - "AppCtx::build materializes templates/act_handover.html + act_acceptance.html on every startup"
affects: [16-03-delivery-print, 16-04, 16-05-tests, 16-documents-html-print]

# Tech tracking
tech-stack:
  added:
    - "base64 0.22 (direct dep, already resolved transitively at 0.22.1 via axum/krilla/rcgen — zero new supply-chain surface)"
  patterns:
    - "Service-layer return-type change (Vec<u8> -> String) originates at exactly one choke point (ActService::render_pdf/render_acceptance_pdf), consumed unchanged by both Tauri and axum adapters per CLAUDE.md's dual-transport note"
    - "Logo as base64 data: URI built in Rust from trusted DB BLOB or canonicalized local file bytes — never a filesystem path reaching the template"
    - ".find(|(f, _)| *f == filename) lookup into DEFAULT_HTML_TEMPLATES instead of a hardcoded array index — robust against future reordering"

key-files:
  created: []
  modified:
    - crates/trackly-app/Cargo.toml
    - crates/trackly-app/src/services/act_service.rs
    - crates/trackly-app/src/services/organization_service.rs
    - crates/trackly-app/src/context.rs
    - crates/trackly-app/src/tauri_cmds/acts.rs
    - crates/trackly-app/src/tauri_cmds/templates.rs
    - crates/trackly-app/src/http/acts.rs

key-decisions:
  - "D-04/D-10/D-11 implemented: render_pdf/render_acceptance_pdf now return Result<String, AppError> (HTML); logo is a base64 data: URI from org_settings BLOB (render_pdf) or a new OrganizationService::read_logo_bytes helper reading the legacy org.json-referenced file (render_acceptance_pdf)"
  - "D-05 implemented: AppCtx::build calls html_templates::materialize_defaults_on_startup right after the existing (unchanged) templates.seed_defaults_on_startup() call"
  - "No new ActService field added for the templates dir — reused pipeline.organization.paths (OrganizationService already exposes pub paths: Arc<Paths>), avoiding a duplicate with_paths(...) builder the plan's <action> text flagged as conditional"
  - "PdfPipelineRefs slimmed to {organization, org_db} — templates/pdf fields became genuinely dead once both render functions stopped reading them; kept the (Some,Some,Some) triple-check in pdf_pipeline() as the 'PDF pipeline wired' guard invariant, just stopped threading the now-unused refs through the struct (avoids a clippy dead_code -D warnings failure)"
  - "Rule 3 (blocking-issue) fix, outside this plan's stated files_modified: tauri_cmds/acts.rs, tauri_cmds/templates.rs, and http/acts.rs all call the two rewired ActService methods directly, in the SAME crate — leaving them on Vec<u8> broke `cargo build -p trackly-app` itself (not just cargo test), which the plan's own acceptance criteria requires to exit 0. Applied the minimal type-plumbing fix (String return type + text/html content-type in the two axum handlers) to keep the crate compiling. Did NOT do Plan 16-03's full scope (srcdoc iframe UX, removing acts_open_pdf_in_system, frontend/ui rewiring) — that remains Plan 16-03's job."

requirements-completed: [SPEC-Req1, SPEC-Req2, SPEC-Req3, SPEC-Req6, SPEC-Req7]

# Metrics
duration: 30min
completed: 2026-07-05
---

# Phase 16 Plan 02: Act-Service HTML Wiring Summary

**Rewired `ActService::render_pdf`/`render_acceptance_pdf` from the krilla/DocSpec pipeline to the new HTML-template path (Plan 16-01's contracts), embedding logos as base64 `data:` URIs and materializing `templates/*.html` defaults on every startup — plus the minimal adapter-layer type fix required to keep `trackly-app` compiling.**

## Performance

- **Duration:** ~30 min
- **Tasks:** 3 completed (+ 1 unplanned Rule-3 fix folded into Task 1's commit)
- **Files modified:** 7

## Accomplishments

- `ActService::render_pdf` now reads `templates/act_handover.html` via `html_templates::load_template` (file-first, embedded-default fallback via `.find()` lookup on `DEFAULT_HTML_TEMPLATES`, not a hardcoded array index), renders through `build_safe_html_env()` (autoescape ON), and returns the HTML string directly — the DocSpec parse/patch/`render_docspec` tail is fully removed.
- `ActService::render_acceptance_pdf` follows the identical shape for `templates/act_acceptance.html`. Since this path uses the legacy `org.json` (no BLOB logo storage), added `OrganizationService::read_logo_bytes` — reuses the existing path-traversal-guarded `safe_logo_canonical`, reads the canonical file via `tokio::fs::read`, infers MIME from the extension (`.png`/`.jpg`/`.jpeg`, default `image/png`).
- Logo embedding in both paths: `format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes))` — `base64` pinned as a direct `trackly-app` dependency (0.22, already resolved transitively at 0.22.1, zero new supply-chain surface, confirmed via `cargo tree -i base64@0.22.1`).
- `ctx` JSON's `org.logo_path` key replaced with `org.logo_data_uri` in both render paths; all other context fields unchanged (D-04 — same `ctx` shape reused).
- `AppCtx::build` now calls `html_templates::materialize_defaults_on_startup(&html_templates_dir)` immediately after the existing `templates.seed_defaults_on_startup().await?` call — additive, idempotent, does not touch the frozen DB-template seed path.
- **Rule 3 fix (deviation):** `tauri_cmds/acts.rs`, `tauri_cmds/templates.rs`, and `http/acts.rs` all call `ActService::render_pdf`/`render_acceptance_pdf` directly within the same `trackly-app` crate — the type change broke `cargo build -p trackly-app` itself, not just `cargo test`. Applied the minimal fix: `build_acts_render_pdf`/`build_devices_render_acceptance_pdf`/`build_templates_render_preview` and their Tauri command wrappers now return `Result<String, AppError>`; the two axum handlers (`handler_render_pdf`/`handler_render_acceptance_pdf`) now respond with `text/html; charset=utf-8` instead of `application/pdf`. Did not touch `acts_open_pdf_in_system` removal, frontend `PdfPreviewModal.svelte`/`acts.ts`/`pdf.ts`, or any other UX rework explicitly scoped to Plan 16-03.

## Task Commits

Each task was committed atomically:

1. **Task 1 (render_pdf HTML rewiring) + Task 2 (render_acceptance_pdf HTML rewiring) + Rule-3 adapter fix** - `4739e4e` (feat) — combined because both service-method rewrites touch the same file (`act_service.rs`) and the Rule-3 adapter fix is a direct, inseparable consequence of the type change introduced by both tasks
2. **Task 3 (materialize templates/ on AppCtx startup)** - `18a825f` (feat)

## Files Created/Modified

- `crates/trackly-app/Cargo.toml` - Added `base64 = "0.22"` as a direct dependency
- `crates/trackly-app/src/services/act_service.rs` - `render_pdf`/`render_acceptance_pdf` rewritten to the HTML path; `PdfPipelineRefs` slimmed to `{organization, org_db}` (dead-code fix)
- `crates/trackly-app/src/services/organization_service.rs` - Added `read_logo_bytes` helper
- `crates/trackly-app/src/context.rs` - `AppCtx::build` materializes HTML template defaults on startup
- `crates/trackly-app/src/tauri_cmds/acts.rs` - `build_acts_render_pdf`/`build_devices_render_acceptance_pdf` + their `#[tauri::command]` wrappers now return `String`
- `crates/trackly-app/src/tauri_cmds/templates.rs` - `build_templates_render_preview` + wrapper now return `String`
- `crates/trackly-app/src/http/acts.rs` - `handler_render_pdf`/`handler_render_acceptance_pdf` now respond with `text/html; charset=utf-8`

## Decisions Made

- Reused `pipeline.organization.paths` (an already-public `Arc<Paths>` field on `OrganizationService`) to resolve the templates directory instead of adding a new `paths` field + `with_paths(...)` builder to `ActService` — the plan's `<action>` text explicitly allowed this shortcut ("if `ActService` already stores an `Arc<Paths>` field... otherwise add one") after confirming no such field existed; `OrganizationService` already had one, avoiding an unnecessary struct/builder change.
- Kept `pdf_pipeline()`'s `(Some, Some, Some)` triple existence-check unchanged (guards "PDF pipeline is wired" — same invariant existing tests rely on), but removed `templates`/`pdf` from the returned `PdfPipelineRefs` struct since neither render function reads them anymore post-rewrite; this was required to satisfy `clippy -D warnings` (dead_code lint).
- Updated two stale doc comments (the old "3-stage pipeline: ... krilla render → Vec<u8>" and "→ PDF bytes" comments) to describe the new HTML pipeline accurately — Rule 1 (misleading documentation left in place would be a correctness bug for future readers).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking issue] Tauri/HTTP adapter call sites broke `cargo build -p trackly-app`**
- **Found during:** Task 1 (immediately after the `render_pdf` signature change)
- **Issue:** `tauri_cmds/acts.rs`, `tauri_cmds/templates.rs`, and `http/acts.rs` all call `ctx.acts.render_pdf`/`render_acceptance_pdf` directly in the same crate. Changing the return type broke the lib crate's own compilation, not just downstream test binaries — this violates the plan's own stated acceptance criteria (`cargo build -p trackly-app` exits 0 after every task).
- **Fix:** Updated the two `build_*` helper functions, their four `#[tauri::command]` wrappers, and the two axum handlers to the new `Result<String, AppError>` signature; switched the HTTP responses' content-type to `text/html; charset=utf-8`. Did NOT perform the full Plan 16-03 scope (removing `acts_open_pdf_in_system`, frontend `PdfPreviewModal.svelte`/`acts.ts`/`pdf.ts` rewiring) — those remain explicitly out of scope for this plan and are Plan 16-03's responsibility.
- **Files modified:** `crates/trackly-app/src/tauri_cmds/acts.rs`, `crates/trackly-app/src/tauri_cmds/templates.rs`, `crates/trackly-app/src/http/acts.rs`
- **Commit:** `4739e4e`

**2. [Rule 1 - Bug/dead-code] `PdfPipelineRefs.templates`/`.pdf` fields became dead code**
- **Found during:** Task 1/2 (post-rewrite `cargo build` warning)
- **Issue:** After both render functions stopped reading `pipeline.templates`/`pipeline.pdf`, the struct fields carrying them became unused, which `clippy -D warnings` (part of this plan's `<verification>`) would reject.
- **Fix:** Removed `templates`/`pdf` from `PdfPipelineRefs`; kept the `(Some, Some, Some)` existence check in `pdf_pipeline()` unchanged (still gates on all three being wired).
- **Files modified:** `crates/trackly-app/src/services/act_service.rs`
- **Commit:** `4739e4e`

## Issues Encountered

None beyond the two auto-fixes above. `cargo fmt` reformatted two multi-line chained calls in `act_service.rs`/`organization_service.rs` on first `--check` failure — routine formatting, not a deviation.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- Plan 16-03 (Tauri/HTTP delivery adapters + frontend) can now build directly on the `String`-returning `ActService` methods and the already-adjusted Tauri/HTTP signatures/content-type from this plan's Rule-3 fix; remaining Plan 16-03 scope is the frontend rework (`PdfPreviewModal.svelte` srcdoc + print(), `acts.ts`/`pdf.ts` typing, removing `acts_open_pdf_in_system` and its now-unused `tauri_plugin_shell::ShellExt` import if nothing else needs it).
- Plan 16-05 (tests) can now write assertions against real HTML strings returned by `render_pdf`/`render_acceptance_pdf` — no adapter changes needed on the Rust side beyond what Plan 16-03 does on the UI.
- `cargo test -p trackly-app` will NOT be fully green yet — existing full-pipeline tests (`pdf_render_act.rs`, `pdf_column_overflow.rs`, `pdf_logo.rs`, `acts_e2e_smoke.rs`, `specta_roundtrip.rs`) still assume `Vec<u8>` from `ActService::render_pdf`/`render_acceptance_pdf` and will fail to compile until Plan 16-05 lands. This is expected per the plan's own `<verification>` note, not a regression introduced here.

---
*Phase: 16-documents-html-print*
*Completed: 2026-07-05*

## Self-Check: PASSED

All 7 created/modified source files and the SUMMARY.md itself verified present on disk. All 3 commit hashes (4739e4e, 18a825f, fe2dda3) verified present in git log.
