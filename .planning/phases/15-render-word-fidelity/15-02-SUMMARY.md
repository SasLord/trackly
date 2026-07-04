---
phase: 15-render-word-fidelity
plan: 02
subsystem: pdf
tags: [docspec, minijinja, act-template, device-card, word-wrap, logo-blob, template-service]

# Dependency graph
requires:
  - phase: 15-render-word-fidelity
    plan: 01
    provides: render_header_two_column, wrap_text_to_width, Section::Signature sublabels, ttf-parser direct dependency
provides:
  - "act_service.rs::render_pdf now propagates org_settings BLOB logo bytes into HeaderBlock (WR-03 closed)"
  - "Section::DeviceCard: hybrid per-device render section (compact identification + wrapped long fields), dispatched in render_section"
  - "act_handover.minijinja rewritten to D-09 block order/wording, D-06 hybrid per-device loop, D-07 signature sublabels"
  - "template_service.rs::validate_preview demo_ctx synced with new template variable schema (org requisites + item specs/kit/condition)"
affects: [15-03-tests-and-determinism-fixture]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "ttf-parser Face built once in render_docspec from the embedded regular font, threaded into render_section as an extra parameter — reused by Section::DeviceCard's long-field wrap calls (same primitive as Plan 15-01, no duplication)"
    - "Logo BLOB bytes bypass the MiniJinja JSON round-trip entirely: act_service.rs mutates the parsed DocSpec.header after serde_json::from_str, before render_docspec — templates never emit logo_bytes themselves"
    - "Template-side conditional inclusion (has_kit/has_specs/has_condition flags + explicit comma logic) used instead of MiniJinja list .append(), to keep long_fields JSON array emission simple and avoid relying on uncertain builtin-list-mutation semantics under the sandboxed safe_env"

key-files:
  created: []
  modified:
    - crates/trackly-app/src/services/act_service.rs
    - crates/trackly-app/src/pdf/docspec.rs
    - crates/trackly-app/src/pdf/renderer.rs
    - crates/trackly-app/templates/act_handover.minijinja
    - crates/trackly-app/src/services/template_service.rs

key-decisions:
  - "render_pdf's None branch (no org_db wired) explicitly returns (dto, None, None) for the 3-tuple — no behavior change for fixtures without org_db, matches D-02 degrade-to-blank contract from Phase 14"
  - "Section::DeviceCard's long_fields renderer performs NO empty-value filtering itself — that responsibility stays in the template (matches the existing conditional-injection idiom used for deadline/parent blocks); the renderer-level test for 'skip empty field' verifies the renderer doesn't crash/stray-render on an empty Vec, not that it filters non-empty-but-blank values"
  - "Title JSON field ('Акт приема-передачи №N') kept distinct from the D-09 heading section's literal text ('Акт приема-передачи', no number) — title/act_label retain the number-suffixed form for internal consistency (act list, filenames), only the visible heading section text matches the Word sample's bare phrase"
  - "act.giver_name is no longer displayed anywhere in the rendered body per D-09 (moved out in favor of receiver_name in the intro paragraph and bare 'Выдал' signature label) — this is an intentional, plan-directed behavior change, not a regression"

requirements-completed: [PDFA-01, PDFA-02, PDFA-05]

# Metrics
duration: 35min
completed: 2026-07-04
---

# Phase 15 Plan 02: Шаблон акта и мультиустройство по образцу Word Summary

**Дефолтный `act_handover.minijinja` переписан под точную структуру Word-образца (шапка/заголовок/вводная/подписи «Выдал-Получил» с сублейблами), плюс новая секция `Section::DeviceCard` для гибридного per-device блока и фикс WR-03 (BLOB-логотип из Settings UI теперь реально попадает в PDF).**

## Performance

- **Duration:** 35 min
- **Started:** 2026-07-04T06:09:56Z
- **Completed:** 2026-07-04T06:24:25Z
- **Tasks:** 3
- **Files modified:** 5 (act_service.rs, docspec.rs, renderer.rs, act_handover.minijinja, template_service.rs)

## Accomplishments

- **WR-03 закрыт:** `act_service.rs::render_pdf` больше не отбрасывает `logo_bytes`/`logo_mime` из `org_db.get_for_pdf()`. Байты BLOB-логотипа (загруженного через Settings UI) мутируются в уже распарсенный `DocSpec.header` до вызова `render_docspec` — это обходит JSON round-trip MiniJinja (шаблон не может разумно эмитить сырые байты как JSON). `org.json` `logo_path` остаётся fallback-источником, приоритет рендерера (`logo_bytes` побеждает) не тронут.
- **Новая секция `Section::DeviceCard`** (docspec.rs) — компактная идентификация устройства (Инв.№/Серийный №/Модель) + перенесённые по словам длинные поля (Комплектация/Тех.характеристики/Состояние) через `wrap_text_to_width` из Plan 15-01 (никогда не через `truncate_to_width`). `render_docspec` теперь строит `ttf_parser::Face` из встроенного regular-шрифта один раз и прокидывает его в `render_section` для доступа к реальным метрикам глифов.
- **`act_handover.minijinja` полностью переписан** под D-09 порядок блоков: шапка (лого + расширенные реквизиты) → заголовок «Акт приема-передачи» → номер/дата → вводная «Настоящим актом утверждаю, что мною {receiver_name} было получено устройство:» → цикл `device_card` по `act.items` (гибридный layout, D-06) → условный блок «Сроком до» → подписи «Выдал»/«Получил» с сублейблами «Подпись»/«ФИО» (D-07).
- **`template_service.rs::validate_preview`'s `demo_ctx` синхронизирован** в том же коммите — добавлены `org.{phone,fax,email,okpo,ogrn}` и `act.items[0].{specs,kit,condition}` — закрывает регрессию Pitfall 3 («undefined value» при рендере превью).
- 3 новых unit-теста в `renderer.rs` (`device_card_renders_identification_and_wrapped_long_fields`, `device_card_skips_empty_long_field`, `two_device_cards_do_not_overlap`) + 1 новый unit-тест в `docspec.rs` (расширение `section_enum_tagged_serde`) — все зелёные. Существующие `template_service` тесты (`validate_preview_returns_pdf_bytes`, `validate_preview_act_acceptance_returns_pdf_bytes`) остаются зелёными после синхронизации `demo_ctx`.

## Task Commits

1. **Task 1: Fix WR-03 logo-BLOB plumbing in act_service.rs render_pdf** - `0161621` (fix)
2. **Task 2: Add hybrid per-device wrap-block dispatch to render_section** - `a44d455` (feat)
3. **Task 3: Rewrite act_handover.minijinja + sync validate_preview demo_ctx** - `6ad0202` (feat)

## Files Created/Modified

- `crates/trackly-app/src/services/act_service.rs` - `render_pdf` now destructures `(dto, logo_bytes, logo_mime)` from `get_for_pdf()` (no discarded bindings), mutates parsed `DocSpec.header.logo_bytes`/`logo_mime` before `render_docspec`
- `crates/trackly-app/src/pdf/docspec.rs` - new `Section::DeviceCard { heading, identification: Vec<KvRow>, long_fields: Vec<KvRow> }` tagged-enum variant; +1 test assertion in `section_enum_tagged_serde`
- `crates/trackly-app/src/pdf/renderer.rs` - `render_docspec` builds a `ttf_parser::Face` from the embedded regular font and threads it into `render_section`; new `DeviceCard` match arm (bold heading, `KeyValueTable`-style identification rows, wrapped long fields at full content width); +3 unit tests
- `crates/trackly-app/templates/act_handover.minijinja` - full rewrite: D-09 block order/wording, D-06 per-device `device_card` loop with conditional long-field inclusion, D-07 two-line signature sublabels, extended header doc-comment
- `crates/trackly-app/src/services/template_service.rs` - `validate_preview`'s `demo_ctx` extended with `org.{phone,fax,email,okpo,ogrn}` and `act.items[0].{specs,kit,condition}`

## Decisions Made

- `None` branch of the `org_db` match in `render_pdf` explicitly returns `(dto, None, None)` — no behavioral change for test fixtures/paths without `org_db` wired.
- `Section::DeviceCard`'s renderer does not filter empty long-field values itself; the template is the single source of truth for which long fields get emitted (matches the codebase's existing conditional-injection idiom for deadline/parent blocks) — keeps the renderer's contract simple and symmetric with `KeyValueTable`.
- Template's `long_fields` JSON array is built via explicit `has_kit`/`has_specs`/`has_condition` boolean flags + manual comma logic instead of MiniJinja's `list.append()` — avoids relying on uncertain builtin list-mutation semantics inside the sandboxed `build_safe_env()` (Strict undefined, fuel-capped) environment; simpler to reason about and matches the existing template's manual-comma style used for the items_table loop.
- `act.giver_name` is intentionally no longer rendered anywhere in the act body (D-09 replaces the old «Сдал: {giver_name}» / «Принял: {receiver_name}» key-value rows with the bare «Выдал»/«Получил» signature labels and moves `receiver_name` into the intro paragraph) — a deliberate, plan-directed content change, not a regression.

## Deviations from Plan

None — plan executed exactly as written. All three tasks' acceptance criteria (grep source assertions, `cargo build`/`clippy -D warnings`/targeted `cargo test`) pass as specified.

## Known Test Drift (expected, deferred to Plan 15-03)

Per the plan's own `<verification>` note ("full-pipeline integration tests ... are extended/regenerated in Plan 15-03 — do not attempt to make them pass in this plan"):

- `crates/trackly-app/tests/pdf_render_act.rs::render_handover_act_produces_cyrillic_pdf` now fails — it asserts the old giver_name (`Сидоров-Петроградский`) appears in the rendered PDF body text, but D-09's rewrite intentionally removed giver_name display from the body (moved to bare "Выдал" signature label; receiver_name now appears in the intro paragraph instead). This is the same category of expected drift as `pdf_determinism.rs::fixture_act_42_renders_to_known_hash` (already failing since Plan 15-01, per that plan's Summary) — both are explicitly scoped to Plan 15-03's fixture/assertion regeneration, not a regression introduced here.
- All other full-pipeline integration tests (`pdf_column_overflow.rs`, `pdf_logo.rs`, `pdf_logo_aspect.rs`, `pdf_text_extract.rs`, and the remaining 6/7 tests in `pdf_render_act.rs`) pass unchanged.

This plan's own required verification gate — `cargo test -p trackly-app --lib pdf::docspec pdf::renderer services::template_service` — is fully green, matching the plan's `<verification>` section exactly.

## Issues Encountered

None beyond the expected/documented integration-test drift above. No auto-fixes (Rules 1-3) were needed beyond what the plan's `<action>` blocks already specified.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- Plan 15-03 must: regenerate `pdf_determinism.rs`'s `act_42.sha256` fixture (header layout changed in 15-01), update `pdf_render_act.rs::render_handover_act_produces_cyrillic_pdf`'s assertion to match the new D-09 body wording (receiver_name in intro paragraph instead of giver_name in a key-value row), and extend/verify multi-device (`device_card` loop) coverage in the full-pipeline integration suite.
- `Section::DeviceCard`, the extended `HeaderBlock` requisites, and the rewritten template are all wired end-to-end and ready for Plan 15-03's test/fixture work — no further renderer or template plumbing is expected to be needed.
- No blockers.

---
*Phase: 15-render-word-fidelity*
*Completed: 2026-07-04*

## Self-Check: PASSED

All created/modified files verified present on disk; all 4 commit hashes (0161621, a44d455, 6ad0202, acad13b) verified present in git log.
