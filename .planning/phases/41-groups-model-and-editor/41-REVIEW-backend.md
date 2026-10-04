---
scope: backend
phase: 41
depth: standard
files_reviewed: 66
critical: 0
warning: 3
info: 6
status: findings
reviewed: 2026-10-04
---

# Phase 41 backend review (Rust + SQL migrations)

All 66 files in `/tmp/review-be.txt` were read or scanned. I read the services, repositories, adapters, DTOs, domain code and both migrations in full. Large pre-existing files (`act_service.rs`, `cartridges_sqlite.rs`, `place_service.rs`, `device_service.rs`, `report_service.rs`) were read around the phase-41 edits. I skimmed the test files. I did not run `cargo`.

The phase diff base (`199438c9^`) spans about 2200 commits, so `git diff` shows many pre-phase-41 changes. Only group, membership, batch and movement-journal changes were reviewed.

## Verdict on the ten invariants

| # | Invariant | Result |
|---|-----------|--------|
| 1 | One transaction per group move | Holds. `group_place.rs` and `group_membership.rs` contain no `commit()`. `move_group`, `set_parent`, `add_devices` and `remove_devices` each open one `conn.transaction()` and commit once. |
| 2 | D-21/D-19 guard in `DeviceService::update` | Holds. `device_service.rs:729` compares `domain_patch.place_id` (an `Option<Option<i64>>`) with the current place inside the writer closure. Clearing the place is rejected, repeating the place passes, and a group with no place does not lock because `locked_group_for_device_in_tx` filters on `g.place_id IS NOT NULL`. |
| 3 | Write-site completeness | Holds. `grep UPDATE devices` finds only `devices_sqlite.rs` and the S9 cartridge backfill. All 8 `act_service.rs` sites are paired correctly: direct paths (create 596, update-added 958, do_return 1786, update_return added 2405 and edited 2503) use `release_device_in_tx`. Restore paths (1153, 2325, undo 3715) use `release_if_locked_device_in_tx`. `DeviceService::delete_soft` releases. S9 only fires `WHERE place_id IS NULL`, so it cannot touch the place of a member of a group that has a place. |
| 4 | Type immutability | Holds. The V045 trigger is `UPDATE OF code, behavior` with a `WHEN` clause, so renames and the `default_props_seeded` UPDATE pass. The service rejects a differing `code`/`behavior`. `delete_type` rejects built-in types and types that still have groups. |
| 5 | Filled-property protection | Holds, with one gap (WR-01). |
| 6 | Permissions on both transports | Holds. Every `http/groups.rs` and `http/group_types.rs` handler calls the shared `build_*` function. `build_*` and the service both call `authorize`, with `ManageGroupTypes` Admin-only, `MutateGroups`/`ReadGroups` Admin or Manager, and Employee excluded. No route bypasses the gate. |
| 7 | Optimistic locking | Holds. `set_values` checks the version before validating values and bumps it by 1. `reorder_properties` bumps only `updated_at_utc`. |
| 8 | Server-side normalization | Holds. `normalize_scalar` is the single entry point and the service stores only its output. |
| 9 | Cycle and nesting safety | Holds. Order in `set_parent_in_tx` is cycle check, then teardown rule, then CAS UPDATE. A nested group's place is denormalized from the root via `set_subtree_place_in_tx`, and `move_group_in_tx` rejects nested groups. |
| 10 | Migrations additive only | Holds. V045 creates tables, indexes and one trigger. V046 is three `ALTER TABLE ... ADD COLUMN` plus partial indexes. Neither rebuilds nor drops a table, so the V042 cascade wipe cannot recur. There is no `CHECK` on `entity_type` or `source` in V040, so `'group'` rows insert fine. |

No real organization or personal data was found. Names in tests are fictional (Иванов И.И., Петров П.П., Сидоров С.С., Кузнецов К.К.). The INN, OGRN, phone and e-mail values in `html_header_parity.rs` and `html_report_render.rs` are clearly synthetic placeholders (sequential digits, reserved-style phone, `test-org` domain).

There is no `unwrap()`, `expect()` or `panic!` on a request path in the new group code. The one `expect` in `place_movements_sqlite.rs:233` is pre-existing and guarded by `is_reportable_place_change`.

## Critical

None.

## Warnings

### WR-01: Un-hiding a property, or editing a hidden property, bypasses the "required" invariant

**File:** `crates/trackly-app/src/services/group_type_service.rs:582-590` and `:672-719`

**Defect:** Turning on "required" is validated only when `current.archived_at_utc.is_none()`. `unarchive_property` never calls `groups_missing_required`, and it keeps whatever `is_required` the property had.

**Scenario:**
1. Property P is required and filled in groups A and B, then hidden (`delete_property` archives it because it is filled).
2. An admin creates group C. `set_values` ignores P because `list_properties(.., false)` skips archived properties, so C has no value for P.
3. The admin un-hides P. Now P is required and live, but C violates it.
4. Every later `set_values` on C fails with «Заполните обязательное свойство», even when the user only edits an unrelated field.
5. The same state is reachable by `update_property(is_required = true)` on a hidden property, which skips the check by design.

This violates the rule that required is rejected when live groups would violate it. `empty_groups_for_property` exists to pre-check this, but the un-hide path never uses it.

**Fix:** In `unarchive_property`, after the live-name check, run `groups_missing_required` when `current.is_required` is true and return `required_violation_error(&violators)` if the list is non-empty. Alternatively, reset `is_required` to false when un-hiding with violators.

### WR-02: Group-type and property mutations are not atomic with their audit row, and `create_property` fakes a rollback

**File:** `crates/trackly-app/src/services/group_type_service.rs:253-291`, `:345-377`, `:396-435`, `:459-526`, `:563-625`, `:642-668`, `:737-761`; repository side `group_types_sqlite.rs:295-312`, `:437-470`, `:571-615`

**Defect:** Each mutation runs as an autocommit statement on the writer connection (`repo.create_type(conn, ..)`, `repo.update_property(conn, ..)` and so on). The audit row is then written in a separate `conn.transaction()`. This breaks the "one transaction per logical operation" rule that `GroupService` follows consistently.

**Scenarios:**
- If the audit insert or a following read (`build_type_dto`, `to_json`) fails after the mutation committed, the caller gets an error but the change persists. A retry then duplicates it: `create_type` makes a second type, and `create_property` fails on the name check.
- `create_property` with `is_required = true` inserts the property, runs the violators check, and on failure calls `delete_property_hard` as a manual compensation. A failure of the following `update_property(conn, id, 1, ..)` leaves an unrequested non-required property behind while returning `Err`. A crash between the steps leaves the same state.

**Fix:** Wrap each closure body in one `conn.transaction()`, pass `&Transaction` to the repository calls (they already deref to `Connection`), and commit once after the audit insert. For `create_property`, run `groups_missing_required` before the INSERT by passing the would-be type id, or insert with `is_required = ?` inside the transaction and let the error drop the transaction instead of calling `delete_property_hard`.

### WR-03: Movements report silently truncates group batches and under-reports `total`

**File:** `crates/trackly-app/src/services/report_service.rs:1546-1563` (`LIMIT 1000`) and `:1692` (`total = rows.len()`)

**Defect:** The query orders `created_at_utc ASC` and cuts at 1000 rows. Phase 41 makes one group move produce up to 1 header row, plus 1 row per device (up to `MAX_DEVICES_PER_BATCH` = 500 per add, more per move), plus cartridge rows. `batch_size` is computed by an uncapped subquery, but `total` reports the capped row count.

**Scenario:** Two or three large AРМ moves in the selected period exceed 1000 rows. The newest rows are dropped, so a batch can show its header with only some members, while `batch_size` still says "N devices". The printable report is supposed to show the full composition (D-27) and nothing tells the user it was cut.

**Fix:** Return the true total with a separate `COUNT(*)` and a `truncated` flag, or paginate by batch so a batch is never split across the limit. At minimum, make the cutoff visible in the DTO.

## Info

### IN-01: `update_return` "retained with change" releases group membership even when the place does not change

**File:** `crates/trackly-app/src/services/act_service.rs:2503-2521`

An edit that only changes a return act's condition keeps the place (`location.or(before.place_id)`). It still calls the unconditional `release_device_in_tx`. If the device was added to a group with no place after the return, a condition-only edit silently ejects it. This is acknowledged in `41-16-SUMMARY.md` and deferred to phase 41.2. To confirm it matters, add a test where a device in a placeless group has its return act's condition edited, then assert membership is lost. Suggested narrowing: use `release_if_locked_device_in_tx` there, or release only when `after.place_id != before.place_id`.

### IN-02: "Required" is satisfied by dead references

**File:** `crates/trackly-infra/src/repos/groups_sqlite.rs:841-857`, `group_types_sqlite.rs:530-569`

`property_filled_in_tx`, `filled_group_count` and `groups_missing_required` count any `value_ref IS NOT NULL` row. `build_card` hides refs to inactive users and soft-deleted devices. A required `users` property whose only user was deactivated counts as filled, but the card shows it empty. The conservative count is right for "filled, so hide instead of delete" in `filled_group_count`. It is arguably wrong for the required check. To confirm, deactivate the sole user of a required users property and call `set_values`; it will pass.

### IN-03: Placing into an archived place is not checked on two paths

**File:** `crates/trackly-app/src/services/group_service.rs:685-703` (`add_devices`) and `:809-824` (`set_parent`)

`create_group` and `move_group_in_tx` reject an archived target place. `add_devices` into a group whose place has since been archived, and `set_parent` under such a parent, push devices into the archived place without a check. Mitigation: skip the propagation, or validate `places.get(..).archived_at_utc` once per call.

### IN-04: `delete_group` has no version check

**File:** `group_service.rs:503-545`, `groups_sqlite.rs:176-187`

Every other group mutation except add/remove devices is a CAS. The delete confirmation shows `direct_device_count`. If a colleague adds devices after the dialog opened, the group is deleted and more devices are released than the user confirmed. Consider taking `version` and using `resolve_group_cas_failure`.

### IN-05: `set_values` can return an error after the change is committed

**File:** `group_service.rs:1298-1299`

`tx.commit()` is followed by `build_card(conn, ..)` on the writer connection. If the read fails, the caller gets `Err` for a change that is already persisted and the version has moved. This is very unlikely, and returning a card read on a reader after the closure would remove it.

### IN-06: Small quality items

- `plural_groups` and `in_groups` (`group_type_service.rs:77-102`) re-implement `plural::ru_plural`, which the phase added for exactly this.
- `normalize_number` (`group_values.rs:80-90`) accepts `1e5`, `.5`, `5.` and `007` and stores them verbatim. Only comma to dot and the leading `+` are normalized, so equal numbers can be stored in different spellings. If numbers will be compared or sorted as text, canonicalize.
- `create_type` does not check name uniqueness among types, so two types named «АРМ» produce indistinguishable default group names (`АРМ #3`).
- `GroupValueInputDto.refs` has no `#[serde(default)]` (`dto/groups.rs`), so a client that omits it for a text property gets a deserialization error rather than a validation message.
- `group_write_sites.rs` gate 1 pairs write sites and releases by a 14-line text window. It is fragile against formatting changes, though intentionally strict.
