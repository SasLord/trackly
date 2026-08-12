---
phase: 35-act-handover-body
verified: 2026-08-12T00:59:17Z
status: gaps_found
score: 4/5 roadmap Success Criteria verified
overrides_applied: 0
re_verification:
  previous_status: gaps_found
  previous_score: 4/5
  gaps_closed:
    - "Success Criterion #2 — per-device attribution in multi-device act_handover.html (CR-01): the {%- if act.items | length == 1 %} gate around the device-name line is removed; every .device-block now prints its own name unconditionally, honoring the user's amended decision D-02a (CONTEXT.md). New regression test render_handover_multi_device_fields_attributable_to_own_device (pdf_render_act.rs) splits rendered HTML by the device-block marker and asserts co-location of each device's name with its own optional-field values (and absence of other devices' values) for N=3, including the zero-optional-fields case — this test is not vacuous: it would fail against the previously gated template."
  gaps_remaining: []
  regressions:
    - "New finding, not a regression of a previously-passed item: WR-02 from the post-closure code review (35-REVIEW.md) — `.signature-name { white-space: nowrap; }` in both act_handover.html and act_acceptance.html has no `min-width: 0` on the flex item, so a signer's printed ФИО (Success Criterion #4 / DOC-08) cannot shrink below its full text width and will overflow the printable page width for realistic long Cyrillic names (reviewer's line-budget math: ~294pt available at 12pt Times ≈ 47-52 characters; the project's own long-name fixture 'Сидоров-Петроградский Иван Александрович' is 39 characters and already at ~80% of that budget). This is newly-printed content in this exact phase (ФИО was not printed in the signature block before Phase 35) and matches a defect class ('ФИО clipping') already recorded as having occurred in a prior live UAT for this project. The Task 4 human-UAT approval in 35-06-SUMMARY.md is weak evidence here because it is not documented to have used a signer name near the length threshold."
gaps:
  - truth: "Success Criterion #4 — блок подписей горизонтальный, отдельная строка на каждого подписанта, ФИО подставляются автоматически из act.giver_name/act.receiver_name без изменений бэкенда"
    status: partial
    reason: >
      Structure and data-flow are correct (horizontal one-line-per-signer block, correct
      context keys, no backend change — all independently confirmed). However, the printed
      ФИО span (`.signature-name`) is `white-space: nowrap` inside a flex row without
      `min-width: 0`, so it cannot shrink below its own content width. For a long Cyrillic
      ФИО (double-barrel surname, patronymic) the row's total required width exceeds the
      ~294pt budget left after the fixed-width signature-line field and label, causing the
      printed name to overflow the page's printable area on both act_handover.html and
      act_acceptance.html. Confirmed present in current HEAD by reading the CSS (no
      `min-width: 0`/`overflow-wrap` anywhere in either template) — this is not a hypothetical,
      it is the documented WR-02 finding from the post-closure code review, independently
      re-derived here. Text-extraction/`html.contains(...)` tests structurally cannot see this
      class of defect (already a recorded lesson in this project's memory), and no test in the
      suite renders/measures a long-name case.
    artifacts:
      - path: "crates/trackly-app/templates/act_handover.html"
        issue: "Lines 117-119: `.signature-row .signature-name { white-space: nowrap; }` has no `min-width: 0` / wrap-permission, so a long ФИО overflows the fixed-width signature row instead of wrapping or shrinking."
      - path: "crates/trackly-app/templates/act_acceptance.html"
        issue: "Lines 103-105: identical duplicated CSS rule, same defect (this file duplicates the signature-block markup/CSS from act_handover.html rather than sharing a partial, per REVIEW.md WR-08 — both copies need the fix)."
    missing:
      - "Add `min-width: 0; white-space: normal; overflow-wrap: break-word;` (or equivalent) to `.signature-row .signature-name` in both act_handover.html and act_acceptance.html so long ФИО wrap within the available row width instead of overflowing off the printable page."
      - "A verification step for a long name (>= 50-55 Cyrillic characters, e.g. reusing the project's own 'Сидоров-Петроградский Иван Александрович'-class fixture) that actually renders the HTML/PDF and confirms the name is fully visible within the page's printable width — text-extraction assertions do not suffice here per this project's own established testing lesson (act-pdf-word-fidelity); a real render/print check (desktop and/or LAN-browser) is required, consistent with how Success Criterion #5 was itself verified in this phase."
deferred: []
human_verification: []
---

# Phase 35: Тело акта приёма-передачи Verification Report

**Phase Goal:** Текст акта приёма-передачи составлен в каноничной форме документа (две
стороны, «составили настоящий акт о нижеследующем», перечень, состояние, срок, подписи),
согласован с пользователем до вёрстки, без полосок-подчёркиваний под автоматически
подставляемым текстом, с горизонтальным блоком подписей — по строке на каждого подписанта
с автоподставленными ФИО.

**Verified:** 2026-08-12
**Status:** gaps_found
**Re-verification:** Yes — after gap closure (Plan 35-06)

## Goal Achievement

### Observable Truths (Roadmap Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Финальный текст акта явно согласован с пользователем ДО вёрстки тела, зафиксировано как первый шаг фазы | ✓ VERIFIED (regression-checked) | Unchanged since prior verification — `35-CONTEXT.md` D-01, text preserved verbatim in `act_handover.html:129` ("Настоящим актом утверждаю, что мною: {{ act.receiver_name }}"). No plan in Wave 5 touched the intro text. |
| 2 | Согласованный текст сверен с каноном вёрстки Word-образца; изменения не ломают Фазы 15/16 | ✓ VERIFIED | The prior gap (CR-01 — anonymous `.device-block` for N>1) is closed: `act_handover.html:140-162` prints `{{ item.name }}` unconditionally inside every device-block (the `length == 1` gate is gone — `grep -c "act.items | length == 1"` → 0). Confirmed by direct read of the template and by re-running the new regression test `render_handover_multi_device_fields_attributable_to_own_device` (`pdf_render_act.rs`), which passed and is structurally non-vacuous (splits HTML by the `<div class="device-block">` marker, asserts each block contains its own name/field values and NOT another device's, for N=3 including a device with zero optional fields). D-02a (CONTEXT.md amendment) explicitly authorizes keeping BOTH the top-level N>1 summary list AND the per-block name — the resulting duplication (code review WR-03) is a deliberate, user-approved trade-off, not a defect. |
| 3 | Нет полосок-подчёркиваний под автоподставляемыми полями; полоски остаются только там, где расписываются от руки | ✓ VERIFIED (regression-checked) | `grep -c border-bottom act_handover.html` → 2 (`.value-blank` line 75, `.signature-field .signature-line` line 110); `act_acceptance.html` → 1 (`.signature-field .signature-line` line 96). Structural gates `field_row_css_has_no_border_bottom_and_only_two_legit_exceptions_remain` AND the new `acceptance_signature_line_css_has_exactly_one_legitimate_border_bottom` (added in Plan 35-06 to close IN-01) both pass — `act_acceptance.html` is now covered by an equivalent structural gate, closing the asymmetry the previous review flagged. |
| 4 | Блок подписей горизонтальный, отдельная строка на каждого подписанта, ФИО автоподставляются из `act.giver_name`/`act.receiver_name` без изменений бэкенда | ✗ FAILED | Structure and data-flow are correct (see Key Link Verification), but `.signature-name { white-space: nowrap; }` without `min-width: 0` on the flex item means a long printed ФИО overflows the page's printable width instead of wrapping — a legibility regression matching a defect class this project has hit before ("ФИО clipping" in a prior live UAT). Present in both `act_handover.html:117-119` and `act_acceptance.html:103-105`. See Gaps. |
| 5 | Рендер настоящего PDF/превью подтверждает вёрстку на обоих транспортах (десктоп и LAN-браузер) | ✓ VERIFIED (human) | Per already-established facts, Plan 35-06 Task 4's `checkpoint:human-verify` (gate=blocking) was approved by the user for the closed CR-01 attribution behavior on both desktop and LAN-browser transports (3-device act, mixed optional fields). Treated as satisfied for what it tested. Note: this approval is NOT strong evidence for Criterion #4's WR-02 finding above — the UAT is not documented to have used a signer name near the ~50-character overflow threshold, so it does not contradict the new gap. |

**Score:** 4/5 roadmap Success Criteria verified. Criterion #2 (the previous gap) is now closed; a new, independent issue (WR-02) causes Criterion #4 to fail on re-review.

### Assessment of Fresh Code-Review Findings Requiring Explicit Judgement

Per the task's instruction, two findings from `35-REVIEW.md` needed an explicit call:

- **WR-02 (`white-space: nowrap` on `.signature-name`):** Judged a **genuine Phase 35 gap**
  against DOC-08 / Success Criterion #4 (see gap entry above). Rationale: this is newly
  printed content introduced by this exact phase (ФИО was not printed in the signature block
  before Phase 35 — CONTEXT.md's own domain note confirms D-06 "частично откатывает" the
  Phase 15 decision to remove it), the defect is objectively present in the current CSS (no
  `min-width: 0` anywhere in either template, confirmed by direct read), the failure mode
  (silent overflow/clipping of a legal document's signer name) is not a cosmetic nit but
  matches a defect class this project has specifically been burned by before, and the fix is
  well-understood and small — this is not an ambiguous judgment call requiring a human
  decision, it is an actionable code defect.
- **WR-05 (`none` literal in the template-editor preview):** Judged **out of this phase's
  scope, not a Phase 35 gap**. `git blame`/`git show 1249e5e` confirms `"suffix": null` in
  `demo_context_for_kind`'s `_` (act_handover) branch predates Phase 35 entirely — Plan 35-01's
  only change to that context block was adding the unrelated `"giver_name"` key next to the
  pre-existing `"suffix": null`. This is a pre-existing template-editor-preview cosmetic defect
  that Phase 35 did not introduce and did not touch the relevant line for; it is correctly
  scoped as a REVIEW.md warning for future work, not a gap against DOC-07/08/09.

WR-03 (device name printed twice for N>1) and WR-04 («Сроком до» printed on return acts) are
direct, deliberate consequences of user decisions D-02a and D-03 respectively (both recorded in
`35-CONTEXT.md`) — not counted as gaps, per the task's explicit instruction.

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/trackly-app/templates/act_handover.html` | Per-device attribution restored (D-02a), signature block intact | ⚠️ PARTIAL | Per-device attribution: ✓ VERIFIED. Signature block ФИО overflow risk: ✗ see gap. |
| `crates/trackly-app/templates/act_acceptance.html` | Signature parity + own DOC-07-equivalent gate | ⚠️ PARTIAL | Signature parity structure and dedup (D-09): ✓ VERIFIED. Shares the same ФИО-overflow defect as act_handover.html (duplicated markup/CSS, REVIEW.md WR-08). |
| `crates/trackly-app/tests/pdf_render_act.rs::render_handover_multi_device_fields_attributable_to_own_device` | New non-vacuous co-location regression test | ✓ VERIFIED | Present, passes, would fail against the pre-Plan-06 gated template (confirmed by reading the assertion logic against the removed gate). |
| `crates/trackly-app/src/pdf/html_templates.rs::upgrade_replaces_v22_legacy_default_with_current_bundled_body` | v22 legacy-slice regression test (WR-01 closure) | ✓ VERIFIED | Present, passes, pulls `bodies.get(2)`, has an anti-vacuous `assert_ne!(v22_body, current)` guard. |
| `crates/trackly-app/tests/html_act_render.rs` (label assertions, WR-02-prior-review closure) | Colon-suffixed label assertions that can't collide with fixture ФИО prefix | ✓ VERIFIED | `"Выдал:"`/`"Получил:"` assertions present at line 188; not vacuous against the "Выдалов В.В."/"Получилов П.П." fixture. |
| `crates/trackly-app/tests/html_field_row_underline_gate.rs::acceptance_signature_line_css_has_exactly_one_legitimate_border_bottom` | act_acceptance.html structural underline gate (IN-01 closure) | ✓ VERIFIED | Present, passes. |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| `act_handover.html` `{{ act.giver_name }}`/`{{ item.name }}` (per-block) | `act_service.rs::render_pdf` ctx | MiniJinja interpolation, `.device-block` loop | ✓ WIRED | Confirmed by rendering N=3 devices with distinct optional fields and asserting each block's own values via the new regression test; `git diff` over `crates/trackly-app/src/services/` for the whole phase is empty — no backend change. |
| `act_acceptance.html` `{{ document.giver_name }}` | `act_service.rs::render_acceptance_pdf` ctx | Pre-existing context key | ✓ WIRED | Unchanged since prior verification; not touched by Plan 35-06. |
| `html_templates.rs::KNOWN_LEGACY_DEFAULTS` | `_legacy_defaults/v22/*.html` | `include_str!` | ✓ WIRED | v22 slice confirmed present and now covered by its own regression test (`bodies.get(2)`), closing the prior review's WR-01. |
| `.signature-name` (printed ФИО) | Page printable width | CSS flexbox layout (`display:flex` row, fixed-width sibling, `white-space:nowrap` on the name span) | ✗ NOT SAFELY WIRED | No `min-width: 0` on the flex item means the name cannot shrink below its own content width when that content is forced `nowrap` — overflow risk for long names. See gap. |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|---------------------|--------|
| `act_handover.html` per-device technical fields (N>1 case) | `item.name` (device identity) | `act_service.rs` items context, joined per `act_items` row | Yes — DB-backed, now rendered unconditionally per block | ✓ FLOWING (gap closed) |
| `act_handover.html` / `act_acceptance.html` signature block | `act.giver_name` / `document.giver_name` | `act_service.rs`, DB-backed, required form field | Yes — DB-backed, non-static | ✓ FLOWING (data correct; CSS presentation of that data is the gap, not the data itself) |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| `length == 1` gate fully removed from `act_handover.html` | `grep -c "act.items | length == 1" act_handover.html` | 0 | ✓ PASS |
| Exactly 2 `border-bottom` in `act_handover.html`, 1 in `act_acceptance.html` | `grep -c border-bottom` on each | 2, 1 | ✓ PASS |
| Backend untouched across the whole phase | `git diff --stat -- crates/trackly-app/src/services/` (full phase range) | empty | ✓ PASS |
| Per-device attribution regression test | `cargo test -p trackly-app --test pdf_render_act` | 13 passed, 0 failed | ✓ PASS |
| v22 legacy-slice regression test | `cargo test -p trackly-app --lib pdf::html_templates` | 13 passed, 0 failed | ✓ PASS |
| act_acceptance.html underline gate + label assertions | `cargo test -p trackly-app --test html_field_row_underline_gate --test html_act_render` | 2 passed / 11 passed, 0 failed | ✓ PASS |
| Privacy gate on requisite literals | `./scripts/check-privacy-requisites.sh` | "Privacy gate OK" | ✓ PASS |
| No debt markers (TBD/FIXME/XXX/TODO/HACK/PLACEHOLDER) in phase-touched files | `grep -n -E "TBD|FIXME|XXX|TODO|HACK|PLACEHOLDER"` across templates/tests/html_templates.rs/template_service.rs | no matches | ✓ PASS |
| `.signature-name` CSS overflow risk for long ФИО | manual CSS read: `.signature-row{display:flex}` + `.signature-field{flex:0 0 160pt}` + `.signature-name{white-space:nowrap}`, no `min-width:0` on `.signature-name` | reviewer's line-budget math (~294pt / ~47-52 chars) independently re-derived from the same CSS values | ✗ FAIL (see gap) |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| DOC-07 | 35-02, 35-04, 35-05, 35-06 | Нет полосок-подчёркиваний под автоподставляемым текстом | ✓ SATISFIED | Structural gates now cover both `act_handover.html` and `act_acceptance.html`; exactly the 2 and 1 legitimate exceptions respectively. |
| DOC-08 | 35-01, 35-02, 35-03, 35-04, 35-05, 35-06 | Горизонтальный блок подписей, автоподстановка ФИО, без изменений бэкенда | ⚠️ PARTIALLY SATISFIED | Structure/wiring/backend-untouched: ✓. Long-ФИО print legibility (WR-02): ✗ — see gap. |
| DOC-09 | 35-02, 35-04, 35-05, 35-06 | Текст акта в каноничной форме (две стороны, предмет, состояние, срок, подписи), согласован до вёрстки | ✓ SATISFIED | Text agreement (D-01) unchanged; multi-device "предмет" attribution (CR-01) now closed by Plan 35-06 — every device is self-identifying regardless of N. |

No orphaned requirements: REQUIREMENTS.md maps only DOC-07/DOC-08/DOC-09 to Phase 35, and all
three appear in the `requirements:` frontmatter of at least one plan (35-01 through 35-06).

### Anti-Patterns Found

No BLOCKER-level anti-patterns (no unresolved TBD/FIXME/XXX debt markers in any file touched by
this phase). The following are code-review WARNING/INFO items from `35-REVIEW.md` that do not
block this phase's roadmap Success Criteria but are worth carrying forward:

| File | Pattern | Severity | Impact |
|------|---------|----------|--------|
| `crates/trackly-app/src/pdf/html_templates.rs` | Missing `_legacy_defaults/v23/` snapshot for the intermediate (gated) `act_handover.html` body that existed between Plans 35-02/35-03 and 35-06 | ⚠️ Warning (already tracked as a separate follow-up per 35-06-SUMMARY.md and 35-REVIEW.md WR-01; not gapped here per task instruction) | Installs that materialized the app between commits `3904da9`..`bbfed54` would not receive the CR-01 auto-upgrade; no release tag carries that body (latest tag `v1.3` predates all of Phase 35) |
| `act_handover.html` / `act_acceptance.html` | Signature-block markup/CSS (~48 lines) duplicated verbatim across two user-editable templates instead of a shared `_header.html`-style partial (REVIEW.md WR-08) | ℹ️ Info/code-quality | The WR-02 fix above must be applied in both files; any future signature-block change has the same double-maintenance risk |
| `crates/trackly-app/tests/pdf_render_act.rs:198-202`, `html_act_render.rs:204-207` | Vacuous act-number assertions (`contains("1")` always-true given `<style>` literals like `1px`) (REVIEW.md WR-07) | ℹ️ Info/test-quality | Pre-existing test-quality gap, not newly introduced by Phase 35's requirements |
| `template_service.rs:508` | `"suffix": null` renders as literal `none` string in the template-editor preview (REVIEW.md WR-05) | ℹ️ Info | Confirmed pre-existing (predates Phase 35, git-blamed to before commit `1249e5e`) — out of this phase's scope |

### Human Verification Required

None required to close this phase's own re-verification decision — the remaining gap (WR-02) is
a concrete, mechanically fixable CSS defect, not an item needing subjective human judgment. Once
fixed, re-verification of the fix itself should include a real render/print check with a long
ФИО (per this project's established "real render, not text-extraction" testing discipline), but
that is a verification step for the fix's own closure, not an open question today.

### Gaps Summary

The previously reported gap (Success Criterion #2 / CR-01 — anonymous multi-device blocks) is
**closed**: `act_handover.html` now prints every device's name inside its own `.device-block`
unconditionally, matching the user's amended decision D-02a, backed by a non-vacuous regression
test that was independently confirmed to fail against the old gated template's logic.

A **new gap** surfaced on this re-review, found by the post-closure code review and independently
re-derived here by reading the CSS directly: the printed ФИО in the horizontal signature block
(`.signature-name`) is `white-space: nowrap` without `min-width: 0` in a fixed-width flex row, in
both `act_handover.html` and `act_acceptance.html`. For realistic long Cyrillic ФИО this overflows
the page's printable width — a legibility defect for content this exact phase newly introduced
into the printed document, and a defect class this project has specifically encountered before in
live UAT. This blocks full closure of Success Criterion #4 / DOC-08. The fix is small and
well-scoped (see the `missing` list in the gaps YAML above); it does not require re-opening any
text or attribution decision from this phase.

---

*Verified: 2026-08-12T00:59:17Z*
*Verifier: Claude (gsd-verifier)*
