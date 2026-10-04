---
phase: 41-groups-model-and-editor
verified: 2026-10-04T16:30:00Z
status: human_needed
score: 8/8 success criteria verified at code level; 0/8 confirmed at runtime in the real application
overrides_applied: 0
re_verification: false
gaps: []
partial_requirements:
  - id: GRP-10
    status: partial
    reason: "Deliberate and recorded. Phase 41 delivers release-on-delete, unchanged devices.place_id, nested groups becoming roots, released count in the response. The half 'deleting an anchor device is forbidden while the group exists' needs groups.anchor_device_id, which V045 does not create (grep over crates/, migrations/, ui/src finds zero hits)."
    action_required: "ROADMAP.md Phase 41.1 lists only GRD-01..GRD-05 and none of its 6 success criteria mentions the deletion prohibition; .planning/phases/41.1-groups-devices-integration/ is empty. The remainder is carried only by REQUIREMENTS.md ([~], 'Phase 41 + 41.1') and STATE.md. Add GRP-10 to the 41.1 Requirements line and add a success criterion, or the remainder can be silently dropped."
warnings:
  - id: W-B01
    severity: warning
    where: "crates/trackly-app/src/services/group_type_service.rs (update_property ~582-590, unarchive_property 672-719)"
    what: "Un-hiding a property does not run the required-violators check, and update_property skips that check when the property is archived. Path: hide -> mark required -> un-hide. Afterwards every set_values on a group lacking that value fails (group_service.rs:1273 checks all live required properties), including edits to unrelated fields. Not data loss; the user is told which property to fill."
    confirmed_by: "read in this verification, not only taken from the review"
  - id: W-B02
    severity: warning
    what: "Type and property mutations are not atomic with their audit row (update_type writes on a bare conn, then opens a separate transaction for audit_log)."
  - id: W-B03
    severity: warning
    what: "Movements report keeps the pre-existing LIMIT 1000 ordered oldest-first (report_service.rs:1561). Large group moves can push newest rows out and split a batch; the user is not told the result is truncated. Journal data itself is correct."
  - id: W-F03
    severity: warning
    what: "GroupTypePanel copy 'Название и набор свойств — можно.' is shown to a read-only manager (also in deferred-items.md); needs a copy decision by the user."
human_verification:
  - test: "Open the 'Группы' section in the running desktop app (cargo tauri dev) AND in a LAN browser after pnpm --dir ui build. Walk the tree (arrows, Home/End, search), select a type and a group, create/rename/delete, move a group."
    expected: "Sidebar item between 'Устройства' and 'Акты'; tree roots are types; right panel switches between type panel and group panel with tabs Состав / Свойства / История; layout scrolls inside its own region with no double scroll; no console errors."
    why_human: "Component code is mounted and compile gates are green, but compile gates cannot see Svelte 5 rune runtime errors (effect_update_depth_exceeded), layout, focus rings, scroll regions or desktop-vs-browser asymmetry. The only runtime evidence is a Playwright WebKit smoke against a fake backend (plan 41-23), which by project rule is not verification; the app runs in WKWebView."
  - test: "Type property editor as admin: add a property of each data type, drag-reorder (and cancel with Esc), hide a filled property, try to change the data type of a filled property, enter bad IP/MAC. Repeat as manager (read-only)."
    expected: "Order persists after reload; Esc/pointercancel restores order with no write; filled property is hidden not deleted; data-type change refused with a message; IP/MAC normalised or rejected; manager sees no edit controls."
    why_human: "Drag gesture, hit-testing through button.drag-handle -> closest('tr') and modal flows are not observable by grep. Checks H20-1, H20-2, H20-3 in 41-VALIDATION.md."
  - test: "Group card: add two users and mark one primary; attach a USB-linked printer via SQL on a LOCAL dev DB plus the same printer as an explicit link."
    expected: "Users field and 'Подключённые принтеры' visible out of the box on a clean DB; exactly one primary; the printer appears once (deduplicated); USB origin is not editable by hand."
    why_human: "Data shape is verified by tests (groups_values_card, usb_printers_for_group CTE) but chips, primary toggle and printer list rendering are visual. Checks H22-1..H22-3."
  - test: "Group move end to end: create a group with a nested group and several devices (one member without a place), move it, then open device history, group history tab, Reports -> Перемещения (expand/collapse batch chevron, filter by device type) and print the report from desktop and from the LAN browser."
    expected: "Toast 'Перенесено: группа и N устройств'; place counters in 'Места' update; each device has a journal row; one batch row in the report expandable to member rows; device without a prior place gets the place with no journal row; the member's PlacePicker is disabled with a 'Место задаётся группой' link; print shows the full batch."
    why_human: "Service-level atomicity and batching are machine-verified; modal, toast, counters, report expansion and multi-page print are UI behaviour. Checks H23-1, H24-1, H24-2, H25-1, H25-2 and scenario S-Б."
  - test: "Role walk-through with two separate sessions: admin, manager, employee."
    expected: "Manager sees 'Группы', can create a group and edit composition, has no type-editing controls and no 'Создать тип'. Employee has no sidebar item and #/groups shows 'Доступ запрещён'."
    why_human: "Server-side matrix is machine-verified on both transports (role_endpoint_matrix, 11 tests green in this verification). Only the UI visibility layer needs eyes. Checks H20-3, H23-2."
  - test: "Decide the copy question for GroupTypePanel read-only text (W-F03) and the GroupAddDevicesModal 'уже в группе' hint styling (H21-3)."
    expected: "User decision recorded."
    why_human: "Product copy, not code correctness."
---

# Phase 41: Группы: модель и редактор — Verification Report

**Phase Goal:** Ввести универсальную группу устройств как единицу размещения поверх дерева мест — тип группы задаёт поведение и набор свойств, состав и место задаются в отдельном разделе «Группы», а устройства в составе следуют за местом группы с записью в историю по каждому.
**Verified:** 2026-10-04
**Status:** human_needed
**Re-verification:** No — initial verification

## Verdict in one paragraph

The backend and data model deliver the phase goal and I could not falsify any of the 8 roadmap success criteria at code level. I re-read the code behind each one, re-derived the `devices.place_id` write-site inventory from source instead of from the registry, and ran the key gates myself. What is NOT established is anything that only shows in the running application: the whole «Группы» UI was never run in WKWebView or a LAN browser by anyone. Those items are routed to a human, not scored as passed. One requirement (GRP-10) is deliberately partial and its remainder is not tracked by the Phase 41.1 roadmap entry. Four non-blocking warnings remain open.

## Goal Achievement

### Observable Truths (ROADMAP success criteria)

| # | Truth | Code-level status | Evidence |
|---|-------|-------------------|----------|
| 1 | Sidebar «Группы» between «Устройства» and «Акты»; tree (types as roots) left, panel right; old grouping renamed «Свернуть одинаковые» | VERIFIED (structure); layout/visual needs human | `ui/src/features/layout/sidebar-config.ts` order Устройства, Группы, Акты with `roles: ['admin','manager']`; `ui/src/routes.ts:24` `'/groups': GroupsPage`, absent from `employeeRoutes`; `GroupTree.svelte` builds `kind:'type'` and `kind:'group'` nodes; `DeviceFilters.svelte:102` label «Свернуть одинаковые»; no «Группир*» string left anywhere in `ui/src`; `check-group-vocabulary` OK (212 files) run in this verification |
| 2 | Three built-in types seeded idempotently by code; renameable, extendable, not deletable; `code`/`behavior` immutable; no duplicate after rename + rerun | VERIFIED | `group_types_sqlite.rs:169` `INSERT ... ON CONFLICT(code) DO NOTHING` (rename survives, id stable); `idx_group_types_code` UNIQUE in V045; DB trigger `trg_group_types_immutable` (BEFORE UPDATE OF code, behavior) in V045; service guard `group_type_service.rs:349-355` rejects changed code/behavior, `:398` rejects delete of `is_builtin`; default properties seeded once through marker `default_props_seeded` set by one conditional `UPDATE ... WHERE default_props_seeded = 0` and inserted only when exactly one row was affected (`group_types_sqlite.rs:209-250`); startup call in `context.rs:343`. Tests `seed_inserts_missing_and_is_idempotent`, `seed_does_not_overwrite_admin_rename`, `seed_rename_survives_restart_and_reseed`, `seed_props_*` exist. Note: «Системный блок» and «Разбор» get no default properties by design (empty list, marker stays 0) |
| 3 | Admin creates own type with property table (Текст, Число, IP, MAC, Пользователи с основным, Ссылки на устройства), order, required, «На карте»; filled property hidden not deleted; data type of filled property frozen; IP/MAC normalised and validated server-side | VERIFIED, with warning W-B01 | `PropertyDataType` has the six tokens; `group_type_properties` columns `sort_order`, `is_required`, `show_on_map`, `archived_at_utc` in V045; `delete_property` archives when `filled_group_count > 0`, hard-deletes otherwise (`group_type_service.rs:645-651`); data-type change refused when filled (`:567-580`); `normalize_ip` (IpAddr canonical form), `normalize_mac` (aa:bb:cc:dd:ee:ff, rejects mixed separators) in `trackly-core/src/domain/group_values.rs`, called from `group_service.rs:944` inside the server write path; primary-value uniqueness enforced by `uq_gpv_primary` and `primaries > 1` check; all type/property mutations gated by `Action::ManageGroupTypes`. WARNING W-B01: the required-flag protection can be bypassed through hide -> required -> un-hide (see Warnings) |
| 4 | Default name «{тип} #{seq}», `seq` from per-type counter, name editable, names not unique, number never parsed back from name | VERIFIED | `group_service.rs:426-427` `seq = next_seq_in_tx` (`SELECT COALESCE(MAX(seq),0)+1 FROM groups WHERE type_id`), name = `format!("{} #{}", gtype.name, seq)` only when no explicit name; `UNIQUE(type_id, seq)` is the race backstop, no unique index on `name`; rename never touches `seq` (`update_group` doc and code); grep finds no name-parsing path. Tests `numbering_*` in `groups_service` |
| 5 | Device in at most one group; explicit removal; nesting with cycle protection and child rules; nested group's place derived from root | VERIFIED | `group_devices.device_id` is PRIMARY KEY (V045); `add_device_in_tx` pre-check plus PK-race mapping to Conflict; `would_create_cycle_in_tx` recursive-CTE with UNION (loop-safe) and `parent_behavior_in_tx` rejecting a `teardown` parent (`groups_sqlite.rs:275-350`); `move_group_in_tx` rejects a nested group with the message naming its root (`group_place.rs`); `remove_devices` service method exists and is `MutateGroups`-gated. `groups_membership` 14 tests green in this verification. Child rule implemented = exactly what the SPEC states (only `teardown` cannot contain groups); brief default says containers may hold containers and anchors, which is not further restricted — consistent with SPEC acceptance text |
| 6 | Group place propagates to all devices including nested groups; each device gets a `place_id` change and a journal row; rows of one move share one batch; individual movement of a member forbidden | VERIFIED at service/data level; UI and print need human | `group_place.rs`: one `Transaction`, no commit inside; `propagate_group_place_in_tx` loops `subtree_device_ids_in_tx`, writes `place_movements` with a single `batch_id` UUID, `group_id` and `entity_label` snapshot on every row (V046 columns); a device with no previous place gets the place with no journal row and an `audit_log` row (D-30). **Write-site inventory re-derived from source:** the only code that writes `devices.place_id` is `devices_sqlite.rs` (update_in_tx, update_status_and_place_in_tx, update_full_in_tx, restore_from_snapshot_in_tx, unused non-tx `update`) and `cartridges_sqlite.rs:651`; callers are `device_service.rs:741` (S1, guarded at lines ~700-720 via `locked_group_for_device_in_tx`), `place_service.rs:763` (S2, group roots ride with the group), `group_place.rs:85`, 8 call sites in `act_service.rs` (596, 958, 1153, 1786, 2325, 2405, 2503, 3715 — S3-S7, each paired with a release), and the cartridge backfill (S9, only writes `WHERE place_id IS NULL`, so it cannot touch a locked member). No unregistered writer found. Gate `group_write_sites` 23/23, `groups_move` 10/10 (including `move_atomic_failure_on_fourth_device_rolls_back_everything` and `move_propagates_to_all_devices_including_nested`) green in this verification. Design note: a group WITHOUT a place does not lock its members (D-21, enforced in `locked_group_for_device_in_tx` by `g.place_id IS NOT NULL`); "forbidden" therefore holds from the moment the group has a place. This is a locked decision in CONTEXT, not a defect. WARNING W-B03 on the report cap |
| 7 | Group card shows composition, place, users (one primary), connected printers: USB derived from `printers.usb_host_device_id` and not hand-editable, network by explicit links; lists deduplicated | VERIFIED at data level; rendering needs human | `build_card` (`group_service.rs:~1050-1135`): USB printers from `usb_printers_for_group` (recursive subtree CTE over live hosts and live printers, `groups_sqlite.rs:541`), explicit links limited to live printers, merged with a `HashSet` on `device_id`, origin tagged `usb`/`link`; no write path exists for `usb_host_device_id` from groups; users come from `group_property_values` with `is_primary`. `GroupPanel`, `GroupPropertiesForm`, `GroupUsersField`, `GroupPrintersList` are imported and mounted (chain GroupsPage -> GroupPanel -> GroupPropertiesForm -> GroupUsersField/GroupPrintersList). `groups_values_card` 19 tests (run by the orchestrator) |
| 8 | Permissions on both transports: types/properties admin only; groups/composition/values admin+manager («Специалист» = existing `manager`, no 4th role); employee no access | VERIFIED | `auth.rs`: `ManageGroupTypes` -> `Role::Admin` only; `MutateGroups`/`ReadGroups` -> Admin or Manager; `enum Role` still has exactly Admin, Manager, Employee. Every public method of `GroupService` and `GroupTypeService` calls `authorize` with the right action (listing in this verification: 17 + 9 methods checked one by one; only the startup seeder is caller-less by design). HTTP: 25 routes in `http/groups.rs` + `http/group_types.rs`, all 25 command names used by `ui/src/lib/api/groups.ts` exist as routes and in the specta/Tauri builder. `role_endpoint_matrix` filtered to groups: 11/11 green in this verification (matrix for both `_http` and `_tauri_path`, immutability not vacuous, route completeness). UI layer: sidebar `roles`, and `/groups` missing from `employeeRoutes` |

**Score:** 8/8 at code level. 0/8 confirmed in the running application.

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `migrations/V045__groups.sql` | group_types, group_type_properties, groups, group_devices, group_property_values, immutability trigger | VERIFIED | additive only, no DROP, no rebuild; no `anchor_device_id` (see GRP-10) |
| `migrations/V046__place_movements_batch.sql` | `batch_id`, `entity_label`, `group_id` + partial indexes | VERIFIED | three `ALTER ... ADD COLUMN`, no rebuild |
| `crates/trackly-app/src/services/group_type_service.rs` | type/property service | VERIFIED | substantive, authorised, wired in `context.rs`, seeded at startup |
| `crates/trackly-app/src/services/group_service.rs` | group CRUD, membership, move, values, card | VERIFIED | substantive, wired |
| `crates/trackly-app/src/services/group_place.rs` | propagation primitives | VERIFIED | single transaction, batch id on every row |
| `crates/trackly-app/src/services/group_membership.rs` | release primitives for write sites | VERIFIED | used by act and device deletion paths |
| `crates/trackly-core/src/domain/group_values.rs` | value normalisation | VERIFIED | IP/MAC/number/text |
| `ui/src/features/groups/*.svelte` (16 files) | the section | WIRED, runtime unverified | all components imported; chain from `routes.ts` -> `GroupsPage` confirmed |
| `ui/src/lib/api/groups.ts` | one call point for 25 commands | VERIFIED | names match HTTP routes and Tauri registration |
| `crates/trackly-app/tests/group_write_sites.rs` | closed write-site gate | VERIFIED | 23 tests green; contains a source scan that fails on an unregistered writer and a pairing check for the 8 act sites |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `routes.ts` | `GroupsPage` | `'/groups'` route | WIRED | admin/manager route table only |
| `GroupsPage` | all group modals/panels | imports | WIRED | 7 direct, 7 transitive |
| UI components | backend | `apiCall` names | WIRED | 25/25 names match route + Tauri registration |
| `DeviceService::update` | group lock | `locked_group_for_device_in_tx` | WIRED | rejects a changed `place_id` for a member of a placed group; same-place resend passes |
| `ActService` 8 sites | release primitives | `release_device_in_tx` / `release_if_locked_device_in_tx` | WIRED | pairing enforced by gate test |
| `PlaceService` bulk move | group roots | `devices_riding_with_group` | WIRED | group members are not moved individually |
| `AppCtx::build` | built-in seeding | `seed_builtin_types_on_startup` | WIRED | `context.rs:343` |

### Data-Flow Trace (Level 4)

| Artifact | Data | Source | Real data | Status |
|----------|------|--------|-----------|--------|
| Group card printers | `printers` | `usb_printers_for_group` CTE + `ref_devices_for_group` | Yes (SQL over live rows) | FLOWING |
| Group tree | groups/types | `groups_list`, `group_types_list` | Yes | FLOWING (backend); rendering unverified |
| Movement timeline/report | `batch_id`, `group_id`, `entity_label` | `place_movements` V046 columns | Yes | FLOWING (data); UI unverified |

### Behavioral Spot-Checks (run in this verification)

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Write-site gate, membership, group move | `cargo test -p trackly-app --test group_write_sites --test groups_membership --test groups_move` | 23 + 14 + 10 passed, 0 failed | PASS |
| Role matrix on both transports | `cargo test -p trackly-app --test role_endpoint_matrix group` | 11 passed, 0 failed | PASS |
| Vocabulary gate | `node ui/scripts/check-group-vocabulary.mjs` | OK, 212 files, no violations | PASS |
| Section gate | `node ui/scripts/check-groups-section.mjs` | PASS, 0 violations | PASS |
| Reorder gate | `node ui/scripts/check-reorder.mjs` | OK, 22 cases | PASS |
| Debt markers in 84 non-test source files changed by the phase | grep TBD/FIXME/XXX/TODO/HACK/todo!/unimplemented! | none | PASS |

I did not re-run the full workspace (the orchestrator did: 156 binaries, 1604 tests, 0 failures) and did not re-run `svelte-check`/`lint`/build. Those results are taken from the orchestrator and from `41-VALIDATION.md` and are not independently confirmed here.

### Probe Execution

Step 7c: SKIPPED — no `scripts/*/tests/probe-*.sh` declared by any Phase 41 plan.

### Requirements Coverage

All 11 phase IDs appear in plan frontmatter and in REQUIREMENTS.md; no requirement is mapped to Phase 41 without a claiming plan (no orphans).

| Requirement | Claimed by | Status | Evidence |
|-------------|-----------|--------|----------|
| GRP-01 | 41-01, 02, 04, 07, 09, 19, 26 | SATISFIED | SC2: trigger + service guard + idempotent seed |
| GRP-02 | 41-01, 02, 04, 07, 09, 18, 20, 26 | SATISFIED (UI gesture needs human) | six data types, order, required, «На карте» |
| GRP-03 | 41-01, 02, 04, 07, 09, 12, 20, 22, 26 | SATISFIED with W-B01 | hide/freeze/normalise hold; required-flag protection has a bypass |
| GRP-04 | 41-06, 08, 13, 18, 19, 21, 23, 26 | SATISFIED (UI needs human) | SC1 + SC4 |
| GRP-05 | 41-01, 06, 08, 10, 11, 13, 14, 21, 26 | SATISFIED | SC5 |
| GRP-06 | 41-01, 05, 06, 10, 11, 13, 15, 17, 21, 23, 24, 25, 26 | SATISFIED at service level (UI/print need human) | SC6 |
| GRP-07 | 41-10, 14, 15, 16, 24, 25, 26 | SATISFIED | S1 guard + closed registry; conditional on the group having a place (D-21) |
| GRP-08 | 41-06, 08, 12, 13, 22, 26 | SATISFIED (rendering needs human) | SC7 |
| GRP-09 | 41-02, 07, 09, 12, 13, 23, 26 | SATISFIED | SC8 |
| GRP-10 | 41-01, 06, 11, 15 claim it complete; 41-08 states partial | **PARTIAL (deliberate)** | see below |
| GRD-06 | 41-03, 26 | SATISFIED | «Свернуть одинаковые» + vocabulary gate |

**GRP-10 reading, confirmed.** Early plans 41-01, 41-06, 41-11 and 41-15 list GRP-10 under `requirements-completed`, but only 41-08 owns the delete behaviour and it correctly records `requirements-partial: [GRP-10]`. In code: `delete_group` (`group_service.rs:503`) releases direct devices (cascade on `group_devices`), leaves `devices.place_id` untouched, nested groups become roots through `ON DELETE SET NULL` while keeping their denormalised place, returns `released_devices` for the confirmation dialog, and writes an audit row. The anchor half is genuinely absent: V045 has no `anchor_device_id`, and nothing in `crates/`, `migrations/` or `ui/src` mentions it. I found nothing else in the phase that was quietly left partial: every other ID resolved to concrete code above. The one thing I flag beyond the already-known deferral is that the remainder is not on the Phase 41.1 roadmap entry (Requirements line `GRD-01..GRD-05`, 6 success criteria, none about it), so the deferral is not yet enforceable by a later verifier. I did not treat it as "deferred" under the later-phase filter because no later-phase text matches it.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `group_type_service.rs` | 582-590, 672-719 | un-hide / archived-required skip the violators check | Warning (W-B01) | group edits can start failing until the new required value is filled |
| `group_type_service.rs` | update_type / property writes | audit row in a separate transaction | Warning (W-B02) | change can persist while the caller sees an error |
| `report_service.rs` | 1561 | `LIMIT 1000` oldest-first with batch grouping | Warning (W-B03) | a batch can be split and truncation is silent |
| `GroupTypePanel.svelte` | read-only text | promises a permission the manager lacks | Info/copy (W-F03) | misleading wording |

No blocker patterns: no debt markers, no stubs, no empty handlers found in the phase's non-test source files.

### Human Verification Required

See the `human_verification` list in the frontmatter. In short: everything about how the «Группы» section actually runs and looks in Tauri WKWebView and in a LAN browser. `41-VALIDATION.md` carries the full list of 17 live checks (H19-1 … H25-2) plus two end-to-end scenarios (S-А drag of a property, S-Б move-to-print) — none of them has been run by anyone. Plan 41-23's Playwright WebKit smoke (about 12 flows, no console errors, no `effect_update_depth_exceeded`) against a fake backend is a useful signal and is NOT counted as verification.

Run on both transports. Fixes in a spawned worktree are not in a running `cargo tauri dev` until merged to main. For the LAN browser run `pnpm --dir ui build` first. Use a local dev DB, not production data.

### Gaps Summary

There are no goal-blocking gaps: all 8 success criteria hold in the code I read and the tests I ran. The status is `human_needed` because the UI runtime is unverified by design, not because something is known broken.

Items to action regardless of the UAT outcome:

1. **GRP-10 remainder is orphaned from the roadmap.** Add `GRP-10` to Phase 41.1's Requirements and a success criterion «удаление устройства-якоря запрещено, пока существует группа» (needs `groups.anchor_device_id` in a 41.1 migration).
2. **W-B01** — run `groups_missing_required` in `unarchive_property` and in `update_property` regardless of archived state (fix is small and local to `group_type_service.rs`).
3. **W-B02 / W-B03 / W-F03** — non-blocking; decide whether to schedule or accept.

The two gate defects found by the code review (INV-7 aliased import, vocabulary-gate prefix leak) were fixed in commit 751d0762 and I did not re-prove them by mutation; I relied on the orchestrator's statement that they were proven before and after.

---

_Verified: 2026-10-04_
_Verifier: Claude (gsd-verifier)_
