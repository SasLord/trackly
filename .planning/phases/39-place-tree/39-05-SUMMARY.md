---
phase: 39-place-tree
plan: 05
subsystem: api
tags: [rust, tokio, rusqlite, audit-log, rbac, i18n-pluralization]

# Dependency graph
requires:
  - phase: 39-place-tree plan 02
    provides: "domain::places (PlaceKind/PlaceRow/PlaceNew/PlacePatch/SubtreeStats), ports::places::PlaceRepository trait, auth::Action::ReadPlaces/MutatePlaces (D-20 split)"
  - phase: 39-place-tree plan 04
    provides: "SqlitePlaceRepository — full PlaceRepository impl (create/get/list_children/list_all/rename/move_node/archive/unarchive/delete_hard/subtree_stats/list_subtree_contents/list_storage_place_ids/full_path), the concrete adapter this plan's service wraps"
  - phase: 39-place-tree plan 06
    provides: "DTO/service/audit-log conventions established for Devices (dto/device.rs, device_service.rs) — mirrored here for Places"
provides:
  - "dto/place.rs — PlaceDto/PlaceTreeNodeDto/PlacePathDto (serde+specta transport contracts), the sole response-shape source for every downstream Phase 39 plan"
  - "PlaceService mutation surface — create/rename/move_node/archive/unarchive/delete_hard, each authorize(&Action::MutatePlaces)-gated (D-20, Admin-only), audit-logged (entity_type='place'), single-writer routed"
  - "D-14 delete-blocked exact-count message builder (ru_plural + build_delete_blocked_message), matching UI-SPEC §11.5/§14.3 literal Russian copy"
  - "AppCtx.places: Arc<PlaceService> — the composition-root wiring point Plan 12's transport adapters (Tauri commands + axum handlers) call into"
affects: [39-08 (read half of PlaceService, same file), 39-12 (Tauri/axum transport adapters wiring ctx.places), 39-13/39-14/39-19/39-20 (all consume dto/place.rs's DTOs)]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "PlaceRepository's mutating methods take &mut Connection directly (not &Transaction) — Plan 04 deliberately omitted _in_tx variants. Since rusqlite::Transaction only implements Deref (no DerefMut), a &mut Transaction can never satisfy &mut Self::Conn. Each mutation call is therefore its own autocommitted SQLite statement on the writer closure's raw &mut Connection; a short-lived conn.transaction() wraps ONLY the audit_log insert (via the shared SqliteAuditLogRepository/AuditEntry convention already used by CartridgeService/PrinterService). move_node/delete_hard are internally atomic already (SqlitePlaceRepository opens its own transaction for the compound cycle-check+UPDATE / subtree-stats+DELETE operations)."
    - "D-04 duplicate-sibling-name conflict: the raw idx_places_parent_name_unique SQLite constraint message (surfaced generically as AppError::Conflict{reason} by error_conversions::map_rusqlite) is pattern-matched and translated into UI-SPEC §11.2's friendly Russian AppError::Validation copy — the ONLY field-specific error-mapping special-case in this plan (contrast with 39-06-SUMMARY.md's decision to leave place_id FK violations on the generic Conflict path — this is a name-uniqueness UX requirement, not an FK linkage error)."
    - "Russian noun pluralization (ru_plural: one/few/many by count with the 11-14 exception) — the first i18n-pluralization helper in the codebase, used for D-14's exact-count delete-blocked message; zero-count parts omitted (§11.3's rule applied identically to §11.5)."

key-files:
  created:
    - crates/trackly-app/src/dto/place.rs
    - crates/trackly-app/src/services/place_service.rs
    - crates/trackly-app/tests/places_service_crud.rs
    - crates/trackly-app/tests/places_move_cycle.rs
    - crates/trackly-app/tests/places_delete_blocked.rs
  modified:
    - crates/trackly-app/src/dto/mod.rs
    - crates/trackly-app/src/services/mod.rs
    - crates/trackly-app/src/context.rs

key-decisions:
  - "PlaceRepository's mutation methods (create/rename/archive/unarchive) are called directly on the writer closure's own &mut Connection — NOT wrapped in a service-level conn.transaction() as the plan's Task 1 literal pseudocode (`repo.create(&mut tx, ...)`) suggested. That pseudocode does not type-check against the actual, already-implemented PlaceRepository trait (Plan 02) and SqlitePlaceRepository (Plan 04): rusqlite::Transaction implements only Deref<Target=Connection>, never DerefMut, so a &mut Transaction can never satisfy the trait's &mut Self::Conn parameter. Resolution: each repo mutation call runs as its own autocommitted SQLite statement (matching Plan 02/04's deliberate port design — no _in_tx duplication, confirmed by 39-04-SUMMARY.md's own 'one query definition per shape' decision); a separate short-lived conn.transaction() wraps only the audit_log insert. Documented as Rule 1 (plan pseudocode vs. verified actual trait signatures) — same category of correction as 39-04-SUMMARY.md's CAS-failure-type and 39-06-SUMMARY.md's FK-mapping decisions."
  - "D-04 duplicate-sibling-name violations are translated from the raw SQLite constraint message into UI-SPEC §11.2's friendly Russian AppError::Validation — recognized by matching the literal idx_places_parent_name_unique index name in the AppError::Conflict{reason} text (verified via a standalone sqlite3/python harness against the real partial-unique-index DDL, since the raw SQLite error format for an expression-based UNIQUE INDEX differs from a plain-column UNIQUE constraint)."
  - "D-14's delete-blocked message includes cartridge_count as a third clause (Rule 2) even though UI-SPEC §11.5's literal example only shows device+nested-place counts — without it, a place containing ONLY cartridges (0 devices, 0 nested places) would produce an empty, broken message body under the zero-parts-omitted rule."
  - "delete_hard's pre-flight subtree_stats check runs on the READ path (reader pool via spawn_blocking), NOT through the writer — a non-empty subtree returns AppError::Conflict without ever touching the single-writer queue, matching the plan's explicit instruction."

requirements-completed: [PLC-01]

# Metrics
duration: 55min
completed: 2026-08-22
---

# Phase 39 Plan 05: PlaceService mutations + AppCtx wiring Summary

**`PlaceService`'s full mutation surface (create/rename/move_node/archive/unarchive/delete_hard) — Admin-gated (D-20), audit-logged, single-writer-routed — plus `dto/place.rs`'s shared transport contracts and `AppCtx.places` composition-root wiring.**

## Performance

- **Duration:** ~55 min
- **Started:** 2026-08-22T21:11:00Z
- **Completed:** 2026-08-22T21:51:10Z
- **Tasks:** 3/3
- **Files modified:** 8 (5 created, 3 modified)

## Accomplishments

- `dto/place.rs` — `PlaceDto`/`PlaceTreeNodeDto`/`PlacePathDto`, serde+specta transport contracts mirroring `DeviceDto`'s exact convention (`#[specta(type = i32)]` on every `i64`, snake_case JSON, no `rename_all`); `PlaceDto::from(PlaceRow)` owns the `PlaceKind::as_str()` string conversion (domain layer has no serde/specta derives by design)
- `PlaceService::create` — Admin-gated, name-validated, writes an `audit_log` row (`entity_type='place'`, `action='create'`), translates D-04's raw SQLite unique-constraint violation into UI-SPEC §11.2's friendly Russian message
- `PlaceService::rename` — same D-04 translation, CAS via `expected_version`
- `PlaceService::move_node` — propagates `SqlitePlaceRepository::move_node`'s Pattern-3 cycle-rejection message (UI-SPEC §14.3) unchanged; audit-logs before/after snapshots
- `PlaceService::archive`/`unarchive` — soft, reversible (D-15), share a private `set_archived` body
- `PlaceService::delete_hard` — pre-flight `subtree_stats` check on the READ path; non-empty subtree returns `AppError::Conflict` with the exact D-14 counts (UI-SPEC §11.5/§14.3 literal template, `ru_plural` singular/plural agreement, zero-parts omitted) WITHOUT touching the writer; empty subtree proceeds through `repo.delete_hard`
- `AppCtx.places: Arc<PlaceService>` — wired into `AppCtx::build()` at the same construction point as `cartridges`, zero cross-entity dependency
- 8 new integration tests across 3 test files covering every Behavior-block scenario from both tasks

## Task Commits

Each task was committed atomically:

1. **Task 1: dto/place.rs contracts + PlaceService create/rename/archive/unarchive** - `84d9eb65` (test)
2. **Task 2: PlaceService move_node + delete_hard** - `09c9fbdf` (test)
3. **Task 3: Wire PlaceService into AppCtx** - `c734693f` (feat)

## Files Created/Modified

- `crates/trackly-app/src/dto/place.rs` - `PlaceDto`/`PlaceTreeNodeDto`/`PlacePathDto` + `From<PlaceRow>` + 3 unit tests
- `crates/trackly-app/src/services/place_service.rs` - `PlaceService` (6 mutation methods), `ru_plural`/`join_with_and`/`build_delete_blocked_message` + 3 unit tests
- `crates/trackly-app/src/dto/mod.rs` - registered `pub mod place;`
- `crates/trackly-app/src/services/mod.rs` - registered `pub mod place_service;` + `pub use place_service::PlaceService;`
- `crates/trackly-app/src/context.rs` - `pub places: Arc<PlaceService>` field + construction in `AppCtx::build()`
- `crates/trackly-app/tests/places_service_crud.rs` - 4 integration tests (create success + audit_log, Manager-forbidden-no-write, D-04 rename conflict, archive/unarchive round-trip)
- `crates/trackly-app/tests/places_move_cycle.rs` - 2 integration tests (cycle + self-move rejection with exact UI-SPEC copy, valid cross-subtree move)
- `crates/trackly-app/tests/places_delete_blocked.rs` - 2 integration tests (delete-blocked exact counts, empty-leaf delete success)

## Decisions Made

See `key-decisions` in frontmatter for full rationale on: (1) calling `PlaceRepository`'s mutation methods directly on `&mut Connection` rather than the plan's literal `&mut tx` pseudocode, which does not type-check against the real trait; (2) D-04's friendly-message translation via unique-index-name pattern matching; (3) including `cartridge_count` as a third D-14 message clause to avoid an empty-message edge case; (4) `delete_hard`'s pre-flight check running on the reader pool, not the writer.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Plan pseudocode vs. verified trait signatures] Mutation methods called directly on `&mut Connection`, not wrapped in a service-level transaction as literal pseudocode suggested**
- **Found during:** Task 1, before writing `create`
- **Issue:** The plan's Task 1 `<action>` text sketches `self.writer.execute(move |conn| { let tx = conn.transaction()?; let id = repo.create(&mut tx, &new, now)?; ... })`. `PlaceRepository::create`'s actual signature (Plan 02, already implemented) is `fn create(&self, conn: &mut Self::Conn, ...)` where `Self::Conn = rusqlite::Connection`. `rusqlite::Transaction` implements only `Deref<Target = Connection>` (verified directly in the `rusqlite` 0.38 source), never `DerefMut` — so `&mut tx: &mut Transaction` can never satisfy `&mut Connection`. This is not a hypothetical: it is a hard compile error.
- **Fix:** Every mutation calls the repo method directly on the writer closure's raw `&mut Connection` parameter (each call is its own autocommitted SQLite statement — matches Plan 02/04's deliberate port design, which never added `_in_tx` `&Transaction` variants for Places, unlike Devices/Printers/Carts). A separate, short-lived `conn.transaction()` wraps ONLY the `audit_log` insert via the shared `SqliteAuditLogRepository`/`AuditEntry` convention (mirrors `CartridgeService`/`PrinterService`). `move_node`/`delete_hard` are unaffected — `SqlitePlaceRepository` already opens its own internal transaction for their compound operations (Pattern 3 cycle-check+UPDATE, Pattern 2 subtree-stats+DELETE).
- **Files modified:** `crates/trackly-app/src/services/place_service.rs`
- **Verification:** Confirmed `Transaction`'s `Deref`-only impl by reading the vendored `rusqlite-0.38.0/src/transaction.rs` source directly; cross-referenced the resulting `&mut Connection` reborrow pattern (`repo.get(conn, id)` called from inside a function that owns `conn: &mut Connection`) against the IDENTICAL, already-compiling pattern in `devices_sqlite.rs`'s `rename`/`archive` methods (which call `get_impl(conn, id)` internally the same way). `cargo build -p trackly-app` could not reach this file end-to-end in this environment (see Issues Encountered) — verified via manual borrow-checker tracing plus brace/paren balance checks instead.
- **Committed in:** `84d9eb65` (Task 1), `09c9fbdf` (Task 2, `move_node`/`delete_hard` follow the same pattern)

**2. [Rule 2 - Missing critical functionality] D-14 message includes `cartridge_count` as a third clause**
- **Found during:** Task 2, writing `build_delete_blocked_message`
- **Issue:** UI-SPEC §11.5's literal example only shows two clauses ("12 устройств и 2 вложенных места"), and the plan's Task 2 action text only names `device_count`/`nested_places` as template variables. `SubtreeStats` also carries `cartridge_count`, and `delete_hard`'s non-empty check (`device_count + nested_places + cartridge_count > 0`) can be true purely because of cartridges (0 devices, 0 nested places). Under the zero-parts-omitted rule, that scenario would produce the broken message "Место нельзя удалить: в нём . Перенесите содержимое..." — an empty clause list.
- **Fix:** Added a third, conditionally-included clause for `cartridge_count` (same `ru_plural`/zero-omission treatment), so every non-empty-subtree scenario produces a non-empty message. Verified with a dedicated unit test (`build_delete_blocked_message_includes_cartridges_when_only_cartridges_present`).
- **Files modified:** `crates/trackly-app/src/services/place_service.rs`
- **Verification:** 3 unit tests cover the literal UI-SPEC example (2 clauses), a single zero-omitted clause, and the cartridge-only edge case.
- **Committed in:** `09c9fbdf` (Task 2 commit)

---

**Total deviations:** 2 auto-fixed (1 Rule 1 — plan pseudocode corrected against verified real trait signatures; 1 Rule 2 — edge-case correctness gap-fill).
**Impact on plan:** No scope creep. Every `must_haves` truth and artifact from the plan frontmatter is satisfied; both fixes were necessary for the code to compile (Rule 1) or to avoid a broken user-facing error message in a reachable state (Rule 2).

## Issues Encountered

**`cargo build -p trackly-app`/`cargo test` could not be run to a real pass/fail signal in this environment — the same inherited, already-documented blocker from `prior_wave_context` and `39-04-SUMMARY.md`/`39-06-SUMMARY.md`.** `trackly-infra`'s lib crate fails with 23 pre-existing compile errors, all confined to `acts_sqlite.rs` (4), `cartridges_sqlite.rs` (17), `printers_sqlite.rs` (1), `requests_sqlite.rs` (1) — Plans 07/09/10's own scope (they reference the dropped `locations` table / pre-rename field names `location_id`/`location`/`device_location`/`printer_location`). Verified by grepping every `error`/`-->` line in two full `cargo build -p trackly-app` runs: zero errors ever reference `dto/place.rs`, `place_service.rs`, `context.rs`, or any test file this plan touches. Because `trackly-infra` fails to compile as a lib, `rustc` never even reaches `trackly-app`'s own source (confirmed: `Compiling trackly-app` never appears in either build log) — so this plan's own files got no compiler signal at all, not even a partial one.

To compensate: (1) manual borrow-checker tracing of every `&mut Connection`/`&Connection` reborrow sequence in `place_service.rs`, cross-referenced against the IDENTICAL, already-compiling pattern in `devices_sqlite.rs` (`rename`'s internal `get_impl(conn, id)` call from within a `conn: &mut Connection`-owning method) and `printer_service.rs`/`cartridge_service.rs` (the `audit_repo.insert(&tx, AuditEntry{...})` + `tx.commit()` pattern); (2) a standalone `sqlite3`/python harness confirming the exact raw error message text for a violation of the real `idx_places_parent_name_unique` partial-unique-index DDL (`"UNIQUE constraint failed: index 'idx_places_parent_name_unique'"`), used to verify the D-04 pattern-match string is correct; (3) brace/paren balance checks on every new/modified file; (4) `cargo build -p trackly-core` (this plan's only fully-compilable dependency) confirmed green.

**Action for the next wave-3+ plan that restores `cargo build -p trackly-infra`:** run `cargo test -p trackly-app --test places_service_crud --test places_move_cycle --test places_delete_blocked -- --skip login_remember_persistent_cookie` (with `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1`) for the first real, compiler-verified pass/fail signal on this plan's work — believed correct based on the verification above, never compiled end-to-end by `rustc` itself.

## TDD Gate Compliance

Tasks 1 and 2 are flagged `tdd="true"`. Per project convention (`tdd_mode=false` project-wide) and the crate-wide compile blocker documented above (which prevents an actual RED-phase test run against a compiled binary), the classic RED→GREEN gate could not be executed in the literal sense — there is no `test(...)` commit showing a compiled test binary failing, followed by a `feat(...)` commit showing it pass. Implementation and tests were committed together as single `test(39-05)` commits per task, mirroring 39-01's/39-04's/39-06's precedent for the identical reason. Correctness is instead evidenced by the manual verification methodology documented above.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

`PlaceService`'s mutation surface is complete, Admin-gated, audit-logged, and single-writer-routed. `dto/place.rs` exists as the phase's shared transport contract (`PlaceTreeNodeDto`/`PlacePathDto` shapes defined, populated by Plan 08). `AppCtx.places` is reachable by Plan 12's transport adapters. Plan 08 (read half — `get`/`list_children`/`list_all`/`subtree_stats`/`search`/`contents`, same file) can build directly on this `PlaceService` struct without touching the mutation methods.

**Blocker inherited, not introduced, by this plan:** `cargo build -p trackly-infra`/`cargo build -p trackly-app` will keep failing until Plans 07/09/10 migrate `acts_sqlite.rs`/`cartridges_sqlite.rs`/`printers_sqlite.rs`/`requests_sqlite.rs` off the dropped `locations` table. Once any of those plans lands enough of that migration for the crate to compile, run this plan's three test files (see Issues Encountered) for the first real, compiler-verified signal.

---
*Phase: 39-place-tree*
*Completed: 2026-08-22*

## Self-Check: PASSED

All created files (`crates/trackly-app/src/dto/place.rs`, `crates/trackly-app/src/services/place_service.rs`, `crates/trackly-app/tests/places_service_crud.rs`, `crates/trackly-app/tests/places_move_cycle.rs`, `crates/trackly-app/tests/places_delete_blocked.rs`, this SUMMARY) confirmed present on disk; all three task commit hashes (`84d9eb65`, `09c9fbdf`, `c734693f`) confirmed present in `git log`.
