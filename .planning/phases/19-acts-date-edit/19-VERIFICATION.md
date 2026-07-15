---
phase: 19-acts-date-edit
verified: 2026-07-12T02:55:00Z
status: verified
human_verification_completed: 2026-07-15  # 7/7 live UAT passed, see 19-HUMAN-UAT.md
score: 3/3 truths verified
overrides_applied: 0
re_verification:
  previous_status: gaps_found
  previous_score: 2/3 truths verified (1 partial with confirmed defect)
  gaps_closed:
    - "CR-01 BLOCKER: ActService::update() never recomputed acts.archived after item-set changes — closed by Plan 19-06 (gated recompute_parent_archived call, 2 new regression tests, independently re-run 13/13 green)"
    - "WR-01: rename of a handover with existing return acts leaked the old act number — closed by Plan 19-07 (same-tx cascade UPDATE to child return acts, rename_with_return_frees_old_number test verified)"
    - "WR-03: retained-item комплектация edits were untraceable (no audit row) — closed by Plan 19-07 (conditional custom:act_item_complectation_edit audit row, complectation_edit_writes_audit test verified)"
    - "WR-02: edit-mode group/quantity picker silently dropped N-1 devices — closed by Plan 19-08 (mode==='edit' gating clamps added rows to a single device at both pick handlers and qty-column render)"
    - "IN-01: todayISO() used local calendar accessors inconsistent with the UTC unixToIso()/isoToUnix() pipeline — closed by Plan 19-08 (todayISO() switched to getUTCFullYear/getUTCMonth/getUTCDate)"
  gaps_remaining: []
  regressions: []
human_verification:
  - test: "Открыть Acts page (Tauri desktop, затем повторно через LAN-браузер после `pnpm --dir ui build`). Выбрать существующий handover-акт. Убедиться, что «Редактировать» активна; нажать — форма открывается предзаполненной текущими данными акта (№, даты, Сдал/Принял, Расположение, Заметки) и текущими позициями."
    expected: "Форма открывается с корректными предзаполненными значениями во всех полях, включая позиции устройств."
    why_human: "Визуальная проверка предзаполнения формы и её вида в реальном браузере/десктопе — не может быть проверена статическим анализом кода."
  - test: "Изменить поле шапки (например, Сдал) и сохранить."
    expected: "Появляется тост об успехе; детальный просмотр немедленно отражает новое значение."
    why_human: "UX-поведение тоста и немедленность обновления UI требуют живой сессии."
  - test: "Добавить позицию со склада, сохранить."
    expected: "Устройство переходит из «на складе» в «в работе» (проверить на странице Devices); новая позиция появляется в списке позиций акта. В edit-режиме поле количества показывает статичную «1» (не редактируемый спиннер) — визуально подтвердить закрытие WR-02."
    why_human: "Межстраничная проверка состояния устройства и визуальное подтверждение qty=1 UI требуют живого запуска приложения."
  - test: "Убрать существующую позицию, сохранить."
    expected: "Устройство возвращается к состоянию/расположению непосредственно перед последним изменением (проверить на странице Devices); позиция исчезает из списка акта."
    why_human: "Проверка фактического состояния устройства после отката требует живого запуска."
  - test: "Отредактировать поле «Комплектация» на сохранённой (retained) позиции, сохранить."
    expected: "Значение сохраняется и видно при повторном открытии формы редактирования. (Бэкенд-запись audit-строки уже подтверждена автотестом complectation_edit_writes_audit — здесь проверяется только UI round-trip.)"
    why_human: "UI round-trip проверка требует живой сессии."
  - test: "Открыть один и тот же акт в двух вкладках браузера, сохранить из вкладки 1, затем попытаться сохранить из вкладки 2 (устаревшая версия)."
    expected: "Появляется тост «изменён другим пользователем» (409/OptimisticLockMismatch), а не силентная ошибка или общая ошибка."
    why_human: "Многовкладочный конкурентный сценарий требует живой сессии с двумя вкладками."
  - test: "Выбрать return-акт — убедиться, что «Редактировать» задизейблена с тултипом; выбрать АРХИВНЫЙ handover-акт — убедиться, что «Редактировать» по-прежнему активна. Затем добавить устройство к архивному акту через Редактировать и убедиться, что после сохранения акт становится НЕ архивным, а устройство теперь возвращаемо через «Возврат»."
    expected: "Return-акт: кнопка задизейблена, тултип объясняет причину. Архивный handover-акт: кнопка активна. После добавления устройства к архивному акту акт становится archived=false и «Возврат» становится доступна для нового устройства (подтверждает практический эффект CR-01-fix в живом UI — уже подтверждено автотестом add_device_to_archived_unarchives на уровне сервиса)."
    why_human: "Визуальная проверка disabled-состояния, тултипа и end-to-end UI-эффекта CR-01-фикса в реальном приложении."
---

# Phase 19: Дата акта и редактирование акта — Verification Report

**Phase Goal:** Дата, введённая пользователем при создании акта, используется как дата акта, а существующий акт можно открыть в рабочей форме редактирования и сохранить изменения.

**Verified:** 2026-07-12
**Status:** human_needed
**Re-verification:** Yes — after gap closure (Plans 19-06, 19-07, 19-08)

## Goal Achievement

### Observable Truths (Roadmap Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | При создании акта значение поля «Когда отдали» сохраняется как дата акта — не подставляется автоматически текущая дата | ✓ VERIFIED | Unchanged from prior verification (regression re-confirmed): `acts_date_source.rs` 2/2 pass (independently re-run). No file touched by this re-verification round affects ACT-01's date-source path except `todayISO()` (create-mode default value only, not the persisted date-source logic). |
| 2 | Кнопка «Редактировать» на карточке существующего акта активна (не задизейблена) | ✓ VERIFIED | Unchanged from prior verification (regression re-confirmed): `ActDetail.svelte:70-81` still enables the button for all handover acts (including archived) and disables only for return-type acts. `ActsPage.svelte:145,150,247` still wires `handleEdit`/`handleEditSaved`/`onEdit={handleEdit}`. |
| 3 | Нажатие «Редактировать» открывает форму со всеми текущими данными акта, и внесённые изменения сохраняются без ошибок — **включая корректный пересчёт производного состояния (archived), номер-каскад и аудит комплектации** | ✓ VERIFIED | CR-01 BLOCKER closed: `crates/trackly-app/src/services/act_service.rs:935` — `recompute_parent_archived(&tx, payload.id, now)?;` now runs inside `update()`'s transaction, placed at step 9a (immediately after the step-9 CAS `update_act_header_in_tx` call at line 878, before the step-10 final-audit fetch at line ~957), gated on `if !added.is_empty() \|\| !removed.is_empty()`. Sequencing matches the plan's hazard analysis exactly (recompute after CAS to avoid a spurious OptimisticLockMismatch). Confirmed by direct source reading, not just SUMMARY claims. Independently re-run: `cargo test -p trackly-app --test acts_update` = 13/13 passed (incl. `remove_last_outstanding_archives_act`, `add_device_to_archived_unarchives`, `rename_with_return_frees_old_number`, `complectation_edit_writes_audit`, and the pre-existing `header_only_edit_does_not_touch_devices` version-gate). WR-01 (number cascade, `act_service.rs` step 9b, `UPDATE acts SET number=?1, updated_at_utc=?2 WHERE parent_act_id=?3 AND deleted_at_utc IS NULL`) and WR-03 (`custom:act_item_complectation_edit` audit row gated on stored != incoming, `act_service.rs` step 7) both confirmed present in source and covered by passing tests. WR-02 (edit-mode single-device clamp) and IN-01 (UTC todayISO) both confirmed present in `ActFormItemsTable.svelte`/`ActFormBody.svelte` source. |

**Score:** 3/3 truths fully verified (Truth 3 upgraded from ⚠ PARTIAL to ✓ VERIFIED — the CR-01 gap that caused the partial rating is closed).

### Deferred Items

None.

### Required Artifacts

| Artifact | Expected | Status | Details |
|---|---|---|---|
| `crates/trackly-app/src/services/act_service.rs` | `recompute_parent_archived` called inside `update()` | ✓ VERIFIED | Line 935, gated, correctly sequenced after CAS header UPDATE (line 878) and before final-audit fetch. Confirmed via direct read, not grep-only. |
| `crates/trackly-app/src/services/act_service.rs` | Number-rename cascade to child return acts (WR-01) | ✓ VERIFIED | `UPDATE acts SET number = ?1, updated_at_utc = ?2 WHERE parent_act_id = ?3 AND deleted_at_utc IS NULL`, co-located with the `custom:act_number_override` audit insert in step 9b, gated on `n != act.number`. |
| `crates/trackly-app/src/services/act_service.rs` | `custom:act_item_complectation_edit` audit row (WR-03) | ✓ VERIFIED | Step 7: SELECT-before-UPDATE equality guard (`stored.as_deref() != Some(v.as_str())`), UPDATE + `audit_repo.insert` only fire on real change. |
| `crates/trackly-app/tests/acts_update.rs` | 13 integration tests (9 original + 4 new) | ✓ VERIFIED | All 13 present and independently re-run: 13/13 passed. New tests (`remove_last_outstanding_archives_act`, `add_device_to_archived_unarchives`, `rename_with_return_frees_old_number`, `complectation_edit_writes_audit`) assert substantive outcomes (archived flag flips, number reuse, audit row count + before/after JSON content), not trivial/stub assertions. |
| `ui/src/features/acts/ActFormItemsTable.svelte` | Edit-mode single-device clamp (WR-02) | ✓ VERIFIED | `mode === 'edit'` gating on both `pickDevice`/pick-group handlers (lines 338, 451 — `quantity: hasSerial \|\| mode === 'edit' ? 1 : ...`) and the qty-column render (line 685 — static `<span class="qty-fixed">1</span>` instead of the editable spinner). |
| `ui/src/features/acts/ActFormBody.svelte` | `todayISO()` on UTC accessors (IN-01) | ✓ VERIFIED | Lines 46-48 use `getUTCFullYear()`/`getUTCMonth()`/`getUTCDate()`; zero remaining local-calendar accessor occurrences confirmed via grep. |

### Key Link Verification

| From | To | Via | Status | Details |
|---|---|---|---|---|
| `ActService::update` (add/remove loops) | `acts.archived` | `recompute_parent_archived(&tx, payload.id, now)` | ✓ WIRED | The one previously-broken link from the prior verification round. Now confirmed present and correctly sequenced. |
| `ActService::update` (number rename) | child return acts' `number` | `UPDATE ... WHERE parent_act_id = ?3` | ✓ WIRED | Confirmed at step 9b, gated on actual rename. |
| `ActService::update` (step 7 комплектация change) | `audit_log` | `audit_repo.insert(custom:act_item_complectation_edit)` | ✓ WIRED | Confirmed gated on stored != incoming; no-op resubmit writes nothing (asserted by test). |
| `ActFormItemsTable` pick handlers (edit mode) | `ActUpdateItemDto` (device_id only) | `mode === 'edit'` clamp to quantity=1 | ✓ WIRED | Visible quantity now always matches what is persisted; no silent N-1 drop possible via the picker UI. |

### Behavioral Spot-Checks (automated test re-execution, independently run by this verifier)

| Behavior | Command | Result | Status |
|---|---|---|---|
| ACT-02 backend update() behaviors incl. all 4 gap-closure regressions | `cargo test -p trackly-app --test acts_update -- --test-threads=1` | 13/13 passed | ✓ PASS |
| ACT-01 sort/render regression (unaffected by this round) | `cargo test -p trackly-app --test acts_date_source -- --test-threads=1` | 2/2 passed | ✓ PASS |
| RBAC regression (unaffected by this round) | `cargo test -p trackly-app --test role_endpoint_matrix -- --test-threads=1` | 1/1 passed | ✓ PASS |
| Act-number display formatting (unaffected by this round) | `cargo test -p trackly-app --test acts_display_rule -- --test-threads=1` | 4/4 passed | ✓ PASS |
| Frontend types/build | `pnpm --dir ui exec svelte-check` | 0 errors, 48 pre-existing unrelated warnings | ✓ PASS |
| Backend lints on gap-closure files | `cargo clippy -p trackly-app --tests -- -D warnings` | clean, 0 warnings | ✓ PASS |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|---|---|---|---|---|
| ACT-01 | 19-01 (+19-08 create-mode `todayISO` UTC unification, incidental) | Использование handover_date_utc как даты акта везде (список/карточка/PDF) | ✓ SATISFIED | Unchanged from prior verification; REQUIREMENTS.md marks Complete, matches code. |
| ACT-02 | 19-02, 19-03, 19-04, 19-05, 19-06, 19-07, 19-08 | Пользователь может отредактировать существующий акт | ✓ SATISFIED | CR-01 gap that previously downgraded this to "MOSTLY SATISFIED" is now closed. REQUIREMENTS.md line 23 already annotates: "(gap CR-01 — reconcile archived при edit — закрыт в Plan 19-06)" and line 71 marks status "Complete" — matches the verified state of the code. |

No orphaned requirements — both ACT-01 and ACT-02 are declared across plan frontmatter (19-01 through 19-08) and both map cleanly to the two ROADMAP success-criteria clusters. REQUIREMENTS.md's Complete markers for both now accurately reflect the codebase.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|---|---|---|---|---|
| `ui/src/features/acts/ActFormBody.svelte` | 184 | `// location_id wiring TODO: Phase 2 currently stores...` | ℹ️ Info | Pre-existing since Plan 19-05 (commit `0b8af64`, not introduced by the gap-closure plans 19-06/07/08). On the CREATE-mode payload path only (location_id intentionally sent as null on create, per an already-documented, scoped design decision — not an unresolved code-path defect). Was not flagged in the prior (gaps_found) verification round either. Does not block ACT-01/ACT-02 truths and is unrelated to CR-01/WR-01/WR-02/WR-03/IN-01. Not treated as a blocker for this re-verification. |
| No `TBD`/`FIXME`/`XXX` markers found in any file modified by Plans 19-06/19-07/19-08 | — | — | — | Debt-marker gate: clean for the gap-closure diffs themselves. |

## Human Verification Required

7 items carried forward from the prior (initial) verification report — all remain manual UAT items requiring a live desktop/browser session; none are blockers to a `passed`-eligible automated result. One item (act-editing gate + archived-flag UI effect) has been updated to explicitly note that its underlying CR-01 defect is now fixed at the service layer and only needs a live-UI confirmation pass, not a code fix.

## Gaps Summary

None remaining. All 5 findings from `19-REVIEW.md` (CR-01 blocker, WR-01, WR-02, WR-03, IN-01) are closed and independently re-verified against source code and freshly re-run tests (not SUMMARY.md claims alone):

- **CR-01** (blocker): `recompute_parent_archived` now called inside `update()`, correctly gated and sequenced. Confirmed by reading `act_service.rs:935` directly and by independently re-running `remove_last_outstanding_archives_act` and `add_device_to_archived_unarchives`.
- **WR-01**: number-rename cascade confirmed present and tested (`rename_with_return_frees_old_number`).
- **WR-02**: edit-mode single-device clamp confirmed present in both mutation and render sites.
- **WR-03**: комплектация audit row confirmed present and tested (`complectation_edit_writes_audit`).
- **IN-01**: `todayISO()` confirmed on UTC accessors.

All automated must-haves pass. Status is `human_needed` (not `passed`) solely because 7 UAT items require a live desktop/browser session that this verifier cannot execute — per the workflow's decision tree, human verification items take priority over an all-green automated score for the final status classification.

---

*Verified: 2026-07-12*
*Verifier: Claude (gsd-verifier)*
