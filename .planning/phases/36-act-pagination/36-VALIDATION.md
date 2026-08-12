---
phase: 36
slug: act-pagination
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-08-12
---

# Phase 36 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Derived from `36-RESEARCH.md` § Validation Architecture.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` (integration targets in `crates/trackly-app/tests/*.rs`) + `pnpm --dir ui lint` for the structural JS gates |
| **Config file** | none separate — workspace `Cargo.toml`; `ui/package.json` `lint` script chains the gates |
| **Quick run command** | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test html_act_render -- --test-threads=1` |
| **Full suite command** | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app -- --test-threads=1` (requires a real `pnpm --dir ui build` beforehand) then `pnpm --dir ui lint` |
| **Estimated runtime** | ~20 s quick, ~3–5 min full |

**Hard constraints (project memory — do not re-litigate):**
- Never run two `cargo test` invocations concurrently — `target/` lock contention looks like a hang.
- `cargo test --workspace` hangs on the pre-existing `login_remember_persistent_cookie`; use targeted `-p trackly-app`, `--skip` that test if running the whole workspace.
- `pnpm --dir ui build` must run before the suite — a placeholder `ui/dist` fails the security-headers SPA test.

---

## Sampling Rate

- **After every task commit:** `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test html_act_render -- --test-threads=1`
- **After every plan wave:** full suite (`cargo test -p trackly-app`) after `pnpm --dir ui build`, plus `pnpm --dir ui lint`
- **Before `/gsd-verify-work`:** full suite green **and** the manual visual pass below completed on BOTH transports
- **Max feedback latency:** ~20 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 36-01-xx | 01 | 1 | DOC-10/DOC-11 | — | N/A (snapshot only) | integration | `cargo test -p trackly-app --lib pdf::html_templates` | ❌ W0 — new v24 upgrade test | ⬜ pending |
| 36-02-xx | 02 | 2 | DOC-10 | — | autoescape retained on all new interpolation sites | integration | `cargo test -p trackly-app --test html_act_render html_handover_single_device_renders_singular_intro_not_plural_summary` | ✅ existing | ⬜ pending |
| 36-02-xx | 02 | 2 | DOC-11 | — | plain `{{ }}` only, no new `\| safe` sink | integration | `cargo test -p trackly-app --test html_act_render html_handover_appendix_table_has_one_row_per_device` | ❌ W0 — new | ⬜ pending |
| 36-02-xx | 02 | 2 | DOC-11 | — | N/A | integration | new test: `<ol>` numbering ↔ appendix `№` column correspondence | ❌ W0 — new | ⬜ pending |
| 36-02-xx | 02 | 2 | DOC-11 | — | N/A | structural | new gate: `break-before: page` + `break-inside: avoid` present on the right selectors (byte-presence, style of `html_page_parity.rs`) | ❌ W0 — new | ⬜ pending |
| 36-03-xx | 03 | 3 | DOC-11 (SC #3) | T-36-CSP | CSP sha256 regenerated after `bootstrapScript.js` edit | structural | `node scripts/check-pagedjs-csp-hash.mjs` (via `pnpm --dir ui lint`) | ✅ existing gate | ⬜ pending |
| 36-03-xx | 03 | 3 | Phase SC #4 | — | print DOM isolation invariants intact | structural | `node scripts/check-print-isolation.mjs` (via `pnpm --dir ui lint`) | ✅ existing gate | ⬜ pending |
| 36-0x-xx | any | any | — | — | N/A | structural | `cargo test -p trackly-app --test html_page_parity` — MUST stay green untouched | ✅ existing | ⬜ pending |
| 36-0x-xx | any | any | — | — | N/A | integration | `cargo test -p trackly-app --test pdf_render_act`, `--test acts_e2e_smoke` — expected drift, must end green | ✅ existing | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/trackly-app/templates/_legacy_defaults/v24/act_handover.html` — snapshot taken **before** any pagination edit; registered in `KNOWN_LEGACY_DEFAULTS`.
- [ ] `upgrade_replaces_v24_legacy_default_with_current_bundled_body` in `html_templates.rs`, mirroring the v21/v22/v23 pattern (`assert_ne!` guard included).
- [ ] New appendix-table structural tests: device count → row count; `quantity` column blank/«—» at 1 and printed at >1; `<ol>` ↔ appendix `№` correspondence.
- [ ] Rewrite (not merely touch) `html_act_render.rs::extract_first_ul` and its dependants — `html_handover_multi_device_renders_plural_summary_listing_every_name`, `render_handover_multi_device_fields_attributable_to_own_device`, `render_handover_multi_device_wraps_long_fields` — all assert the pre-36 `<ul>` + repeated `.device-block` shape that D-07/D-08 remove.
- [ ] Custom Paged.js `afterPageLayout` handler for `<thead>` repetition, landed in BOTH `ui/src/lib/pdfPreview/bootstrapScript.js` and the `printViaTopLevel()` ESM path in `ui/src/features/acts/PdfPreviewModal.svelte` (D-15a). **User decided 2026-08-12: implement directly, no preliminary spike.**
- [ ] CSP sha256 constant in `crates/trackly-app/src/http/mod.rs` regenerated after the `bootstrapScript.js` edit.

---

## Manual-Only Verifications

Text-extraction tests cannot see page breaks, repeated `<thead>`, or whether the zebra background actually printed. Every row below is required before phase close, on **both** transports (desktop `cargo tauri dev`; LAN browser after `pnpm --dir ui build`).

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| N=1 act fully on one sheet with full description | DOC-10 (SC #1) | Geometry invisible to text extraction | Preview a 1-device handover act; confirm one sheet and full description present |
| N>1 first sheet shows ONLY the `<ol>` summary + referral line | DOC-11 (SC #2) | Same | Preview a 2–3-device act; confirm no full field descriptions on sheet 1 |
| Appendix starts on sheet 2 with forced break; appendix mark + thead render correctly | DOC-11 (SC #3) | Page-break rendering and thead repetition are exactly what text extraction cannot see | Same preview; scroll every appendix sheet. Use a 15+-device fixture so the appendix spans 2+ sheets and thead repetition is observable |
| Zebra + `print-color-adjust: exact` survives real print | D-04/D-05 | Background suppression is print-time browser behavior, absent from the on-screen srcdoc preview | Trigger a real print / "Save as PDF" (not just the modal) on both transports with the print dialog's background-graphics setting at its DEFAULT; confirm zebra visible, and that the hairline fallback keeps the table legible if it is not |
| Print isolation unaffected by the new appendix CSS | Phase SC #4 | The structural gate proves invariants exist in source, not that the render looks right | Live LAN print of a multi-device act; confirm no app chrome/typography bleeds into the printed appendix pages |
| Windows / WebView2 run | all | Dev machine is macOS only | Deferred pre-close UAT item for the user, as in prior phases |

---

## Validation Sign-Off

- [ ] All tasks have an `<automated>` verify or a Wave 0 dependency
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30 s
- [ ] Manual-only table completed on both transports
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
