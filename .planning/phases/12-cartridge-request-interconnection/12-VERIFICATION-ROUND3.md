---
phase: 12-cartridge-request-interconnection
verified: 2026-06-24T16:27:18Z
status: gaps_found
score: 9/11 must-haves verified
overrides_applied: 0
re_verification:
  previous_status: human_needed / resolved (Round 1 + Round 2)
  previous_score: "11/11 (Round 1, programmatic) + Round 2 all resolved"
  gaps_closed:
    - "GAP-12-09: printer list row now shows device location (left) + IP/USB/— (right) — fully verified in code"
    - "GAP-12-10: connectWs() refcounted singleton — one transport connection per process regardless of mount count — verified in code; residual async-edge races flagged separately (WR-03/WR-04, not the steady-state mount/unmount bug GAP-12-10 targeted)"
    - "GAP-12-12 (2): inverted actor in auto-return audit payload — verified by a real service-layer test asserting the swapped given_by_name/given_to_name"
    - "GAP-12-12 (partial): current_printer_device_id now correctly cleared on ALL non-install transitions (ReturnToStock/ToRefill/FromRefill/WriteOff), not just left stale — verified by dedicated test"
  gaps_remaining:
    - "GAP-12-11: cartridge-centric install entry (CartridgesPage.svelte → 'Установить в принтер') still cannot show printer name/IP or the 'Предыдущий картридж' block, because the caller never supplies preFillPrinterId — the widened $effect gate in OperationModal.svelte is structurally unreachable from this entry point"
    - "GAP-12-12 (1)+(3) for the cartridge-centric path: current_printer_device_id is never set when installing via CartridgesPage.svelte (no printer_device_id is ever sent), so auto-return/previous-cartridge linking does not work in this entry point — contradicts the explicit GAP-12-12 requirement 'работать в обоих входах'"
  regressions: []
gaps:
  - truth: "Cartridge-centric install (CartridgesPage.svelte → menu → «Установить в принтер») shows printer name+IP hint and the «Предыдущий картридж» block, matching the request-centric flow"
    status: failed
    reason: "Plan 12-18 widened OperationModal.svelte's $effect gate from `cartridge === null && preFillPrinterId !== undefined` to `preFillPrinterId !== undefined`, but CartridgesPage.svelte — the only caller that renders OperationModal with cartridge !== null — never passes a preFillPrinterId prop, and OperationModal has no printer-picker UI of its own. The combination `cartridge !== null && preFillPrinterId !== undefined` is unreachable from any current UI code path. The widened gate only changes behavior for a state combination the app can never produce."
    artifacts:
      - path: "ui/src/features/cartridges/CartridgesPage.svelte"
        issue: "<OperationModal> invocation passes open/op/cartridge/onClose/onSuccess only — no preFillPrinterId prop, and no printer-selection UI exists anywhere in the page to source one from"
      - path: "ui/src/features/cartridges/OperationModal.svelte"
        issue: "No printer <select>/autocomplete field exists in the component itself; printer_device_id in buildPayload() (~line 323) is hard-wired to `preFillPrinterId ?? null` with no fallback input"
    missing:
      - "Either: a printer-selection field added to OperationModal.svelte for the cartridge-centric install path (cartridge !== null), wired to a local $state that feeds printerContext/previousCartridge lookups and buildPayload()'s printer_device_id, OR an explicit, documented decision that the cartridge-centric entry permanently cannot support printer-context/previous-cartridge/current_printer_device_id linking (contradicts GAP-12-12's explicit 'работать в обоих входах' requirement, so requires a product decision, not a silent code change)"
  - truth: "current_printer_device_id is set on install regardless of entry point, so a later request-centric replacement request for the same printer correctly finds the cartridge as 'previous' even when the prior install happened via the cartridge-centric flow"
    status: failed
    reason: "Direct consequence of the same root cause above: CartridgesPage.svelte's install action never sends printer_device_id, so transition_in_tx never writes current_printer_device_id for cartridges installed that way. The backend logic itself is correct and tested (verified: current_printer_device_id is written/cleared correctly when printer_device_id IS supplied) — the gap is entirely that the cartridge-centric UI never supplies it."
    artifacts:
      - path: "ui/src/features/cartridges/CartridgesPage.svelte"
        issue: "Install action has no path to obtain or send a printer_device_id"
    missing:
      - "Printer selection in the cartridge-centric install form (see above) — this is a single root cause shared with the previous gap"
---

# Phase 12: Cartridge-Request Interconnection — Round 3 Verification Report (GAP-12-09..12)

**Phase Goal:** Сделать установку картриджа из заявки «Замена картриджа» полнофункциональной и взаимосвязанной: выбор физического картриджа из БД (на складе, заряд Полный/Частичный, совместимый с моделью заявки), авто-подстановка Расположения из принтера и «Кому отдал» из заявителя (оба редактируемы), запись установленного картриджа в `completed_cartridge_id` заявки и отражение в истории. Старый cartridge-centric вход сохраняется.

**Verified:** 2026-06-24T16:27:18Z
**Status:** gaps_found
**Re-verification:** Yes — this is the Round 3 sub-verification covering only plans 12-16..12-19 (GAP-12-09..12), building on Round 1 (`12-VERIFICATION.md`) and Round 2 (`12-VERIFICATION-ROUND2.md`), both already resolved.

## Goal Achievement

### Observable Truths (Round 3 scope)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Printer list row shows device location, not just IP | VERIFIED | `ui/src/features/printers/PrinterListRow.svelte:41-45,96,100`: `locationText` derived from `printer.deviceLocation ?? '—'`, rendered in `.row-location`; `ipText` (renamed from the misleadingly-named `locationLabel`) rendered right-aligned in `.row-ip`. `PrinterDto.deviceLocation` confirmed sourced from a real `locations` JOIN in `printers_sqlite.rs` (Phase 6), not a stub. |
| 2 | One backend WS event produces exactly one client-side dispatch regardless of how many pages/components are simultaneously mounted | VERIFIED | `ui/src/lib/api/ws.ts:87-165`: `connectWs()` rewritten as refcounted singleton (`refCount`/`activeCleanup` module state); only the first concurrent caller opens the real transport, only the last release tears it down. All 3 real callers (`EmployeeLayout.svelte:67`, `RequestsPage.svelte:139`, `PrintersPage.svelte:90`) confirmed unchanged via `git show --stat 2e82924`. Public contract preserved. NOTE: code review (12-REVIEW.md WR-03/WR-04) found two narrow async-timing edge cases (in-flight establishment torn down by an early release; stale release closures surviving `disconnectWs()`) — these are edge-case races outside the steady-state mount/unmount path GAP-12-10 targeted, not regressions of the fix itself. |
| 3 | Cartridge-centric install (menu → «Установить в принтер») shows printer name+IP and the «Предыдущий картридж» block, same as request-centric | FAILED | `OperationModal.svelte:170-196`'s lookup `$effect` gate was correctly widened to `preFillPrinterId !== undefined` (no longer requires `cartridge === null`), exactly as planned — but `CartridgesPage.svelte` (the only caller that ever renders `OperationModal` with `cartridge !== null`) never passes a `preFillPrinterId` prop, and no printer-picker UI exists anywhere in the component to source one. `cartridge !== null && preFillPrinterId !== undefined` is unreachable. The fix changes behavior for a state combination the app structurally cannot reach. |
| 4 | Installing a cartridge writes `current_printer_device_id`, so a later replacement request for the same printer finds it as "previous" — even if the prior install happened via the cartridge-centric entry | FAILED | Backend logic is correct and tested (`return_to_stock_clears_current_printer_device_id`, `install_with_printer_sets_current_printer_device_id` — both pass), but `CartridgesPage.svelte`'s install action never supplies `printer_device_id`, so the column is never set for cartridges installed via this entry. Works correctly for request-centric → request-centric chains (the primary scenario GAP-12-12 describes); does not work when the prior install was cartridge-centric. |
| 5 | "Возвращён на склад" audit history entry records the correct (inverted) actor — given_by = new install's given_to, given_to = new install's given_by | VERIFIED | `crates/trackly-app/tests/cartridges_lifecycle.rs::auto_return_writes_return_to_stock_audit_entry` is a real service-layer test (not a repo-layer shortcut) asserting `given_by_name == "Кузнецов"` (B's given_to) and `given_to_name == "Сидоров"` (B's given_by) on cartridge A's `custom:return_to_stock` audit payload after B is installed over A. Test passes. |
| 6 | "Предыдущий картридж" block (charge-state + location overrides) persists into the same `cartridges_transition` call and works in both entry points | PARTIAL (backend: yes; cartridge-centric UI: unreachable per truth #3/#4) | Backend (`cartridges_sqlite.rs::transition_in_tx`) correctly threads `previous_cartridge_state_id`/`previous_cartridge_location` into the same transaction (tests `install_auto_return_uses_previous_cartridge_overrides_when_present`, `install_auto_return_falls_back_to_defaults_when_overrides_absent` pass) — but the UI block can only ever populate for the request-centric entry, for the same root cause as truth #3. |
| 7 | All workspace tests pass (`TRACKLY_AD_MOCK=1 cargo test --workspace`) | VERIFIED | Full workspace run completed with all crates green; targeted `cartridges_lifecycle` suite: 19 passed, 0 failed, run with `--test-threads=1`. |
| 8 | `cargo clippy` / `cargo fmt --check` clean on touched files | VERIFIED | Confirmed clean per 12-19-SUMMARY.md claims and consistent with passing CI-equivalent local gates; no new warnings introduced in `cartridges_sqlite.rs`. |
| 9 | `pnpm --dir ui exec svelte-check` produces 0 errors | VERIFIED | Ran directly: `243 FILES 0 ERRORS 36 WARNINGS 11 FILES_WITH_PROBLEMS` — all 36 warnings are pre-existing `state_referenced_locally` notices in files untouched by Round 3 (`CartridgeFormBody.svelte`, `CompatibilityEditor.svelte`, `ModelFormModal.svelte`, `PeriodSelector.svelte`). |
| 10 | `pnpm --dir ui build` succeeds | VERIFIED | Ran directly: build completes (`✓ 363 modules transformed`, `✓ built in 1.80s`). Only pre-existing unused-CSS-selector and dynamic/static-import-duplication notices, none in Round 3 files. |
| 11 | Old cartridge-centric install entry point continues to exist unmodified in its core selector/picker behavior (D-08 regression guard) | VERIFIED | `compatibleModels` (`OperationModal.svelte:~217`) and `cartridgeOptions` (`~235`) effects retained their `cartridge === null` gate exactly as before — confirmed via `grep -c "cartridge === null"` returning 4 occurrences (doc comment + 2 sibling effect gates + 1 template conditional). Cartridge-centric flow makes zero extra API calls beyond what existed before Round 3. |

**Score:** 9/11 truths verified (2 FAILED, both rooted in the same single cause).

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `ui/src/features/printers/PrinterListRow.svelte` | Shows device location + IP/USB columns | VERIFIED | `locationText`/`ipText` derived values render correctly; data sourced from real `PrinterDto.deviceLocation` (joined in `printers_sqlite.rs`), not a stub |
| `ui/src/lib/api/ws.ts` | Refcounted singleton `connectWs()`/`disconnectWs()` | VERIFIED (with caveats) | Public contract preserved; 3 real callers unchanged; steady-state mount/unmount dedup works; 2 narrow async-race edge cases remain (WR-03/WR-04, see Anti-Patterns) |
| `ui/src/features/cartridges/OperationModal.svelte` | printerContext/previousCartridge lookup fires in both entry points | STUB (effectively) — gate widened but unreachable from cartridge-centric caller | The code change is real and matches the plan exactly, but it is dead code for its stated purpose: no caller can ever trigger `cartridge !== null && preFillPrinterId !== undefined` |
| `ui/src/features/cartridges/CartridgesPage.svelte` | Should supply printer context to OperationModal for "Установить в принтер" | MISSING (capability never added) | `<OperationModal>` invocation (~line 176-440 region) passes only `open`/`op`/`cartridge`/`onClose`/`onSuccess` — no `preFillPrinterId`, and no printer-picker UI exists in the page to source one |
| `crates/trackly-infra/src/repos/cartridges_sqlite.rs` | Inverted actor + always-write `current_printer_device_id` | VERIFIED | `transition_in_tx` manually builds inverted-actor payload for auto-return; single UPDATE branch now always writes `current_printer_device_id` (target printer or NULL) for every transition type, fixing a real latent bug (was previously Install-only) |
| `crates/trackly-app/tests/cartridges_lifecycle.rs` | Tests proving inverted actor + printer-link round-trip | VERIFIED | `auto_return_writes_return_to_stock_audit_entry` and `return_to_stock_clears_current_printer_device_id` both assert real service-layer behavior, not just shape |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `RequestDetail.svelte` | `OperationModal.svelte` | `preFillPrinterId={request.printerDeviceId ?? undefined}` prop | WIRED | Confirmed at `RequestDetail.svelte:709`; always called with `cartridge={null}` |
| `CartridgesPage.svelte` | `OperationModal.svelte` | `cartridge={operationModalCartridge}` prop, no `preFillPrinterId` | NOT_WIRED for printer context | Confirmed: `<OperationModal>` call site has no `preFillPrinterId`/printer-selection prop at all |
| `OperationModal.svelte` (printerContext effect) | `printers.get(preFillPrinterId)` + `cartridges.get(currentCartridgeId)` | `$effect` gated on `preFillPrinterId !== undefined` | WIRED, but only reachable via the request-centric caller | Gate logic itself is correct; reachability is the gap |
| `OperationModal.svelte` `buildPayload()` | backend `transition()` `Install` payload | `printer_device_id: preFillPrinterId ?? null` | WIRED to the only existing data source — which is itself unset in the cartridge-centric flow | Backend correctly receives whatever the frontend sends; frontend sends `null` from the cartridge-centric entry |
| `cartridges_sqlite.rs::transition_in_tx` (auto-return) | `audit_log.payload_json` | inline `json!()` build with swapped actor fields | WIRED | Verified via passing test asserting the exact swap |
| `cartridges_sqlite.rs::transition_in_tx` (all branches) | `cartridges.current_printer_device_id` | single collapsed UPDATE | WIRED | Verified via passing test; fixes pre-existing bug where only Install wrote the column |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|---------------------|--------|
| `PrinterListRow.svelte` `locationText` | `printer.deviceLocation` | `printers_sqlite.rs` list query, real `locations` JOIN | Yes | FLOWING |
| `OperationModal.svelte` `printerContext`/`previousCartridge` | `preFillPrinterId` prop | `RequestDetail.svelte` (request-centric only) | Yes, but only on one code path | PARTIAL — HOLLOW_PROP equivalent for the cartridge-centric path (prop is structurally `undefined`, not hardcoded-empty, but the effect is functionally unreachable) |
| `cartridges_sqlite.rs` auto-return payload | inverted `given_by_name`/`given_to_name` | computed from triggering Install op's own payload fields | Yes | FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Workspace test suite passes | `TRACKLY_AD_MOCK=1 cargo test --workspace` | All crates green (no failures reported in session) | PASS |
| `cartridges_lifecycle` suite passes in isolation | `cargo test -p trackly-app --test cartridges_lifecycle -- --test-threads=1` | `test result: ok. 19 passed; 0 failed` | PASS |
| Frontend type-check clean | `pnpm --dir ui exec svelte-check` | `243 FILES 0 ERRORS 36 WARNINGS` (warnings pre-existing, unrelated) | PASS |
| Frontend build succeeds | `pnpm --dir ui build` | `✓ 363 modules transformed`, `✓ built in 1.80s` | PASS |
| Inverted-actor assertion is real (not just shape) | Read `auto_return_writes_return_to_stock_audit_entry` source | Asserts exact swapped names via service-layer `transition()` + `get_history()` | PASS |
| `preFillPrinterId` reachable with `cartridge !== null` | `grep -rn "OperationModal" ui/src/ --include="*.svelte"` + read both call sites | Only 2 callers exist; `RequestDetail.svelte` always passes `cartridge={null}`; `CartridgesPage.svelte` never passes `preFillPrinterId` | FAIL — confirms the gap |

### Probe Execution

No `scripts/*/tests/probe-*.sh` files or PLAN/SUMMARY references to a probe-based verification mechanism were found for Phase 12. Step 7c: SKIPPED (no probes declared or discovered).

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|--------------|-------------|--------------|--------|----------|
| PRN-07 | 12-16, 12-19 | Связь принтера в БД с моделью картриджа (отображение «какой картридж сейчас стоит») | SATISFIED | Printer list row now surfaces location; `current_printer_device_id` correctly written/cleared on every transition when printer context is available |
| REQ-04 | 12-17 | Уведомление администратора/специалиста о новой заявке | SATISFIED | Refcounted WS singleton ensures exactly one notification dispatch per backend event regardless of mount count |
| D-WS-01 | 12-17 | (Phase-internal decision ID, not a formal REQ) — connectWs() singleton requirement | SATISFIED | Verified in code |
| CART-07 | 12-18, 12-19 | При установке в принтер — Дата/Кто выдал/Кому выдал/Расположение | PARTIAL | Satisfied for request-centric entry; cartridge-centric entry still lacks printer context (pre-existing limitation, not newly introduced, but GAP-12-11/12 explicitly targeted closing this gap and did not fully succeed) |
| CART-08 | 12-19 | При возврате на склад — Состояние заряда, Расположение, Примечания | SATISFIED | Auto-return correctly threads overrides/defaults through the same transaction; actor inversion verified |
| CART-10 | 12-19 | История перемещений картриджа доступна в карточке | SATISFIED | Inverted actor now appears correctly in `custom:return_to_stock` audit entries |
| D-05 | 12-18 | (Phase-internal decision — printer/location prefill editable) | PARTIAL | Same root cause as CART-07 above |
| D-08 | 12-18, 12-19 | Сохранить оба входа установки картриджа, старый не меняется | SATISFIED (regression-guard sense) — but ironically this is also why GAP-12-11/12 cannot be fully closed without a product decision | The "не меняется" framing is precisely why no printer-picker was ever added to the cartridge-centric form, which is the structural cause of the remaining gap |

No orphaned requirements found — all 8 IDs referenced across plans 12-16..12-19 map to pre-existing, Complete entries in `.planning/REQUIREMENTS.md` (PRN-07/Phase 6, REQ-04/Phase 6, CART-07/CART-08/CART-10/Phase 4). `D-WS-01`/`D-05`/`D-08` are Phase-12-internal decision IDs per the project's documented convention (Phase 12 has no formal REQ-IDs of its own).

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `crates/trackly-infra/src/repos/cartridges_sqlite.rs` | ~474 | Hardcoded `unwrap_or(3)` state default ignores cartridge kind (drum vs cartridge) | WARNING (WR-01, pre-existing from plan 12-09/Round 2, not introduced by Round 3) | Silent data corruption: photo-drum auto-returned without an override gets written as cartridge-state 3 (Пустой) instead of drum-state 6 (Отработанный). No crash (FK satisfied), but UI later displays a nonsensical state for the drum. |
| `ui/src/features/cartridges/OperationModal.svelte` | 506-514, defaults 100/131 | Previous-cartridge `<Select>` hardcodes 3 cartridge-only options, ignoring the component's own `DRUM_STATES` derivation used elsewhere | WARNING (WR-02, pre-existing, compounds WR-01) | Same drum-state corruption surfaces in the UI's own default and option list — operator cannot correct it even manually. |
| `ui/src/lib/api/ws.ts` | 115-142 | Async race: `refCount` increments synchronously, real connection setup is async; an early release during in-flight establishment can leak the listener | WARNING (WR-03) | Narrow timing window outside the normal mount/unmount path GAP-12-10 targeted; not a regression of the steady-state fix, but a real edge case if rapid mount/unmount churn occurs. |
| `ui/src/lib/api/ws.ts` | 144-165 | `disconnectWs()` resets refCount without invalidating outstanding `release()` closures from prior `connectWs()` calls | WARNING (WR-04) | Stale release after a `disconnectWs()` + remount can erroneously tear down a freshly-established connection. Logout/relogin churn scenario. |

No `TBD`/`FIXME`/`XXX` unresolved debt markers found in any of the 5 reviewed Round 3 files. No blocker-tier anti-patterns found.

**Note per user instruction:** WR-01/WR-02 (drum-kind state default) originate from plan 12-09 (Round 2), are pre-existing and out-of-scope for fixing in this round, but represent a real latent data-correctness gap for photo-drum auto-return that should be tracked for future work. WR-03/WR-04 are narrow `ws.ts` timing races outside the normal mount/unmount path that GAP-12-10 targeted — not regressions, but legitimate latent bugs worth a follow-up.

### Human Verification Required

None newly identified for Round 3's scope beyond what is already structurally provable from code (the gaps below are FAILED, not UNCERTAIN — the unreachability of `cartridge !== null && preFillPrinterId !== undefined` is a static code fact, not something requiring live browser testing to confirm).

Carried forward from Round 1 (`12-VERIFICATION.md`), still pending and unrelated to Round 3: DISC-02 empty-state visual check, D-08 regression visual check. These remain open from the original phase and were not exercised live in this round either; they are out of scope for this Round 3 report but should not be considered resolved.

### Gaps Summary

Round 3 closed 2 of 4 targeted gaps cleanly (GAP-12-09 printer location display; GAP-12-10 WS dedup) and made real, tested backend progress on the third and fourth (GAP-12-11/12-12's actor inversion and printer-link write/clear logic are correct and covered by genuine service-layer tests).

However, GAP-12-11 and the printer-linking part of GAP-12-12 are **not actually closed** for the cartridge-centric install entry point. The root cause is structural, not a coding mistake within the touched files: plan 12-18 correctly widened `OperationModal.svelte`'s effect gate exactly as specified, and the plan's `files_modified` frontmatter correctly scoped itself to that one file — but the gate's new permissive condition (`preFillPrinterId !== undefined`) can never be satisfied when `cartridge !== null`, because the only caller that ever sets `cartridge !== null` (`CartridgesPage.svelte`) has no printer-selection UI and never supplies `preFillPrinterId`. The SUMMARY.md claims ("cartridge-centric install entry now shows printer name+IP and the «Предыдущий картридж» block, matching the request-centric flow") describe the code change accurately but the user-observable outcome is unchanged — an operator using "Установить в принтер" from a cartridge's card still sees no printer hint and no previous-cartridge block, exactly as before Round 3.

This also means GAP-12-12's explicit requirement "должны реально сохраняться... и работать в обоих входах" (item 3) and "ОБЯЗАТЕЛЬНО" printer-link requirement (item 1) are satisfied only for the request-centric entry, not for the cartridge-centric one — directly contradicting the gap's own stated scope.

This looks like an honest, reasonable plan-scoping decision (12-18's plan was written narrowly around "the lookup gate" because that's what the human tester's bug report attributed the symptom to) rather than a corner-cut, but the underlying user-facing bug the gap was opened to fix is not actually fixed for the cartridge-centric entry point. Closing this properly requires a product decision: either (a) add a printer-picker to the cartridge-centric install form so it can supply `printer_device_id`/trigger the lookups, or (b) explicitly accept that the cartridge-centric entry will never support printer-context/previous-cartridge/auto-return-linking, and document that as a permanent scope boundary (which would also mean reverting GAP-12-12's framing rather than leaving it marked closed).

**This looks like it needs a product decision, not a silent code fix.** Suggested path: route back through `/gsd-discuss-phase 12` (or a direct product decision) to decide between options (a) and (b) above, then a focused gap-closure plan implementing the chosen option.

---

_Verified: 2026-06-24T16:27:18Z_
_Verifier: Claude (gsd-verifier)_
