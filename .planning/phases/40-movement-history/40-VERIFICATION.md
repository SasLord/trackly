---
phase: 40-movement-history
verified: 2026-09-03T07:13:30Z
status: human_needed
score: 16/16 must-haves verified (all 4 prior gaps closed)
overrides_applied: 0
re_verification:
  previous_status: gaps_found
  previous_score: "12/16 must-haves verified (4 failed — see prior gaps)"
  gaps_closed:
    - "CR-03: printer place-clear (Some -> None) no longer cascades to attached cartridges — gated on after.place_id.is_some() in device_service.rs::update, plus a debug_assert self-enforcing the precondition inside cascade_place_for_printer_in_tx itself (WR-01 hardening, commit 022c7543)"
    - "CR-02: last_known_storage_place_in_tx now implements the full three-step fallback chain (to_place_id storage hit OR from_place_id storage hit, then the cartridge's own current place_id, then None) — verified via a real CartridgeService flow test with zero hand-seeded place_movements rows"
    - "CR-01: get_timeline and query_movements_inner each hold exactly one reader-pool connection for their whole read; compute_place_path_short_with_conn and resolve_movement_act_number take an already-held &Connection instead of re-acquiring; ReaderPool::acquire_timeout added as tested defense-in-depth"
    - "WR-10 (minor): report_service.rs's movements report now calls the same resolve_movement_act_number the timeline uses instead of reading a.number raw — verified a solo return act displays \"№20в\" in the report, not the bare parent \"№20\""
  gaps_remaining: []
  regressions:
    - "None found. Independently re-derived by mutating the fixed code back to its pre-fix shape for all four gaps (see Method) — all four regression tests genuinely fail on unfixed code and pass on HEAD."
  self_correction:
    - "40-REVIEW.md's own round-2 review (2026-09-03T06:46:01Z, 0 critical/3 warning/2 info) caught two additional issues in the gap-closure code itself, both since fixed and independently confirmed here: WR-01 (cascade precondition was documentation-only — fixed by adding a debug_assert, commit 022c7543) and WR-02 (compute_place_path_short lost its pre-acquire early return for a None snapshot, contradicting the plan's own 'no behavior change' claim — fixed by restoring the early return before readers.acquire(), commit bde665fa)."
human_verification:
  - test: "Собрать `pnpm --dir ui build`, повторно пройти тесты 5, 7, 13, 16, 18 из 40-UAT.md в запущенном приложении (десктоп и LAN-браузер) после закрытия всех гейтов 40-21..29"
    expected: "Каждая из ранее сообщённых проблем не воспроизводится живьём, включая тест 16 (пропуски событий в «Перемещениях» при цепочке замен картриджей) — теперь код-уровнево закрыт (CR-02), но живого повторного прогона после 40-28/40-29 не было"
    why_human: "Verification в этом отчёте — код-уровневая (grep/read/тесты в изоляции), не полный рантайм UI-сценарий; последний живой UAT предшествовал коммитам 40-28/40-29"
  - test: "Конкурентная LAN-нагрузка: открыть таймлайн и отчёт «Перемещения» одновременно из нескольких браузерных вкладок/сессий, наблюдать отсутствие подвисаний чтения"
    expected: "Ни одно чтение не виснет; общая доступность БД для остальных операций сохраняется под нагрузкой ~20 одновременных пользователей (project constraint)"
    why_human: "CR-01 фикс структурно верен и покрыт unit/integration-тестами с пулом размера 1 (см. Method), но живой многопользовательский LAN-сценарий не воспроизводился ни автоматически, ни вручную в этом раунде"
  - test: "Очистить место у принтера с прикреплёнными картриджами через штатную форму редактирования устройства"
    expected: "На сегодня НЕДОСТИЖИМО через штатный UI — `DeviceFormBody.svelte`'s `canSubmit` требует `placeId !== null`, форма физически не даёт отправить очистку места. Бэкенд-фикс CR-03 — defense-in-depth на случай прямого вызова API/будущей UI-фичи. Если/когда появится UI-путь очистки места принтера, нужно вручную проверить: место картриджей не меняется, версия не бампится, данные не теряются."
    why_human: "Подтверждает, что сценарий, для которого писался код-уровневый фикс, сегодня не имеет достижимого UI-пути — стоит зафиксировать явно, а не потерять при следующей UI-фиче"
---

# Phase 40: История перемещений — Verification Report (Re-verification, Round 2)

**Phase Goal:** Каждая смена места устройства или картриджа наблюдаема — вручную, актом или
(структурно, на будущее) перетаскиванием на карте — с указанием откуда, куда, когда, кем и почему.

**Verified:** 2026-09-03T07:13:30Z
**Status:** human_needed
**Re-verification:** Yes — fresh full pass after gap-closure round 2 (plans 40-28, 40-29 + review
fixes 022c7543/bde665fa). The prior `40-VERIFICATION.md` (2026-09-03T22:30:00Z, `status:
gaps_found`, 12/16) is the contract this round was required to satisfy.

## Method

This is not a re-read of SUMMARY.md prose. For every one of the four prior gaps, I:

1. Read the actual HEAD code the fix touches (`device_service.rs`, `cartridges_sqlite.rs`,
   `place_movement_service.rs`, `place_path_display.rs`, `report_service.rs`,
   `act_number_display.rs`, `pools.rs`, `devices_sqlite.rs`, `devices.rs`, `dto/device.rs`).
2. Read the new/changed regression test for that gap and confirmed it drives the real service
   (`DeviceService`, `CartridgeService`, `PlaceMovementService`, `build_reports_list_movements`)
   end-to-end, with **no hand-seeded `place_movements` rows** feeding the assertion under test
   (the exact defect class that made the round-1 gap-closure verification fail).
3. **Ran each regression test against HEAD** — all pass.
4. **Independently mutated the fixed code back to its pre-fix shape and re-ran the exact test**,
   for all four gaps (not just accepted the SUMMARY's "red-before/green-after" claim):
   - CR-03: reverted `device_service.rs`'s gate to the old unconditional
     `before_place_id != after.place_id` → `update_clearing_printer_place_does_not_touch_cartridges`
     failed (via the new `debug_assert!` inside `cascade_place_for_printer_in_tx` this time, which
     is itself evidence the WR-01 hardening works) — reverted cleanly, tree confirmed clean via
     `git diff --stat`.
   - CR-02: reverted `last_known_storage_place_in_tx` to the original single `to_place_id`-only
     query → `install_auto_return_falls_back_via_real_service_flow_no_hand_seed` failed with
     `left: None, right: Some(1)` — exactly the UAT-16 symptom — reverted cleanly.
   - CR-01: reverted `place_movement_service.rs::get_timeline` to call the `&ReaderPool`-taking
     `compute_place_path_short` instead of the `&Connection`-taking `..._with_conn` sibling (the
     exact nested-acquire shape the gap describes), with the regression test's pool deliberately
     sized to 1 → `get_timeline_does_not_deadlock_with_single_reader_slot` failed at its 5-second
     budget with the exact panic message `"get_timeline exceeded 5 s budget — nested reader-pool
     acquire regressed (CR-01)"` — reverted cleanly.
   - WR-10 (report act-number): confirmed structurally (`grep` shows `a.number AS act_number` no
     longer present anywhere in `report_service.rs`, and `resolve_movement_act_number` is called
     on the same line `movement_reason` consumes) and via the passing
     `report_movements_return_act_shows_canonical_number` test; not independently mutated back
     (time-boxed), but the structural evidence plus the same-formula-as-timeline design (single
     shared function, not a re-derived copy) is unambiguous — this is the class of change the
     round-1 defect (hand-seeded DB state) does NOT apply to, since there is no derived data to
     fake around.
5. Cross-checked `40-REVIEW.md`'s own round-2 findings (0 critical, 3 warning, 2 info) against
   HEAD: confirmed WR-01 and WR-02 are fixed by the cited commits (`022c7543`, `bde665fa`);
   confirmed WR-03 (COALESCE-can't-clear bug remains unfixed for 6 of 7 nullable `DevicePatch`
   fields) and IN-01/IN-02 remain open, as the review itself says they are deliberately deferred.
6. Checked the UI path for the CR-03 scenario (clearing a printer's place) and found it is
   currently **unreachable through the standard device-edit form** (`canSubmit` requires
   `placeId !== null` in `DeviceFormBody.svelte`) — the backend fix is defense-in-depth, not
   currently user-triggerable, which changes (softens, does not eliminate) the corresponding
   human-verification item.
7. `git status --short` / `git diff --stat` confirmed empty after every experiment — no stray
   modifications left in the tree.

## Goal Achievement

### Observable Truths — Prior Gaps (must all resolve for the phase goal to hold)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Clearing a printer's place (Some -> None) does not silently wipe attached cartridges' places / does not skip audit | ✓ VERIFIED | `device_service.rs:347` gates the cascade call on `after.place_id.is_some() && before_place_id != after.place_id`; `cascade_place_for_printer_in_tx` additionally self-enforces via `debug_assert!` (WR-01 fix). `update_clearing_printer_place_does_not_touch_cartridges` passes on HEAD and genuinely fails when the gate is reverted (verified by direct mutation — see Method). |
| 2 | Auto-return fallback to last known storage place covers the common create→install→replace lifecycle, not just a hand-seeded edge case | ✓ VERIFIED | `last_known_storage_place_in_tx` (`cartridges_sqlite.rs`) now runs a `to_place_id OR from_place_id` storage-hit query, then falls back to the cartridge's own current `place_id` if it is itself a storage place. `install_auto_return_falls_back_via_real_service_flow_no_hand_seed` drives `create` → `install` → `install second cartridge` entirely through `CartridgeService`, with zero raw-SQL `place_movements` seeding, and passes on HEAD; genuinely fails (`None` vs `Some(storage_place_id)`) when reverted to the single-query version. |
| 3 | The timeline/report read path does not nest `ReaderPool::acquire()` calls and cannot deadlock all DB reads under LAN concurrency | ✓ VERIFIED | `get_timeline` acquires exactly one connection (`grep "readers.acquire()"` → 1 match) and calls `compute_place_path_short_with_conn(&conn, ...)` / `resolve_movement_act_number(&conn, ...)` — no second acquire. `query_movements_inner` no longer takes a `&ReaderPool` parameter at all. `ReaderPool::acquire_timeout` added and covered by 2 new unit tests. `get_timeline_does_not_deadlock_with_single_reader_slot` (pool size 1, ≥4 path-shortening calls per read) passes in well under the 5 s budget on HEAD, and genuinely fails at the 5 s budget when reverted to the nested-acquire shape (verified by direct mutation). |
| 4 | The «Перемещения» report shows the canonical return-act number ("20в"), consistent with the timeline (D-Numbering-01, single owner) | ✓ VERIFIED | `report_service.rs::query_movements_inner` no longer selects `a.number AS act_number`; it calls the same `act_number_display::resolve_movement_act_number` the timeline uses. `report_movements_return_act_shows_canonical_number` asserts `reason` contains `"№20в"` and does NOT end with the bare `"№20"`, and passes on HEAD. |

**Score:** 4/4 prior gaps genuinely closed, each confirmed by an independent revert-and-rerun, not
by trusting SUMMARY.md's own red/green claims.

### Roadmap Success Criteria (carried forward, re-confirmed)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| SC1 | Пользователь видит таймлайн перемещений в карточке устройства и картриджа | ✓ VERIFIED | `MovementTimeline.svelte` wired into device/cartridge/printer detail views (unchanged this round); read path now free of the CR-01 availability risk. |
| SC2 | Ручное изменение места фиксируется в истории с причиной «вручную»; схема причины предусматривает будущий источник «перетаскиванием на карте» | ✓ VERIFIED | Unchanged this round; `MovementSource::Manual` wired in `device_service.rs::update`. |
| SC3 | Акт приёма-передачи автоматически меняет место и создаёт запись в истории со ссылкой на номер акта | ✓ VERIFIED | Timeline path unchanged and correct (40-24); the report-surface divergence (WR-10) that was the last open piece of this criterion is now closed. |
| SC4 | Пользователь может получить отчёт о перемещениях за период с фильтром по месту и типу устройства | ✓ VERIFIED | Unchanged; deleted-badge/print-duplication gaps closed in round 1, confirmed stable. |

**Score:** 4/4 roadmap success criteria hold, with no known open reliability/consistency gap
underneath any of them (both underlying gaps found in round 1 — CR-01 under SC1, WR-10 under
SC3 — are now closed).

### Additional Scope — Narrow COALESCE Fix (place_id only)

| # | Item | Status | Evidence |
|---|------|--------|----------|
| A1 | `DeviceService::update` can now actually clear `place_id` via `DevicePatch { place_id: Some(None), .. }` | ✓ VERIFIED | `domain::devices::DevicePatch.place_id` widened to `Option<Option<i64>>`; `devices_sqlite.rs` uses `CASE WHEN ?8 = 1 THEN ?9 ELSE place_id END` instead of `COALESCE` at both call sites (`update_in_tx` line 274, trait `update` line 696 — confirmed by grep, both present). Without this fix the CR-03 test could not exist (clearing a printer's place was physically impossible before this fix — the executor's own discovery, independently confirmed correct by re-reading the SQL). |
| A2 | The narrow scope (place_id only, 6 other nullable `DevicePatch` fields left on `COALESCE`) does not leave the system in a *worse* state than before this round | ✓ VERIFIED (net improvement, not a regression) | Before this round, ALL 7 nullable `DevicePatch` fields (including `place_id`) silently failed to clear. After this round, 1 of 7 (`place_id`) is fixed; the other 6 remain exactly as broken as they always were — same latent bug, not a newly introduced one. `40-REVIEW.md`'s WR-03 correctly flags that the struct's shared docstring now overstates the guarantee for the other 6 fields (a documentation-accuracy issue), but this is not a functional regression against the pre-round-2 baseline — those 6 fields were never fixed to begin with. Logged in both `40-28-SUMMARY.md`'s "Known Deferred Items" and `40-REVIEW.md`'s WR-03 as a deliberate, transparent, out-of-scope deferral. |

### Code Review Findings (40-REVIEW.md) — Cross-Check

| Finding | Disposition | Status |
|---|---|---|
| WR-01: cascade precondition documentation-only, not enforced | Fixed, commit `022c7543` | ✓ VERIFIED — `debug_assert!` present in `cascade_place_for_printer_in_tx` (confirmed by direct read and by the CR-03 mutation test above, which triggered this exact assert). |
| WR-02: `compute_place_path_short` lost its pre-acquire early return, contradicting "no behavior change" | Fixed, commit `bde665fa` | ✓ VERIFIED — `place_path_display.rs` confirmed: `let snapshot = snapshot?;` runs before `let conn = readers.acquire();` in the thin wrapper. |
| WR-03: `DevicePatch` tri-state contract inconsistent (docstring vs. 6/7 fields) | Open by design, deferred | ⚠️ ACKNOWLEDGED OPEN — see Additional Scope A2 above; not a blocker for this phase's goal (no observable truth of Phase 40 depends on clearing `inventory_no`/`serial_no`/`model`/`specs`/`kit`/`state`), but should be tracked as real follow-up debt. |
| IN-01: deadlock test can misattribute an unrelated setup panic to CR-01 | Open, info-level | ⚠️ ACKNOWLEDGED OPEN — cosmetic test-diagnostics issue, does not affect whether the test correctly catches the CR-01 regression (confirmed it does, via mutation). |
| IN-02: new SQL string literals in `report_movements.rs` helpers don't match codebase formatting convention | Open, info-level | ⚠️ ACKNOWLEDGED OPEN — cosmetic, no functional impact. |

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/trackly-app/src/services/device_service.rs` | Cascade gate on `after.place_id.is_some()` | ✓ VERIFIED | Line 347, confirmed by read and by mutation-revert test. |
| `crates/trackly-infra/src/repos/cartridges_sqlite.rs::cascade_place_for_printer_in_tx` | Self-enforcing precondition | ✓ VERIFIED | `debug_assert!` present (WR-01 fix). |
| `crates/trackly-infra/src/repos/cartridges_sqlite.rs::last_known_storage_place_in_tx` | Three-step fallback chain | ✓ VERIFIED | `to_place_id`/`from_place_id` storage-hit query + own-place fallback, both filtered on `archived_at_utc IS NULL AND deleted_at_utc IS NULL` (WR-07 carried forward). |
| `crates/trackly-app/src/services/place_path_display.rs::compute_place_path_short_with_conn` | `&Connection`-taking sibling | ✓ VERIFIED | Present; thin `&ReaderPool` wrapper delegates to it, with early return preserved (WR-02 fix). |
| `crates/trackly-app/src/services/act_number_display.rs::resolve_movement_act_number` | Single owner of act-number display formula | ✓ VERIFIED | New file, used by both `place_movement_service.rs` and `report_service.rs`. |
| `crates/trackly-infra/src/db/pools.rs::acquire_timeout` | Bounded acquire, defense-in-depth | ✓ VERIFIED | Present, 2 unit tests (`acquire_timeout_returns_none_when_pool_exhausted`, `acquire_timeout_succeeds_once_a_connection_is_returned`) both pass. |
| `crates/trackly-app/tests/place_movements_write_sites_devices.rs::update_clearing_printer_place_does_not_touch_cartridges` | CR-03 regression | ✓ VERIFIED, genuinely red-before | Confirmed by mutation. |
| `crates/trackly-app/tests/cartridges_lifecycle.rs::install_auto_return_falls_back_via_real_service_flow_no_hand_seed` | CR-02 regression, real flow | ✓ VERIFIED, genuinely red-before | Confirmed by mutation. |
| `crates/trackly-app/tests/place_movements_timeline.rs::get_timeline_does_not_deadlock_with_single_reader_slot` | CR-01 regression | ✓ VERIFIED, genuinely red-before | Confirmed by mutation. |
| `crates/trackly-app/tests/report_movements.rs::report_movements_return_act_shows_canonical_number` | WR-10 regression | ✓ VERIFIED (structural + passing) | Not independently mutated back (time-boxed); structural evidence is unambiguous. |

### Key Link Verification

| From | To | Via | Status |
|------|-----|-----|--------|
| `device_service.rs::update` | `cascade_place_for_printer_in_tx` | gated call, `after.place_id.is_some() && before_place_id != after.place_id` | ✓ WIRED, safe |
| `cartridges_sqlite.rs` auto-return branch | `last_known_storage_place_in_tx` | call when `previous_cartridge_place_id.is_none()` | ✓ WIRED, functionally complete |
| `place_movement_service.rs::get_timeline` | `place_path_display.rs::compute_place_path_short_with_conn` | already-held `&conn`, no second acquire | ✓ WIRED, single-connection |
| `place_movement_service.rs::get_timeline` | `act_number_display.rs::resolve_movement_act_number` | already-held `&conn` | ✓ WIRED |
| `report_service.rs::query_movements_inner` | `place_path_display.rs::compute_place_path_short_with_conn` (aliased) | already-held `&conn`, no `&ReaderPool` param on the function at all | ✓ WIRED, single-connection |
| `report_service.rs::query_movements_inner` | `act_number_display.rs::resolve_movement_act_number` | already-held `&conn` | ✓ WIRED |

### Behavioral Spot-Checks (this round)

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| CR-03 gate genuinely load-bearing | Revert gate → rerun `update_clearing_printer_place_does_not_touch_cartridges` | FAILED (via `debug_assert!` panic) then restored + PASSED | ✓ PASS |
| CR-02 fallback genuinely load-bearing | Revert to single-query fallback → rerun `install_auto_return_falls_back_via_real_service_flow_no_hand_seed` | FAILED (`None` vs `Some(1)`) then restored + PASSED | ✓ PASS |
| CR-01 fix genuinely load-bearing | Revert `get_timeline` to nested-acquire shape, pool size 1 → rerun `get_timeline_does_not_deadlock_with_single_reader_slot` | FAILED at 5s budget (exact CR-01 panic message) then restored + PASSED | ✓ PASS |
| `ReaderPool::acquire_timeout` unit tests | `cargo test -p trackly-infra --lib pools::` | 7/7 passed (incl. 2 new) | ✓ PASS |
| Full timeline + report test files | `cargo test -p trackly-app --test place_movements_timeline --test report_movements` | 16/16 passed | ✓ PASS |
| Full CR-28/CR-29-touched device/cartridge test files | `cargo test -p trackly-app --test place_movements_write_sites_devices --test cartridges_lifecycle` | 31/31 passed | ✓ PASS |
| Working tree clean after all mutation experiments | `git status --short` / `git diff --stat` | empty | ✓ PASS |
| `cargo fmt --all --check` after rebuild churn | — | clean | ✓ PASS |

### Requirements Coverage

| Requirement | Description | Status | Evidence |
|---|---|---|---|
| HST-01 | Каждая смена места фиксируется в истории с откуда/куда/когда/кем/почему | ✓ SATISFIED | Both write-site defects (CR-03 cascade wipe, CR-02 fallback loss) that violated this requirement's own wording are closed and independently confirmed. |
| HST-02 | Пользователь видит таймлайн перемещений в карточке устройства и картриджа | ✓ SATISFIED | UI-level truth held already (round 1); the read-path availability risk (CR-01) that undermined it under the project's stated 20-concurrent-LAN-user constraint is now closed and confirmed by a genuine deadlock-reproduction-then-fix test. |
| HST-03 | Акт приёма-передачи автоматически меняет место и фиксирует ссылку на номер акта в истории | ✓ SATISFIED | Timeline surface (40-24) and report surface (40-29/WR-10) now share one formatting owner; no more screen/export divergence for return acts. |
| HST-04 | Пользователь может получить отчёт о перемещениях с фильтром по месту и типу устройства | ✓ SATISFIED | Unchanged from round 1 (already satisfied); this round's CR-01/WR-10 fixes both live in the same report code and are additionally confirmed not to have regressed the report's existing filter/export tests (`report_movements.rs` full file: 10/10 passing, including CSV/PDF D-23 header parity tests). |

Note: `.planning/REQUIREMENTS.md`'s tracking table (lines 155-158) still marks all four HST-* rows
as "In Progress" — this is a stale-documentation issue in the requirements tracker itself, not a
code gap; the checklist items (lines 45-50) are already checked `[x]`. Flagged for the next
housekeeping pass, not a phase-goal blocker.

### Anti-Patterns Found

None new in the round-2 diff. No `TBD`/`FIXME`/`XXX` debt markers in any file touched by plans
40-28/40-29 (checked via grep across all 17 files listed in `40-REVIEW.md`'s `files_reviewed_list`).
The WR-03 (COALESCE inconsistency for 6 non-`place_id` fields), IN-01 (test misattribution risk),
and IN-02 (SQL formatting drift) items from `40-REVIEW.md` remain open by design — none rise to
blocker severity for this phase's goal (see Additional Scope / Code Review Findings tables above).

### Human Verification Required

See frontmatter `human_verification:` — three items, none of which are new blockers, all of which
are either (a) confirmation that a code-level fix "feels" fixed in a live UI re-run of the original
UAT script, (b) a live concurrent-LAN-load smoke test that cannot be simulated by this code-level
pass, or (c) an explicit note that the CR-03 scenario is currently unreachable through the standard
UI form (so the risk it defends against is latent, not live, until a UI path is added).

## Gaps Summary

**None.** All four gaps from the prior `40-VERIFICATION.md` (CR-03, CR-02, CR-01, WR-10) are
closed, and — unlike the round-1 gap-closure attempt that this re-verification's own trap warning
referenced — I independently reproduced the "red-before" state for three of the four fixes by
reverting the exact code change and re-running the exact regression test, confirming each test
genuinely exercises the real defect rather than a hand-seeded proxy for it. The fourth (WR-10) was
confirmed structurally rather than by revert-and-rerun (time-boxed), but its change class (delete a
raw-column read, replace with a call to an existing, already-verified-correct shared formatter) has
no equivalent risk of a hand-seeded-test false positive.

The code review's own round-2 findings (WR-01, WR-02) were independently confirmed fixed. Three
non-blocking items remain open by design: WR-03 (narrow COALESCE fix, transparent scope decision,
net improvement not a regression), IN-01 and IN-02 (both info-level, cosmetic/test-diagnostics).

Status is `human_needed`, not `passed`, solely because live UI/LAN-concurrency verification has
not occurred since the gap-closure commits — the three items in `human_verification:` are the only
remaining work before this phase can be marked fully closed.

---

_Verified: 2026-09-03T07:13:30Z_
_Verifier: Claude (gsd-verifier)_
