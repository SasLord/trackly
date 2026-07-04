---
phase: 15-render-word-fidelity
plan: 01
subsystem: pdf
tags: [krilla, ttf-parser, docspec, word-wrap, pdf-rendering, signature, header-layout]

# Dependency graph
requires:
  - phase: 14-data-and-act-structure
    provides: extended org_settings requisites (phone/fax/email/OKPO/OGRN), device specs/kit/condition, render-context plumbing
provides:
  - "render_header_two_column: reusable two-column header renderer (logo left, requisites right) replacing the previous inline draw_text sequence in render_docspec"
  - "wrap_text_to_width: real glyph advance-width word-wrap primitive on top of ttf-parser, for long device fields (consumed by Plan 15-02's per-device wrap-block)"
  - "Section::Signature extended with left_sublabel/right_sublabel (Option<String>, #[serde(default)]) — two-line 'Подпись'/'ФИО' signature rendering (D-07)"
  - "ttf-parser promoted from transitive to direct dependency of trackly-app, exact-pinned to 0.25.1"
affects: [15-02-template-and-multidevice, 15-03-tests-and-determinism-fixture]

# Tech tracking
tech-stack:
  added: [ttf-parser 0.25.1 (direct dependency, promoted from transitive via krilla->rustybuzz/skrifa)]
  patterns:
    - "Anchor-parametrized logo drawing (draw_logo_from_bytes_at/draw_logo_at_path take tx/ty) instead of hardcoded top-right/left position — reused by both the legacy call site (now removed) and the new two-column header"
    - "Real font-metrics word-wrap (ttf_parser::Face::glyph_hor_advance) kept strictly separate from the existing 0.5*font_size single-line truncate_to_width approximation — no shared code path, preserving the B-3 byte-identical invariant"
    - "Signature sub-label rendering only advances the y-cursor by an extra line when at least one sublabel is Some — old specs without sublabels produce byte-identical output to pre-Phase-15 rendering"

key-files:
  created: []
  modified:
    - crates/trackly-app/Cargo.toml
    - crates/trackly-app/src/pdf/docspec.rs
    - crates/trackly-app/src/pdf/renderer.rs

key-decisions:
  - "ttf-parser promoted to direct dependency via Task 0 human-verify checkpoint (approved by orchestrator pre-flight: crates.io publisher harfbuzz/RazrFalcon, MIT/Apache-2.0, github.com/harfbuzz/ttf-parser, already resolved transitively at 0.25.1 in Cargo.lock)"
  - "Section::Signature sublabels use plain #[serde(default)] + Option<String> idiom (defaulting to None), not the fn-default idiom used for spacer_pt, since absence must mean 'render old single-line layout', not 'render an empty string line'"
  - "2-column header grid stays fixed regardless of logo presence (no adaptive single-column fallback) — RESEARCH Open Question 3 resolution, simplest correct behavior"
  - "Empty requisite lines (phone/fax/email/OKPO+OGRN) are skipped entirely, not rendered as blank '—' placeholder lines — simplest correct degrade per D-08б"
  - "draw_logo_from_bytes/draw_logo_top_right renamed to draw_logo_from_bytes_at/draw_logo_at_path with explicit tx/ty params (private fns, no external call sites) so the same scale/decode/graceful-degrade logic serves both the old top-right anchor semantics and the new header left-column anchor"

patterns-established:
  - "Pattern: extracting a previously-inline hardcoded draw sequence into a standalone (surface, ..., y) -> f32 function matching render_section's signature shape, for reuse and independent unit testing"
  - "Pattern: dual-source text measurement in renderer.rs — truncate_to_width (0.5*font_size approximation, pinned for ItemsTable/report tables) vs wrap_text_to_width (real ttf-parser metrics, for long wrapping key-value fields) — never cross-call between the two"

requirements-completed: [PDFA-01, PDFA-02, PDFA-05, PDFA-07]

# Metrics
duration: 25min
completed: 2026-07-04
---

# Phase 15 Plan 01: Рендер-примитивы (два столбца шапки, перенос текста, двухстрочные подписи) Summary

**Три новых рендер-примитива в krilla-рендерере (`render_header_two_column`, `wrap_text_to_width` на базе `ttf-parser`, двухстрочный `Section::Signature`) — фундамент для соответствия образцу Word, без изменения шаблона или пайплайна акта.**

## Performance

- **Duration:** 25 min
- **Started:** 2026-07-04T05:50:19Z
- **Completed:** 2026-07-04T06:05:26Z
- **Tasks:** 2 (+ 1 pre-approved checkpoint)
- **Files modified:** 3 (Cargo.toml, docspec.rs, renderer.rs)

## Accomplishments

- `ttf-parser 0.25.1` промоутнут в прямую зависимость `trackly-app` (был транзитивным через `krilla`) — human-verify checkpoint пройден оркестратором заранее (легитимность пакета подтверждена: crates.io, harfbuzz/RazrFalcon, MIT/Apache-2.0).
- `Section::Signature` расширен опциональными `left_sublabel`/`right_sublabel` (`#[serde(default)]`, `Option<String>`) для двухстрочных подписей «Подпись»/«ФИО» под «Выдал»/«Получил» (D-07) — старые JSON/шаблоны без этих ключей продолжают рендериться однострочно, без изменений.
- Захардкоженная последовательность `draw_text`-вызовов шапки в `render_docspec` вынесена в отдельную функцию `render_header_two_column`: лого в фиксированной левой колонке (переиспользует существующую логику масштабирования/graceful-degradation, только сменён якорь позиционирования), реквизиты построчно в правой колонке с пропуском пустых строк (D-08б).
- Новый примитив `wrap_text_to_width` на основе `ttf_parser::Face::glyph_hor_advance` — точный перенос длинного текста по словам с реальными метриками шрифта (вместо приближения `0.5 * font_size`), без обрезки эллипсисом; корректно обрабатывает патологический случай (одно слово длиннее `max_width`) без паники и бесконечного цикла.
- 6 новых unit-тестов (`renderer.rs`) + 2 новых unit-теста (`docspec.rs`) — все зелёные; все существующие тесты `pdf::docspec`/`pdf::renderer` модулей, а также интеграционные `pdf_render_act.rs`/`pdf_column_overflow.rs`/`pdf_logo.rs` зелёные (никаких неожиданных регрессий).

## Task Commits

1. **Task 1: Add ttf-parser dependency + extend Section::Signature with two-line sublabels** - `7f01fb2` (feat)
2. **Task 2: Implement render_header_two_column, wrap_text_to_width, and two-line Signature render** - `04b11d3` (feat)

**Plan metadata:** (this commit — created after SUMMARY.md write)

## Files Created/Modified

- `crates/trackly-app/Cargo.toml` - added `ttf-parser = "0.25.1"` as a direct dependency
- `crates/trackly-app/src/pdf/docspec.rs` - extended `Section::Signature` with `left_sublabel`/`right_sublabel`; updated `sample_docspec()` and `section_enum_tagged_serde` test fixture construction sites; added 2 new tests
- `crates/trackly-app/src/pdf/renderer.rs` - added `render_header_two_column`, `wrap_text_to_width`, renamed `draw_logo_from_bytes`/`draw_logo_top_right` to anchor-parametrized `draw_logo_from_bytes_at`/`draw_logo_at_path`; extended the `Section::Signature` render arm; added 6 new unit tests

## Decisions Made

- Sub-labels default to `None` (not an empty-string default-fn), matching the existing `HeaderBlock.logo_bytes`/`logo_mime` idiom, since the absence of these fields must trigger the old single-line rendering path, not print an empty extra line.
- The 2-column header grid is unconditionally fixed (logo column always reserved, even if empty) rather than collapsing to single-column when no logo is present — matches RESEARCH's recommended resolution to Open Question 3, avoids adaptive-layout complexity not required by any PDFA-* requirement.
- Renamed the two logo-drawing helper functions (private, no external call sites) to take explicit `tx`/`ty` anchor parameters instead of hardcoding the top-right corner internally — this let the new header column reuse the exact same scale/decode/graceful-degrade logic without duplication, per RESEARCH's Don't-Hand-Roll guidance.

## Deviations from Plan

None - plan executed exactly as written. Task 0's checkpoint was pre-approved by the orchestrator per the `<checkpoint_note>` in the execution context (ttf-parser legitimacy already verified: crates.io registry, harfbuzz/RazrFalcon publisher, MIT/Apache-2.0 dual license, already-resolved transitive dependency at the exact pinned version).

## Issues Encountered

- `cargo fmt` reformatted two multi-line function signatures/assert calls in the new test code (cosmetic only, no logic change) — applied via `cargo fmt` before committing Task 2.
- `Face::from_slice` triggered a deprecation warning (ttf-parser 0.25.1 prefers `Face::parse`) in the test helper — fixed inline (Rule 1, trivial API rename, no behavior change) before `cargo clippy -D warnings` gate.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- `render_header_two_column`, `wrap_text_to_width`, and the extended `Section::Signature` are ready for Plan 15-02 to wire into the rewritten `act_handover.minijinja` template and the per-device hybrid wrap-block (D-06).
- `pdf_determinism.rs::fixture_act_42_renders_to_known_hash` now fails as expected (documented Pitfall 1 — hash drift from the changed header byte output) — regeneration of `act_42.sha256` is explicitly deferred to Plan 15-03 per the phase RESEARCH/plan verification notes, not a regression to fix here. `rendering_twice_yields_identical_bytes` (the actual determinism guarantee) still passes.
- No blockers for Plan 15-02.

---
*Phase: 15-render-word-fidelity*
*Completed: 2026-07-04*
