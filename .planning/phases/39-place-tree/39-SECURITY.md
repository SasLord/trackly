---
phase: 39
slug: place-tree
status: verified
threats_open: 0
asvs_level: 1
created: 2026-08-26
---

# Phase 39 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

Audit scope: git range `9b44bbd4..HEAD` (phase-39 implementation), migrations
`V037`/`V038`, `crates/trackly-core`, `crates/trackly-infra`, `crates/trackly-app`,
`ui/src`, plus the phase's `.planning/` artifacts. The threat register was authored
at plan time across 22 PLAN files (`39-01` … `39-22`); this audit **verifies the
declared mitigations exist in the implemented code** — it is not a retroactive
STRIDE scan.

**No SUMMARY declares a `## Threat Flags` section** (verified: `awk` over all 22
`39-*-SUMMARY.md` returns nothing). Because that channel was silent, new attack
surface was instead derived mechanically from the diff (`git diff 9b44bbd4..HEAD --
crates/trackly-app/src/http/ | grep 'route("'`): 12 new `/api/v1/places_*` routes,
12 new Tauri commands, 1 new `cartridge_storage_place_ids` endpoint — **every one
maps to a register threat**; one legacy endpoint (`cartridges_suggest_location`)
was removed, net-reducing surface. See "Unregistered Flags" below.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| LAN browser → `POST /api/v1/places_*` | Any host on the LAN can reach these 12 routes when server mode is on. Identity is derived **server-side** from the `tower-sessions` session (`http/auth.rs:99 session_identity`), never from the request body — 12/12 handlers call it (`grep -c` = 12). Unauthenticated → `AppError::Unauthorized`. | place ids, names, kind tokens, search text |
| Tauri webview → `invoke("places_*")` | Local IPC; identical gate — both transports delegate to the same `build_places_*` helpers, which `authorize()` before touching the service. | same |
| Caller `place_id` (device/cartridge/act/report write + filter paths) | Caller-supplied integer FK. Existence enforced by `ON DELETE RESTRICT` FKs added in `V038` with `PRAGMA foreign_keys = ON` (`db/pragmas.rs:58,70`). No freeform place text reaches any write path. | integer ids only |
| CSV-import place-path text | Untrusted uploaded file content. Resolved in Rust against a server-fetched, non-archived `place_full_paths` candidate set (`device_service.rs:626-690`), exact case-insensitive match only. Never concatenated into SQL, never auto-creates a node. | free text |
| `place_path_snapshot` (D-16) | Computed **server-side only** from the validated `place_id` via `PlaceRepository::full_path`. No create/update DTO carries a snapshot field; no UI code constructs one. | derived text, server-authored |
| `place_full_paths` view | Read-only recursive-CTE VIEW over `places`; no user input reaches its definition. | derived paths |
| Existing portable DB → V037/V038 | An operator's real pre-Phase-39 DB runs these migrations on first launch. Schema-only migration; location data is intentionally dropped (locked decision). | operator DB file |

---

## Threat Register — Verification Result

**46 threats: 46 closed, 0 open.** 33 `mitigate` (mitigation located in code + test),
13 `accept` (rationale re-checked against the code as built).

### Mitigate (33)

| Threat ID | Category | Evidence | Status |
|-----------|----------|----------|--------|
| T-39-01-03 | DoS | `crates/trackly-infra/tests/migration_idempotency.rs:74 places_migration_drops_locations_and_adds_place_columns` asserts `locations` count == 0, dropped columns absent, `place_id`/`bulk_place_id`/`place_path_snapshot`/`place_id_override` present, `place_full_paths` view count == 1, over the full V001-V038 chain. **Executed: 2 passed, 0 failed.** | closed |
| T-39-02-01 | EoP | `crates/trackly-core/src/auth.rs:153` — `MutatePlaces` sits in the `matches!(identity.role, Role::Admin)` arm alongside `ManageUsers`/`ManageSettings`, with an explicit "do NOT move this into Admin\|Manager" comment. Unit test `auth.rs:389 authorize_manager_mutate_places_forbidden`. | closed |
| T-39-02-02 | Info Disclosure | `auth.rs:165` — `ReadPlaces` in the `Admin\|Manager` arm; Employee falls through to `Err(Forbidden)`. Unit test `auth.rs:410 authorize_employee_read_places_forbidden`; HTTP proof `role_endpoint_matrix.rs:1664-1691` (Case 47). | closed |
| T-39-02-03 | Tampering | `crates/trackly-core/src/domain/places.rs:37-52` — closed 6-arm match, `other => Err(AppError::Validation{field:"kind"})`. No `unwrap`, no default arm. Test `place_kind_from_str_unknown_lists_all_six_values_in_russian`. | closed |
| T-39-04-01 | Tampering (SQLi) | `crates/trackly-infra/src/repos/places_sqlite.rs` — every caller value bound via `rusqlite::params![]` (lines 79, 98, 145, 220, 266, 284, 313, 341, 380, 398, 416, 431, 470). The four `format!`-built SQL strings (78, 189, 310, 320/322) interpolate **only compile-time constants** (`SELECT_PLACES`, `cte`, `place_filter`) — verified by reading each site; zero caller-supplied values reach SQL text. | closed |
| T-39-04-02 | Tampering / integrity | Defense-in-depth confirmed both layers: `places_sqlite.rs:449-466` runs `subtree_stats_impl` inside the delete tx and returns `AppError::Conflict` on any non-zero count; `V038` FKs are `ON DELETE RESTRICT` and `db/pragmas.rs:58` sets `foreign_keys = ON`. **`places_crud.rs` (8 passed) + `places_delete_blocked.rs` (6 passed) executed green.** | closed |
| T-39-04-03 | Tampering (cycle) | `places_sqlite.rs:360-392` — recursive-CTE ancestor check runs as the first statement inside the same `conn.transaction()` as the `UPDATE`, `Some(new_parent)` only. Test `places_move_cycle.rs::move_into_own_descendant_is_rejected_with_ui_spec_copy` **PASS**; infra twin `places_crud.rs::move_node_into_own_descendant_rejected_as_validation_error` **PASS**. | closed |
| T-39-05-01 | EoP | `crates/trackly-app/src/services/place_service.rs` — `authorize(caller, &Action::MutatePlaces)?` is the **first statement** of all six mutations: `create:149`, `rename:201`, `move_node:259`, `archive:304`, `unarchive:310`, `delete_hard:370`. Test `places_service_crud.rs::create_forbidden_for_manager_before_any_db_write` **PASS**. | closed |
| T-39-05-02 | Tampering / integrity | `place_service.rs:372-400` — `subtree_stats` read on the reader pool before the writer is touched; `nested_places + device_count + cartridge_count + referencing_act_count > 0` → `AppError::Conflict`. No cascade, no reparenting anywhere in the file. **6/6 `places_delete_blocked.rs` PASS.** | closed |
| T-39-05-03 | Repudiation | `audit_repo.insert(... entity_type: "place" ...)` at `place_service.rs:174` (create), `229` (rename), `281` (move), `346` (`set_archived`, serving both archive and unarchive), `418` (delete). All six mutations covered. Test `places_service_crud.rs::create_inserts_place_and_audit_log` **PASS**. | closed |
| T-39-06-01 | EoP | Repo-wide sweep for the removed auto-create surface returns **zero matches** (excluding `node_modules`/`target`/`.git`/`.planning`): `resolve_location_id_in_tx`, `upsert_location_in_tx`, `INSERT OR IGNORE INTO locations`, `locations_autocomplete`. All three device write paths take a caller `place_id`: `device_service.rs:154` (create), `:256` (update), `:988` (bulk_create) — no name-based resolve. | closed |
| T-39-06-02 | Tampering | `device_service.rs:626-643` fetches `place_repo.list_all(conn, /*include_archived=*/false)` once and builds a `HashMap<full_path.to_lowercase(), place_id>`; `:670-690` does exact-key lookup, miss → `RowError` with UI-SPEC copy, `continue` (never silently ignored, never auto-created). Test `devices_csv_import.rs::import_commit_unresolved_place_reports_row_error_with_exact_copy` **PASS** (11/11 in binary). | closed |
| T-39-07-01 | Tampering | All four snapshot sites compute server-side via `places_repo.full_path(&tx, pid)`: `act_service.rs:286` (create), `:687` (update), `:1222` (return), `:2050` (update-return). Adversarial check: **no** create/update DTO carries a snapshot field — `place_path_snapshot` appears only on the read-side `ActDto` (`dto/act.rs:72`), never on `ActCreateDto:213` / `ActUpdateDto:265` / `ActReturnDto:151` / `ActUpdateReturnDto:334`. | closed |
| T-39-07-02 | Repudiation | Audit-log JSON keys renamed in lockstep: `"place_id"` + `"place_path_snapshot"` at `act_service.rs:492-493, 994, 1008, 2089, 2103`. Grep for surviving `"location_id"` / `"location"` JSON keys across all `services/*.rs` and `repos/*.rs` → **zero matches**. | closed |
| T-39-07-03 | Repudiation | Write side `act_service.rs:3054` emits `"place_id": row.place_id`; read side `devices_sqlite.rs:364` `snapshot.get("place_id").and_then(as_i64)` → bound into `place_id = ?10` of the restore UPDATE. Keys match; no orphaned `location_id` key on either side. | closed |
| T-39-08-01 | Tampering (SQLi) | `place_service.rs:563-604` — the query string is never placed in SQL. Candidate set comes from `repo.list_all` (no caller predicate); filtering is `full_path.to_lowercase().contains(&needle)` in Rust. Executor's own grep confirms `LIKE`/`GLOB` appear only in doc-comment prose in this file. | closed |
| T-39-08-02 | DoS | `place_service.rs:50 SEARCH_QUERY_MAX_CHARS = 100` enforced at `:566` (`chars().count()`, Cyrillic-safe) and `:53 SEARCH_RESULT_LIMIT = 50` enforced at `:596 .take(...)`. Tests `search_rejects_query_longer_than_100_chars`, `search_caps_results_at_50_rows` **PASS**. | closed |
| T-39-08-03 | Info Disclosure | `place_service.rs:578` calls `list_all(&conn, false)`; `places_sqlite.rs:322` maps `false` → `AND p.archived_at_utc IS NULL`. Test `places_search.rs::search_excludes_archived_place` **PASS**. `list_subtree_contents` deliberately unfiltered per D-15's literal scope. | closed |
| T-39-09-01 | EoP | Same zero-match sweep as T-39-06-01; `upsert_location_in_tx` has no surviving reference anywhere in the repository. | closed |
| T-39-09-03 | Tampering (SQLi) | `crates/trackly-infra/src/repos/cartridges_sqlite.rs:796-830` — place-path query text is compared in Rust against `SELECT place_id, full_path FROM place_full_paths` (no caller predicate); the resulting ids are bound as generated `?N` placeholders (`:822-830`), never interpolated as values. Tests `cartridges_place_search.rs` **4/4 PASS**. | closed |
| T-39-09-04 | Repudiation | `crates/trackly-app/src/specta_export.rs:83` registers `cartridge_storage_place_ids` in `collect_commands!`; `ui/src/bindings.ts` contains the generated binding (2 matches). HTTP twin at `http/cartridges.rs:451`. | closed |
| T-39-09-05 | EoP | `cartridge_service.rs::update` is now a single parameterized `UPDATE cartridges SET place_id=?1 …` (`:203-206`). No `INSERT OR IGNORE INTO locations` remains in the file (or repo). | closed |
| T-39-10-01 | Tampering (SQLi) | `report_service.rs:1061-1072, 1189-1200, 1670-1681, 1756-1767` — each place filter pushes the value into `owned_params` and emits a `?{idx}` placeholder inside the recursive-CTE prefix; storage-place clauses (`:1102, 1222, 1332, 1433, 1533, 1710, 1788`) are constant SQL over a CTE. No value concatenation introduced. | closed |
| T-39-11-01 | Tampering | Bulk/override values are plain `Option<i64>` FKs taken straight from the caller (`act_service.rs:1214, 1302, 1643, 1671`) and required-when-per-row by service validation (`:1101-1106, 1523-1528`); existence enforced by V038 `ON DELETE RESTRICT` + `foreign_keys=ON`. No new resolve/create surface. | closed |
| T-39-11-02 | Repudiation | `crates/trackly-app/tests/acts_place_snapshot.rs:248 rename_after_handover_does_not_alter_printed_snapshot` asserts the frozen snapshot and the live `place_full_paths` value **diverge** after an unrelated rename. **Executed: 4 passed, 0 failed.** | closed |
| T-39-12-01 | EoP | Gate lives in the shared helpers — `tauri_cmds/places.rs:37, 50, 62, 73, 84, 95` all `authorize(caller, &Action::MutatePlaces)?` before delegating. Both transports reach them: `http/places.rs:115-207` (12 handlers, each `session_identity` then `build_places_*`) and the thin `#[tauri::command]` wrappers. **`role_endpoint_matrix.rs` Case 45 (Manager → all 6 HTTP mutation routes → 403) and Case 48 (Manager Identity → all 6 `build_places_*` → Forbidden) executed green (1 passed, 0 failed).** | closed |
| T-39-12-02 | Info Disclosure | `tauri_cmds/places.rs:101, 111, 123, 135, 152, 167` — `authorize(caller, &Action::ReadPlaces)?` on all 6 read helpers. Case 47 (Employee → `places_list_all`/`places_get` → 403) and Case 46 (Manager → 200, proving the split is precise, not blanket) **PASS**. | closed |
| T-39-12-03 | Repudiation | `specta_export.rs:182-193` registers all 12 `places_*` commands; `specta_export.rs` is the **single** source feeding `main.rs:264 .invoke_handler(builder.invoke_handler())`, so registration == reachability. `cargo test -p trackly-app --test export_bindings` **executed: 1 passed, 0 failed** (the stale `device_location_id` assertion noted in `deferred-items.md` was superseded by Plan 22 and no longer exists). | closed |
| T-39-15-01 | Tampering | Import resolution never creates a node: `device_service.rs:670-690` matches against the pre-fetched tree and emits a per-row `RowError` on miss. Test `import_commit_unresolved_place_reports_row_error_with_exact_copy` **PASS**. *Implementation deviation (verified, stronger than declared):* resolution uses a server-side exact `full_path` HashMap rather than the `places_search` substring endpoint the plan named — exact match cannot fuzzily bind a row to an unintended place. | closed |
| T-39-17-01 | Tampering | UI forms transmit integers only: `ActFormBody.svelte:176, 206 place_id`, `ReturnModal.svelte:158 placeIdOverride`, `:278, 320 bulk_place_id`. Repo-wide grep for `place_path_snapshot`/`placePathSnapshot` in `ui/src` returns **only** generated `bindings.ts` type declarations — no UI code path constructs snapshot text. | closed |
| T-39-21-01 | DoS | Automated half: `migration_idempotency.rs::places_migration_drops_locations_and_adds_place_columns` **executed green**. Manual half: user-performed upgrade of a real pre-Phase-39 portable DB, recorded at `39-21-SUMMARY.md:164-170` ("performed live by the user against a real database — not simulated"). | closed |
| T-39-21-02 | Tampering | The backstop sweep was re-run independently by this audit, not taken on trust: zero matches repo-wide for `resolve_location_id_in_tx`, `upsert_location_in_tx`, `INSERT OR IGNORE INTO locations`, `locations_autocomplete`. Remaining `locations` tokens are confined to historical migrations V002-V005, V037/V038 comments, doc prose, and `migration_idempotency.rs`'s deliberate absence-assertions. | closed |
| T-39-22-01 | Tampering | Fixtures create real rows via `SqlitePlaceRepository::create` on the service writer: `devices_grouping.rs:31-50 create_place`, `acts_search.rs:197-215`, `acts_e2e_smoke.rs:184-192`. No fabricated integer literal is passed as a `place_id` in these fixtures. | closed |

### Accept (13)

Each rationale was re-checked against the code as built, not taken from the plan text.

| Threat ID | Category | Rationale verified against implementation | Status |
|-----------|----------|-------------------------------------------|--------|
| T-39-01-01 | Tampering | `place_full_paths` is a `CREATE VIEW` over `places` (`V037__places.sql`), read-only, no user input in its definition. Holds. | accepted |
| T-39-01-02 | Info Disclosure | `DROP TABLE locations` at `V038:57`, documented as a locked decision in `39-01-PLAN.md` `must_haves.truths` (line 20) and REQUIREMENTS Out of Scope. Migration is schema-only, not ETL — confirmed by reading V038 in full. Holds. | accepted |
| T-39-03-01 | Tampering | **Rationale amended — see "Accepted-risk corrections" below.** Existence validation holds (FK `ON DELETE RESTRICT` + `foreign_keys=ON`); the "and is not archived" half is not implemented anywhere and is not intended to be, per D-15. Residual risk is data hygiene inside an already-authorized role, not a boundary bypass. | accepted (amended) |
| T-39-06-03 | Info Disclosure | Device reads are gated by `Action::ReadData` (Admin\|Manager) — the same role set as `ReadPlaces`. The place-path join therefore discloses nothing an authorized device reader could not already see. Holds. | accepted |
| T-39-09-02 | Info Disclosure | `build_cartridge_storage_place_ids` calls `authorize(caller, &Action::ReadData)?` (`tauri_cmds/cartridges.rs:210`) on both transports (HTTP handler `http/cartridges.rs:399` delegates to the same helper) — Employee is rejected. Payload is bare `Vec<i64>` node ids, no PII. Holds. | accepted |
| T-39-10-02 | Info Disclosure | `RequestPrinterOptionDto` (`dto/request.rs:106-114`) is exactly `{id, name, place}`; the SQL at `request_service.rs:257-264` selects exactly `d.id, d.name, pfp.full_path`. No SNMP/community/IP/serial field widened. Holds. | accepted |
| T-39-13-01 | EoP | `PlacePicker.svelte:114 isAdmin` gates the create-row render at `:726`; the real boundary is `build_places_create`'s `MutatePlaces` gate, proven by role-matrix Cases 45/48. Holds. | accepted |
| T-39-14-01 | EoP | `PlaceTreeNode.svelte:181 {#if isAdmin}` wraps `ActionMenu`; drag-start is short-circuited by `PlaceTree.svelte:698 if (!isAdmin) return;` and `:461`. Server gate rejects regardless. Holds. | accepted |
| T-39-14-02 | DoS | `PlaceTree.svelte:231` performs one `places_list_all` per load; confirmed real scale ~300 rows. Holds. | accepted |
| T-39-16-01 | Tampering | `CartridgeTransitionOp::Install.place_id` is `Option<i64>` (`domain/cartridges.rs:113`); `None` is meaningful (D-07). Install's printer-derived default is server-computed from `devices.place_id` (`cartridge_service.rs:391-417`) and falls back to `None` on a miss rather than a forged default. Holds. | accepted |
| T-39-18-01 | Info Disclosure | Every report command gates `authorize(caller, &Action::ReadData)?` (`tauri_cmds/reports.rs:120-264, 584` — 15 call sites). Place filter adds no bypass. Holds. | accepted |
| T-39-19-01 | EoP | Modals are reachable only from Admin-gated entry points (`PlacesPage.svelte:211`, `PlaceTreeNode.svelte:181`); underlying `places_create`/`rename`/`move` are server-rejected for non-Admin. Holds. | accepted |
| T-39-20-01 | Info Disclosure | `build_places_contents` gates `Action::ReadPlaces` (`tauri_cmds/places.rs:152`) and the service re-gates at `place_service.rs:543`. Manager sees the same entities it already sees in device/cartridge lists. Holds. | accepted |

---

## Accepted-risk corrections

**T-39-03-01 — plan rationale overstates what was built.** The register defers
"validating that a supplied `place_id` actually exists **and is not archived**" to
Plan 09. Existence is enforced (FK `ON DELETE RESTRICT`, V038 + `foreign_keys=ON`).
The archived check does **not** exist: `grep -n "archived" cartridge_service.rs
device_service.rs` returns exactly one hit, a comment on the CSV candidate-set
fetch. This is consistent with D-15 (`39-CONTEXT.md:103-106`), which defines
archival as *hiding the node from `PlacePicker`* — not as a write constraint — so
the code matches the product decision and the register's wording is what is wrong.

Residual risk, accepted: an already-authorized Admin/Manager calling the API
directly (bypassing `PlacePicker`, which filters archived nodes) can assign an item
to an archived place. Impact is data hygiene, not confidentiality, integrity of
another tenant's data, or privilege escalation. **Not a blocker.** If archived
places should ever become write-forbidden, that is a new product decision requiring
its own threat entry, not a Phase 39 gap.

---

## Unregistered Flags

**None.** No `## Threat Flags` section exists in any of the 22 SUMMARY files, so
that channel supplied nothing. Rather than treat silence as "no new surface", new
surface was recomputed from the diff:

| New surface (diff-derived) | Mapped threat |
|----------------------------|---------------|
| 12 × `POST /api/v1/places_*` HTTP routes | T-39-12-01 / T-39-12-02 |
| 12 × `places_*` Tauri commands | T-39-12-01 / T-39-12-02 / T-39-12-03 |
| `POST /api/v1/cartridge_storage_place_ids` + Tauri twin | T-39-09-02 / T-39-09-04 |
| `places` table, `place_full_paths` view, 6 new FK columns | T-39-01-01 / T-39-01-02 / T-39-01-03 / T-39-04-02 |
| Removed: `cartridges_suggest_location` route, three auto-create code paths | net surface reduction (T-39-21-02) |

---

## Non-blocking observations (outside this register)

Recorded for traceability; none is a Phase 39 mitigation gap, none blocks ship.

1. **Pre-existing FTS interpolation in `cartridges_sqlite.rs:842`.** The FTS5
   `MATCH` phrase is string-interpolated (`fts_query_escaped`, double-quote
   escaped) rather than bound. This predates Phase 39 and is covered by the
   existing `T-04-02-01` mitigation; Phase 39's own change to that method (the
   place-path branch) is fully parameterized. Flagged only so it is not mistaken
   for new surface.
2. **Code-review items deferred to milestone debt** (`39-REVIEW.md:296-409`):
   WR-01 stale content-count badges, WR-02 unordered debounced-search responses,
   IN-01 dead `PlaceTreeNodeDto`, IN-02 case-sensitive D-04 sibling uniqueness.
   None maps to a register threat; WR-02 and IN-02 are UX/data-hygiene, not
   authorization or injection.

---

## Audit trail

Tests executed by this audit (not accepted from SUMMARY/VERIFICATION narration):

| Command | Result |
|---------|--------|
| `cargo test -p trackly-app --test role_endpoint_matrix --test export_bindings -- --test-threads=1` | 1 + 1 passed, 0 failed |
| `cargo test -p trackly-app --test places_delete_blocked --test places_move_cycle --test places_search --test places_service_crud --test places_contents --test acts_place_snapshot --test devices_csv_import` | 6+2+5+4+3+4+11 = 35 passed, 0 failed |
| `cargo test -p trackly-infra --test migration_idempotency --test places_crud --test devices_place_search --test cartridges_place_search` | 2+8+5+4 = 19 passed, 0 failed |

Greps executed (repo-wide, excluding `node_modules`/`target`/`.git`/`.planning`):
`resolve_location_id_in_tx`, `upsert_location_in_tx`, `INSERT OR IGNORE INTO
locations`, `locations_autocomplete` → **0 matches**.

Implementation files were **not modified** by this audit. Only this file was written.

---

## Security Audit 2026-08-26

| Metric | Count |
|--------|-------|
| Threats found | 46 |
| Closed | 46 |
| Open | 0 |

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-08-26 | 46 | 46 (33 mitigate verified + 13 accept re-checked) | 0 | gsd-security-auditor |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented (see "Accept (13)" table and "Accepted-risk corrections")
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-08-26
