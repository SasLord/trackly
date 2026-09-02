---
phase: 40-movement-history
verified: 2026-09-02T15:32:10Z
status: human_needed
score: 4/4 success criteria mechanically verified; 5 manual UAT items outstanding (UI runtime + print/export visual checks)
overrides_applied: 0
human_verification:
  - test: "Открыть карточку устройства (модалка «Просмотр устройства», D-14) с ≥2 записями в истории и проверить, что секция «История перемещений» рендерится и консоль браузера/webview чистая"
    expected: "Таймлайн рендерится без ошибок рун Svelte 5; строки в формате «ДД.ММ — откуда → куда · Кем · причина»"
    why_human: "svelte-check/eslint не видят рантайм-ошибок рун Svelte 5; ни одна из четырёх точек монтирования MovementTimeline не наблюдалась в запущенном приложении"
  - test: "Открыть карточку картриджа с историей операций и с перемещениями; убедиться, что ОБЕ секции («Журнал операций» и «Перемещения», D-16) присутствуют и не потеряны"
    expected: "Видны обе секции одновременно, «Журнал операций» показывает прежний числовой place_id (сознательный долг, не эта фаза), «Перемещения» — новый таймлайн с читаемыми путями"
    why_human: "Визуальная проверка layout/потери секции недоступна текстовым assert'ам"
  - test: "Навести курсор на длинный сокращённый путь в таймлайне (D-17/D-18) и убедиться, что tooltip показывает полный путь на реальных ширинах модалки"
    expected: "title= показывает полный сохранённый путь; сокращённая форма читаема в строке"
    why_human: "Layout/overflow при реальной ширине не виден текстовым тестам"
  - test: "Экспортировать отчёт «Перемещения» в PDF и открыть файл; затем открыть редактор шаблонов и убедиться, что предпросмотр по-прежнему рендерится"
    expected: "PDF открывается, кириллица не искажена, разбиение на страницы корректно; предпросмотр шаблона не ломается (WR-01's «(удалено)» суффикс не вводит новую переменную шаблона — риск низкий, но не проверен визуально)"
    why_human: "Strict-undefined в редакторе шаблонов и PDF-наложение текста невидимы тестам на извлечение текста"
  - test: "Собрать `pnpm --dir ui build` и повторить оба предыдущих UI-проверки (таймлайн в карточках, отчёт) в LAN-браузере"
    expected: "Тот же результат, что и в десктоп-приложении; никакой асимметрии каскада печати или DOM-протечки"
    why_human: "Печать/DOM-каскад асимметричны между десктопом и браузером — известный паттерн проекта (lan_print_dom_leakage)"
---

# Phase 40: История перемещений — Verification Report

**Phase Goal:** Каждая смена места устройства или картриджа наблюдаема — вручную, актом или
(структурно, на будущее) перетаскиванием на карте — с указанием откуда, куда, когда, кем и почему.

**Verified:** 2026-09-02T15:32:10Z
**Status:** human_needed
**Re-verification:** No — initial verification

## Method

This is an initial (non-re-verification) goal-backward pass. Every claim below is backed by a
direct code read (`Read`/`grep` against the actual files in `crates/` and `ui/src/`), not by
trusting `40-SUMMARY.md` or `40-REVIEW.md` prose. Where `40-REVIEW.md`'s Fix Outcomes section
claimed a defect was fixed, the fix was independently re-derived by reading the current code at
the cited line numbers (not by trusting the commit message). The full workspace test suite and
both frontend gates (`svelte-check`, `pnpm lint`) were re-run fresh in this session, not copied
from the review's reported numbers.

```
TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test --workspace --no-fail-fast \
  -- --test-threads=1 --skip login_remember_persistent_cookie
→ exit 0, 131 binaries, 1156 passed, 0 failed  (matches the state claimed in the brief)

pnpm --dir ui svelte-check → 284 files, 0 ERRORS, 60 WARNINGS (pre-existing rune-capture style
  warnings across the whole app, not phase-40-specific defects — see analysis below)

pnpm --dir ui lint → eslint clean, prettier clean, all 7 check-*.mjs gates PASS including
  check-placepath-parity (23 cases, 0 divergences) and check-place-path-short

node scripts/check-privacy.mjs --hashes scripts/privacy-tokens.sha256 → PASS, 0 violations
```

## Goal Achievement

### Success Criteria (from ROADMAP.md)

| # | Criterion | Status | Evidence |
|---|-----------|--------|----------|
| 1 | Пользователь видит в карточке устройства и картриджа таймлайн перемещений: откуда, куда, когда, кем, почему | ⚠️ MECHANICALLY VERIFIED / UI RUNTIME UNVERIFIED | Backend: `PlaceMovementService::get_timeline` (gated `ReadPlaces`) → `MovementEntryDto` (all 5 fields present: `from/to_place_path(_short)`, `created_at_utc`, `actor_display`, `source`/`note`/`act_number`). Frontend: `MovementTimeline.svelte` mounted in `PlaceEntityViewModal.svelte` (device+printer, entry point wired from `DeviceContextMenu.svelte`→`DeviceListRow.svelte`), `CartridgeDetail.svelte`, `PrinterDetail.svelte` (3 distinct mount points, all confirmed by direct grep of the render tree). Compiles clean (`svelte-check` 0 errors), all rune usage in `MovementTimeline.svelte` is pure prop-driven with no local effect/derived entanglement. **Not observed rendering in a running app** — see Human Verification #1/#2/#3. |
| 2 | Ручное изменение места фиксируется в истории с причиной «вручную»; схема причины предусматривает будущий источник «перетаскиванием на карте» | ✓ VERIFIED | `device_service.rs:316` and `cartridge_service.rs:275` both call `record_movement_if_applicable(..., MovementSource::Manual, ...)` on the manual-edit write path. `MovementSource` enum (`trackly-core/src/domain/place_movements.rs`) has 4 variants incl. `Map` and `Workstation`, unused today but reserved; migration `V040` stores `source` as unconstrained `TEXT` (no `CHECK`), confirmed by direct read of `V040__place_movements.sql`. Tests `place_movements_write_sites_devices.rs`/`_cartridges.rs` pass. |
| 3 | Акт приёма-передачи автоматически меняет место переданных устройств и создаёт запись в истории со ссылкой на номер акта | ✓ VERIFIED | 7 call sites in `act_service.rs` (create:498, update added:781, update removed/CR-01 fix:952, do_return:1530, update_return removed/CR-01 fix:2022, update_return added:2109, update_return retained:2195) all pass `act_id: Some(...)`. CR-01 (act-edit-drops-a-device losing its history entry) independently re-verified fixed: `place_before_restore = devices_repo.get_in_tx(...)` captured BEFORE `restore_from_snapshot_in_tx`, then `record_movement_if_applicable` called with the real pre/post values — matches the review's stated correction (not its original, wrong, snippet). CR-02 (act number resolved as wrong SQL type, silently `None`) independently re-verified fixed: `place_movement_service.rs:83-94` now reads `r.get::<_, i64>(0)` and `.map(|n| n.to_string())`, with a passing regression test `place_movements_act_number_resolves`. D-03 undo scoping verified: `delete_by_act_id_in_tx` called at all 3 correct points in `delete_soft`'s LIFO cascade (return-loop, handover-own, standalone-return). |
| 4 | Пользователь может получить отчёт о перемещениях за период с фильтром по месту и типу устройства | ✓ VERIFIED (mechanically) / PDF visual UNVERIFIED | `report_service.rs::query_movements_inner` implements D-23 columns, D-24 two independent subtree-inclusive filters combined by AND (`from_subtree`/`to_subtree` CTEs, confirmed by direct read), D-25 `is_deleted` marker now present in CSV/PDF body (WR-01 fix independently re-verified: `row_field`'s `"device_name"` arm appends `" (удалено)"`), D-26 CSV/PDF export parity. `ReportSubNav.svelte`/`ReportFilters.svelte`/`ReportsPage.svelte` wire the new "Перемещения" domain with `fromPlaceId`/`toPlaceId` filters. EX-01 fix (tab counter always 0) independently re-verified: `get_report_counts` now has a `"movements"` branch calling the same `query_movements_inner`. Both transports gate on `Action::ReadPlaces`, including export (WR-02 fix independently re-verified via `export_gate_action` helper used by both Tauri and HTTP handlers). **PDF rendering/Cyrillic fidelity not visually inspected** — see Human Verification #4/#5. |

**Score:** 4/4 success criteria have a fully-wired, tested backend and frontend implementation.
0/4 have been visually confirmed running. Per the phase brief's own framing, this is reported
honestly as `human_needed`, not upgraded to `passed`.

### D-01..D-29 Decision Coverage (40-CONTEXT.md)

| Decision | Status | Evidence |
|---|---|---|
| D-01 Standalone table, single writer, same-tx | ✓ | `V040__place_movements.sql` is a dedicated table; `record_movement_if_applicable` is the sole insert path (doc comment + grep confirms no other `INSERT INTO place_movements` anywhere in the codebase); all 13 call sites operate on the caller's already-open `&Transaction<'_>`. |
| D-02 No retroactive backfill | ✓ | Migration inserts nothing; `place_movements_starts_empty` test passes. |
| D-03 Act delete/undo deletes movement rows | ✓ | `delete_by_act_id_in_tx` called at 3 points inside `delete_soft`'s LIFO cascade, scoped per-act, in the same transaction as `undo_device_mutations_for_act`. |
| D-04 Status-only change is not a movement | ✓ | `is_reportable_place_change` requires both sides `Some` AND different; unit tests cover the equal-Some case. |
| D-05 Cartridge ops write only if place changes | ✓ | `cartridges_sqlite.rs:586` transition main mutation and `:674` nested auto-return both go through the same guarded helper; `source` stays `Manual`, distinction lives in `note` (confirmed by direct read). |
| D-06 Only place→place (NULL edges skipped) | ✓ | `is_reportable_place_change` requires both `Some`; 3 dedicated repo tests (`record_movement_skips_when_place_unchanged`, `_on_first_assignment_from_null`, `_when_cleared_to_null`) all pass; act `do_return`'s `effective_location=None` case explicitly relies on this guard (comment + code confirmed). |
| D-07 Closed 4-value source enum + free note | ✓ | `MovementSource` enum, 4 variants, `TEXT` column (no SQL CHECK — deliberate, matches IN-01 lesson), `note: Option<&str>`. |
| D-08 Note optional | ✓ | `note TEXT NULL` in schema; DTO `note: Option<String>`. |
| D-09 Dual actor snapshot (user_id + ФИО) | ✓ | `user_id NULL`/`actor_name_snapshot TEXT NULL` columns; snapshot resolved at write time via `SELECT full_name FROM users` with `.ok()` soft-degrade. |
| D-10 Dual place snapshot (id + path) | ✓ | `from_place_id`+`from_place_path`, `to_place_id`+`to_place_path`, both resolved via `PlaceRepository::full_path` at write time, never a later JOIN. |
| D-11 Actor display precedence (ФИО→login→«система») | ✓ | `place_movement_service.rs`'s `actor_display` match arm implements exactly this 3-way fallback. |
| D-12 Read access Admin+Manager via ReadPlaces, gated on both transports | ✓ | `PlaceMovementService::get_timeline` and `build_reports_list_movements` both call `authorize(caller, &Action::ReadPlaces)` first; both Tauri `place_movements_get_timeline`/`reports_list_movements` and HTTP `handler_get_timeline`/`handler_list_movements` delegate to the same gated `build_*` function; `role_endpoint_matrix.rs` Cases 52-59 assert Manager-allow/Employee-403 on both transports for timeline, report list, report export, and bulk-move. |
| D-13 Mutation permissions unchanged | ✓ | Manual writes reuse existing `MutateDevices`/`MutateCartridges` (no new mutate Action introduced); bulk move explicitly reuses these two, not a new blanket action. |
| D-14 Device — extend PlaceEntityViewModal, open from device list | ✓ | `PlaceEntityViewModal.svelte` renders `DeviceFormBody` (readonly) + new "История перемещений" `DetailSection`; entry point wired `DeviceListRow.svelte`→`DeviceContextMenu.svelte`→`PlaceEntityViewModal` («Просмотр» menu item). |
| D-15 Minimal card scope (no act list, no new actions) | ✓ | Modal footer only has «Перейти к устройству»/«Редактировать» — no related-acts list added, confirmed by direct read of the full component. |
| D-16 Cartridge — keep BOTH sections | ✓ | `CartridgeDetail.svelte` retains "Журнал операций" (unchanged, still the raw numeric `place_id` compromise, per file-header comment) AND adds a new "Перемещения" `DetailSection` right below it — both present simultaneously in the render tree. |
| D-17 Short path in row + full path tooltip | ✓ | `MovementTimeline.svelte`: `title={entry.from_place_path}` (full) on a button showing `entry.from_place_path_short ?? entry.from_place_path`. |
| D-18 Shorten the STORED snapshot, not live path | ✓ | `place_movement_service.rs` calls `compute_place_path_short(readers, Some(row.from_place_id), Some(row.from_place_path.clone()))` — operates on the snapshot column, single owner (`place_path_display.rs`), no JS mirror (confirmed by `check-placepath-parity` PASS + no duplicate implementation found by grep). |
| D-19 Clickable place + act number | ✓ | `onNavigateToPlace`/`onNavigateToAct` wired in `MovementTimeline.svelte`, consumed in `PlaceEntityViewModal`/`CartridgeDetail`/`PrinterDetail` via `push('#/places?id=...')`/`push('#/acts?id=...')`. |
| D-20 Newest-first, unpaginated | ✓ | `ORDER BY created_at_utc DESC, id DESC` in `get_history`, no `LIMIT`; `MovementTimeline.svelte` renders the full `entries` array with no "load more" control. |
| D-21 Printer recorded as `device`, shares the same timeline | ✓ | `MovementEntityKind` has no `Printer` variant; `PrinterDetail.svelte` calls the timeline with `entityType: 'device', entityId: p.deviceId`. |
| D-22 Own "Перемещения" ReportSubNav group | ✓ | `ReportSubNav.svelte` `DomainKey` includes `'movements'` as its own top-level domain, not nested. |
| D-23 Report columns (Дата·Предмет·Тип·Откуда·Куда·Кем·Причина) | ✓ | `columns_for("movements")` in `report_service.rs` (Tauri side) returns exactly these 7 labels; index-alignment test `column_labels_for_is_index_aligned_with_columns_for` passes. |
| D-24 Two independent subtree-inclusive place filters, AND semantics | ✓ | `query_movements_inner` builds `from_subtree`/`to_subtree` `WITH RECURSIVE` CTEs independently, both pushed into the same AND-joined `clauses` vec; `report_movements_place_filters` test (seeds 3 movements, asserts exactly the AND-matching one survives) passes. |
| D-25 Soft-deleted items stay, marked «удалено» | ✓ | `is_deleted` computed in the SQL (`CASE WHEN ... THEN 1 ELSE 0 END`), rendered as a live-table badge (`ReportTable.svelte::showDeletedBadge`) AND (after WR-01 fix) appended to the exported CSV/PDF body via `row_field`'s `"device_name"` arm. |
| D-26 CSV+PDF export parity | ✓ | `build_reports_export_csv`/`_export_pdf` both handle `report_type: "movements"`; `report_movements_export_csv_has_d23_headers`/`_export_pdf_has_d23_headers` plus the new body-content tests all pass. |
| D-27 Manual place change stays in the existing edit form | ✓ | No new "Переместить…" dialog introduced for single-item moves; `device_service.rs::update`/`cartridge_service.rs::update` write `MovementSource::Manual` inline with the existing PATCH flow. |
| D-28 Bulk move via place-contents panel | ✓ | `PlaceContents.svelte` "Перенести всё содержимое в…" button → `places_move_subtree_contents` → `PlaceService::move_subtree_contents` (one transaction, one row per moved item, `MutateDevices`+`MutateCartridges` gate) — independently re-read in full, including the confirm-count fetch's `cancelled` closure guard against stale async responses. |
| D-29 No WebSocket, load on open/after save | ✓ | No `WebSocket`/`ws.` usage found in any of the 3 mount-point files or `MovementTimeline.svelte` itself (grep confirmed zero hits); timeline is fetched inside each parent's own `$effect` keyed on the entity prop. |

**All 29 locked decisions are implemented and independently confirmed in the codebase — none
found merely claimed.** No decision required an override.

### Required Artifacts

| Artifact | Expected | Status | Details |
|---|---|---|---|
| `migrations/V040__place_movements.sql` | New table, append-only, indexes for entity/created/from/to/act | ✓ VERIFIED | Read in full; matches D-01/D-02/D-06/D-07/D-08/D-09/D-21/D-03 doc comments and schema. |
| `crates/trackly-core/src/domain/place_movements.rs` | `MovementSource`, `MovementEntityKind`, `is_reportable_place_change` | ✓ VERIFIED | Pure domain, 190 lines, 10 unit tests, all pass. |
| `crates/trackly-infra/src/repos/place_movements_sqlite.rs` | Single-writer repo: insert/record_movement_if_applicable/delete_by_act_id/get_history | ✓ VERIFIED | 237 lines, matches doc contract; every write site funnels through `record_movement_if_applicable`. |
| `crates/trackly-app/src/services/place_movement_service.rs` | HST-02 read service, `ReadPlaces` gate | ✓ VERIFIED | 149 lines; CR-02 fix confirmed present. |
| `crates/trackly-app/src/dto/place_movements.rs` | `MovementEntryDto` | ✓ VERIFIED | Flat DTO, all fields present, 3 unit tests pass. |
| `crates/trackly-app/src/{tauri_cmds,http}/place_movements.rs` | Both-transport timeline read | ✓ VERIFIED | Both gate `ReadPlaces`, both delegate to same `build_place_movements_get_timeline`. |
| `ui/src/lib/components/MovementTimeline.svelte` | Shared, prop-driven timeline row component | ✓ VERIFIED (static) / UNVERIFIED (runtime) | Pure presentational, no local state entanglement; not observed rendering. |
| `ui/src/features/places/PlaceEntityViewModal.svelte` | D-14 device+printer mount point | ✓ VERIFIED (static) / UNVERIFIED (runtime) | Wired correctly; not observed rendering. |
| `ui/src/features/cartridges/CartridgeDetail.svelte` | D-16 cartridge mount point, both sections | ✓ VERIFIED (static) / UNVERIFIED (runtime) | Both sections present in source; not observed rendering. |
| `ui/src/features/printers/PrinterDetail.svelte` | D-21 printer mount point | ✓ VERIFIED (static) / UNVERIFIED (runtime) | Reads device-id timeline; not observed rendering. |
| `ui/src/features/reports/{ReportSubNav,ReportFilters,ReportsPage,ReportTable}.svelte` | HST-04 report UI | ✓ VERIFIED (static) / UNVERIFIED (runtime) | Domain, filters, export wired; not observed rendering. |
| `ui/src/features/places/PlaceContents.svelte` | D-28 bulk-move dialog | ✓ VERIFIED (static) / UNVERIFIED (runtime) | Button + modal + cancelled-guard fetch wired; not observed rendering. |

### Key Link Verification

| From | To | Via | Status | Details |
|---|---|---|---|---|
| `device_service::update` | `place_movements` | `record_movement_if_applicable` in same tx | ✓ WIRED | Confirmed line 316. |
| `cartridge_service::update` | `place_movements` | same helper | ✓ WIRED | Confirmed line 275. |
| `cartridges_sqlite::transition_in_tx` (main + nested auto-return) | `place_movements` | same helper, correct entity attribution (`prev_id` not new cartridge) | ✓ WIRED | Confirmed lines 586, 674-687; auto-return uses `prev_id`/`prev_current.place_id`, not the newly-installed cartridge. |
| `act_service::create/update/do_return/update_return` (7 sites) | `place_movements` | same helper | ✓ WIRED | All 7 confirmed, including the two CR-01-fixed "removed"/"un-return" branches. |
| `place_service::move_subtree_contents` (device+cartridge branches) | `place_movements` | same helper | ✓ WIRED | Confirmed lines 719, 765. |
| `act_service::delete_soft` (3 undo points) | `place_movements` deletion | `delete_by_act_id_in_tx` in same tx as `undo_device_mutations_for_act` | ✓ WIRED | Confirmed lines 2643, 2676, 2704. |
| `PlaceMovementService::get_timeline` | Tauri + HTTP | `authorize(ReadPlaces)` first line, both transports delegate to same `build_*` | ✓ WIRED | Confirmed. |
| `ReportService::list_movements`/export | Tauri + HTTP | `Action::ReadPlaces` via `export_gate_action` (list) and same helper (export, both transports) | ✓ WIRED | Confirmed both `tauri_cmds/reports.rs` and `http/reports.rs` delegate to the same gated `build_*` functions. |
| `DeviceListRow` → `DeviceContextMenu` → `PlaceEntityViewModal` | UI mount chain | prop drilling + `viewRow` state | ✓ WIRED (static) | Confirmed by grep chain; not runtime-observed. |
| `PlaceContents` "Перенести всё содержимое" | `places_move_subtree_contents` | `apiCall` | ✓ WIRED (static) | Confirmed; not runtime-observed. |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|---|---|---|---|---|
| `MovementTimeline.svelte` (all 3 mount points) | `entries: MovementEntryDto[]` | `apiCall('place_movements_get_timeline', ...)` inside each parent's `$effect` | Yes — real DB query via `SqlitePlaceMovementsRepository::get_history`, no static fallback found | ✓ FLOWING (mechanically; UI render not observed) |
| `ReportsPage.svelte` movements domain | `filter.from_place_id`/`to_place_id` | `ReportFilters.svelte` bound to real component state, passed through to `reports_list_movements` | Yes — real recursive CTE query | ✓ FLOWING (mechanically; UI render not observed) |
| `PlaceContents.svelte` bulk-move dialog | `moveCount` | Own `$effect` fetching `places_contents` with `nested: true`, cancelled-guarded | Yes — real subtree query, correctly always-nested regardless of the "Только здесь" toggle | ✓ FLOWING (mechanically; UI render not observed) |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|---|---|---|---|
| Full workspace test suite (includes every HST-01..04 automated test named in 40-VALIDATION.md's Per-Task Verification Map, plus all 5 fix regression tests) | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test --workspace --no-fail-fast -- --test-threads=1 --skip login_remember_persistent_cookie` | exit 0, 131 binaries, 1156 passed, 0 failed | ✓ PASS |
| `place_movements_act_edit_remove_records_reversion`, `place_movements_return_edit_unreturn_records_reversion` (CR-01 regression) | included above | `ok` | ✓ PASS |
| `place_movements_act_number_resolves` (CR-02 regression) | included above | `ok` | ✓ PASS |
| `report_movements_export_csv_marks_deleted_device_in_body`, `report_movements_export_pdf_marks_deleted_device_in_body` (WR-01 regression) | included above | `ok` | ✓ PASS |
| `movements_export_gate_is_read_places_not_read_data`, `other_report_types_keep_read_data_gate` (WR-02 regression) | included above | `ok` | ✓ PASS |
| `report_movements_get_report_counts_reflects_real_rows`, `report_movements_get_report_counts_respects_place_filter` (EX-01 regression) | included above | `ok` | ✓ PASS |
| Frontend type/lint gates | `pnpm --dir ui svelte-check` / `pnpm --dir ui lint` | 0 errors (60 pre-existing whole-app warnings, none phase-40-introduced by content); all 7 `check-*.mjs` PASS | ✓ PASS |
| Privacy gate | `node scripts/check-privacy.mjs --hashes scripts/privacy-tokens.sha256` | PASS, 0 violations | ✓ PASS |
| Live app rendering of any of the 4 timeline mount points, the report page, or the bulk-move dialog | — | not run (no server start permitted in this verification pass; also a Tauri desktop app can't be meaningfully spot-checked headlessly) | ? SKIP — routed to Human Verification |

### Probe Execution

No `scripts/*/tests/probe-*.sh` files exist in this repository and neither PLAN nor SUMMARY nor
VALIDATION for Phase 40 reference a probe script. Step 7c: SKIPPED — not applicable to this
project's test infrastructure (plain `cargo test`, no probe-script convention).

### Requirements Coverage

| Requirement | Source Plan(s) | Description | Status | Evidence |
|---|---|---|---|---|
| HST-01 | 40-01, 40-03..40-09 | Каждая смена места записывается в историю (откуда/куда/когда/кем/почему; вручную/актом/картой) | ✓ SATISFIED | All 13 write sites instrumented, D-04/D-06 guard centralized, D-09/D-10/D-11 satisfied, schema future-proofed for map/workstation sources. |
| HST-02 | 40-10, 40-15..40-17 | Таймлайн в карточке устройства и картриджа | ⚠️ SATISFIED (backend+wiring) / NEEDS HUMAN (visible render) | Read path, DTO, and all 3 UI mount points wired and type-checked; not observed rendering live. |
| HST-03 | 40-06, 40-09, 40-20 | Акт автоматически меняет место и создаёт запись со ссылкой на номер акта | ✓ SATISFIED | 7 write sites incl. two CR-01-fixed reversion branches; CR-02-fixed act-number resolution; D-03 undo scoping. |
| HST-04 | 40-11..40-14, 40-18..40-19 | Отчёт за период с фильтром по месту и типу устройства | ⚠️ SATISFIED (backend+wiring) / NEEDS HUMAN (PDF visual) | Filters, columns, D-25 marker (list+export), export gate, tab-counter all correct and tested; PDF Cyrillic/pagination fidelity not visually inspected. |

No orphaned requirements found in `REQUIREMENTS.md` for Phase 40 beyond HST-01..04 — the section
also lists WKS-03/WKS-06 and MAP-* as depending on this phase's schema (not owned by it); their
schema-readiness prerequisite (D-07's `map`/`workstation` source tokens, `entity_type` covering
both kinds) is confirmed present.

### Anti-Patterns Found

None. Scanned every backend file in `40-REVIEW.md`'s `files_reviewed_list` plus the UI mount-point
files for `TBD`/`FIXME`/`XXX`/`TODO`/`HACK`/`PLACEHOLDER` and Russian placeholder-copy patterns
(`placeholder`, `coming soon`, `not yet implemented`) — zero hits. No empty-implementation
patterns (`return null`/`return {}`/`=> {}`) found in the phase's own new files. The one known,
explicitly-documented compromise (`CartridgeDetail.svelte`'s pre-existing numeric `place_id`
display in "Журнал операций") is unchanged by this phase, not a new defect, and is called out in
its own file-header comment as intentionally out of scope (D-16).

### Human Verification Required

See YAML frontmatter `human_verification` for the structured list. Summary: 5 items, all carried
forward verbatim from `40-VALIDATION.md`'s "Manual-Only Verifications" table (none dropped) —
rune-runtime rendering in both card modals, tooltip-at-real-width behavior, PDF export + template
editor regression, and LAN-browser parity. These exist precisely because `svelte-check`/eslint/
`cargo test` cannot observe Svelte 5 rune runtime errors, layout/overflow, or print-time DOM
rendering — this is a known, documented limitation of this project's test infrastructure
(`compile_gates_miss_svelte_runtime` in project memory), not a gap specific to this phase's
execution quality.

### Gaps Summary

No code-level gaps were found. All 4 ROADMAP success criteria have complete, correctly-wired,
tested implementations at every layer (schema → domain → repo → service → both transports → UI
component tree). All 29 CONTEXT.md decisions were independently re-derived from the code, not
copied from REVIEW.md's claims. Both defects the code review found (CR-01, CR-02) plus both
warnings (WR-01, WR-02) plus the orchestrator-found EX-01 were independently re-verified fixed in
the current `main` branch — each fix's regression test was confirmed present AND passing in a
fresh full-suite run in this session, not merely cited from the review document.

The phase is withheld from `passed` status solely because five behaviors require a running,
visually-inspected app to confirm (Svelte 5 rune runtime correctness, layout at real widths, PDF
visual fidelity, LAN-browser parity) — exactly the set of checks `40-VALIDATION.md` itself
pre-identified as impossible to verify mechanically. This is the honest, non-padded reporting the
brief asked for: "compiles and is wired" is not being upgraded to "works."

---

*Verified: 2026-09-02T15:32:10Z*
*Verifier: Claude (gsd-verifier)*
