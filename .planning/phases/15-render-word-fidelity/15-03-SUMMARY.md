---
phase: 15-render-word-fidelity
plan: 03
subsystem: testing
tags: [pdf, krilla, minijinja, integration-tests, determinism-fixture, wr-03-regression]

# Dependency graph
requires:
  - phase: 15-render-word-fidelity
    plan: 01
    provides: render_header_two_column, wrap_text_to_width, two-line Section::Signature sublabels
  - phase: 15-render-word-fidelity
    plan: 02
    provides: WR-03 logo-BLOB fix in act_service.rs, Section::DeviceCard, rewritten act_handover.minijinja (D-06/D-07/D-09)
provides:
  - "Full-pipeline (act_service::render_pdf) test proving BLOB logo reaches the rendered PDF — closes the WR-03 regression gap that direct-renderer tests never caught"
  - "1-vs-5 device coverage with long-field wrap assertions (no ellipsis truncation, mid-value survival) via the real act_handover.minijinja template"
  - "Two-line signature sublabel coverage (Подпись/ФИО under Выдал/Получил) and D-09 intro-phrase coverage, both via the full render_pdf pipeline"
  - "Regenerated pdf_determinism.rs act_42.sha256 pinned hash reflecting Plan 15-01/15-02's renderer changes — deliberate, reviewable regeneration per T-15-09"
affects: []

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Full-pipeline test fixtures (writer/readers/templates/organization/pdf/org_db wired via ActService::with_pdf_pipeline/with_org_db) duplicated locally in pdf_logo.rs and pdf_column_overflow.rs rather than exporting pdf_render_act.rs's private helpers — keeps each test file's diff self-contained, per the plan's own scoping guidance"
    - "Direct act_items UPDATE (complectation_at_time SET ... WHERE act_id=? AND device_id=?) used to inject long test values post-creation, mirroring the existing devices.notes UPDATE idiom already used elsewhere in the same test files"

key-files:
  created: []
  modified:
    - crates/trackly-app/tests/pdf_render_act.rs
    - crates/trackly-app/tests/pdf_logo.rs
    - crates/trackly-app/tests/pdf_column_overflow.rs
    - crates/trackly-app/tests/fixtures/act_42.sha256
    - crates/trackly-app/tests/acts_e2e_smoke.rs

key-decisions:
  - "render_handover_act_produces_cyrillic_pdf's assertion changed from the old giver_name-in-body wording to receiver_name (rendered by D-09's intro paragraph) — this is the planned N=1 regression anchor, not new test authoring; the N=5 case with wrap coverage is the new render_handover_multi_device_wraps_long_fields test"
  - "acts_e2e_smoke.rs::handover_pdf_render_within_e2e (a file NOT in this plan's files_modified) exhibited the exact same D-09 giver_name-in-body drift as pdf_render_act.rs — fixed as a Rule 1 auto-fix (bug caused by this phase's own template rewrite, scoped strictly to the one assertion line) rather than left failing, since 'full cargo test -p trackly-app suite green' is this plan's own phase-gate requirement"
  - "act_42.sha256 regenerated from 88df7f9d... to caaca9c5... by running the test, copying the printed actual hash, and overwriting only the .sha256 file — act_42.json fixture input confirmed untouched via git diff --stat (T-15-09 mitigation: deliberate, reviewable regeneration, not a blanket UPDATE_EXPECT-style auto-accept)"
  - "Full workspace gate run with TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 (project MEMORY convention) — without these, restore_request_visibility_http.rs fails on a real-LDAP-unavailable 503 vs expected 403; this is a pre-existing, out-of-scope environment requirement, not a regression from this plan"

requirements-completed: [PDFA-01, PDFA-02, PDFA-05, PDFA-07, PDFA-08]

# Metrics
duration: 50min
completed: 2026-07-04
---

# Phase 15 Plan 03: Регрессионные тесты и детерминизм-фикстура Summary

**Full-pipeline тест закрывает WR-03 (BLOB-логотип реально доходит до PDF через act_service::render_pdf), добавлено покрытие 1-vs-5 устройств с переносом длинных полей без обрезки, двухстрочные подписи и D-09 вводная фраза — все через реальный пайплайн, плюс осознанно регенерирована pinned-hash фикстура pdf_determinism.rs.**

## Performance

- **Duration:** ~50 min
- **Started:** 2026-07-04T06:25:56Z
- **Completed:** 2026-07-04T07:14:17Z
- **Tasks:** 3
- **Files modified:** 5 (pdf_render_act.rs, pdf_logo.rs, pdf_column_overflow.rs, act_42.sha256, acts_e2e_smoke.rs)

## Accomplishments

- **WR-03 регрессия закрыта тестом:** `pdf_logo.rs::blob_logo_via_full_pipeline_renders_in_act_pdf` сохраняет BLOB-логотип через `OrgDbService::save_logo`, создаёт акт и рендерит его через `ActService::render_pdf` (не `PdfRenderer::render_docspec` напрямую, как все существующие тесты в этом файле) — это именно тот тест, который упал бы до фикса в Plan 15-02.
- **1-vs-N мультиустройство:** `pdf_render_act.rs::render_handover_multi_device_wraps_long_fields` — акт на 5 устройств, длинное (150+ символов) кириллическое значение `complectation_at_time` на 2 из 5 позиций через прямой `UPDATE act_items`; проверяется присутствие всех 5 имён устройств, отсутствие '…' (перенос вместо обрезки) и сохранность маркера из середины длинного значения.
- **Двухстрочные подписи + D-09 вводная фраза:** `signature_renders_two_line_labels` (Выдал/Получил/Подпись/ФИО) и `render_handover_act_contains_d09_intro_phrase` («Настоящим актом утверждаю» + интерполированное `receiver_name`) — оба через полный пайплайн.
- **`pdf_column_overflow.rs::device_card_long_field_wraps_instead_of_truncating`** — full-pipeline контраст к существующему `long_name_truncated_does_not_overlap_inv_no`: доказывает, что `ItemsTable` по-прежнему обрезает свои колонки эллипсисом, а новые `DeviceCard` длинные поля переносятся, а не обрезаются — оба кодовых пути сосуществуют корректно.
- **`act_42.sha256` регенерирован** (`88df7f9d…` → `caaca9c5…`) — детерминированный дрейф хэша из-за изменений рендерера в Plan 15-01/15-02, ожидаемый и задокументированный шаг (не регрессия). `act_42.json` (входная фикстура) не тронут — подтверждено `git diff --stat`.
- **Побочно обнаружен и исправлен ещё один экземпляр D-09-дрейфа** в `acts_e2e_smoke.rs::handover_pdf_render_within_e2e` (файл не входил в `files_modified` плана) — та же самая категория ожидаемого дрейфа, что и в `pdf_render_act.rs` (giver_name больше не рендерится в теле акта), но обнаруженная только при полном прогоне сьюта. Исправлено как Rule 1 auto-fix.
- **Полный `cargo test -p trackly-app`** (74 тестовых бинаря, `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1`) зелёный, `cargo clippy -p trackly-app -- -D warnings` чист, `cargo fmt --check` чист — phase-gate выполнен.

## Task Commits

1. **Task 1: Add multi-device (1 vs N) wrap test and two-line signature test to pdf_render_act.rs** - `dd3d268` (test)
2. **Task 2: Add full-pipeline logo test to pdf_logo.rs + extend pdf_column_overflow.rs contrast test** - `f872045` (test)
3. **Task 3: Run full trackly-app test suite, regenerate pdf_determinism.rs pinned hash, final phase-gate checks** - `e097fa6` (test)

## Files Created/Modified

- `crates/trackly-app/tests/pdf_render_act.rs` - added `render_handover_multi_device_wraps_long_fields`, `signature_renders_two_line_labels`, `render_handover_act_contains_d09_intro_phrase`; updated `render_handover_act_produces_cyrillic_pdf`'s assertion from stale giver_name wording to receiver_name (D-09 regression anchor)
- `crates/trackly-app/tests/pdf_logo.rs` - added `blob_logo_via_full_pipeline_renders_in_act_pdf` (full pipeline: `OrgDbService::save_logo` → `ActService::create` → `ActService::render_pdf`)
- `crates/trackly-app/tests/pdf_column_overflow.rs` - added `device_card_long_field_wraps_instead_of_truncating` (full pipeline, contrasts with the pre-existing `ItemsTable` truncation test in the same file)
- `crates/trackly-app/tests/fixtures/act_42.sha256` - regenerated pinned hash (single-line format preserved)
- `crates/trackly-app/tests/acts_e2e_smoke.rs` - fixed `handover_pdf_render_within_e2e`'s stale giver_name-in-body assertion (D-09 drift, Rule 1 auto-fix)

## Decisions Made

- `render_handover_act_produces_cyrillic_pdf` retained as the N=1 regression anchor per the plan's instruction, with its assertion updated to `receiver_name` (rendered by D-09's intro paragraph) instead of the removed giver_name-in-body wording.
- Both new full-pipeline test files (`pdf_logo.rs`, `pdf_column_overflow.rs`) build their own local fixture setup (writer/readers/templates/organization/pdf/org_db) rather than exporting `pdf_render_act.rs`'s private `make_full_pipeline*` helpers — keeps each file's diff scoped to itself, per the plan's explicit allowance.
- `acts_e2e_smoke.rs`'s independently-discovered D-09 drift was fixed in-scope (Rule 1) rather than deferred, since this plan's own phase-gate requires the full `cargo test -p trackly-app` suite green, and the fix was a single assertion-line change with no architectural implications.
- Full workspace gate command run with `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1` per project MEMORY convention (`ci_test_requirements.md`) — `restore_request_visibility_http.rs` requires these to avoid attempting a real LDAP bind; this is unrelated to Phase 15 and out of this plan's scope boundary.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed D-09 giver_name-in-body drift in acts_e2e_smoke.rs (outside plan's files_modified)**
- **Found during:** Task 3 (full-suite phase-gate run)
- **Issue:** `handover_pdf_render_within_e2e` asserted `text.contains("Сидоров-Петроградский")` (the old giver_name-in-body wording). D-09's template rewrite (Plan 15-02) intentionally removed giver_name from the rendered body — it now only appears via the bare "Выдал" signature label. This is the exact same drift category documented for `pdf_render_act.rs` in 15-02's SUMMARY, but in a file the plan's frontmatter didn't list, so it wasn't caught until the full-suite run.
- **Fix:** Changed the assertion to `text.contains("Петров")` (the act's `receiver_name`, "Петров П.П.", which D-09's intro paragraph does render).
- **Files modified:** `crates/trackly-app/tests/acts_e2e_smoke.rs`
- **Verification:** `cargo test -p trackly-app --test acts_e2e_smoke -- --test-threads=1` — all 4 tests pass.
- **Committed in:** `e097fa6` (Task 3 commit)

---

**Total deviations:** 1 auto-fixed (1 bug fix, Rule 1)
**Impact on plan:** Necessary to satisfy this plan's own phase-gate requirement (full `cargo test -p trackly-app` green). Same root cause and same fix pattern as the plan's own explicitly-anticipated drift in `pdf_render_act.rs`, just discovered in an adjacent file. No scope creep — single assertion line changed.

## Issues Encountered

- `export_bindings` test failed transiently in one interrupted background run (likely a stale/concurrent binary artifact) but passed cleanly both in isolation and in the final clean full-suite run — not a real failure, no code change needed.
- `restore_request_visibility_http.rs` fails without `TRACKLY_AD_MOCK=1`/`TRACKLY_SNMP_MOCK=1` (attempts a real LDAP bind, gets 503 instead of the expected 403) — confirmed as a pre-existing, documented (project MEMORY `ci_test_requirements.md`) environment requirement unrelated to Phase 15; out of this plan's scope boundary per the deviation rules' scope-boundary guidance.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Phase 15 (render-word-fidelity) is complete: all 3 plans (15-01 renderer primitives, 15-02 template rewrite + WR-03 fix, 15-03 regression tests + determinism fixture) executed and committed.
- Full `cargo test -p trackly-app` (74 test binaries), `cargo clippy -p trackly-app -- -D warnings`, and `cargo fmt --check` are all green — ready for `/gsd-verify-work`.
- Milestone v1.1.1 (PDF-акт по образцу Word) has both its phases (14-data-and-act-structure, 15-render-word-fidelity) complete.
- No blockers.

---
*Phase: 15-render-word-fidelity*
*Completed: 2026-07-04*
