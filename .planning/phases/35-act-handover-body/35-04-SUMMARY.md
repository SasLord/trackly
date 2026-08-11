---
phase: 35-act-handover-body
plan: 04
subsystem: tests
tags: [minijinja, html-templates, integration-tests, structural-gate, act-handover]

# Dependency graph
requires:
  - phase: 35-act-handover-body
    plan: 02
    provides: "act_handover.html body reworked per D-01..D-12 (plain-text field rows, horizontal signature block with printed act.giver_name/act.receiver_name, no 'ФИО' sublabel) — the markup these tests assert against"
  - phase: 35-act-handover-body
    plan: 03
    provides: "act_acceptance.html signature-block parity (D-09) — duplicate 'Кто передал'/'Кто принял' table rows removed"
provides:
  - "pdf_render_act.rs and html_act_render.rs tests updated to assert the current D-06/D-07/D-09 markup instead of the Phase 15 D-09 two-line-signature/ФИО-sublabel behavior"
  - "acts_e2e_smoke.rs comment corrected to describe current behavior (assertion was already compatible)"
  - "New structural regex gate (html_field_row_underline_gate.rs) durably guarding DOC-07: .field-row carries no border-bottom, and exactly two border-bottom declarations exist in act_handover.html's <style> block, belonging to .value-blank and .signature-field .signature-line"
affects: [35-05]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "CSS-by-selector structural gate (extract_style_block + extract_rule_body helpers) — checks CSS rule bodies by exact selector rather than a markup text range, avoiding the false-negative trap where the entire <style> block sits before any markup-range marker"

key-files:
  modified:
    - crates/trackly-app/tests/pdf_render_act.rs
    - crates/trackly-app/tests/html_act_render.rs
    - crates/trackly-app/tests/acts_e2e_smoke.rs
  created:
    - crates/trackly-app/tests/html_field_row_underline_gate.rs

key-decisions:
  - "Full-suite verification command from the plan (`cargo test -p trackly-app -- --test-threads=1`) reliably hangs on the pre-existing auth_remember_cookie test (login_remember_persistent_cookie), which lives inside the trackly-app package and is therefore included by a package-scoped run, not just `--workspace` as project memory states. Ran the equivalent verification with `-- --skip login_remember_persistent_cookie --test-threads=1` instead — all other tests in the package pass."

patterns-established:
  - "CSS-by-selector structural gate pattern (html_field_row_underline_gate.rs) — reusable template for any future 'this CSS property must not appear in rule X, and must appear in exactly these N other rules' regression guard."

requirements-completed: [DOC-07, DOC-08, DOC-09]

# Metrics
duration: ~40min (includes ~50min lost to a killed background full-suite run that hit the known auth_remember_cookie hang before the --skip workaround was found)
completed: 2026-08-11
---

# Phase 35 Plan 04: Test updates for D-06/D-07/D-09 signature-block drift + DOC-07 structural gate Summary

**Rewrote three existing integration test files to assert the current horizontal signature-block markup (printed act.giver_name/act.receiver_name, no "ФИО" sublabel) instead of the Phase 15 D-09 behavior Plans 02/03 intentionally superseded, and added a new cheap structural regex gate that durably guards DOC-07's "no underline except two allowed spots" invariant by CSS selector.**

## Performance

- **Duration:** ~40 min of active work (plus ~50 min lost investigating a killed background `cargo test -p trackly-app` run that hit the known `auth_remember_cookie` test hang)
- **Started:** 2026-08-11T13:20:00Z (approx.)
- **Completed:** 2026-08-11T14:00:49Z
- **Tasks:** 3
- **Files modified:** 3, **files created:** 1

## Accomplishments

- `pdf_render_act.rs`: renamed `signature_renders_two_line_labels` to `signature_renders_giver_name_horizontal_block`, dropped the removed "ФИО" sublabel assertion, added an explicit assertion that `act.giver_name` ("Иванов И.И.") is printed in the signature block; updated the stale Phase 15 D-09 doc-comment on `render_handover_act_produces_cyrillic_pdf` to describe the current D-06 behavior; confirmed `render_handover_act_contains_d09_intro_phrase` is byte-for-byte unchanged (`git diff` shows zero changes to its body)
- `html_act_render.rs`: removed "ФИО" from `html_handover_contains_required_blocks_and_logo`'s expected-label list, added an explicit assertion that the giver's printed name ("Выдалов В.В.") appears; extended `html_acceptance_contains_required_blocks` with an assertion that "Кто передал" no longer appears, confirming the D-09 table-row dedup
- `acts_e2e_smoke.rs`: replaced the stale Phase 15 D-09 comment on `handover_pdf_render_within_e2e` with a description of the current D-06 behavior; the assertion itself (`html.contains("Петров")`) was already compatible and left unchanged
- Created `crates/trackly-app/tests/html_field_row_underline_gate.rs`, modeled on the existing `html_page_parity.rs` structural-gate pattern: reads `act_handover.html` via compile-time `include_str!`, extracts the `<style>` block content, then extracts individual CSS rule bodies by exact selector (`.field-row`, `.value-blank`, `.signature-field .signature-line`). Asserts `.field-row` has no `border-bottom`, exactly 2 `border-bottom` occurrences exist in the whole style block, and both belong to the two legitimate sources (blank-deadline fallback and signature line)
- Ran the full `trackly-app` test package (minus the known pre-existing `auth_remember_cookie` hang) — all tests pass, confirming no regressions from Plans 02/03's markup changes beyond the planned test drift this plan closes

## Task Commits

Each task was committed atomically:

1. **Task 1: tests/pdf_render_act.rs — rewrite under D-06/D-07** - `7c433b3` (test)
2. **Task 2: tests/html_act_render.rs — drop ФИО assertion, extend dedup check** - `af5cc44` (test)
3. **Task 3: acts_e2e_smoke.rs comment + new html_field_row_underline_gate.rs** - `e0382cb` (test)

**Plan metadata:** this SUMMARY + STATE/ROADMAP updates (see final commit below)

## Files Created/Modified

- `crates/trackly-app/tests/pdf_render_act.rs` — `signature_renders_two_line_labels` renamed and rewritten; doc-comment on `render_handover_act_produces_cyrillic_pdf` updated; `render_handover_act_contains_d09_intro_phrase` and `render_handover_multi_device_wraps_long_fields` left untouched (confirmed no `<span class="value">` dependency in the latter)
- `crates/trackly-app/tests/html_act_render.rs` — `html_handover_contains_required_blocks_and_logo` label list and new giver-name assertion; `html_acceptance_contains_required_blocks` extended with dedup assertion
- `crates/trackly-app/tests/acts_e2e_smoke.rs` — comment-only change on `handover_pdf_render_within_e2e`; `acceptance_pdf_render_smoke` untouched
- `crates/trackly-app/tests/html_field_row_underline_gate.rs` (new) — DOC-07 structural regex gate

## Decisions Made

- The new structural gate checks CSS **by selector**, not by markup text range, per the plan's `<interfaces>` guidance — a range-based approach (e.g. "text between `{% include %}` and `.signatures`") would be unusable by construction here since the entire `<style>` block sits before the `{% include %}` marker in the current template
- Reused the `regex` crate (already a normal, non-dev dependency of `trackly-app`, `Cargo.toml:54`) rather than adding a new dependency

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking issue] Plan's literal full-suite verify command hangs on a pre-existing, out-of-scope test**
- **Found during:** Task 3 verification (running the plan's specified `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app -- --test-threads=1`)
- **Issue:** The command ran for ~50 minutes with no output and was ultimately killed by the environment. Investigation found the cause: `crates/trackly-app/tests/auth_remember_cookie.rs`'s `login_remember_persistent_cookie` test is a *pre-existing* hang (Phase 09-04, cookie-expiry timing) documented in project memory as affecting `cargo test --workspace`. That test file lives *inside* the `trackly-app` package itself, so a package-scoped `-p trackly-app` run (without a `--test` filter) also pulls it in and hangs — not just `--workspace` as the memory's wording suggested.
- **Fix:** Re-ran the equivalent full-package verification with `-- --skip login_remember_persistent_cookie --test-threads=1` appended. Every other test in the package compiled and passed (confirmed via full log inspection — the tail-truncated summary showed the final ~15 test binaries all green, and the overall command exited 0).
- **Files modified:** none — verification-only workaround, no code change
- **Commit:** n/a

No other deviations — all three tasks executed as specified in the plan and its acceptance criteria.

## Issues Encountered

The pre-existing `auth_remember_cookie` hang (see deviation above) is out of scope for this plan per the Scope Boundary rule (it is not caused by this plan's changes) and is not fixed here. It remains a known, previously-documented issue; future full-suite runs on this package should use `-- --skip login_remember_persistent_cookie` (or a more targeted `--test` filter) rather than the plan's literal `-p trackly-app -- --test-threads=1` command.

## Verification Results

- `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test pdf_render_act -- --test-threads=1` — 12 passed, including the renamed `signature_renders_giver_name_horizontal_block`
- `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test html_act_render -- --test-threads=1` — 11 passed
- `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test acts_e2e_smoke -- --test-threads=1` — 4 passed
- `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test html_field_row_underline_gate -- --test-threads=1` — 1 passed
- `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app -- --skip login_remember_persistent_cookie --test-threads=1` — full package green (exit code 0), confirming no regressions beyond the planned drift this plan closes
- `grep -c "border-bottom" crates/trackly-app/templates/act_handover.html` — 2 (matches the new structural gate's assertion)

## Privacy Check

All ФИО literals used or added in this plan's test changes are pre-existing fictional names already committed in the test suite ("Иванов И.И.", "Выдалов В.В.", "Петров П.П.") — no new literals introduced, no real organization or personnel data. `git diff` for this plan's commits reviewed and contains no real names, requisites, or database-sourced fixture strings.

## User Setup Required

None — test-only changes, no backend or runtime configuration touched.

## Next Phase Readiness

Plan 05 can proceed. All test drift identified in CONTEXT.md C-03 and RESEARCH.md Pitfall 3 is now closed: `pdf_render_act.rs`, `html_act_render.rs`, and `acts_e2e_smoke.rs` all assert current D-06/D-07/D-09 behavior, and a new durable structural gate (`html_field_row_underline_gate.rs`) protects DOC-07's underline-removal invariant going forward. No blockers. Note for Plan 05 or any future full-suite verification on this package: use `-- --skip login_remember_persistent_cookie` to avoid the pre-existing hang documented above.

---
*Phase: 35-act-handover-body*
*Completed: 2026-08-11*

## Self-Check: PASSED

All created/modified files and commit hashes verified present.
