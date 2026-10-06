---
phase: 41
slug: groups-model-and-editor
status: verified
threats_open: 0
asvs_level: 1
created: 2026-10-06
---

# Phase 41 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.
> Phase 41 (universal groups model + group editor) spans 34 plans; every plan carried a
> plan-time `<threat_model>` block, so this audit **verified the declared mitigations**
> rather than building a register retroactively.

**Result: 167 threats, 167 CLOSED, 0 OPEN.** 137 `mitigate` dispositions were traced to a
control in the implemented code; 30 `accept` dispositions are recorded in the Accepted Risks
Log below.

## Audit method

Four independent auditors each took one slice of the register and were held to a standard
deliberately stricter than the executors' own closure claims, which were treated as
**unverified self-reports**:

- an `authorize(Action::…)` call had to be *present in the named method*, with the Action
  matching the one the register named (not a neighbouring Action reused);
- a DoS ceiling had to be *enforced at a call site*, not merely declared as a constant;
- an inventory claim ("release on all 8 sites") had to be *recounted* independently;
- a test counted as evidence only if it would *fail with the control removed* — vacuous
  fixtures, non-unique mutation anchors and source-grepping gates were downgraded to
  weak evidence;
- `{@html}` absence was verified by grep, not by assertion.

The full test suite was **not** re-run (~80 min, single-run constraint); verification is by
source reading plus the recorded runs in `41-VALIDATION.md`. No implementation file was
modified during the audit.

| Slice | Plans | Threats | Result |
|-------|-------|---------|--------|
| A | 41-01 … 41-09 | 43 | 43 CLOSED (38 mitigate + 5 accept) |
| B | 41-10 … 41-17 | 46 | 46 CLOSED (39 mitigate + 7 accept) |
| C | 41-18 … 41-26 | 46 | 46 CLOSED (38 mitigate + 8 accept) |
| D | 41-27 … 41-34 | 32 | 32 CLOSED (22 mitigate + 10 accept) |

---

## Trust Boundaries

The 34 plan-time boundary tables consolidate into these classes. Paths are relative to the
repository root.

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| LAN browser → axum | Untrusted client with a cookie session POSTs `/api/v1/groups_*` and `/api/v1/group_types_*` (25 group commands + 10 type commands) | Group/type names, ids, property values (ip/mac/number/text), device and user id batches |
| webview → Tauri invoke | Local command invocation; same `build_*` entry points as HTTP | Identical payloads — schemas defined once, used by both transports |
| transport → service | `build_*` is the shared point; `authorize` is duplicated here and in the service (defence in depth) | Caller identity (role, user_id) + DTO |
| role → action | `authorize()` is the single decision point for both transports | Role, Action |
| service → repository | Parameters originate from an untrusted client and were validated upstream | Validated names, ids, normalised values |
| repository → SQLite | The only place that composes SQL for groups, types and properties | Bound parameters only (`rusqlite::params!`) |
| SQL layer → service | DB constraints are the last rail when the service is wrong | Trigger / PK / UNIQUE verdicts |
| writer transaction → audit_log | Trace of schema and composition changes is part of the audit journal | `user_id`, action, before/after JSON, batch_id |
| migration → user's live DB | DDL runs on a live organisation DB in a portable build | Schema changes (V045 additive, V046 ADD COLUMN only) |
| server → UI | Group/type/property names and path snapshots are displayed | Text rendered through Svelte interpolation (escaped by default) |
| journal → report → reader/print | Group and device name snapshots are emitted to HTML/CSV for the archive | Server-rendered, autoescaped HTML; `csv_safe` for CSV |
| `groups.userOptions` → UI | User picker list | **Only** `id`, `full_name`, `login` — no email, role or hash |
| UI role gate → server | `{#if canEdit}` / `{#if isAdmin}` is UX only; it is **not** a security control | — (the boundary is the server `authorize`) |
| URL hash and localStorage → page | Ids from the hash/store are untrusted; requests still pass authorization | Node keys, selected tab — no personal data |
| server response (`total`, `rows`) → banner | UI must not compute truncation from its own row count | Server-computed `total` |
| working tree → public repository | Everything committed stays in git history; the only control is the pre-commit check | Code, fixtures, planning artifacts |

---

## Threat Register

167 threats. Status `CLOSED` throughout; qualifiers in parentheses point at the
*Residual items* section. Evidence is quoted as recorded by the auditor who read it.

| Threat ID | Category | Component | Disposition | Status | Evidence (path:line) |
|-----------|----------|-----------|-------------|--------|----------------------|
| T-41-01-01 | Tampering | V045 trg_group_types_immutable | mitigate | CLOSED | `migrations/V045__groups.sql:47-52` (`BEFORE UPDATE OF code, behavior` + `RAISE(ABORT)`); tests `crates/trackly-infra/tests/groups_migration.rs:193-263` (abort + row re-read unchanged; same-value UPDATE must not fire), `crates/trackly-infra/tests/group_types_repo.rs:156-213` |
| T-41-01-02 | Tampering | V045/V046 on a live DB | mitigate | CLOSED | `migrations/V045__groups.sql:26-122` — CREATE only, no DROP/rebuild; `migrations/V046__place_movements_batch.sql:16-23` — three `ALTER TABLE ... ADD COLUMN` + 2 indexes only; test `crates/trackly-infra/tests/groups_migration.rs:120-190` runs the REAL runner `run_up_to(44)` → seed → `run`, compares `act_items` and `place_movements` row tuples before/after (`:152-157`), asserts `pragma_foreign_key_check` = 0 (`:159-160`) and no `place_movements_new` leftovers (`:161-168`) |
| T-41-01-03 | Tampering | group_devices | mitigate | CLOSED | `migrations/V045__groups.sql:93` `device_id INTEGER PRIMARY KEY`; tests `crates/trackly-infra/tests/groups_migration.rs:280-294`, `crates/trackly-infra/tests/groups_repo.rs:186-189` (direct INSERT bypassing the service is rejected by the PK) |
| T-41-01-04 | Denial of Service | seq race | mitigate | CLOSED | `migrations/V045__groups.sql:87` `CREATE UNIQUE INDEX idx_groups_type_seq ON groups(type_id, seq)`; writer-side `MAX(seq)+1` at `crates/trackly-app/src/services/group_service.rs:429` (`repo.next_seq_in_tx` inside the writer transaction); tests `crates/trackly-infra/tests/groups_migration.rs:275-278`, `crates/trackly-app/tests/groups_service.rs:588-607` (concurrent creates → {1,2}) |
| T-41-01-05 | Information Disclosure | test data | mitigate | CLOSED | Fixtures use only the project's standard fictional surnames — `crates/trackly-infra/tests/groups_migration.rs:124,134`; privacy gate is fail-closed in `.githooks/pre-commit:25,31` (`check-privacy.mjs --staged --hashes`) and in CI as the first step, `.github/workflows/ci-fast.yml:31` |
| T-41-02-01 | Elevation of Privilege | Action::ManageGroupTypes | mitigate | CLOSED | Distinct variant `crates/trackly-core/src/auth.rs:129`; Admin-only bucket `crates/trackly-core/src/auth.rs:168-174` (grouped with ManageUsers/ManageSettings/MutatePlaces, `matches!(role, Role::Admin)`); `MutatePlaces` is **not** reused for groups — separate `MutateGroups`/`ReadGroups` at `:132,134` in the Admin|Manager bucket `:184-186`; matrix test `crates/trackly-core/src/auth.rs:444-458` (manager + employee → Forbidden) |
| T-41-02-02 | Tampering | group_values::normalize_* | mitigate | CLOSED | Pure server-side normalisation, no I/O: `crates/trackly-core/src/domain/group_values.rs:44` (ip), `:52` (mac), `:80` (number, `> NUMBER_MAX_CHARS` → reject at `:82`), `:93` (text, `> TEXT_MAX_CHARS` → reject at `:98`), dispatcher `:106-124` rejects Users/DeviceRefs as scalars. Limits: `crates/trackly-core/src/domain/groups.rs:13` (TEXT 2000), `:15` (NUMBER 32). Actually called at `crates/trackly-app/src/services/group_service.rs:945` (`prepare_value`, reached from `set_values` `:1247`) |
| T-41-02-03 | Denial of Service | batch inputs | mitigate | CLOSED | Constants `crates/trackly-core/src/domain/groups.rs:17` (MAX_REFS_PER_PROPERTY=100), `:19` (MAX_DEVICES_PER_BATCH=500); exported and **enforced**: `crates/trackly-app/src/services/group_service.rs:355-362` (for_devices), `:607-614` (`normalize_device_ids`, called by add_devices `:643` and remove_devices `:742`), `:964-968` (refs per property). Pinned by `crates/trackly-core/src/domain/groups.rs:318-321` |
| T-41-02-04 | Information Disclosure | journal tokens | accept | CLOSED (accepted) | Rationale verbatim below; `MovementSource::Group` is a compile-time enum token, not user data (used at `crates/trackly-app/src/services/group_place.rs:99,275`) |
| T-41-03-01 | Tampering | vocabulary gate | mitigate | CLOSED | `ui/scripts/check-group-vocabulary.mjs` — shared scanner `scanSource()` at `:127`, allowlist with narrow per-file markers at `:62-92`, deny-list of the OLD concept evaluated BEFORE the allowlist and before `markers: null` (`:143-146`); `runSelfTest()` at `:205` runs the SAME `scanSource` over 22 fixtures with both positives (`expectViolations: 0`) and negatives (`expectViolations: 1`) — e.g. `:212-237` encode three holes a reviewer proved on a mutated copy — and exits non-zero on mismatch `:387-388`. Wired in `ui/package.json:16`: `check-group-vocabulary.mjs --selftest && check-group-vocabulary.mjs` |
| T-41-03-02 | Information Disclosure | selftest fixtures | mitigate | CLOSED | Fixture `src` strings are synthetic UI labels / identifiers only — `ui/scripts/check-group-vocabulary.mjs:206-371`; no names, no org data. Privacy gate as T-41-01-05 (`.githooks/pre-commit:31`, `.github/workflows/ci-fast.yml:31`) |
| T-41-04-01 | Tampering | SQL over names/ids | mitigate | CLOSED | Every user value bound: `crates/trackly-infra/src/repos/group_types_sqlite.rs:184-193` (seed), `:299-320` (update_type), `:548-551` (get_by_code), `:601-606` (list_properties), `:508-513` (reorder UPDATE). The only `format!` uses in SQL interpolate compile-time consts `SELECT_TYPES`/`SELECT_PROPERTIES` (`:61,125,526,548,596`) — never user input |
| T-41-04-02 | Tampering | code/behavior change | mitigate | CLOSED | No repository method can change them: `GroupTypePatch` carries only name/sort_order/quick_action_* — `crates/trackly-core/src/domain/groups.rs:146-152`; `update_type_on` SQL touches only those columns — `crates/trackly-infra/src/repos/group_types_sqlite.rs:301-308`. DB trigger as second rail (V045:47). Test asserts the row is unchanged after the aborted direct UPDATE: `crates/trackly-infra/tests/group_types_repo.rs:167-198` |
| T-41-04-03 | Tampering | seed overwrites a rename | mitigate | CLOSED | `ON CONFLICT(code) DO NOTHING` — `crates/trackly-infra/src/repos/group_types_sqlite.rs:183`. Non-vacuous test: admin renames to a value that **differs** from the seed name, re-seed returns 0 inserted, name survives, id unchanged, exactly one row — `crates/trackly-infra/tests/group_types_repo.rs:302-354` |
| T-41-04-04 | Tampering | reorder with foreign ids | mitigate | CLOSED | Set comparison before any write: `crates/trackly-infra/src/repos/group_types_sqlite.rs:483-512` (live ids SELECTed for the type, both vectors sorted, `live != requested` → `Validation`, so a foreign id, a duplicate or an omission all fail). Single transaction: trait impl opens it — `:701-712` |
| T-41-04-05 | Denial of Service | reorder of a large list | accept | CLOSED (accepted) | Rationale verbatim below; the service ceiling it refers to does exist: `MAX_PROPERTIES_PER_TYPE = 50` at `crates/trackly-app/src/services/group_type_service.rs:93`, enforced `:472-478` |
| T-41-05-01 | Elevation of Privilege | place_movements_get_timeline(entity_type='group') | mitigate | CLOSED | `authorize(caller, &Action::ReadPlaces)?` is the **first statement**: `crates/trackly-app/src/services/place_movement_service.rs:56` (signature ends `:55`). Test: employee → `AppError::Forbidden` for `entity_type = "group"` — `crates/trackly-app/tests/place_movements_group_fields.rs:320-328` (manager succeeds first at `:311-318`, so the assertion is not vacuous) |
| T-41-05-02 | Tampering | journal SQL | mitigate | CLOSED | `crates/trackly-infra/src/repos/place_movements_sqlite.rs:91-114` — the single `insert_in_tx`, 15 bound `?N` params, no interpolation; actor lookup also bound `:241-249`. Group writes never call it directly — they go through `record_batch_movement_if_applicable` (`:175`) |
| T-41-05-03 | Repudiation | batch journal | mitigate | CLOSED | Shared helper resolves `actor_name_snapshot` and forwards `user_id` for **every** row: `crates/trackly-infra/src/repos/place_movements_sqlite.rs:241-269`. Both group write sites use it — `crates/trackly-app/src/services/group_place.rs:93-107` (per device, with batch_id/entity_label/group_id) and `:271-286` (the group row itself) |
| T-41-05-04 | Information Disclosure | entity_label | accept | CLOSED (accepted) | Rationale verbatim below; `entity_label` is populated from the group name snapshot only — `crates/trackly-app/src/services/group_place.rs:284` (`Some(&group.name)`) |
| T-41-06-01 | Tampering | group nesting cycle | mitigate | CLOSED | `would_create_cycle_in_tx` — recursive `ancestors` CTE with `UNION` (not `UNION ALL`, so pre-corrupted data cannot loop) at `crates/trackly-infra/src/repos/groups_sqlite.rs:278-300`; called **before** the UPDATE inside the same transaction at `:334-339`, UPDATE only at `:348-353`. Test asserts `parent_group_id` is unchanged after refusal — `crates/trackly-infra/tests/groups_repo.rs:244-296` |
| T-41-06-02 | Tampering | membership in two groups | mitigate | CLOSED | Pre-check → `Conflict` at `crates/trackly-infra/src/repos/groups_sqlite.rs:199-206`, PK violation mapped to `Conflict` at `:211-217`, PK itself at `migrations/V045__groups.sql:93`. Test `crates/trackly-infra/tests/groups_repo.rs:164-195` covers both the service path and a direct INSERT bypassing the pre-check |
| T-41-06-03 | Tampering | SQL injection | mitigate | CLOSED | `rusqlite::params!` / `params_from_iter` throughout `crates/trackly-infra/src/repos/groups_sqlite.rs`; the only `format!` calls (`:201,214`) build **error messages**, not SQL. Variable-length `IN (...)` lists are built from `vec!["?"; ids.len()]` placeholders with `params_from_iter` — `:730-750` (`existing_ids`), `:819-834` (`live_users_by_ids`); heads are consts `:753-755` |
| T-41-06-04 | Information Disclosure | printers/devices query | mitigate | CLOSED | `deleted_at_utc IS NULL` on every device join/filter: `crates/trackly-infra/src/repos/groups_sqlite.rs:412,552,556,586,617,643,678`, `:755` (LIVE_DEVICES_HEAD), `:805-806` (live printer devices) |
| T-41-06-05 | Denial of Service | tree_counts over 5000 devices | accept | CLOSED (accepted) | Rationale verbatim below; the single recursive `closure` query is at `crates/trackly-infra/src/repos/groups_sqlite.rs:608-620`, and the indexes it names exist — `migrations/V045__groups.sql:88` (`idx_groups_parent`), `:98` (`idx_group_devices_group`) |
| T-41-07-01 | Elevation of Privilege | type/property mutations | mitigate | CLOSED | `authorize(caller, &Action::ManageGroupTypes)?` as first statement in all 8 mutations: `crates/trackly-app/src/services/group_type_service.rs:252,314,399,456,549,649,692,751`. Tests: types `crates/trackly-app/tests/groups_types_service.rs:692-724` and properties `:1535-1585` — manager and employee → `Forbidden` on every mutation; manager allowed on the read `empty_groups_for_property`, employee denied |
| T-41-07-02 | Tampering | code/behavior change via DTO | mitigate | CLOSED | DTO carries the fields; service compares against the current row and refuses any difference — `crates/trackly-app/src/services/group_type_service.rs:351-367`. DB trigger is the second rail (V045:47). Non-vacuous tests send a **differing** value and then re-read the row: `crates/trackly-app/tests/groups_types_service.rs:482-516` (code), `:519-544` (behavior), plus `:546-568` proving same-value + new name still succeeds |
| T-41-07-03 | Tampering | seed overwrites types | mitigate | CLOSED | Same `ON CONFLICT(code) DO NOTHING` (`crates/trackly-infra/src/repos/group_types_sqlite.rs:183`) called from the startup seed `crates/trackly-app/src/services/group_type_service.rs:821-871`; one-shot property seed guarded by the `default_props_seeded` marker inside one transaction (`crates/trackly-infra/src/repos/group_types_sqlite.rs:209+`). Restart/rename test: `crates/trackly-infra/tests/group_types_repo.rs:302-354` |
| T-41-07-04 | Tampering | value loss when editing the schema | mitigate | CLOSED | Deleting a filled property archives instead of dropping — `crates/trackly-app/src/services/group_type_service.rs:660-666` (`filled_group_count > 0 → archive_property_on`); changing `data_type` of a filled property is refused — `:575-591`. Test with a direct `SELECT COUNT(*) FROM group_property_values` proving values survive: `crates/trackly-app/tests/groups_types_service.rs:947-972`; data_type refusal `:1010-1075`, and `:1077-1109` shows the same type is not treated as a change and an empty property can be retyped (so the refusal is specific, not blanket) |
| T-41-07-05 | Denial of Service | mass properties | mitigate | CLOSED | `MAX_PROPERTIES_PER_TYPE = 50` at `crates/trackly-app/src/services/group_type_service.rs:93`, enforced on create at `:472-478` (counts archived too, `include_archived = true`); name length via `validate_name` (`NAME_MAX_CHARS = 200`, `crates/trackly-core/src/domain/groups.rs:11,259-263`) applied at `group_type_service.rs:253,318,457,553` |
| T-41-07-06 | Repudiation | admin schema edits | mitigate | CLOSED | `audit_repo.insert` in the same transaction as every mutation: `crates/trackly-app/src/services/group_type_service.rs:286` (type create), `:371` (type update), `:429` (type delete), `:520` (property create), `:623` (property update), `:666` (property delete/archive — action is `archive` or `delete`), `:725` (unarchive), `:769` (reorder_properties); each carries `user_id` |
| T-41-08-01 | Elevation of Privilege | group CRUD | mitigate | CLOSED | `authorize(...)` is the first statement of all 15 public methods: `crates/trackly-app/src/services/group_service.rs:204,219,237,291,354` (ReadGroups) and `:394,466,508,557,642,741,794,1211` (MutateGroups), `:1153,1189` (ReadGroups). Tests: reads `crates/trackly-app/tests/groups_service.rs:432-462`, writes `:920-941`. See *Weak evidence 3* for test-coverage scope |
| T-41-08-02 | Tampering | name / seq | mitigate | CLOSED | `seq` computed server-side inside the writer transaction — `crates/trackly-app/src/services/group_service.rs:429` (`repo.next_seq_in_tx(&tx, type_id)`), client cannot supply it (`GroupCreateDto` has no seq; see `:409-411` for the fields actually read); `UNIQUE(type_id, seq)` backstop `migrations/V045__groups.sql:87`; name trimmed and capped by `validate_name` at `:397-401`. Tests `crates/trackly-app/tests/groups_service.rs:493-541` (rename does not shift the counter), `:588-607` (concurrent → distinct seq), `:651-657` (failed creates do not burn a seq) |
| T-41-08-03 | Tampering | place_id on create | mitigate | CLOSED | Existence + non-archived checked server-side inside the transaction before insert — `crates/trackly-app/src/services/group_service.rs:413-423` (`NotFound → place_error("Место не найдено.")`, `archived_at_utc.is_some() → place_error("Место в архиве...")`). Test rejects both a non-existent id and an archived place: `crates/trackly-app/tests/groups_service.rs:696-705` |
| T-41-08-04 | Information Disclosure | groups_for_devices / groups_search | mitigate | CLOSED | Both gated `ReadGroups` = Admin\|Manager (`crates/trackly-app/src/services/group_service.rs:291,354`; employee excluded by `crates/trackly-core/src/auth.rs:184-186`); ceilings: 500 ids `:355-362`, 50 results `:53` + enforced `:338-340`, query length 100 chars `:51,292`. Response DTO carries id/name/type_name/device_count/has_parent only (`:330-336`) — no personal data |
| T-41-08-05 | Repudiation | group mutations | mitigate | CLOSED | `audit_repo.insert` inside the same `tx` with `user_id = caller.user_id`, committed together: `crates/trackly-app/src/services/group_service.rs:439-450` (create), `:481-493` (update), `:523-537` (delete), plus `:712` (add_devices), `:847` (set_parent), `:1283-1296` (set_values) |
| T-41-08-06 | Denial of Service | batch ids | mitigate | CLOSED | `MAX_DEVICES_PER_BATCH` (domain const, `crates/trackly-core/src/domain/groups.rs:19`) enforced in `for_devices` at `crates/trackly-app/src/services/group_service.rs:355-362` and in `normalize_device_ids` `:607-614`, which add_devices (`:643`) and remove_devices (`:742`) call immediately after `authorize` |
| T-41-09-01 | Elevation of Privilege | HTTP route open while the Tauri command is closed (and vice versa) | mitigate | CLOSED | Both transports are thin adapters over the same `build_*`, which carries its own `authorize` (defence in depth): `crates/trackly-app/src/tauri_cmds/group_types.rs:35,45,58,68,78,90,102,112,123,135`; HTTP handlers import and call exactly those — `crates/trackly-app/src/http/group_types.rs:20-26`, routes `:259-286`. Matrix 3 roles × 2 transports: HTTP `crates/trackly-app/tests/role_endpoint_matrix.rs:3494-3551` (also asserts 401 without a session, and that Admin is **not** 401/403/415/422 so the denial assertions cannot be vacuous), direct `build_*` `:3560-3589` via `gt_call_build` `:3292-3360` |
| T-41-09-02 | Elevation of Privilege | manager mutates a type/property | mitigate | CLOSED | `GateKind::TypeMut` rows → `StatusCode::FORBIDDEN` for the manager cookie — `crates/trackly-app/tests/role_endpoint_matrix.rs:3526-3531`, and `Err(AppError::Forbidden)` on the direct path `:3571-3575`; the driver table `group_type_cases()` `:3229-3289` marks 8 of 10 commands as `TypeMut` |
| T-41-09-03 | Tampering | code/behavior change over HTTP | mitigate | CLOSED | `crates/trackly-app/tests/role_endpoint_matrix.rs:3596-3694`: explicitly asserts the payload differs from the DB (`assert_ne!` at `:3640,3658`), expects 400, and re-reads code/behavior/version from the DB comparing the whole tuple (`:3650-3655,3666-3671`); the rename control case returns 200 and leaves code/behavior intact (`:3674-3692`) |
| T-41-09-04 | Tampering | unregistered route bypasses the test table | mitigate | CLOSED | `group_types_http_route_completeness` — `crates/trackly-app/tests/role_endpoint_matrix.rs:3885-3904`: `include_str!("../src/http/group_types.rs")`, regex over `"/api/v1/(\w+)"`, asserts the set difference is empty in **both** directions and `in_source.len() == 10`. See *Weak evidence 2* |
| T-41-09-05 | Information Disclosure | employee reads types | mitigate | CLOSED | Reads gated `ReadGroups` (`crates/trackly-app/src/tauri_cmds/group_types.rs:35,135`), which excludes employee (`crates/trackly-core/src/auth.rs:184-186`). Test: employee → 403 on all 10 commands including the two reads — `crates/trackly-app/tests/role_endpoint_matrix.rs:3545-3550`, and `Err(Forbidden)` on the direct path `:3583-3587` |
| T-41-09-06 | Denial of Service | unauthorised mass calls | accept | CLOSED (accepted) | Rationale verbatim below. **The rationale is partly inaccurate** — see *Weak evidence 1*. The half that holds: a session is mandatory, `session_identity()` is called by every group-type handler and no-session requests get 401, asserted at `crates/trackly-app/tests/role_endpoint_matrix.rs:3498-3503` |
| T-41-10-01 | Elevation of Privilege | groups_move | mitigate | CLOSED | `crates/trackly-app/src/services/group_service.rs:557` — `authorize(caller, &Action::MutateGroups)?` first statement of `move_group`; second gate `crates/trackly-app/src/tauri_cmds/groups.rs:175`; policy `crates/trackly-core/src/auth.rs:184-186` (Admin\|Manager only); test `crates/trackly-app/tests/groups_move.rs:596` (`move_rights_employee_forbidden_manager_allowed`) |
| T-41-10-02 | Tampering | partial move on failure | mitigate | CLOSED | single `conn.transaction()` `group_service.rs:565`, commit `:575`; movements+audit written inside `group_place.rs:204-236`; fault-injection test with SQLite `RAISE(ABORT)` trigger on the 4th device asserting rollback of groups/devices/place_movements/audit_log: `crates/trackly-app/tests/groups_move.rs:549-581` |
| T-41-10-03 | Tampering | nested group moved bypassing root | mitigate | CLOSED | `crates/trackly-app/src/services/group_place.rs:165-176` — `if group.parent_group_id.is_some()` → `AppError::Validation{field:"place_id"}`; denormalised nested place written only via `propagate_group_place_in_tx` (`group_place.rs:251`); HTTP test `crates/trackly-app/tests/role_endpoint_matrix.rs:4561-4588` (400, place unchanged) |
| T-41-10-04 | Tampering | target place missing/archived | mitigate | CLOSED | `group_place.rs:181-201` — `deps.places.get(tx, target_place_id)` inside the same tx, `NotFound` → «Место не найдено», `archived_at_utc.is_some()` → «Место в архиве» |
| T-41-10-05 | Repudiation | move with no trace | mitigate | CLOSED | batch movements `group_place.rs:271-286` and `:91-104` (`batch_id`, `entity_label`, `user_id`); group-level audit `group_place.rs:224-236` (`action:"move"`, payload with `batch_id`, `from`, `to`, `user_id`) — all on the same `&Transaction` committed at `group_service.rs:575` |
| T-41-10-06 | Denial of Service | very large group composition | accept | CLOSED (accepted) | rationale in accepted-risks section |
| T-41-10-07 | Information Disclosure | summary/audit | accept | CLOSED (accepted) | rationale in accepted-risks section; spot-check: audit payloads carry only ids/counters (`group_place.rs:218-223`, `group_service.rs:708-715`), no personal names |
| T-41-11-01 | Elevation of Privilege | composition mutations | mitigate | CLOSED | `group_service.rs:642` (add_devices), `:741` (remove_devices), `:794` (set_parent) — `authorize(&Action::MutateGroups)` first statement; transport gates `tauri_cmds/groups.rs:151,163,139`; tests `crates/trackly-app/tests/groups_membership.rs:724` and `:1004` |
| T-41-11-02 | Tampering | nesting cycle | mitigate | CLOSED | recursive-CTE ancestors check `crates/trackly-infra/src/repos/groups_sqlite.rs:275-300` called from `set_parent_in_tx` `:336-341` inside the tx; test asserts `parent_group_id` unchanged and version unchanged `crates/trackly-app/tests/groups_membership.rs:788-813` |
| T-41-11-03 | Tampering | nesting under a teardown group | mitigate | CLOSED | `groups_sqlite.rs:342-347` — `parent_behavior_in_tx(tx,p) == "teardown"` → Validation; test `groups_membership.rs:817-827`; HTTP test `role_endpoint_matrix.rs:4538-4559` |
| T-41-11-04 | Tampering | device in two groups | mitigate | CLOSED | service check `group_service.rs:670-678` (`group_of_device_in_tx`, whole batch aborted); DB backstop `migrations/V045__groups.sql:92-93` (`group_devices.device_id INTEGER PRIMARY KEY`); test `groups_membership.rs:444` |
| T-41-11-05 | Denial of Service | device_ids batch | mitigate | CLOSED | constant `crates/trackly-core/src/domain/groups.rs:19` (`MAX_DEVICES_PER_BATCH = 500`); enforced in `normalize_device_ids` `group_service.rs:593-620` (dedup via `HashSet`, empty rejected, cap) and called at `:643` and `:742`; also enforced on `for_devices` read `:355`; test `groups_membership.rs:579` |
| T-41-11-06 | Tampering | denormalised place drift | mitigate | CLOSED | single write path `propagate_group_place_in_tx` `group_place.rs:251-328` (used by `move_group_in_tx` `:204`, `set_parent` `group_service.rs:812`, bulk move `place_service.rs:840`); whole-table invariant walk (every live group → root, place equality + cycle check) `crates/trackly-app/tests/groups_membership.rs:205-243`, exercised after 5 distinct mutations in `:1046-1110` |
| T-41-11-07 | Repudiation | composition changes | mitigate | CLOSED | `add_devices` audit `group_service.rs:706-722`; `set_parent` audit `:845-860`; `custom:group_member_released` with `{group_id, act_id}` `crates/trackly-app/src/services/group_membership.rs:33-47`; all with `user_id` and inside the same tx; test `groups_membership.rs:621` |
| T-41-12-01 | Tampering | ip/mac/number values | mitigate | CLOSED | pure-domain normalisation `crates/trackly-core/src/domain/group_values.rs:9-120` called transport-independently from the service `group_service.rs:945-948`; writes fully parameterised `crates/trackly-infra/src/repos/groups_sqlite.rs:503-524` (`rusqlite::params!`, no concatenated user input); HTTP test `role_endpoint_matrix.rs:4344-4433` |
| T-41-12-02 | Tampering | refs to dead/foreign users and devices | mitigate | CLOSED | liveness in the same tx `group_service.rs:1246-1266` (`live_user_ids_in_tx`/`live_device_ids_in_tx`, `groups_sqlite.rs:780-795` + `existing_ids` `:730-750` placeholder-bound); read-side live filter in `build_card` `group_service.rs:1029-1045` |
| T-41-12-03 | Information Disclosure | groups_user_options | mitigate | CLOSED | DTO has exactly 3 fields `crates/trackly-app/src/dto/groups.rs:204-211`; SQL selects only `id, full_name, login` `groups_sqlite.rs:861-878`; gate `ReadGroups` `group_service.rs:1153`; JSON key-set test `crates/trackly-app/tests/groups_values_card.rs:913-921` and over HTTP `role_endpoint_matrix.rs:4276-4300`; `users_list` still manager-forbidden `role_endpoint_matrix.rs:4303-4311` |
| T-41-12-04 | Elevation of Privilege | employee reads card/values | mitigate | CLOSED | `authorize(&Action::ReadGroups)` `group_service.rs:1189` (card), `:1153` (user_options); `MutateGroups` `:1211` (set_values); policy excludes Employee `crates/trackly-core/src/auth.rs:184-186` with test `:460-474`; Forbidden tests `groups_values_card.rs:900-905` and the 15-command matrix `role_endpoint_matrix.rs:4197-4203` |
| T-41-12-05 | Denial of Service | huge ref lists / text | mitigate | CLOSED | constants `crates/trackly-core/src/domain/groups.rs:13,15,17` (2000/32/100) + `group_service.rs:898` (`USER_OPTIONS_LIMIT = 50`); enforced: refs cap `group_service.rs:964-969`, text/number caps `group_values.rs:31,82,98`, options cap `group_service.rs:1176` (`.take(USER_OPTIONS_LIMIT)`), query length `:1154-1161`; tests `groups_values_card.rs:885-890`, `:910` |
| T-41-12-06 | Tampering | two concurrent editors | mitigate | CLOSED | CAS on group version `group_service.rs:1222-1229` + `bump_group_version_in_tx` `:1273` (`groups_sqlite.rs:759`); atomic per-property replacement inside the same tx `groups_sqlite.rs:492-535` (DELETE+INSERT under a SAVEPOINT); tests prefixed `values_cas_` `crates/trackly-app/tests/groups_values_card.rs` |
| T-41-13-01 | Elevation of Privilege | employee calls groups_* via HTTP or build_* | mitigate | CLOSED | 15 `build_groups_*` each with `authorize` as first statement `crates/trackly-app/src/tauri_cmds/groups.rs:36-187`; HTTP handlers call only those wrappers (20 `build_groups_` references, zero direct service/repo access in `crates/trackly-app/src/http/groups.rs`); matrix 3 roles x 2 transports `crates/trackly-app/tests/role_endpoint_matrix.rs:4167-4206` (HTTP) and `:4213-4231` (direct build_*); non-vacuous: employee must get exactly 403, so an SPA-fallback 200 for a missing route fails the test |
| T-41-13-02 | Elevation of Privilege | manager creates/edits a type via groups | mitigate | CLOSED | `ManageGroupTypes` is Admin-only `crates/trackly-core/src/auth.rs:172-173` with test `:445-458`; cross-test manager: group 200 / type 403 with DB counts both ways plus an admin non-vacuity control `role_endpoint_matrix.rs:4237-4274` |
| T-41-13-03 | Tampering | invalid ip/mac/number over HTTP | mitigate | CLOSED | server-side normalisation in the shared service (see T-41-12-01); HTTP test asserts 400, row count unchanged, group version unchanged, then a valid value passing through the same route, plus 3 MAC spellings normalising to one form in DB and in the card `role_endpoint_matrix.rs:4319-4433` |
| T-41-13-04 | Tampering | cycle/teardown/nested group over HTTP | mitigate | CLOSED | rules live in repo/service (T-41-11-02/03, T-41-10-03); HTTP tests 400 + DB unchanged `role_endpoint_matrix.rs:4444-4610` (busy device, self-nesting, teardown parent, nested `groups_move`) |
| T-41-13-05 | Information Disclosure | groups_user_options leaks extra fields | mitigate | CLOSED | same evidence as T-41-12-03; HTTP key-set assertion tagged `T-41-13-05` at `role_endpoint_matrix.rs:4288-4299` |
| T-41-13-06 | Tampering | unregistered route outside the matrix | mitigate | CLOSED | `groups_http_route_completeness` `role_endpoint_matrix.rs:4620-4640`: regex over `src/http/groups.rs`, two-way set difference against the driver table, plus `assert_eq!(in_source.len(), 15)`; the table itself drives real HTTP calls in Case 81 |
| T-41-14-01 | Tampering | bypass via devices_update | mitigate | CLOSED | guard in the service, shared by both transports: `crates/trackly-app/src/services/device_service.rs:727-739` (`locked_group_for_device_in_tx` → Validation on `place_id`); both transports funnel through `build_devices_update` (`crates/trackly-app/src/http/devices.rs:188`, `crates/trackly-app/src/tauri_cmds/devices.rs:211`); tests `crates/trackly-app/tests/group_write_sites.rs:815` (HTTP), `:944` (Tauri path), `:359` (9-row service table) |
| T-41-14-02 | Tampering | race "added to group" ‖ "place changed" | mitigate | CLOSED | guard reads membership inside the same writer closure that performs the UPDATE: `device_service.rs:706` (`conn.transaction()`), guard `:729`, write `:741`, commit at the end of the same closure; serialisation by the single writer; PK backstop `migrations/V045__groups.sql:93` |
| T-41-14-03 | Tampering | dead members after device deletion | mitigate | CLOSED | `device_service.rs:854-862` — `release_device_in_tx` inside `delete_soft`'s transaction; tests `group_write_sites.rs:495` (service), `:982` (HTTP), `:545` (no-membership branch writes nothing) |
| T-41-14-04 | Denial of Service | false rejection when saving a member | mitigate | CLOSED | comparison against the current value `device_service.rs:727-728` (`if new_place != before_place_id`); scenario table diverges by construction — `PlacePatch::Current` + rename → Ok vs `PlacePatch::Other` → Reject, including the case where the device's place differs from the group's `group_write_sites.rs:241-358` |
| T-41-14-05 | Elevation of Privilege | manager/employee via devices_update | accept | CLOSED (accepted) | rationale in accepted-risks section; spot-check: `MutateDevices` unchanged and the guard applies to all roles — test `group_write_sites.rs:462` (`s1_table_manager_is_also_guarded`) |
| T-41-15-01 | Tampering | bypassing the ban via bulk move | mitigate | CLOSED | `crates/trackly-app/src/services/place_service.rs:724-756` builds `devices_riding_with_group` / `group_roots` from `group_of_device_in_tx` + `root_group_id_in_tx`; such devices skipped in the single loop `:751-756`; whole groups moved via `move_group_in_tx` `:839-849`; tests `crates/trackly-app/tests/places_move_groups.rs:239`, `:338`, `:377` |
| T-41-15-02 | Tampering | partial move on failure | mitigate | CLOSED | one `conn.transaction()` `place_service.rs:706`, commit `:851`; fault-injection test (ABORT trigger on a group member after single devices were already updated) asserting rollback of devices, group place, movements and audit `places_move_groups.rs:528-583` |
| T-41-15-03 | Repudiation | duplicate journal rows | mitigate | CLOSED | group devices excluded from the single-device loop `place_service.rs:751-756` (explicitly to avoid two rows: one `manual`, one `group`); COUNT-per-`entity_id` assertions `places_move_groups.rs:365-366` (exactly 1 row per device) and source-mix assertions `:310-318` |
| T-41-15-04 | Denial of Service | deleting a place with groups fails with a raw FK error | mitigate | CLOSED | `referencing_group_count` in the stats SQL `crates/trackly-infra/src/repos/places_sqlite.rs:226`, folded into the pre-check total `place_service.rs:485-494`, Russian message with plural `:993-1003`; test asserts Conflict, no `FOREIGN KEY` substring, «1 группа», prefix «Место нельзя удалить:» `crates/trackly-app/tests/places_delete_blocked.rs:755-790` |
| T-41-15-05 | Elevation of Privilege | bulk-move rights | accept | CLOSED (accepted) | rationale in accepted-risks section; spot-check: `authorize(MutateDevices)` + `authorize(MutateCartridges)` unchanged at `place_service.rs:697-698` |
| T-41-16-01 | Tampering | bypassing the ban by moving through an act | mitigate | CLOSED | release present at all 8 `devices.place_id` write sites of `crates/trackly-app/src/services/act_service.rs`: `:604`, `:966`, `:1155`, `:1795`, `:2327`, `:2414`, `:2512`, `:3717` — each on the same `&tx` and each passing `Some(act_id)`; count independently verified by grep (8 call sites, no more, no fewer); scenarios S3-S7 `crates/trackly-app/tests/group_write_sites.rs:1239,1279,1319,1346,1375,1402,1426,1453,1481,1509,1550` |
| T-41-16-02 | Tampering | future write site without release | mitigate | CLOSED | counting gate `group_write_sites.rs:1687-1737`: exactly 8 write-site calls and 8 release calls in `act_service.rs`, plus per-site pairing in a 14-line window with the correct helper kind (`release_if_locked_` for restore paths, `release_` otherwise, each forbidding the other), `assert_eq!(paired, 8)`; repo-wide scan gate `:1747-1780` with registry `:1608-1612` and an anti-vacuity assertion `sources.len() > 50`; counter self-test `:1658-1680`. See Weak evidence. |
| T-41-16-03 | Tampering | restore overwrites "place belongs to the group" | mitigate | CLOSED | `release_if_locked_device_in_tx` `crates/trackly-app/src/services/group_membership.rs:57-73` (checks `locked_group_for_device_in_tx` first) wired to all three restore paths `act_service.rs:1155`, `:2327`, `:3717`; pairing gate enforces the helper kind per path; tests S4b `group_write_sites.rs:1319`, `:1346` (dormant group keeps membership), S6c `:1453`, S7 `:1481`, `:1550` |
| T-41-16-04 | Repudiation | device left the composition with no trace | mitigate | CLOSED | `group_membership.rs:32-47` — `custom:group_member_released` with payload `{group_id, act_id}` and `user_id`, inserted on the same `&Transaction` as the act write; `Some(act_id)` passed at all 8 sites (verified individually); payload assertion helper `group_write_sites.rs:1198-1218` |
| T-41-16-05 | Denial of Service | release failure breaks the act | accept | CLOSED (accepted) | rationale in accepted-risks section |
| T-41-17-01 | Tampering (Injection/XSS) | HTML injection via group name in print | mitigate | CLOSED | report rendered through the autoescaping environment: `crates/trackly-app/src/services/report_service.rs:1144` passes `build_safe_html_env()`, defined `crates/trackly-app/src/pdf/minijinja_env.rs:118-127` (`AutoEscape::Html`, strict undefined, no loader); cells interpolated plainly `crates/trackly-app/templates/report.html:148-152` (`<td>{{ cell }}</td>`), no `\| safe` anywhere in `report.html`; the only `\| safe` uses are the org logo data-URI and server-escaped org full name `templates/_header.html:92,105`; no `{@html}` anywhere in `ui/src` (single match is a comment in `ui/src/lib/components/MovementTimeline.svelte:161`); name length bounded `crates/trackly-core/src/domain/groups.rs:11` (`NAME_MAX_CHARS = 200`) |
| T-41-17-02 | Repudiation | batch without full composition in the archive | mitigate | CLOSED | print path test: 7 data rows, group row present, 6 member rows `crates/trackly-app/tests/group_report_batch.rs:513-542`; CSV path test: header set unchanged, 7 data lines, 1 `;Группа;` row, each of 6 devices once `:546-584`; template-ignorance gate `:586-591` |
| T-41-17-03 | Information Disclosure | batch fields in ReportRow | accept | CLOSED (accepted) | rationale in accepted-risks section; spot-check: `batch_id` is a v4 UUID `crates/trackly-app/src/services/group_place.rs:203`, `batch_label`/`entity_label` is the group-name snapshot `report_service.rs:1695-1699` |
| T-41-17-04 | Tampering | report SQL | mitigate | CLOSED | new query parts carry no user input: `report_service.rs:1578-1604` (literal `batch_id`, `entity_label`, `'Группа'` label, correlated `batch_size` subquery); every filter bound as a positional parameter `:1512-1565` (`?N` + `owned_params`), executed via `query_map(param_refs)` `:1607-1609`; only the row cap `MOVEMENTS_REPORT_LIMIT` is interpolated and it is a crate constant |
| T-41-17-05 | Elevation of Privilege | report access | accept | CLOSED (accepted) | rationale in accepted-risks section |
| T-41-18-01 | Elevation of Privilege | сокрытие кнопок в UI | accept | CLOSED | accepted-risks log below; server boundary independently confirmed: `crates/trackly-core/src/auth.rs:172-186` |
| T-41-18-02 | Tampering | рассинхрон имён команд | mitigate | CLOSED | `ui/src/lib/api/groups.ts:42-109` — 25 `apiCall<…>('cmd')`; all 25 names present in `ui/src/bindings.ts` (verified name-by-name) |
| T-41-18-03 | Tampering | регресс чистых функций порядка | mitigate | CLOSED | `ui/scripts/check-reorder.mjs:198-219` executes the real `ui/src/lib/utils/reorder.ts` against the fixture; mutant selftest `:148-199` with unique-anchor assertion `:181-190`; wired into `ui/package.json:16` |
| T-41-18-04 | Information Disclosure | фикстуры | mitigate | CLOSED | `ui/scripts/fixtures/reorder/cases.json` — 22 cases, only integers and single-letter placeholders (`a`..`d`); no personal/org data |
| T-41-19-01 | Elevation of Privilege | скрытие кнопок типов для manager | accept | CLOSED | accepted-risks log; UI gate at `ui/src/features/groups/GroupTreeNode.svelte:111` (`showTypeMenu = kind==='type' && canEditTypes`), server `auth.rs:172-174` |
| T-41-19-02 | Tampering | смена code/behavior через форму | mitigate | CLOSED | `ui/src/features/groups/GroupTypeFormModal.svelte:106-111` (rename DTO = `{name}` only); server rejects divergence `crates/trackly-app/src/services/group_type_service.rs:359-367` |
| T-41-19-03 | Injection (XSS) | имя группы/типа в дереве | mitigate | CLOSED | `{@html}` count in `ui/src/features/groups/` = 0 (grep); search highlight built by `String.slice` `GroupTreeNode.svelte:99-110` and rendered as interpolation `:183-184` |
| T-41-19-04 | Denial of Service | перезагрузка дерева по placeContentEventsStore | mitigate | CLOSED (weak) | `ui/src/features/groups/GroupTree.svelte:383-389` — effect reads `refreshToken`/`placeContentEventsStore.seq`, `loadAll()` inside `untrack`; runtime only via manual UAT console instruction (`41-HUMAN-UAT.md:22`) |
| T-41-19-05 | Information Disclosure | localStorage | accept | CLOSED | accepted-risks log; key `trackly:groups:expanded` stores only node keys `GroupTree.svelte:92-100,234` |
| T-41-20-01 | Elevation of Privilege | manager видит контролы правки типа | mitigate | CLOSED (weak: UI gate is UX only) | UI: `ui/src/features/groups/GroupsPage.svelte:332` `canEdit={isAdmin}` → `GroupTypePropertiesTable.svelte:521,534,551,570,581`. Real boundary: `ManageGroupTypes` admin-only `auth.rs:172-174`, enforced at `group_type_service.rs:252,314,399,456,549,649,692,751`; test `auth.rs:450-456` |
| T-41-20-02 | Tampering | потеря значений при смене data_type / удалении | mitigate | CLOSED | UI disable: `GroupTypePropertiesTable.svelte:551-557` (typePicker `disabled` = `filled_group_count > 0`); server rejects type change when filled `group_type_service.rs:578-591`; delete archives instead `:660-662,680` |
| T-41-20-03 | Tampering | потеря порядка при сбое перестановки | mitigate | CLOSED | `GroupTypePropertiesTable.svelte:212-235` — single atomic `reorderProperties` `:224`, optimistic `optimisticIds` `:220`, rollback in `finally` `:233`, error Toast `:226-229` |
| T-41-20-04 | Injection (XSS) | имена свойств/групп | mitigate | CLOSED | no `{@html}` in `ui/src/features/groups/` (grep 0); `{p.name}` interpolation `GroupTypePropertiesTable.svelte:546` |
| T-41-20-05 | Denial of Service | pointer-обработчики и эффекты | mitigate | CLOSED (weak) | no `$effect` at all in `GroupTypePropertiesTable.svelte`; state written from pointer handlers `:486-489`; sibling panel writes inside `untrack` `GroupTypePanel.svelte:79-90`; runtime deferred to manual console check |
| T-41-21-01 | Tampering | выбор занятого устройства/цикла/teardown | mitigate | CLOSED | server refusal surfaced, UI does not pre-block: `GroupContentsTable.svelte:187-188,386-387,402-403` (`catch → pushToast`); `GroupMoveModal.svelte:104-110` (VALIDATION inline, else Toast) |
| T-41-21-02 | Elevation of Privilege | manager/employee и контролы состава | mitigate | CLOSED (weak: UI gate is UX only) | UI: `GroupsPage.svelte:342` `canEdit={canMutate}` → `GroupContentsTable.svelte:458,491` and footer `:561` (`footer={canEdit ? addRow : undefined}` — gates the "Добавить несколько…" entry at `:547` too). Real boundary: `MutateGroups` `group_service.rs:394,466,508,557,642,741,794,1211`; test `crates/trackly-app/tests/groups_service.rs:920-941` |
| T-41-21-03 | Tampering | рассинхрон счётчиков «Мест» | mitigate | CLOSED | `GroupContentsTable.svelte:201,384,400`; `GroupAddDevicesModal.svelte:262`; gate `ui/scripts/check-place-tree-invalidation.mjs:750-767` (markers resolved through import aliases `:824-861`) + enclosing-function assertion `:866-881` — deleting the notify call fails the gate |
| T-41-21-04 | Injection (XSS) | имена в Dropdown/таблице | mitigate | CLOSED | no `{@html}` in `ui/src/lib/components/Dropdown.svelte` or `ui/src/features/groups/`; option name interpolated `Dropdown.svelte:777-779` |
| T-41-21-05 | Denial of Service | частые запросы поиска | accept | CLOSED | accepted-risks log; controls exist: 250 ms debounce `Dropdown.svelte:300`, `limit: 20` `GroupContentsTable.svelte:261`, server cap 500 `crates/trackly-core/src/domain/groups.rs:19` enforced `group_service.rs:355,607` |
| T-41-21-06 | Tampering | регресс общего Dropdown | mitigate | CLOSED | `Dropdown.svelte:84` `flat = false` default; without `flat` the else-branch keeps the previous count+chevron markup `:787-793`; live check passed (`41-UAT.md` item 8 — act form + printer picker, `result: pass`) |
| T-41-22-01 | Tampering | ip/mac/число в форме | mitigate | CLOSED | no client-side ip/mac formula in `GroupPropertiesForm.svelte` (only placeholder/hint strings `:292-306`); server error mapped per field `:164-188` |
| T-41-22-02 | Information Disclosure | список пользователей | mitigate | CLOSED | `GroupUsersField.svelte:56` uses only `groups.userOptions`; DTO carries id/full_name/login only `crates/trackly-app/src/dto/groups.rs:205-210`; service under `ReadGroups` `group_service.rs:1153`; no `email` reference anywhere in `ui/src/features/groups/` |
| T-41-22-03 | Elevation of Privilege | manager редактирует свойства группы | accept | CLOSED | accepted-risks log |
| T-41-22-04 | Injection (XSS) | имена пользователей/принтеров | mitigate | CLOSED | no `{@html}` in `ui/src/features/groups/` (incl. `GroupUsersField.svelte`, `GroupPrintersList.svelte`) |
| T-41-22-05 | Tampering | потеря данных при конфликте версий | mitigate | CLOSED | `GroupPropertiesForm.svelte:196-199` sends `version`; `:181-186` maps `OPTIMISTIC_LOCK_MISMATCH` to "Обновите карточку"; server CAS `group_service.rs` set_values +19/+25 (`group.version != version → OptimisticLockMismatch`) |
| T-41-22-06 | Denial of Service | эффекты инициализации формы | mitigate | CLOSED (weak) | `GroupPropertiesForm.svelte:73-79` — effect reads `card`, writes inside `untrack`; runtime deferred to manual console check |
| T-41-23-01 | Elevation of Privilege | employee открывает #/groups | mitigate | CLOSED | `ui/src/routes.ts:39-43` — `/groups` genuinely absent from `employeeRoutes`, `'*': AccessDenied`; wiring `ui/src/App.svelte:93-95`; gate rule (D) `ui/scripts/check-groups-section.mjs:281-289` with negative selftest fixture (д) `:489-498` that asserts adding `/groups` to `employeeRoutes` is a violation; server 403 `groups_service.rs:437-458` |
| T-41-23-02 | Elevation of Privilege | manager правит типы через UI | mitigate | CLOSED (weak: UI gate is UX only) | same chain as T-41-20-01: `GroupsPage.svelte:316,332` (`isAdmin`), `GroupTree.svelte:497`, `GroupTreeNode.svelte:111`; server `ManageGroupTypes` admin-only `auth.rs:172-174` |
| T-41-23-03 | Tampering | перенос как побочный эффект сохранения | mitigate | CLOSED | `GroupPropertiesForm.svelte` has no place field (only the `placeholder` attribute at `:303` matches "place"); move is a separate confirmed action `GroupMoveModal.svelte:89-97` |
| T-41-23-04 | Tampering | рассинхрон счётчиков «Мест» | mitigate | CLOSED | `GroupMoveModal.svelte:99` inside `confirmMove()`; `GroupsPage.svelte:243` inside `removeFromParent()`; both covered by the alias-resolving INV-7 marker set `check-place-tree-invalidation.mjs:750-767` (W-F01 fix: literal-substring matching previously missed `groups as groupsApi`) |
| T-41-23-05 | Injection (XSS) | имена групп/мест на странице | mitigate | CLOSED | `{@html}` count across `ui/src/` = 0 real occurrences (single match is a comment at `MovementTimeline.svelte:161`) |
| T-41-23-06 | Information Disclosure | localStorage | accept | CLOSED | accepted-risks log; keys hold `{kind,id}` and a tab key `GroupsPage.svelte:66-105` |
| T-41-24-01 | Tampering | обход запрета через DevTools/прямой запрос | mitigate | CLOSED | server guard `crates/trackly-app/src/services/device_service.rs:720-739` (rejects place change while in a group with a place, compares against the current row inside the tx); test `crates/trackly-app/tests/group_write_sites.rs:349`; UI block is explicitly labelled a hint `DeviceFormBody.svelte:299-307,1036` |
| T-41-24-02 | Injection (XSS) | group_label и group_name | mitigate | CLOSED | `MovementTimeline.svelte:168` interpolation, button handler takes `entry.group_id as number` `:166`; `DeviceFormBody.svelte:1046-1047` same pattern |
| T-41-24-03 | Denial of Service | запрос членства при открытии формы | mitigate | CLOSED | `DeviceFormBody.svelte:496-515` — single `groups.forDevices([target.id])` in `onMount` (edit, non-readonly only), `.catch(() => membership = null)` `:511-514`, form keeps working |
| T-41-24-04 | Elevation of Privilege | forDevices для employee | accept | CLOSED | accepted-risks log; server `ReadGroups` on `for_devices` `group_service.rs:354`, employee-forbidden test `groups_service.rs:456-458` |
| T-41-25-01 | Repudiation | потеря строк пакета в печатном архиве | mitigate | CLOSED | `@media print` count in `ui/src/features/reports/ReportTable.svelte` = 0; intent note `:451-455`; server print emits the flat set — `crates/trackly-app/tests/group_report_batch.rs:528` asserts 7 data rows in the HTML and `:536` each device exactly once; `:589` template is batch-unaware |
| T-41-25-02 | Injection (XSS) | имя группы в отчёте | mitigate | CLOSED | no `{@html}` in `ReportTable.svelte`; `{formatCellDisplay(...)}` `:372`; server print renders through `build_safe_html_env()` `crates/trackly-app/src/services/report_service.rs:1143` = `AutoEscape::Html` `crates/trackly-app/src/pdf/minijinja_env.rs:118-126` |
| T-41-25-03 | Tampering | потеря строк при фильтрах/LIMIT | mitigate | CLOSED | `ReportTable.svelte:215-235` `synthesizeHeader` + headerless-batch branch `:266-275`; `batch_size` counted server-side by `batch_id` subquery `report_service.rs:1592-1596`, documented `crates/trackly-app/src/dto/reports.rs:159-163` |
| T-41-25-04 | Tampering | рассинхрон счётчиков после массового переноса | mitigate | CLOSED | `ui/src/features/places/PlaceContents.svelte:301` inside `handleMoveConfirm()`; INV-7 marker for this call `check-place-tree-invalidation.mjs:736-738` |
| T-41-25-05 | Elevation of Privilege | массовый перенос | accept | CLOSED | accepted-risks log |
| T-41-26-01 | Information Disclosure | реальные данные в тестах/артефактах | mitigate | CLOSED | `scripts/check-privacy.mjs` wired to `core.hooksPath=.githooks` → `.githooks/pre-commit` and to CI `.github/workflows/ci-fast.yml:31`; re-ran it now: `PASS — 0 нарушений`; test data fictional (`crates/trackly-app/tests/group_write_sites.rs:751-752`) |
| T-41-26-02 | Tampering | ложно-зелёный результат «по своим файлам» | mitigate | CLOSED | `.planning/phases/41-groups-model-and-editor/41-VALIDATION.md:290` — whole-workspace sequential run, 158 binaries, 1625 passed, 0 failed; `:289` `cargo clippy --workspace --all-targets -- -D warnings` exit 0 |
| T-41-26-03 | Tampering | следы мутационных проверок в коммите | mitigate | CLOSED | `git status --porcelain` empty; no mutation markers in tracked source; both mutation harnesses mutate in-memory constants or a scratch copy only (`check-reorder.mjs:196-199` `--impl=`, `check-groups-section.mjs:420-520` in-memory fixtures) |
| T-41-26-04 | Repudiation | живые проверки закрыты без запуска приложения | mitigate | CLOSED | `41-HUMAN-UAT.md:1-25` tracking doc handed to the user; `41-VERIFICATION.md` `re_verification.previous_status: human_needed` / `previous_score: "8/8 на уровне кода, 0/8 в рантайме"`; live acceptance recorded afterwards, `41-UAT.md:137` `passed: 21` |
| T-41-26-05 | Denial of Service | параллельные cargo test конфликтуют | mitigate | CLOSED | `41-VALIDATION.md:282` ("строго последовательно, по одному `cargo` за раз; рабочее дерево чистое") and a single full run `:290` |
| T-41-27-01 | Elevation of Privilege | menu items visible outside clipping container | accept | CLOSED | Role gates confirmed wrapping the component: `ui/src/features/places/PlaceTreeNode.svelte:181` (`{#if isAdmin}` → ActionMenu :183), `ui/src/features/groups/GroupTreeNode.svelte:197` and `:218` (`showTypeMenu` / `showGroupMenu` → ActionMenu :199, :220), `ui/src/features/groups/GroupTypePropertiesTable.svelte:589` (`{#if canEdit}` → ActionMenu :590). **Real boundary is server-side `authorize`**: `crates/trackly-app/src/services/group_type_service.rs:252,314,399,456,549,649,692,751` (`Action::ManageGroupTypes`, first line of each mutation) |
| T-41-27-02 | Denial of Service (a11y) | Modal focus trap vs. portaled panel | mitigate | CLOSED | `ui/src/lib/components/ActionMenu.svelte:232` (`use:portal`), `:141-147` (Tab closes menu, returns focus to trigger), `:100-103` and `:136-140` (Esc `preventDefault`+`stopPropagation`); `ui/src/lib/utils/portal.ts:24` sets `data-tr-portal`; `ui/src/lib/components/Modal.svelte:106` (`PORTAL_FOCUSABLE_SELECTOR`), `:150-170` (`trapTab` merges scoped+portaled nodes), `:181` (`defaultPrevented` → Esc does not close the dialog), `:203` (`<svelte:window onkeydown>` so panel-in-body events still reach the trap). Live check R2 passed 2026-10-06 (`41-HUMAN-UAT.md` item 21 `result: pass`) |
| T-41-27-03 | Tampering (UI regression) | return of non-portal mode | mitigate | CLOSED | `ui/scripts/check-action-menu-portal.mjs` — rule A requires `use:portal` + `use:actionMenuPortalPosition` on the single `role="menu"` tag (:221-252), B blocks the `portal` prop/`usePortal` returning (:253-277), C requires `position: fixed` (:278-303), D rejects a `portal` attribute at any call site (:306-325), E vacuity guard `MIN_CALL_SITES = 13` (:57); no `portal` in `Props` (`ActionMenu.svelte:11-38`); registered with `--selftest` in `ui/package.json:16` |
| T-41-27-04 | Information Disclosure | orphaned panel in `<body>` after unmount | accept | CLOSED | `ui/src/lib/utils/portal.ts:29-31` (`destroy()` removes the node); panel is inside `{#if open}` (`ActionMenu.svelte:225-239`) so it unmounts with the row |
| T-41-28-01 | Spoofing (misleading copy) | type-panel hint for manager | mitigate | CLOSED | `ui/src/features/groups/GroupTypePanel.svelte:130` — single unconditional line «Код и поведение типа изменить нельзя.», no role branch anywhere in the file (grep: only `canEdit` gates for controls, :136, :143), no word «можно»/«можете». Server boundary: `group_type_service.rs:252…751` (`ManageGroupTypes`), role matrix tests `crates/trackly-app/tests/groups_types_service.rs:690` (`rights_types_by_role`) and `:1535` (`rights_properties_by_role`). Live check R3 passed (`41-HUMAN-UAT.md` item 22) |
| T-41-28-02 | Information Disclosure | phase document edits | accept | CLOSED | Privacy gate live: `.githooks/pre-commit:31` (staged blobs, fail-closed on missing node) + `.github/workflows/ci-fast.yml:31`; gate re-run by auditor on HEAD: `PASS — 0 нарушений` |
| T-41-29-01 | Tampering (bypass via state sequence) | `update_property` / `unarchive_property` | mitigate | CLOSED (deviation, see below) | Invariant enforced on three entries: hide clears the flag — `crates/trackly-infra/src/repos/group_types_sqlite.rs:404-421` (`archive_property_on`, `is_required = 0` at :416); update rejects required-on-hidden — `group_type_service.rs:593-595` (`// W-B01:1` → `hidden_property_required_error()`, defined :112); unhide normalizes — `group_types_sqlite.rs:429-450` (`unarchive_property_on`, `is_required = 0` at :445) with rationale at `group_type_service.rs:708-716`. Tests `crates/trackly-app/tests/groups_types_service.rs:1305` (`protect_d_hide_then_require_is_rejected`), `:1383` (`…clears_flag`), `:1466` (`protect_d_unarchive_clears_legacy_required_flag`) |
| T-41-29-02 | Denial of Service (user locked out of `set_values`) | `group_service::set_values` after un-hide | mitigate | CLOSED | `crates/trackly-app/tests/groups_types_service.rs:1436-1446` — after un-hide with a violator group present, `unarchive_property` succeeds, `back.is_required == false`, and `set_values` on an unrelated field passes; mechanism is the `is_required = 0` in `group_types_sqlite.rs:445` |
| T-41-29-03 | Repudiation | required flag dropped on hide without trace | mitigate | CLOSED | `crates/trackly-app/tests/groups_types_service.rs:1449-1460` asserts exactly one `audit_log` row `action='archive'` with `before_json LIKE '%"is_required":true%'`; written inside the same transaction as the mutation (`group_type_service.rs:657-679`) |
| T-41-29-04 | Elevation of Privilege | manager changes required flag | accept | CLOSED | `group_type_service.rs:549` (`update_property`) and `:692` (`unarchive_property`) — `authorize(caller, &Action::ManageGroupTypes)?` is the first statement, unchanged; matrix test `tests/groups_types_service.rs:1535` |
| T-41-30-01 | Repudiation / Information Disclosure (silent truncation) | `query_movements_inner`, print, CSV | mitigate | CLOSED (partial, see weak evidence) | True `total`: `crates/trackly-app/src/services/report_service.rs:1733-1745` (`// W-B03:count`); notice text single source `:262-269`; CSV trailing record `:973-984`; print line appended to `filter_summary` `crates/trackly-app/src/tauri_cmds/reports.rs:399-412`, shared by both transports (`crates/trackly-app/src/http/reports.rs:295` calls the same builder); screen banner `ui/src/features/reports/ReportsPage.svelte:622-623`; tests `crates/trackly-app/tests/report_movements_truncation.rs:145-340`. `place_movements` journal untouched (no writes added in the diff of these files). Live: banner + print + CSV passed R5 (`41-HUMAN-UAT.md` item 24 `note`) |
| T-41-30-02 | Denial of Service | 1000-row ceiling as a safety valve | accept | CLOSED | Ceiling kept as a constant `report_service.rs:256` (`MOVEMENTS_REPORT_LIMIT = 1000`), ordering unchanged (`ORDER BY pm.created_at_utc ASC, pm.id ASC`, `:1602`), extra `COUNT(*)` runs only at the ceiling (`:1734`) and exactly once per request |
| T-41-30-03 | Tampering | count query drifting from row query (false `total`) | mitigate | CLOSED | `report_service.rs:1735-1741` reuses the very same `with_prefix`, `where_clause`, `param_refs` built at `:1567-1577`; conditions reference only `pm.*` and `d.type_id` (`:1517-1565`), so the count query's single `devices` join is sufficient; test `tests/report_movements_truncation.rs:183` (`report_trunc_total_respects_filters`, subtree CTE + device-type join) |
| T-41-30-04 | Injection | CSV formulas / notice text | mitigate | CLOSED | Notice is a constant template of two integers (`report_service.rs:262-269`, no caller-supplied text); CSV write path passes it through `csv_safe` (`:979`, helper `:247-253`, unit test `:2827-2833`) |
| T-41-30-05 | Elevation of Privilege | `ReadPlaces` gate on the movements report | accept | CLOSED | `crates/trackly-app/src/tauri_cmds/reports.rs:306-316` (`export_gate_action` → `Action::ReadPlaces` for `movements`), pinning test `:330-332` (`movements_export_gate_is_read_places_not_read_data`) plus `:338-348` guard that other types keep `ReadData`; gate applied before any work in the print path (`:390`) |
| T-41-31-01 | Tampering (loss of a property definition because copy promised reversibility) | removal modal / menu | mitigate | CLOSED | Branch on `filled_group_count` before the decision: `ui/src/features/groups/propertyRemoval.ts:29-50`; menu label and danger class `GroupTypePropertiesTable.svelte:509` (`{@const removal = …}`), `:613-616`; modal title/body/confirm from the same module `:670-681`; empty property → «Удалить безвозвратно» with `confirmVariant: 'destructive'` (`propertyRemoval.ts:47-48`). Gate `ui/scripts/check-property-removal.mjs` executes the fixture against the real module (:104-130) and enforces S1-S3 against the real `.svelte` (:146-185); M1 (unconditional hide) and M2 (`>= 0`) are in the selftest mutant list (:230-262) |
| T-41-31-02 | Repudiation | race between modal open and confirm | accept | CLOSED | Server decides by the current count (`group_type_service.rs:660-666`), success toast is taken from the server response: `GroupTypePropertiesTable.svelte:382` (`outcome.archived ? 'Свойство скрыто' : 'Свойство удалено'`) |
| T-41-31-03 | Elevation of Privilege | «Удалить свойство» visible to non-admin | mitigate | CLOSED | Row menu still inside `{#if canEdit}` — `GroupTypePropertiesTable.svelte:589-620`; real boundary `group_type_service.rs:649` (`delete_property` → `ManageGroupTypes`); matrix test `tests/groups_types_service.rs:1535` unchanged |
| T-41-31-04 | Information Disclosure / Injection | property name inside modal text | mitigate | CLOSED | Rendered by Svelte interpolation only: `GroupTypePropertiesTable.svelte:672` (`{removalCopy.body}`), `:616` (`{removal.menuLabel}`); `grep -rn "@html" ui/src/features/groups/ ui/src/features/reports/` → no matches; fixture name is fictional («Хост», `ui/scripts/fixtures/property-removal/cases.json`) |
| T-41-31-05 | Tampering | JS rule drifting from the server rule (`> 0`) | mitigate | CLOSED | Shared golden fixture read from disk by both sides: `ui/scripts/check-property-removal.mjs:46` and `crates/trackly-app/tests/groups_property_removal_parity.rs:17-21`; Rust test exercises the real `delete_property` and asserts `archived == (kind == "hide")` (`:143-152`) with `MIN_CASES = 6` anti-shrink guard (`:21`, `:156`) |
| T-41-32-01 | Repudiation (schema change without audit trace on partial failure) | 8 `GroupTypeService` mutations | mitigate | CLOSED | One transaction per mutation, audit insert inside it, commit last — `group_type_service.rs` tx/insert/commit triples at 264/286/299, 356/371/384, 407/429/442, 470/520/533, 574/623/636, 657/666/679, 700/725/738, 761/769/782 (8 of 8). Fault injection tests `tests/groups_types_service.rs:1683-1980` (10 `atomic_*`, `install_audit_fault` trigger harness, each asserting the mutation did not stick and then succeeding after `remove_audit_fault`) |
| T-41-32-02 | Tampering (change committed although the caller saw an error) | `update_type`, `create_property`, … | mitigate | CLOSED | Early returns drop the `Transaction` → rollback: `create_property` `group_type_service.rs:503-507` returns `required_violation_error` after the insert with no compensating hard delete (the former compensation is gone from this path); guard test `tests/groups_types_service.rs:1789` (`atomic_create_property_required_violation_leaves_nothing`) |
| T-41-32-03 | Denial of Service (long writer hold) | widened transaction | accept | CLOSED | Mutations touch single rows (`group_types_sqlite.rs:369-399`, `404-450`); all writes go through the single writer (`group_type_service.rs:…self.writer.execute(...)` in every mutation), so serialization pre-exists |
| T-41-32-04 | Elevation of Privilege | authorization of mutations | accept | CLOSED | `authorize(caller, &Action::ManageGroupTypes)?` is the first statement of all 8 mutations and sits before the transaction is opened (`group_type_service.rs:252,314,399,456,549,649,692,751` vs. tx at 264,356,407,470,574,657,700,761) |
| T-41-33-01 | Repudiation (silent data loss on the report screen) | `ReportsPage` | mitigate | CLOSED | Banner driven by the **server** field: `ui/src/features/reports/ReportsPage.svelte:550-553` (`movementsTruncationNotice(rows.rows.length, rows.total)`) and `:622-623` (`{#if truncationNotice}` → `role="status"`); gate rule S3 forces the second argument to end in `.total` and the first in `.length` without `.total` (`ui/scripts/check-report-truncation.mjs:169-181`); text pinned by the shared fixture (`ui/scripts/fixtures/report-truncation/cases.json`, 8 cases) |
| T-41-33-02 | Injection (XSS) | banner text | mitigate | CLOSED | `ReportsPage.svelte:623` renders `{truncationNotice}` by interpolation; no `{@html}` in `ui/src/features/reports/` (grep, 0 matches); text is a constant template of two numbers (`ui/src/features/reports/truncationNotice.ts:18-21`) |
| T-41-33-03 | Denial of Service | `$derived` / page effects | mitigate | CLOSED | `grep -c '\$effect' ui/src/features/reports/ReportsPage.svelte` → **0**; the notice is a pure `$derived` (`:550-553`) that writes no readable state. Live console check for `effect_update_depth_exceeded` is part of R1-R5, which passed (R5's only defect is unrelated, see below) |
| T-41-33-04 | Tampering | screen text diverging from print text | mitigate | CLOSED | One fixture, both sides: `crates/trackly-app/tests/report_movements_truncation.rs:19-20,130,335` (`report_trunc_notice_matches_golden_fixture`) and `ui/scripts/check-report-truncation.mjs:46-124`; mutants M2/M3 plus M4/M6/M7 in the selftest (`:230-262`), anchor uniqueness enforced by `assertUniqueAnchor` (`:261-271`) before every mutation |
| T-41-34-01 | Information Disclosure (real personal/org data in a public repo) | tests, fixtures, SUMMARY, UAT docs | mitigate | CLOSED | Gate wired at both points: `.githooks/pre-commit:31` (`--staged`, fail-closed if node missing, `core.hooksPath=.githooks` confirmed) and `.github/workflows/ci-fast.yml:31`; auditor re-ran the gate on HEAD → `PASS — 0 нарушений`; spot-read of the wave's new fixtures/tests shows only fictional values («Хост», «Стойка …», «Склад …»); guarantee boundary recorded in `41-34-SUMMARY.md` («проверен диф волны, не история») |
| T-41-34-02 | Repudiation (false gap closure without a live check) | `41-UAT.md`, `41-HUMAN-UAT.md` | mitigate | CLOSED | At plan time no R1-R5 item was marked `pass` and both gaps carried `fixed_pending_reverify`; the live re-verification was subsequently performed by the user (commits `294eda36`, `b875fa27`) and only then flipped: `41-HUMAN-UAT.md` items 20-23 `result: pass`, gap entries carry `verified_by: "живая перепроверка R1/R2 … R4 от 2026-10-06 (десктоп + LAN)"`; item 24 (R5) remains `result: issue` / `status: failed`, i.e. the open defect was not laundered into `passed` |
| T-41-34-03 | Tampering (regression outside plan files) | trackly-app / workspace / UI chain | mitigate | CLOSED (self-reported run, see weak evidence) | Full-chain run recorded in `41-34-SUMMARY.md` (7 CI steps, 158 binaries, 1625 passed, 0 failed) and a second full regression after the WR-01 fix (commit `81f15b57`: 157 targets, 1624 passed, 0 failures); the chain itself is a real enforced gate: `.github/workflows/ci-fast.yml:92` (fmt), `:95` (clippy `-D warnings`), `:113` (`cargo test --workspace`), `:120` (svelte-check), `:124` (`pnpm lint`, which executes all three new gates with `--selftest` per `ui/package.json:16`) |
| T-41-34-04 | Denial of Service (build hang) | `login_remember_persistent_cookie` | accept | CLOSED | Pre-existing hang, worked around with `--skip` in the local wave run only (`41-34-SUMMARY.md`, task 1, step 5); not introduced by this wave; CI invokes `cargo test --workspace` without the skip (`ci-fast.yml:113`) |

---

## Accepted Risks Log

30 threats carry an `accept` disposition authored at plan time. Rationales are quoted verbatim
(Russian, as written in the plan). Accepted risks do not resurface in future audit runs.

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| R-41-01 | T-41-02-04 | «токен group — константа, чувствительных данных не несёт» | plan-time disposition, 41-02-PLAN | 2026-10-04 |
| R-41-02 | T-41-04-05 | «число свойств типа ограничено практикой (единицы-десятки), сервис держит потолок» — ceiling verified: `MAX_PROPERTIES_PER_TYPE = 50`, `group_type_service.rs:93`, enforced `:472-478` | plan-time disposition, 41-04-PLAN | 2026-10-04 |
| R-41-03 | T-41-05-04 | «имя группы — не персональные данные; снимок остаётся читаемым после удаления группы осознанно» | plan-time disposition, 41-05-PLAN | 2026-10-04 |
| R-41-04 | T-41-06-05 | «один рекурсивный запрос по индексам idx_groups_parent/idx_group_devices_group, не N+1» — indexes verified present, `V045__groups.sql:88,98` | plan-time disposition, 41-06-PLAN | 2026-10-04 |
| R-41-05 | T-41-09-06 | **Rationale corrected by audit — see Register corrections below.** As written: «существующий rate-limit middleware build_router; сессия обязательна». Corrected: there is **no** router-wide rate limit; the governor is route-scoped to `auth_login` and `request_ad_restore` only. The accepted residual is "an *authenticated* admin/manager session can issue unbounded calls on a LAN"; unauthenticated flooding is answered with 401 before any service work (`role_endpoint_matrix.rs:3498-3503`) | plan-time disposition, 41-09-PLAN; correction by audit 2026-10-06 | 2026-10-04 |
| R-41-06 | T-41-10-06 | «состав ограничен практикой (до сотен); один запрос writer; очередь writer с send_timeout» — no code-enforced bound on composition size; a group with tens of thousands of members would hold the single writer for one transaction | plan-time disposition, 41-10-PLAN | 2026-10-04 |
| R-41-07 | T-41-10-07 | «только имена групп и счётчики; ФИО не пишутся» — spot-checked: audit payloads carry only ids/counters (`group_place.rs:218-223`, `group_service.rs:708-715`) | plan-time disposition, 41-10-PLAN | 2026-10-04 |
| R-41-08 | T-41-14-05 | «права devices_update не меняются этим планом (MutateDevices); guard добавляет ограничение для всех ролей» — verified by `group_write_sites.rs:462` (`s1_table_manager_is_also_guarded`) | plan-time disposition, 41-14-PLAN | 2026-10-04 |
| R-41-09 | T-41-15-05 | «гейты MutateDevices/MutateCartridges не менялись; группа едет в рамках тех же прав» — spot-checked `place_service.rs:697-698` | plan-time disposition, 41-15-PLAN | 2026-10-04 |
| R-41-10 | T-41-16-05 | «release — два простых SQL в той же tx; ошибка откатывает акт целиком, как любая ошибка записи» | plan-time disposition, 41-16-PLAN | 2026-10-04 |
| R-41-11 | T-41-17-03 | «batch_id — UUID без смысла, entity_label — имя группы» — spot-checked: v4 UUID `group_place.rs:203`, name snapshot `report_service.rs:1695-1699` | plan-time disposition, 41-17-PLAN | 2026-10-04 |
| R-41-12 | T-41-17-05 | «гейты reports_* не меняются» | plan-time disposition, 41-17-PLAN | 2026-10-04 |
| R-41-13 | T-41-18-01 | «UI — только UX-слой (UI-SPEC §7.3); права проверены на сервере (планы 09, 13)» | plan-time disposition, 41-18-PLAN | 2026-10-04 |
| R-41-14 | T-41-19-01 | «UX-слой; права на сервере (планы 09, 13); пункты меню типа не рендерятся для не-admin» | plan-time disposition, 41-19-PLAN | 2026-10-04 |
| R-41-15 | T-41-19-05 | «хранит только id раскрытых узлов (без персональных данных)» — key `trackly:groups:expanded`, `GroupTree.svelte:92-100,234` | plan-time disposition, 41-19-PLAN | 2026-10-04 |
| R-41-16 | T-41-21-05 | «встроенный 250мс-дебаунс Dropdown; лимит 20 результатов; потолок 500 id на сервере» — all three verified (`Dropdown.svelte:300`, `GroupContentsTable.svelte:261`, `group_service.rs:355,607`) | plan-time disposition, 41-21-PLAN | 2026-10-04 |
| R-41-17 | T-41-22-03 | «разрешено (MutateGroups admin\|manager); employee доступа к разделу не имеет — гейт сервера» | plan-time disposition, 41-22-PLAN | 2026-10-04 |
| R-41-18 | T-41-23-06 | «только id выбранного узла/вкладка» — `GroupsPage.svelte:66-105` | plan-time disposition, 41-23-PLAN | 2026-10-04 |
| R-41-19 | T-41-24-04 | «форма устройства employee недоступна; groups_for_devices закрыт ReadGroups на сервере (план 13)» | plan-time disposition, 41-24-PLAN | 2026-10-04 |
| R-41-20 | T-41-25-05 | «гейты MutateDevices/MutateCartridges сервера не менялись (план 15)» | plan-time disposition, 41-25-PLAN | 2026-10-04 |
| R-41-21 | T-41-27-01 | «роль-гейты стоят в разметке потребителя вокруг самого `<ActionMenu>`; `overflow` контейнера границей безопасности не был; серверные проверки (`ManageGroupTypes`, `MutateGroups`) остаются источником истины» | plan-time disposition, 41-27-PLAN | 2026-10-05 |
| R-41-22 | T-41-27-04 | «`use:portal` удаляет узел в `destroy()`; ActionMenu размонтируется вместе со строкой» — verified `portal.ts:29-31`, `ActionMenu.svelte:225-239` | plan-time disposition, 41-27-PLAN | 2026-10-05 |
| R-41-23 | T-41-28-02 | «правка копирайта не вносит данных; privacy-гейт перед коммитом» | plan-time disposition, 41-28-PLAN | 2026-10-05 |
| R-41-24 | T-41-29-04 | «`authorize(Action::ManageGroupTypes)` — первой строкой каждого метода, не менялась; матрица `rights_properties_by_role` остаётся зелёной» | plan-time disposition, 41-29-PLAN | 2026-10-05 |
| R-41-25 | T-41-30-02 | «потолок и порядок сохранены; добавленный `COUNT(*)` выполняется только когда потолок достигнут, использует те же условия и один раз за запрос; индексы таблицы те же» | plan-time disposition, 41-30-PLAN | 2026-10-05 |
| R-41-26 | T-41-30-05 | «`authorize_report_export` и `build_reports_list_movements` не менялись (WR-02 тест `movements_export_gate_is_read_places_not_read_data` остаётся зелёным)» | plan-time disposition, 41-30-PLAN | 2026-10-05 |
| R-41-27 | T-41-31-02 | «сервер решает по актуальному счёту; тост успеха берётся из ответа сервера (`outcome.archived`), поэтому пользователь узнаёт фактический исход; потерь данных нет (в худшую сторону — сохраняется больше, а не меньше)» | plan-time disposition, 41-31-PLAN | 2026-10-05 |
| R-41-28 | T-41-32-03 | «операции короткие (единицы строк); writer — единственная точка записи и без того сериализует мутации; добавлено только объединение уже существующих шагов» | plan-time disposition, 41-32-PLAN | 2026-10-05 |
| R-41-29 | T-41-32-04 | «`authorize(Action::ManageGroupTypes)` — первой строкой каждого метода, до открытия транзакции, не менялась; `rights_*` и `role_endpoint_matrix` в наборе прогонов» | plan-time disposition, 41-32-PLAN | 2026-10-05 |
| R-41-30 | T-41-34-04 | «предсуществующее подвисание обходится `--skip`; не вводится волной» — pre-existing hang of `login_remember_persistent_cookie`; CI invokes `cargo test --workspace` without the skip (`ci-fast.yml:113`) | plan-time disposition, 41-34-PLAN | 2026-10-05 |

---

## Register corrections

Two register entries describe a control that does not match the shipped code. Neither reopens a
threat — in both cases the *invariant* holds — but the register wording would mislead a future
reader, so the corrected text is recorded here and supersedes the plan text.

1. **T-41-09-06 (41-09-PLAN) — names a rate limit that does not cover these routes.**
   The plan justifies accepting the DoS risk with "существующий rate-limit middleware
   `build_router`". There is no router-wide rate limit: `tower_governor::GovernorLayer` is
   attached with `.route_layer()` to exactly two routes — `/api/v1/auth_login`
   (`crates/trackly-app/src/http/mod.rs:118-124`) and `/api/v1/request_ad_restore` (`:140-142`).
   The ten group-type routes merged at `:163` inherit the session layer and the security headers
   but **no** throughput limit. Disposition stays `accept`; the residual is correctly stated in
   R-41-05. Per-request work is bounded by the verified 500/100/50/2000-char ceilings
   (T-41-02-03, T-41-08-04, T-41-08-06).

2. **T-41-29-01 (41-29-PLAN) — the declared control was deliberately replaced.**
   The plan promises that `unarchive` **refuses** when a legacy "hidden + required + violators"
   state is found. The shipped code does not refuse: `unarchive_property_on`
   (`crates/trackly-infra/src/repos/group_types_sqlite.rs:429-450`) clears `is_required` in the
   same UPDATE, with the rationale at `group_type_service.rs:708-716` — review finding WR-01
   established that refusing locked the user into a dead end. The register's invariant
   ("a hidden property is never required") is enforced on all three entry points and tested;
   the new form is strictly better for T-41-29-02. The test name changed accordingly
   (`protect_d_unarchive_clears_legacy_required_flag`). Register wording is superseded by this
   entry.

---

## Residual items

CLOSED threats whose proof is thinner than the register implies. None is an exploitable gap
today and none reaches `block_on: high` at ASVS L1 for a single-org LAN tool. Listed so a
future reader does not over-trust the register.

### Scope-of-claim issues

1. **Write-site inventory is understated (T-41-16-02).** The gate watches three repository
   methods and its own comment at `group_write_sites.rs:1584-1587` claims the repository writes
   `devices.place_id` "with exactly three methods". That is inaccurate:
   `SqliteDeviceRepository::update_in_tx` also writes it
   (`crates/trackly-infra/src/repos/devices_sqlite.rs:285`), as do `create_in_tx` and
   `clone_device_in_tx`. No hole exists now — the single `update_in_tx` device caller sits behind
   the S1 guard and create/clone yield non-member devices — but a future service moving a device
   via `update_in_tx` would be caught by **neither** gate (gate 1 is scoped to `act_service.rs`,
   gate 2 is keyed on the three names). Same class as the phase-40.1 inventory lesson the gate's
   own comment cites. Severity: low. → candidate for `deferred-items.md`.

2. **Source-grep gates cannot see routes registered elsewhere (T-41-09-04, T-41-13-06,
   T-41-16-02).** These gates read Rust source text with regex/substring counting rather than
   interrogating the axum `Router`. They are bidirectional, count-pinned (10 and 15 routes) and
   carry anti-vacuity guards, so they do what the register promised — but an endpoint registered
   from another module, or a path built by concatenation, is invisible to them. Directly adjacent
   to this project's known `spa_fallback` behaviour (see item 7). Severity: low-medium.
   A stronger form would enumerate the real `Router` or probe each table path for not-404.

3. **T-41-18-02 has no permanent gate.** All 25 `apiCall` wrapper names in
   `ui/src/lib/api/groups.ts:42-109` were verified against `ui/src/bindings.ts` by hand during
   this audit. `svelte-check` types the argument shapes but no `check-*.mjs` asserts the name
   set, so a renamed command plus a stale wrapper would be caught only by the next manual pass.

4. **T-41-23-01 gate asserts the tables, not the wiring.** `check-groups-section.mjs` strips
   comments and parses the route arrays (far stronger than a naive grep; negative fixture (д)
   proves it fails when `/groups` is added to `employeeRoutes`). It does **not** assert that
   `App.svelte` selects `employeeRoutes` for the employee role — that wiring
   (`App.svelte:93-95`) was read manually, and removing it would still pass the gate.

### Evidence that is structural rather than tested

5. **UI role gates are not controls (T-41-20-01, T-41-21-02, T-41-23-02, T-41-27-01,
   T-41-31-03).** The `{#if canEdit}` / `{#if isAdmin}` wrappers exist exactly where claimed,
   but a client-side `{#if}` is UX, not security — anyone can call `/api/v1/*` directly. These
   are CLOSED **only** because the server-side `authorize(...)` is present on every corresponding
   service method and covered by role tests. If the register is ever read as "the UI gate is the
   mitigation", that reading is wrong.

6. **Runtime rune behaviour is not provable by compile gates (T-41-19-04, T-41-20-05,
   T-41-22-06, T-41-27-02, T-41-33-03).** `untrack` is genuinely at the write site in every
   named component and `$effect` count is 0 where claimed, but nothing automated proves the
   absence of `effect_update_depth_exceeded` — `svelte-check`, `eslint` and `pnpm build` are
   blind to it on this project. The runtime coverage is the live session of 2026-10-06
   (`41-HUMAN-UAT.md` items 20-23, `result: pass`), not a gate.

7. **T-41-30-01 is mitigated on three of four channels.** Banner, print line and CSV record all
   work on both transports and were live-verified. The tab-badge channel is dead over LAN:
   `reports_get_report_counts` has no HTTP route (`crates/trackly-app/src/http/reports.rs:305-360`
   registers 15 report endpoints, not this one), `spa_fallback`
   (`crates/trackly-app/src/http/mod.rs:252`) returns `index.html`/200 for unrouted `/api/v1/*`,
   and the client swallows it with an empty `.catch` (`ReportsPage.svelte:449`). Pre-phase-41
   defect (introduced in phase 28), caught by live check R5, already recorded as deferred debt.
   The repudiation threat stays mitigated because the badge **under**-reports rather than
   overstating completeness. Severity: low for this threat, medium as a product defect.

8. **T-41-14-02 (race) is argued structurally, not raced.** The guard provably reads membership
   in the same writer closure as the UPDATE (`device_service.rs:706`/`:729`/`:741`), the
   single-writer architecture serialises, and `group_devices`' PK is the backstop. No test
   actually races two writers. Sound for a single-writer design.

9. **T-41-17-01 has no markup-escaping regression test.** The escaping itself was read directly
   (`report_service.rs:1144` → `minijinja_env.rs:118-127`, plain `{{ cell }}` at
   `report.html:148-152`, no `| safe` in `report.html`). What is missing is a test asserting that
   a group name containing HTML markup comes out escaped in the movements report; the nearest
   tests assert row counts and plain-text cell contents.

10. **T-41-25-01 rests on an absence-grep.** "No print rules in `ReportTable.svelte`" is true
    today (`@media print` count 0), but nothing prevents a future batch-collapse print rule
    arriving from a global stylesheet; only the server-side 7-row test
    (`group_report_batch.rs:528`) would still hold, and it does not inspect the screen DOM.

### Verification-completeness issues

11. **T-41-08-01 — `authorize` present on 15 methods, Forbidden asserted on 15 across slices.**
    Slice A's own tests cover 8 of the 15 `GroupService` methods; the remaining seven
    (`move_group`, `add_devices`, `remove_devices`, `set_parent`, `user_options`, `card`,
    `set_values`) are covered by the plan-13 matrix verified in slice B — 15 `build_groups_*`
    wrappers, 3 roles × 2 transports (`role_endpoint_matrix.rs:4167-4206` and `:4213-4231`).
    **Cross-slice check performed at assembly: the gap slice A flagged is closed by slice B.**

12. **T-41-34-03 — regression completeness is a self-report.** The CI chain is a genuine enforced
    gate, but the wave's local run deviated twice: `cargo fmt` was checked only on the 7 files of
    the wave (pre-existing drift elsewhere, so CI's `cargo fmt --all --check` at
    `ci-fast.yml:92` is not demonstrated green by this wave's evidence), and
    `login_remember_persistent_cookie` was skipped locally while CI runs it unskipped. "The whole
    CI chain" is true of the steps that were run, not literally of the CI job as configured.

13. **T-41-26-02's full run excludes one test.** The 1625-passed workspace run uses
    `--skip login_remember_persistent_cookie` (pre-existing hang, `41-VALIDATION.md:336`).
    Recorded as separate project debt, not introduced by this phase.

14. **Gate selftests mutate in-memory fixtures, not the real files.** The three new gates prove
    mutant-detection against embedded reference fixtures; the real-file mutation runs are
    executor self-reports. Mitigating and checked: the structural rules do run against the real
    sources on every `pnpm lint`, and this project's recorded "green while disabled" failure mode
    is explicitly guarded — `check-report-truncation.mjs:261-271` counts occurrences and exits 1
    unless the mutation anchor appears exactly once, using `split`/`join` instead of `replace`.
    `MIN_CALL_SITES = 13` / `MIN_CASES = 6` / 8-case guards block the "shrink the fixture instead
    of fixing the code" escape. Rated CONFIRMED rather than plausible-only.

### Suggested for a future register

15. **An unrouted `/api/` path answering HTML 200 instead of 404 is a silent-failure pattern,
    not only a counter bug.** Worth registering as a threat in its own right when the
    `spa_fallback` debt is picked up: the masking is what let item 7 survive unnoticed from
    phase 28.

### Negative findings (checked, no gap)

- **`groups.removeDevices` has no `notifyPlaceContentChanged` — and should not.**
  `remove_devices` → `release_device_in_tx`
  (`crates/trackly-app/src/services/group_membership.rs:18-48`) only drops the membership row
  and writes audit; it never touches `devices.place_id`. The INV-7 registry deliberately
  enumerates only `groups_move` / `groups_add_devices` / `groups_set_parent`, matching the server
  mutations that actually move places.
- **"Single INSERT point" for the journal is scoped to groups, not repo-wide.** Both group write
  sites go through `record_batch_movement_if_applicable`; two other
  `INSERT INTO place_movements` statements exist (`report_service.rs:3598`,
  `cartridges_sqlite.rs:3072`), both predating phase 41 and outside this register.

### Unregistered attack surface

None. All 34 SUMMARY files declare no new surface, and the audit found none: no new HTTP route,
no new auth path, no unregistered schema migration, no new `{@html}`, no new outbound call. The
only surface finding is the **absence** of a route (item 7), which predates this phase.

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-10-06 | 167 | 167 | 0 | gsd-security-auditor ×4 (slices A/B/C/D), assembled by orchestrator |

Privacy gate (`scripts/check-privacy.mjs`) re-run against HEAD during the audit:
**PASS — 0 нарушений**. Wired fail-closed at `.githooks/pre-commit:31` and as the first CI step
(`.github/workflows/ci-fast.yml:31`). Guarantee boundary unchanged: the gate covers what is
staged/HEAD, not git history.

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer) — 137 mitigate, 30 accept, 0 transfer
- [x] Accepted risks documented in Accepted Risks Log — 30 entries, R-41-01 … R-41-30
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter
- [x] Two register wording corrections recorded (T-41-09-06, T-41-29-01)
- [x] 15 residual items recorded, none at `block_on: high`

**Approval:** verified 2026-10-06
