---
phase: 16-documents-html-print
plan: 05
subsystem: testing
tags: [minijinja, html-render, act-service, integration-tests, xss-safe-filter]

# Dependency graph
requires:
  - phase: 16-documents-html-print
    plan: 02
    provides: "ActService::render_pdf/render_acceptance_pdf return Result<String, AppError> (HTML), not Vec<u8>"
  - phase: 16-documents-html-print
    plan: 01
    provides: "act_handover.html/act_acceptance.html templates, html_templates module, build_safe_html_env"
provides:
  - "cargo test -p trackly-app fully green again after the Vec<u8>->String migration (76/76 test binaries pass)"
  - "html_act_render.rs — dedicated D-14 (items 1-4) coverage for both acts"
  - "Fix: org.logo_data_uri | safe in both HTML templates — logo data: URIs no longer corrupted by autoescape entity-encoding"
  - "pdf_determinism.rs's 2 heavy krilla-path tests marked #[ignore] per D-13"
affects: [16-documents-html-print]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "HTML-string assertions (html.contains(...)) replace pdf-extract/PDF-magic-header checks for all ActService-mediated tests"
    - "Scoped `| safe` filter for server-constructed, non-user-controlled values (base64 data: URIs) in an otherwise-autoescaped MiniJinja HTML environment"

key-files:
  created:
    - crates/trackly-app/tests/html_act_render.rs
  modified:
    - crates/trackly-app/tests/pdf_render_act.rs
    - crates/trackly-app/tests/pdf_column_overflow.rs
    - crates/trackly-app/tests/pdf_logo.rs
    - crates/trackly-app/tests/acts_e2e_smoke.rs
    - crates/trackly-app/tests/pdf_determinism.rs
    - crates/trackly-app/templates/act_handover.html
    - crates/trackly-app/templates/act_acceptance.html

key-decisions:
  - "render_with_missing_template_returns_notfound/render_with_broken_template_returns_validation renamed to render_falls_back_to_embedded_default_when_template_file_missing/render_falls_back_to_embedded_default_when_broken_template_row_present — the DB-backed document_templates table is no longer read by render_pdf's HTML path, so corrupting/soft-deleting it must no longer surface as an error (T-16-14 mitigation: explicit rename+reassert, not silent deletion)"
  - "render_handover_multi_device_paginates_when_overflowing_one_page deleted — krilla page-count internals have no HTML equivalent; browser pagination via CSS cannot be asserted from a raw HTML string in a Rust unit test (T-16-13, accepted per threat register)"
  - "pdf_determinism.rs's 2 tests marked #[ignore] (D-13) rather than deleted — still runnable explicitly via `cargo test -- --ignored` as a deliberate bit-rot check"
  - "Direct-renderer (frozen krilla path) tests in pdf_logo.rs (5 tests) and pdf_column_overflow.rs (3 tests) left completely untouched — they call PdfRenderer::render_docspec directly, not through ActService"

requirements-completed: [SPEC-Req3, SPEC-Req8]

# Metrics
duration: 45min
completed: 2026-07-05
---

# Phase 16 Plan 05: Test Migration to HTML Contract Summary

**Migrated all ActService-mediated PDF-byte test assertions to HTML-string assertions, added a 6-test D-14 coverage suite, and fixed a real logo-corruption bug (autoescape entity-encoding the `/` in base64 `data:` URIs) that the old byte-based tests could never have caught.**

## Performance

- **Duration:** ~45 min
- **Tasks:** 3 completed
- **Files modified:** 7 (1 created, 6 modified)

## Accomplishments

- `cargo test -p trackly-app` compiles and passes fully green again (76/76 test result blocks, 0 failed) — restoring the suite broken by Plan 16-02's `Vec<u8>` → `String` return-type change.
- Every `ActService::render_pdf`/`render_acceptance_pdf` call site across `pdf_render_act.rs`, `pdf_column_overflow.rs`, `pdf_logo.rs`, and `acts_e2e_smoke.rs` now asserts directly on the returned HTML string (`html.contains(...)`, `!html.contains('…')`, `html.contains("data:image/png;base64,")`) instead of `pdf_extract`/PDF-magic-header/image-XObject checks.
- New `html_act_render.rs` (6 tests) proves all 4 D-14 requirements for both acts: required blocks/fields + logo `data:` URI presence, 1-vs-N device completeness with no truncation, fallback-to-embedded-default vs on-disk-file-with-live-edit, and offline/no-CDN safety (no `http(s)://` in either act's markup).
- `pdf_determinism.rs`'s 2 heavy frozen-krilla tests marked `#[ignore]` per D-13 — confirmed still green via `cargo test -- --ignored`; fast direct-renderer bit-rot guards in `pdf_logo.rs`/`pdf_column_overflow.rs` remain un-ignored.
- **Rule 1 bugfix, found via test migration:** both HTML templates interpolated `{{ org.logo_data_uri }}` under MiniJinja's autoescape-ON HTML environment, which entity-encoded the `/` characters in `data:image/png;base64,...` into `&#x2f;` — silently corrupting the URI scheme and breaking every rendered logo (D-11) in production, undetected until `blob_logo_via_full_pipeline_renders_in_act_pdf` was migrated to assert on the actual HTML content instead of a raw byte marker. Fixed with a scoped `| safe` filter, documented inline as safe because the value is exclusively server-constructed from base64 output + a hardcoded mime whitelist (never user-controlled HTML) — does not reopen T-16-01's XSS mitigation.

## Task Commits

Each task was committed atomically:

1. **Task 1: Migrate existing ActService-mediated full-pipeline tests off PDF assertions** - `bbf3e4b` (test) — includes the Rule 1 logo-corruption bugfix in the two template files, discovered while migrating `pdf_logo.rs`'s full-pipeline test
2. **Task 2: New html_act_render.rs — D-14 items 1-4 coverage for both acts** - `ae22b9b` (test)
3. **Task 3: D-13 krilla test hygiene — mark heavy direct-renderer tests #[ignore]** - `dd5cc2c` (test)

## Files Created/Modified

- `crates/trackly-app/tests/html_act_render.rs` - New: 6 tests covering D-14 items 1-4 for both acts
- `crates/trackly-app/tests/pdf_render_act.rs` - 11 tests migrated to HTML-string assertions; 2 tests renamed/rewritten (fallback behavior, not error); 1 krilla-page-count test deleted
- `crates/trackly-app/tests/pdf_column_overflow.rs` - 1 full-pipeline test (`device_card_long_field_wraps_instead_of_truncating`) migrated to HTML; 3 direct-renderer tests untouched
- `crates/trackly-app/tests/pdf_logo.rs` - 1 full-pipeline test (`blob_logo_via_full_pipeline_renders_in_act_pdf`) migrated to HTML `data:` URI assertion; 5 direct-renderer tests untouched
- `crates/trackly-app/tests/acts_e2e_smoke.rs` - 3 tests (`handover_pdf_render_within_e2e`, `acceptance_pdf_render_smoke`, `document_acceptance_pdf_renders_correct_calendar_date_for_same_day_msk_selection`) migrated to HTML-string assertions
- `crates/trackly-app/tests/pdf_determinism.rs` - Both tests marked `#[ignore]` with D-13 rationale comment
- `crates/trackly-app/templates/act_handover.html` - Rule 1 fix: `org.logo_data_uri | safe` (was unescaped-breaking under autoescape)
- `crates/trackly-app/templates/act_acceptance.html` - Same Rule 1 fix

## Decisions Made

- Renamed (rather than deleted) `render_with_missing_template_returns_notfound`/`render_with_broken_template_returns_validation` to explicitly assert the new graceful-fallback behavior — T-16-14 mitigation, prevents a future reader from assuming missing-template handling was simply dropped.
- Deleted `render_handover_multi_device_paginates_when_overflowing_one_page` (krilla page-count internals) with no HTML-equivalent replacement — browser-side CSS pagination is not assertable from a raw HTML string in a Rust unit test; accepted per the plan's own threat register (T-16-13).
- Kept every direct-renderer (frozen krilla path) test completely untouched in `pdf_logo.rs`/`pdf_column_overflow.rs`/`pdf_determinism.rs` — they call `PdfRenderer::render_docspec` directly, decoupled from `ActService`'s HTML rewrite.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Logo `data:` URI corrupted by MiniJinja autoescape entity-encoding**
- **Found during:** Task 1 (migrating `pdf_logo.rs`'s `blob_logo_via_full_pipeline_renders_in_act_pdf` to assert `html.contains("data:image/png;base64,...")`)
- **Issue:** Both `act_handover.html` and `act_acceptance.html` interpolate `{{ org.logo_data_uri }}` inside an `<img src="...">` attribute under `build_safe_html_env()`'s autoescape-ON MiniJinja environment. Autoescape HTML-entity-encodes `/` into `&#x2f;`, turning `data:image/png;base64,...` into `data:image&#x2f;png;base64,...` — an invalid URI scheme that silently fails to load in any browser. This broke every rendered logo in production since Plan 16-02 (D-11's entire purpose), undetected until this plan's test migration asserted on the real HTML content instead of a PDF byte marker.
- **Fix:** Added a scoped `{{ org.logo_data_uri | safe }}` in both templates, with an inline comment explaining why this specific `| safe` does not reopen the T-16-01 XSS mitigation: the value is exclusively constructed server-side from base64 output (RFC 4648 alphabet) plus a hardcoded mime whitelist in `act_service.rs` — never user-controlled HTML.
- **Files modified:** `crates/trackly-app/templates/act_handover.html`, `crates/trackly-app/templates/act_acceptance.html`
- **Commit:** `bbf3e4b` (Task 1)

---

**Total deviations:** 1 auto-fixed (1 bug)
**Impact on plan:** Necessary correctness fix directly surfaced by the planned test migration — no scope creep. Without it, D-14 item 1 (logo presence) would have been provably false in production despite the plan's own new `html_act_render.rs` test passing against the bug (had the bugfix not been applied, `html_handover_contains_required_blocks_and_logo` would have failed too, since it also asserts `html.contains("data:image/png;base64,")`).

## Issues Encountered

None beyond the auto-fix above.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- `cargo test -p trackly-app` is fully green (76/76 test binaries), `cargo clippy -p trackly-app --tests -- -D warnings` clean, `cargo fmt --check` clean.
- Phase 16 (documents-html-print) has no further planned work after this plan per the phase's 5-plan/4-wave structure — ready for phase close/verification.
- The logo-corruption bugfix means D-11 (self-contained base64 logo embedding) is now actually functional end-to-end, not just structurally wired.

---
*Phase: 16-documents-html-print*
*Completed: 2026-07-05*

## Self-Check: PASSED

All 7 created/modified files verified present on disk. All 3 commit hashes (bbf3e4b, ae22b9b, dd5cc2c) verified present in git log.
