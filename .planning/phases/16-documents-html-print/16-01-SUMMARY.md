---
phase: 16-documents-html-print
plan: 01
subsystem: pdf
tags: [minijinja, html, print-css, templates, self-contained]

# Dependency graph
requires:
  - phase: 15-render-word-fidelity
    provides: act_handover.minijinja / act_acceptance.minijinja frozen Word-sample layout to port to HTML
provides:
  - "Paths::templates_dir() accessor (<exe_dir>/templates)"
  - "html_templates::{resolve_templates_dir, materialize_defaults_on_startup, load_template, DEFAULT_HTML_TEMPLATES}"
  - "minijinja_env::build_safe_html_env() — autoescape ON MiniJinja environment"
  - "act_handover.html / act_acceptance.html self-contained HTML template files"
affects: [16-02-act-service-html-wiring, 16-03-delivery-print, 16-documents-html-print]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "File-backed template materialize-on-startup (insert-only, no auto-upgrade-in-place) — distinct from DB-backed template_service's auto-upgrade branch"
    - "Read-on-render with graceful embedded-default fallback (no panics, no Result propagation on missing file)"
    - "Dev/test env-var path override mirroring TRACKLY_AD_MOCK/TRACKLY_SNMP_MOCK precedent (TRACKLY_TEMPLATES_DIR)"
    - "Sibling MiniJinja environment constructor pattern (build_safe_env / build_safe_html_env) differing only in AutoEscape mode"

key-files:
  created:
    - crates/trackly-app/templates/act_handover.html
    - crates/trackly-app/templates/act_acceptance.html
    - crates/trackly-app/src/pdf/html_templates.rs
  modified:
    - crates/trackly-app/src/pdf/minijinja_env.rs
    - crates/trackly-app/src/pdf/mod.rs
    - crates/trackly-infra/src/paths.rs

key-decisions:
  - "D-01/D-02: MiniJinja reused as the HTML engine (zero new dependency); runtime engine chosen over compile-time askama/maud so template edits apply without rebuild"
  - "D-03: templates are plain author-editable HTML+inline-<style> files, same {{ }}/{% %} syntax as existing .minijinja files"
  - "D-05/D-06/D-07/D-08: materialize-on-startup (insert-only), fallback-on-missing (never panics), env-override resolver (TRACKLY_TEMPLATES_DIR), read-on-render (no file watcher)"
  - "D-11 (partial, contract only): logo referenced as org.logo_data_uri (data: URI) instead of a filesystem path — actual data: URI construction is Plan 16-02's scope"
  - "D-12: print CSS @page A4 portrait + page-break-inside: avoid on device blocks, authored directly into both .html files"
  - "T-16-01 mitigation: build_safe_html_env sets AutoEscape::Html unconditionally; no | safe filter used anywhere in the new templates"

requirements-completed: [SPEC-Req1, SPEC-Req2, SPEC-Req4, SPEC-Req6, SPEC-Req7]

# Metrics
duration: 25min
completed: 2026-07-05
---

# Phase 16 Plan 01: HTML Template Contracts Summary

**Ported both act templates to self-contained HTML with inline print CSS, and built the file-resolver/materialize/fallback loader plus an autoescape-ON MiniJinja environment — the contracts Plan 16-02's act_service wiring will consume.**

## Performance

- **Duration:** ~25 min
- **Tasks:** 3 completed
- **Files modified:** 6 (3 created, 3 modified)

## Accomplishments
- `Paths::templates_dir()` accessor added, mirroring `logs_dir()`'s shape (`<exe_dir>/templates`), with zero env-var logic inside `Paths` itself.
- `html_templates.rs` module: `resolve_templates_dir` (env-override-first), `materialize_defaults_on_startup` (idempotent insert-only), `load_template` (read-on-render with embedded-default fallback), `DEFAULT_HTML_TEMPLATES` constant — all covered by 5 passing unit tests.
- `act_handover.html` / `act_acceptance.html` created as self-contained HTML5 documents (inline `<style>`, print CSS `@page A4 portrait` + `page-break-inside: avoid`), reproducing every block from their `.minijinja` analogs (header+requisites, centered title, field_row-style device fields including all 6 field labels, "Сроком до", two-line "Выдал/Получил" signatures).
- `build_safe_html_env()` added to `minijinja_env.rs` as a sibling to `build_safe_env()` — identical safe-mode invariants (`UndefinedBehavior::Strict`, `set_recursion_limit(64)`, `set_fuel(Some(100_000))`, no loader), only `AutoEscape::Html` differs.

## Task Commits

Each task was committed atomically:

1. **Task 1: Add Paths::templates_dir() accessor** - `5360818` (feat)
2. **Task 3: Port act_handover.html / act_acceptance.html + add build_safe_html_env** - `3dad322` (feat) — executed before Task 2 per plan's explicit sequencing note, to avoid a temporarily-broken compile (`include_str!` in Task 2 depends on these files existing)
3. **Task 2: html_templates.rs — resolver, materialize-on-startup, read-on-render loader** - `ed9650e` (feat)

_No TDD gate applies — `tdd="true"` task 2 landed test+implementation in a single commit per plan's `<behavior>`/`<action>` structure (tests were written alongside the implementation in one file, not as a separate RED-phase commit); all 5 tests pass._

## Files Created/Modified
- `crates/trackly-infra/src/paths.rs` - Added `templates_dir: PathBuf` field + `templates_dir()` accessor
- `crates/trackly-app/templates/act_handover.html` - Self-contained HTML template for the handover act
- `crates/trackly-app/templates/act_acceptance.html` - Self-contained HTML template for the acceptance document
- `crates/trackly-app/src/pdf/html_templates.rs` - Templates-dir resolver, materialize-on-startup, read-on-render loader, 5 unit tests
- `crates/trackly-app/src/pdf/minijinja_env.rs` - Added `build_safe_html_env()` (autoescape ON)
- `crates/trackly-app/src/pdf/mod.rs` - Registered `pub mod html_templates;`

## Decisions Made
- Executed Task 3 before Task 2 (plan explicitly permits either order "since both land in the same commit boundary") to keep every intermediate commit green (no broken `include_str!` reference).
- Env-var test isolation for `TRACKLY_TEMPLATES_DIR` uses a `static Mutex<()>` guard around each test that touches the env var (no existing precedent for env-var-isolated tests in this codebase to mirror; `unsafe fn` wrappers used per Rust 2024's `set_var`/`remove_var` signature, consistent with `webview_env.rs`'s existing `unsafe` usage pattern).
- Kept `DEFAULT_HTML_TEMPLATES` as `&[(&str, &str)]` (filename, body) — no third "display name" element like `template_service::DEFAULT_TEMPLATES`, since file-backed templates have no DB `name` column to populate.

## Deviations from Plan

None - plan executed exactly as written. Task ordering (3 before 2) was explicitly pre-authorized by the plan's own `<action>` text for Task 2.

## Issues Encountered

None. `cargo fmt` auto-reformatted 3 long lines in `html_templates.rs` on first `cargo fmt --check` failure (`--check` reported diffs, then `cargo fmt` applied them) — this is routine formatting cleanup, not a deviation requiring a separate task.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Plan 16-02 can now wire `act_service.rs::render_pdf`/`render_acceptance_pdf` against `html_templates::{resolve_templates_dir, materialize_defaults_on_startup, load_template}` and `minijinja_env::build_safe_html_env` directly.
- No wiring into `act_service.rs` was done in this plan (explicitly out of scope, confirmed by plan's own success criteria) — `render_pdf`/`render_acceptance_pdf` still call the krilla/DocSpec pipeline unchanged.
- The `org.logo_data_uri` context key is referenced in both new `.html` templates but the actual base64 `data:` URI construction (reading `logo_bytes`/`logo_mime` and building the string) is Plan 16-02's responsibility, per D-04/D-11.

---
*Phase: 16-documents-html-print*
*Completed: 2026-07-05*

## Self-Check: PASSED

All 6 created/modified source files and the SUMMARY.md itself verified present on disk. All 4 commit hashes (5360818, 3dad322, ed9650e, 7207ab9) verified present in git log.
