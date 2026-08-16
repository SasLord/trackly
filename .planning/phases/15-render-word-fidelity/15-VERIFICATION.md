---
phase: 15-render-word-fidelity
verified: 2026-07-04T10:00:00Z
status: passed
score: 5/5 must-haves verified
overrides_applied: 0
reverification: "Gap WR-05/PDFA-02 closed by plan 15-04 (see 'Re-Verification' section at end); prior gaps_found history preserved below."
prior_status: gaps_found
gaps:
  - truth: "An act with N (2+) devices renders each device's identification + long fields as a repeating per-device card, all positions present, no clipping/overlap (roadmap Success Criterion #2: 'без обрезки/наложения текста')"
    status: partial
    reason: >
      The renderer has zero vertical pagination/bounds-checking. render_docspec builds
      exactly one krilla page and never compares the running y-cursor against
      A4_HEIGHT_PT - MARGIN_PT anywhere in render_docspec or render_section. Empirically
      confirmed by rendering an act with 10 devices (well within the app's own 100-item
      limit) each carrying a populated long Комплектация field: the resulting PDF has
      exactly 1 page marker, yet pdf_extract recovers all 10 device names and the long-kit
      text — proving krilla emits content-stream text-show operators unconditionally,
      irrespective of whether the y-coordinate falls within the visible A4 page area.
      Content past ~790pt is drawn but not visible when the PDF is actually viewed/printed
      — a silent, undetectable-by-text-extraction data-loss/overlap defect for any
      realistic multi-device act (reviewer's own math: ~4-5 devices with populated long
      fields already exceeds one page). The phase's own test suite
      (render_handover_multi_device_wraps_long_fields) asserts only via text extraction,
      which cannot catch this because it reads the content stream, not rendered page
      geometry — so the gap is real and untested by the phase's regression suite despite
      "all positions present" reading as satisfied by extraction.
    artifacts:
      - path: "crates/trackly-app/src/pdf/renderer.rs"
        issue: "render_docspec (lines 129-156) and render_section never check y against A4_HEIGHT_PT - MARGIN_PT; no page-break/pagination logic exists anywhere in the module (confirmed: A4_HEIGHT_PT has exactly one use site, in PageSettings::from_wh, never in a bounds comparison)"
    missing:
      - "Page-break logic in the section-render loop: when y would exceed A4_HEIGHT_PT - MARGIN_PT, finish the current page and start a new one, resetting y to MARGIN_PT (as flagged in 15-REVIEW.md WR-05)"
      - "Or, at minimum, a documented/enforced practical item-count cap tied to the single-page assumption, if pagination is deliberately deferred to a future phase"
      - "A regression test that actually measures rendered page count or asserts a page-break occurred for N devices with long fields, since text-extraction assertions cannot catch this class of defect"
deferred: []
human_verification:
  - test: "Visual pixel-level fidelity to the Word sample (spacing, weights, overall layout) for a typical 1-3 device act"
    expected: "Generated PDF looks structurally and stylistically consistent with the original Word sample (исходный образец не хранится в репозитории; shapka, title, intro, device block, deadline, signatures)"
    why_human: "Aesthetic/layout match is subjective and out of reach for grep/text-extraction assertions — VALIDATION.md itself scopes this as the one Manual-Only Verification for the phase."
  - test: "Render a handover act with 4+ devices each carrying populated Комплектация/Технические характеристики/Состояние text, open the resulting PDF in a viewer, and visually confirm every device card is visible on a page (not clipped past the bottom margin)"
    expected: "Either all device cards fit on a single visible page, or the document paginates so every card is visible on some page — no card should be drawn past the printable page area"
    why_human: "This is the WR-05 gap (see Gaps Summary) — text-extraction tests pass even when content overflows the page invisibly; only visual/print inspection or PDF-page-geometry tooling can catch this, and no such tooling was used in this phase's automated tests."
---

# Phase 15: Рендер и соответствие образцу Verification Report

**Phase Goal:** Сгенерированный PDF акта визуально воспроизводит структуру и содержание образца Word (шапка, заголовок, вводная формулировка, мультиустройство, срок, подписи), кириллица рендерится корректно, а дефолтный редактируемый шаблон и тесты обновлены и проходят.
**Verified:** 2026-07-04
**Status:** gaps_found
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | PDF воспроизводит все блоки образца в правильном порядке (шапка/заголовок/вводная/устройства/срок/подписи) | ✓ VERIFIED | `act_handover.minijinja` (lines 34-122) emits header → heading "Акт приема-передачи" → number/date paragraph → D-09 intro paragraph with `receiver_name` → `device_card` loop → conditional "Сроком до" → signature block with sublabels, in exactly that order. Confirmed rendered by `render_docspec`/`render_section` (renderer.rs). Tests `render_handover_act_contains_d09_intro_phrase`, `signature_renders_two_line_labels` pass. |
| 2 | Акт с N(2+) устройствами печатает все позиции без обрезки/наложения текста | ⚠️ PARTIAL | `Section::DeviceCard` renders per-device long fields via real `wrap_text_to_width` (no ellipsis truncation) — confirmed in code and via `render_handover_multi_device_wraps_long_fields` (5 devices, long Cyrillic kit value, no `…`, mid-value substring present). **However**, no vertical pagination exists anywhere in the renderer (confirmed empirically: a 10-device act with populated long fields renders as a single-page PDF where content silently draws past the visible page area — text-extraction assertions cannot detect this). This defeats "без... наложения" for any real multi-device act beyond ~4-5 populated devices, which is well within the app's supported 100-item range. See Gaps. |
| 3 | Блок подписей печатает две двухстрочные подписи «Выдал»/«Получил» с «Подпись»/«ФИО» | ✓ VERIFIED | `Section::Signature` extended with `left_sublabel`/`right_sublabel` (docspec.rs 92-108), rendered in `render_section`'s Signature arm (renderer.rs 478-534) with a second text line drawn only when a sublabel is present (verified backward-compat via `signature_sublabels_default_to_none_when_absent` unit test). Template emits exactly `"Подпись"`/`"ФИО"` under `"Выдал"`/`"Получил"`. Test `signature_renders_two_line_labels` passes (asserts all 4 strings present in extracted text). |
| 4 | Кириллица рендерится корректно во всех блоках (включая мультиустройство и реквизиты) | ✓ VERIFIED | Existing embedded-DejaVu-Sans pipeline unchanged; regression-tested via `render_handover_act_produces_cyrillic_pdf` (receiver name), `render_handover_multi_device_wraps_long_fields` (150+ char Cyrillic kit string, mid-value substring survives), template_service demo_ctx and act_service.rs context all carry Cyrillic org/device data through unchanged font-embedding code path. No regressions in existing Cyrillic tests. |
| 5 | Дефолтный шаблон обновлён, сидируется при первом запуске, редактируем через document_templates; существующие тесты проходят + новые тесты на мультиустройство добавлены | ✓ VERIFIED | `act_handover.minijinja` rewritten (verified content matches D-09 order/wording); `templates_seed.rs` tests (`default_seeded_on_first_startup`, `seed_is_idempotent`, `seed_restores_after_full_soft_delete`) pass unchanged — seeding mechanism untouched. Full `cargo test -p trackly-app` run by this verifier: 75/75 test-result blocks `ok`, 0 `FAILED`. `cargo clippy -p trackly-app -- -D warnings` clean. `cargo fmt --check` clean. New tests confirmed present and passing: `render_handover_multi_device_wraps_long_fields`, `signature_renders_two_line_labels`, `render_handover_act_contains_d09_intro_phrase`, `blob_logo_via_full_pipeline_renders_in_act_pdf`, `device_card_long_field_wraps_instead_of_truncating`. |

**Score:** 4/5 truths fully verified (1 partial — pagination gap)

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/trackly-app/src/pdf/docspec.rs` | `Section::Signature` sublabels + `Section::DeviceCard` variant | ✓ VERIFIED | Both present, correctly `#[serde(default)]`/backward-compat, unit-tested (`section_enum_tagged_serde`, `signature_sublabels_default_to_none_when_absent`, `signature_sublabels_round_trip_when_present`) |
| `crates/trackly-app/src/pdf/renderer.rs` | `render_header_two_column`, `wrap_text_to_width`, extended Signature/DeviceCard render arms | ✓ VERIFIED | All three present and substantive (real ttf-parser glyph-metrics wrap, real two-column draw logic, real backward-compat-preserving sublabel rendering) — not stubs. Wired: called from `render_docspec` for every render. |
| `crates/trackly-app/Cargo.toml` | `ttf-parser` promoted to direct dependency | ✓ VERIFIED | `ttf-parser = "0.25.1"` present; `use ttf_parser::Face;` imported and used in renderer.rs |
| `crates/trackly-app/src/services/act_service.rs` | `render_pdf` propagates BLOB logo bytes (WR-03 fix) | ✓ VERIFIED | `let (org_dto, logo_bytes, logo_mime) = ...` — no discarded `_logo_bytes`/`_logo_mime` bindings remain (grep confirms zero matches); `spec.header.logo_bytes = Some(bytes)` mutation present exactly once, before `render_docspec` call |
| `crates/trackly-app/templates/act_handover.minijinja` | Rewritten to D-09 order/wording, D-06 per-device loop, D-07 sublabels | ✓ VERIFIED | Read in full — matches all claimed structure, uses `device_card` section type, conditional long-field emission, correct signature block |
| `crates/trackly-app/src/services/template_service.rs` | `validate_preview` demo_ctx synced with new schema | ✓ VERIFIED | `phone/fax/email/okpo/ogrn` + `specs/kit/condition` present in `demo_ctx` (lines 269-295) |
| `crates/trackly-app/tests/pdf_render_act.rs`, `pdf_logo.rs`, `pdf_column_overflow.rs`, `fixtures/act_42.sha256` | New regression tests + regenerated determinism fixture | ✓ VERIFIED | All new tests present, run, and pass (verified directly by this verifier, not just SUMMARY claim) |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| `renderer.rs::render_docspec` | `render_header_two_column` | Direct function call | ✓ WIRED | Confirmed at renderer.rs:140 |
| `renderer.rs::wrap_text_to_width` | `ttf_parser::Face::glyph_hor_advance` | Font metrics call | ✓ WIRED | Confirmed at renderer.rs:337 |
| `act_service.rs::render_pdf` | `docspec.rs::HeaderBlock.logo_bytes` | Post-parse mutation before render_docspec | ✓ WIRED | Confirmed at act_service.rs:1466-1468 |
| `act_handover.minijinja` | `docspec.rs::Section::DeviceCard` | MiniJinja `for item in act.items` loop emitting `"type": "device_card"` | ✓ WIRED | Confirmed in template (lines 60-84) and matching enum tag in docspec.rs |
| `renderer.rs::render_docspec` | Page-bounds / pagination | y-cursor vs A4_HEIGHT_PT comparison | ✗ NOT WIRED | No such comparison exists anywhere in the module — confirmed by grep (A4_HEIGHT_PT has exactly one use site: page-size construction) and by an empirical 10-device render (single page, content silently past visible area) |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|---------------------|--------|
| `HeaderBlock.logo_bytes` | `logo_bytes` (act_service.rs) | `org_db.get_for_pdf()` → real `org_settings` BLOB column read | Yes | ✓ FLOWING (confirmed via passing `blob_logo_via_full_pipeline_renders_in_act_pdf` full-pipeline test, not direct-renderer bypass) |
| `Section::DeviceCard.long_fields` | `item.kit/specs/condition` (template) | `act.items[].complectation_at_time/specs/condition_at_time` from real DB rows via `act_service.rs` items_json mapping | Yes | ✓ FLOWING (confirmed via `render_handover_multi_device_wraps_long_fields` — direct `UPDATE act_items` then full-pipeline render recovers the value) |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Full trackly-app test suite green | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app` | 75/75 test-result blocks `ok`, 0 `FAILED` | ✓ PASS |
| Clippy clean | `cargo clippy -p trackly-app -- -D warnings` | Finished, no warnings | ✓ PASS |
| Fmt clean | `cargo fmt --check` | No diff | ✓ PASS |
| Phase-specific test files individually | `cargo test -p trackly-app --test pdf_render_act/pdf_logo/pdf_column_overflow/pdf_determinism -- --test-threads=1` | All pass (10+4+6+2 tests) | ✓ PASS |
| Multi-device pagination check (verifier-authored, not in phase's test suite) | Ad-hoc 10-device render + PDF page-marker count + text extraction | 1 page marker for 10 devices with long fields; all text still extractable (proves silent off-page draw, not a crash) | ✗ FAIL (confirms WR-05 gap) — temp test file removed after verification, not committed |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|--------------|-------------|--------------|--------|----------|
| PDFA-01 | 15-01, 15-02, 15-03 | Structure/order match to Word sample | ✓ SATISFIED | Template order, header two-column, D-09 intro all present and tested |
| PDFA-02 | 15-01, 15-02, 15-03 | N-device act prints all positions correctly, no truncation/overlap | ⚠️ PARTIAL | Wrap-without-truncation for long fields is real and tested; "no overlap/no clipping" is defeated for realistic multi-device counts by the missing-pagination gap (WR-05) |
| PDFA-05 | 15-01, 15-02, 15-03 | Two-line signatures Подпись/ФИО under Выдал/Получил | ✓ SATISFIED | Implemented, backward-compat, tested |
| PDFA-07 | 15-01, 15-03 | Cyrillic renders correctly across new template fields | ✓ SATISFIED | Regression-tested with long Cyrillic values, no font-pipeline changes |
| PDFA-08 | 15-03 | Existing PDF tests pass + new tests for template/multi-device added | ✓ SATISFIED (as literally scoped) | All existing + new tests pass; note the new multi-device tests do not exercise page-count/geometry, only text-extraction — a blind spot inherited from PDFA-08's own test design, not a violation of its literal wording |

No orphaned requirements: all 5 phase-assigned IDs (PDFA-01/02/05/07/08) appear in at least one plan's `requirements:` frontmatter and are traced Complete in REQUIREMENTS.md; nothing outside this set is mapped to Phase 15.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| — | — | No TBD/FIXME/XXX/TODO/HACK/PLACEHOLDER markers found in any phase-modified file | — | None — clean |

The 6 WARNING findings in `15-REVIEW.md` (WR-01 logo/text overlap in header, WR-02 DeviceCard identification/heading no width clamp, WR-03 requisites no width clamp, WR-04 space-width heuristic underestimate, WR-05 no pagination, WR-06 regex recompilation perf) were reviewed. WR-05 is escalated here to a phase-gap because it directly defeats roadmap Success Criterion #2 ("без обрезки/наложения текста") for realistic multi-device acts, a scenario the phase's goal explicitly names ("мультиустройство"). WR-01/02/03/04/06 are real fidelity/perf defects but do not on their own defeat a specific must-have truth for typical (short-requisite, few-device) acts — they remain advisory per the task's framing and are not elevated to blockers.

### Human Verification Required

1. **Visual pixel-level fidelity to Word sample**
   **Test:** Generate an act PDF (1-3 devices, typical requisites) and visually compare against the original Word sample (исходный образец не хранится в репозитории).
   **Expected:** Structural/stylistic match (spacing, weights, block order) per VALIDATION.md's own Manual-Only Verification scope.
   **Why human:** Subjective aesthetic comparison, out of reach of grep/text-extraction.

2. **Multi-device page overflow check**
   **Test:** Render a handover act with 4+ devices, each with populated Комплектация/Технические характеристики/Состояние, open the PDF in a viewer/printer preview.
   **Expected:** Every device card is visible somewhere in the printed/viewed document — no card silently drawn past the page's bottom margin.
   **Why human:** This verifier confirmed programmatically that content overflows a single page silently (text-extraction still "sees" the content even when it's off-page) — only visual inspection or PDF page-geometry tooling (not used by the phase's own tests) can confirm the actual severity/frequency in real usage.

### Gaps Summary

Phase 15 delivers a real, substantively-implemented renderer capability (two-column header, real glyph-metric word-wrap, two-line signatures, per-device hybrid card, BLOB logo plumbing fix) and a fully rewritten default template matching the Word sample's block order and wording. All claimed commits exist, all claimed tests exist and pass, the full `cargo test -p trackly-app` suite (75 test blocks) is green, and clippy/fmt are clean — none of this was fabricated in the SUMMARYs.

The one real gap: **the renderer has no vertical pagination**, and this was already flagged by the phase's own code review (WR-05) as an advisory warning. Verification escalates it to a phase-blocking gap because the roadmap's own Success Criterion #2 for this phase explicitly requires "Акт с несколькими устройствами (2+) печатает все позиции корректно... без обрезки/наложения текста" — and an empirical test performed during this verification (10 devices, populated long fields, single-page PDF, all text still extractable despite being drawn past the printable area) demonstrates that for any realistic multi-device act, content silently overflows past the visible/printable page — a data-loss/overlap outcome for the end user, exactly what the success criterion prohibits. The phase's own new tests (`render_handover_multi_device_wraps_long_fields`) cannot catch this because they assert via `pdf_extract`'s text stream reading, which recovers text regardless of its page-geometry position.

This is not a fabricated-summary problem — the executor's SUMMARYs accurately describe what was built and tested — it is a scope gap between "what the automated tests can observe" (text presence) and "what the roadmap success criterion requires" (no visual clipping/overlap). Recommend a closure plan that either (a) adds page-break/pagination logic to `render_docspec`/`render_section` per the REVIEW.md WR-05 fix suggestion, or (b) if pagination is deliberately deferred, documents an explicit practical item-count cap and downgrades PDFA-02/the roadmap SC to reflect the single-page constraint, with human sign-off recorded as an override.

---

*Verified: 2026-07-04*
*Verifier: Claude (gsd-verifier)*

---

## Re-Verification (2026-07-04, after plan 15-04)

**Verdict: PASSED — 5/5 must-haves verified. The single prior gap (WR-05 / PDFA-02) is closed.**

Gap-closure plan `15-04` added vertical pagination to the krilla renderer. Re-verification confirms:

| WR-05 must-have | Evidence | Status |
|-----------------|----------|--------|
| `render_docspec`/`render_section` compare the running y-cursor against `A4_HEIGHT_PT - MARGIN_PT` and start a new page (y reset to `MARGIN_PT`) before overflowing content | `renderer.rs:147` (`page_bottom = A4_HEIGHT_PT - MARGIN_PT`), section loop `renderer.rs:151-188` triggers `doc.start_page_with(...)` + `y = MARGIN_PT` on overflow for both DeviceCard (`:158-164`) and non-card sections (`:171-177`) | ✓ CLOSED |
| DeviceCard kept atomic across a page break (whole card moves to a fresh page, never split mid-card) | Measure-then-place: `measure_device_card_height` (`renderer.rs:422`) computes full card height using the same `wrap_text_to_width` calls as the draw arm; the card is moved as a unit before drawing (`:157-164`) | ✓ CLOSED |
| Multi-page verified by counting ACTUAL PDF page objects, not text-extraction alone | `render_handover_multi_device_paginates_when_overflowing_one_page` (`pdf_render_act.rs:337`) asserts `pdf_extract::extract_text_from_mem_by_pages(...).len() > 1` for an 8-device act with long Cyrillic fields, plus a no-data-loss check that all 8 device names survive across pages | ✓ CLOSED |
| Common single-device / typical act still renders exactly 1 page (no regression) | `fixture_act_42_renders_to_known_hash` passes with `act_42.sha256` **unchanged** (byte-identical output); `rendering_twice_yields_identical_bytes` still green | ✓ CLOSED |

### Re-Verification Spot-Checks (run by orchestrator, one `cargo test` at a time per project rule)

| Command | Result |
|---------|--------|
| `cargo test -p trackly-app --test pdf_render_act -- --test-threads=1` | 11/11 ok (incl. new pagination test) |
| `cargo test -p trackly-app --test pdf_determinism -- --test-threads=1` | 2/2 ok — `act_42.sha256` fixture unchanged |

Executor's full-suite run (`TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app`) reported green with clippy `-D warnings` and `fmt --check` clean on the same tree.

### Requirements Coverage — updated

| Requirement | Prior | Now | Note |
|-------------|-------|-----|------|
| PDFA-02 | ⚠️ PARTIAL (pagination missing) | ✓ SATISFIED | Multi-device acts now paginate; cards atomic; verified by real page-count assertion. Roadmap Success Criterion #2 ("без обрезки/наложения текста") met for realistic multi-device counts. |

**Advisory (not blocking, unchanged from prior review):** WR-01/02/03/04/06 from `15-REVIEW.md` remain open fidelity/perf refinements (header logo/text overlap clamp, width clamps, space-width heuristic, regex recompilation). None defeats a phase must-have; carry to backlog if desired.

*Re-verified: 2026-07-04 — Claude (orchestrator, inline; gsd-verifier hit session limit before writing)*
