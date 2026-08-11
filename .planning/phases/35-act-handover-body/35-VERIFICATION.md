---
phase: 35-act-handover-body
verified: 2026-08-11T00:00:00Z
status: gaps_found
score: 4/5 must-haves verified (roadmap SC), 1 additional derived truth FAILED
overrides_applied: 0
gaps:
  - truth: "Success Criterion #2 — согласованный текст сверен с каноном вёрстки Word-образца, изменения не ломают то, ради чего делались Фазы 15/16 (per-item description must remain attributable to its device)"
    status: failed
    reason: >
      For multi-device acts (act.items | length > 1), act_handover.html suppresses the
      per-device "было получено устройство: {{ item.name }}" line inside every
      .device-block (the {%- if act.items | length == 1 %} gate at line 142). Only a
      single top-level <ul> of names renders once, followed by a sequence of unlabeled
      .device-block divs containing Инвентарный номер/Серийный номер/Модель/
      Комплектация/Технические характеристики/Состояние rows with no name, heading, or
      divider identifying which device each block describes (separated only by
      margin-bottom: 8pt). The pre-Phase-35 body
      (_legacy_defaults/v22/act_handover.html) always printed the device name inside
      every block regardless of item count, so this is a fidelity regression against
      the canon Phase 15/16 established (self-identifying device blocks), independently
      confirmed by code review CR-01 and by manual reading of the current template.
      CONTEXT.md D-02 only decided to avoid repeating the LABEL text ("было получено
      устройство:") for N>1 — it never discusses or approves removing the device name
      from each per-item technical-field block. No test in the suite catches this
      (render_handover_multi_device_wraps_long_fields only asserts device-name presence
      and relative ordering, never co-location with its own field block).
    artifacts:
      - path: "crates/trackly-app/templates/act_handover.html"
        issue: "Lines 140-164: device-block loop only prints the item name (line 142-144) when act.items | length == 1; for N>1 every device-block is anonymous."
    missing:
      - "Print the device name inside every per-item .device-block, not only when there is exactly one item (drop or restructure the length==1 gate per CR-01's suggested fix), while keeping the D-02 decision to avoid repeating the full sentence label on every block if desired (e.g. a lightweight per-block heading instead of the full 'было получено устройство:' phrase)."
      - "A regression test asserting each device's technical fields are co-located with (or otherwise attributable to) that device's own name for N>1 acts, not just that names appear somewhere in the document."
deferred: []
human_verification: []
---

# Phase 35: Тело акта приёма-передачи Verification Report

**Phase Goal:** Текст акта приёма-передачи составлен в каноничной форме документа (две
стороны, «составили настоящий акт о нижеследующем», перечень, состояние, срок, подписи),
согласован с пользователем до вёрстки, без полосок-подчёркиваний под автоматически
подставляемым текстом, с горизонтальным блоком подписей — по строке на каждого подписанта
с автоподставленными ФИО.

**Verified:** 2026-08-11
**Status:** gaps_found
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths (Roadmap Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Финальный текст акта явно согласован с пользователем ДО вёрстки тела, зафиксировано как первый шаг фазы | ✓ VERIFIED | `.planning/phases/35-act-handover-body/35-CONTEXT.md` D-01 + `35-DISCUSSION-LOG.md` ("Текст акта — форма изложения" table) record the user's choice ("B — оставить текст образца") as part of context-gathering (2026-08-11), which precedes all 5 plans (35-01..35-05, all also dated 2026-08-11, plan-01 timestamped ~12:12 vs context gathered earlier same session). Text was not rewritten in any plan — `render_handover_act_contains_d09_intro_phrase` was explicitly left untouched (35-04-SUMMARY.md: "confirmed byte-for-byte unchanged"). |
| 2 | Согласованный текст сверен с каноном вёрстки Word-образца; изменения не ломают Фазы 15/16 | ✗ FAILED | Signature-block layout (horizontal, per-signer line) correctly restores the Word-sample canon per CONTEXT.md's own analysis ("Word-образец уже горизонтальный по подписям"). **However**, for multi-device acts (N>1), the per-item technical-field blocks in `act_handover.html` lost their device-name association — a regression against the pre-Phase-35 canon-compliant body, independently confirmed by code review (`35-REVIEW.md` CR-01) and my own reading of `crates/trackly-app/templates/act_handover.html:140-164`. See Gaps section. |
| 3 | Нет полосок-подчёркиваний под автоподставляемыми полями; полоски остаются только там, где расписываются от руки | ✓ VERIFIED | `grep -n "border-bottom" crates/trackly-app/templates/act_handover.html` → exactly 2 matches (line 75 `.value-blank`, line 110 `.signature-field .signature-line`); `grep -n "border-bottom" act_acceptance.html` → exactly 1 match (`.signature-field .signature-line`, no blank-underline case needed there). Structural regression gate `crates/trackly-app/tests/html_field_row_underline_gate.rs::field_row_css_has_no_border_bottom_and_only_two_legit_exceptions_remain` asserts this by CSS selector at compile-time (`include_str!`), not by markup range — durable against silent reintroduction. |
| 4 | Блок подписей горизонтальный, отдельная строка на каждого подписанта, ФИО автоподставляются из `act.giver_name`/`act.receiver_name` без изменений бэкенда | ✓ VERIFIED | Both `act_handover.html` (lines 174-191) and `act_acceptance.html` (lines 129-146) contain two `div.signature-row` blocks ("Выдал:"/"Получил:"), each with one `.signature-line` (empty underline for handwritten signature), a "Подпись" sublabel, and the printed name (`{{ act.giver_name }}`/`{{ act.receiver_name }}` or `{{ document.giver_name }}`/`{{ document.receiver_name }}`). Data-flow traced: `act_service.rs:2639-2640` populates `act.giver_name`/`act.receiver_name` from the real `act` DB row (not hardcoded) into the render context — no backend change (confirmed `git diff --stat e0d2dca~1..HEAD` shows zero changes to `act_service.rs`). |
| 5 | Рендер настоящего PDF/превью подтверждает вёрстку на обоих транспортах (десктоп и LAN-браузер) | ✓ VERIFIED (human) | Per task instructions, this criterion was performed and approved by the human user (`35-05-SUMMARY.md` Task 2, checkpoint:human-verify, gate=blocking, response: approved). Treated as satisfied by human confirmation per already-established facts — not re-verified by automation in this report. Note: the user's UAT explicitly covered "plural device-list summary for N>1 devices" at a glance, but the CR-01 defect (missing per-block name for N>1) is a subtle content-attribution issue not obviously visible without deliberately cross-checking field values against device identity — the human approval does not contradict CR-01's finding. |

**Score:** 4/5 roadmap Success Criteria verified; Criterion #2 fails specifically for the
multi-device (N>1) path.

### CR-01 Assessment: Genuine Phase 35 Gap, Not Deferred Scope to Phase 36

Per the task's explicit instruction to assess whether CR-01 is (a) a genuine gap or (b)
legitimately deferred to Phase 36, my reasoning:

- **CONTEXT.md D-02** (the only decision touching multi-device text) explicitly scopes
  itself to the *label line* only: "без повтора метки на каждое устройство" (no repeat
  of the label "было получено устройство:" on each device). It does not discuss, and the
  discussion log does not record any option about, removing the device name from each
  per-item technical-field block entirely.
- **Phase 36's roadmap scope** (`.planning/ROADMAP.md` lines 173-189, DOC-10/DOC-11) is
  specifically about *pagination*: single-page-vs-"Приложение №1" branching and page
  breaks. Its success criteria describe page-1 showing only a name list (no full
  description) and page-2+ showing a table of full descriptions — this does not
  retroactively excuse the *current*, already-shipped Phase 35 body (which is not
  paginated at all yet) from being unambiguous on its own.
  Whether Phase 36's eventual table format naturally reintroduces per-row name
  association is not yet designed or planned — deferring on that assumption would be
  exactly the kind of "vague or tangential match" the verification process instructs to
  reject (`references/gates.md` Step 9b: "Be conservative... when in doubt, keep it as a
  real gap").
- **35-05-SUMMARY.md**'s own recorded user scope clarification is specifically about
  *pagination* ("'Приложение №1' on page 2+, full-description table only from page 2"),
  not about per-block device-name association on the (currently unpaginated) body this
  phase delivers.
- **Practical impact today**: any multi-device handover act generated with the
  Phase-35-complete codebase (no Phase 36 yet) produces a document where technical
  fields cannot be reliably attributed to a specific device — a real, present-tense
  defect, not a future one Phase 36 will "get around to."

Conclusion: **CR-01 is a genuine Phase 35 gap**, not legitimately deferred to Phase 36. It
directly breaks roadmap Success Criterion #2 ("не ломают то, ради чего делались Фазы
15/16") for the multi-device path, and touches DOC-09's "предмет" (subject/item)
component of the canonical form.

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/trackly-app/templates/_legacy_defaults/v22/act_handover.html` | Byte-identical pre-Phase-35 snapshot | ✓ VERIFIED | Present; diff against current `act_handover.html` is non-trivial (body reworked), confirming snapshot predates the rework. |
| `crates/trackly-app/templates/_legacy_defaults/v22/act_acceptance.html` | Byte-identical pre-Phase-35 snapshot | ✓ VERIFIED | Present alongside handover snapshot. |
| `crates/trackly-app/src/pdf/html_templates.rs` | `KNOWN_LEGACY_DEFAULTS` v22 entries for both templates | ✓ VERIFIED | `grep -n "v22"` shows both `include_str!` entries wired at lines 81/89. |
| `crates/trackly-app/src/services/template_service.rs` | `demo_context_for_kind` `_` branch carries `act.giver_name` | ✓ VERIFIED | `"giver_name": "Иванов И.И."` present at line 477 (fictional placeholder, privacy-compliant). |
| `crates/trackly-app/templates/act_handover.html` | Reworked body per D-01..D-12 | ⚠️ PARTIAL | Underline removal (D-10), plain-text field rows (D-11), unconditional deadline (D-03), horizontal signatures (D-06/D-07/D-08), stub removal (D-12) all verified present. Multi-device per-block name association regressed (CR-01) — see gap above. |
| `crates/trackly-app/templates/act_acceptance.html` | Signature-block parity + table dedup (D-09) | ✓ VERIFIED | "Кто передал"/"Кто принял" rows removed from `table.kv` (only "Дата" row remains); signature block matches `act_handover.html`'s markup/CSS class-for-class. |
| `crates/trackly-app/tests/html_field_row_underline_gate.rs` | New structural DOC-07 regression gate | ✓ VERIFIED | Present, asserts CSS-by-selector, passed in 35-04-SUMMARY.md's verification run. |
| `crates/trackly-app/tests/pdf_render_act.rs`, `html_act_render.rs`, `acts_e2e_smoke.rs` | Test drift closed (C-03) | ✓ VERIFIED | All three files modified per 35-04-SUMMARY.md; `render_handover_act_contains_d09_intro_phrase` confirmed untouched (D-01 not reopened). |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| `act_handover.html` `{{ act.giver_name }}` | `act_service.rs::render_pdf` ctx | MiniJinja interpolation reading pre-existing context key | ✓ WIRED | `act_service.rs:2639-2640` populates `act.giver_name`/`act.receiver_name` from the real `act` row (`act.giver_name`, not a literal); confirmed no backend file changed in this phase's diff. |
| `act_acceptance.html` `{{ document.giver_name }}` | `act_service.rs::render_acceptance_pdf` ctx | Pre-existing context key (Phase 20) | ✓ WIRED | Context key already existed before Phase 35; template now reads it in the reworked signature block only (table duplication removed). |
| `html_templates.rs::KNOWN_LEGACY_DEFAULTS` | `_legacy_defaults/v22/*.html` | `include_str!` | ✓ WIRED | Confirmed via grep; both filename slices extended with the v22 element, matching the Phase 34 precedent pattern that fixes the "installed copies don't see new default" class of bug. |
| `demo_context_for_kind` | `act_handover.html` template preview | `UndefinedBehavior::Strict` context parity | ✓ WIRED | `act.giver_name` present in both the real render context and the preview demo context — no crash risk from the newly-referenced key confirmed by code review. |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|---------------------|--------|
| `act_handover.html` signature block | `act.giver_name` | `act_service.rs:2639` reads `act.giver_name` from the `acts` table row (`ActCreateDto.giver_name` is a required form field) | Yes — DB-backed, non-static | ✓ FLOWING |
| `act_acceptance.html` signature block | `document.giver_name` | Pre-existing `render_acceptance_pdf` context (Phase 20), unchanged by Phase 35 | Yes — DB-backed | ✓ FLOWING |
| `act_handover.html` per-device technical fields (N>1 case) | `item.name` (device identity) | `act_service.rs` items_json (`it.device_name`) — present in context for every item | Data present in context, but template does not render it per-block for N>1 (see CR-01) | ⚠️ DISCONNECTED (rendering gap, not a data-source gap — the name is available in `act.items[].name` but the N>1 template branch discards it per-block) |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Exactly 2 legitimate `border-bottom` sources remain in `act_handover.html` | `grep -c border-bottom act_handover.html` | 2 | ✓ PASS |
| `act_acceptance.html` has no duplicate ФИО rows in table | manual read of `table.kv` (lines 114-127) | Only "Дата" row present | ✓ PASS |
| Backend (`act_service.rs`) untouched by this phase | `git diff --stat e0d2dca~1..HEAD -- crates/trackly-app/src/services/act_service.rs` | empty diff | ✓ PASS |
| N>1 device-block anonymization | manual read of `act_handover.html:140-164` | `{%- if act.items | length == 1 %}` gates the name line; N>1 blocks have no name | ✗ FAIL (see CR-01 gap) |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| DOC-07 | 35-02, 35-04, 35-05 | Нет полосок-подчёркиваний под автоподставляемым текстом | ✓ SATISFIED | Structural gate + grep confirm exactly the 2 legitimate exceptions remain. |
| DOC-08 | 35-01, 35-02, 35-03, 35-04, 35-05 | Горизонтальный блок подписей, автоподстановка ФИО, без изменений бэкенда | ✓ SATISFIED | Both templates verified; backend diff empty. |
| DOC-09 | 35-02, 35-04, 35-05 | Текст акта в каноничной форме (две стороны, предмет, состояние, срок, подписи), согласован до вёрстки | ⚠️ PARTIALLY SATISFIED | Text agreement (D-01, timing) and single-device "предмет" description verified. Multi-device "предмет" description (CR-01) is not reliably attributable per device — undermines this requirement's "предмет" clause for N>1 acts. |

No orphaned requirements: REQUIREMENTS.md maps only DOC-07/DOC-08/DOC-09 to Phase 35, and
all three appear in at least one plan's `requirements:` frontmatter.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `crates/trackly-app/templates/act_handover.html` | 142-144 | Conditional gate (`length == 1`) silently drops device-identifying content for the N>1 branch | 🛑 Blocker | This is the CR-01 regression — not a TBD/FIXME marker, but an observable functional defect with the same real-world consequence (ambiguous printed legal document). |

No TBD/FIXME/XXX/HACK/PLACEHOLDER debt markers found in any file modified by this phase
(checked `act_handover.html`, `act_acceptance.html`, `html_templates.rs`,
`template_service.rs`, all four test files, both v22 snapshots).

### Human Verification Required

None outstanding. Criterion #5's mandatory human UAT (desktop + LAN-browser transports)
was already performed and approved by the user as part of Plan 05's blocking checkpoint
(`35-05-SUMMARY.md`, Task 2). No further human verification items were identified beyond
what has already been resolved.

### Gaps Summary

Phase 35 substantially achieves its goal for the DOC-07 (underline removal) and DOC-08
(horizontal signature block, backend-free ФИО autofill) requirements — both are
structurally verified with durable regression gates, not just text-extraction tests. The
text-agreement timing requirement (Success Criterion #1) is also satisfied: the CONTEXT.md
record shows the text-form decision (D-01) was made and recorded before any plan began
editing template bodies.

The one real gap is narrow but functionally significant: **for multi-device handover acts
(N>1 items), the reworked `act_handover.html` body loses the per-device name label inside
each device's technical-field block**, making it impossible to reliably tell which printed
Инвентарный номер/Серийный номер/Модель/Комплектация/Технические характеристики/Состояние
values belong to which listed device. This is a regression against the pre-Phase-35 body
(which always self-identified each block) and against roadmap Success Criterion #2 ("не
ломают то, ради чего делались Фазы 15/16"). It was independently found by code review
(CR-01) and confirmed here by direct reading of the current template. It was not discussed
or approved by the user in CONTEXT.md/DISCUSSION-LOG (D-02 only covers the summary-label
text, not per-block name suppression), and is not clearly covered by Phase 36's pagination
scope (which addresses page breaks, not per-block content attribution on the current,
unpaginated body).

**Recommendation:** A small closure plan restoring the device name inside every
`.device-block` for N>1 (per CR-01's suggested fix — printing the name in every block
regardless of item count, optionally keeping the top-level `<ul>` summary as well) would
close this gap without reopening any of the already-approved D-01..D-12 decisions.

---

_Verified: 2026-08-11_
_Verifier: Claude (gsd-verifier)_
