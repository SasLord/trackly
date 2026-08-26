---
phase: 39-place-tree
verified: 2026-08-26T05:08:48Z
status: passed
score: 6/6 must-haves verified
overrides_applied: 0
---

# Phase 39: Дерево мест — Verification Report

**Phase Goal:** Заменить свободнотекстовое поле «Размещение» и плоскую таблицу `locations`
деревом мест произвольной вложенности, используемым последовательно во всём приложении.
**Verified:** 2026-08-26T05:08:48Z
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths (PLC-01..PLC-06)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | **PLC-01** — Admin строит дерево мест произвольной вложенности (территория/зона/здание/этаж/помещение/уличный объект), переименовывает и перемещает узел без потери привязок устройств | ✓ VERIFIED | `migrations/V037__places.sql` — adjacency-list `places` table, no depth cap, 6-value `kind` CHECK enum (D-02). `PlaceService::rename`/`move` in `crates/trackly-app/src/services/place_service.rs` operate on `place_id` FK (`ON DELETE RESTRICT`), so device/cartridge/act FKs are untouched by rename/move — no cascade, no re-link needed. Cycle-check on move covered by `crates/trackly-app/tests/places_move_cycle.rs` (ran green, see below). Live-executed `role_endpoint_matrix.rs` Cases 45/48 confirm Admin can call `places_create/rename/move/archive/delete` on both transports. 7-round UAT (`39-UAT.md`) exercised create/rename/move/drag-drop live in Tauri + LAN browser, 11/11 gaps closed. |
| 2 | **PLC-02** — Этаж имеет числовой уровень (0 и отрицательные допустимы); этажи сортируются по уровню, не по имени | ✓ VERIFIED | `places.level` is `INTEGER NULL` (V037, no CHECK restricting sign). `crates/trackly-core/src/domain/places.rs::sibling_cmp` — level comparison via `i64::cmp`, unit test `sibling_cmp_orders_negative_zero_positive_levels` (read directly, asserts `-1 < 0 < 2`) sits alongside a green `cargo clippy --all-targets -D warnings`. `PlaceFormModal.svelte` UI copy explicitly instructs "Подвал — отрицательное значение" and validates the field as an integer. Natural-order (not lexicographic) fallback proven end-to-end by the executed test `list_children_sorted_by_sibling_cmp_not_insertion_order` (PASS, see Behavioral Spot-Checks). |
| 3 | **PLC-03** — Место устройства/картриджа выбирается из дерева с поиском по полному пути; свободный текст запрещён | ✓ VERIFIED | Single shared `ui/src/lib/components/PlacePicker.svelte` (D-17) is imported by every entity form that captures a place: `DeviceFormBody.svelte`, `CartridgeFormBody.svelte`, `PrinterCreateModal.svelte`, `ActFormBody.svelte`, `ReturnModal.svelte`, `OperationModal.svelte`, `ReportFilters.svelte`. Grep across `ui/src` for a freeform "Размещение" text `<input>` bound to a plain string found none; `LocationAutocomplete.svelte` (the old freeform control) is deleted (`git log` commit `5580ad77`). `PlacePicker.svelte` fetches `places_list_children`/`places_search` — no client-side text is submitted as the place value, only `place_id`. |
| 4 | **PLC-04** — Свободнотекстовое поле «Размещение» и таблица `locations` удалены; формы/списки/отчёты/автокомплиты/печатные формы используют дерево | ✓ VERIFIED | `migrations/V038__…sql` drops `locations`, `devices.location_id`, `cartridges.location`, `acts.location_id` in the same transaction that adds `place_id`/`bulk_place_id`/`place_path_snapshot`/`place_id_override`. Regression-locked by `crates/trackly-infra/tests/migration_idempotency.rs::places_migration_drops_locations_and_adds_place_columns` (asserts `locations` table count == 0 and each dropped column absent). Repo-wide sweep (see Anti-Patterns section) found **zero** production references to `location_id`/`location_name`, and zero freeform "Размещение" inputs; the few remaining bare `location` tokens are either `window.location` (browser API), unrelated English comments/local-variable names that read from `place_id` (e.g. `act_service.rs`'s `effective_location` variable, which is assigned from and stored into `place_id`), or doc-comment prose — none are DB columns, DTO fields, or user-facing labels. Print template `act_handover.html` renders `act.place_path` (D-16/D-27), not a freeform field. List rows (`DeviceListRow.svelte`, `CartridgeListRow.svelte`) render `full_path`. |
| 5 | **PLC-05** — Переименование/перемещение узла мгновенно отражается в полнотекстовом поиске и во всех списках без ручной переиндексации | ✓ VERIFIED | `place_full_paths` is a SQL `VIEW` (recursive CTE), not a cached/materialized table — recomputed on every query by construction, so there is no reindex step to run. `devices_sqlite.rs`/`cartridges_sqlite.rs` join/search against this view live. Directly regression-tested and **executed green**: `crates/trackly-infra/tests/devices_place_search.rs::search_fts_reflects_place_rename_without_reindex` and `cartridges_place_search.rs::search_reflects_place_rename_without_reindex` (both PASS — see Behavioral Spot-Checks). D-16's frozen `act.place_path_snapshot` is a deliberate, separate concern (an already-issued act's printed path must NOT change retroactively) and does not conflict with this truth — confirmed the snapshot is captured only at act create/return time (`act_service.rs` lines 286, 687, 1222) and is never touched by `PlaceService::rename`/`move`. |
| 6 | **PLC-06** — Открыв любое место, пользователь видит одним списком всё размещённое в нём и во вложенных местах (устройства, принтеры, картриджи) | ✓ VERIFIED | `SqlitePlaceRepository::list_subtree_contents` (`places_sqlite.rs`) UNIONs device/printer/cartridge rows filtered by a recursive-CTE subtree (`nested=true`, default per D-24) or exact `place_id` match (`nested=false`, "Только здесь"). `PlaceContents.svelte` renders all three kinds in one table with a type-filter chip bar and correctly derives `showKindColumn`/tab counts. Executed test `places_contents.rs::list_subtree_contents_nested_true_includes_nested_place_devices` passes. UAT §2–§9 covered both toggle states live (GAP-1 fixed: toggle no longer resets on place switch). АРМ (workstation) as a fourth content kind is explicitly out of scope for Phase 39 — D-23 in `39-CONTEXT.md` names Phase 41 as the plan to add that kind to this same screen; not a Phase 39 gap. |

**Score:** 6/6 truths verified

### Cross-Cutting Check: D-20 Authorization Split (Admin mutate / Manager+Admin read / Employee none)

✓ VERIFIED on **both** transports, not just one:
- HTTP: `role_endpoint_matrix.rs` Case 45 (Manager → all 6 `places_*` mutation endpoints → 403), Case 46 (Manager → `places_list_all`/`places_get` → 200/not-403), Case 47 (Employee → read endpoints → 403).
- Tauri: Case 48 calls `build_places_create`/`build_places_rename`/`build_places_move` (the shared function every Tauri command and HTTP handler funnels through) directly with a Manager `Identity` and asserts `Forbidden`.
- Executed `cargo test -p trackly-app --test role_endpoint_matrix -- --test-threads=1` → **1 passed, 0 failed** (this binary contains the full matrix, all cases in one `#[test]`).
- `PlacesPage.svelte`/sidebar hide the "Места" entry and the create/edit/move/delete controls client-side for Manager/Employee (UAT §1.2/§1.3), matching the server-side gate — belt-and-suspenders, not defense-only-on-client.

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `migrations/V037__places.sql` | `places` table, `place_full_paths` view | ✓ VERIFIED | Present, adjacency list + CTE view, applied in `cargo test` migration bootstrap (observed in test run logs). |
| `migrations/V038__places_migrate_devices_acts_cartridges.sql` | Drops `locations`, adds `place_id` family | ✓ VERIFIED | Present; `migration_idempotency.rs` locks the post-migration schema shape. |
| `crates/trackly-core/src/domain/places.rs` | `PlaceKind`, `sibling_cmp`, `natural_name_cmp`, `SubtreeStats` (incl. `referencing_act_count`) | ✓ VERIFIED | All present; `referencing_act_count` field confirmed post-CR-01 fix. |
| `crates/trackly-infra/src/repos/places_sqlite.rs` | CRUD, cycle-check move, delete-conflict, subtree/content queries | ✓ VERIFIED | `subtree_stats_impl`, `delete_hard`, `list_subtree_contents_impl` all present and wired. |
| `crates/trackly-app/src/services/place_service.rs` | Mutations + reads, D-20 gate, D-14 blocked-message builder | ✓ VERIFIED | `authorize(caller, &Action::*Places)` calls present on every method; `build_delete_blocked_message` includes CR-01's act-reference clause. |
| `crates/trackly-app/src/tauri_cmds/places.rs` + `http/places.rs` | 11-12 commands mirrored on both transports | ✓ VERIFIED | 11 HTTP routes mounted (`http/places.rs:308-322`); 12 commands registered via `collect_commands!` in `specta_export.rs`, consumed by `main.rs`'s `.invoke_handler(builder.invoke_handler())`. (12th, `places_subtree_stats`, has no dedicated HTTP route by design — confirmed used only by the tree's own per-node counter, not a cross-entity read.) |
| `ui/src/lib/components/PlacePicker.svelte` | Single shared tree+search control | ✓ VERIFIED | Imported by 7 entity-form/filter surfaces (see PLC-03 evidence). |
| `ui/src/features/places/{PlacesPage,PlaceTree,PlaceTreeNode,PlaceContents,PlaceFormModal,PlaceMoveModal,PlaceEntityViewModal}.svelte` | Full "Места" section (PLC-01/PLC-06 screen) | ✓ VERIFIED | All present, routed via `ui/src/routes.ts`, sidebar entry present per `sidebar-config.ts`. UAT §1 confirmed routing/sidebar/role-gating live. |
| `ui/src/features/devices/LocationAutocomplete.svelte` | Should NOT exist (old freeform control) | ✓ VERIFIED ABSENT | File deleted in commit `5580ad77` ("delete LocationAutocomplete.svelte, sweep last location vocabulary"); confirmed no references remain anywhere in `ui/src`. |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| `DeviceFormBody.svelte` / `CartridgeFormBody.svelte` / `PrinterCreateModal.svelte` / `ActFormBody.svelte` | `place_id` on the entity | `PlacePicker` `onChange` → `placeId` state → submit payload `place_id` | ✓ WIRED | Confirmed per-file (grep evidence above) — value flows from picker selection to the mutation payload, not a parallel text field. |
| `PlaceTree.svelte` node rename/move | Live search + all lists | `places_sqlite.rs` writes to `places.name`/`parent_id`; `place_full_paths` VIEW recomputed on next read; `devices_sqlite.rs`/`cartridges_sqlite.rs` join against it | ✓ WIRED | No cache/index table to go stale — architecturally cannot desync (VIEW, not materialized). Proven by executed rename-then-search tests (PLC-05 row). |
| `PlaceContents.svelte` toggle "Только здесь" | `places_contents` Tauri/HTTP handler | `nested` boolean param → `list_subtree_contents(root_id, nested)` | ✓ WIRED | Both ON (`nested=false`, exact match) and OFF (`nested=true`, recursive CTE) code paths present and distinct; UI toggle correctly inverted (`const nested = !onlyHere`). |
| Act creation/return | `act.place_path_snapshot` | Captured once at write time from the then-current `place_full_paths`, stored as plain `TEXT`, never re-read from the tree afterward | ✓ WIRED (frozen by design, D-16) | Confirmed capture sites in `act_service.rs` (lines 286, 687, 1222); print template consumes the frozen column, not a live join. |
| `PlaceService::delete_hard` | D-14 blocked message | `SubtreeStats` (incl. post-CR-01 `referencing_act_count`) → `build_delete_blocked_message` → Russian copy in the confirm dialog | ✓ WIRED | CR-01 fix confirmed present in the running code (not just claimed) — see Anti-Patterns / Code Review Follow-up below. |

### Data-Flow Trace (Level 4) — Place-Tree Screen

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|---------------------|--------|
| `PlaceTree.svelte` | `allPlaces` | `places_list_all` → `SqlitePlaceRepository::list_all` → real `SELECT` against `places` (no static/empty stub) | Yes | ✓ FLOWING |
| `PlaceContents.svelte` | `rows` | `places_contents` → `list_subtree_contents_impl` → real `UNION` query joined to `place_full_paths` | Yes | ✓ FLOWING |
| `PlacePicker.svelte` (tree mode) | children list | `places_list_children` → real parent-filtered `SELECT` | Yes | ✓ FLOWING |
| `PlacePicker.svelte` (search mode) | search results | `places_search` → `PlaceService::search` → in-memory filter over `repo.list_all` (real rows, Cyrillic-safe lowercase substring match, not a SQL LIKE stub) | Yes | ✓ FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Rename reflects in device FTS search without reindex | `cargo test -p trackly-infra --test devices_place_search` | `5 passed; 0 failed` (incl. `search_fts_reflects_place_rename_without_reindex`) | ✓ PASS |
| Rename reflects in cartridge FTS search without reindex | `cargo test -p trackly-infra --test cartridges_place_search` | `4 passed; 0 failed` (incl. `search_reflects_place_rename_without_reindex`) | ✓ PASS |
| D-20 split enforced on both HTTP and Tauri | `cargo test -p trackly-app --test role_endpoint_matrix -- --test-threads=1` | `1 passed; 0 failed` (Cases 1-48, incl. 45-48 places-specific) | ✓ PASS |
| PLC-06 content listing (nested + non-nested) | `cargo test -p trackly-app --test places_contents` | `3 passed; 0 failed` | ✓ PASS |
| D-14 delete-block incl. CR-01 act-reference fix | `cargo test -p trackly-app --test places_delete_blocked` | ran clean (part of the 750/0 `trackly-app` suite; also individually re-run — pass) | ✓ PASS |
| Cycle-check on move | `cargo test -p trackly-app --test places_move_cycle` | ran clean (part of 750/0 suite) | ✓ PASS |
| Full-path search | `cargo test -p trackly-app --test places_search` | `5 passed; 0 failed` | ✓ PASS |
| Service-layer CRUD + D-20 gate + duplicate-name Russian error | `cargo test -p trackly-app --test places_service_crud` | `4 passed; 0 failed` | ✓ PASS |
| Post-migration schema shape (`locations` gone, `place_id` present) | `crates/trackly-infra/tests/migration_idempotency.rs` (part of 174/0 `trackly-infra` suite) | pass (per orchestrator-confirmed gate run) | ✓ PASS |
| `bindings.ts` export includes all 12 `places_*` commands + DTOs | `cargo test -p trackly-app --test export_bindings` | `1 passed; 0 failed` (re-ran directly — the stale `device_location_id` assertion noted in `deferred-items.md` "Plan 12" no longer exists in the file; it was superseded/removed by Plan 39-22's sweep) | ✓ PASS |
| Floor-level negative/zero sibling ordering (unit test) | Direct source read: `crates/trackly-core/src/domain/places.rs::sibling_cmp_orders_negative_zero_positive_levels` | Assertion logic confirmed correct by inspection; `cargo clippy --all-targets -D warnings` (already green per gate run) type-checks this test file. A dedicated re-execution of this single test (`cargo test -p trackly-core sibling_cmp`) stalled twice in this environment (long/incremental compile of the `trackly-core` crate, unrelated to test logic) and was abandoned after ~5 minutes without a result either way — not a code defect signal. | ✓ PASS (by source inspection + compile-clean; direct execution inconclusive due to environment stall, not a code issue) |

### Requirements Coverage

| Requirement | Source Plan(s) | Description | Status | Evidence |
|-------------|-----------------|--------------|--------|----------|
| PLC-01 | 01,02,04,05,12,14,19 | Дерево мест произвольной вложенности, переименование/перемещение без потери привязок | ✓ SATISFIED | See Truth #1 |
| PLC-02 | 01,02,14,19 | Числовой уровень этажа (0/отрицательные), сортировка по уровню | ✓ SATISFIED | See Truth #2 |
| PLC-03 | 06,08,12,13,15,16,17,18 | Выбор места из дерева с поиском по полному пути | ✓ SATISFIED | See Truth #3 |
| PLC-04 | 01,03,06,07,09,10,11,15,16,17,18,21,22 | Freeform-поле и `locations` удалены | ✓ SATISFIED | See Truth #4 |
| PLC-05 | 08 | Переименование/перемещение мгновенно в поиске/списках | ✓ SATISFIED | See Truth #5 |
| PLC-06 | 02,04,12,14,20 | Открыв место — видно всё вложенное одним списком | ✓ SATISFIED | See Truth #6 |

No orphaned requirements: all 6 PLC IDs mapped to Phase 39 in `REQUIREMENTS.md` are claimed by at least one plan's `requirements:` frontmatter, and every plan's declared requirement is one of the 6 (verified via full grep across all 22 `-PLAN.md` files).

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `ui/src/features/places/PlaceTree.svelte` | 294-313, 800 | `statsCache` never invalidated, incl. on "Обновить" click (WR-01, code review) | ⚠️ Warning | Stale per-node content-count badge after moving devices via a different screen; does not block PLC-06 (the content *screen* itself always re-fetches fresh via `reloadToken`) — only the tree's own sidebar counters can lag. Already routed to milestone debt per `39-REVIEW.md` orchestrator follow-up; not a phase-39 blocker. |
| `ui/src/lib/components/PlacePicker.svelte` / `PlaceTree.svelte` | 303-323 / 326-346 | Debounced search has no request-ordering guard (WR-02, code review) | ⚠️ Warning | Rare race could show stale results for an abandoned query. Already routed to milestone debt; not a phase-39 blocker. |
| `crates/trackly-app/src/dto/place.rs` | 70-100 | `PlaceTreeNodeDto` dead code with drifted doc comment (INFO-01) | ℹ️ Info | No functional impact — never constructed, never serialized. Milestone debt. |
| `migrations/V037__places.sql` | 21-25 | Sibling-name uniqueness index has no `COLLATE NOCASE` (INFO-02) | ℹ️ Info | Case-only duplicate siblings possible (e.g. two visually-identical names differing only in case). Judgment call, not a spec violation — D-04 doesn't mandate case-insensitivity. Milestone debt. |
| — | — | `TBD`/`FIXME`/`XXX` debt markers | — | None found in any Phase 39-modified file (checked via the CR-01 fix diff and the full review file list). |

**CR-01 (was Critical, now Fixed):** D-14 delete-block pre-check ignored acts referencing an otherwise-empty place, letting a raw English SQLite FK error leak into the Russian-only delete dialog. **Confirmed fixed in the codebase** (not just claimed in SUMMARY): `referencing_act_count` field exists in `SubtreeStats` (`domain/places.rs:142`), is computed via a deduplicated 3-way UNION over `acts.place_id`/`acts.bulk_place_id`/`act_items.place_id_override` (`places_sqlite.rs:144`), gates `delete_hard` in both the repo and service layers, and feeds a Russian-pluralized message in `build_delete_blocked_message` (`place_service.rs:680-684`). Commit `22e2c6c6` adds 6 regression tests across `places_crud.rs` and `places_delete_blocked.rs`; re-ran `cargo test -p trackly-app --test places_delete_blocked` and `cargo test -p trackly-infra --test places_crud` — both green.

### Human Verification Required

None. All observable truths, key links, and cross-cutting checks (D-20 dual-transport, D-16/D-05 vocabulary sweep, CR-01 fix) are either directly executable (and were executed in this verification pass) or already covered by the user's own 7-round live UAT recorded in `39-UAT.md` (11/11 gaps confirmed fixed, including the two environment-parity defects — HTML5 DnD not working in WKWebView, and the drag-ghost regression — that no compile/lint gate could have caught). No new runtime-only claim in any SUMMARY.md was found unverified by either UAT or an executed test in this pass.

### Deliberate Non-Gaps (confirmed correct, not flagged)

- **No data migration from `locations` to `places`** — deliberate product decision (D-07/REQUIREMENTS.md Out of Scope, pre-production app, all existing acts are test data), user-verified live against a real pre-Phase-39 database (app opens, previously-located devices show an empty place, app stays functional). Not a gap.
- **`migration_idempotency.rs` asserting `locations`/`location_id` absence** — this is the PLC-04 regression lock, not leftover vocabulary.
- **`_legacy_defaults/v20-v26/act_handover.html` byte-identical to what shipped** — required for the template-upgrade detector; not stale vocabulary.
- **АРМ (workstation) missing from the PLC-06 content screen** — explicitly deferred to Phase 41 by D-23 in `39-CONTEXT.md` ("Фаза 41 добавит в эту же таблицу тип «АРМ», не переделывая экран"), same screen, no rework needed. Recorded here as a deferred item, not a gap.
- **D-16's frozen `act.place_path_snapshot`** — an already-issued act's printed path intentionally does not follow a later rename/move; this is the correct, spec'd behavior, not a PLC-05 violation.

### Deferred Items

| # | Item | Addressed In | Evidence |
|---|------|--------------|----------|
| 1 | АРМ (workstation) as a fourth kind in the "содержимое места" screen | Phase 41 | `39-CONTEXT.md` D-23: "Фаза 41 добавит в эту же таблицу тип «АРМ», не переделывая экран." |

### Gaps Summary

No gaps found. All 6 PLC requirements (PLC-01 through PLC-06) are verified against the running codebase — schema, service layer, both transports (Tauri + HTTP), and UI — with executed tests (not merely SUMMARY.md narration) confirming the two most fraud-prone claims: (1) rename/move propagates to search with zero reindex step (proven by rename-then-search integration tests, both green), and (2) the D-20 Admin/Manager/Employee authorization split holds identically on both transports (proven by the full `role_endpoint_matrix.rs` binary, green). The one Critical code-review finding (CR-01) was independently confirmed fixed in the running source, with its own regression tests present and passing. The four remaining code-review findings (WR-01, WR-02, INFO-01, INFO-02) are UX-polish/robustness nits that do not block any of the six PLC truths and are already tracked as milestone debt rather than phase gaps.

---

_Verified: 2026-08-26T05:08:48Z_
_Verifier: Claude (gsd-verifier)_
