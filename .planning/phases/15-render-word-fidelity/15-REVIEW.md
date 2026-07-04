---
phase: 15-render-word-fidelity
reviewed: 2026-07-04T00:00:00Z
depth: standard
files_reviewed: 10
files_reviewed_list:
  - crates/trackly-app/Cargo.toml
  - crates/trackly-app/src/pdf/docspec.rs
  - crates/trackly-app/src/pdf/renderer.rs
  - crates/trackly-app/src/services/act_service.rs
  - crates/trackly-app/src/services/template_service.rs
  - crates/trackly-app/templates/act_handover.minijinja
  - crates/trackly-app/tests/acts_e2e_smoke.rs
  - crates/trackly-app/tests/pdf_column_overflow.rs
  - crates/trackly-app/tests/pdf_logo.rs
  - crates/trackly-app/tests/pdf_render_act.rs
findings:
  critical: 0
  warning: 6
  info: 4
  total: 10
status: issues_found
---

# Phase 15: Code Review Report

**Reviewed:** 2026-07-04
**Depth:** standard
**Files Reviewed:** 10
**Status:** issues_found

## Summary

Phase 15 reworks the handover-act PDF renderer to match a Word sample: a two-column
header (logo left / requisites right), real glyph-metric word-wrap (`wrap_text_to_width`
via `ttf-parser`), two-line signatures, a per-device `DeviceCard` hybrid layout, and the
WR-03 fix that propagates the `org_settings` BLOB logo bytes into the parsed `DocSpec`
before krilla rendering. The serde contract is backward-compatible (`#[serde(default)]` +
`Option<T>`), the WR-03 fix is correct and now covered by a full-pipeline regression test,
and the wrap function is guarded against the single-long-word infinite-loop pathology.

No BLOCKER-severity defects were found — the changes are functionally correct for the
happy path and the render pipeline degrades gracefully. However, the renderer has **no
vertical bounds checking anywhere** (single-page, cursor-only layout), and several text
paths draw at fixed X offsets with **no horizontal width clamp**, so real-world inputs
(tall logos, many devices, long model/heading strings, wide right-column requisites) will
silently overflow the page or overlap. These are correctness-adjacent fidelity defects that
undermine the stated goal ("match the Word sample") and are the bulk of the findings below.

## Warnings

### WR-01: Header logo (up to 50pt tall) can overlap the first body section

**File:** `crates/trackly-app/src/pdf/renderer.rs:185-312` (esp. 199-204, 311)
**Issue:** `render_header_two_column` draws the logo in the left column at `(MARGIN_PT, y)`
with a height of up to `LOGO_HEIGHT_PT = 50.0`, but the returned `cursor_y` is computed
**only** from the right-column requisite text lines. When the requisites column is short
(e.g. only `org_name` + `org_address` + `ИНН/КПП` + date — the common case with empty
phone/fax/email/OKPO/OGRN), the returned cursor advances roughly `4 * (BODY_SIZE_PT+~) ≈
70pt`, which happens to clear a 50pt logo — but any org that fills fewer requisite lines
(all extended fields empty and short address), or a future `LOGO_HEIGHT_PT` increase, will
have the first `Section::Heading` drawn on top of the logo. The layout takes `max(logo
bottom, text bottom)` nowhere.
**Fix:** Track the logo's bottom edge and return `cursor_y.max(logo_bottom)`:
```rust
let logo_bottom = if header.logo_bytes.is_some() || header.logo_path.is_some() {
    y + LOGO_HEIGHT_PT
} else {
    y
};
// ...after building requisites...
cursor_y.max(logo_bottom) + HEADER_TRAILING_PAD_PT
```
Note `scale_logo_dimensions` may yield a shorter box, so ideally thread the actual
`final_h` back out of the draw helpers rather than assuming the max.

### WR-02: DeviceCard identification values and heading are drawn with no width clamp — silent horizontal overflow

**File:** `crates/trackly-app/src/pdf/renderer.rs:542-572`
**Issue:** In `Section::DeviceCard`, the `heading` (line 542) and each identification
`value` (line 563, drawn at `MARGIN_PT + 120.0`) are passed straight to `surface.draw_text`
with no truncation or wrap. A heading like `"Устройство №1: Ноутбук Lenovo ThinkPad X1
Carbon Gen 11 14-inch"` or a long `Модель`/`Серийный №` value runs past the right page
margin (usable value width is only ~375pt starting at x=170) and is clipped by the page
edge — data loss with no warning. The `long_fields` path correctly wraps, but the
`identification` path does not. `ItemsTable` at least truncates with an ellipsis; this new
path does neither.
**Fix:** Wrap identification values with `wrap_text_to_width` (like long_fields), or at
minimum truncate with `truncate_to_width` against the available column width
(`A4_WIDTH_PT - MARGIN_PT - (MARGIN_PT + 120.0)`). For the heading, wrap against
`A4_WIDTH_PT - 2.0*MARGIN_PT`.

### WR-03: Right-column requisites have no width clamp — long org name/address overflow into or past the page edge

**File:** `crates/trackly-app/src/pdf/renderer.rs:215-308`
**Issue:** Every requisite line (`org_name` at heading size, `org_address`, `Тел./Факс/
E-mail`, `ОКПО/ОГРН`, `ИНН/КПП`, date) is drawn at `text_col_x = MARGIN_PT +
HEADER_LOGO_COL_WIDTH_PT + HEADER_COL_GAP_PT = 182pt` with no wrapping. The right column is
only ~363pt wide (595 − 182 − 50 margin). A realistic full address ("г. Москва,
Ленинградский проспект, д. 80, корп. 17, офис 401") or a long org name at 14pt will run off
the page. Given the phase goal is Word-sample fidelity, an overflowing header is a visible
defect.
**Fix:** Wrap each requisite line with `wrap_text_to_width(regular_face, line, size,
right_col_width)` and advance `cursor_y` per produced line. This also requires threading the
`Face` into `render_header_two_column` (currently only `render_section` receives it).

### WR-04: `wrap_text_to_width` space-width heuristic underestimates the real space advance, so wrapped lines can still overflow when rendered

**File:** `crates/trackly-app/src/pdf/renderer.rs:347-351`
**Issue:** The wrap decision approximates inter-word space as `font_size * 0.25`, but the
actual PDF text is rendered by krilla using DejaVu Sans's real space glyph advance
(~651/2048 em ≈ `0.318 * font_size`, i.e. ~3.18pt at 10pt vs the 2.5pt assumed). Word
advance widths use the true `glyph_hor_advance`, but the space between words does not. On a
line with many short words the accumulated space underestimate (~0.68pt/space) can push the
*rendered* line past `max_width` even though the wrap math thought it fit. The unit test
masks this by allowing `epsilon = font_size` slack, and DeviceCard's full ~495pt content
width has headroom, so it does not currently visibly break — but it is a latent fidelity bug
and directly contradicts the "real glyph-metric" claim in the doc comment (line 314-318).
**Fix:** Measure the space advance from the font like any other glyph instead of hardcoding:
```rust
let space_advance = face.glyph_index(' ')
    .and_then(|g| face.glyph_hor_advance(g))
    .map(|a| a as f32 * scale)
    .unwrap_or(font_size * 0.25);
```
and use `space_advance` both in the fit test and the accumulation.

### WR-05: No vertical pagination — multi-device acts silently render off the bottom of a single A4 page

**File:** `crates/trackly-app/src/pdf/renderer.rs:129-156`
**Issue:** `render_docspec` builds exactly one page and walks all sections with a
monotonically increasing `y` cursor; nothing checks `y` against `A4_HEIGHT_PT - MARGIN_PT`.
`ActService::create` permits up to 100 items (`act_service.rs:134`), and each item now
renders a full `DeviceCard` (heading + 3 identification lines + up to 3 wrapped long-field
blocks). Even ~4–5 devices with populated `Комплектация`/`Технические характеристики`/
`Состояние` will exceed one page; everything past ~790pt is drawn beyond the page and lost.
The test `render_handover_multi_device_wraps_long_fields` uses 5 devices but only sets long
fields on 2, and asserts on text extraction (which recovers off-page text from the content
stream), so it does **not** catch the visual overflow.
**Fix:** This is acknowledged as out of scope in the module comment ("Pagination is out of
scope"), but Phase 15's own change (per-device cards + up-to-100 items) makes overflow the
common case rather than the exception. At minimum, add a page-break check in
`render_section`/the section loop: when `y` would exceed `A4_HEIGHT_PT - MARGIN_PT`, call
`page.finish()` + `doc.start_page_with(...)` and reset `y`. If deferring, file a tracked
follow-up and cap the practical item count, because "все устройства на одной странице,
обрезано" is a data-loss-on-print outcome for the user.

### WR-06: `normalize_pdf_for_determinism` recompiles three regexes on every render

**File:** `crates/trackly-app/src/pdf/renderer.rs:867-886`
**Issue:** The three `Regex::new(...)` calls run on every `render_docspec` invocation. The
inline comment explicitly rejects `OnceLock` ("would add multi-threading surface we don't
need"), but `regex::bytes::Regex` is `Send + Sync` and `OnceLock`/`LazyLock` is the standard,
lint-clean idiom for exactly this. Recompiling per call is wasted work on the hot path and
the stated rationale is inaccurate (there is no added concurrency surface — the regex is
immutable). This is a maintainability/quality issue, not a correctness one (performance is
out of v1 scope, but the misleading justification is worth correcting).
**Fix:**
```rust
use std::sync::LazyLock;
static RE_CREATION: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"/CreationDate \(D:[^)]*\)").unwrap());
// ...same for mod/producer, reference them in the function body.
```

## Info

### IN-01: `wrap_text_to_width` dead comment block adds noise

**File:** `crates/trackly-app/src/pdf/renderer.rs:367-373`
**Issue:** The `if lines.is_empty() { ... }` block contains only a multi-line comment and no
code — it's a no-op branch left in to explain a decision. It reads as if logic was removed
but the scaffold stayed.
**Fix:** Delete the empty `if` and move the explanation to a one-line comment above the
final `lines` return, e.g. `// All-whitespace/empty input legitimately yields zero lines.`

### IN-02: Stale "Phase 3" identifiers baked into deterministic PDF output

**File:** `crates/trackly-app/src/pdf/renderer.rs:51`, `renderer.rs:883`, and doc comments
**Issue:** `PINNED_PRODUCER = "Trackly Phase 3"` (and the normalize replacement string) hard-
code an old phase label that is now embedded in every produced PDF's `/Producer` entry and
pinned by the determinism hash fixture. It's factually stale (this is Phase 15) and couples a
user-visible metadata string to an internal phase number. Not worth changing now if it would
churn the pinned `act_42.sha256`, but flag it: the producer string should be a stable product
name like `"Trackly"`, not a phase marker.
**Fix:** Rename to a phase-independent constant (`"Trackly"`) in a dedicated change that also
re-pins the fixture hash; leave as-is otherwise to avoid an unintended determinism-test break.

### IN-03: `truncate_to_width` still uses the 0.5×font-size average-glyph approximation

**File:** `crates/trackly-app/src/pdf/renderer.rs:629-645`
**Issue:** Phase 15 introduced real glyph metrics (`wrap_text_to_width`) but left the older
`truncate_to_width` on the crude `avg_glyph_w = 0.5 * font_size` heuristic. It's now only
reachable via `Section::ItemsTable`, which the `act_handover` template no longer emits (it
uses `device_card`); `act_acceptance` uses `key_value_table`, not `items_table`. So the
function is effectively dead for the shipped templates but retained for the serde variant and
its dedicated tests. Two width-measurement strategies now coexist for the same font.
**Fix:** Either migrate `ItemsTable` cells to the `Face`-based measurement for consistency,
or document that `ItemsTable` is legacy/unused by shipped templates and slated for removal.
No action required for correctness.

### IN-04: `render_pdf` doc comment lists 6 numbered pipeline stages but the body has diverged

**File:** `crates/trackly-app/src/services/act_service.rs:1333-1341`
**Issue:** The doc comment enumerates a 6-step pipeline that predates the D-05 org_settings
split and the WR-03 BLOB-logo injection (lines 1460-1469). The actual method now also reads
`org_db.get_for_pdf()`, merges a fallback `OrgSettingsDto`, and mutates `spec.header.logo_
bytes` after deserialization — none of which appear in the numbered list. Stale doc drift.
**Fix:** Update the numbered list to reflect the org_settings source, the logo-bytes
post-injection step, and the fallback branch.

---

_Reviewed: 2026-07-04_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
