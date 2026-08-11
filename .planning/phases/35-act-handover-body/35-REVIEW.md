---
phase: 35-act-handover-body
reviewed: 2026-08-11T00:00:00Z
depth: standard
files_reviewed: 10
files_reviewed_list:
  - crates/trackly-app/src/pdf/html_templates.rs
  - crates/trackly-app/src/services/template_service.rs
  - crates/trackly-app/templates/_legacy_defaults/v22/act_acceptance.html
  - crates/trackly-app/templates/_legacy_defaults/v22/act_handover.html
  - crates/trackly-app/templates/act_acceptance.html
  - crates/trackly-app/templates/act_handover.html
  - crates/trackly-app/tests/acts_e2e_smoke.rs
  - crates/trackly-app/tests/html_act_render.rs
  - crates/trackly-app/tests/html_field_row_underline_gate.rs
  - crates/trackly-app/tests/pdf_render_act.rs
findings:
  critical: 1
  warning: 2
  info: 1
  total: 4
status: issues_found
---

# Phase 35: Code Review Report

**Reviewed:** 2026-08-11T00:00:00Z
**Depth:** standard
**Files Reviewed:** 10
**Status:** issues_found

## Summary

Phase 35 reworks the body of `act_handover.html` and `act_acceptance.html` (horizontal
one-line signature block with printed giver/receiver names, unconditional "Сроком до"
field, dedup of "Кто передал"/"Кто принял" rows, underline removal), snapshots the
pre-phase bodies into `_legacy_defaults/v22/`, registers them in
`KNOWN_LEGACY_DEFAULTS`, adds `act.giver_name`/context wiring, and updates/adds tests.

Verified: `act.giver_name` and `document.giver_name` are supplied by both the real
render path (`act_service.rs::render_pdf`/`render_acceptance_pdf`) and the preview demo
context (`template_service::demo_context_for_kind`) — no `UndefinedBehavior::Strict`
crash risk from the newly-referenced key. Verified the `_legacy_defaults/v22/*.html`
snapshots are byte-identical to the pre-phase-35 on-disk bodies (`git show
e0d2dca^:...`), and that `KNOWN_LEGACY_DEFAULTS` was updated for both filenames — so the
auto-upgrade path will correctly reach existing installs.

One real functional defect found in the reworked `act_handover.html` body: for
multi-device acts (N>1), the per-device technical-detail blocks lose their association
with the device name they describe (see CR-01). Two test-quality gaps found alongside
the otherwise solid new/updated test coverage. No privacy violations, no injection/XSS
issues, no hardcoded secrets found — all names/org data in templates and tests are
fictional placeholders consistent with project convention.

## Critical Issues

### CR-01: Multi-device acts lose per-item device-name association in the printed body

**File:** `crates/trackly-app/templates/act_handover.html:131-164`
**Issue:**
For a single-device act (`act.items | length == 1`), each device-block still prints
`"было получено устройство: {{ item.name }}"` before its technical fields (line 143).
For a multi-device act (`length > 1`), that per-block name line is suppressed — the only
place device names appear is a single `<ul>` at the top (lines 131-138), followed by a
sequence of unlabeled `.device-block` divs (lines 140-164) containing only
`Инвентарный номер:`/`Серийный номер:`/`Модель:`/`Комплектация:`/`Технические
характеристики:`/`Состояние:` rows — with no heading, marker, or divider identifying
which device each block describes.

This is a regression from the pre-Phase-35 body (`_legacy_defaults/v22/act_handover.html`
lines 123-166), which *always* printed `"было получено устройство: {{ item.name }}"`
inside every device-block regardless of item count, so each block was self-identifying.

Consequences for a real multi-device handover act (a common path — the app explicitly
supports bulk device handover):
- A reader cannot reliably tell which set of printed technical fields belongs to which
  listed device name, especially once fields differ in count/length between devices (no
  visual divider beyond an 8pt margin) or the document spans a page break.
- If a device has none of the optional fields populated (`inventory_no`/`serial_no`/
  `model`/`kit`/`specs`/`condition` all empty), its `.device-block` renders completely
  empty — that device contributes *zero* identifiable content beyond bare list
  membership in the `<ul>`.
- This directly touches the project's stated core value ("акты приёма-передачи ...
  должно работать надёжно ... без потери истории") — an ambiguous legal transfer
  document is a real-world correctness defect, not a cosmetic one.

No existing test catches this: `render_handover_multi_device_wraps_long_fields` and
`render_handover_default_template_uses_field_rows_not_device_card` in
`tests/pdf_render_act.rs` / `tests/html_act_render.rs` only assert substring presence
and relative ordering of device *names*, never that a name is co-located with (or
otherwise associable to) its own field block.

**Fix:**
Print the device name inside every per-item block, not just when there is exactly one
item — drop the `length == 1` gate (or keep the top `<ul>` as an optional summary and
add a lightweight per-block label for the N>1 case):

```jinja
{%- for item in act.items %}
<div class="device-block">
  <div class="field-row">было получено устройство: {{ item.name }}</div>
  {%- if item.inventory_no %}
  <div class="field-row">Инвентарный номер: {{ item.inventory_no }}</div>
  {%- endif %}
  ...
```

If the `<ul>` summary for N>1 is intentionally kept as well, that's fine — but the
per-block identification must not be removed for the multi-device case.

## Warnings

### WR-01: New `_legacy_defaults/v22` snapshot has no dedicated upgrade-regression guard

**File:** `crates/trackly-app/src/pdf/html_templates.rs:75-105` (registration),
`:415-465` (existing v21-only guard test)
**Issue:**
This phase appended a `v22` entry to `KNOWN_LEGACY_DEFAULTS` for both
`act_handover.html` and `act_acceptance.html` (the exact pattern the module's own doc
comment at lines 57-63 calls "the extension point"). Phase 34 added an analogous `v21`
entry and, alongside it, a dedicated test —
`upgrade_replaces_v21_legacy_default_with_current_bundled_body` — that pulls
`bodies.get(1)` specifically and asserts `assert_ne!(v21_body, current, ...)` as a
precondition guard, explicitly to catch the failure mode "the snapshot was taken AFTER
the rewrite instead of before" (see the test's own doc comment, "Pitfall 5").

Phase 35 did not add the equivalent `bodies.get(2)` / v22 guard test. The existing
`.first()`-based test (v20) and the `.get(1)`-based test (v21) do not exercise index 2,
so nothing in the test suite proves the newly-captured v22 body (a) differs from the
current bundled default, or (b) actually drives a real auto-upgrade for installs
currently on the pre-Phase-35 body — which is precisely the population of real installs
this phase's snapshot exists to reach. (I independently confirmed via
`git show e0d2dca^:...` that the v22 snapshot is correct — but that verification is not
encoded as a regression test, so a future accidental re-snapshot would go undetected,
same as the exact bug class the v21 test was written to prevent.)

**Fix:** Add a `v22`-indexed sibling to `upgrade_replaces_v21_legacy_default_with_current_bundled_body`
(or generalize it to loop over all indices instead of hardcoding `.get(1)`), e.g.:

```rust
#[test]
fn upgrade_replaces_v22_legacy_default_with_current_bundled_body() {
    // mirrors upgrade_replaces_v21_legacy_default_with_current_bundled_body,
    // using bodies.get(2) instead of bodies.get(1)
}
```

### WR-02: Overlapping/vacuous label assertion in `html_handover_contains_required_blocks_and_logo`

**File:** `crates/trackly-app/tests/html_act_render.rs:184-202`
**Issue:**
```rust
let act = create_handover(&p.acts, &[device_id], "Выдалов В.В.", "Получилов П.П.").await;
...
for expected in ["Акт приема-передачи", "Выдал", "Получил", "Подпись"] {
    assert!(html.contains(expected), ...);
}
...
assert!(html.contains("Выдалов В.В."), "expected printed giver_name in signature block. ...");
```
The fixture's giver name is `"Выдалов В.В."` and receiver name is `"Получилов П.П."` —
both of which contain `"Выдал"`/`"Получил"` as a literal prefix. So the loop's
`html.contains("Выдал")` / `html.contains("Получил")` assertions pass unconditionally
once the giver/receiver names render anywhere in the document (e.g. via the signature
block this very phase reworked, or the intro paragraph) — independent of whether the
actual `"Выдал:"`/`"Получил:"` static signature labels are present at all. This test is
supposed to be the D-14 "required blocks/labels" regression gate exactly for the
signature-block layout this phase rewrote (grid two-column → horizontal one-line), so
having its core label checks be able to pass vacuously undermines its purpose. Phase 35
added a new, precise `"Выдалов В.В."` assertion right below the existing loop but left
the now-doubly-satisfied `"Выдал"`/`"Получил"` checks unchanged, which also makes the
test harder to read (two assertions checking near-identical substrings for different
reasons).

**Fix:** Assert on the label text including the colon (`"Выдал:"`, `"Получил:"`), which
does not collide with the fixture names, or use fixture names that don't share a prefix
with the label text:

```rust
for expected in ["Акт приема-передачи", "Выдал:", "Получил:", "Подпись"] {
    assert!(html.contains(expected), ...);
}
```

## Info

### IN-01: `act_acceptance.html` has no structural underline/CSS regression gate

**File:** `crates/trackly-app/tests/html_field_row_underline_gate.rs`
**Issue:** This phase added a dedicated structural CSS gate
(`field_row_css_has_no_border_bottom_and_only_two_legit_exceptions_remain`) scoped
exclusively to `act_handover.html`, verifying exactly two `border-bottom` declarations
survive in its `<style>` block. `act_acceptance.html` received the equivalent Phase 35
D-09 signature-block rework (its own `.signature-field .signature-line { border-bottom:
... }`), but has no analogous guard. Lower priority than WR-01/WR-02 since
`act_acceptance.html` never had the field-row label/value underline pattern this gate
exists to guard against (it uses `table.kv`, not `.field-row`), so the risk surface is
smaller — but a future edit to `act_acceptance.html`'s `<style>` block could
silently reintroduce an unwanted underline (e.g. on `table.kv td`) with no test noticing.
**Fix:** Either extend `html_field_row_underline_gate.rs` to cover
`act_acceptance.html`'s `<style>` block with an equivalent "exactly one legitimate
`border-bottom`" assertion, or add a short doc note explaining why it was intentionally
scoped to `act_handover.html` only.

---

_Reviewed: 2026-08-11T00:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
