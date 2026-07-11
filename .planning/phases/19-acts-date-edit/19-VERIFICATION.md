---
phase: 19-acts-date-edit
verified: 2026-07-12T02:30:00Z
status: gaps_found
score: 2/3 truths verified (1 partial with confirmed defect)
overrides_applied: 0
gaps:
  - truth: "Нажатие «Редактировать» открывает форму со всеми текущими данными акта, и внесённые изменения сохраняются без ошибок (для ВСЕХ сценариев редактирования)"
    status: partial
    reason: >
      Core mechanics (button enabled, form opens prefilled from acts.get(id),
      header/position/комплектация edits persist, CAS/optimistic-lock, D-07/D-08
      guards) are implemented and covered by 9 passing integration tests
      (acts_update.rs) plus verified frontend wiring. However, ActService::update
      never calls recompute_parent_archived (crates/trackly-app/src/services/act_service.rs,
      full body of update(), lines 578-953) while every sibling mutation that
      changes the handover/return device balance does (do_return line 1333,
      delete_soft line 1750). Confirmed by direct code inspection — recompute_parent_archived
      does not appear anywhere inside update()'s transaction body. Two
      UI-reachable, no-error-surfaced scenarios leave acts.archived inconsistent
      with the true outstanding-device count: (1) adding a device to an already-
      archived handover act via Edit (enabled by design, D-07) transitions the
      device to в_работе but the act stays archived — the device becomes
      unreturnable via UI (Возврат is disabled for archived acts); (2) removing
      the last outstanding device from a non-archived act (allowed since it's
      outstanding) leaves the act non-archived even though handover_total <=
      returned_total, so it stays visibly "active" with zero real outstanding
      devices. Both are silent (no error toast, no validation failure) — the
      save reports success while corrupting derived state, directly undermining
      the phase's/project's "работает надёжно" requirement for saved edits.
      No test in acts_update.rs exercises `archived` (confirmed via grep — zero
      matches). This was independently confirmed via code review (19-REVIEW.md
      CR-01, critical) and by this verifier reading the same source.
    artifacts:
      - path: "crates/trackly-app/src/services/act_service.rs"
        issue: "ActService::update (lines 578-953) never calls recompute_parent_archived after the add/remove device-item loops, unlike do_return (line 1333) and delete_soft (line 1750)"
    missing:
      - "Call `recompute_parent_archived(&tx, payload.id, now)` inside update()'s transaction, after the item add/remove loops resolve the final item set and before (or immediately after) update_act_header_in_tx, accounting for the version bump it performs so the CAS header UPDATE and returned ActDto.version stay consistent."
      - "Regression test: create a 2-device handover, return one device (do_return), then remove the other device via update() — assert the act becomes archived=true afterward."
      - "Regression test: on an archived handover act, add a new device via update() — assert the act's archived flag correctly reflects the new outstanding device (archived=false) OR document/enforce a different intended behavior if archived-act editing is meant to keep archived=true by design (currently undefined/inconsistent)."
deferred: []
human_verification:
  - test: "Открыть Acts page (Tauri desktop, затем повторно через LAN-браузер после `pnpm --dir ui build`). Выбрать существующий handover-акт. Убедиться, что «Редактировать» активна; нажать — форма открывается предзаполненной текущими данными шапки (№, даты, Сдал/Принял, Расположение, Заметки) и текущими позициями."
    expected: "Форма открывается с корректными предзаполненными значениями во всех полях, включая позиции устройств."
    why_human: "Визуальная проверка предзаполнения формы и её вида в реальном браузере/десктопе — не может быть проверена статическим анализом кода."
  - test: "Изменить поле шапки (например, Сдал) и сохранить."
    expected: "Появляется тост об успехе; детальный просмотр немедленно отражает новое значение."
    why_human: "UX-поведение тоста и немедленность обновления UI требуют живой сессии."
  - test: "Добавить позицию со склада, сохранить."
    expected: "Устройство переходит из «на складе» в «в работе» (проверить на странице Devices); новая позиция появляется в списке позиций акта."
    why_human: "Межстраничная проверка состояния устройства требует живого запуска приложения."
  - test: "Убрать существующую позицию, сохранить."
    expected: "Устройство возвращается к состоянию/расположению непосредственно перед последним изменением (проверить на странице Devices); позиция исчезает из списка акта."
    why_human: "Проверка фактического состояния устройства после отката требует живого запуска."
  - test: "Отредактировать поле «Комплектация» на сохранённой (retained) позиции, сохранить."
    expected: "Значение сохраняется и видно при повторном открытии формы редактирования."
    why_human: "UI round-trip проверка требует живой сессии."
  - test: "Открыть один и тот же акт в двух вкладках браузера, сохранить из вкладки 1, затем попытаться сохранить из вкладки 2 (устаревшая версия)."
    expected: "Появляется тост «изменён другим пользователем» (409/OptimisticLockMismatch), а не силентная ошибка или общая ошибка."
    why_human: "Многовкладочный конкурентный сценарий требует живой сессии с двумя вкладками."
  - test: "Выбрать return-акт — убедиться, что «Редактировать» задизейблена с тултипом; выбрать АРХИВНЫЙ handover-акт — убедиться, что «Редактировать» по-прежнему активна."
    expected: "Return-акт: кнопка задизейблена, тултип объясняет причину. Архивный handover-акт: кнопка активна (в отличие от «Возврат», которая для архивных задизейблена)."
    why_human: "Визуальная проверка disabled-состояния и тултипа в реальном UI."
  - test: "(Дополнительно, вытекает из CR-01/gap выше) Добавить устройство к архивному акту через Редактировать, затем попытаться его вернуть через Возврат."
    expected: "Ожидание команды: устройство должно быть возвращаемым. Фактическое поведение (при текущем коде): акт остаётся archived=true, кнопка «Возврат» недоступна для архивного акта — устройство «застревает» в статусе «в работе» без пути возврата через UI."
    why_human: "Подтверждает практический эффект CR-01 в реальном UI; технически уже подтверждено анализом кода, но стоит проверить визуально перед принятием решения об исправлении."
---

# Phase 19: Дата акта и редактирование акта — Verification Report

**Phase Goal:** Дата, введённая пользователем при создании акта, используется как дата акта, а существующий акт можно открыть в рабочей форме редактирования и сохранить изменения.

**Verified:** 2026-07-12
**Status:** gaps_found
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths (Roadmap Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | При создании акта значение поля «Когда отдали» сохраняется как дата акта — не подставляется автоматически текущая дата | ✓ VERIFIED | `ActDto.handover_date_utc` present in Rust struct + generated `bindings.ts` (grep confirmed). `list()`/`search_acts()` in `acts_sqlite.rs` sort `ORDER BY a.handover_date_utc DESC` (both call sites, lines 295/601 — no `created_at_utc` ORDER BY remains). `render_pdf`'s act+parent date blocks read `act.handover_date_utc`/`parent.handover_date_utc` (lines 1851-1852, 1895-1896). `ActListRow.svelte`/`ActDetail.svelte` derive displayed date from `act.handover_date_utc`. Regression tests `acts_date_source.rs` (2/2 pass) and 2 new `html_act_render.rs` tests (8/8 pass in that file) independently re-run and green. |
| 2 | Кнопка «Редактировать» на карточке существующего акта активна (не задизейблена) | ✓ VERIFIED | `ActDetail.svelte:70-81` — button is enabled (`{#if onEdit && act.act_type === 'handover'}`) for all handover acts, including archived ones (no `!act.archived` condition, confirmed by grep — that identifier only appears in the separate «Возврат» button's condition). Disabled with an explanatory tooltip only for return-type acts (D-07). `ActsPage.svelte` wires `onEdit={handleEdit}` into `<ActDetail>` (line 247) — the button is never left unsupplied, unlike the pre-phase-19 state RESEARCH.md diagnosed. |
| 3 | Нажатие «Редактировать» открывает форму со всеми текущими данными акта, и внесённые изменения сохраняются без ошибок | ⚠ PARTIAL / GAP | Mechanically wired and tested for the core scenarios: `ActFormBody.svelte` prefills all header fields + positions from `initialAct` (an `acts.get(id)` result, not a stale list row — Pitfall 5 honored); `handleSubmit`'s edit branch builds `ActUpdateDto` and calls `acts.update()` → `POST /api/v1/acts_update` / Tauri `acts_update` → `build_acts_update` (RBAC-gated, `Case 42` 403-for-Employee test passes) → `ActService::update`. Backend: 9/9 `acts_update.rs` integration tests independently re-run and pass (header-only edit is device-inert, add transitions device, CAS/`OptimisticLockMismatch`, D-07 return-act rejection, D-06 remove restores most-recent snapshot, D-08 rejects removal of returned device, number-uniqueness re-check). **However:** confirmed by direct code reading that `ActService::update` (act_service.rs:578-953) never calls `recompute_parent_archived`, unlike `do_return` (line 1333) and `delete_soft` (line 1750) — a real, reproducible defect (matches 19-REVIEW.md CR-01, independently reconfirmed here) that silently corrupts the `archived` flag in two UI-reachable edit scenarios (see Gaps below). No test in `acts_update.rs` exercises `archived` (grep returns zero matches). |

**Score:** 2/3 truths fully verified; 1 truth partially verified with a confirmed functional gap.

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|---|---|---|---|---|
| ACT-01 | 19-01 | Использование handover_date_utc как даты акта везде (список/карточка/PDF) | ✓ SATISFIED | Verified above (Truth 1). REQUIREMENTS.md marks Complete — matches actual code. |
| ACT-02 | 19-02, 19-03, 19-04, 19-05 | Пользователь может отредактировать существующий акт | ⚠ MOSTLY SATISFIED (gap) | Button active + form + save path fully wired and tested (Truths 2-3), but CR-01's `archived`-recompute omission is a real, unaddressed defect within the same code path REQUIREMENTS.md marks "Complete." REQUIREMENTS.md's binary Complete status does not reflect this known gap. |

No orphaned requirements — both ACT-01 and ACT-02 are declared in plan frontmatter (`requirements:` fields across 19-01 through 19-05) and both map cleanly to the two ROADMAP success-criteria clusters.

### Required Artifacts

| Artifact | Expected | Status | Details |
|---|---|---|---|
| `crates/trackly-app/src/dto/act.rs` | `ActDto.handover_date_utc`, `ActUpdateDto`/`ActUpdateItemDto` | ✓ VERIFIED | All fields present, `snake_case_json_invariant`-style tests pass. |
| `crates/trackly-infra/src/repos/acts_sqlite.rs` | ORDER BY handover_date_utc; `update_act_header_in_tx`; `recompute_parent_archived` (pre-existing) | ✓ VERIFIED (sort+CAS) / ⚠ NOT CALLED from update() | Sort order and CAS UPDATE both correct; `recompute_parent_archived` exists but its only two callers remain `do_return`/`delete_soft` — `update()` is not a third caller (the gap). |
| `crates/trackly-infra/src/repos/audit_log_sqlite.rs` | `select_latest_device_mutation` | ✓ VERIFIED | Present, `ORDER BY ... DESC LIMIT 1`, used correctly by `update()`'s remove-device restore path (proven by `double_edit_restores_most_recent_snapshot` test). |
| `crates/trackly-app/src/services/act_service.rs` | `ActService::update` + `validate_update` + `populate_outstanding_device_ids_in_tx` | ✓ VERIFIED (existence/wiring) / ⚠ GAP (archived recompute) | Function exists, compiles, all 9 targeted tests pass; missing `recompute_parent_archived` call (see Gaps). |
| `crates/trackly-app/tests/acts_update.rs` | 9 integration tests | ✓ VERIFIED | All 9 re-run independently, all pass. Zero `archived`-flag assertions (confirms the gap is untested, not just unfixed). |
| `crates/trackly-app/src/tauri_cmds/acts.rs`, `src/http/acts.rs` | `build_acts_update`/`acts_update` (Tauri) + `handler_update`/route (axum) | ✓ VERIFIED | Both present, both delegate to the same `build_acts_update`; RBAC regression (`role_endpoint_matrix.rs`, Case 42) independently re-run, passes. |
| `ui/src/lib/api/acts.ts` | `acts.update(payload)` client | ✓ VERIFIED | Present, typed against `ActUpdateDto`/`ActDto`. |
| `ui/src/features/acts/ActFormBody.svelte`, `ActFormItemsTable.svelte`, `ActFormModal.svelte` | edit-mode prefill/submit, комплектация editing, mode-aware modal | ✓ VERIFIED | Confirmed via grep: `mode === 'edit'` branches present in state-init and submit; `complectation_at_time` field + conditional input present; specs/devices.notes correctly kept out of scope (no new editable input near those terms). |
| `ui/src/features/acts/ActDetail.svelte` | D-07 button gating | ✓ VERIFIED | `act.act_type !== 'handover'` gates disabled state; no `act.archived` condition on the Edit button (confirmed distinct from Возврат's condition). |
| `ui/src/features/acts/ActsPage.svelte` | onEdit orchestration, second edit-mode modal instance | ✓ VERIFIED | `handleEdit`/`handleEditSaved`, `onEdit={handleEdit}` wired into `ActDetail`, separate `<ActFormModal mode="edit">` instance confirmed present. |

### Key Link Verification

| From | To | Via | Status | Details |
|---|---|---|---|---|
| `acts_sqlite.rs` | `acts.handover_date_utc` | `ORDER BY a.handover_date_utc DESC` | ✓ WIRED | Both `list()` and `search_acts()` confirmed. |
| `ActListRow.svelte`/`ActDetail.svelte` | `ActDto.handover_date_utc` | `formatDate(act.handover_date_utc)` | ✓ WIRED | Confirmed via grep, no `created_at_utc` remnants in these two components. |
| `ActFormBody.svelte` submit | `acts.update` | `acts.update(updatePayload)` in the `mode === 'edit'` branch | ✓ WIRED | Confirmed at line 158. |
| `acts.update` client | `POST /api/v1/acts_update` / Tauri `acts_update` | `apiCall<ActDto>('acts_update', {payload})` | ✓ WIRED | Confirmed in `ui/src/lib/api/acts.ts:31`. |
| `build_acts_update` (both transports) | `ActService::update` | `ctx.acts.update(payload)` | ✓ WIRED | Single shared helper, both Tauri and axum call it — no duplicated authorize logic. |
| `ActService::update` (add/remove loops) | `acts.archived` | `recompute_parent_archived` | ✗ NOT WIRED | Confirmed missing — see Gaps. This is the one broken link in an otherwise fully-wired chain. |
| `ActDetail.svelte` | `ActsPage.svelte::handleEdit` | `onclick={() => onEdit(act)}` | ✓ WIRED | Confirmed. |

### Behavioral Spot-Checks (automated test re-execution)

| Behavior | Command | Result | Status |
|---|---|---|---|
| ACT-01 sort/render regression | `cargo test -p trackly-app --test acts_date_source` | 2/2 passed | ✓ PASS |
| ACT-01 PDF date-source regression | `cargo test -p trackly-app --test html_act_render` | 8/8 passed | ✓ PASS |
| ACT-02 backend update() behaviors | `cargo test -p trackly-app --test acts_update` | 9/9 passed | ✓ PASS |
| bindings.ts regeneration/assertions | `cargo test -p trackly-app --test export_bindings` | (ran as part of full re-verification, see terminal output above) | ✓ PASS |
| RBAC regression incl. Case 42 | `cargo test -p trackly-app --test role_endpoint_matrix` | 1/1 test binary passed (includes Case 42) | ✓ PASS |
| Regression: act-number display formatting unaffected | `cargo test -p trackly-app --test acts_display_rule` | 4/4 passed | ✓ PASS |
| Frontend types/build | `pnpm --dir ui exec svelte-check` | 0 errors, 48 pre-existing warnings unrelated to Phase 19 files | ✓ PASS |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|---|---|---|---|---|
| `crates/trackly-app/src/services/act_service.rs` | 578-953 (whole `update()` body) | Missing `recompute_parent_archived` call present in sibling mutation paths | 🛑 Blocker (confirmed functional defect, CR-01 in code review) | Silent `archived`-flag corruption in 2 identified edit scenarios; devices can be stranded `в_работе` with no UI return path. |
| `crates/trackly-app/src/services/act_service.rs` | 792-809, 868-878 | Renaming a handover with existing returns leaves the return rows' stored `number` stale (WR-01, from code review) | ⚠ Warning | Old act number permanently unreusable; display unaffected (JOIN-based). Not central to phase goal wording but a real data-integrity nit. |
| `ui/src/features/acts/ActFormBody.svelte` / `ActFormItemsTable.svelte` | 150-156 / 680-694 (per code review WR-02) | Edit-mode item table still shows the group/quantity picker but the submit path silently sends only 1 device per row | ⚠ Warning | User can select a group of N devices via the picker, only 1 is actually added, with no error/warning — directly relevant to "изменения сохраняются без ошибок" being silently incomplete rather than erroring. |
| `crates/trackly-app/src/services/act_service.rs` | 756-770 (per code review WR-03) | Retained-item комплектация edits are not written to `audit_log` | ⚠ Warning | Audit-trail completeness gap, not a save-failure. |
| No `TBD`/`FIXME`/`XXX` markers found in any Phase 19 file | — | — | — | Debt-marker gate: clean. |

## Deferred Items

None — no later phase in the current milestone was found to address CR-01/WR-01/WR-02/WR-03; these are open findings against Phase 19 itself.

## Human Verification Required

See frontmatter `human_verification` — 7 items harvested from Plan 19-05 Task 3's deferred `<human-check>` block (workflow.human_verify_mode=end-of-phase), plus 1 additional item this verifier added to directly exercise CR-01's practical effect (archived-act edit → stranded device). None of these have been executed yet; SUMMARY.md 19-05 explicitly defers them to phase-end verification, which is this report.

## Gaps Summary

The phase substantially achieves its stated goal: ACT-01 (date-source fix) is fully verified end-to-end with passing regression tests, and ACT-02's core mechanics (button enabled, form opens prefilled, header/position/комплектация edits save, CAS/RBAC/D-07/D-08 all enforced server-side) are implemented, wired, and covered by 9 passing integration tests plus confirmed frontend wiring.

However, one confirmed, reproducible functional defect prevents full verification of Success Criterion 3 ("внесённые изменения сохраняются без ошибок") for all edit scenarios: `ActService::update` never recomputes the act's derived `archived` flag after changing its device set, unlike every sibling mutation (`do_return`, `delete_soft`) that touches the same handover/return balance. This was independently reconfirmed by direct source inspection (not just trusted from 19-REVIEW.md) — the entire body of `update()` (act_service.rs:578-953) contains no `recompute_parent_archived` call, and no test in `acts_update.rs` exercises the `archived` field. The result is a silent (no error, no toast) data-integrity corruption reachable via the UI in two scenarios: editing an archived act to add a device (stranding it `в_работе` with no return path), and removing the last outstanding device from a non-archived act (leaving it incorrectly non-archived). This directly undermines the phase's and project's "работает надёжно" intent for the editing feature this phase exists to deliver.

Two related but lower-severity warnings (WR-02: edit-mode group/quantity picker silently drops devices; WR-01: rename leaves return rows' stale number) further erode confidence that "изменения сохраняются без ошибок" holds for all edit paths, though they are not classified as blockers here.

**Recommendation:** Add the missing `recompute_parent_archived(&tx, payload.id, now)` call inside `ActService::update`'s transaction (after the item add/remove loops resolve the final item set), plus the two regression tests described in the gaps section, before considering ACT-02 fully closed. WR-01/WR-02/WR-03 should be triaged (fix now or explicitly deferred with a tracked follow-up) but do not block this specific re-verification path.

---

_Verified: 2026-07-12_
_Verifier: Claude (gsd-verifier)_
