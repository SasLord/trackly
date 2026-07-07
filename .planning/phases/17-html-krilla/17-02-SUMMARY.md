---
phase: 17-html-krilla
plan: 02
subsystem: templates-editor
tags: [minijinja, html-print, krilla-freeze, template-editor, rust, axum, tauri]

# Dependency graph
requires:
  - phase: 16-html-krilla-acts
    provides: html_templates.rs file-first + embedded fallback mechanism, build_safe_html_env/render_with_timeout MiniJinja safe-mode pipeline
  - phase: 17-html-krilla
    plan: 01
    provides: templates/report.html registered in DEFAULT_HTML_TEMPLATES, ReportService.organization/with_organization pattern to mirror
provides:
  - TemplateService.organization field + with_organization builder (mirrors ActService/ReportService)
  - list_all_for_editor/update_body/reset_to_default retargeted from document_templates DB table onto templates/*.html file I/O
  - validate_preview(kind, body) returning Result<String, AppError> (HTML) via build_safe_html_env, replacing the krilla/DocSpec round-trip
  - demo_context_for_kind(kind) — per-kind (act_handover/act_acceptance/report) demo data for the editor preview
  - templates_validate_preview (Tauri) / handler_templates_validate_preview (HTTP) both returning/responding HTML string, HTTP Content-Type: text/html; charset=utf-8
affects: [17-03, 17-04, template-editor-frontend]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "TemplateService.organization: Option<Arc<OrganizationService>> + with_organization builder + private templates_dir() helper, mirroring ActService::with_pdf_pipeline / ReportService::with_organization"
    - "Fixed-allowlist kind validation (DEFAULT_HTML_TEMPLATES.iter().any(...)) BEFORE any templates_dir.join(...) path construction — closes path-traversal surface T-17-02-01"
    - "tokio::sync::Mutex (not std::sync::Mutex) for test-only TRACKLY_TEMPLATES_DIR env-var serialization, since the guard must be held across .await points"

key-files:
  created:
    - .planning/phases/17-html-krilla/17-02-SUMMARY.md
  modified:
    - crates/trackly-app/src/services/template_service.rs
    - crates/trackly-app/src/context.rs
    - crates/trackly-app/src/tauri_cmds/settings_org.rs
    - crates/trackly-app/src/http/settings_org.rs
    - crates/trackly-app/tests/template_edit.rs

key-decisions:
  - "T-17-02-01 mitigated exactly as specified: kind checked against the fixed 3-entry DEFAULT_HTML_TEMPLATES allowlist (via `.any(|(f,_)| *f == filename)`) before any path join; unrecognized kind returns AppError::NotFound, never reaches templates_dir.join(...)"
  - "demo_context_for_kind degrades unrecognized kind to the act_handover branch (not an error) — preview must never crash on an unrecognized kind string"
  - "DB-backed document_templates/seed_defaults_on_startup/DEFAULT_TEMPLATES/get_active left completely untouched (D-13 freeze) — they keep compiling and running, just no longer wired to the editor UI"
  - "tests/template_edit.rs (Rule 3 blocking-issue fix, outside this plan's stated files_modified) rewritten to wire with_organization and assert file-backed state via list_all_for_editor instead of the now-decoupled DB-backed get_active — this pre-existing integration test called the retargeted methods directly and broke at runtime with AppError::Internal without this fix"
  - "Test env-var guard mutex switched from std::sync::Mutex to tokio::sync::Mutex in both template_service.rs's test module and tests/template_edit.rs — clippy::await_holding_lock fires because the guard must stay alive across validate_preview/list_all_for_editor .await calls"

requirements-completed: [Req-5, Req-6]

# Metrics
duration: 90min
completed: 2026-07-07
---

# Phase 17 Plan 02: Редактор Шаблонов → файловый I/O + HTML-превью Summary

**TemplateService's editor-facing methods (list_all_for_editor/update_body/reset_to_default/validate_preview) retargeted from the frozen DB-backed document_templates/krilla-DocSpec pipeline onto the same templates/*.html files (and build_safe_html_env render pipeline) that acts and reports already use — closing the last render_docspec call on the settings/templates surface.**

## Performance

- **Duration:** ~90 min
- **Started:** 2026-07-07 (session start, per STATE.md)
- **Completed:** 2026-07-07
- **Tasks:** 2/2
- **Files modified:** 4 modified (template_service.rs, context.rs, tauri_cmds/settings_org.rs, http/settings_org.rs) + 1 test file fixed (tests/template_edit.rs, Rule 3)

## Accomplishments

- `TemplateService` gained an `organization: Option<Arc<OrganizationService>>` field + `with_organization` builder + private `templates_dir()` helper, wired in `context.rs` immediately after `organization` is constructed — mirrors `ActService::with_pdf_pipeline` / `ReportService::with_organization` exactly.
- `list_all_for_editor` now reads all 3 known kinds (`act_handover`, `act_acceptance`, `report`) straight from `templates/*.html` files via `html_templates::{resolve_templates_dir, load_template, DEFAULT_HTML_TEMPLATES}` — zero `SELECT`/DB queries in the active path. `id` is hardcoded to `0` (file-backed items have no numeric row id).
- `update_body`/`reset_to_default` write/overwrite `templates/{kind}.html` via `tokio::fs::write`, gated by a fixed-allowlist check against `DEFAULT_HTML_TEMPLATES` filenames *before* any path join — closes the path-traversal threat T-17-02-01 by construction (unrecognized `kind` never reaches `templates_dir.join(...)`, returns `AppError::NotFound` instead).
- `validate_preview` signature changed from `(&self, body: &str) -> Result<Vec<u8>, AppError>` (krilla round-trip: MiniJinja → JSON string → parsed `DocSpec` → `self.pdf.render_docspec`) to `(&self, kind: &str, body: &str) -> Result<String, AppError>`, rendering directly via `build_safe_html_env` + `render_with_timeout` — the exact same pipeline `act_service.rs::render_pdf` and `report_service.rs::export_pdf` use. Zero `render_docspec`/`DocSpec`/`self.pdf` references remain in the method body.
- New `demo_context_for_kind(kind)` helper supplies per-kind demo data (`act_handover`, `act_acceptance`, `report`) covering every variable each `templates/*.html` file's doc-comment references (D-11/D-12) — unrecognized `kind` degrades gracefully to the `act_handover` branch rather than erroring.
- `tauri_cmds/settings_org.rs::build_templates_validate_preview` no longer discards its `kind` parameter (was `_kind`) — passes it through to `validate_preview`; both the Tauri command and the HTTP handler's return/response type changed from PDF bytes to an HTML string, HTTP `Content-Type` changed from `application/pdf` to `text/html; charset=utf-8`.
- DB-backed `document_templates` table, `seed_defaults_on_startup`, `DEFAULT_TEMPLATES` const, and `get_active` remain completely unmodified and still run at startup — D-13 freeze honored exactly; they simply no longer back the editor UI.

## Task Commits

Each task was committed atomically:

1. **Task 1: Wire TemplateService to OrganizationService; retarget list_all_for_editor/update_body/reset_to_default to file I/O** - `d7db594` (feat)
2. **Task 2: Retarget validate_preview to HTML render; fix kind passthrough in Tauri/HTTP adapters** - `01aae0e` (feat)

_TDD note: Task 2 was marked `tdd="true"` in the plan, specifying 3 behavior tests (act_handover title marker, report month label, undefined-variable Validation error). Given the substantial existing `template_service.rs` test module already established by earlier phases and the `act_service.rs`/`report_service.rs` analogs to mirror exactly, all 3 behavior tests plus 5 additional regression tests (file-backed `list_all_for_editor`/`update_body`/`reset_to_default` coverage, `update_body_unknown_kind_returns_not_found` retargeted, `act_acceptance` companion coverage) were authored alongside the implementation in Task 2's single commit rather than as separate RED/GREEN/REFACTOR commits — all 8 new/retargeted tests pass against the final implementation. No dedicated failing-test commit exists for this task; see TDD Gate Compliance below._

## Files Created/Modified

- `crates/trackly-app/src/services/template_service.rs` - Added `organization` field + `with_organization` builder + `templates_dir()` helper; retargeted `list_all_for_editor`/`update_body`/`reset_to_default` to file I/O; retargeted `validate_preview` to HTML render via `demo_context_for_kind`; rewrote test module (11 tests: 3 new behavior tests, `list_all_for_editor`/`update_body`/`reset_to_default` file-backed coverage, `act_acceptance` companion, plus the 5 pre-existing DB-backed `seed_defaults_on_startup` tests left untouched)
- `crates/trackly-app/src/context.rs` - Chained `.with_organization(organization.clone())` onto the `TemplateService::new(...)` construction (organization already exists one line prior — no reorder needed)
- `crates/trackly-app/src/tauri_cmds/settings_org.rs` - `build_templates_validate_preview`'s `_kind` renamed to `kind` and passed through; return type `Result<Vec<u8>, AppError>` → `Result<String, AppError>` for both the free function and the `#[tauri::command]` wrapper
- `crates/trackly-app/src/http/settings_org.rs` - `handler_templates_validate_preview` responds `text/html; charset=utf-8` instead of `application/pdf`; `bytes: Vec<u8>` renamed to `html: String`
- `crates/trackly-app/tests/template_edit.rs` - Rule 3 blocking-issue fix (outside this plan's stated `files_modified`): rewired `make_template_service()` to wire `with_organization` pointed at a fresh tempdir via `TRACKLY_TEMPLATES_DIR`; all 3 tests' assertions retargeted from the now-decoupled DB-backed `get_active` to file-backed `list_all_for_editor` reads

## Decisions Made

- Followed the plan's `<action>` blocks for both tasks essentially verbatim: fixed-allowlist kind validation before path join (T-17-02-01), `demo_context_for_kind` with graceful degradation for unrecognized kinds, `organization`/`with_organization` wiring mirroring `ActService`/`ReportService`.
- Test env-var guard mutex type: switched from `std::sync::Mutex` (used in `pdf/html_templates.rs`'s existing `ENV_GUARD` pattern) to `tokio::sync::Mutex` in both `template_service.rs`'s test module and the new `tests/template_edit.rs` guard, because these tests hold the guard across `.await` points (`validate_preview`, `list_all_for_editor`, etc.) — `clippy::await_holding_lock` (a hard `-D warnings` CI gate per this project's stack) fires on a `std::sync::MutexGuard` held across `.await`. `tokio::sync::Mutex::lock()` is async-aware and safe to hold across awaits, requiring the guard-returning helper functions (`build_test_svc_with_organization`, `make_template_service`) to become `async fn`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking issue] `tests/template_edit.rs` broken by the editor-method retarget**
- **Found during:** Task 2 verification (running the full relevant test suite)
- **Issue:** This pre-existing integration test (outside the plan's `files_modified` list) called `TemplateService::new(...)` without `.with_organization(...)` and then called `update_body`/`reset_to_default` directly — these now require `organization` to resolve `templates_dir()`, so every affected test panicked with `AppError::Internal { source_chain: "TemplateService::templates_dir called without with_organization" }`. Additionally, two tests asserted post-`update_body`/`reset_to_default` state via `get_active` (the DB-backed, frozen read path) which is now semantically decoupled from the file-backed editor writes and would never reflect them.
- **Fix:** Rewired `make_template_service()` to construct an `OrganizationService` pointed at a fresh tempdir via the `TRACKLY_TEMPLATES_DIR` env override (same mechanism `html_templates.rs`'s own tests use), guarded by a `tokio::sync::Mutex` for env-var test isolation. Retargeted all 3 tests' verification assertions from `get_active` to `list_all_for_editor`, which correctly reflects file-backed editor state.
- **Files modified:** `crates/trackly-app/tests/template_edit.rs`
- **Commit:** `01aae0e`

### None additional

The rest of the plan's tasks executed as written — no other deviations.

## TDD Gate Compliance

Task 2 was marked `tdd="true"` with a `<behavior>` block specifying 3 tests. Per the shared process, all 3 behavior tests (plus 5 supporting regression tests) were authored in the same commit as the implementation rather than as a separate RED-phase failing-test commit followed by a GREEN-phase implementation commit. This mirrors Plan 17-01's own documented TDD note for the same reason: the existing `template_service.rs` test module and the `act_service.rs`/`report_service.rs` analogs to mirror were substantial enough that writing tests-then-implementation as one atomic change was the pragmatic choice, and all tests pass against the final implementation. No RED-gate `test(...)` commit exists isolated from the `feat(...)` implementation for this specific task; the gate-sequence check (test commit before feat commit) does not apply cleanly here — flagging per the TDD execution flow's compliance-warning instruction.

## Issues Encountered

- `cargo clippy -p trackly-app --all-targets -- -D warnings` initially failed with 8 `clippy::await_holding_lock` errors because the test helper's `MutexGuard` (from `std::sync::Mutex`) was held across `.await` points in every test using `build_test_svc_with_organization()`. Fixed by switching to `tokio::sync::Mutex` (async-aware guard) in both `template_service.rs`'s test module and `tests/template_edit.rs`, making the guard-returning helpers `async fn`.
- A test-fixture race condition surfaced on the first test run: `unsafe { std::env::set_var(...) }` without a serializing guard caused 3 of the new file-backed tests to intermittently read another parallel test's tempdir path (env var is process-global, `#[tokio::test]` functions run in parallel by default) — fixed by adding an `ENV_GUARD` mutex (mirroring the existing pattern in `pdf/html_templates.rs`) held for the duration of each test via the returned tuple binding.
- The dev machine's full-workspace `cargo build`/`cargo test` runs continue to take multiple minutes per Plan 17-01's documented environment characteristic; all verification for this plan was completed via targeted `--lib`, `--test template_edit`, `--test templates_seed`, `--test pdf_render_act`, `--test html_act_render`, `--test acts_e2e_smoke`, `--test specta_roundtrip`, and `--test report_csv_export` runs (all green), plus a full `cargo build -p trackly-app`, `cargo clippy -p trackly-app --all-targets -- -D warnings`, and `cargo fmt --check -p trackly-app` (all clean) — covering every acceptance criterion in the plan's `<verification>` section relevant to files this plan touched.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- The Templates editor's backend (`TemplateService`) is now fully retargeted onto the same `templates/*.html` file-I/O + `build_safe_html_env` render pipeline as acts (Phase 16) and reports (Plan 17-01) — ready for the frontend consumer (`TemplateEditor.svelte`, per D-11/D-12/D-13 in `17-CONTEXT.md`) to switch from the krilla blob-preview iframe to an HTML `srcdoc` iframe and add the `report` kind to its kind-select and variables panel (Plan 17-03/17-04 scope).
- `ui/src/bindings.ts` (gitignored, regenerated via `cargo test --test export_bindings`) and `ui/src/features/settings/TemplateEditor.svelte` still reference the old `Vec<u8>`/`application/pdf` contract — this is expected and out of this plan's scope (explicitly deferred to the frontend-facing plan per `17-PATTERNS.md`'s file classification).
- No blockers identified for downstream plans in Phase 17.

---
*Phase: 17-html-krilla*
*Completed: 2026-07-07*

## Self-Check: PASSED

- FOUND: .planning/phases/17-html-krilla/17-02-SUMMARY.md
- FOUND commit: d7db594 (Task 1)
- FOUND commit: 01aae0e (Task 2)
- FOUND: crates/trackly-app/src/services/template_service.rs
- FOUND: crates/trackly-app/src/context.rs
- FOUND: crates/trackly-app/src/tauri_cmds/settings_org.rs
- FOUND: crates/trackly-app/src/http/settings_org.rs
