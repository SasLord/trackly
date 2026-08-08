---
phase: 34-document-header
plan: 02
subsystem: pdf-templates
tags: [minijinja, html-templates, privacy-scrub, header-partial]

# Dependency graph
requires: ["34-01: org_settings.full_name column + OrgSettingsDto.full_name + org_full_name_html helper"]
provides:
  - "crates/trackly-app/templates/_header.html — shared header partial (D-04/D-06/D-07/D-08/D-12), privacy-scrubbed"
  - "All three canonical templates ({% include \"_header.html\" %}) + unified D-09/D-10/D-11 typography"
  - "_legacy_defaults/v21/ snapshots + DEFAULT_HTML_TEMPLATES/KNOWN_LEGACY_DEFAULTS registration (D-14/D-15)"
  - "D-16 tracing::warn! on skipped auto-upgrade"
  - "tests/html_header_parity.rs — include-gate + privacy-safe structural test (DOC-05)"
affects: [34-03, 34-04, 34-05, 34-06]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Shared Jinja partial pattern: {% include \"_header.html\" %} resolved from the in-memory Environment registry (D-13), never a filesystem loader — structurally guarantees DOC-04 (identical header across all three forms)"
    - "Legacy-snapshot upgrade detection: KNOWN_LEGACY_DEFAULTS[filename] is an append-only slice of pre-change bodies; each future header/typography change adds one more slice element (never replaces v20), with a fail-closed else branch that now logs instead of silently skipping"

key-files:
  created:
    - crates/trackly-app/templates/_header.html
    - crates/trackly-app/templates/_legacy_defaults/v21/act_handover.html
    - crates/trackly-app/templates/_legacy_defaults/v21/act_acceptance.html
    - crates/trackly-app/templates/_legacy_defaults/v21/report.html
    - crates/trackly-app/tests/html_header_parity.rs
  modified:
    - crates/trackly-app/templates/act_handover.html
    - crates/trackly-app/templates/act_acceptance.html
    - crates/trackly-app/templates/report.html
    - crates/trackly-app/src/pdf/html_templates.rs

key-decisions:
  - "Rescued the hand-edited reference geometry from target/debug/templates/act_handover.html (still present, mtime/size matched research) rather than falling back to the RESEARCH.md inventory — direct source, then manually substituted org.full_name/org.name for the hardcoded real organization name (never a whole-file copy, T-34-02-01 mitigation)."
  - "v21 snapshot taken via `cp` of the three canonical files BEFORE any rewrite edit, per D-15/Pitfall-5 timing — verified post-rewrite via non-empty diff against v21, both manually and via the plan's automated verify script."
  - "_header.html registered in DEFAULT_HTML_TEMPLATES (materialize target) but deliberately NOT registered in KNOWN_LEGACY_DEFAULTS (no legacy predecessor to upgrade from) — required adjusting the pre-existing .first()-based upgrade test (which now iterates 4 DEFAULT_HTML_TEMPLATES entries, not 3) to skip filenames with no registered legacy slice instead of panicking."

requirements-completed: [DOC-04, DOC-05, DOC-06]

duration: ~25min
completed: 2026-08-09
---

# Phase 34 Plan 02: Shared document header partial (_header.html) rescue + rewrite Summary

**Rescued the user's hand-edited reference header into a privacy-scrubbed shared `_header.html` Jinja partial, then rewrote all three canonical HTML templates to include it, unifying header geometry/typography while intentionally leaving the pre-existing render tests broken until Plan 34-03 wires the include registration.**

## Performance

- **Duration:** ~25 min
- **Tasks:** 3 completed
- **Files modified:** 9 (5 created, 4 modified)

## Accomplishments

- `crates/trackly-app/templates/_header.html` created as a partial (Jinja doc-comment, no `<!DOCTYPE`/`<html>` wrapper): centered flex `.header` column (`width: 80mm`), logo `img` (`max-height: 80pt; max-width: 140pt`), `.orgName` (12pt bold centered, `org.full_name` and `org.name` independently guarded per D-04), `.requisites` (11pt centered, D-07 order: address → address_line2 → phone → fax → email → ОКПО/ОГРН → ИНН/КПП), plus D-06 `overflow-wrap`/`hyphens` guards not present in the reference. Zero hardcoded organization name — verified by grep before commit (only label words: Телефон/Факс/E-mail/ОКПО/ОГРН/ИНН/КПП/Логотип).
- `_legacy_defaults/v21/{act_handover,act_acceptance,report}.html` snapshot the pre-Phase-34 bodies, taken via `cp` **before** any rewrite edit (verified non-empty diff against the rewritten canon afterward).
- All three canonical templates rewritten: `.header`/`.orgName`/`.requisites` CSS+markup deleted and replaced with a single `{% include "_header.html" %}`; `body { font-family }` unified to `"Times New Roman", Georgia, "PT Serif", "Liberation Serif", "DejaVu Serif", serif; font-size: 12pt` (D-09/D-10, replacing the old sans-serif chains at 11pt/10.5pt); top-level title font-size unified to 14pt (`report.html`'s `.title` dropped from 15pt; `act_handover.html`'s `.title` and `act_acceptance.html`'s `h1` were already 14pt). No field-row/table/device-block/signature markup touched.
- `html_templates.rs`: `_header.html` added as a 4th `DEFAULT_HTML_TEMPLATES` entry (materialized on any install missing it); v21 added as a second `KNOWN_LEGACY_DEFAULTS` slice element per filename (v20 untouched); the previously-silent upgrade-skip branch now emits `tracing::warn!` naming the file path (D-16); new unit test `upgrade_replaces_v21_legacy_default_with_current_bundled_body` proves the v21 element specifically (not just `.first()`/v20) drives a real upgrade, with an `assert_ne!` precondition guard against a mistimed snapshot.
- `tests/html_header_parity.rs` (new): `all_three_templates_include_header_partial` (substring gate) + `header_partial_org_name_node_has_no_hardcoded_literal` (regex-strips all Jinja expressions/statements from the `.orgName` fragment and asserts no letter remains — proves no hardcoded org name without ever writing the real name into the test file).

## Task Commits

Each task was committed atomically:

1. **Task 1: Rescue — create _header.html from the reference, privacy-scrubbed** - `4f44811` (feat)
2. **Task 2: v21 snapshot + rewrite the three canonical templates** - `ed87bea` (feat)
3. **Task 3: html_templates.rs registration + D-16 warn branch + structural tests** - `f44baea` (feat)

## Files Created/Modified

- `crates/trackly-app/templates/_header.html` — new shared header partial
- `crates/trackly-app/templates/_legacy_defaults/v21/act_handover.html` — pre-rewrite snapshot
- `crates/trackly-app/templates/_legacy_defaults/v21/act_acceptance.html` — pre-rewrite snapshot
- `crates/trackly-app/templates/_legacy_defaults/v21/report.html` — pre-rewrite snapshot
- `crates/trackly-app/templates/act_handover.html` — header block replaced with include, typography unified
- `crates/trackly-app/templates/act_acceptance.html` — header block replaced with include, typography unified
- `crates/trackly-app/templates/report.html` — header block replaced with include, typography unified, title 15pt→14pt
- `crates/trackly-app/src/pdf/html_templates.rs` — `_header.html` registered, v21 legacy slice added, D-16 warn branch, new upgrade test
- `crates/trackly-app/tests/html_header_parity.rs` — new structural test file

## Decisions Made

- Used the still-present `target/debug/templates/act_handover.html` (mtime/size matched research expectations) as the direct rescue source rather than the RESEARCH.md fallback inventory — read once, geometry and markup manually transcribed with the hardcoded real organization name replaced by `org.full_name`/`org.name` placeholders. The real name was never written into any committed artifact (verified via `git diff --cached` grep before every commit in this plan).
- `KNOWN_LEGACY_DEFAULTS`'s pre-existing `.first()`-based upgrade test iterates all of `DEFAULT_HTML_TEMPLATES`, which now includes `_header.html` (no legacy slice registered by design). Adjusted that test (and the new v21 test) to skip filenames with no registered legacy slice instead of panicking on `.expect()` — a direct, in-scope consequence of adding the 4th `DEFAULT_HTML_TEMPLATES` entry.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Pre-existing `.first()`-based upgrade test would panic once `_header.html` joined `DEFAULT_HTML_TEMPLATES`**
- **Found during:** Task 3, writing the new v21 upgrade test alongside the existing `upgrade_replaces_untouched_legacy_default_with_current_bundled_body`
- **Issue:** That test iterates `DEFAULT_HTML_TEMPLATES` and does `.find(...).and_then(...).expect("legacy snapshot registered for filename")` for every entry. Once `_header.html` (deliberately absent from `KNOWN_LEGACY_DEFAULTS`, D-14) became a 4th `DEFAULT_HTML_TEMPLATES` entry, this `.expect()` would panic on the missing lookup.
- **Fix:** Changed both the pre-existing test and the new v21 test to skip (`continue`) filenames with no registered legacy slice, rather than treating a missing slice as a test failure — matches the plan's own stated intent that `_header.html` "gets NO entry in KNOWN_LEGACY_DEFAULTS."
- **Files modified:** `crates/trackly-app/src/pdf/html_templates.rs`
- **Verification:** `cargo test -p trackly-app --lib html_templates` — 9/9 tests pass.
- **Committed in:** `f44baea` (Task 3 commit)

**2. [Rule 1 - Bug] Doc-comment prose accidentally duplicated the literal `{% include "_header.html" %}` string, breaking the "exactly once" acceptance criterion**
- **Found during:** Task 2 self-verification (occurrence-count check beyond the plan's file-count-only automated verify script)
- **Issue:** The doc-comments added to explain the header's new location quoted the literal Jinja include directive in prose, making it appear twice per file (once in the comment, once in the actual `{% include %}` call) instead of the plan's stated "exactly once."
- **Fix:** Reworded the doc-comment prose to describe the mechanism without repeating the literal directive text (e.g. "pulled in below via the Jinja include directive").
- **Files modified:** `crates/trackly-app/templates/act_handover.html`, `act_acceptance.html`, `report.html`
- **Verification:** `grep -o 'include "_header.html"' <file> | wc -l` → 1 for all three files.
- **Committed in:** `ed87bea` (Task 2 commit)

---

**Total deviations:** 2 auto-fixed (2 Rule 1/bug)
**Impact on plan:** Both were direct, in-scope consequences of this plan's own changes (adding `_header.html` to the registry; adding include-directive doc-comments) — no scope creep, no architectural changes.

## Issues Encountered

None beyond the two deviations above.

## User Setup Required

None — no external service configuration required.

## Scope Note (per plan's explicit design)

Per this plan's `<objective>`, the include registration mechanism (`render_with_timeout`'s `extra_templates` parameter) and the `org.full_name` render context wiring are deliberately NOT done here — that is Plan 34-03's job. As a direct, expected consequence, every pre-existing test that renders `act_handover.html`/`act_acceptance.html`/`report.html` (`html_act_render.rs`, `html_report_render.rs`, `pdf_render_act.rs`, `pdf_text_extract.rs`, `pdf_column_overflow.rs`, `pdf_logo*.rs`, `template_edit.rs`, `templates_seed.rs`) will currently fail with "template not found: _header.html" or similar. This plan's own verification was scoped to `cargo build -p trackly-app` (succeeds) plus the two new/updated test targets (`--lib html_templates`, `--test html_header_parity`), per the plan's explicit instruction not to run or "fix" the full suite here.

## Next Phase Readiness

- Plan 34-03 can now wire `render_with_timeout`'s `extra_templates` parameter to register `_header.html` in the minijinja `Environment`, and extend the render context with `org.full_name` (already available end-to-end from Plan 34-01's `org_full_name_html` helper) — no template-contract changes needed, only the wiring.
- No blockers for Plan 34-03/34-04/34-05/34-06.

## Self-Check: PASSED

- FOUND: crates/trackly-app/templates/_header.html
- FOUND: crates/trackly-app/templates/_legacy_defaults/v21/act_handover.html
- FOUND: crates/trackly-app/templates/_legacy_defaults/v21/act_acceptance.html
- FOUND: crates/trackly-app/templates/_legacy_defaults/v21/report.html
- FOUND: crates/trackly-app/tests/html_header_parity.rs
- FOUND commit: 4f44811 (Task 1)
- FOUND commit: ed87bea (Task 2)
- FOUND commit: f44baea (Task 3)

---
*Phase: 34-document-header*
*Completed: 2026-08-09*
