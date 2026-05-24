---
phase: 01-foundation
plan: 03
subsystem: infra
tags: [rust, sqlite, rusqlite, refinery, migrations, schema, wal, pragma, fts5, audit-log, integration-tests]

requires:
  - 01-01 (workspace, rusqlite 0.38, refinery 0.9, MSRV 1.88, tempfile workspace-dep)
  - 01-02 (AppError stub with Internal + Validation variants; Paths::resolve test seam)
provides:
  - migrations/V001..V012 — full v1 schema as 12 forward-only refinery SQL files, each ending PRAGMA user_version = N
  - trackly_infra::db::pragmas::apply_writer_pragmas (WAL + sync=NORMAL + busy_timeout=5000 + FK=ON + wal_autocheckpoint=1000 + temp_store=MEMORY + mmap_size=128 MiB)
  - trackly_infra::db::pragmas::apply_reader_pragmas (query_only=ON + FK=ON + busy_timeout=5000 + temp_store=MEMORY + mmap_size=128 MiB)
  - trackly_infra::db::migrations::run(&mut Connection) -> MigrationReport { schema_version, applied_count } via refinery::embed_migrations!("../../migrations")
  - trackly_infra::test_support::test_db() -> (rusqlite::Connection, tempfile::TempDir) — canonical fixture for every integration test in Plans 03/04/05/06
  - 13 integration tests + 5 lib unit tests locking the schema invariants in CI
affects: [04-appctx-writer, 05-tauri-specta-axum, 06-procmon-ci, all-future-phases]

tech-stack:
  added: []  # All deps already pinned in Plan 01-01 (rusqlite 0.38, refinery 0.9, tempfile 3)
  patterns:
    - "Per-connection PRAGMA application (NOT inside migration SQL) — writer pragmas applied before refinery transaction starts so journal_mode=WAL persists into file header via the first migration's writes"
    - "embed_migrations! at compile time (../../migrations relative to trackly-infra/Cargo.toml) — no separate migrations/ folder needs to ship beside the .exe in portable mode"
    - "Refinery default per-migration transaction (NOT set_grouped(false)) — confirmed by idempotency test; second run reports applied_count=0"
    - "test_support is pub (NOT #[cfg(test)]) so downstream crates' integration tests can use the fixture"
    - "Schema-invariant tests as CI gates — any future migration that violates D-Schema-03/04 fails per_record_invariants test on the next PR"
    - "Allowlist pattern in invariant tests for legitimate exceptions (act_items.*_at_time TEXT snapshots; sessions.expiry_date tower-sessions convention)"

key-files:
  created:
    - migrations/V001__init_pragmas_and_lookups.sql
    - migrations/V002__core_entities.sql
    - migrations/V003__devices.sql
    - migrations/V004__acts.sql
    - migrations/V005__cartridges.sql
    - migrations/V006__requests.sql
    - migrations/V007__document_templates.sql
    - migrations/V008__audit_log.sql
    - migrations/V009__counters.sql
    - migrations/V010__sessions.sql
    - migrations/V011__scheduled_tasks.sql
    - migrations/V012__indexes_and_fts.sql
    - crates/trackly-infra/src/db/mod.rs
    - crates/trackly-infra/src/db/pragmas.rs
    - crates/trackly-infra/src/db/migrations.rs
    - crates/trackly-infra/src/test_support/mod.rs
    - crates/trackly-infra/src/test_support/test_db.rs
    - crates/trackly-infra/tests/seed_data.rs
    - crates/trackly-infra/tests/per_record_invariants.rs
    - crates/trackly-infra/tests/audit_log_schema.rs
    - crates/trackly-infra/tests/migration_idempotency.rs
  modified:
    - crates/trackly-infra/Cargo.toml (tempfile moved from [dev-dependencies] to [dependencies])
    - crates/trackly-infra/src/lib.rs (pub mod db; pub mod test_support)

key-decisions:
  - "embed_migrations!(\"../../migrations\") — relative to trackly-infra/Cargo.toml; macro is the refinery 0.9 form; verified to compile by `cargo build -p trackly-infra` on first attempt"
  - "PRAGMA journal_mode=WAL applied via pragma_update_and_check (not pragma_update) — SQLite returns the actual journal mode it switched to, which we validate equals 'wal' case-insensitively; this catches the rare case where WAL fails to engage (e.g., on a read-only mount)"
  - "MigrationReport { schema_version: u32, applied_count: usize } — schema_version is u32 (SQLite stores user_version as i64; we narrow with try_into and report Internal on overflow)"
  - "Reader PRAGMA includes query_only=ON (per <interfaces>) — prevents accidental writes through reader-pool connections; Plan 04's read pool will inherit this"
  - "test_db() panics on any error — test infrastructure, failures here mean the harness itself is broken; production code paths use Result<MigrationReport, AppError> as the contract"
  - "audit_log.id column test relaxed to not_null=false — SQLite's PRAGMA table_info reports notnull=0 on INTEGER PRIMARY KEY columns (implicitly non-null via rowid alias). Documented in-comment in audit_log_schema.rs"
  - "Non-_at_utc timestamp-lookalike allowlist: act_items.condition_at_time + act_items.complectation_at_time (TEXT state snapshots, NOT timestamps); sessions.expiry_date (tower-sessions canonical column name, but still asserted INTEGER)"
  - "Refinery default per-migration transaction kept (NOT set_grouped) — confirmed by idempotency test: second run on same conn returns applied_count=0; reopen + third run also returns 0; PRAGMA journal_mode='wal' on reopen proves Pitfall #4 mitigated"

requirements-completed: [FOUND-03, FOUND-07, FOUND-08, FOUND-09, FOUND-10]

duration: ~6 min
completed: 2026-05-24
---

# Phase 1 Plan 03: Schema + V001..V012 refinery migrations + PRAGMA discipline + test fixtures Summary

**Full v1 SQLite schema authored as 12 forward-only refinery migrations (V001..V012) — every entity from devices/acts/cartridges through audit_log/counters/sessions/scheduled_tasks plus FTS5 virtual tables with `unicode61 remove_diacritics 2` for Cyrillic-aware search; per-connection PRAGMA discipline (`apply_writer_pragmas` sets WAL + sync=NORMAL + busy_timeout=5000 + FK=ON + wal_autocheckpoint=1000 + temp_store=MEMORY + mmap_size=128 MiB, applied before refinery's first transaction so WAL persists into the file header per Pitfall #4); refinery wrapper via `embed_migrations!("../../migrations")` returning `MigrationReport { schema_version, applied_count }`; tempfile-backed `test_db()` public fixture; 13 integration tests + 5 lib unit tests lock D-Schema-02/03/04/05 invariants in CI.**

## Performance

- **Duration:** ~6 min wall clock
- **Started:** 2026-05-24T23:11Z
- **Completed:** 2026-05-24T23:17Z
- **Tasks:** 3 / 3
- **Files created:** 21
- **Files modified:** 2

## Accomplishments

- **12 refinery migrations authored** covering the complete v1 schema:
  - V001 (lookups + seeds): `device_types` [`Устройство`, `Принтер`]; `device_statuses` [`На складе`, `В работе`, `На ремонте`, `Списано`]; `cartridge_states` [`Полный`, `Частичный`, `Пустой`]; `cartridge_statuses` [`На складе`, `В работе`, `На заправке`, `Списано`].
  - V002 — `users` (login + role + password_hash NULL for AD users + ad_user 0/1) + `locations` (name + kind + address).
  - V003 — `devices` (FK → device_types, device_statuses, locations; standard4).
  - V004 — `acts` (number + sub_number + parent_act_id self-FK ON DELETE RESTRICT; CHECK act_type IN ('handover','return'); partial unique index on (number, COALESCE(sub_number, 0)) WHERE deleted_at_utc IS NULL) + `act_items` junction (NO standard4).
  - V005 — `cartridge_models` (partial unique on (brand, model)) + `cartridge_model_compatibility` junction + `cartridges` (code TEXT UNIQUE, status_id, state_id, holder_name).
  - V006 — `requests` (CHECK request_type IN ('cartridge_replace','free_form','ad_register'); CHECK status IN ('open','in_progress','completed','rejected'); requested_by/assigned_to FKs).
  - V007 — `document_templates` (CHECK kind IN ('act_handover','act_acceptance'); partial unique on (kind) WHERE is_active=1 AND deleted_at_utc IS NULL).
  - V008 — `audit_log` (hard-delete; D-Schema-05 columns).
  - V009 — `counters` (seed: `act_number=0`, `cartridge_seq=0`).
  - V010 — `sessions` (BLOB pk; tower-sessions backing schema).
  - V011 — `scheduled_tasks` (Phase 7 supervisor placeholder).
  - V012 — 14 cross-table indexes + 3 FTS5 virtual tables (`devices_fts`, `acts_fts`, `cartridges_fts`) with `unicode61 remove_diacritics 2` tokenizer.

- **PRAGMA discipline locked.** `apply_writer_pragmas` runs `journal_mode=WAL` first via `pragma_update_and_check` (validates the engine actually switched to WAL), then `synchronous=NORMAL`, `busy_timeout=5000`, `foreign_keys=ON`, `wal_autocheckpoint=1000`, `temp_store=MEMORY`, `mmap_size=128 MiB`. `apply_reader_pragmas` drops `journal_mode` and `synchronous` (write-only) and adds `query_only=ON` to prevent accidental writes through reader-pool connections.

- **Refinery wired.** `embed_migrations!("../../migrations")` from `crates/trackly-infra/src/db/migrations.rs` resolves to the workspace-root `migrations/` directory; refinery 0.9 `Runner::run` is the canonical invocation; we read back `PRAGMA user_version` after refinery returns and pack it into `MigrationReport { schema_version: u32, applied_count: usize }`.

- **`test_db()` fixture is public** (NOT `#[cfg(test)]`) so downstream consumers (`trackly-app/tests/*` in Plans 04/05) can use it without re-deriving the schema setup. Returns `(Connection, TempDir)` — caller drops the guard last.

- **18 tests green** (`cargo test -p trackly-infra`):
  - 5 lib unit tests: writer pragmas, reader pragmas, fresh-run applies 12, idempotent on same conn, test_db() helper.
  - 4 audit_log_schema tests: columns present + types, two indexes exist, both indexes cover correct column order.
  - 3 per_record_invariants tests: 8 user-mutable tables have standard4; 10 system+junction tables lack deleted_at_utc/version; every `_at_utc` column is INTEGER and no other column carries a timestamp-shaped suffix (allowlist documented in-source).
  - 1 migration_idempotency test: fresh DB → 12 applied → re-run same conn → 0 applied → close → reopen + apply_writer_pragmas → run → 0 applied AND `journal_mode='wal'` (Pitfall #4 resolved).
  - 5 seed_data tests: every lookup table seeded with exact Russian strings; counters seeded with both names + zero values.

- **`cargo clippy --workspace --all-targets -- -D warnings`** still passes.
- **`cargo build --workspace`** still succeeds.

## Task Commits

1. **Task 1:** `feat(01-03): author V001..V012 refinery migrations covering full v1 schema` — `c019ccb`
2. **Task 2:** `feat(01-03): add db::pragmas + db::migrations + test_support::test_db` — `d7d8799`
3. **Task 3:** `test(01-03): add schema-invariant integration tests (D-Schema-02/03/04/05)` — `ace4b28`

_Final plan-metadata commit will be added by the orchestrator after this SUMMARY is written._

## Decisions Made

See `key-decisions` frontmatter for the full list. Most impactful for downstream plans:

- **`embed_migrations!("../../migrations")`** — Plan 04 / 05 do NOT need to repeat this; they consume `trackly_infra::db::migrations::run` directly.
- **`MigrationReport { schema_version: u32, applied_count: usize }`** — the **public surface** Plan 04's `AppCtx::build` will use to implement downgrade protection (compare `schema_version` against the constant `12` and refuse to open if the file's version exceeds it).
- **`apply_writer_pragmas` / `apply_reader_pragmas` signatures** — both take `&Connection` and return `Result<(), AppError>`. Plan 04's writer worker calls `apply_writer_pragmas` once at construction; Plan 04's read pool calls `apply_reader_pragmas` on every connection it hands out.
- **`test_db() -> (Connection, TempDir)`** — Plan 04's `concurrent_writes.rs` and Plan 05's `health_smoke.rs` use this as-is.
- **`schema_version = 12` is the constant** Plan 04's `AppCtx::build` should hardcode (or import via a `pub const SCHEMA_VERSION: u32 = 12` we could add later if needed — Plan 03 leaves this implicit).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `audit_log.id` notnull mismatch — SQLite reports notnull=0 on INTEGER PRIMARY KEY**

- **Found during:** Task 3 (first `cargo test --test audit_log_schema` run)
- **Issue:** Test asserted `audit_log.id` is `NOT NULL` (conceptually true — primary keys must be non-null). SQLite's `PRAGMA table_info` reports `notnull=0` on `INTEGER PRIMARY KEY` columns because they're implicitly non-null via the rowid alias. The first run failed `assertion `left == right` failed: audit_log.id not_null want true, got false`.
- **Fix:** Relaxed the assertion to `not_null=false` and documented the quirk in an inline comment so future readers don't think the schema is wrong.
- **Files modified:** `crates/trackly-infra/tests/audit_log_schema.rs`
- **Verification:** `cargo test --test audit_log_schema` now passes 4/4.
- **Committed in:** `ace4b28`

**2. [Rule 1 - Bug] Timestamp-suffix invariant test rejected legitimate non-timestamp columns**

- **Found during:** Task 3 (first `cargo test --test per_record_invariants` run)
- **Issue:** The `all_timestamp_columns_use_at_utc_suffix_and_integer_type` test flagged three columns as violators that are actually legitimate non-timestamps or legacy-named timestamps:
  - `act_items.condition_at_time` (TEXT) — snapshot of the device's *state* at the moment of the act, not a timestamp value.
  - `act_items.complectation_at_time` (TEXT) — same as above.
  - `sessions.expiry_date` (INTEGER) — tower-sessions canonical column name; using their convention keeps the future custom `SessionStore` impl trivial.
- **Fix:** Added an explicit allowlist of `(table, column)` pairs to the test with a comment explaining each entry. Additionally, kept the `sessions.expiry_date` IS INTEGER assertion as a positive check (since it really is a timestamp).
- **Files modified:** `crates/trackly-infra/tests/per_record_invariants.rs`
- **Verification:** `cargo test --test per_record_invariants` now passes 3/3.
- **Committed in:** `ace4b28`

---

**Total deviations:** 2 auto-fixed (2× Rule 1 Bug). Both were test-author bugs surfaced on first run; both fixes are documented in-source. No architectural changes. No checkpoints surfaced to the user.

## Issues Encountered

None. All 18 tests passed within two iterations of the test files (first iteration: 16/18; second iteration after the two test fixes: 18/18). The schema SQL itself was correct on first authorship — no migration files needed re-editing.

## User Setup Required

None. The `test_db()` fixture is fully self-contained: tempfile, pragmas, migrations, all in one call.

## Indexes Created in V012 (Full List)

For downstream-plan visibility:

| Index | Table | Columns |
|-------|-------|---------|
| `idx_audit_log_entity` | `audit_log` | `(entity_type, entity_id, created_at_utc)` |
| `idx_audit_log_user` | `audit_log` | `(user_id, created_at_utc)` |
| `idx_acts_parent` | `acts` | `(parent_act_id)` |
| `idx_act_items_act` | `act_items` | `(act_id)` |
| `idx_act_items_device` | `act_items` | `(device_id)` |
| `idx_cartridges_model` | `cartridges` | `(model_id)` |
| `idx_cartridge_compat_model` | `cartridge_model_compatibility` | `(cartridge_model_id)` |
| `idx_devices_location` | `devices` | `(location_id)` |
| `idx_devices_status` | `devices` | `(status_id)` |
| `idx_devices_type` | `devices` | `(type_id)` |
| `idx_requests_status` | `requests` | `(status)` |
| `idx_requests_assigned` | `requests` | `(assigned_to_user_id)` |
| `idx_sessions_expiry` | `sessions` | `(expiry_date)` |
| `idx_scheduled_tasks_next_run` | `scheduled_tasks` | `(next_run_at_utc)` |

Plus 4 unique indexes defined in their owning migrations:
- `idx_acts_number_sub_unique` on `acts(number, COALESCE(sub_number, 0))` WHERE deleted_at_utc IS NULL (V004)
- `idx_cartridge_models_brand_model_unique` on `cartridge_models(brand, model)` WHERE deleted_at_utc IS NULL (V005)
- `idx_document_templates_kind_active_unique` on `document_templates(kind)` WHERE is_active = 1 AND deleted_at_utc IS NULL (V007)
- Implicit UNIQUE indexes from `UNIQUE` column constraints (users.login, locations.name, cartridges.code, etc.).

## FTS5 Tokenizer Note (`unicode61 remove_diacritics 2`)

Per SQLite docs (https://sqlite.org/fts5.html#tokenizers), `unicode61` is FTS5's canonical Unicode-aware tokenizer; `remove_diacritics 2` normalises both single-codepoint accented characters and multi-codepoint combining sequences. For Cyrillic this means `ё` and `е` are treated as equivalent in search (`ё` lacks a precomposed canonical decomposition but the tokenizer falls back to compatibility decomposition where it does help). Phase 2's `devices_fts` triggers and Phase 2's actual search queries will exercise this in practice; Phase 1 ships the virtual tables only.

**Verification path for downstream:** when Phase 2 wires `devices_fts` triggers, a quick `SELECT * FROM devices_fts WHERE devices_fts MATCH 'елка'` against a row containing `'ёлка'` should match. If it does NOT match, the workaround is either (a) Phase 3 PR upgrading to a custom tokenizer (e.g., porus2's `russian-stemmer-tokenizer`) or (b) pre-normalising `ё → е` in the indexer.

## FTS5 Trigger Status

**NOT created in Phase 1.** Phase 2 (`devices_fts` triggers — INSERT/UPDATE/DELETE on `devices`), Phase 3 (`acts_fts` triggers), and Phase 4 (`cartridges_fts` triggers) own the sync triggers when the corresponding write operations land. This split keeps Phase 1 focused on schema-only and avoids dead-code triggers that would be edited again later.

## Refinery Transaction Behaviour

**Default (per-migration transaction)** — `set_grouped(false)` is the refinery 0.9 default. Confirmed empirically via the `migration_idempotency.rs` integration test: a second `run()` call on the same `Connection` returns `applied_count = 0` and `schema_version = 12` (refinery's `refinery_schema_history` table tracks completed migrations by checksum). Refinery's checksum tracking also means any future edit to an already-applied `V0XX_*.sql` file will fail loudly on the next startup (T-03-01 mitigation).

## Next Phase Readiness

**Ready for Plan 04** (AppCtx + writer worker + reader pool + downgrade protection):

- `db::pragmas::apply_writer_pragmas` is the helper Plan 04's single-writer task calls at construction.
- `db::pragmas::apply_reader_pragmas` is the helper Plan 04's reader pool calls on every connection.
- `db::migrations::run` is the helper Plan 04's `AppCtx::build` calls AFTER opening the writer connection but BEFORE serving requests.
- `MigrationReport.schema_version` is what Plan 04's downgrade-check compares against the embedded constant `12`. If `file_version > 12`, `AppCtx::build` returns `AppError::DatabaseFromNewerVersion { binary: 12, file: <found> }` (Plan 04 will need to add this variant to `AppError` — currently only `Internal` + `Validation` per Plan 02).
- `test_db()` is the canonical fixture for Plan 04's `concurrent_writes.rs` integration test (25 + 25 tasks) and Plan 05's `health_smoke.rs`.

**Carry-forward notes for downstream plans:**

- The schema version constant is implicit — Plan 04 should either hardcode `12` or add `pub const SCHEMA_VERSION: u32 = 12` to `db::migrations` if downgrade tests need to import it. Recommendation: add the const in Plan 04 alongside the downgrade check.
- `act_items.condition_at_time` and `complectation_at_time` are TEXT (state snapshots, not timestamps) — when Plan 04's audit-log writer captures act-item history, it should serialise the existing TEXT value, not synthesise a new timestamp.
- `sessions.expiry_date` is INTEGER unix epoch seconds (tower-sessions convention). When Plan 05 implements the custom `SessionStore`, use that column name verbatim.
- FTS5 sync triggers (devices_fts/acts_fts/cartridges_fts) are NOT created in Phase 1 — Phase 2/3/4 own them. If Plan 04 needs to write to `devices`, it does NOT need to also write to `devices_fts` — the rows will be invisible to FTS until Phase 2's triggers land.
- Refinery checksum tracking means **never edit an already-applied `V0XX_*.sql` file** — always create `V0(N+1)_*.sql` for additive changes. T-03-01 in the threat model documents this; the per_record_invariants test enforces it behaviourally for D-Schema-03/04 violations on new tables.

**No blockers** for Plan 04.

## Threat Flags

None — no new security-relevant surface beyond what the plan's `<threat_model>` already covers. T-03-01 (checksum drift) is mitigated by refinery's built-in checksum tracking; T-03-03 (invariant violation) is mitigated behaviourally by `per_record_invariants.rs`; T-03-05 (partial migration) is mitigated by refinery's per-migration transaction.

## Self-Check: PASSED

Verified after writing SUMMARY:

- All 12 migration files exist in `migrations/` with `PRAGMA user_version = N;` markers.
- `crates/trackly-infra/src/db/mod.rs`, `pragmas.rs`, `migrations.rs` exist.
- `crates/trackly-infra/src/test_support/mod.rs`, `test_db.rs` exist.
- All 4 integration test files exist in `crates/trackly-infra/tests/`.
- All 3 task commits present in git log: `c019ccb`, `d7d8799`, `ace4b28`.
- `cargo test -p trackly-infra` passes (5 lib + 13 integration = 18 tests).
- `cargo clippy --workspace --all-targets -- -D warnings` passes.
- `cargo build --workspace` passes.
- `cargo fmt --all -- --check` passes.

---

*Phase: 01-foundation*
*Completed: 2026-05-24*
