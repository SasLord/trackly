---
phase: 40-movement-history
verified: 2026-09-03T22:30:00Z
status: gaps_found
score: 12/16 must-haves verified (4 failed — see gaps)
overrides_applied: 0
re_verification:
  previous_status: human_needed
  previous_score: "4/4 success criteria mechanically verified; 5 manual UAT items outstanding"
  note: |
    The prior 40-VERIFICATION.md (2026-09-02) predates the 2026-09-03 live UAT and the entire
    gap-closure round (plans 40-21..40-27). It is superseded, not extended — this is a fresh
    full pass, not an optimized re-check of a prior gaps: list (the prior report had no
    gaps: section; it was human_needed, not gaps_found).
  gaps_closed:
    - "cargo fmt drift in phase-40 test files (CR-04, fixed in commit 9d128257)"
    - "Deleted-badge (D-25) invisible in the live Перемещения report table (UAT test 13 — fixed in 40-25, confirmed by direct code read)"
    - "Grouped device list place inversion (UAT test 13 second issue — fixed in 40-26, confirmed by direct code read)"
    - "Timeline act-link opens wrong Акты subsection / wrong return-act number (UAT test 7 — fixed in 40-24, confirmed by direct code read)"
    - "LAN print duplicate first page (UAT test 18 — fixed in 40-27, confirmed by direct code read)"
    - "Install-into-printer blocked by mandatory place field (UAT test 5 partial — fixed in 40-23, confirmed by direct code read)"
    - "Empty/short timeline gives no explanation for D-06 first-placement gap (wontfix_by_decision item — UI text added in 40-24, confirmed present)"
  gaps_remaining:
    - "Cartridge does not follow printer when printer's place is CLEARED (Some->None) — cascade silently wipes cartridge places with no movement row and no audit_log row (CR-01/CR-03, new code from 40-21 itself)"
    - "Return-to-stock auto-fallback to last known storage place does not cover the primary/common scenario (first install after creation) — the exact UAT-16 defect the gap plan claims to close (CR-02)"
    - "Reader-pool nested-acquire deadlock risk in the timeline read path with no acquire timeout (CR-01, inherited from 40-10 but still load-bearing for HST-02)"
  regressions:
    - "None found relative to the pre-gap-closure baseline — all 3 remaining issues are either pre-existing (CR-01, from 40-10) or introduced by new gap-closure code with no prior working behavior to regress from (CR-02, CR-03)."
gaps:
  - truth: "Cascade: when a printer's place is cleared (set to empty), attached cartridges' place changes are still recorded in movement history"
    status: failed
    reason: |
      `DeviceService::update` fires `cascade_place_for_printer_in_tx` on any
      `before_place_id != after.place_id`, including `Some(P) -> None`. The cascade
      unconditionally sets every attached cartridge's `place_id = NULL` (UPDATE with no
      guard on `new_place_id.is_some()`), then calls `record_movement_if_applicable`,
      which the D-06 guard (`is_reportable_place_change`) makes a silent no-op for any
      `Some -> None` transition. Result: clearing one field on a printer erases the
      recorded location of every cartridge attached to it, with no `place_movements` row
      and no `audit_log` row — the data is unrecoverable from the app. No test exercises
      `Some -> None` (`place_movements_write_sites_devices.rs` only covers `A -> B`).
      This is new code from this phase's own gap-closure round (40-21) — not a
      pre-existing bug being carried forward.
    severity: major
    artifacts:
      - path: "crates/trackly-app/src/services/device_service.rs"
        issue: "lines ~338-348: cascade call is not gated on after.place_id.is_some()"
      - path: "crates/trackly-infra/src/repos/cartridges_sqlite.rs"
        issue: "cascade_place_for_printer_in_tx (~956-1010): unconditional UPDATE place_id=NULL for every attached cartridge when new_place_id is None"
      - path: "crates/trackly-app/tests/place_movements_write_sites_devices.rs"
        issue: "update_cascades_place_to_attached_cartridges only tests A->B; no Some->None test exists"
    missing:
      - "Skip the cascade (or cascade only the movement-eligible subset) when after.place_id is None — a printer with an unknown place says nothing about where its cartridges are"
      - "Regression test asserting a cartridge keeps its place and version==1 when the printer's place is cleared"
  - truth: "Когда авто-возврат предыдущего картриджа при установке нового не получает явного previous_cartridge_place_id, картриджу подставляется его последнее известное складское место (из place_movements), а не NULL"
    status: failed
    reason: |
      `last_known_storage_place_in_tx` derives the fallback exclusively from
      `place_movements.to_place_id` WHERE `places.is_storage = 1`. Per this phase's own
      D-06 rule, a cartridge's FIRST place assignment (created at the warehouse, or any
      `NULL -> place` transition) never produces a movement row. The common real
      lifecycle — create at storage (no row) -> install into printer (one row, S -> Q,
      Q not storage) -> second cartridge installed into same printer, triggering this
      cartridge's auto-return with no explicit place -> fallback query finds no
      to_place_id row that is a storage place -> returns None -> place_id set to NULL —
      is exactly the UAT-reported defect (test 16, "return-to-stock-empty-place-field")
      and it STILL reproduces on the first/most common install-then-replace cycle. The
      new regression test (`install_auto_return_falls_back_to_last_known_storage_place`)
      does not catch this because it hand-seeds a place_movements row via raw SQL into
      the storage place before either Install call — DB state the real code path never
      produces on its own — so the test is green while the user-visible defect survives.
    severity: major
    artifacts:
      - path: "crates/trackly-infra/src/repos/cartridges_sqlite.rs"
        issue: "last_known_storage_place_in_tx (~928-943) only checks to_place_id WHERE is_storage=1; ignores from_place_id and the cartridge's own current place_id; also ignores archived_at_utc/deleted_at_utc (WR-07)"
      - path: "crates/trackly-app/tests/cartridges_lifecycle.rs"
        issue: "install_auto_return_falls_back_to_last_known_storage_place hand-seeds place_movements via raw SQL — does not drive the flow that actually reaches this code path in production"
    missing:
      - "Fallback chain: (1) explicit override, (2) from_place_id of the movement that took the cartridge OUT of a storage place, (3) the cartridge's own place_id if it is already a storage place, (4) only then NULL"
      - "Filter candidate storage places on archived_at_utc IS NULL AND deleted_at_utc IS NULL (WR-07)"
      - "A test that drives the whole flow through CartridgeService (create at storage -> install -> install second cartridge into same printer) with NO hand-seeded place_movements row, asserting the auto-returned cartridge lands back at the storage place"
  - truth: "The timeline read path (HST-02) is safe under the project's stated concurrent-access model (SQLite WAL + reader pool, ~20 LAN users) and does not risk starving or deadlocking all DB reads"
    status: failed
    reason: |
      `PlaceMovementService::get_timeline` acquires one reader connection and holds it
      for the entire row loop (`let conn = readers.acquire();`). Inside the loop it calls
      `compute_place_path_short(&readers, ...)` twice per row, and that function opens a
      SECOND connection from the SAME pool. `ReaderPool::acquire()` blocks on a
      `std::sync::Condvar` with NO timeout when the pool is exhausted (confirmed by
      direct read of `pools.rs::acquire` — a plain `loop { ... available.wait(conns) }`).
      Production reader-pool size is 8. If enough concurrent timeline reads each hold
      their outer connection and then each try to take a second one, every one of them
      parks forever — every DB read in the app hangs, not just the timeline. Short of a
      full deadlock, a 20-row timeline still performs 41 pool acquisitions instead of 1
      and holds 2 of 8 connections for the whole read. The same shape exists in the
      movements report (report_service.rs, up to 2000 nested acquisitions per render).
      This machinery is Plan 40-10's (not a gap-closure plan), but it is the sole backing
      implementation for HST-02's "user sees the timeline" promise and is unresolved as
      of this verification pass.
    severity: major
    artifacts:
      - path: "crates/trackly-app/src/services/place_movement_service.rs"
        issue: "get_timeline holds one reader conn across the row loop (line ~64) while compute_place_path_short (called twice per row, lines ~145/150) acquires a second one from the same pool"
      - path: "crates/trackly-app/src/services/place_path_display.rs"
        issue: "compute_place_path_short(&ReaderPool, ...) always calls readers.acquire() itself — no variant accepts an already-held &Connection"
      - path: "crates/trackly-infra/src/db/pools.rs"
        issue: "ReaderPool::acquire() (~74-92): blocking Condvar wait with no timeout — an exhausted pool parks the calling thread indefinitely"
      - path: "crates/trackly-app/src/services/report_service.rs"
        issue: "movements report (~1495-1559) has the same nested-acquire shape, up to LIMIT 1000 rows"
    missing:
      - "A &Connection-taking sibling of compute_place_path_short in place_path_display.rs so callers that already hold a reader never acquire a second one"
      - "get_timeline (and the movements report) read variant/separator settings ONCE from the already-held connection, loop with the pure shorten_place_path formula only"
      - "Consider a bounded acquire() (or acquire_timeout()) as a defense-in-depth measure independent of the nested-acquire fix"
  - truth: "Отчёт «Перемещения» показывает канонический номер возврата (например «20в»), согласованно с таймлайном (D-Numbering-01, single owner)"
    status: failed
    reason: |
      Plan 40-24 correctly routed the TIMELINE through format_act_number (confirmed by
      direct code read of place_movement_service.rs). Its sibling surface — the
      «Перемещения» report, also built in this phase (40-11/40-12) and re-touched in
      40-25 for the deleted badge — still selects the raw a.number column and never
      calls format_act_number. A return act now displays as "20в" in the device-card
      timeline and as the bare "20" in the report table/CSV/PDF — indistinguishable from
      the parent handover act on that surface. This is the exact screen-vs-export
      divergence class this phase itself added a structural gate against (WR-03/D-25),
      just for a different column.
    severity: minor
    artifacts:
      - path: "crates/trackly-app/src/services/report_service.rs"
        issue: "lines ~1479,1505: `a.number AS act_number` read raw, never passed through format_act_number (dto/act.rs)"
    missing:
      - "Reuse format_act_number (or extract one shared \"resolve display act number for an act_id\" helper) so report_service.rs and place_movement_service.rs have a single owner"
human_verification:
  - test: "Собрать `pnpm --dir ui build`, повторно пройти тесты 5, 7, 13, 16, 18 из 40-UAT.md в запущенном приложении (десктоп и LAN-браузер) после закрытия гейтов 40-21..27"
    expected: "Каждая из 5 UAT-проблем действительно не воспроизводится живьём (код-уровень фиксов подтверждён этим отчётом, но живого повторного прогона после гап-клозура не было)"
    why_human: "Verification в этом отчёте — код-уровневая (grep/read), не рантайм; последний живой UAT предшествовал гап-клозур коммитам"
  - test: "OperationModal (40-23): открыть установку картриджа в принтер, у которого места нет, оставить поле «Место» пустым, отправить"
    expected: "Форма не блокирует отправку; сервер резолвит/не резолвит место согласно D-13 без ошибки формы"
    why_human: "Client-side validate() гейт подтверждён чтением кода, но реальный рендер подсказки/поведение поля не наблюдались в запущенном приложении"
  - test: "Очистить место у принтера с прикреплёнными картриджами (после того как gap CR-03 будет закрыт) и убедиться, что поведение соответствует продуктовому решению — оставить место картриджей нетронутым"
    expected: "Место картриджей не меняется, версия не бампится, никакой потери данных"
    why_human: "Требует UI-действия «очистить место» на форме принтера — сценарий не покрыт ни одним текущим Rust/JS тестом"
---

# Phase 40: История перемещений — Verification Report

**Phase Goal:** Каждая смена места устройства или картриджа наблюдаема — вручную, актом или
(структурно, на будущее) перетаскиванием на карте — с указанием откуда, куда, когда, кем и почему.

**Verified:** 2026-09-03T22:30:00Z
**Status:** gaps_found
**Re-verification:** Yes — full pass after the 2026-09-03 UAT + 7-gap closure round (plans
40-21..40-27); the prior VERIFICATION.md (2026-09-02) predates both and is superseded.

## Method

Every claim below is backed by direct `Read`/`grep` against the current `HEAD` (commit
`9d128257`), not by trusting `*-SUMMARY.md` prose or `40-REVIEW.md`'s claims. `40-REVIEW.md`'s
three open Critical Issues (CR-01, CR-02, CR-03) were independently re-derived by reading the
exact cited code (not accepted at face value) — all three are confirmed real and unfixed. CR-04
(rustfmt drift) is confirmed fixed (`cargo fmt --all --check` is clean at HEAD; fix commit
`9d128257`). One additional gap not flagged as a remaining CR by the orchestrator (report-service
act-number formatting, WR-10 in the review) was independently confirmed and is included below at
minor severity because it is a genuine, currently-live divergence on a surface this phase built.

The already-established facts (full workspace test suite 1166/1166 passing with the documented
pre-existing skip, clean clippy, clean fmt, clean UI build/lint) are accepted as given per the
task brief and were not independently re-run — but per this report's own findings, a clean test
suite does not prove the goal is achieved: the two most serious defects (CR-02, CR-03) are
UNCOVERED by any passing test, and one test (`install_auto_return_falls_back_to_last_known_storage_place`)
is actively green while the scenario it claims to close still reproduces.

## Goal Achievement

### Observable Truths — Roadmap Success Criteria

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| SC1 | Пользователь видит таймлайн перемещений в карточке устройства и картриджа (откуда/куда/когда/кем/почему) | ✓ VERIFIED | `MovementTimeline.svelte` wired into `PlaceEntityViewModal.svelte`, `CartridgeDetail.svelte`, `PrinterDetail.svelte`; UAT 2026-09-03 tests 2, 4, 6 all `pass` live. Reliability caveat: see gap "reader-pool deadlock risk" below — the read path backing this truth has an unresolved availability risk under concurrent load. |
| SC2 | Ручное изменение места фиксируется в истории с причиной «вручную»; схема причины предусматривает будущий источник «перетаскиванием на карте» | ✓ VERIFIED | UAT test 3 `pass` live; `MovementSource::Manual` confirmed wired in `device_service.rs::update`; `MovementSource` domain enum has room for a future map-drag source per `40-01-PLAN.md`. |
| SC3 | Акт приёма-передачи автоматически меняет место переданных устройств и создаёт запись в истории со ссылкой на номер акта | ⚠️ MOSTLY VERIFIED | Core write path confirmed (UAT tests 7 `pass`-after-fix, 8 `pass`); deep-link subsection + canonical return-number fixed in 40-24 (confirmed by code read). Gap: the sibling movements-REPORT surface (also this phase's own deliverable) still shows the bare parent number for return acts (see gaps: WR-10) — minor severity, does not affect the timeline itself. |
| SC4 | Пользователь может получить отчёт о перемещениях за период с фильтром по месту и типу устройства | ✓ VERIFIED | UAT tests 11, 12, 14 `pass` live; deleted-badge live-table gap (test 13) closed by 40-25, confirmed by code read (`ReportsPage.svelte:607` passes `reportType={reportTypeKey()}` to the one `<ReportTable>` instance; `ReportTable.svelte:175` gates on `reportType === 'movements'`). |

**Score:** 4/4 roadmap success criteria hold at the surface level, but SC1 and SC3 each carry an
unresolved reliability/consistency gap discovered underneath them (see Gaps).

### Observable Truths — Gap-Closure Plans (40-21..40-27)

| # | Truth (from PLAN must_haves) | Status | Evidence |
|---|-------|--------|----------|
| 1 | 40-21: printer place change cascades to attached cartridges' places, logged with note «вместе с принтером» | ⚠️ PARTIAL | `Some -> Some` transition VERIFIED (`update_cascades_place_to_attached_cartridges` test passes, confirmed by reading the assertions). `Some -> None` (clearing a printer's place) FAILS — see gaps: silently wipes cartridge places, no movement row, no audit row, untested. |
| 2 | 40-21: explicit cartridge place on install backfills an unset printer's place | ✓ VERIFIED (write happens) | Confirmed at `cartridges_sqlite.rs:614-644`: `UPDATE devices SET place_id=... WHERE place_id IS NULL` executes. Caveat (WR-06, not independently escalated to a blocking gap here): the paired `record_movement_if_applicable(None, Some(explicit))` call is dead by construction (D-06 guard makes `None -> Some` non-reportable), so the write is real but never appears in the printer's own timeline or `audit_log` — an audit-trail gap, not a data-correctness gap. |
| 3 | 40-22: auto-return with no explicit place falls back to the cartridge's last known storage place instead of NULL | ✗ FAILED | See gaps — confirmed by direct code read that the fallback query only checks `to_place_id` rows, which the phase's own D-06 rule prevents from ever being written for a cartridge's first placement; the regression test hand-seeds the missing data via raw SQL rather than driving the real flow. |
| 4 | 40-23: install into a printer without a place does not block on the form's required-place validation | ✓ VERIFIED | `OperationModal.svelte::validate()` (~573): `placeId` required only when `effectivePrinterId === undefined`; confirmed no other gate blocks submit when a printer is selected. |
| 5 | 40-24: deep-link from timeline opens the correct Акты subsection (Акты/Возвраты/Архив) and highlights the row | ✓ VERIFIED | `ActsPage.svelte` derives `activeTab` from the target act's `act_type`/`archived` on first resolution of `initialFocusId` (confirmed at lines ~127-152), guarded against re-firing on subsequent normal row clicks. |
| 6 | 40-24: timeline shows the canonical return-act number (e.g. «20в»), not the bare parent number | ✓ VERIFIED | `place_movement_service.rs` resolves `act_type`/`sub_number`/`parent_number`/`sibling_return_count` and calls `format_act_number` (confirmed at ~lines 104-140), replacing the prior raw `SELECT number`. |
| 7 | 40-24: empty/short timeline explains that first placement is not recorded (D-06) | ✓ VERIFIED | Explanatory paragraph present in both the empty-state and non-empty-footer branches of `MovementTimeline.svelte` (confirmed at lines ~87-90 and ~134-137; cosmetic duplication/CSS-class mismatch noted as info-level only, IN-05). |
| 8 | 40-25: «Удалено» badge visible in the LIVE Перемещения report table, not only in export | ✓ VERIFIED | `ReportsPage.svelte:607` passes `reportType={reportTypeKey()}` to the sole `<ReportTable>` instance (confirmed only one occurrence exists — the review's WR-03 concern about the structural gate's robustness against a second occurrence does not currently manifest as a live bug); `ReportTable.svelte:175` gate confirmed. |
| 9 | 40-26: expanded device group shows each device's own place; the group row shows a place only when uniform across the group | ✓ VERIFIED | `list_by_ids` now uses `from_row_with_short_path` (confirmed, was `from_row` with hardcoded `None`); `place_distinct_count` computed identically in all three `list_grouped` SQL branches and threaded through `DeviceGroupRow`/`DeviceGroup` DTOs to `DeviceGroupRow.svelte` (confirmed). Minor caveat (WR-11, not escalated): the FTS-search branch's place-path subqueries don't apply the same `MATCH` filter as the outer query, so in a narrow edge case (search + heterogeneous same-model devices) the displayed path could belong to a device outside the matched set even though `place_distinct_count` says 1. |
| 10 | 40-27: LAN print/export of the Перемещения report never produces a duplicated first page | ✓ VERIFIED | `printViaTopLevel` now clears `printRoot`/destroys the previous `activePolisher` UNCONDITIONALLY at the start of every run (confirmed at lines ~393-395), and `handlePrint` has a `printing` re-entrancy guard (confirmed at lines ~542-556). Residual edge case (WR-04, not escalated): the previous run's `afterprint` listener is not explicitly removed at the start of the next run, which could theoretically race a very fast second click against a slow first-run `afterprint` in some engines — flagged for human verification, not a confirmed reproducible defect. |

**Score:** 7/10 gap-closure truths fully verified, 2 partial (audit-trail/edge-case caveats not
escalated to blocking), 1 failed outright (auto-return fallback).

### Deferred Items

None. Phase 41 (АРМ) and later phases were checked against every gap found here; none of the
CR-01/CR-02/CR-03/WR-10 gaps are covered by a later phase's stated goal or success criteria —
Phase 41 addresses workstation-composition place cascades, a different (though related) surface,
and does not mention the reader pool, the storage-place fallback, or the report act-number
formatting.

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `migrations/V040__place_movements.sql` | `place_movements` table + indexes | ✓ VERIFIED | UAT test 1 confirms migration applies cleanly on both a fresh dev DB and a real V38 working copy. |
| `crates/trackly-infra/src/repos/cartridges_sqlite.rs::cascade_place_for_printer_in_tx` | Cascade printer place to cartridges | ⚠️ STUB-LIKE GAP | Exists, wired, tested for `Some->Some`; untested and unsafe for `Some->None` (see gaps). |
| `crates/trackly-infra/src/repos/cartridges_sqlite.rs::last_known_storage_place_in_tx` | Storage-place fallback | ✗ FUNCTIONALLY INCOMPLETE | Exists, wired, but its only positive test does not exercise a reachable production path (see gaps). |
| `crates/trackly-app/src/services/place_movement_service.rs::get_timeline` | Timeline read | ⚠️ WIRED BUT RISKY | Exists, wired to both transports, functionally correct output; concurrency-unsafe implementation (nested pool acquire, no timeout). |
| `ui/src/lib/components/MovementTimeline.svelte` | Shared timeline component | ✓ VERIFIED | Mounted in 3 real screens + showcase; D-06 explanatory text present. |
| `ui/scripts/check-report-type-parity.mjs`, `ui/scripts/check-print-idempotency.mjs` | Structural regression gates | ⚠️ WEAKER THAN INTENDED | Both gates pass today (no current bypass in the actual codebase), but the review demonstrated by mutation that both can be satisfied by a comment rather than real behavior — a future regression of the same shape could slip through silently. Not a gap against the CURRENT code, but a gap in the gate's protective value. |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| `device_service.rs::update` | `cartridges_sqlite.rs::cascade_place_for_printer_in_tx` | direct call inside the same `tx` | ✓ WIRED (partially unsafe) | Called unconditionally on any place change including clearing — see gaps. |
| `cartridges_sqlite.rs` auto-return branch | `last_known_storage_place_in_tx` | direct call when `previous_cartridge_place_id` is `None` | ✓ WIRED (functionally incomplete) | Wired correctly; the function it calls doesn't return the right answer in the common case. |
| `MovementTimeline.svelte` | `ActsPage.svelte` | `onNavigateToAct` → `#/acts?id=N` → `activeTab` derived from `act.act_type`/`archived` | ✓ WIRED | Confirmed end-to-end. |
| `ReportsPage.svelte` | `ReportTable.svelte` | `reportType={reportTypeKey()}` prop | ✓ WIRED | Confirmed single call site, correct value. |
| `PdfPreviewModal.svelte::handlePrint` | `printViaTopLevel` | `printing` in-flight guard | ✓ WIRED | Confirmed. |

### Behavioral Spot-Checks

Not run as live browser/app sessions (out of scope for this code-level pass — see Human
Verification Required). `cargo fmt --all --check` was re-run directly and confirmed clean.

### Requirements Coverage

| Requirement | Source Plans | Description | Status | Evidence |
|---|---|---|---|---|
| HST-01 | 40-01,03,04,05,07,08,13,14,19,21,22,26 | Каждая смена места фиксируется в истории с откуда/куда/когда/кем/почему | ⚠️ PARTIALLY SATISFIED | Manual/act paths solid (UAT pass); cascade write path has a silent, unlogged data-loss case (CR-03) and the auto-return fallback still loses place data in the common case (CR-02) — both are direct violations of HST-01's own wording for those specific write sites. |
| HST-02 | 40-02,10,14,15,16,17,24 | Пользователь видит таймлайн перемещений в карточке устройства и картриджа | ⚠️ PARTIALLY SATISFIED | UI-level truth holds (UAT pass); the read path backing it has an unresolved concurrency/availability risk (CR-01) that is outside this phase's UAT scope (single-user manual testing would never surface it) but squarely inside the project's own stated 20-concurrent-user LAN requirement. |
| HST-03 | 40-06,09,10,20,24 | Акт приёма-передачи автоматически меняет место и фиксирует ссылку на номер акта в истории | ⚠️ PARTIALLY SATISFIED | Timeline surface fully correct after 40-24; the movements REPORT (a different surface, also this phase's deliverable) still shows the wrong number for return acts. |
| HST-04 | 40-11,12,18,25,27 | Пользователь может получить отчёт о перемещениях с фильтром по месту и типу устройства | ✓ SATISFIED | All UAT tests for this requirement pass; deleted-badge and print-duplication gaps closed and confirmed by code read. |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `crates/trackly-app/src/services/device_service.rs` | ~338-348 | Missing guard on a `Some -> None` branch before a destructive cascade | 🛑 Blocker | Silent, unlogged data loss (CR-03) |
| `crates/trackly-infra/src/repos/cartridges_sqlite.rs` | ~928-943 | Incomplete fallback query (single-direction lookup) presented as fixing a bug it doesn't fix in the common case | 🛑 Blocker | User-visible defect (CR-02) survives behind a green test |
| `crates/trackly-app/src/services/place_movement_service.rs` | ~64,145-154 | Nested resource acquisition from a bounded pool with no timeout | 🛑 Blocker | Unrecoverable app-wide DB-read hang under concurrent load (CR-01) |
| `crates/trackly-app/tests/cartridges_lifecycle.rs` | ~1246-1260 | Test seeds unreachable DB state via raw SQL to make a broken code path pass | ⚠️ Warning | Test gives false confidence; masks CR-02 |
| `crates/trackly-app/src/services/report_service.rs` | ~1479,1505 | Raw column read where a canonical formatter exists and is used elsewhere for the same data | ⚠️ Warning | Screen/report number divergence for return acts (WR-10) |
| `ui/scripts/check-report-type-parity.mjs`, `check-print-idempotency.mjs` | multiple | Structural gate anchored on text position/comment content rather than parsed AST semantics | ⚠️ Warning | Gate can be satisfied by a comment (proven by mutation in 40-REVIEW.md); not exploited in current code |

No `TBD`/`FIXME`/`XXX` debt markers found in any file modified by plans 40-01..40-27.

### Human Verification Required

### 1. Live re-run of the 5 previously-failing UAT tests after gap closure

**Test:** In the running desktop app and in a LAN browser, repeat UAT tests 5, 7, 13, 16, 18 from
`40-UAT.md` exactly as originally performed.
**Expected:** Each of the 5 originally-reported issues no longer reproduces (except the two
confirmed-still-broken scenarios below, which should reproduce until fixed).
**Why human:** This report verifies the fix code exists and is wired; it does not replace a live
re-run of the exact UAT script, which is the only way to confirm the fix "feels" fixed to the
reporting user.

### 2. Clearing a printer's place with attached cartridges

**Test:** Attach a cartridge to a printer, then edit the printer and clear its «Место» field
(leave it empty), save. Open the cartridge's card.
**Expected (currently, per this report):** The cartridge silently loses its recorded place with
no entry in its movement history — this is the CR-03 gap, expected to reproduce until fixed.
**Why human:** Confirms the code-level finding against the real running UI before treating it as
authoritative for a product decision.

### 3. First install-then-replace cycle for a freshly created cartridge

**Test:** Create a new cartridge at a storage place, install it into a printer, then install a
second cartridge into the SAME printer without specifying a place for the first cartridge's
return.
**Expected (currently, per this report):** The first cartridge's place is cleared to empty
instead of falling back to its original storage place — this is the CR-02 gap, expected to
reproduce until fixed (this is the exact UAT-16 scenario).
**Why human:** Confirms the code-level finding against the real running UI.

## Gaps Summary

Four gaps block clean phase-goal achievement, three of them major:

1. **Printer place clear silently wipes attached cartridges' places** (CR-03) — new code from
   this phase's own gap-closure round, unlogged, untested, directly contradicts HST-01.
2. **Auto-return storage-place fallback doesn't cover the scenario it was built for** (CR-02) —
   the UAT-16 defect this gap plan claims to close still reproduces on the common path; its
   regression test passes only because it seeds unreachable DB state.
3. **Reader-pool nested-acquire has no timeout and can deadlock all DB reads** (CR-01) — inherited
   from Plan 40-10, not a gap-closure regression, but still the sole mechanism behind HST-02 and
   unresolved; severity is amplified by the project's own 20-concurrent-LAN-user requirement.
4. **Movements report shows the wrong number for return acts** (minor) — a screen/export
   consistency gap on a surface (`report_service.rs`) this phase itself built, parallel to the one
   the phase just fixed on the timeline surface.

Five of the seven original UAT gaps are closed and independently confirmed by code read (deleted
badge, grouped-list inversion, act-link subsection/number in the timeline, LAN print duplication,
install-place-optional). rustfmt drift (CR-04) is fixed. The remaining work is concentrated in
`crates/trackly-infra/src/repos/cartridges_sqlite.rs` and `place_movement_service.rs` — the same
two files the code review already pointed at, now independently confirmed by direct verification.

---

_Verified: 2026-09-03T22:30:00Z_
_Verifier: Claude (gsd-verifier)_
