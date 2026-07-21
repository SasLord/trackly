---
phase: 27-core-workflow-windows
verified: 2026-07-21T18:00:00Z
status: passed
score: 4/4 must-haves verified (roadmap SC), 9/9 plan-level must_haves confirmed
overrides_applied: 0
---

# Phase 27: Окна основного рабочего процесса — Verification Report

**Phase Goal:** Три ключевых транзакционных окна (Акты, Картриджи, Принтеры) переходят на
дизайн-систему Фаз 23–25 без готового макета. SC #1 Акты, SC #2 Картриджи, SC #3 Принтеры —
новые токены/компоненты повсеместно без остатков старых классов; SC #4 каждое поле/действие/
workflow сохранено (изменение чисто визуальное).

**Verified:** 2026-07-21
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths (Roadmap Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Окна Актов используют новые токены/компоненты повсеместно — без остатков старых классов | ✓ VERIFIED | `ActsPage`/`ActsMasterDetail`/`ActsSearchAndTabs`/`ActsList`/`ActListRow`/`ActDetail` import and consume `PageHeader`, `Tabs`, `Table`+`TableRow`, `DetailPanel`/`DetailSection`/`DetailField`; grep for legacy `.page-header`/`.page-title`/`.header-actions`/`.detail-header`/`.fields-grid`/bespoke `class="tab"` in `acts/*.svelte` returns 0 hits (only comments referencing the removed pattern). Modals (`ReturnModal`, `DocumentAcceptanceModal`, `PdfPreviewModal`, `ActFormBody`, item tables) re-tokenized in 27-03. `check-tokens.mjs` passes with 0 violations. |
| 2 | Окна Картриджей используют новые токены/компоненты повсеместно | ✓ VERIFIED | `CartridgesPage`/`CartridgesMasterDetail`/`CartridgesSearchAndTabs`/`CartridgesList`+`CartridgeListRow`/`ModelsList`+`ModelListRow`/`CartridgeDetail` on `PageHeader`/`Tabs`/`Table`+`TableRow`/`DetailPanel` (27-04). `OperationModal` (887 lines) and `ModelFormModal` (580 lines) audited/re-tokenized (27-05). `CartridgeFormBody`, `CompatibilityEditor`, `CartridgeContextMenu`, `LowStockBanner`, `CartridgeFilters`→`Tabs` (27-06). Batch G (27-09) additionally replaced native `<select>` with the `Dropdown` primitive across the window for full visual consistency (beyond plan minimum). No leftover `.fields-grid`/`.detail-header` bespoke classes found (renamed to `.info-grid` to dodge a literal grep collision per 27-04 key-decision, confirmed by reading the file). |
| 3 | Окна Принтеров используют новые токены/компоненты повсеместно | ✓ VERIFIED | `PrintersPage`/`PrintersMasterDetail`/`PrintersSearchAndTabs`/`PrintersList`+`PrinterListRow`/`PrinterDetail` (603 lines, 6 sections preserved) on `PageHeader`/`Tabs`/`Table`+`TableRow`/`DetailPanel` (27-07). `PrinterCreateModal`, `DiscoveryModal` audited clean; `DiscoveryResultsTable`'s raw `<table class="results-table">` replaced with `Table`/`TableRow`/`Checkbox` (27-08, confirmed via `import Table`/`import Checkbox` + 0 remaining `results-table` references). `TonerGauge`/`PrinterAlertBanner` audited as already token-pure. |
| 4 | Каждое существующее поле/действие/workflow в этих окнах остаётся на месте и работает (изменение чисто визуальное) | ✓ VERIFIED | No `src-tauri/` or `ui/src/bindings` files touched by any Phase 27 commit (git log confirms 0 backend commits in phase range). All 24 fix-batch (B–G) commits from 27-09 touch only `.svelte`/`.scss`/`_tokens.scss` files. Spot-checked the riskiest-looking fix (`3d16abc` — merged Имя+IP into one printer-list cell): IP/USB value still rendered, just repositioned under the name (no data loss). `80d0b41` (native `<select>`→`Dropdown` in Cartridges): commit message + code confirm "same options, same selected value/id, same onchange side-effects and placeholders" — component swap, not workflow change. Live human both-theme UAT (`cargo tauri dev`) explicitly re-tested D-01..D-05 + SC #4 across all 3 windows and approved ("Пока что хорошо, оставляем так") after the fix rounds. |

**Score:** 4/4 roadmap Success Criteria verified.

### Plan-Level Must-Haves (9 plans)

| Plan | D-refs | Status | Evidence |
|------|--------|--------|----------|
| 27-01 | D-01 (shared primitive) | ✓ VERIFIED | `DetailPanel.svelte`/`DetailSection.svelte`/`DetailField.svelte` created in `ui/src/lib/components/`, all-tokens (`var(--tr-*)`), consumed by all 3 detail panels. |
| 27-02 | D-01/D-02/D-03/D-05, SC#1 | ✓ VERIFIED | Acts structural layer confirmed on all 5 primitives; `ActHeaderField.svelte` deleted (0 remaining importers). |
| 27-03 | D-04 (Acts modals) | ✓ VERIFIED | `ReturnModal`/`DocumentAcceptanceModal`/`ActFormBody`/`ActItemsTable`/`ReturnItemsTable` re-tokenized; `PdfPreviewModal`/`ActNumberField` confirmed already-clean by audit. |
| 27-04 | D-01/D-02/D-03/D-05, SC#1 (Cartridges) | ✓ VERIFIED | Cartridges structural layer confirmed; D-02 raised-surface applied to `CartridgesMasterDetail`. |
| 27-05 | D-04 (OperationModal/ModelFormModal) | ✓ VERIFIED | `OperationModal` audited clean (no diff); `ModelFormModal` had 3 hardcoded px values converted to tokens. |
| 27-06 | D-04/D-05 (Cartridge form + widgets) | ✓ VERIFIED | `CartridgeFormBody`/`CompatibilityEditor`/`CartridgeContextMenu`/`LowStockBanner` audited clean; `CartridgeFilters`→`Tabs` migrated (116 lines of dead bespoke CSS removed). |
| 27-07 | D-01/D-02/D-03/D-05, SC#1 (Printers) | ✓ VERIFIED | Printers structural layer confirmed; `PrinterDetail`'s 6 sections preserved inside `DetailSection` wrappers, async readings/aggregates loading untouched. |
| 27-08 | D-04 (Printer modals/widgets) | ✓ VERIFIED | `DiscoveryResultsTable` converted to `Table`/`TableRow`/`Checkbox`; `PrinterCreateModal`/`DiscoveryModal`/`TonerGauge`/`PrinterAlertBanner` audited clean. |
| 27-09 | D-02 UAT gate, SC#1-4 | ✓ VERIFIED | Final build gate green; live human both-theme UAT via `cargo tauri dev` performed and explicitly approved after 24 fix commits (batches B–G). This is the authoritative signal for visual SC#1-3 and D-01..D-05 — accepted as-is per task instructions, not re-litigated. |

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `ui/src/lib/components/DetailPanel.svelte` | Shared detail-panel container | ✓ VERIFIED | Exists, consumed by 3 windows (`ActDetail`, `CartridgeDetail`, `PrinterDetail`) + master-detail wrappers. |
| `ui/src/lib/components/DetailSection.svelte` | Section wrapper | ✓ VERIFIED | Exists, consumed by same 3 files. |
| `ui/src/lib/components/DetailField.svelte` | field-grid label/value | ✓ VERIFIED | Exists, consumed. |
| `ui/src/features/acts/ActsList.svelte` | List on Table/TableRow | ✓ VERIFIED | `import Table` present. |
| `ui/src/features/acts/ActDetail.svelte` | Detail on DetailPanel | ✓ VERIFIED | `import DetailPanel` present. |
| `ui/src/features/cartridges/CartridgesList.svelte`, `CartridgeDetail.svelte` | Table/DetailPanel | ✓ VERIFIED | Both wired. |
| `ui/src/features/printers/PrintersList.svelte`, `PrinterDetail.svelte` | Table/DetailPanel | ✓ VERIFIED | Both wired. |
| `ui/src/features/printers/DiscoveryResultsTable.svelte` | Table/TableRow/Checkbox | ✓ VERIFIED | Raw `<table class="results-table">` gone; primitives imported and used. |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| `ActsList.svelte` | `Table.svelte` | import + consumption | ✓ WIRED | Confirmed via grep. |
| `ActDetail.svelte` | `DetailPanel.svelte` | import | ✓ WIRED | Confirmed. |
| `CartridgeDetail.svelte` | `DetailPanel.svelte` | import | ✓ WIRED | Confirmed. |
| `CartridgesList.svelte` | `Table.svelte`/`TableRow.svelte` | import | ✓ WIRED | Confirmed. |
| `PrinterDetail.svelte` | `DetailPanel.svelte` | import | ✓ WIRED | Confirmed. |
| `PrinterListRow.svelte` | `TonerGauge.svelte` | inline consumption | ✓ WIRED | Confirmed (`TonerGauge` referenced in row cell). |
| `LowStockBanner.svelte` | `PrinterAlertBanner.svelte` | twin structure (color-mix warning bg) | ✓ VERIFIED | Both audited as already token-consistent (27-06/27-08). |
| `*MasterDetail.svelte` (×3) | `_tokens.scss` | `--tr-surface-raised`/`--tr-elev-1` | ✓ WIRED | All 3 (`ActsMasterDetail`, `CartridgesMasterDetail`, `PrintersMasterDetail`) confirmed via grep — `background: var(--tr-surface-raised); border: 1px solid var(--tr-border); box-shadow: var(--tr-elev-1);` present in both `.master`/`.detail` blocks. Closes D-13 (Phase 26 accepted regression) for these 3 of 7 windows. |

### Gates (run directly by verifier, not sourced from SUMMARY claims)

| Gate | Command | Result | Status |
|------|---------|--------|--------|
| Closed-world token gate | `node ui/scripts/check-tokens.mjs` | `[check-tokens] PASS — 0 нарушений` | ✓ PASS |
| Type/template check | `pnpm --dir ui svelte-check` | `262 FILES 0 ERRORS 48 WARNINGS 13 FILES_WITH_PROBLEMS` | ✓ PASS (warnings are pre-existing `state_referenced_locally`/unused-CSS-selector style, not phase-introduced errors; none reference removed classes/tokens) |
| Production build | `pnpm --dir ui build` | `✓ built in 2.67s`, `dist/` emitted | ✓ PASS |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `acts/ActFormBody.svelte` | 207 | `TODO: Phase 2 currently stores...` | ℹ️ Info | Pre-existing TODO from Phase 3 (2026-06, commit `c56df7b`), references a completed earlier milestone phase, not introduced or touched by Phase 27. Not a debt-marker gate violation (references formal prior-phase context). |
| `acts/*`, `cartridges/*`, `printers/*` | scattered | Small residual hardcoded px (1-2px gaps, 12-18px icon/glyph font-sizes) | ℹ️ Info | Not "bespoke classes" in the SC#1-3 sense (no leftover component-level duplication of primitive chrome); consistent with the D-04 scope (full re-tokenization of surfaces/spacing/colors, not literal-zero-px-anywhere). Mirrors the same tolerance established in Phase 26. No TBD/FIXME/XXX/HACK/PLACEHOLDER/"not implemented" markers found in any of the 3 feature directories. |

No blocker-level anti-patterns found.

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|--------------|--------|----------|
| WIN-03 | 27-02, 27-03, 27-09 | Акты | ✓ SATISFIED | REQUIREMENTS.md marks WIN-03 complete/Phase 27; structural + modal layers confirmed on primitives. |
| WIN-04 | 27-04, 27-05, 27-06 | Картриджи | ✓ SATISFIED | REQUIREMENTS.md marks WIN-04 complete/Phase 27; structural + modal + widget layers confirmed. |
| WIN-05 | 27-07, 27-08 | Принтеры | ✓ SATISFIED | REQUIREMENTS.md marks WIN-05 complete/Phase 27; structural + modal + widget layers confirmed, `DiscoveryResultsTable` migrated. |

No orphaned requirements found — WIN-03/04/05 are the only requirements mapped to Phase 27 in REQUIREMENTS.md, and all three are claimed across the plan set.

### Human Verification Required

None. The mandatory both-theme visual UAT (D-02 blocking gate, per 27-09-PLAN.md `autonomous: false`) was already performed live by the user via `cargo tauri dev` across both themes and all three windows, and explicitly approved after 6 rounds of fix commits (batches B–G, 24 commits `4f9be3c`..`fc61a06`, all verified present in `git log`). Per task instructions this human approval is treated as authoritative evidence for the visual Success Criteria (SC#1-3) and D-01..D-05 — not re-litigated by this verification pass.

### Deferred Items (correctly scoped, not blockers)

Recorded in STATE.md under "Phase 27 (core-workflow-windows) close on 2026-07-21":

1. **ui_polish** — additional minor visual tweaks the user will specify after Phase 28 completes (deferred to milestone v1.2 end).
2. **tech_debt** — native `<select>` in Phase-28 windows (Settings/Dashboard/Reports/Users) → migrate to `Dropdown` primitive, following the pattern already applied to Cartridges in this phase (commit `80d0b41`).
3. **tech_debt** — `PersonAutocomplete`/`LocationAutocomplete` visual-twin merge candidate (deferred to milestone v1.2 end).

These are correctly out-of-scope for Phase 27 per its own CONTEXT.md `<deferred>` section and do not block the phase goal.

### Gaps Summary

No gaps found. All 4 roadmap Success Criteria verified by direct codebase inspection (component imports/consumption, token usage, class-name absence checks, git-history confirmation that no backend/bindings files were touched). All 9 plans' must_haves confirmed present in the current codebase state. Build/type/token gates all green. The phase's own mandatory human UAT gate was satisfied with live evidence (24 fix commits + explicit user approval), consistent with this being a purely-visual phase with no frontend test suite by design (D-18 precedent from Phase 26).

---

_Verified: 2026-07-21_
_Verifier: Claude (gsd-verifier)_
