---
phase: 18
slug: autocomplete-dropdowns
status: validated
nyquist_compliant: partial
wave_0_complete: true
created: 2026-07-15
---

# Phase 18 — Validation Strategy

> Per-phase validation contract. Reconstructed retroactively (State B) at milestone
> v1.1.2 close. Backend list_grouped contract is fully automated; the pure-UI
> autocomplete/dropdown behaviors (AUTO-01/AUTO-02) are manual-only because the
> project has no frontend test harness by established convention (frontend is
> validated via `/gsd-ui-review` + live UAT, never unit-tested).

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` (Rust, backend contract only) |
| **Config file** | none — workspace default |
| **Quick run command** | `cargo test -p trackly-app --test devices_grouping` |
| **Full suite command** | `cargo test -p trackly-app` |
| **Estimated runtime** | ~1.1s (grouping suite) |
| **Frontend** | No harness (no vitest/jest/playwright anywhere in `ui/`) — deliberate project convention |

---

## Sampling Rate

- **After every backend task commit:** Run `cargo test -p trackly-app --test devices_grouping`
- **Before `/gsd-verify-work`:** Full backend suite green + live UAT for UI behaviors
- **Max feedback latency:** ~1s (backend)

---

## Per-Task Verification Map

| Requirement | Plan | Behavior | Test Type | Automated Command | Status |
|-------------|------|----------|-----------|-------------------|--------|
| AUTO-03 | 18-01 | Multi-field text filter (name/inventory_no/serial_no/model) via devices_fts MATCH + build_fts_query sanitizer | unit | `cargo test -p trackly-app --test devices_grouping grouping_true_branch_filters_by_name_text grouping_true_branch_filters_by_inventory_and_serial grouping_true_branch_query_sanitizes_special_chars` | ✅ green |
| AUTO-04 | 18-01 | True-branch sorts by count DESC, name ASC (not alphabetical) | unit | `cargo test -p trackly-app --test devices_grouping grouping_true_branch_sorts_by_count_desc` | ✅ green |
| AUTO-05 | 18-01 | Groups by (type_id, name, model), condition repurposed as drill-in signal (distinct count) | unit | `cargo test -p trackly-app --test devices_grouping grouping_true_branch_splits_by_model grouping_act_form_groups_by_name_and_model_not_condition model_key_splits_groups_condition_does_not grouping_condition_distinct_count_mixed` | ✅ green |
| AUTO-01 | 18-02, 18-03 | Native `<select>` wrappers keep browser popup; custom autocompletes use `use:portal` + `use:dropdownAnchor` (no custom overlay clipped by containers) | manual | — (no frontend harness) | manual-only |
| AUTO-02 | 18-04 | Focus-open device picker: focusing the input fetches immediately (delay 0) and shows top-20 groups by stock, no text required | manual | — (no frontend harness) | manual-only |

*Backend grouping suite: 24 passing tests total; the rows above name the requirement-specific cases.*

---

## Wave 0 Requirements

Existing infrastructure (`crates/trackly-app/tests/devices_grouping.rs`) covers all
automatable phase requirements (AUTO-03/04/05). No new backend harness needed.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Portal-anchored dropdown positioning: dropdown escapes the act-modal/table overflow container, repositions on capture-phase scroll/resize, flips upward near viewport bottom | AUTO-01 | Pure DOM/CSS layout behavior; project has no frontend test harness (no vitest/playwright), and introducing one for a single phase is off-convention and disproportionate. Validated via `/gsd-ui-review` (18-UI-SPEC.md contract) + live UAT. | Open an act edit form; focus a device-picker row near the bottom of the viewport; confirm the dropdown renders above the input, is not clipped, and stays anchored while scrolling the modal. |
| Native `<select>` invariant (no hidden custom overlay on Select/CartridgeSelect/GroupedPrinterSelect/PrinterSelect) | AUTO-01 | Structural/source invariant confirmed by code re-read in 18-03; no runtime harness exists to assert absence of a custom listbox. | Re-read the 4 wrapper components; confirm each uses a native `<select>` with only a decorative `position:absolute` caret (documented invariant comment present). |
| Focus-open + real-time filter + keyboard nav (ArrowUp/Down/Enter/Tab/Escape) + empty-state ("Ничего не найдено") in the ActFormItemsTable device picker | AUTO-02, AUTO-03 (UI surface) | Interactive Svelte behavior; no frontend harness. Backend filter contract (AUTO-03) IS automated in devices_grouping.rs; only the UI wiring is manual. | Focus a device input (dropdown opens with top-20 by stock); type to filter; navigate with arrows; select with Enter; clear to a no-match query and confirm empty-state renders instead of closing. |

---

## Validation Sign-Off

- [x] All automatable requirements (AUTO-03/04/05) have green automated tests
- [x] Pure-UI requirements (AUTO-01/02) documented as manual-only with rationale + steps
- [x] Backend grouping suite runs green (24 passed, 2026-07-15)
- [x] No watch-mode flags
- [ ] `nyquist_compliant: true` — **not set**: partial by design (frontend has no test harness; UI behaviors are manual-only per project convention)

**Approval:** approved 2026-07-15 (retroactive, milestone v1.1.2 close) — status `partial` accepted as tech-debt-free for a UI phase without a frontend harness.
