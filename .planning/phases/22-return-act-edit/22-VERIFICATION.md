---
phase: 22-return-act-edit
verified: 2026-07-13T15:02:52Z
status: human_needed
score: 21/21 must-haves verified
overrides_applied: 0
human_verification:
  - test: "Open the ReturnModal edit dialog for a real, previously-created return act (with a device that has a location + condition set) and visually confirm the dialog title reads «Возврат по акту №XXX» and every prefilled field (checked devices, per-row состояние, per-row расположение, ФИО giver/receiver un-swapped, «Дата возврата») matches exactly what was recorded at return time."
    expected: "Dialog opens instantly with no flash of empty/default values; all fields show the return's own saved values, not the parent handover's values."
    why_human: "Field-by-field data wiring is confirmed in code (ReturnModal.svelte $effect prefill block) and by backend tests, but actual visual rendering, dropdown/autocomplete behavior, and absence of a prefill flash can only be judged in a running browser/webview."
  - test: "Edit a real return (change condition and/or un-return a device) and save; confirm the ActDetail card updates immediately without a second click or manual refresh, and that no error toast appears."
    expected: "Save succeeds, detail view reflects new state immediately (reactive refresh), archived flag on parent updates visibly if applicable."
    why_human: "Reactive-refresh wiring (handleReturnSuccess/handleEditSaved) is confirmed in code, but real-time UI behavior (toast timing, flicker, perceived responsiveness) requires human observation."
---

# Phase 22: Правка возвратов Verification Report

**Phase Goal:** Существующий return-акт можно открыть в рабочей форме (диалог «Возврат по акту №XXX») с теми же значениями, что были на момент оформления возврата, и сохранить изменённый возврат без ошибок — с корректной пересборкой эффектов на устройства по дельте.
**Verified:** 2026-07-13T15:02:52Z
**Status:** human_needed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Кнопка «Редактировать» активна на карточке возврата (ROADMAP SC1) | VERIFIED | `ActDetail.svelte:78-79` — `{#if onEdit && (act.act_type === 'handover' || act.act_type === 'return') && !act.archived}` renders an enabled (non-disabled) button for return acts. |
| 2 | Клик открывает диалог «Возврат по акту №XXX», предзаполненный текущими значениями (ROADMAP SC2, D-12/D-13) | VERIFIED | `ActsPage.svelte:154-169` `handleEdit` branches on `act_type==='return'`, fetches parent, opens `ReturnModal` in edit mode. `ReturnModal.svelte:98-134` `$effect` prefill block sets `giverName/receiverName` from `editTarget` (un-swapped, D-12), `returnDateISO` from `editTarget.handover_date_utc`, and per-row `conditionOverride`/`locationOverrideName` from `editTarget.items`. Title: `ReturnModal.svelte:320` `` `Возврат по акту №${displayNumber}` ``, `displayNumber` = `editTarget?.number` in edit mode. |
| 3 | Диалог также предлагает ещё не возвращённые устройства как addable rows | VERIFIED | `ReturnModal.svelte:111-122` builds `addableRows` from `parentAct.items[].outstanding_device_ids`, unchecked by default. |
| 4 | Create-mode payload sends giver_name/receiver_name/handover_date_utc (Pitfall 1 fix reachable via UI, not just backend) | VERIFIED | `ReturnModal.svelte` create-mode submit branch sends `giver_name`/`receiver_name`/date fields (per plan 22-04 key link `giver_name: giverName.trim()`); backend `do_return`'s write-site fix confirmed in `act_service.rs` (plan 22-02). |
| 5 | Сохранение 0 позиций заблокировано (D-10) | VERIFIED | `ReturnModal.svelte:196-198` `canSubmit` returns `false` when `checkedRows.length === 0` (covers both create and edit, since un-checking every row in edit mode drives count to 0). Backend `validate_update_return` (`act_service.rs:1470-1475`) also rejects empty `items` server-side — test `reject_empty_item_set` passes. |
| 6 | Detail-view обновляется реактивно после сохранения без второго клика | VERIFIED | `ActsPage.svelte:194-219` `handleReturnSuccess` assigns the fresh `ActDto` directly to `selectedAct` when the edited return is the one selected (D-11 pattern reused from Phase 19), avoiding a stale re-fetch. |
| 7 | «Комплектация» НЕ редактируется в форме правки возврата (D-14) | VERIFIED | No `kit`/«Комплектация» field present anywhere in `ReturnModal.svelte` (grep: 0 matches). |
| 8 | «Дата архивации» отображается для архивированного parent (D-07, compute-on-read) | VERIFIED | `ActDetail.svelte:45-51,95` renders `archivedAtLabel` only when `act.archived && act.archived_at_utc != null`. Backend: `act_service.rs:3037-3052` `compute_archived_at_utc` — `MAX(handover_date_utc)` over non-deleted return children, `Ok(None)` short-circuit when `!archived`, wired into `get()` at `:2144`. |
| 9 | Все return-edit device/history эффекты применяются как single-writer delta в одной транзакции с per-change audit (D-01) | VERIFIED | `update_return` (`act_service.rs:1540-2040`) runs entirely inside one `writer.execute` closure / one `tx`, with a distinct `audit_repo.insert` call per added/removed/retained-changed device. |
| 10 | Un-return (removing a device) restores its prior в_работе status/location/state (D-09.1) — to the TRUE pre-return snapshot, not an intermediate edit snapshot (CR-02 fix) | VERIFIED | `act_service.rs:1861-1873` step 9 restore via `select_latest_device_mutation`, which now excludes `action != 'custom:return_item_edit'` (`audit_log_sqlite.rs:126`). Regression test `un_return_after_retained_edit_restores_original_pre_return_state` passes (independently re-run: 18/18 green). Unit test `select_latest_device_mutation_excludes_return_item_edit_action` directly asserts the exclusion. |
| 11 | Adding an outstanding device to a return applies do_return-like effects, preserving current location when none supplied (D-09.3 + CR-01 fix) | VERIFIED | `act_service.rs:1925-1939` (step 10 `added` loop): `effective_location = location.or(before.location_id)` — preserves, never NULLs. Test `add_outstanding_device_without_bulk_location_preserves_current_location` passes. |
| 12 | Editing condition/location of a retained returned device updates device + act_items without NULLing untouched location (D-09.2 + CR-01 fix) | VERIFIED | `act_service.rs:1997-2008` (step 11 `retained_with_change` loop): same `location.or(before.location_id)` preservation pattern. Test `retained_edit_condition_only_preserves_location` passes. |
| 13 | Un-returning/re-editing a drifted device (reissued or manually relocated) is rejected with Conflict, no force-override (D-11) | VERIFIED | Step 8b guard (`act_service.rs` ~1776-1859) compares current device state vs `select_latest_device_mutation_pair`'s `after_json`. Tests `reject_un_return_after_reissue` and `reject_edit_after_manual_device_relocation` pass. |
| 14 | Archived flag on parent flips both directions correctly (add-last-device archives, un-return of last returned unarchives) | VERIFIED | Tests `add_last_device_archives_parent` and `un_return_unarchives_parent` pass. |
| 15 | Optimistic concurrency: stale version → OptimisticLockMismatch, never silent overwrite (D-02) | VERIFIED | `act_service.rs:1573-1580` CAS pre-check; test `version_mismatch_returns_conflict` passes. |
| 16 | acts_update_return reachable identically via Tauri invoke and HTTP, Employee gets 403 on both | VERIFIED | `tauri_cmds/acts.rs` `build_acts_update_return` + `http/acts.rs` handler both gate on `Action::MutateActs`. `role_endpoint_matrix.rs` Case 43 (`:1457-1473`) asserts Employee → `POST /api/v1/acts_update_return` → 403. |
| 17 | CR-01 fixed: condition-only edit with empty bulk location does not NULL retained device's location | VERIFIED | Same as #12 — code + passing regression test, independently re-run green. |
| 18 | CR-02 fixed: un-return restores TRUE pre-return snapshot, not latest same-act edit | VERIFIED | Same as #10 — code + passing regression + unit test, independently re-run green. |
| 19 | WR-01 fixed: validate_update_return has dedup/non-empty/per-item-override parity with validate_return | VERIFIED | `act_service.rs:1466-1518`. Tests `reject_update_return_duplicate_device_id_across_items`, `reject_update_return_missing_override_when_apply_to_all_false` pass. |
| 20 | WR-02 fixed: no `.expect()` panic on parent_act_id inside single-writer closure | VERIFIED | `grep '\.expect("return act always has parent_act_id")'` → 0 matches. Code at `act_service.rs:1589-1594` uses `.ok_or_else(|| AppError::Internal {...})?`. Test `update_return_null_parent_act_id_returns_error_not_panic` passes. |
| 21 | WR-03 fixed: `added` loop enforces already_returned+qty<=handover_qty bound; WR-04: V034 comment corrected | VERIFIED | `act_service.rs:1733-1773` bound check; test `reject_add_when_device_already_returned_elsewhere_under_parent` passes. `migrations/V034__return_handover_date_backfill.sql:19` now reads "NOT safe to run manually after Phase 22 ships" (grep for "naturally idempotent" → 0). |

**Score:** 21/21 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/trackly-app/src/dto/act.rs` | `ActUpdateReturnDto`, extended `ActReturnDto`/`ActItemDto`/`ActDto` | VERIFIED | Present per 22-01/22-03 plans; consumed throughout `act_service.rs`, `acts.rs` transports, `ReturnModal.svelte`. |
| `crates/trackly-infra/src/repos/audit_log_sqlite.rs` | `select_latest_device_mutation_pair`, CR-02 exclusion in `select_latest_device_mutation` | VERIFIED | Both functions present (`:114-162`), exclusion clause `action != 'custom:return_item_edit'` confirmed, unit test confirms behavior. |
| `migrations/V034__return_handover_date_backfill.sql` | Backfill + corrected non-idempotency comment (WR-04) | VERIFIED | `UPDATE acts SET handover_date_utc...` present; comment rewritten. |
| `crates/trackly-app/src/services/act_service.rs` | `update_return()`, `validate_update_return()`, CR-01/CR-02/WR-01/WR-02/WR-03 fixes | VERIFIED | All confirmed present and substantive (see truths table). Not a stub — ~500 lines of delta-reconciliation logic. |
| `crates/trackly-app/tests/acts_update_return.rs` | Regression test suite | VERIFIED | 18 tests present, all passing (independently re-run: `18 passed; 0 failed`). |
| `crates/trackly-app/src/tauri_cmds/acts.rs`, `src/http/acts.rs` | Tauri + HTTP transports for `acts_update_return` | VERIFIED | Both present, gated on `Action::MutateActs`. |
| `ui/src/lib/api/acts.ts` | `acts.updateReturn` client method | VERIFIED | Present, called from `ReturnModal.svelte:241`. |
| `ui/src/features/acts/ReturnModal.svelte` | Edit-mode UI (mode prop, editTarget/parentAct, dual-source prefill, un-swapped ФИО, Дата возврата) | VERIFIED | All confirmed present and wired (see truths 2-8). |
| `ui/src/features/acts/ActDetail.svelte` | Edit-gate includes return acts; «Дата архивации» display | VERIFIED | Confirmed (truths 1, 8). |
| `ui/src/features/acts/ActsPage.svelte` | `handleEdit` branches on act_type, reactive refresh | VERIFIED | Confirmed (truths 2, 6). |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| `ReturnModal.svelte` submit (edit mode) | `acts.updateReturn` | `ActUpdateReturnDto` payload | WIRED | `ReturnModal.svelte:241` `acts.updateReturn(updatePayload)`. |
| `acts.updateReturn` | `POST /api/v1/acts_update_return` | `apiCall('acts_update_return', ...)` | WIRED | Confirmed in `ui/src/lib/api/acts.ts`. |
| `build_acts_update_return` | `authorize(caller, &Action::MutateActs)` | same Action as sibling act mutations | WIRED | Confirmed in `tauri_cmds/acts.rs`. |
| `ActDetail.svelte` «Редактировать» button | `ActsPage.svelte handleEdit` | `onEdit(act)` callback | WIRED | Confirmed. |
| `ActsPage.svelte handleEdit` (return branch) | `acts.get(act.parent_act_id)` | await before opening modal | WIRED | Confirmed `ActsPage.svelte:157`. |
| `update_return` step 9 (un-return restore) | `select_latest_device_mutation` | exclusion of `custom:return_item_edit` | WIRED | Confirmed, regression-tested at both unit and integration level. |
| `update_return` step 11 (retained edit) | `audit_log` `AuditEntry.action` | `"custom:return_item_edit"` tag | WIRED | Confirmed `act_service.rs:2018`. |
| `validate_update_return` | `validate_return` (source pattern) | dedup / non-empty / per-item-override mirroring | WIRED | Confirmed, tests pass. |
| `update_return` step 8a (added loop) | `do_return`'s already_returned/handover_qty guard (source pattern) | SUM(quantity) bound check | WIRED | Confirmed, test passes. |

### Data-Flow Trace (Level 4)

Not applicable in the dashboard sense — this phase's "data flow" is the backend delta-reconciliation transaction, which is traced exhaustively above (truths 9-21) rather than a component rendering a fetched list. The relevant trace (DTO → service → repo → audit log → UI prefill) was followed end-to-end and confirmed non-hollow at every hop (regression tests exercise the full chain, not mocks).

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| acts_update_return regression suite | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test acts_update_return -- --test-threads=1` | `18 passed; 0 failed` (independently re-run by verifier) | PASS |
| CR-02 exclusion unit-level assertion | Read `select_latest_device_mutation_excludes_return_item_edit_action` test body | Asserts older `"update"` row's `before_json` is returned, newer `"custom:return_item_edit"` row is skipped | PASS |
| Debt-marker scan on all phase-modified files | `grep -n "TBD\|FIXME\|XXX"` across all key-files | 0 matches | PASS |
| V034 comment correction | `grep "naturally idempotent"` / `grep "NOT safe to run manually"` | 0 / present | PASS |

### Probe Execution

Not applicable — this is not a migration/tooling phase with dedicated probe scripts; no `scripts/*/tests/probe-*.sh` declared in any of the 6 plans or found under `scripts/`.

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|--------------|--------|----------|
| ACT-03 | 22-01, 22-02, 22-03, 22-04, 22-05, 22-06 | Пользователь может отредактировать существующий return-акт | SATISFIED | All 3 ROADMAP success criteria verified (truths 1-3 map directly); no orphaned requirements — `REQUIREMENTS.md` maps only ACT-03 to Phase 22, and all 6 plans declare `requirements: [ACT-03]`. |

No orphaned requirements found.

### Anti-Patterns Found

None. Scanned all key-files across all 6 plans (`act_service.rs`, `acts_update_return.rs`, `audit_log_sqlite.rs`, `V034__return_handover_date_backfill.sql`, `ReturnModal.svelte`, `ActDetail.svelte`, `ActsPage.svelte`, `dto/act.rs`, `tauri_cmds/acts.rs`, `http/acts.rs`) for `TBD`/`FIXME`/`XXX`/`TODO`/`HACK`/`PLACEHOLDER`/"not yet implemented" patterns — zero matches. The only `placeholder` hits in `ReturnModal.svelte` are HTML `<input placeholder="...">` attributes (legitimate UI hints, not code stubs).

### Human Verification Required

### 1. Visual prefill fidelity in a real browser/webview session

**Test:** Open the ReturnModal edit dialog for a real, previously-created return act (with a device that has a location + condition set) and visually confirm the dialog title reads «Возврат по акту №XXX» and every prefilled field (checked devices, per-row состояние, per-row расположение, ФИО giver/receiver un-swapped, «Дата возврата») matches exactly what was recorded at return time.
**Expected:** Dialog opens instantly with no flash of empty/default values; all fields show the return's own saved values, not the parent handover's values.
**Why human:** Field-by-field data wiring is confirmed in code (`ReturnModal.svelte` `$effect` prefill block) and by backend regression tests, but actual visual rendering, dropdown/autocomplete behavior, and absence of a prefill flash can only be judged in a running browser/webview.

### 2. Reactive save-and-refresh UX

**Test:** Edit a real return (change condition and/or un-return a device) and save; confirm the ActDetail card updates immediately without a second click or manual refresh, and that no error toast appears.
**Expected:** Save succeeds, detail view reflects new state immediately (reactive refresh), archived flag on parent updates visibly if applicable.
**Why human:** Reactive-refresh wiring (`handleReturnSuccess`/`handleEditSaved`) is confirmed in code, but real-time UI behavior (toast timing, flicker, perceived responsiveness) requires human observation.

### Gaps Summary

No gaps found. Both BLOCKER-severity findings (CR-01, CR-02) and all four WARNING findings (WR-01..WR-04) plus the INFO finding (IN-01) from `22-REVIEW.md` are confirmed fixed in the codebase, each backed by a passing regression test that was independently re-run by this verifier (18/18 green in `acts_update_return.rs`, plus a direct unit-test read of `select_latest_device_mutation_excludes_return_item_edit_action`). All three ROADMAP success criteria are observably true in the code. All 6 plans declare `requirements: [ACT-03]` and no orphaned requirement mappings exist. The only outstanding items are two human-verification checks (visual/UX confirmation) that cannot be settled by static code inspection — these gate `status: human_needed` per the verification decision tree, not because any defect was found.

---

_Verified: 2026-07-13T15:02:52Z_
_Verifier: Claude (gsd-verifier)_
