---
phase: 15-render-word-fidelity
plan: 04
subsystem: pdf
tags: [krilla, pagination, pdf-rendering, pdf_extract, gap-closure]

# Dependency graph
requires:
  - phase: 15-render-word-fidelity
    provides: "render_docspec/render_section (Plan 01), DeviceCard hybrid layout + wrap_text_to_width (Plan 02), full-pipeline regression tests + act_42 determinism fixture (Plan 03)"
provides:
  - "Vertical pagination in render_docspec/render_section: y-vs-(A4_HEIGHT_PT - MARGIN_PT) bounds check that starts a new krilla page and resets y to MARGIN_PT on overflow"
  - "measure_device_card_height helper: measure-then-place height computation for Section::DeviceCard, keeping cards atomic across page boundaries (never split mid-card)"
  - "Full-pipeline page-count regression test (render_handover_multi_device_paginates_when_overflowing_one_page) proving real multi-page output for an 8-device act with long fields"
  - "N=1 single-page regression guard added to the existing render_handover_act_produces_cyrillic_pdf test"
affects: [pdf, act-rendering, print-fidelity]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Measure-then-place pagination: a pure height-computation helper (measure_device_card_height) mirrors the exact draw-time arithmetic (including wrap_text_to_width calls) so measurement and drawing cannot disagree"
    - "Page-count assertion via pdf_extract::extract_text_from_mem_by_pages — page-tree-aware, catches content silently drawn past the printable page area that plain text-extraction assertions cannot detect"

key-files:
  created: []
  modified:
    - crates/trackly-app/src/pdf/renderer.rs
    - crates/trackly-app/tests/pdf_render_act.rs

key-decisions:
  - "Header (лого + реквизиты) рендерится только на первой странице; продолжения не повторяют шапку — соответствует структуре образца Word"
  - "DeviceCard — единственный вариант Section, требующий measure-then-place; остальные варианты (Paragraph/Heading/KeyValueTable/ItemsTable/Signature/Spacer) используют упрощённую pre-draw проверку по одной строке высоты"
  - "act_42.sha256 НЕ регенерирован — bounds-check ни разу не сработал для однодевайсового фикстура, байты рендера не изменились (проверено, не предположено)"

requirements-completed: [PDFA-02]

# Metrics
duration: 25min
completed: 2026-07-04
---

# Phase 15 Plan 04: Пагинация PDF-рендерера (закрытие WR-05) Summary

**Рендерер krilla теперь переносит контент на новую A4-страницу при переполнении, с атомарным сохранением DeviceCard целиком на одной странице — закрывает единственный найденный при верификации гэп (WR-05, score 4/5 → PDFA-02 полностью покрыт).**

## Performance

- **Duration:** ~25 min
- **Started:** 2026-07-04T08:11:54Z (per session STATE.md; this plan executed as final phase 15 plan)
- **Completed:** 2026-07-04
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments
- `render_docspec` теперь содержит реальную проверку `y` против `A4_HEIGHT_PT - MARGIN_PT` в цикле отрисовки секций: при переполнении текущая страница финишируется и стартует новая (`doc.start_page_with(...)`) с `y = MARGIN_PT`
- `Section::DeviceCard` измеряется ДО отрисовки через новый `measure_device_card_height` (переиспользует `wrap_text_to_width` для подсчёта строк — та же логика, что и при реальной отрисовке), гарантируя, что карточка устройства никогда не разрывается между страницами
- Новый full-pipeline регрессионный тест (`render_handover_multi_device_paginates_when_overflowing_one_page`) доказывает реальную многостраничность через `pdf_extract::extract_text_from_mem_by_pages` — именно тот инструмент, который верификатор указал как отсутствующий в тестовом покрытии фазы (текстовая экстракция сама по себе не может обнаружить контент, отрисованный за пределами видимой страницы)
- Типичный случай (1-2 коротких устройства, включая существующий фикстур `act_42`) подтверждён явной проверкой количества страниц == 1 — не предположением об отсутствии регрессии

## Task Commits

Each task was committed atomically:

1. **Task 1: Add page-break/pagination logic to render_docspec and render_section, keeping DeviceCard atomic across page boundaries** - `169019d` (feat)
2. **Task 2: Add full-pipeline page-count regression test to pdf_render_act.rs, regenerate act_42.sha256 if needed, final phase-gate checks** - `ba5ea52` (test)

_Note: Task 2's commit also folds in a trivial `cargo fmt` formatting fix to the Task 1 test helper code (Rule 1 auto-fix, discovered while running the `fmt --check` gate) — no separate commit was warranted for a 2-line whitespace-only change._

## Files Created/Modified
- `crates/trackly-app/src/pdf/renderer.rs` - Page-break loop in `render_docspec`; new `measure_device_card_height` helper; 3 new unit tests (`device_cards_paginate_when_exceeding_one_page`, `single_short_device_card_stays_on_one_page`, `device_card_never_split_across_page_boundary`)
- `crates/trackly-app/tests/pdf_render_act.rs` - New full-pipeline test `render_handover_multi_device_paginates_when_overflowing_one_page` (8 devices, all with 150+ char Cyrillic long fields); extended `render_handover_act_produces_cyrillic_pdf` with a 1-page assertion

## Decisions Made
- Header renders once, on page 1 only — matches the Word sample's letterhead structure (no repeated header on continuation pages), per the plan's explicit instruction
- Non-`DeviceCard` section variants use a cheap pre-draw bounds check (one line-height as the minimum advance) rather than full measure-then-place, since they're short fixed/near-fixed-height blocks — only `DeviceCard` needed the heavier measurement path
- `act_42.sha256` left untouched: verified via `cargo test --test pdf_determinism fixture_act_42_renders_to_known_hash` that the hash is unchanged (the pagination bounds check never fires for the single-device fixture, so renderer output bytes are byte-identical) — not assumed, actually run and confirmed

## Deviations from Plan

None — plan executed exactly as written. The only unplanned adjustment was a trivial `cargo fmt` formatting fix to two lines of test-helper code in `renderer.rs` (Rule 1 auto-fix, required to pass the `cargo fmt --check` gate specified in the plan's own verification block), folded into Task 2's commit.

## Issues Encountered

None of substance. `PageSettings` required an explicit `.clone()` at each `start_page_with(...)` call site (it derives `Clone`, not `Copy`) — a one-line compile fix, not a deviation from the plan's design.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Phase 15 (render-word-fidelity) is now fully complete: all 5 must-have truths from `15-VERIFICATION.md` are satisfied (the WR-05 pagination gap that scored the phase 4/5 is closed), and PDFA-02 ("N(2+) устройств печатают все позиции без обрезки/наложения текста") is now genuinely satisfied for realistic multi-device acts, not just the common 1-2 device case. Roadmap Success Criterion #2 is met.

Verification checklist for this plan:
- `cargo test -p trackly-app --lib pdf::renderer -- --test-threads=1` — 16/16 green (3 new pagination tests)
- `cargo test -p trackly-app --test pdf_render_act -- --test-threads=1` — 11/11 green (1 new multi-page test + extended N=1 assertion)
- `cargo test -p trackly-app --test pdf_determinism -- --test-threads=1` — 2/2 green, `act_42.sha256` unchanged
- `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app -- --test-threads=1` — full workspace-crate suite green, 0 failures
- `cargo clippy -p trackly-app -- -D warnings` — clean
- `cargo fmt --check` — clean

No blockers. Milestone v1.1.1 (PDF-акт по образцу Word) is ready for `/gsd-transition` / milestone close now that both Phase 14 and Phase 15 (with this gap-closure plan) are done.

---
*Phase: 15-render-word-fidelity*
*Completed: 2026-07-04*
