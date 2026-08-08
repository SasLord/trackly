---
phase: 34-document-header
plan: 03
subsystem: pdf-templates
tags: [minijinja, multi_template, html-templates, org-full-name, privacy-scrub]

# Dependency graph
requires: ["34-01: org_settings.full_name + OrgSettingsDto.full_name + org_full_name_html helper", "34-02: _header.html partial + {% include \"_header.html\" %} in all 3 templates"]
provides:
  - "render_with_timeout(env, name, template_src, ctx, extra_templates) — registers named partials (e.g. _header.html) before render, D-13-compliant (no env.set_loader anywhere)"
  - "minijinja crate feature multi_template enabled — required for {% include %} to parse at all"
  - "org.full_name wired (via org_full_name_html) into all 4 render contexts: act_service::render_pdf, act_service::render_acceptance_pdf, report_service::export_pdf, template_service::validate_preview/demo_context_for_kind"
  - "tests/html_header_parity.rs Test 3 — render-gate proving byte-identical header fragments across all 3 document forms"
affects: [34-04, 34-05, 34-06]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "minijinja Environment::add_template_owned loop for extra_templates BEFORE the main template's add_template_owned, both before get_template/render (D-13) — extras converted to owned Strings before the spawn_blocking 'static move closure, mirroring template_src's existing to_owned() pattern"
    - "minijinja's multi_template Cargo feature is required for the {% include %} statement to parse — distinct from (and does not enable) a filesystem loader; env.set_loader remains never-called"

key-files:
  created: []
  modified:
    - crates/trackly-app/Cargo.toml
    - crates/trackly-app/src/pdf/minijinja_env.rs
    - crates/trackly-app/src/services/act_service.rs
    - crates/trackly-app/src/services/report_service.rs
    - crates/trackly-app/src/services/template_service.rs
    - crates/trackly-app/tests/html_report_render.rs
    - crates/trackly-app/tests/html_header_parity.rs
    - crates/trackly-app/tests/pdf_render_act.rs

key-decisions:
  - "Enabled minijinja's `multi_template` Cargo feature (Rule 3 — blocking, not a new package, just a feature flag on an already-present dependency): without it, {% include %} fails to parse at all ('unknown statement include'), regardless of D-13's registration-order fix. This is orthogonal to the 'no filesystem loader' invariant — env.set_loader is still never called."
  - "list_all_for_editor now filters out filenames starting with '_' (Rule 1 bug fix): Plan 34-02's registration of _header.html into DEFAULT_HTML_TEMPLATES surfaced it as a 4th 'editable kind' in the TemplateEditor list, breaking the pre-existing 3-kinds assertion. Partials are not standalone editable document kinds."
requirements-completed: [DOC-04, DOC-05]

duration: ~50min
completed: 2026-08-09
---

# Phase 34 Plan 03: Wire the shared header partial into every render path Summary

**Made `_header.html` actually render by teaching `render_with_timeout` to register extra partials before render (D-13) and enabling minijinja's `multi_template` feature, then wired `org.full_name` (escaped via `org_full_name_html`) into all four render contexts — restoring the full pre-existing HTML/PDF render test suite to green.**

## Performance

- **Duration:** ~50 min
- **Tasks:** 3 completed
- **Files modified:** 8 (0 created, 8 modified)

## Accomplishments

- `render_with_timeout` gained a 5th parameter, `extra_templates: &[(&str, &str)]`, converted to owned `Vec<(String, String)>` before the `spawn_blocking` closure and registered via a loop of `add_template_owned` calls that land BEFORE the main template's registration and BEFORE `get_template`/`render` (D-13). `env.set_loader` remains never called anywhere in the file (grep-confirmed).
- Discovered and fixed a genuine blocker: minijinja's `{% include %}` statement does not parse at all without the crate's `multi_template` feature flag — added it to `Cargo.toml` (`default-features = false, features = [..., "multi_template"]`). This is orthogonal to the "no filesystem loader" invariant; `multi_template` only enables the include/import/extends *syntax*, not a loader.
- Rewrote the stale `build_safe_html_env` doc-comment (C-04): no longer claims zero `| safe` usage; now names the two sanctioned exceptions (`org.logo_data_uri`, `org.full_name`) and states the server-side-only-escaping invariant explicitly.
- All four render call sites (`act_service::render_pdf`, `act_service::render_acceptance_pdf`, `report_service::export_pdf`, `template_service::validate_preview`) now: (1) resolve `_header.html`'s embedded default from `DEFAULT_HTML_TEMPLATES`, (2) load it file-first via `html_templates::load_template`, (3) pass it as `render_with_timeout`'s `extra_templates`, and (4) supply `ctx["org"]["full_name"]` pre-escaped through `org_full_name_html`.
- `demo_context_for_kind`'s shared `org` block gained a fictional multiline `full_name` and had its pre-existing REAL phone/fax/OKPO/OGRN/email values (committed since Phase 15, `6ad0202`) replaced with fictional values matching `tests/org_settings.rs`'s existing fictional test patterns (`+7 495 123-45-67` / `12345678` / `1027700123456` style) — this touch does not add a new commit carrying the leak forward.
- New render-gate test (`tests/html_header_parity.rs` Test 3): after a real `OrgDbService::save_fields` write including a non-empty `full_name`, renders all three document types through the real pipeline and asserts the `<!-- HEADER-START -->...<!-- HEADER-END -->` fragments are byte-identical across all three — a structural, not merely substring-based, proof of DOC-04.
- New end-to-end case (`tests/pdf_render_act.rs`): a multiline `full_name` set via `OrgPatch`/`save_fields` renders `<br />` in the actual `render_pdf` output and never a literal `\n`, proving the escape-then-`<br>` transform reaches production output, not just `org_full_name_html`'s isolated unit tests.
- Fixed a regression surfaced by this plan's own full-suite run: `list_all_for_editor` iterated all of `DEFAULT_HTML_TEMPLATES` unfiltered, so Plan 34-02's `_header.html` registration made it appear as a 4th "editable kind," breaking the pre-existing `list_all_for_editor_returns_all_known_kinds_from_files` test (expected exactly 3). Filtered out filenames starting with `_` (shared partials are not standalone editable document kinds).

## Task Commits

Each task was committed atomically:

1. **Task 1: render_with_timeout accepts extra partials (D-13) + C-04 doc fix** - `d69615c` (feat)
2. **Task 2: Wire org.full_name + _header.html into all 4 render call sites** - `9c0cd8f` (feat)
3. **Task 3: Render-gate cross-form test + full_name render-through case (+ editor-list fix)** - `69e8fe9` (feat)

## Files Created/Modified

- `crates/trackly-app/Cargo.toml` - enabled minijinja's `multi_template` feature (required for `{% include %}` to parse; deviation, Rule 3)
- `crates/trackly-app/src/pdf/minijinja_env.rs` - `render_with_timeout` gains `extra_templates` param; 4 pre-existing tests updated to pass `&[]`; 2 new tests for partial registration + missing-partial error; C-04 doc-comment rewritten
- `crates/trackly-app/src/services/act_service.rs` - `render_pdf`/`render_acceptance_pdf`: `_header.html` load + `ctx["org"]["full_name"]` via `org_full_name_html`
- `crates/trackly-app/src/services/report_service.rs` - `export_pdf`: same wiring (local `org` binding)
- `crates/trackly-app/src/services/template_service.rs` - `validate_preview`: same header-partial wiring; `demo_context_for_kind`: fictional `full_name` + real-value privacy scrub; `list_all_for_editor`: filters partials
- `crates/trackly-app/tests/html_report_render.rs` - `empty_org()` helper gains `full_name: String::new()`
- `crates/trackly-app/tests/html_header_parity.rs` - Test 3 render-gate + supporting pipeline/fixture helpers
- `crates/trackly-app/tests/pdf_render_act.rs` - new multiline-`full_name` render-through case

## Decisions Made

- `multi_template` feature addition treated as Rule 3 (blocking auto-fix), not a new package-manager install — it enables a feature of an already-present, already-vetted dependency (`minijinja`), not a new crate.
- `list_all_for_editor` filter treated as Rule 1 (bug auto-fix) — a direct, in-scope consequence of Plan 34-02's registration of `_header.html`, surfaced only once this plan's full-suite run actually exercised that code path (Plan 34-02 explicitly scoped its own verification away from the full suite).
- Privacy scrub of `demo_context_for_kind`'s real requisites (phone/fax/OKPO/OGRN/email) done in the same touch that adds `full_name`, per the plan's explicit design intent — not the full-repo privacy cleanup (that remains PRIV-01, Phase 37).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] minijinja's `{% include %}` statement failed to parse without the `multi_template` Cargo feature**
- **Found during:** Task 1, first `cargo test -p trackly-app --lib pdf::minijinja_env` run against the new `extra_templates` behavior tests
- **Issue:** `render_with_timeout(&env, "main", "{% include \"partial\" %}Hi {{ name }}", ..., &[("partial", "PARTIAL-")])` failed with `Template parse error: syntax error: unknown statement include`. The crate's `minijinja` dependency was `default-features = false` with only `["builtins", "json", "fuel", "serde"]` enabled — none of which include `multi_template`, the feature that enables the `{% include %}`/`{% import %}`/`{% extends %}` statement syntax.
- **Fix:** Added `"multi_template"` to `crates/trackly-app/Cargo.toml`'s minijinja feature list. This is orthogonal to D-13's "no filesystem loader" invariant — `multi_template` enables the statement *syntax* only; it does not enable a `Loader`, and `env.set_loader` remains never called (grep-confirmed, `grep -c "set_loader"` returns 0).
- **Files modified:** `crates/trackly-app/Cargo.toml`
- **Verification:** All 10 `pdf::minijinja_env` unit tests pass, including the 2 new partial-registration tests; the whole crate compiles and the full HTML render suite passes.
- **Committed in:** `d69615c` (Task 1 commit)

**2. [Rule 1 - Bug] `list_all_for_editor` surfaced `_header.html` as a 4th editable "kind" once it joined `DEFAULT_HTML_TEMPLATES`**
- **Found during:** Task 3's full-suite verification run (`cargo test -p trackly-app --lib -- --test-threads=1`)
- **Issue:** `services::template_service::tests::list_all_for_editor_returns_all_known_kinds_from_files` failed: `assertion left == right failed: must return exactly 3 known kinds — left: 4, right: 3`. Plan 34-02 registered `_header.html` as a 4th `DEFAULT_HTML_TEMPLATES` entry (by design, so `load_template`/`render_with_timeout` call sites could resolve it), but `list_all_for_editor` iterated the whole array unfiltered, so the partial leaked into the editor's "editable document kinds" list.
- **Fix:** Added `.filter(|(filename, _)| !filename.starts_with('_'))` to `list_all_for_editor`'s iterator chain — shared partials are not standalone editable document kinds; no user-facing preview/save flow exists for an isolated header fragment.
- **Files modified:** `crates/trackly-app/src/services/template_service.rs`
- **Verification:** `list_all_for_editor_returns_all_known_kinds_from_files` and the other 10 `template_service` lib tests pass; the full `--lib` suite (154/154) passes.
- **Committed in:** `69e8fe9` (Task 3 commit)

---

**Total deviations:** 2 auto-fixed (1 Rule 3/blocking, 1 Rule 1/bug)
**Impact on plan:** Both were required to reach the plan's stated primary gate (full pre-existing HTML/PDF render test suite restored to green) — no scope creep, no architectural changes.

## Test Results (foreground, per orchestrator instruction)

- `cargo test -p trackly-app --test html_act_render --test html_report_render --test pdf_render_act --test html_header_parity -- --test-threads=1` → **33/33 passed** (10 + 8 + 12 + 3), 0 failed.
- `cargo test -p trackly-app --test pdf_text_extract --test pdf_column_overflow --test pdf_logo --test pdf_logo_aspect --test template_edit --test templates_seed -- --test-threads=1` → **27/27 passed** (1 + 4 + 6 + 6 + 6 + 4), 0 failed.
- `cargo test -p trackly-app --lib -- --test-threads=1` → **154/154 passed**, 0 failed.
- `pnpm --dir ui build` → succeeded (required prerequisite for `security_headers`'s SPA test in the broader suite).
- `auth_remember_cookie` intentionally excluded per project convention (pre-existing, unrelated hang) — not part of this plan's scope.

## Issues Encountered

None beyond the two deviations documented above. A background full-suite run was attempted mid-session but abandoned per orchestrator guidance (background task notifications for long-running foreground-preferred commands are unreliable in this harness) — all final verification was re-run and confirmed in the foreground per the instructions above.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- The full pre-existing HTML/PDF render test suite is green again; Plan 34-02's intentional intermediate breakage is fully resolved.
- `org.full_name` reaches every render path, always pre-escaped via `org_full_name_html`.
- `demo_context_for_kind`'s real-requisite leak does not carry forward into any new commit from this plan.
- No blockers for Plan 34-04/34-05/34-06.

## Self-Check: PASSED

- FOUND: crates/trackly-app/src/pdf/minijinja_env.rs
- FOUND: crates/trackly-app/src/services/act_service.rs
- FOUND: crates/trackly-app/src/services/report_service.rs
- FOUND: crates/trackly-app/src/services/template_service.rs
- FOUND: crates/trackly-app/tests/html_header_parity.rs
- FOUND: crates/trackly-app/tests/pdf_render_act.rs
- FOUND commit: d69615c (Task 1)
- FOUND commit: 9c0cd8f (Task 2)
- FOUND commit: 69e8fe9 (Task 3)

---
*Phase: 34-document-header*
*Completed: 2026-08-09*
