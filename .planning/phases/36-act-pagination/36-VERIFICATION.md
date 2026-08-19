---
phase: 36-act-pagination
verified: 2026-08-13T16:30:08Z
status: passed
score: 4/4 roadmap success criteria verified (2 закрыты живой проверкой 2026-08-19)
overrides_applied: 0
resolved: 2026-08-19
resolution: >
  Все пять пунктов human_verification ниже закрыты пользователем 2026-08-19 живой проверкой
  на Windows/WebView2 из релизной сборки v1.3.3 (portable-zip) — на обоих транспортах,
  включая печать при настройках диалога по умолчанию и LAN-браузер. Дефектов не выявлено,
  правок кода не потребовалось. Success Criteria #1 и #4 переходят из отложенных в
  VERIFIED, score 2/4 → 4/4, status human_needed → passed. Детали по пунктам —
  36-UAT.md (status: complete, 8 pass / 1 skipped / 0 blocked).
  Пункты ниже сохранены как исторический перечень того, что именно проверялось.
human_verification_resolved: true
human_verification:
  - test: "N=1 акт с полным набором полей (инв.№, серийный №, модель, комплектация, тех. характеристики, состояние) — рендер настоящего PDF/«Сохранить как PDF» на обоих транспортах (десктоп + LAN-браузер)"
    expected: "Весь акт помещается на одном листе вместе с полным описанием устройства (DOC-10, Success Criterion #1)"
    why_human: "Требует геометрического подтверждения реального рендера/печати; text-extraction тесты не видят разбиение на листы. Пользователь явно отложил эту проверку 2026-08-13 («смогу сделать только завтра, пропустим как выполненный») — не подтверждено, не провалено."
  - test: "Реальная печать / «Сохранить как PDF» многоустройственного акта на обоих транспортах при НАСТРОЙКАХ ДИАЛОГА ПЕЧАТИ ПО УМОЛЧАНИЮ (не только предпросмотр в модальном окне/iframe)"
    expected: "Зебра appendix-таблицы (D-04) видна в напечатанном/сохранённом результате; если фон не печатается — волосяная линия (D-05) удерживает таблицу читаемой"
    why_human: "print-color-adjust: exact — известный источник расхождений между iframe-предпросмотром и реальным диалогом печати браузера/WebView2 (зависит от настройки «Фоновые рисunки», которая у пользователя может быть выключена по умолчанию). Явно отложено пользователем 2026-08-13."
  - test: "Живой LAN-браузерный транспорт end-to-end (N=1, N=2-3, N=1-с-длинным-полем) — открыть предпросмотр из браузера в локальной сети после `pnpm --dir ui build`"
    expected: "То же поведение пагинации, что и на десктопе: appendix-thead повторяется, «Приложение №1» только на первом листе приложения, зебра, отсутствие page-break внутри группы устройства"
    why_human: "LAN-путь верстается напрямую в DOM приложения (`printViaTopLevel`), отдельным `import('pagedjs')`-путём от desktop/preview UMD-пути — оба места кода проверены на существование хендлера статически (grep), но НИ ОДНО живое подтверждение в LAN-браузере в рамках этой фазы не выполнено."
  - test: "Печать многостраничного акта в LAN-браузере — визуальная проверка отсутствия протечки постороннего DOM/типографики приложения в печатные листы"
    expected: "Печатный DOM изолирован (Phase Success Criterion #4); durable-гейт `ui/scripts/check-print-isolation.mjs` остаётся зелёным"
    why_human: "Структурный гейт `check-print-isolation.mjs` подтверждён зелёным на исходниках (см. Anti-Patterns/Automated Checks ниже), но это статическая проверка CSS-инвариантов, а не подтверждение живого рендера в LAN-браузере — критерий фазы прямо требует именно живого подтверждения, которое явно отложено пользователем."
  - test: "Полный прогон на Windows/WebView2"
    expected: "Пагинация, thead-repeat, зебра и печать ведут себя идентично macOS/WKWebView"
    why_human: "Dev-машина — только macOS; целевая платформа — Windows. Отложенный пункт для каждой фазы этого проекта, не специфичен для Фазы 36."
---

# Phase 36: Пагинация акта по количеству устройств — Verification Report

**Phase Goal:** Разбивка акта приёма-передачи по листам зависит от количества устройств — один
лист с полным описанием для одного устройства, «Приложение №1» с полной таблицей со второго листа
для нескольких.

**Verified:** 2026-08-13T16:30:08Z
**Status:** human_needed
**Re-verification:** No — initial verification (no prior `36-VERIFICATION.md` existed)

## Honesty Note (read this first)

This phase's manual verification is **incomplete by explicit user decision**, recorded verbatim in
`36-05-SUMMARY.md`: *"проверку печати смогу сделать только завтра, так что пропустим этот тест, как
выполненный. В случае косяков с печатью, поправим позже."* The user accepted the risk of shipping
without that verification — this is **not** the same as those items having passed. This report
treats them as **unverified**, not as passed, per that explicit instruction and per this
verification's own adversarial mandate (SUMMARY claims are not evidence).

Everything below that is marked VERIFIED was independently re-checked against the codebase in this
session (tests re-run, files re-read, gates re-executed) — it was not accepted from SUMMARY.md text
alone.

## Goal Achievement

### Observable Truths (ROADMAP.md Success Criteria — the phase contract)

| # | Truth (Success Criterion) | Status | Evidence |
|---|------|--------|----------|
| 1 | Акт на одно устройство целиком умещается на одном листе с полным описанием — **подтверждено рендером настоящего PDF на обоих транспортах** | ? UNCERTAIN (human_needed) | Template branch (N=1 `.device-block` flow, byte-identical to Phase 35) exists and is covered by 21/21 green tests in `html_act_render.rs` (`html_handover_single_device_renders_singular_intro_not_plural_summary` incl. negative assertions that no appendix/ol leaks in at N=1). **The live-PDF-render confirmation this exact criterion requires was explicitly deferred by the user** (`36-05-SUMMARY.md`, `36-CONTEXT.md` deferred section). DOC-10 correspondingly left unchecked in `REQUIREMENTS.md` (line 39, `- [ ] **DOC-10**`) — this is the accurate, honest state, not an oversight. |
| 2 | Акт на несколько устройств выводит на первом листе только перечень имён и отсылку к «Приложению №1», без полного описания | ✓ VERIFIED | Re-ran `html_act_render.rs`/`pdf_render_act.rs` in this session — 21/21 and 15/15 green, including `html_handover_multi_device_renders_plural_summary_listing_every_name` (asserts `.device-block` fully absent at N>1) and `render_handover_default_template_uses_field_rows_not_device_card`. Live desktop UAT (`c11b0d9`, reconfirmed in `36-05`) confirmed pagination renders correctly with real page breaks — the branching this criterion depends on is exercised by that same live render. |
| 3 | Со второго листа — «Приложение №1» с полной таблицей; разрыв страницы и заголовок корректно рендерятся через Paged.js — **подтверждено живым превью** | ✓ VERIFIED | `RepeatTableHeadHandler` exists and is wired in both `bootstrapScript.js` (UMD) and `PdfPreviewModal.svelte::printViaTopLevel` (ESM) — confirmed via direct grep in this session (`afterPageLayout`/`registerHandlers`/`table.appendix-table` present in both files). **Live desktop confirmation** (user, 2026-08-13, post `c11b0d9` ES6-class fix): appendix `<thead>` repeats on every appendix sheet, «Приложение №1» mark only on the first appendix sheet, device row groups not split across page boundaries, no console errors. `node ui/scripts/check-pagedjs-csp-hash.mjs` — OK (re-run this session). |
| 4 | Печать многостраничного акта на обоих транспортах не ломает изоляцию печатного DOM — durable-гейт `check-print-isolation.mjs` остаётся зелёным | ? UNCERTAIN (human_needed) | `node ui/scripts/check-print-isolation.mjs` — **PASS, 0 нарушений** (re-run this session, confirms the structural/static half of this criterion). However the criterion as worded requires this to hold **on a live print of both transports** — LAN-browser transport was **not exercised at all** in this phase (per `36-05-SUMMARY.md` explicit list), and live print-DOM isolation on desktop was not separately confirmed beyond the pagination-structure checks in item #3. Structural gate green ≠ live confirmation performed. |

**Score:** 2/4 truths fully verified; 2/4 explicitly deferred by user decision (not failed, not silently skipped — documented as open in every SUMMARY from 36-05 onward).

### Requirements Coverage

| Requirement | Source Plans | Description | Status | Evidence |
|---|---|---|---|---|
| DOC-10 | 36-01, 36-02, 36-03, 36-05 | Акт на одно устройство умещается на одном листе с полным описанием | ✗ NOT SATISFIED (deferred, not failed) | Template/test-level implementation exists and is green (21/21 `html_act_render.rs`), but SC#1's explicit "live PDF render on both transports" bar is not met. `REQUIREMENTS.md` line 39 correctly shows `[ ]` (unchecked) — matches this verification's finding, not a drift to report. |
| DOC-11 | 36-01..36-06 | Акт на несколько устройств: перечень+отсылка на первом листе, «Приложение №1» с таблицей со второго | ✓ SATISFIED (with caveat) | Template branching, appendix table (7-col, zebra, dash-for-empty, `break-before`/`break-inside`), thead-repeat handler, and the D-17 quantity-aggregation gap-closure (`group_items_for_print`, `act.items_grouped`) are all implemented, unit/integration-tested (90/90 test-result blocks reported green in `36-03`/`36-05`/`36-06`; this session's targeted re-run of `html_act_render`, `html_page_parity`, `pdf_render_act`, `pdf::html_templates`, and `group_items_for_print_tests` — all green), and **live-confirmed by the user on desktop** across two UAT rounds (`36-04`, `36-06`). `REQUIREMENTS.md` line 41 shows `[x]` — literal DOC-11 text is satisfied. Caveat: the phase-level SC#4 (print-DOM isolation on a **live** print, both transports) is a separate roadmap criterion that remains open — see Truth #4 above. |

No orphaned requirements found — `REQUIREMENTS.md`'s Phase 36 row lists exactly DOC-10/DOC-11, matching every plan's `requirements:` frontmatter field.

### Required Artifacts

| Artifact | Expected | Status | Details |
|---|---|---|---|
| `crates/trackly-app/templates/act_handover.html` | N=1/N>1 branching, appendix table, thead-repeat target markup, `act.items_grouped` consumption | ✓ VERIFIED | Re-read in this session: `ol.device-summary` + `table.appendix-table` both iterate `act.items_grouped` (grep confirmed); `break-before: page`, `break-inside: avoid`, `print-color-adjust: exact` all present; `@page` block untouched. |
| `crates/trackly-app/templates/_legacy_defaults/v24/act_handover.html`, `v25/act_handover.html` | Byte-identical pre-edit snapshots for the upgrade path | ✓ VERIFIED | Both directories exist (`ls _legacy_defaults/` shows v20..v25); `KNOWN_LEGACY_DEFAULTS` registers 6 elements for `act_handover.html` (confirmed via grep); `upgrade_replaces_v24_...` and `upgrade_replaces_v25_...` both green in this session's re-run (16/16 `pdf::html_templates` module tests). |
| `crates/trackly-app/src/services/act_service.rs` — `group_items_for_print()` | D-17 aggregation, wired into `render_pdf`'s `ctx.act.items_grouped` | ✓ VERIFIED | 4/4 unit tests green in this session (`identical_printed_fields_merge_into_one_group_with_summed_quantity`, `differing_inventory_no_prevents_merge_and_preserves_both_numbers`, `single_item_passthrough_yields_quantity_one`, `output_order_follows_first_occurrence_not_alphabetical`). |
| `ui/src/lib/pdfPreview/bootstrapScript.js` — `RepeatTableHeadHandler` | Native ES6 class, `afterPageLayout` hook, scoped to `table.appendix-table` | ✓ VERIFIED | Grep-confirmed: `class RepeatTableHeadHandler extends window.PagedModule.Handler`, `afterPageLayout`, `registerHandlers`, scope strictly `table.appendix-table`. `node ui/scripts/check-pagedjs-csp-hash.mjs` — OK (includes the `checkHandlerIsNativeClass` structural guard added after the live-UAT-found ES5-pseudo-inheritance defect). |
| `ui/src/features/acts/PdfPreviewModal.svelte` — `printViaTopLevel` mirror handler | Logically identical handler for the LAN/ESM path | ✓ VERIFIED (code); ⚠️ NOT LIVE-CONFIRMED | Grep-confirmed present (`afterPageLayout`, `registerHandlers`, `table.appendix-table`, native `class ... extends Handler`). Code parity with the UMD path is structurally sound, but this specific path (LAN transport) has never been exercised live in this phase — see Truth #4/#1. |
| `crates/trackly-app/src/http/mod.rs` — CSP `script-src` sha256 | Synced to current `bootstrapScript.js` bytes | ✓ VERIFIED | `node ui/scripts/check-pagedjs-csp-hash.mjs` exits 0 with `OK` in this session — hash matches current file bytes. |

### Key Link Verification

| From | To | Via | Status | Details |
|---|---|---|---|---|
| `act_handover.html` (`ol`/appendix) | `act_service.rs::render_pdf` | `act.items_grouped` render-context key | ✓ WIRED | Template reads `act.items_grouped`; Rust populates it in `render_pdf` alongside unchanged raw `act.items` (D-13 threshold untouched). Confirmed by both grep and the green `group_items_for_print_tests` + `html_handover_duplicate_position_ol_shows_multiplication_suffix`/`html_handover_single_position_quantity_five_still_uses_appendix_branch` tests (re-run green this session). |
| `bootstrapScript.js` | `crates/trackly-app/src/http/mod.rs` (CSP) | sha256 hash check script | ✓ WIRED | `check-pagedjs-csp-hash.mjs` — OK. |
| `PdfPreviewModal.svelte::printViaTopLevel` | Paged.js ESM `Handler`/`registerHandlers` | `import('pagedjs')` destructure + registration before `new Previewer()` | ✓ WIRED (statically); not exercised live in this phase | Code path confirmed by grep; no live LAN-browser session was run to exercise this code path end-to-end. |

### Automated Checks (re-executed this session, not taken from SUMMARY.md)

| Check | Command | Result |
|---|---|---|
| Template/render tests | `cargo test -p trackly-app --test html_act_render --test html_page_parity --test pdf_render_act` | 21+1+15 = 37/37 green |
| Legacy-defaults upgrade path | `cargo test -p trackly-app --lib pdf::html_templates` | 16/16 green (incl. v24, v25 upgrade tests) |
| D-17 aggregation unit tests | `cargo test -p trackly-app --lib group_items_for_print` | 4/4 green |
| CSP hash / native-class regression guard | `node ui/scripts/check-pagedjs-csp-hash.mjs` | OK |
| Print-DOM isolation structural gate | `node ui/scripts/check-print-isolation.mjs` | PASS, 0 нарушений |
| Privacy gate | `./scripts/check-privacy-requisites.sh` | OK — approved placeholders only |
| Debt-marker scan (`TBD`/`FIXME`/`XXX`/`TODO`/`HACK`/`PLACEHOLDER`) on all phase-modified files | grep | 0 matches |
| Git history sanity | `git log --oneline` | All commits referenced by 36-01..36-06 SUMMARYs present (`2c0df06`, `9b783a7`, `cb7c53f`, `fcb6297`, `fd1f01c`, `66ef269`, `2b8662a`, `b349fda`, `f66032b`, `c11b0d9`, `be1376d`, `c1b8934`, `b865736`, `757362e`, `d80faa2`) |

Full-workspace `cargo test -p trackly-app -- --test-threads=1` (all 90 binaries) was **not** re-run in
this verification session — it takes ~60-100 minutes per 36-03/36-05/36-06's own logged wall-clock
times, and the targeted subset above (37 template/render tests + 20 unit tests covering every file
this phase touched) gives equivalent signal for the phase's own scope without the wait. The full-run
result already documented in `36-05-SUMMARY.md` (90/90 green) is corroborated, not contradicted, by
every targeted re-run performed here.

### Anti-Patterns Found

None. No `TBD`/`FIXME`/`XXX`/`TODO`/`HACK`/`PLACEHOLDER` markers in any file this phase modified. No
empty stub implementations, no hardcoded-empty data flowing to render output.

### Data-Flow Trace (Level 4)

`act.items_grouped` is populated by `group_items_for_print()` operating on real `ActItemDto` rows
loaded from the database (`load_items_for_act`) — not a static/empty fallback. Verified by the 4
Rust unit tests (real aggregation logic, not a stub) plus the two new `html_act_render.rs` tests that
exercise the actual `ActItemNewDto { device_ids: Vec::new(), quantity: N }` clone-on-handover
production code path (not direct DB manipulation) — this was a specific improvement made in 36-06
after the team recognized the original D-03 quantity test only proved the template *could* render a
number, not that production data ever produced one greater than a dash. ✓ FLOWING.

### Two Real Defects Found by Live UAT (not by the automated suite)

Documented here as evidence the automated suite, while thorough, did not catch everything —
consistent with this phase's own stated purpose for the mandatory human-verify checkpoints:

1. **`RepeatTableHeadHandler` used ES5 pseudo-inheritance** (`Handler.call(this, ...)`) against
   Paged.js's native ES6 `Handler` class → `TypeError` thrown at construction → silently swallowed by
   the existing D-02 graceful-degrade path → the entire preview fell back to unpaginated rendering
   (no page breaks, no backgrounds) with no test failure anywhere. Caught only because the Task 3
   checkpoint in `36-04` was `gate="blocking"` and the user actually looked at the running app. Fixed
   in `c11b0d9`; a structural regression guard (`checkHandlerIsNativeClass`) was added inside
   `ui/scripts/check-pagedjs-csp-hash.mjs` so this specific regression class can't silently return.
2. **`act_items.quantity` hardcoded to `1` on INSERT**, with multiplicity expressed as N separate
   rows — every test in the suite (across Phases 16 through 36-05) seeded *distinct* devices, so none
   of them ever constructed the actual data shape a real "quantity: 3" selection produces. D-03's
   "Кол-во column shows N when quantity>1" was consequently unreachable in production. Found live,
   fixed via D-17 / gap-closure plan 36-06 (`group_items_for_print`, `act.items_grouped`,
   `_legacy_defaults/v25`), and live-reverified by the user.

### Carried-Forward, Explicitly Out-of-Scope

The same item-duplication defect (D-17's root cause) is still visible on the **act-view screen**
(`ui/src/features/acts/ActItemsTable.svelte` renders one row per `act_item` with «Количество» always
showing 1). This is a pre-existing defect, **not introduced by Phase 36**, and was explicitly deferred
in `36-CONTEXT.md`'s "Дубли позиций в экране просмотра акта" note — correctly out of scope for this
verification, not a gap of this phase.

### Human Verification Required

See YAML frontmatter `human_verification:` for the structured list (5 items) consumed by the
downstream HUMAN-UAT.md sink. Summary:

1. **N=1 one-sheet, live PDF render, both transports** (DOC-10 / Success Criterion #1) — explicitly deferred by the user 2026-08-13.
2. **Real print / "Save as PDF" with print-dialog defaults** (zebra + D-05 hairline fallback) — explicitly deferred.
3. **Live LAN-browser transport, end-to-end**, all three fixtures (N=1, N>1, long-field) — not exercised this phase at all.
4. **Print-DOM isolation on a live LAN print** (Success Criterion #4) — structural gate green, live confirmation outstanding.
5. **Windows/WebView2 run** — deferred pre-close item common to every phase (macOS-only dev machine).

### Gaps Summary

There are no code-level gaps: every artifact this phase was supposed to produce exists, is
substantive (not a stub), is wired into its consumer, and is covered by green automated tests that
were independently re-run in this verification session (not accepted from SUMMARY.md text). Two real
defects surfaced during live UAT were fixed and verified before this report was written.

What remains open is **verification coverage, not implementation** — and it is open by the user's own
explicit, recorded decision to defer real-print/LAN-transport/DOM-isolation-on-live-print/N=1-one-sheet
checks to a later session. Per this workflow's mandate, that deferral must resurface here rather than
be treated as a pass: Success Criteria #1 and #4 are marked UNCERTAIN, not VERIFIED, and this report's
status is `human_needed`, not `passed`.

---

*Verified: 2026-08-13T16:30:08Z*
*Verifier: Claude (gsd-verifier)*
