---
quick_id: 260704-wxw
slug: act-pdf-word-fidelity-redesign
phase: 260704-wxw
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - crates/trackly-app/src/pdf/docspec.rs
  - crates/trackly-app/src/pdf/renderer.rs
  - crates/trackly-app/templates/act_handover.minijinja
  - crates/trackly-app/tests/pdf_render_act.rs
  - crates/trackly-app/tests/pdf_column_overflow.rs
  - crates/trackly-app/tests/fixtures/act_42.sha256
autonomous: true
requirements: [WXW-01, WXW-02, WXW-03, WXW-04]
must_haves:
  truths:
    - "Section::FieldRow renders a label on the left and a word-wrapped value on the right, with a thin underline drawn under the value"
    - "The default act_handover.minijinja output contains zero occurrences of the substring Устройство № for any number of items"
    - "The rendered act body contains the full-length labels Инвентарный номер:, Серийный номер:, Модель:, Комплектация:, Технические характеристики:, Состояние:, Сроком до: (each only when its source value is non-empty) - never the abbreviated forms Инв.№/Серийный №"
    - "A handover act with 2+ devices renders both device names, each device's own field-row block, in item order, with no numbering/heading between devices"
    - "Pagination still works: a FieldRow-based act whose content exceeds one page's usable height renders 2+ pages (pdf_extract::extract_text_from_mem_by_pages), and no realistic FieldRow's label/value pair is split across a page boundary"
    - "Section::DeviceCard variant and its renderer/measurement code remain present and covered by existing tests (backward compatibility for any template that still emits it)"
    - "tests/fixtures/act_42.sha256 matches the actual SHA256 of rendering tests/fixtures/act_42.json through the current renderer (act_42.json uses only KeyValueTable/ItemsTable/Signature - unaffected by additive FieldRow changes; verify, regenerate only if drift is actually observed)"
  artifacts:
    - path: "crates/trackly-app/src/pdf/docspec.rs"
      provides: "Section::FieldRow { label: String, value: String } variant, serde tag field_row"
      contains: "FieldRow"
    - path: "crates/trackly-app/src/pdf/renderer.rs"
      provides: "FieldRow draw-arm with label/value columns, underline, wrap-aware pagination measurement"
      contains: "Section::FieldRow"
    - path: "crates/trackly-app/templates/act_handover.minijinja"
      provides: "Rewritten default act_handover template emitting field_row sections instead of device_card"
      contains: "field_row"
  key_links:
    - from: "crates/trackly-app/templates/act_handover.minijinja"
      to: "crates/trackly-app/src/pdf/docspec.rs::Section::FieldRow"
      via: "MiniJinja JSON emission deserialized by serde into DocSpec"
      pattern: "type.*field_row"
    - from: "crates/trackly-app/src/pdf/renderer.rs::render_docspec pagination loop"
      to: "crates/trackly-app/src/pdf/renderer.rs::render_section Section::FieldRow arm"
      via: "pre-draw bounds check using a measured FieldRow height (wrap-aware), consistent with the existing DeviceCard measure-then-place pattern"
      pattern: "measure_field_row_height|FieldRow"
---

<objective>
Rewrite the default act_handover PDF template and add a Section::FieldRow DocSpec/renderer variant so the rendered Акт приёма-передачи PDF matches the structure of the Word reference sample (исходный образец не хранится в репозитории): body content as "метка | значение" rows on a thin underline (not device_card boxes), full-length field labels, no "Устройство №N" headings between devices, and pagination preserved.

Purpose: The current default template renders each device as a device_card block with a "Устройство №N: {name}" heading and abbreviated labels (Инв.№/Серийный №). This does not match the Word original the organization actually uses, which lists label/underlined-value rows with full labels and no per-device heading/counter.

Output: Section::FieldRow variant in docspec.rs plus a krilla draw-arm in renderer.rs (with underline + wrap + pagination-safe measurement), a fully rewritten act_handover.minijinja, updated tests in pdf_render_act.rs and pdf_column_overflow.rs asserting the new structure, and a verified act_42.sha256.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md
@./CLAUDE.md
@crates/trackly-app/src/pdf/docspec.rs
@crates/trackly-app/src/pdf/renderer.rs
@crates/trackly-app/templates/act_handover.minijinja
@crates/trackly-app/src/services/act_service.rs
@crates/trackly-app/tests/pdf_render_act.rs
@crates/trackly-app/tests/pdf_column_overflow.rs
@crates/trackly-app/tests/pdf_determinism.rs
@crates/trackly-app/tests/fixtures/act_42.sha256
@crates/trackly-app/tests/fixtures/act_42.json
</context>

<interfaces>
<!-- Existing DocSpec/Section contract the executor works against. No codebase
     exploration needed for these shapes -- copied verbatim from the repo. -->

From crates/trackly-app/src/pdf/docspec.rs (Section enum, serde tag = "type", rename_all = "snake_case"):

Section variants today: Paragraph { text, style }, Heading { level, text }, KeyValueTable { rows: Vec<KvRow> }, ItemsTable { columns: Vec<String>, rows: Vec<Vec<String>> }, Signature { left_label, right_label, spacer_pt, left_sublabel: Option<String>, right_sublabel: Option<String> }, Spacer { height_pt }, DeviceCard { heading, identification: Vec<KvRow>, long_fields: Vec<KvRow> } (KEEP as-is, do not remove or modify). KvRow is `{ key: String, value: String }`.

Add a new variant `FieldRow { label: String, value: String }` -- reuse the plain always-present-String idiom (no Option/sublabel wrapping needed; a template that wants to omit a row simply doesn't emit that JSON array element, same idiom already used for DeviceCard.long_fields).

From crates/trackly-app/src/pdf/renderer.rs (constants/helpers the FieldRow arm reuses -- do not redefine, do not change values):

- `const A4_WIDTH_PT: f32 = 595.276;`
- `const A4_HEIGHT_PT: f32 = 841.890;`
- `const MARGIN_PT: f32 = 50.0;`
- `const BODY_SIZE_PT: f32 = 10.0;`
- `const HEADING_SIZE_PT: f32 = 14.0;`
- `pub fn wrap_text_to_width(face: &Face, text: &str, font_size: f32, max_width: f32) -> Vec<String>`
- `fn measure_device_card_height(section: &Section, regular_face: &Face) -> f32` -- pattern to mirror for `measure_field_row_height`
- `fn render_section(surface, section: &Section, font_regular: &Font, font_bold: &Font, regular_face: &Face, mut y: f32) -> f32`

render_docspec's per-section pagination loop (inside the `for section in &spec.sections` block) currently special-cases only Section::DeviceCard for measure-then-place; every other variant uses a cheap `min_advance = BODY_SIZE_PT + 4.0` pre-draw bounds check. FieldRow values can wrap to multiple lines (like DeviceCard.long_fields), so FieldRow must join the DeviceCard measure-then-place branch (not the cheap single-line branch) -- otherwise a wrapped multi-line FieldRow value could be split mid-value across a page boundary.

krilla line-drawing: no existing call in this file draws a line/rectangle (only draw_text and draw_image are used). Resolve the correct krilla 0.7 API for a simple filled thin rectangle or stroked line (check krilla::path::PathBuilder plus surface.fill_path(&path, paint, fill_rule) or surface.stroke_path -- inspect the krilla 0.7 docs/source, e.g. `cargo doc -p krilla --open` or grep the vendored krilla-0.7 source under the cargo registry for fill_path/stroke_path/PathBuilder signatures) before implementing the underline draw call.
</interfaces>

<tasks>

<task type="auto">
  <name>Task 1: Add Section::FieldRow to DocSpec (contract-first)</name>
  <files>crates/trackly-app/src/pdf/docspec.rs</files>
  <action>
    Add a new Section::FieldRow { label: String, value: String } variant to the Section enum (serde tag "field_row" via the existing #[serde(tag = "type", rename_all = "snake_case")] discriminator -- no extra attributes needed, both fields are always-present plain Strings, mirroring the simplest existing variant shape like Heading). Do not modify Section::DeviceCard or any other existing variant -- this is purely additive.

    Add a one-sentence doc comment above the new variant (matching the existing per-variant doc style) explaining it is the label/underlined-value row used by the Word-fidelity act body redesign, and that empty-value rows are the template's responsibility to omit before emission (same idiom as DeviceCard.long_fields).

    Extend the #[cfg(test)] module with two new focused unit tests (do not touch sample_docspec() or any existing test): one asserting serde_json::to_value(&Section::FieldRow{label, value})["type"] == "field_row" with both fields present in the JSON; one asserting full round-trip (serde_json::to_string -> serde_json::from_str -> assert_eq!) for a FieldRow containing Cyrillic label/value text (mirrors the existing signature_sublabels_round_trip_when_present pattern).
  </action>
  <verify>
    <automated>cargo test -p trackly-app --lib pdf::docspec::tests</automated>
  </verify>
  <done>New Section::FieldRow variant compiles, serializes to {"type":"field_row","label":"...","value":"..."}, round-trips through serde with Cyrillic content, and all pre-existing docspec tests still pass unmodified.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: FieldRow draw-arm with underline + pagination-safe measurement</name>
  <files>crates/trackly-app/src/pdf/renderer.rs</files>
  <behavior>
    - render_section's new Section::FieldRow arm draws label (left column, width approx 42% of A4_WIDTH_PT - 2*MARGIN_PT) and value (right column, width approx 58% of the same usable width), value word-wrapped via wrap_text_to_width at BODY_SIZE_PT, with a thin horizontal line drawn immediately under the LAST wrapped line of value only (not under every line, not under the label) -- this is the underline the value sits on, matching the Word sample's fill-in-the-blank look.
    - A short label + short value advances the y-cursor by exactly one line-height plus trailing padding (no wasted vertical space).
    - A long value that wraps to N lines advances the y-cursor by N line-heights; the underline is drawn exactly once, positioned under the last wrapped line.
    - render_docspec's pagination loop treats Section::FieldRow the same way it already treats Section::DeviceCard: measured via a new measure_field_row_height(section: &Section, regular_face: &Face) -> f32 helper (mirrors measure_device_card_height's exact wrap arithmetic -- line count from wrap_text_to_width on the value at the FieldRow's value-column width, BODY_SIZE_PT + 4.0-ish per line, small fixed padding) BEFORE drawing, so a wrapped FieldRow is never split across a page boundary for realistic field values.
    - Rendering the same FieldRow-containing DocSpec twice produces byte-identical output (no new non-determinism introduced by the underline draw call or the measurement helper).
  </behavior>
  <action>
    First resolve the krilla 0.7 API for drawing a simple thin filled rectangle or stroked line (see interfaces above for where to look -- there is no existing precedent in this file). Implement the underline as a thin (approx 0.5-0.75pt) horizontal shape spanning the value column's width, positioned approx 2pt below the baseline of the last wrapped value line.

    Add the Section::FieldRow { label, value } arm to render_section, alongside the existing Section::DeviceCard arm, implementing the behavior above. Compute the label/value column x-offsets and widths as percentages of A4_WIDTH_PT - 2.0 * MARGIN_PT (42%/58% split) -- do not introduce new named point-constants for the split itself since it is percentage-based per the target structure, but do reuse MARGIN_PT/A4_WIDTH_PT for the usable-width computation exactly as Section::ItemsTable's usable_width local does.

    Add measure_field_row_height as a sibling function to measure_device_card_height. Extend render_docspec's pagination loop so the measure-then-place branch also covers Section::FieldRow (e.g. change the existing `if let Section::DeviceCard { .. } = section` check to `if matches!(section, Section::DeviceCard { .. } | Section::FieldRow { .. })`, then internally dispatch to measure_device_card_height or measure_field_row_height depending on which variant it actually is). Do not alter Section::DeviceCard's own measurement or drawing logic.

    Add 4 new #[cfg(test)] tests in this file's existing mod tests block, following the exact style of the existing device_card_* / two_device_cards_do_not_overlap / device_cards_paginate_when_exceeding_one_page tests:
    - field_row_renders_label_and_short_value: a single short FieldRow renders both label and value text in extracted PDF text.
    - field_row_wraps_long_value_without_truncating: a FieldRow with a 150+ char Cyrillic value wraps across multiple lines (no ellipsis anywhere in extracted text) and a middle-of-value marker substring survives.
    - two_field_rows_render_in_order_without_overlap: two FieldRow sections render with the first section's label appearing before the second section's label in extracted text (mirrors two_device_cards_do_not_overlap).
    - field_rows_paginate_when_exceeding_one_page: enough FieldRow sections with long values force pdf_extract::extract_text_from_mem_by_pages to return 2+ pages (mirrors device_cards_paginate_when_exceeding_one_page).
  </action>
  <verify>
    <automated>cargo test -p trackly-app --lib pdf::renderer::tests</automated>
  </verify>
  <done>FieldRow renders label/value with a visible underline under the value only, wraps long values across multiple lines without truncation, participates in the same measure-then-place pagination guard as DeviceCard, two consecutive renders of a FieldRow-containing DocSpec are byte-identical, and all pre-existing renderer tests (DeviceCard, Signature, header, wrap_text_to_width) still pass unmodified.</done>
</task>

<task type="auto">
  <name>Task 3: Rewrite act_handover.minijinja to emit field_row instead of device_card</name>
  <files>crates/trackly-app/templates/act_handover.minijinja</files>
  <action>
    Fully rewrite the template body per the TARGET STRUCTURE:

    1. Keep the existing header JSON block (org_name/org_inn/org_kpp/org_address/logo_path/org_phone/org_fax/org_email/org_okpo/org_ogrn/act_label/date_label) completely unchanged -- the two-column header renderer already consumes all these fields; no header changes are in scope here.
    2. Keep the centered bold "Акт приема-передачи" heading section and the "№ {{ act.number }}{{ suffix }} от {{ date_human }}" paragraph section unchanged.
    3. Replace the intro-paragraph-plus-device_card-loop section list with field_row sections, in this order:
       - One field_row: label = "Настоящим актом утверждаю, что мною:", value = act.receiver_name -- emitted once, before the items loop (replaces the old single paragraph that concatenated receiver_name into running prose).
       - For each item in act.items, in order, with NO "Устройство №{{loop.index}}" heading and no per-device counter of any kind: emit field_row sections for, in this exact order -- "было получено устройство:" -> item.name (always emitted, no guard); "Инвентарный номер:" -> item.inventory_no (guarded, only emit when truthy, same {%- set has_x = item.x -%} / {%- if has_x %} pattern the old template used for long_fields); "Серийный номер:" -> item.serial_no (same guard pattern); "Модель:" -> item.model (same guard pattern); "Комплектация:" -> item.kit (same guard pattern); "Технические характеристики:" -> item.specs (same guard pattern); "Состояние:" -> item.condition (same guard pattern). Handle comma placement between conditionally-emitted JSON array elements exactly the way the old template chained the trailing-comma-only-if-more-follows logic between long_fields entries -- every field_row (guarded or not) needs a trailing comma UNLESS it is the last emitted element in the overall sections array for that device's block. After each device's full field_row block, emit one spacer section (height_pt 8.0) before the next device begins (omit the trailing spacer after the very last device -- the deadline/parent/signature blocks that follow already carry their own leading spacers).
       - After the items loop: keep the existing if act.deadline_human / elif act.deadline / endif structure, but change the emitted section from a paragraph with concatenated text to a field_row with label = "Сроком до:" and value = act.deadline_human (or act.deadline in the elif branch). Omit entirely if neither is set (unchanged from current behavior).
       - Preserve the existing act.parent paragraph block completely unchanged (return-act parent reference stays a paragraph, out of scope for this redesign).
    4. Remove all device_card JSON emission (the heading/identification/long_fields per-item block) entirely -- replaced by the field_row sequence above.
    5. Keep the trailing Signature section (Выдал/Получил with Подпись/ФИО sublabels) completely unchanged.
    6. Update the top-of-file doc comment's "Available context variables" list only if field usage changed shape (it has not -- same act.items[].*/act.deadline*/act.receiver_name fields, just emitted as field_row instead of device_card/paragraph); otherwise leave the comment as-is. Do add one sentence noting the body now emits field_row sections for this Word-fidelity redesign, not device_card.
    7. Follow the existing Pitfall-6 idiom throughout: every optional field uses an {% if %} truthy-guard so an empty source value never emits an empty-but-present field_row -- omit the row's JSON array element entirely when the source value is empty, exactly like the old template's has_kit/has_specs/has_condition gates for long_fields.
  </action>
  <verify>
    <automated>cargo build -p trackly-app</automated>
  </verify>
  <done>act_handover.minijinja contains zero device_card emissions and zero "Устройство №" text, emits field_row sections for the intro line, each device's fields (only non-empty ones, full-length labels), and the deadline line; the template still parses as valid MiniJinja (cargo build succeeds, template is embedded via include_str! at compile time).</done>
</task>

<task type="auto">
  <name>Task 4: Update pdf_render_act.rs and pdf_column_overflow.rs for the new structure, verify act_42.sha256</name>
  <files>crates/trackly-app/tests/pdf_render_act.rs, crates/trackly-app/tests/pdf_column_overflow.rs, crates/trackly-app/tests/fixtures/act_42.sha256</files>
  <action>
    Update pdf_render_act.rs's full-pipeline tests that exercise the real act_handover template (all tests in this file call p.acts.render_pdf(act.id), which renders through the template rewritten in Task 3):
    - render_handover_act_produces_cyrillic_pdf: no structural change needed (it asserts "Петров"/act number/single-page count, none of which reference device_card wording) -- re-run and confirm it still passes as-is; if the 2-device single-page assumption changes due to FieldRow's different vertical footprint per device, adjust only the page-count expectation, not the Cyrillic/number assertions.
    - render_handover_act_contains_d09_intro_phrase: unaffected (asserts "Настоящим актом утверждаю" + receiver_name presence -- the field_row intro row preserves this exact phrase per Task 3). Re-run to confirm.
    - signature_renders_two_line_labels: unaffected (signature block untouched). Re-run to confirm.
    - render_handover_multi_device_wraps_long_fields: update the doc-comment above it if it references "DeviceCard wrap path" -- rephrase to "FieldRow wrap path" -- but keep the assertions (all 5 device names present, no ellipsis, middle-of-value marker survives) unchanged; they remain valid for FieldRow's wrap behavior.
    - render_handover_multi_device_paginates_when_overflowing_one_page: keep assertions (2+ pages, no data loss across pages) -- re-run and confirm still passes; FieldRow's per-device vertical footprint differs from DeviceCard's, so if 8 devices no longer force 2+ pages, increase the device count or the per-item long-value length in this test's fixture data until pagination is again exercised (do not weaken the assertion itself).
    - Add ONE new full-pipeline test, e.g. render_handover_default_template_uses_field_rows_not_device_card: seed 2+ devices, create a handover act, set non-empty inventory_no/serial_no/model values on the resulting act_items rows directly via a writer UPDATE (mirroring the complectation_at_time UPDATE idiom already used in this file, since seed_devices alone does not populate these fields), render via p.acts.render_pdf, extract text, and assert: full-length labels "Инвентарный номер:", "Серийный номер:", "Модель:" are present; absence of the substrings "Устройство №1" and "Устройство №2"; absence of the abbreviated legacy labels "Инв.№" and "Серийный №"; both seeded device names appear in item order (first device's name index less than second device's name index in the extracted text).

    Check pdf_column_overflow.rs's device_card_long_field_wraps_instead_of_truncating test (full-pipeline, renders through the real template): its assertions (no ellipsis, middle-of-value marker present) remain valid under FieldRow's wrap behavior -- re-run to confirm passes unmodified; if it fails, update only the doc-comment above it (it currently references "the new Section::DeviceCard long-field wrap-blocks" -- rephrase to reference FieldRow) without weakening the assertions. Do NOT touch this file's ItemsTable-truncation tests (truncate_to_width_*, long_name_truncated_does_not_overlap_inv_no) -- they exercise an unrelated, untouched code path.

    Verify act_42.sha256: run pdf_determinism::fixture_act_42_renders_to_known_hash (see verify command below). act_42.json (the fixture input) uses only KeyValueTable/ItemsTable/Signature/Heading/Spacer sections -- none of which this plan's Task 1/2 changes touch (FieldRow is purely additive; no existing match arm's output changes). Expect this test to PASS UNCHANGED. If it unexpectedly fails (hash drift), regenerate by taking the actual printed hash from the test failure message, updating tests/fixtures/act_42.sha256 to that value, and documenting in the plan SUMMARY why drift occurred (it should not, given the additive nature of the change) -- do not regenerate speculatively without an observed failure.
  </action>
  <verify>
    <automated>TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test pdf_render_act -- --test-threads=1</automated>
    <automated>TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test pdf_column_overflow -- --test-threads=1</automated>
    <automated>cargo test -p trackly-app --test pdf_determinism</automated>
  </verify>
  <done>All full-pipeline tests in pdf_render_act.rs pass against the rewritten template (no "Устройство №N", full-length labels present, device order preserved, multi-device pagination still exercised); pdf_column_overflow.rs's DeviceCard-wrap test still passes (doc-comment updated if needed, assertions unchanged); pdf_determinism.rs's act_42 hash test passes -- either unchanged (expected) or regenerated with a documented reason (only if actual drift observed).</done>
</task>

<task type="auto">
  <name>Task 5: Final verification gate</name>
  <files>(no new files -- verification only)</files>
  <action>
    Run the full crate test suite and lint/format gates, one command at a time (project rule: one cargo test invocation at a time -- target/ lock contention otherwise looks like a multi-minute hang). Fix any regressions surfaced (e.g. other tests incidentally asserting old device_card wording that were not enumerated in Task 4 -- grep the full crates/trackly-app/tests/ tree for "Устройство №", "device_card", "Инв.№", "Серийный №" as a final sweep before declaring done, and update any stragglers found using the same pattern as Task 4).
  </action>
  <verify>
    <automated>TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app</automated>
  </verify>
  <done>Full trackly-app test suite green, cargo clippy -p trackly-app -- -D warnings clean, cargo fmt --check clean, and a final grep of crates/trackly-app/tests/ for the old-format strings in default-act_handover-path tests returns no unexpected matches (DeviceCard's own dedicated unit tests in renderer.rs/docspec.rs are expected and fine -- they test the variant's continued backward-compat existence, not the default template).</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| org data -> PDF template context | Org requisites and act/device fields (user-entered via Settings/Devices UI) flow into MiniJinja context and then into rendered PDF text -- same trust boundary as the pre-existing template pipeline, unchanged by this plan. |
| MiniJinja template -> DocSpec JSON -> krilla | Template output is deserialized into the fully-typed DocSpec/Section enum (no raw PDF ops) before krilla renders it -- this plan preserves that invariant by adding FieldRow as another fully-typed variant, not a raw/free-form field. |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-WXW-01 | Tampering | act_handover.minijinja template body | accept | Template body is either the bundled compile-time default (include_str!) or an admin-only update_body write (already RBAC-gated elsewhere, unchanged by this plan) -- not user/employee-writable. |
| T-WXW-02 | Information Disclosure | FieldRow value rendering (item.specs/kit/condition free text) | accept | Same free-text fields already rendered pre-existing via DeviceCard.long_fields -- no new data exposed, only re-labeled/re-laid-out; org-internal act content, not exposed outside the LAN app. |
| T-WXW-03 | Denial of Service | wrap_text_to_width / measure_field_row_height on attacker-controlled long values | accept | wrap_text_to_width already has a pathological-single-word-does-not-loop guard (existing, reused unchanged); FieldRow values come from the same DB-stored device/act fields as DeviceCard's long_fields today, no new unbounded-input surface introduced. |
| T-WXW-SC | Tampering | No new package-manager installs in this plan (pure Rust/template changes to existing deps) | accept | No new crate dependencies added -- krilla/serde/minijinja already vetted in prior phases; no Package Legitimacy Gate applicable. |
</threat_model>

<verification>
Run, one at a time (project rule -- never run two cargo test invocations concurrently):

1. `cargo test -p trackly-app --lib pdf::docspec::tests` -- FieldRow variant unit tests + all pre-existing docspec tests.
2. `cargo test -p trackly-app --lib pdf::renderer::tests` -- FieldRow draw-arm + pagination + determinism unit tests, all pre-existing renderer tests (DeviceCard/Signature/header/wrap).
3. `cargo build -p trackly-app` -- confirms act_handover.minijinja (embedded via include_str!) is syntactically consistent with the rest of the crate.
4. `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test pdf_render_act -- --test-threads=1` -- full-pipeline template rewrite coverage.
5. `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test pdf_column_overflow -- --test-threads=1` -- DeviceCard-wrap backward-compat + ItemsTable-truncation regression guard.
6. `cargo test -p trackly-app --test pdf_determinism` -- act_42.sha256 pinned-hash check.
7. `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app` -- full crate suite, no regressions anywhere else (e.g. acts_e2e_smoke.rs, pdf_logo.rs, templates_seed.rs).
8. `cargo clippy -p trackly-app -- -D warnings` -- lint gate.
9. `cargo fmt --check` -- format gate.
</verification>

<success_criteria>
- Section::FieldRow { label, value } exists in docspec.rs, is fully typed (no raw PDF ops), serializes/deserializes with tag field_row, and has dedicated unit test coverage.
- render_section's FieldRow arm draws a two-column label/value row with the value word-wrapped and underlined (thin line under the last wrapped line only); render_docspec's pagination loop measures FieldRow via a new measure_field_row_height before drawing, preventing mid-value page splits for realistic content.
- Section::DeviceCard and all its existing tests remain present and passing -- no removal, no behavior change.
- act_handover.minijinja no longer emits device_card or any "Устройство №N" text; it emits field_row sections with full-length labels (Инвентарный номер:/Серийный номер:/Модель:/Комплектация:/Технические характеристики:/Состояние:/Сроком до:), each guarded to omit empty values, for the intro line and every device in act.items in order, with no per-device heading or counter.
- All full-pipeline tests in pdf_render_act.rs and the DeviceCard-wrap test in pdf_column_overflow.rs pass against the rewritten template; a new test explicitly proves the absence of "Устройство №1"/"Устройство №2" and the abbreviated legacy labels, and the presence of the full-length labels, in a 2+ device act.
- tests/fixtures/act_42.sha256 matches the actual hash of the pinned fixture (unchanged, since act_42.json never touches FieldRow/DeviceCard) -- or, if genuine drift is observed, is regenerated with a documented reason.
- cargo test -p trackly-app (one invocation at a time), cargo clippy -p trackly-app -- -D warnings, and cargo fmt --check all exit 0.
</success_criteria>

<output>
Create `.planning/quick/260704-wxw-act-pdf-word-fidelity-redesign/260704-wxw-SUMMARY.md` when done.
</output>
