---
phase: 34-document-header
verified: 2026-08-11T06:30:00Z
status: human_needed
score: 3/4 must-haves verified (1 requires fresh human confirmation)
overrides_applied: 0
human_verification:
  - test: "Re-confirm visual header parity (desktop Tauri webview + LAN browser) on the CURRENT code, specifically the empty-org.full_name case (C-01) and a filled-org.full_name case, on all three print forms."
    expected: "No stray leading `<br />`, no orphan `()` around the short name when full_name is empty; a full_name + short-name-in-parentheses on its own line when full_name is set; identical geometry/typography across all three forms; no overflow on long full_name/address."
    why_human: "The only completed human checkpoint (34-06 Task 1/2, approved at commit 2038f4e, 2026-08-09T07:08:58+07:00) predates the CR-01 fix (commit e306b77, 2026-08-09T16:22:09+07:00) by ~9 hours. CR-01 was a real shipped visual defect (unguarded org.full_name producing a stray <br /> and orphan parentheses on every upgraded install with empty full_name) that directly contradicts the C-01 behavior the human's sign-off describes. The sign-off was made against pre-fix (or the stale hand-edited target/debug/templates/act_handover.html) rendering, not the code as it exists today. An automated regression test (`empty_full_name_renders_bare_short_name_without_stray_br_or_orphan_parens` in html_header_parity.rs) now encodes the correct behavior and is designed to fail on the pre-fix template, which is strong evidence the code is now correct — but the phase's own Success Criterion #2 explicitly requires confirmation via a real rendered PDF/preview, not a text/DOM assertion test, precisely because overlap/overflow/visual defects are not visible to extraction-based tests (project convention, see `act-pdf-word-fidelity` memory). This has not been re-done since the fix landed."
  - test: "Look at the `templates_status` badge (WR-05, «изменён вручную» / «файл не читается») in Settings → Шаблоны in the running app."
    expected: "Badge renders correctly beside the template selector for each of the three editable kinds, updates after save/reset, does not block editing when the status fetch fails."
    why_human: "WR-05 was added during the code-review fix pass, after the 34-06 UAT checkpoints had already been approved. REVIEW-FIX.md explicitly records: 'Not verified live — no UAT was performed... the WR-05 badge... only type-checked, linted and built.' This is a scope addition beyond plan 34-05's explicit 'no UI is added in this phase' boundary; it is a reasonable response to a 'dead API surface' review finding, not a defect, but it has never been looked at running."
gaps: []
deferred: []
---

# Phase 34: Единая шапка документов — Verification Report

**Phase Goal:** Все три печатные формы (акт приёма-передачи, акт приёмки, отчёт) выводят
одинаковую шапку — лого, название и реквизиты организации из `org.name` — в единой типографике,
и эта шапка доходит до уже установленных копий Trackly после обновления, а не только до новых.

**Verified:** 2026-08-11
**Status:** human_needed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths (Roadmap Success Criteria, DOC-04/05/06)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Правки шапки из `target/debug/templates/` перенесены в канонический `crates/trackly-app/templates/_header.html`, не утеряны при `cargo clean` | ✓ VERIFIED | `_header.html` exists, is `include_str!`-embedded in `DEFAULT_HTML_TEMPLATES` (html_templates.rs:41-42), doc-comment records the rescue provenance and privacy scrub. `target/debug/templates/` no longer contains the three hand-edited reference bodies (only the runtime-materialized `_header.html` remains), consistent with the claimed D-18 deletion. |
| 2 | Все три шаблона рендерят идентичную по вёрстке и типографике шапку, подтверждено рендером настоящего PDF/превью (не текстовым тестом) на обоих транспортах | ? UNCERTAIN | Structurally: all three templates `{% include "_header.html" %}` (confirmed by grep on act_handover.html:110, act_acceptance.html:82, report.html:112); shared CSS lives only in `_header.html` (D-05/D-09/D-10/D-12 markup read directly — 80mm centered column, 12pt bold name, 11pt requisites, Times New Roman/serif fallback chain, overflow-wrap+hyphens). A render-gate test (`header_fragment_identical_across_all_three_forms`) asserts byte-identical header fragments across all three forms including the empty-full_name path. **However**, the phase's own Success Criterion #2 requires human confirmation via a real rendered PDF/preview, and the one completed checkpoint (34-06 Tasks 1/2) was approved **before** the CR-01 fix landed (see human_verification below) — the current code has not been eyeballed since. |
| 3 | Название организации берётся из `org.name`/`org.full_name`; ни в одном из трёх шаблонов нет захардкоженного реального названия | ✓ VERIFIED | `header_partial_org_name_node_has_no_hardcoded_literal` test in `html_header_parity.rs` (structural gate). Manual grep of all three shipped templates + `_legacy_defaults/v21/*` for literal org-name patterns found none — only `{{ }}`/`{% %}` expressions and the D-04 `<br />`/`()` markup. `.orgName` node in `_header.html` contains `{{ org.full_name | safe }}` / `{{ org.name }}` only. |
| 4 | Установка с файлами, совпадающими с известным legacy-дефолтом, получает новую шапку без ручного удаления `templates/`; установка с пользовательскими правками — не затирается | ✓ VERIFIED | `upgrade_replaces_untouched_legacy_default_with_current_bundled_body` and `upgrade_leaves_user_customized_file_untouched` unit tests in `html_templates.rs` cover both branches for all four managed files (act_handover, act_acceptance, report, _header — v21 slices registered per D-15, WR-06 closes the "new file has no legacy entry" gap). Additionally corroborated by a **real pre-Phase-34 install** observed live during 34-06 UAT: `act_handover.html` (hand-edited, matched neither current nor legacy body) was correctly left untouched while `act_acceptance.html`/`report.html` (untouched legacy bodies) silently auto-upgraded — both branches of the mechanism exercised on a genuine install, not only synthetic fixtures. The originally-planned scripted `TRACKLY_TEMPLATES_DIR` scratch-dir procedure (34-06 Task 1 Step 7) was not run as written, but the real-install observation is at least as strong evidence for this specific truth. |

**Score:** 3/4 truths fully verified; 1 requires fresh human confirmation (not failed — strong automated + structural evidence, but the phase's own success criterion mandates a real-render human check that hasn't happened against current code).

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `migrations/V036__org_settings_full_name.sql` | `full_name TEXT NOT NULL DEFAULT ''`, `PRAGMA user_version = 36` | ✓ VERIFIED | File present, exact SQL confirmed by read. |
| `crates/trackly-app/src/pdf/minijinja_env.rs::org_full_name_html` | escape-then-`<br>` helper + tests | ✓ VERIFIED | Function present with 5+ unit tests (escape order, empty input, CRLF/CR normalization, ampersand escaping). |
| `crates/trackly-app/templates/_header.html` | shared header partial, HEADER-START/END markers, D-04/D-05/D-07/D-08 markup | ✓ VERIFIED | Read in full; markup matches all cited must-haves including the CR-01-fixed independently-guarded name lines. |
| `crates/trackly-app/templates/_legacy_defaults/v21/{act_handover,act_acceptance,report}.html` | pre-Phase-34 snapshots | ✓ VERIFIED | Present, registered in `KNOWN_LEGACY_DEFAULTS` (grep confirmed, second element per filename). |
| `crates/trackly-app/tests/html_header_parity.rs` | substring + privacy + render-gate + empty-full_name regression tests | ✓ VERIFIED (code-level; not re-executed live this session, see Notes) | 8 test functions present: template-include substring gate, no-hardcoded-literal gate, cross-form byte-identity, no-logo-wrapper, and the CR-01 regression test with an explicit stated non-vacuous proof against the pre-fix template. |
| `crates/trackly-app/tests/templates_status.rs` | Current/Customized/Unreadable status coverage | ✓ VERIFIED | 3 test functions present covering all three statuses. |
| `ui/src/features/settings/OrgSettings.svelte` | `full_name` field via shared `Textarea.svelte` | ✓ VERIFIED | `fullName` state, `Textarea` import/usage, round-trip through `settings_save_org_fields` confirmed by read. |
| `crates/trackly-app/src/tauri_cmds/settings_org.rs` / `http/settings_org.rs` | `templates_status` / `handler_templates_status`, `ManageSettings`-gated on both transports | ✓ VERIFIED | Both handlers present, `authorize(&caller, &Action::ManageSettings)` confirmed on both call sites; POST route registered. |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| `act_handover.html`/`act_acceptance.html`/`report.html` | `_header.html` | `{% include "_header.html" %}` | ✓ WIRED | Confirmed by grep at all three call sites. |
| `html_templates.rs::DEFAULT_HTML_TEMPLATES` | `materialize_defaults_on_startup` | 4th tuple `("_header.html", include_str!(...))` | ✓ WIRED | Confirmed at html_templates.rs:41-42. |
| `act_service.rs::render_pdf` / `render_acceptance_pdf`, `report_service.rs::export_pdf`, `template_service.rs::demo_context_for_kind` | `pdf::minijinja_env::org_full_name_html` | `"full_name": org_full_name_html(&org...full_name)` | ✓ WIRED | All four call sites confirmed by grep (act_service.rs:2641,2809; report_service.rs:705; template_service.rs:451) — no raw interpolation path found. |
| `minijinja_env.rs::render_with_timeout` | `_header.html` partial registration | `add_template_owned` before `get_template`/`render`, no `set_loader` | ✓ WIRED | Confirmed by code read: both `add_template_owned` calls precede `get_template`; `set_loader` absent from file (only referenced in a doc comment explaining its absence). |
| `OrgSettings.svelte::saveOrg` | `OrgPatch.full_name` (backend) | `apiCall('settings_save_org_fields', { patch: { full_name: fullName } })` | ✓ WIRED | Confirmed in OrgSettings.svelte and mirrored server-side in `org_db_service.rs::save_fields` (`full_name=?12` in UPDATE). |
| `tauri_cmds/settings_org.rs::build_templates_status` | `html_templates::{DEFAULT_HTML_TEMPLATES,KNOWN_LEGACY_DEFAULTS,resolve_templates_dir}` | read-only reuse | ✓ WIRED | Confirmed by code read; no write path, `spawn_blocking`-wrapped per WR-04 fix. |

### Requirements Coverage

| Requirement | Source Plan(s) | Description | Status | Evidence |
|-------------|-----------------|--------------|--------|----------|
| DOC-04 | 34-02, 34-03, 34-06 | Все три формы выводят одинаковую шапку в единой типографике | ? NEEDS HUMAN (structurally SATISFIED, visual re-confirmation pending) | See Truth #2 above. |
| DOC-05 | 34-01, 34-02, 34-04 | Название организации из `org.name`; ни один шаблон не хардкодит реальное название | ✓ SATISFIED | See Truth #3 above; DB/DTO/UI round trip confirmed end to end. |
| DOC-06 | 34-02, 34-05, 34-06 | Уже установленные копии получают новую шапку после обновления без ручного удаления `templates/` | ✓ SATISFIED | See Truth #4 above. |

No orphaned requirements: REQUIREMENTS.md maps exactly DOC-04/05/06 to Phase 34 and all three appear in the plans' `requirements:` frontmatter.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| — | — | No TBD/FIXME/XXX/TODO/HACK/PLACEHOLDER markers found in any file touched by this phase (`_header.html`, `minijinja_env.rs`, `html_templates.rs`, `org_db_service.rs`, `OrgSettings.svelte`, `TemplateEditor.svelte`) | — | none |

**Notable (informational, not blocking DOC-04/05/06):**

- **Scope deviation (WR-05):** plan 34-05 explicitly stated "no UI is added in this phase" for `templates_status`; the code-review fix pass added a `templates_status` badge to `TemplateEditor.svelte` anyway, in response to a "dead API surface" review finding. This is a reasonable, documented judgment call, not a defect — but per REVIEW-FIX.md it has never been exercised in a running app (see human_verification).
- **API behavior change (WR-07):** `reports_export_pdf`/`reports_export_csv` with `period: null` for a period-scoped report type now returns a validation error instead of silently answering. Documented, tested, and the UI never sends that combination — flagged here only because it is a boundary-visible change that landed inside this phase's fix pass, outside DOC-04/05/06's direct scope.
- **PRIV-01/WR-11 outstanding:** real-looking organization requisites from an earlier phase remain in public git history. HEAD is clean and a CI privacy gate (`scripts/check-privacy-requisites.sh`, wired into `ci-fast.yml`) now exists, but the history rewrite itself was deliberately not performed pending separate user authorization (recorded as decision PRIV-01 in STATE.md). Out of DOC-04/05/06 scope but worth carrying forward — this is what PRIV-01/PRIV-02 (future requirement) exist to formalize.

### Behavioral / Test Spot-Checks

Targeted attempt to re-run `cargo test -p trackly-app --test html_header_parity` live in this
verification session stalled during compilation (no forward progress observed over several
minutes with no output) and was killed rather than left to consume the session budget; it was not
re-attempted. This is a build-environment issue in this session, not a claim about the code.
**Not counted as a failure** — code-level inspection of the test file and the render/service wiring
directly confirms the logic described in REVIEW-FIX.md's own reported run (`html_header_parity: 5
passed (2 new)`), and the git history shows the CR-01 fix's stated non-vacuous proof (fails on the
pre-fix template with the exact reported fragment) is a specific, checkable claim consistent with
the diff. Treat this as static-verification-only for the test-execution layer; if a fresh run is
wanted, re-run `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test
html_header_parity -- --test-threads=1` in a clean shell outside this session.

### Human Verification Required

See YAML frontmatter `human_verification` for full detail. Summary:

1. **Re-confirm visual header parity on current code (desktop + LAN browser)** — the one
   completed UAT checkpoint predates the CR-01 fix by ~9 hours; the sign-off does not reflect the
   code as it exists today, even though automated regression tests now encode the fix correctly.
2. **Look at the WR-05 `templates_status` badge in the running Settings → Шаблоны screen** — added
   after all UAT was completed, never visually verified.

### Gaps Summary

No FAILED must-haves. The phase's data-layer, template-layer, and upgrade-mechanism artifacts are
all present, wired, and covered by automated tests that match their stated intent — including a
targeted regression test for the one critical defect (CR-01) found and fixed during code review.
The sole open item is procedural, not a code defect: the phase's own Success Criterion #2 requires
a **human** confirmation via real rendered preview, and the only such confirmation on record was
made against a version of the code that predates a real, shipped visual bug fix. This is escalated
to the developer rather than marked FAILED because the evidence for correctness is otherwise
strong (structural tests, code read, non-vacuous fix verification) — but the phase's own gate
demands eyes-on confirmation that has not yet happened against current `main`.

---

_Verified: 2026-08-11_
_Verifier: Claude (gsd-verifier)_
