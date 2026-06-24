---
phase: 12-cartridge-request-interconnection
verified: 2026-06-25T00:30:00Z
status: passed
score: 5/5 must-haves verified
overrides_applied: 0
re_verification:
  previous_status: gaps_found (Round 3)
  previous_score: "9/11 (Round 3) — 2 FAILED truths carried into Round 4 scope"
  gaps_closed:
    - "GAP-12-11: cartridge-centric install entry (CartridgesPage.svelte → «Установить в принтер») now has a reachable optional printer selector (PrinterSelect.svelte) feeding the same printerContext/previousCartridge lookup the request-centric flow uses — printer name/IP hint and «Предыдущий картридж» block are now observable in this entry point."
    - "GAP-12-12 п.1/3: selecting a printer in the cartridge-centric flow now sets effectivePrinterId → printer_device_id is sent in buildPayload() → current_printer_device_id is written/cleared and previous-cartridge auto-return engages — confirmed working in both entry points via the unified effectivePrinterId derived."
  gaps_remaining: []
  regressions: []
---

# Phase 12: Cartridge-Request Interconnection — Round 4 Verification Report (GAP-12-11, GAP-12-12 п.1/3)

**Phase Goal:** Сделать установку картриджа из заявки полнофункциональной и взаимосвязанной; cartridge-centric вход (установка с карточки картриджа) теперь тоже поддерживает ОПЦИОНАЛЬНЫЙ выбор принтера, привязку `current_printer_device_id` и возврат предыдущего картриджа.

**Verified:** 2026-06-25T00:30:00Z
**Status:** passed
**Re-verification:** Yes — Round 4, covering exclusively plan 12-20's closure of the two truths FAILED in `12-VERIFICATION-ROUND3.md` (GAP-12-11, GAP-12-12 п.1/3). Plans 12-01..12-19 already verified in Rounds 1-3 and are out of scope for this report.

## Goal Achievement

### Observable Truths (Round 4 scope)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Cartridge-centric install (`CartridgesPage.svelte` → «Установить в принтер») can show printer name/IP + «Предыдущий картридж» block — a printer selector now exists and feeds the lookup (GAP-12-11) | VERIFIED | `ui/src/features/cartridges/OperationModal.svelte:262-291` adds a new `$effect` gated on `cartridge !== null && preFillPrinterId === undefined` that loads `printers.list()` + `cartridges.modelsGetCompatibleDevices()`. The template (`517-534`) renders `<PrinterSelect>` under that exact gate. Selecting a printer sets `selectedPrinterId`, which flows into `effectivePrinterId` (`174`), which drives the pre-existing `printerContext`/`previousCartridge` lookup `$effect` (`204-230`, unchanged logic, only `preFillPrinterId` → `effectivePrinterId` substitution) — confirmed by reading the gate condition and the diff against `a6227c3`. `CartridgesPage.svelte:428-434` confirmed unchanged (`<OperationModal>` call still has no `preFillPrinterId`), which is precisely the condition that now activates the new selector instead of leaving the path dead. |
| 2 | Selecting a printer in the cartridge-centric flow sends `printer_device_id` → `current_printer_device_id` is set + previous-cartridge auto-return engages, working «в обоих входах» (GAP-12-12 п.1/3) | VERIFIED | `buildPayload()` (`OperationModal.svelte:382`) now reads `printer_device_id: effectivePrinterId ?? null` (was `preFillPrinterId ?? null`). `effectivePrinterId = preFillPrinterId ?? selectedPrinterId` (`174`) is a strict superset: request-centric path is byte-identical (own prop wins), cartridge-centric path now has a non-null source via `selectedPrinterId`. Backend write/clear logic (`transition_in_tx` in `cartridges_sqlite.rs`) and auto-return logic were already verified correct and tested in Round 3 (`return_to_stock_clears_current_printer_device_id`, `install_with_printer_sets_current_printer_device_id`, both still pass — confirmed via `TRACKLY_AD_MOCK=1 cargo test --workspace`, full green, 0 backend files touched this round). The only missing piece Round 3 identified — a UI data source for `printer_device_id` in the cartridge-centric entry — is now supplied. |
| 3 | D-20: printer selection optional — no selection → `printer_device_id` null, no regression to legacy/request-centric path | VERIFIED | `selectedPrinterId` defaults to `undefined` (`164`) and is reset to `undefined` on every modal open (`139`). `PrinterSelect.svelte:79` renders `<option value="">Без привязки к принтеру</option>` as a valid empty selection. `effectivePrinterId` falls through to `undefined` when neither prop nor local state is set, and `buildPayload()` sends `null`. Request-centric path verified byte-identical via `git diff a6227c3 HEAD -- ui/src/features/requests/RequestDetail.svelte` → empty diff; `RequestDetail.svelte` still passes `cartridge={null}` + `preFillPrinterId={request.printerDeviceId ?? undefined}` unconditionally (lines 709-710), so the new selector's render gate (`cartridge !== null && preFillPrinterId === undefined`) never activates there. |
| 4 | D-21: compatible-first list with all-printers fallback, no blocking when compatibility unset | VERIFIED | `PrinterSelect.svelte:47-64` (`groups` `$derived.by`): when `compatibleDeviceIds.size === 0` returns a single flat group (no optgroup header, no blocking); otherwise splits into `Совместимые принтеры` (first) and `Остальные принтеры` (still rendered, never hidden). Source confirms `grep -c "optgroup"` = 2 occurrences (conditional two-group render) and the flat-list `{:else if compatibleDeviceIds.size === 0}` branch in the template (`82-87`) bypasses optgroup entirely. Failure path (`OperationModal.svelte:284-290`) sets `printerOptions = []` on lookup error — fail-safe degrades to "Принтеры не найдены", not a blocked install. |
| 5 | D-22: previous-cartridge block reuse, editable location + charge-state default Пустой | VERIFIED | Zero new markup added for this block — `{#if previousCartridge}` (`OperationModal.svelte:562`) is the exact same pre-existing block from Round 2/3 (D-16), now reachable because `effectivePrinterId` (not just `preFillPrinterId`) drives the lookup that populates `previousCartridge`. `previousCartridgeStateId` defaults to `3` ("Пустой") both at declaration (`109`) and on every modal-open reset (`141`). `previousCartridgeLocation` defaults to `''` (editable via `LocationAutocomplete`, `592-598`). Both flow into the same `transition()` call via `buildPayload()`'s `previous_cartridge_state_id`/`previous_cartridge_location` (`383-385`) — confirmed already-tested backend behavior (`install_auto_return_uses_previous_cartridge_overrides_when_present` test, verified passing in Round 3, re-confirmed green this round). |

**Score:** 5/5 truths verified.

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `ui/src/lib/components/PrinterSelect.svelte` | New compatibility-priority printer `<select>`/`<optgroup>` component | VERIFIED | 171 lines; exports `Props` with `options`, `compatibleDeviceIds`, `value`, `disabled?`, `invalid?`, `id?`, `onchange?` — all 6 fields present (`18-26`). `optgroup` appears 2x in template (conditional grouped render). Placeholder `"Без привязки к принтеру"` appears exactly once (`79`). Markup/SCSS structurally mirrors `GroupedPrinterSelect.svelte` as planned. |
| `ui/src/features/cartridges/OperationModal.svelte` | `selectedPrinterId` state + `effectivePrinterId` wired into existing printerContext/previousCartridge lookup and `buildPayload()` | VERIFIED | `effectivePrinterId` defined via `$derived` (`174`) with exactly 3 downstream uses: `printerContextHint` (`182,185,187`), lookup `$effect` gate+call (`205,211`), `buildPayload()` (`382`) — matches plan's acceptance criterion exactly. `grep -c "cartridge === null"` returns 4 (unchanged from Round 3 baseline) confirming `compatibilityUnconfigured`/`cartridgeOptions` request-centric gates were not touched. |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `PrinterSelect` `onchange` (UI, cartridge-centric) | `OperationModal` `selectedPrinterId` $state | `onchange={(v) => { selectedPrinterId = v ? parseInt(v, 10) : undefined; }}` | WIRED | Confirmed at `OperationModal.svelte:529-531`. |
| `selectedPrinterId` / `preFillPrinterId` | `effectivePrinterId` $derived | `preFillPrinterId ?? selectedPrinterId` | WIRED | Single source-of-truth pattern confirmed at line 174; unifies both UI entry points. |
| `effectivePrinterId` | `printerContext`/`previousCartridge` lookup `$effect` | gate `effectivePrinterId !== undefined`, `printers.get(effectivePrinterId)` | WIRED | Confirmed lines 204-230; same lookup logic as Round 3, just re-pointed at the unified variable. |
| `effectivePrinterId` | `buildPayload()` → `cartridges_transition` (backend) | `printer_device_id: effectivePrinterId ?? null` | WIRED | Confirmed line 382. Backend `transition_in_tx` write/clear logic verified correct and tested in Round 3 — unchanged this round (0 `crates/` diff confirmed via `git diff a6227c3 HEAD -- crates/`). |
| `OperationModal` new printer-list `$effect` | `printers.list()` + `cartridges.modelsGetCompatibleDevices()` | `Promise.all([...])`, gated on `cartridge !== null && preFillPrinterId === undefined` | WIRED | Confirmed lines 270-291; both calls are real, pre-existing, RBAC-gated (`Action::ReadData`) endpoints — `cartridge_models_get_compatible_devices` confirmed gated identically to the already-tested `printers_get_compatible_models`/`printers_list` (Cases 15/16/33 in `role_endpoint_matrix.rs`), though no dedicated test case exists for this specific endpoint's read gate (see Anti-Patterns/caveats). |
| `CartridgesPage.svelte` | `OperationModal` | `<OperationModal open op cartridge onClose onSuccess />` (no `preFillPrinterId`) | UNCHANGED / confirmed as the activating condition | `git diff a6227c3 HEAD -- ui/src/features/cartridges/CartridgesPage.svelte` → empty. This is the exact precondition (`preFillPrinterId === undefined`) the new selector's gate requires — confirming the fix correctly targets the previously-unreachable code path. |
| `RequestDetail.svelte` | `OperationModal` | `cartridge={null}` `preFillPrinterId={request.printerDeviceId ?? undefined}` | UNCHANGED, no regression | `git diff a6227c3 HEAD -- ui/src/features/requests/RequestDetail.svelte` → empty. New selector's gate (`cartridge !== null`) never satisfied here. |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|---------------------|--------|
| `PrinterSelect` `options` prop | `printerOptions` | `printers.list({status:null,search:null},{offset:0,limit:500})` → real `printers_sqlite.rs::list` query | Yes, but capped (see WR-01 caveat below) | FLOWING (capped at 200 server-side) |
| `PrinterSelect` `compatibleDeviceIds` prop | `compatibleDeviceIds` | `cartridges.modelsGetCompatibleDevices(cartridge.model_id)` → real `printer_cartridge_models` junction read | Yes | FLOWING |
| `OperationModal` `previousCartridge` | `effectivePrinterId` → `printers.get()` → `cartridges.get()` | Same pre-existing real DB reads verified in Round 2/3 | Yes | FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Frontend type-check clean | `pnpm --dir ui exec svelte-check` | `244 FILES 0 ERRORS 36 WARNINGS 11 FILES_WITH_PROBLEMS` — all 36 warnings pre-existing in untouched files (`CartridgeFormBody.svelte`, `CompatibilityEditor.svelte`, `ModelFormModal.svelte`, `PeriodSelector.svelte`) | PASS |
| Frontend build succeeds | `pnpm --dir ui build` | `✓ 365 modules transformed`, `✓ built in 1.81s` | PASS |
| Backend untouched, full workspace tests green | `TRACKLY_AD_MOCK=1 cargo test --workspace` | All crates `ok`, 0 failures (cartridges_lifecycle, seed_data, phase06_stubs, doc-tests all passing) | PASS |
| Backend diff confined to docs/planning only | `git diff a6227c3 HEAD -- crates/` | Empty — no Rust files touched this round | PASS |
| `CartridgesPage.svelte` unchanged (confirms gate activates on existing call site) | `git diff a6227c3 HEAD -- ui/src/features/cartridges/CartridgesPage.svelte` | Empty diff | PASS |
| `RequestDetail.svelte` unchanged (confirms no regression) | `git diff a6227c3 HEAD -- ui/src/features/requests/RequestDetail.svelte` | Empty diff | PASS |
| `ui/dist` rebuilt and committed (LAN-browser parity) | `git status --short ui/dist` | Clean — no pending diff, matches SUMMARY's claim | PASS |
| `cartridge_models_get_compatible_devices` RBAC gate confirmed | Read `tauri_cmds/cartridges.rs:178-188` | `authorize(caller, &Action::ReadData)` — same gate as tested `printers_list`/`printers_get_compatible_models` | PASS (gate verified; dedicated test case absent, see caveats) |

### Probe Execution

No `scripts/*/tests/probe-*.sh` files or PLAN/SUMMARY references to a probe-based verification mechanism found for plan 12-20. Step 7c: SKIPPED (no probes declared or discovered).

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|--------------|-------------|--------------|--------|----------|
| D-20 | 12-20 | Printer selection in cartridge-centric install is optional, no regression to legacy/request-centric path | SATISFIED | Truth #3 above |
| D-21 | 12-20 | Compatible-first printer list with all-printers fallback, no blocking when compatibility unset | SATISFIED | Truth #4 above |
| D-22 | 12-20 | Previous-cartridge block reuse, editable location + charge-state default Пустой | SATISFIED | Truth #5 above |

D-20/D-21/D-22 are Phase-12-internal decision IDs (documented in `12-CONTEXT.md` `<gap_closure_round4>`), not formal `REQUIREMENTS.md` entries — confirmed via `grep` against `REQUIREMENTS.md` (no `D-20`/`D-21`/`D-22` rows exist; Phase 12 has no formal REQ-IDs of its own, consistent with Round 3's finding and the ROADMAP.md note "нет формальных REQ-ID, фаза идёт от пользовательских решений"). No orphaned requirements for this round.

Note: ROADMAP.md's Phase 12 goal text still reads "Старый cartridge-centric вход сохраняется" (old entry preserved unchanged) — this is the original D-08 framing from before Round 4. The user's Round 4 decision (`12-CONTEXT.md` `<gap_closure_round4>`, "РЕШЕНИЕ user") explicitly and consciously supersedes this narrow framing for the install-printer-linkage capability specifically (the old "no printer" path is still 100% preserved; what's added is a new optional capability, not a removal). This is a documented, deliberate scope evolution, not a silent deviation — no override entry is needed since the codebase truths being verified here are the Round 4 decisions (D-20/21/22) themselves, which explicitly supersede the relevant slice of D-08.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `ui/src/features/cartridges/OperationModal.svelte` | 277 | New selector requests `limit: 500` but backend hard-caps at `min(200)` (confirmed at `crates/trackly-infra/src/repos/printers_sqlite.rs:314`) — no pagination loop | WARNING (WR-01, code review) | Confirmed real: on fleets >200 printers, `printerOptions` silently truncates server-side (`ORDER BY p.id DESC LIMIT 200`), and a compatible printer outside the first 200 becomes unselectable with no error/"showing N of M" indicator. **Assessed against current scale:** org fleet is ~40 printers per user-confirmed context — this does not currently trigger and does not block the gap closure's correctness today. It is a genuine latent scale ceiling that should be tracked for follow-up before any deployment approaches 200+ printers, not a blocker for this round. |
| `ui/src/lib/components/PrinterSelect.svelte` | 38-41, 85, 92 | Option `value`/label use `deviceId`, not printer record `id` — same convention as the pre-existing (already-verified) request-centric `preFillPrinterId` path | WARNING (WR-02, code review, pre-existing convention) | Internally consistent with the rest of the codebase's `printer_device_id`/`preFillPrinterId` usage (verified: `printers.get(effectivePrinterId)` is called identically to the pre-existing `printers.get(preFillPrinterId)` call, which Round 1-3 already verified as functioning correctly in production use). Not a new inconsistency introduced by this round — flagged for traceability only. |
| `ui/src/features/cartridges/OperationModal.svelte` | 427-455 (`validate()`) | `previousCartridgeLocation` has no required-field validation; can be submitted as empty string when `previousCartridge !== null` | WARNING (WR-03, code review) | Confirmed real: backend (`cartridges_sqlite.rs:475`) accepts an empty-string location for the auto-returned previous cartridge with no validation rejecting it. This is a data-quality gap (operator could skip filling in where the old cartridge physically went), not a functional break of the gap closure — D-22's stated requirement was "editable fields with a default", which is satisfied; mandatory validation was never part of D-22's scope. Recommend tracking as a small follow-up, not a blocker. |

No `TBD`/`FIXME`/`XXX` unresolved debt markers found in either of the 2 files modified this round. No blocker-tier anti-patterns found.

**WR-01 scale assessment (explicitly requested):** WR-01 does not undermine the gap closure for the org's current scale (~40 printers, well under the 200-row server cap), and the failure mode if it were ever triggered is graceful (selector still functions, just incomplete — never a crash or a blocked install, since D-20 keeps "no printer" always available). It is correctly classified as a non-blocking warning for this round, with a recommendation to fix before fleet size approaches 200.

### Human Verification Required

None required to confirm the gap closure itself — the unreachability fix from Round 3 (GAP-12-11/12) was a structural/static code fact (a missing UI affordance + dead gate condition), and the fix is now traceable end-to-end through static analysis: the new selector's render gate matches the existing call site's actual prop values, the new `$effect`'s data sources are real RBAC-gated endpoints, and the unified `effectivePrinterId` correctly feeds the already-tested backend write/clear/auto-return logic. `svelte-check`/build/cargo test all green.

Carried forward from Round 1 (`12-VERIFICATION.md`) and Round 3, still pending and unrelated to Round 4's scope: DISC-02 empty-state visual check, D-08 regression visual check (general UAT of the modal's look-and-feel, not specific to this round's logic). These remain open from the original phase scope and are not newly introduced or resolved by this round.

### Gaps Summary

Round 4 (plan 12-20) cleanly closes both truths that were FAILED in `12-VERIFICATION-ROUND3.md`:

- **GAP-12-11** (cartridge-centric install entry could not show printer name/IP or the previous-cartridge block): closed via a new `PrinterSelect.svelte` component and a gated `$effect` in `OperationModal.svelte` that loads the printer list + reverse compatibility lookup specifically for the cartridge-centric path (`cartridge !== null && preFillPrinterId === undefined`).
- **GAP-12-12 п.1/3** (printer linkage/auto-return did not work in the cartridge-centric entry, contradicting "работать в обоих входах"): closed via the new `effectivePrinterId = preFillPrinterId ?? selectedPrinterId` unification, which feeds the same already-tested backend write/clear/auto-return logic from either UI entry point.

Both fixes are confirmed via direct source inspection of the diff against the Round 3 baseline commit (`a6227c3`), not SUMMARY.md narrative — the call sites (`CartridgesPage.svelte`, `RequestDetail.svelte`) were independently confirmed unchanged, which is precisely what makes the new gate's activation condition (cartridge-centric, no incoming printer context) correctly target the previously-dead code path while leaving the request-centric path byte-identical.

Three non-blocking warnings carry forward from the code review (`12-REVIEW.md`), all assessed as caveats rather than blockers for this round:
- **WR-01** (200-row server pagination cap vs. 500-row client request): real, but does not trigger at the org's current ~40-printer scale; recommend fixing before fleet size approaches 200.
- **WR-02** (deviceId vs. printer-record-id convention): pre-existing convention, not a new inconsistency introduced by this round.
- **WR-03** (no validation on previous-cartridge return location): data-quality gap, not a functional break — D-22's stated requirement (editable field with a sane default) is satisfied.

No regressions found in either the request-centric flow (`RequestDetail.svelte`, byte-identical diff) or the legacy "no printer" cartridge-centric flow (selecting nothing in the new dropdown reproduces exactly the pre-Round-4 payload).

Phase 12's full goal (all rounds) now appears structurally complete at the code level. The two items still open from Round 1 (DISC-02 empty-state visual, D-08 regression visual) are general UAT/visual checks unrelated to this round's specific fix and were never claimed as closed by plan 12-20.

---

_Verified: 2026-06-25T00:30:00Z_
_Verifier: Claude (gsd-verifier)_
