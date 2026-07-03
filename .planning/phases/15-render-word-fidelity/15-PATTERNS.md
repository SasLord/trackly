# Phase 15: Рендер и соответствие образцу - Pattern Map

**Mapped:** 2026-07-04
**Files analyzed:** 9 (2 modified core structs, 1 modified renderer, 1 rewritten template, 2 modified services, 4 test files extended)
**Analogs found:** 9 / 9 — this phase is entirely self-referential: every file to
touch already exists in the repo, and the closest analog for each is almost
always *itself* (extend an existing pattern in-place) rather than a different
file. Noted explicitly per file below.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|--------------------|------|-----------|-----------------|----------------|
| `crates/trackly-app/src/pdf/docspec.rs` | model (serde IR) | transform (JSON→typed IR) | itself — `HeaderBlock` Phase-14 extension (same file, lines 33-66) | exact (self-analog, proven backward-compat pattern) |
| `crates/trackly-app/src/pdf/renderer.rs` | service (pure rendering) | transform (IR→PDF bytes) | itself — existing `render_section`/logo functions (same file) | exact (self-analog; new helpers follow existing function shape) |
| `crates/trackly-app/templates/act_handover.minijinja` | config/template (MiniJinja→JSON) | transform | itself (full rewrite) + `act_acceptance.minijinja` (sibling template, same DocSpec contract) | exact (self) / role-match (sibling) |
| `crates/trackly-app/src/services/template_service.rs` | service | CRUD (seed) + transform (demo_ctx) | itself — `validate_preview`'s `demo_ctx` (same file, lines 253-343) | exact (self) |
| `crates/trackly-app/src/services/act_service.rs` | service | request-response (context assembly) | itself — `render_pdf` (same file, lines 1342-1457) + `render_acceptance_pdf` (lines 1463+) as secondary reference | exact (self) |
| `crates/trackly-app/tests/pdf_render_act.rs` | test (integration) | request-response assertions | itself — existing tests in same file | exact (self) |
| `crates/trackly-app/tests/pdf_column_overflow.rs` | test (integration) | transform assertions | itself + `pdf_render_act.rs`'s pdf_extract pattern | exact (self) |
| `crates/trackly-app/tests/pdf_logo.rs` | test (integration) | transform assertions | itself + `render_pdf_with_null_specs_and_empty_requisites_succeeds` in `pdf_render_act.rs` (full-pipeline pattern via `act_service::render_pdf`, not `render_docspec` direct) | role-match (need to switch from direct-renderer tests to full-pipeline tests) |
| `crates/trackly-app/tests/pdf_determinism.rs` + `tests/fixtures/act_42.{json,sha256}` | test fixture (regression) | batch (hash pin) | itself | exact (self — regenerate only) |

## Pattern Assignments

### `crates/trackly-app/src/pdf/docspec.rs` (model, transform)

**Analog:** itself — the `HeaderBlock` struct already demonstrates the exact
backward-compat pattern D-07 requires for `Signature`.

**The proven `#[serde(default)]` pattern to copy** (lines 33-66, Phase 14 addition):
```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct HeaderBlock {
    pub org_name: String,
    // ... required fields unchanged from Phase 3-7 ...
    /// Extended requisites (PDFA-03, Phase 14): phone/fax/email/OKPO/OGRN.
    /// `#[serde(default)]` keeps old templates/JSON that omit these keys
    /// deserializing correctly with an empty string (RESEARCH Pitfall 7).
    #[serde(default)]
    pub org_phone: String,
    #[serde(default)]
    pub org_fax: String,
    #[serde(default)]
    pub org_email: String,
    #[serde(default)]
    pub org_okpo: String,
    #[serde(default)]
    pub org_ogrn: String,
    pub act_label: String,
    pub date_label: String,
}
```

**Existing `Signature` variant to extend** (lines 92-98):
```rust
Signature {
    left_label: String,
    right_label: String,
    #[serde(default = "default_spacer_pt")]
    spacer_pt: f32,
},
```
with `fn default_spacer_pt() -> f32 { 24.0 }` (line 117-119) — the exact same
"default fn" pattern (not just `#[serde(default)]` on `Option`) can be reused
if the new sub-labels should default to a non-empty string; if they should
default to `None`/skip-old-behavior, use the plain `#[serde(default)]` +
`Option<String>` style from `HeaderBlock.logo_bytes`/`logo_mime` (lines 44-48):
```rust
/// Logo raw bytes from org_settings BLOB (Phase 7 plan 02).
/// Takes priority over `logo_path` when present.
#[serde(default)]
pub logo_bytes: Option<Vec<u8>>,
```

**Recommended concrete extension for D-07** (following the `Option<String>` +
`#[serde(default)]` idiom, since old JSON/templates must render single-line
signatures unchanged when sub-labels are absent):
```rust
Signature {
    left_label: String,
    right_label: String,
    #[serde(default = "default_spacer_pt")]
    spacer_pt: f32,
    /// Two-line signature sub-labels («Подпись» / «ФИО», D-07, Phase 15).
    /// `None` on old JSON/templates → renderer falls back to the existing
    /// single-line layout (backward-compat, same idiom as HeaderBlock above).
    #[serde(default)]
    left_sublabel: Option<String>,
    #[serde(default)]
    right_sublabel: Option<String>,
},
```

**Test pattern to copy** — `signature_spacer_pt_defaults_when_absent` (lines
228-240) is the direct analog for a new
`signature_sublabels_default_to_none_when_absent` test:
```rust
#[test]
fn signature_spacer_pt_defaults_when_absent() {
    let json = serde_json::json!({
        "type": "signature",
        "left_label": "L",
        "right_label": "R"
    });
    let section: Section = serde_json::from_value(json).expect("parse");
    match section {
        Section::Signature { spacer_pt, .. } => assert_eq!(spacer_pt, 24.0),
        _ => panic!("wrong variant"),
    }
}
```
Also update `sample_docspec()` (lines 125-169) and `round_trip_full_doc`
(lines 171-177) if new required struct fields are added elsewhere (they
aren't, if the extension stays additive-optional).

---

### `crates/trackly-app/src/pdf/renderer.rs` (service, transform)

**Analog:** itself. Three distinct existing code shapes to imitate for the
three new pieces of work:

**1. Header rendering — CURRENTLY NOT a reusable function.** The header block
(lines 126-183 inside `render_docspec`) is inline `draw_text` calls hardcoded
into the render method body. There is no `render_header(...)` function to
"call" — **Pattern 1 in RESEARCH.md requires extracting this into a new
function first**, following the shape of the existing `render_section`
function (lines 204-329) which already takes `(surface, ..., font_regular,
font_bold, y) -> f32` and returns the advanced y-cursor:
```rust
fn render_section(
    surface: &mut krilla::surface::Surface<'_>,
    section: &Section,
    font_regular: &Font,
    font_bold: &Font,
    mut y: f32,
) -> f32 {
    match section { /* ... */ }
}
```
New `render_header_two_column(surface, header, font_regular, font_bold, y) -> f32`
should follow this exact signature shape (surface + refs + y-cursor in/out).

**2. Logo drawing — reuse position logic, change anchor.** `draw_logo_from_bytes`
(lines 393-461) and `draw_logo_top_right` (lines 468-565) already contain the
full graceful-degradation + `scale_logo_dimensions` + `push_transform`/`pop`
pattern. For the 2-column header (D-08б), copy the `tx`/`ty` computation
style but anchor left instead of top-right:
```rust
// Existing top-right anchor (lines 456-460) — COPY THE PATTERN, change tx:
let tx = A4_WIDTH_PT - MARGIN_PT - final_w;  // current: right-aligned
let ty = MARGIN_PT;
surface.push_transform(&Transform::from_translate(tx, ty));
surface.draw_image(image, size);
surface.pop();
// New (left column): tx = MARGIN_PT (fixed left anchor), same ty/push/pop shape.
```
`scale_logo_dimensions` (lines 377-383, already tested via `pdf_logo.rs`) is
reused unchanged — do not reimplement aspect-fit math.

**3. ItemsTable truncation — DO NOT touch, DO NOT reuse for long fields.**
`truncate_to_width` (lines 348-364) is explicitly single-line, ellipsis-only,
and pinned by the `B-3` byte-identical invariant (see docstring + tested in
`pdf_column_overflow.rs`). The new word-wrap helper for Комплектация/Тех.
характеристики/Состояние must be a **separate function** (e.g.
`wrap_text_to_width`, per RESEARCH Pattern 2), not a modification of
`truncate_to_width`. Reuse only the *column-width computation* idiom:
```rust
// Source: renderer.rs:265-273 — reuse this usable_width/margin arithmetic
// for the new wrap-block's max_width, do not reuse col_width equal-division
// (that's the Pitfall 4 anti-pattern — equal-width grid can't host wrap text).
let usable_width = A4_WIDTH_PT - 2.0 * MARGIN_PT;
```

**Existing `Section::Signature` render arm to extend** (lines 302-326):
```rust
Section::Signature {
    left_label,
    right_label,
    spacer_pt,
} => {
    y += spacer_pt;
    let mid = A4_WIDTH_PT / 2.0;
    surface.draw_text(Point::from_xy(MARGIN_PT, y), font_regular.clone(),
        BODY_SIZE_PT, left_label, false, TextDirection::Auto);
    surface.draw_text(Point::from_xy(mid + 10.0, y), font_regular.clone(),
        BODY_SIZE_PT, right_label, false, TextDirection::Auto);
    y + BODY_SIZE_PT + 4.0
}
```
Destructure the new `left_sublabel`/`right_sublabel` fields here and, if
`Some`, draw a second `surface.draw_text` row per side at `y + BODY_SIZE_PT +
2.0` before advancing the cursor — same `draw_text` call shape as the
existing two calls, just parametrized per side in a small loop or repeated
block.

**Error handling pattern (module-wide):** every drawing helper in this file is
**graceful-only** — no `Result` propagation for rendering failures, only
`tracing::warn!` + early `return` (see `draw_logo_from_bytes` lines 399-406,
412-420, 432-436, 439-445, 450-454). Any new helper (header two-column, word
wrap) that hits a recoverable condition (missing font glyph, empty text, zero
width) MUST follow this same warn-and-degrade idiom, never `panic!` or bubble
an `AppError` out of `render_docspec`'s per-section loop.

---

### `crates/trackly-app/templates/act_handover.minijinja` (config/template, transform)

**Analog:** itself (full rewrite) — structure/idiom to preserve from the
current file:

**Header comment block pattern** (lines 1-13) — keep updating the "Available
context variables" doc comment to match any new keys the rewritten template
consumes (it must stay in sync with what `act_service.rs::render_pdf` actually
puts in `ctx`, per Pitfall 3).

**Optional-field defaulting idiom** (used throughout, e.g. line 41):
```jinja
{{ (act.location_name | default("—", true)) | tojson }}
```
This `| default("X", true)` (the `true` = treat empty-string as missing too)
pattern is the established way to degrade empty/missing requisites to "—" per
D-08б — reuse it for `org.phone`/`org.fax`/`org.email`/`org.okpo`/`org.ogrn`.

**Conditional section injection idiom** (lines 42-49, deadline/parent blocks):
```jinja
{%- if act.deadline_human %},
{ "key": "Сроком до",     "value": {{ act.deadline_human | tojson }} }
{%- elif act.deadline %},
{ "key": "Сроком до",     "value": {{ act.deadline | tojson }} }
{%- endif %}
```
Reuse this exact `{%- if X %},...{%- elif Y %}...{%- endif %}` comma-handling
idiom for any new conditionally-present block in the rewritten template
(e.g. optional org requisites lines in the 2-column header, or omitting a
device's Комплектация line when empty).

**Items loop idiom** (lines 64-76) — the existing multi-device loop pattern
(`{%- for item in act.items %} ... {% if not loop.last %},{% endif %} {%-
endfor %}`) is the direct analog for both (a) the compact `ItemsTable` rows
AND (b) the new per-device wrap-block loop that emits the hybrid long-field
sections — same `loop.index`/`loop.last` idiom, just emitting a different
JSON section shape per iteration.

**Sibling template for cross-check:** `act_acceptance.minijinja` (not read in
full here, but referenced in `template_service.rs` `DEFAULT_TEMPLATES` at line
38-42) shares the same `DocSpec` JSON contract — if the rewritten
`act_handover.minijinja` introduces a new reusable JSON section shape (e.g.
the wrap-block), verify `act_acceptance.minijinja` doesn't need the same
(likely out of scope per CONTEXT: "прочие виды документов кроме акта
приёма-передачи").

---

### `crates/trackly-app/src/services/template_service.rs` (service, CRUD + transform)

**Analog:** itself — `validate_preview`'s `demo_ctx` (lines 253-343) is the
mirror of `act_service.rs::render_pdf`'s `ctx` construction and MUST be kept
in sync per Pitfall 3.

**Current demo_ctx gap** (confirmed by direct read, lines 262-308): `act.items[0]`
currently has only `name/inventory_no/serial_no/model/quantity` — **missing
`specs`/`kit`/`condition`** that the real `act_service.rs::render_pdf` already
emits (lines 1397-1406 there). Also missing `org.phone/fax/email/okpo/ogrn`
(present in real ctx at act_service.rs:1416-1420, absent in demo_ctx's `org`
block at lines 263-269).

**Required sync edit** — extend the existing json! macro literal (same
structure, add sibling keys):
```rust
// CURRENT (template_service.rs:262-269) — missing phone/fax/email/okpo/ogrn:
"org": {
    "name": "ООО Демо Организация",
    "inn": "7700000000",
    "kpp": "770000000",
    "address": "г. Москва, ул. Примерная, д. 1",
    "logo_path": null
},
// REQUIRED: add "phone", "fax", "email", "okpo", "ogrn" keys (empty-string
// or realistic demo values), mirroring act_service.rs ctx["org"] shape.

// CURRENT items[0] (lines 281-288) — missing specs/kit/condition:
"items": [
    {
        "name": "HP LaserJet Pro M404n",
        "inventory_no": "ИНВ-001",
        "serial_no": "SN-001",
        "model": "LaserJet Pro M404n",
        "quantity": 1
    }
]
// REQUIRED: add "specs", "kit", "condition" keys to this item object,
// mirroring act_service.rs items_json map (lines 1397-1406).
```

**Fallback DocSpec construction pattern** (lines 316-338) — if the new
`Signature` sub-fields are added as required-in-template (they shouldn't be,
per serde(default)), this fallback `DocSpec { ... }` literal construction
would also need updating; confirmed NOT required if `Section::Signature`
extension stays `#[serde(default)]`-optional (this fallback path doesn't
construct a `Signature` section at all — only a `Paragraph`, so it's
unaffected by D-07).

**Test pattern to copy for regression coverage** — `validate_preview_returns_pdf_bytes`
(lines 419-444) already exists and exercises exactly this demo_ctx against the
real embedded template; it will fail immediately if demo_ctx isn't updated in
lockstep with the rewritten `act_handover.minijinja` (this is the intended
regression signal per Pitfall 3 — no new test file needed here, just keep this
one green).

---

### `crates/trackly-app/src/services/act_service.rs` (service, request-response)

**Analog:** itself — `render_pdf` (lines 1342-1457).

**WR-03 / D-08а logo fix — exact one-line change required** (lines 1356-1360):
```rust
// CURRENT — discards the BLOB bytes it just fetched:
let org_dto = match pipeline.org_db {
    Some(org_db) => {
        let (dto, _logo_bytes, _logo_mime) = org_db.get_for_pdf().await?;
        dto
    }
    None => crate::dto::reports::OrgSettingsDto { /* ... */ },
};
// ...
"logo_path": safe_logo.map(|p| p.display().to_string()),
// logo_bytes/logo_mime never constructed into ctx/HeaderBlock at all.
```
**Required fix pattern** — capture `logo_bytes`/`logo_mime` alongside `dto`
(rename, don't discard), and add them to the `ctx["org"]` json literal
(lines 1410-1422) as new keys `logo_bytes`/`logo_mime` (base64 — see note
below) so the template can emit them into `HeaderBlock.logo_bytes`. **Renderer
already implements the correct priority** (docspec.rs `logo_bytes: Option<Vec<u8>>`
+ renderer.rs lines 178-183 `if logo_bytes.is_some() { draw_from_bytes } else
if logo_path.is_some() { draw_from_path }`) — this is purely an
`act_service.rs` plumbing fix, not a renderer change. `safe_logo` (org.json
path) MUST remain as the `logo_path` fallback — do not remove it (Pitfall 5).

**Byte transport note:** `HeaderBlock.logo_bytes: Option<Vec<u8>>` is
constructed by the **Rust-side DocSpec deserialization**, but the template
only emits **JSON** — raw `Vec<u8>` can't cross a MiniJinja→`serde_json::Value`
boundary as JSON array-of-numbers efficiently for real PNG sizes. Two viable
approaches, pick one and note it in the plan:
  (a) bypass the template for logo — `act_service.rs` constructs the
      `HeaderBlock` fields for logo directly (not via template JSON), by
      post-processing the parsed `DocSpec` after `serde_json::from_str`
      (i.e. `spec.header.logo_bytes = Some(logo_bytes); spec.header.logo_mime
      = Some(logo_mime);` right after line 1454's parse, before calling
      `render_docspec`), OR
  (b) base64-encode `logo_bytes` into the template ctx as a string and add a
      matching `#[serde(with = "base64_serde")]`-style decode step (more
      template plumbing, more surface area for MiniJinja fuel cost per
      Pitfall 6).
  **Recommendation for planner:** (a) is simpler, smaller diff, avoids fuel
  cost concerns entirely, and keeps `HeaderBlock`'s existing `Vec<u8>` shape
  intact — mirrors how `logo_path`'s `safe_logo` value is already computed in
  Rust (line 1354) and merely inserted into the ctx as a string, except here
  the bytes bypass the JSON round-trip entirely.

**Items context — already correct, no change needed** (lines 1393-1408):
```rust
let items_json: Vec<serde_json::Value> = act
    .items
    .iter()
    .map(|it| {
        serde_json::json!({
            "name": it.device_name,
            "inventory_no": it.inventory_no,
            "serial_no": it.serial_no,
            "model": it.model,
            "specs": it.specs,
            "kit": it.complectation_at_time,
            "condition": it.condition_at_time,
            "quantity": it.quantity,
        })
    })
    .collect();
```
This confirms RESEARCH's claim: `specs`/`kit`/`condition` already flow into
`ctx["act"]["items"]` — Phase 15 only needs the **template** to emit them
(currently `act_handover.minijinja`'s `items_table` columns list, lines 63-76,
omits these three fields entirely) and the **renderer** to draw the wrap-block
that consumes them. No `act_service.rs` change needed for items/specs — only
for logo (above).

---

### `crates/trackly-app/tests/pdf_render_act.rs` (test, request-response assertions)

**Analog:** itself — existing tests in the same file demonstrate every needed
assertion idiom.

**Full-pipeline construction pattern** (lines 39-103) — `make_full_pipeline()`
and `make_full_pipeline_with_org_db()` are the two fixtures; new multi-device
and signature-label tests should use `make_full_pipeline_with_org_db()` (the
latter, richer one) since D-08/D-09 assertions need real `org_db` requisites
flowing through, following the existing test `render_pdf_with_filled_specs_and_requisites_surfaces_data`
(lines 344-412) almost verbatim as a template — it already: seeds devices,
sets `device.notes` (specs), calls `org_db.save_fields(...)`, creates a
handover act, calls `render_pdf`, extracts text via `pdf_extract`, and asserts
substring presence.

**pdf_extract assertion idiom** (lines 400-408, reused everywhere in this file):
```rust
let text = pdf_extract::extract_text_from_mem(&bytes).expect("extract");
assert!(
    text.contains("Ромашка"),
    "org_settings org_name missing from rendered PDF. Head: {:?}",
    text.chars().take(500).collect::<String>()
);
```
Use this exact shape for the two Wave-0 gap tests RESEARCH identifies:
- `render_handover_multi_device_wraps_long_fields` (PDFA-02) — seed 2+ devices
  via `seed_devices(&p.writer, N)` (existing helper, lines 105-126), set long
  `notes`/`complectation_at_time`/`condition_at_time` per device (via direct
  `writer.execute` UPDATE, same idiom as lines 350-362), render, assert no `'…'`
  ellipsis marker present for the long fields (contrast with
  `pdf_column_overflow.rs`'s existing `text.contains('…')` assertion — this
  new test asserts the **opposite** for the wrap-block specifically).
- `signature_renders_two_line_labels` (PDFA-05) — render any handover act,
  assert `text.contains("Выдал")`, `text.contains("Получил")`,
  `text.contains("Подпись")`, `text.contains("ФИО")`.

**Device seeding helper to extend** — `seed_devices` (lines 105-126) only sets
`name`; a new variant or inline `writer.execute` UPDATE (same pattern as lines
350-362 setting `notes`) is needed to populate `complectation_at_time`/
`condition_at_time` on the act_items row (see `INSERT INTO devices` at
line 113 vs. the act_item columns `condition_at_time`/`complectation_at_time`
confirmed in `act_service.rs:409-410`) — set these via the act creation path
or a direct UPDATE on `act_items` after `create_handover_with_giver`.

---

### `crates/trackly-app/tests/pdf_column_overflow.rs` (test, transform assertions)

**Analog:** itself — the file's existing structure is the direct template for
a new "wrap does NOT truncate" counterpart test.

**Existing assertion to contrast against** (lines 82-91):
```rust
assert!(
    text.contains('…'),
    "long cell must be truncated with '…', got: {text:?}"
);
assert!(
    !text.contains(LONG_NAME),
    "long name should be truncated, not fully present"
);
```
The new wrap-block test (for Комплектация/Тех.характеристики, D-06) must
assert the **inverse**: `!text.contains('…')` AND the full long string IS
present (across multiple lines, so use a substring that doesn't span an
inserted line-break, e.g. check for a word from the middle of the long spec
text, not the entire string verbatim if wrapping inserts newlines mid-`pdf_extract`
output).

**DocSpec-literal test fixture pattern** (lines 13-33, `spec_with_long_name`)
— reuse this direct-`DocSpec`-construction style (bypassing MiniJinja/template
entirely) for a focused unit-style test of the new wrap primitive, in addition
to the full-pipeline integration test in `pdf_render_act.rs`.

---

### `crates/trackly-app/tests/pdf_logo.rs` (test, transform assertions)

**Analog:** `pdf_render_act.rs::render_pdf_with_filled_specs_and_requisites_surfaces_data`
(full-pipeline pattern) — **role-match, not exact**, because the existing
tests in `pdf_logo.rs` all call `PdfRenderer::render_docspec` **directly**
(bypassing `act_service::render_pdf` and `OrgDbService::get_for_pdf`
entirely) — this is exactly why WR-03 (BLOB logo silently dropped in
`act_service.rs`) wasn't caught by the existing test suite (RESEARCH Wave 0
gap, confirmed by direct read: `base_spec()` at lines 23-44 constructs
`HeaderBlock` inline, never touches `org_db`).

**New test needed** (closes WR-03 regression gap) — follow the full-pipeline
construction from `pdf_render_act.rs` (`make_full_pipeline_with_org_db()`,
`OrgDbService::save_logo` or equivalent to populate `logo_blob`), then call
`p.acts.render_pdf(act.id)` (not `render_docspec` directly), and assert the
same `/Subtype /Image` marker this file already checks for:
```rust
// Existing assertion idiom to reuse (pdf_logo.rs:73-78):
assert!(
    bytes_contain(&bytes, b"/Subtype /Image") || bytes_contain(&bytes, b"/XObject"),
    "rendered PDF must contain /Subtype /Image or /XObject when logo_path is set; \
     got {} bytes",
    bytes.len()
);
```
Need to check `OrgDbService` for a `save_logo`/`update_logo` method (not read
in this pass — grep confirms `logo_blob` write happens at
`org_db_service.rs:158` inside some method) to seed the BLOB in the test setup
before calling `render_pdf`.

**Existing direct-`render_docspec` tests stay as-is** — `act_with_logo_renders_image_in_pdf`,
`logo_path_none_renders_without_panic`, `logo_path_missing_file_is_graceful`,
`logo_bytes_blob_renders_image_in_pdf`, `logo_bytes_takes_priority_over_logo_path`
(lines 50-173) continue to validate the **renderer's** priority logic in
isolation — keep these, they're still valid unit-level coverage; only *add*
the new full-pipeline test, don't replace these.

---

### `crates/trackly-app/tests/pdf_determinism.rs` + fixtures (test fixture, batch)

**Analog:** itself — no code change to the test file; only the two fixture
artifacts need regeneration.

**Expected/required action per Pitfall 1** (not optional — the plan MUST
include this step): after `renderer.rs`/`docspec.rs`/template changes land,
`fixture_act_42_renders_to_known_hash` (lines 20-37) WILL fail because
`render_docspec`'s byte output changes globally (header layout, signature
rendering). Regenerate via:
```rust
// test file already prints the actual hash on failure (lines 32-36):
"PDF hash drift detected. If the change is intentional, update \
 crates/trackly-app/tests/fixtures/act_42.sha256 to {actual}."
```
Run the test once post-implementation, copy `{actual}` into
`tests/fixtures/act_42.sha256`. **Do not edit `act_42.json` itself** unless
the new `Signature` sub-label fields are made non-optional in some other part
of the pipeline (they should stay optional, so `act_42.json`'s existing
`{"type": "signature", ...}` object without sub-labels remains valid input —
verifying backward-compat end-to-end via this exact fixture is a bonus
regression check, not just a hash-pin).

`rendering_twice_yields_identical_bytes` (lines 39-63) requires no changes —
it's format-agnostic and will keep passing as long as the new
`render_header_two_column`/wrap functions are themselves deterministic (pure
functions of `DocSpec` input, no `HashMap` iteration or timestamp use — follow
the same determinism discipline as `normalize_pdf_for_determinism`, lines
573-592, and the module docstring pattern that documents *why* each
determinism guard exists).

## Shared Patterns

### Backward-compat serde extension (`#[serde(default)]`)
**Source:** `crates/trackly-app/src/pdf/docspec.rs` lines 44-61 (`HeaderBlock`
Phase 14 additions)
**Apply to:** `Section::Signature` new sub-label fields (D-07), and any other
additive DocSpec field this phase introduces.
```rust
#[serde(default)]
pub org_phone: String,
```
Old JSON/templates that omit the key deserialize with the zero-value default
(`String::new()` or `None` for `Option<T>`), never erroring.

### Graceful-degradation rendering (never panic, never bubble render errors)
**Source:** `crates/trackly-app/src/pdf/renderer.rs` — every `draw_logo_*`
function (lines 393-565), pattern repeated ~6 times.
**Apply to:** All new renderer helpers (header two-column, word-wrap).
```rust
match some_fallible_step() {
    Ok(v) => v,
    Err(e) => {
        tracing::warn!(error = %e, "description — skipping");
        return; // or return current y-cursor unchanged
    }
}
```

### pdf_extract-based text assertion
**Source:** `crates/trackly-app/tests/pdf_render_act.rs` lines 173-184,
400-408; `pdf_column_overflow.rs` lines 69-91; `pdf_logo.rs` lines 73-78
(byte-marker variant for images).
**Apply to:** All new/extended PDF integration tests in this phase.
```rust
let text = pdf_extract::extract_text_from_mem(&bytes).expect("extract");
assert!(text.contains("expected substring"), "... Head: {:?}",
    text.chars().take(500).collect::<String>());
```

### Full-pipeline test fixture construction
**Source:** `crates/trackly-app/tests/pdf_render_act.rs` lines 39-103
(`make_full_pipeline` / `make_full_pipeline_with_org_db`).
**Apply to:** New tests needing to exercise `act_service::render_pdf` end to
end (multi-device, signature labels, logo-via-BLOB) — prefer
`make_full_pipeline_with_org_db()` whenever the test needs org requisites or
BLOB logo, since it wires the real `OrgDbService` production path instead of
the degraded `None` branch.

### MiniJinja optional-field JSON-emission idiom
**Source:** `crates/trackly-app/templates/act_handover.minijinja` line 41 and
lines 42-49.
**Apply to:** The rewritten template's new conditional blocks (requisites,
Сроком до, per-device wrap fields).
```jinja
{{ (value | default("—", true)) | tojson }}
{%- if condition %},
{ "key": "...", "value": ... }
{%- endif %}
```

## No Analog Found

None. Every file in scope already exists in the codebase with an established
pattern to extend (this phase is a rework/rewrite phase, not new-file
creation) — see per-file "Analog" notes above where the analog is explicitly
"itself."

## Metadata

**Analog search scope:** `crates/trackly-app/src/pdf/`,
`crates/trackly-app/src/services/`, `crates/trackly-app/templates/`,
`crates/trackly-app/tests/` (targeted grep + full reads of all 9 in-scope
files plus `org_db_service.rs::get_for_pdf` and `dto/reports.rs::OrgSettingsDto`
for the logo-BLOB plumbing).
**Files scanned:** 11 read in full or targeted range (docspec.rs, renderer.rs,
act_handover.minijinja, template_service.rs, act_service.rs [targeted
1330-1470], pdf_render_act.rs, pdf_column_overflow.rs, pdf_logo.rs,
pdf_determinism.rs, fixtures/act_42.json, org_db_service.rs [targeted
355-400]).
**Pattern extraction date:** 2026-07-04
