//! Integration test: `V042__acts_number_text.sql` — `acts.number` INTEGER →
//! TEXT rebuild (Phase 40.2 Plan 06, NUM-14).
//!
//! Mirrors `trackly-infra`'s own `v032_data_transform_drops_empty_and_preserves_populated`
//! test pattern (`crates/trackly-infra/src/db/migrations.rs`): stand up a
//! pre-V042-shaped `acts` table (INTEGER `number`, same column set/order as
//! the real cumulative V004+V014+V015+V038 schema, minus FK REFERENCES
//! annotations this test doesn't need enforced), seed realistic rows —
//! including a partial return with `sub_number`/`parent_act_id`, using
//! fictional names per the project's no-real-data rule — apply the REAL
//! V042 SQL via `include_str!`, and assert:
//!   (a) `number` is now TEXT storage class holding the exact original
//!       digits (no data loss across the rebuild),
//!   (b) every row's `id` is unchanged (so FK targets like
//!       `act_items.act_id` / `place_movements.act_id` still resolve to the
//!       same logical row after the rename),
//!   (c) `idx_acts_number_sub_unique` exists after the rebuild AND actually
//!       rejects a live duplicate `(number, sub_number)` pair.
//!
//! `run_applies_all_known_migrations_on_fresh_db` (in `db::migrations`'s own
//! test module) already proves V042 applies cleanly through the REAL
//! refinery pipeline (including the `PRAGMA foreign_keys = OFF/ON` dance
//! while `act_items`/`place_movements` reference `acts(id)`) — this test
//! adds the data-preservation assertions that fresh-DB test cannot cover.

use rusqlite::Connection;
use tempfile::TempDir;

use trackly_infra::db::pragmas::apply_writer_pragmas;

/// The actual V042 migration SQL, embedded at compile time so this test
/// exercises the shipped file rather than a hand-copied duplicate.
const V042_SQL: &str = include_str!("../../../migrations/V042__acts_number_text.sql");

fn fresh_conn() -> (Connection, TempDir) {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("acts-number-text-migration.db");
    let conn = Connection::open(&path).expect("open");
    apply_writer_pragmas(&conn).expect("writer pragmas");
    (conn, dir)
}

/// Pre-V042 `acts` schema — the cumulative shape from V004 (base table) +
/// V014 (`deadline_utc`) + V015 (`handover_date_utc`) + V038 (`place_id`,
/// `bulk_place_id`, `place_path_snapshot`), WITHOUT foreign key REFERENCES
/// annotations (this test only needs the exact column set V042's own
/// INSERT/SELECT column list depends on, not FK enforcement — same
/// minimalism as the `v032_data_transform_...` precedent).
fn create_pre_v042_acts_table(conn: &Connection) {
    conn.execute_batch(
        "CREATE TABLE acts (
            id                  INTEGER PRIMARY KEY AUTOINCREMENT,
            number              INTEGER NOT NULL,
            sub_number          INTEGER NULL,
            parent_act_id       INTEGER NULL,
            act_type            TEXT    NOT NULL CHECK (act_type IN ('handover', 'return')),
            giver_name          TEXT    NOT NULL,
            receiver_name       TEXT    NOT NULL,
            notes               TEXT    NULL,
            archived            INTEGER NOT NULL DEFAULT 0,
            created_at_utc      INTEGER NOT NULL,
            updated_at_utc      INTEGER NOT NULL,
            deleted_at_utc      INTEGER NULL,
            version             INTEGER NOT NULL DEFAULT 1,
            deadline_utc        INTEGER NULL,
            handover_date_utc   INTEGER NOT NULL DEFAULT 0,
            place_id            INTEGER NULL,
            bulk_place_id       INTEGER NULL,
            place_path_snapshot TEXT NULL
         );
         CREATE UNIQUE INDEX idx_acts_number_sub_unique
           ON acts(number, COALESCE(sub_number, 0))
           WHERE deleted_at_utc IS NULL;
         CREATE INDEX idx_acts_parent ON acts(parent_act_id);
         CREATE INDEX idx_acts_parent_act_id ON acts(parent_act_id) WHERE parent_act_id IS NOT NULL;",
    )
    .expect("create pre-V042 acts table");
}

#[test]
fn migrates_integer_number_to_text_preserving_ids_and_unique_index() {
    let (conn, _guard) = fresh_conn();
    create_pre_v042_acts_table(&conn);

    let now = 1_700_000_000_i64;

    // Row 1: handover act #42 (fictional names, no real org/person data).
    conn.execute(
        "INSERT INTO acts (id, number, sub_number, parent_act_id, act_type, \
         giver_name, receiver_name, created_at_utc, updated_at_utc, version, \
         handover_date_utc) \
         VALUES (1, 42, NULL, NULL, 'handover', 'Иванов И.И.', 'Петров П.П.', \
         ?1, ?1, 1, ?1)",
        [now],
    )
    .expect("seed handover act #42");

    // Row 2: its sole partial return (sub_number=1, parent_act_id=1) — the
    // display rule would render this as "42в" (D-06 territory for later
    // tasks; this migration test only cares about raw column preservation).
    conn.execute(
        "INSERT INTO acts (id, number, sub_number, parent_act_id, act_type, \
         giver_name, receiver_name, created_at_utc, updated_at_utc, version, \
         handover_date_utc) \
         VALUES (2, 42, 1, 1, 'return', 'Петров П.П.', 'Иванов И.И.', \
         ?1, ?1, 1, ?1)",
        [now + 10],
    )
    .expect("seed return act");

    // Row 3: an unrelated handover act #7 — proves the rebuilt index still
    // distinguishes distinct numbers correctly (not just "any dup fails").
    conn.execute(
        "INSERT INTO acts (id, number, sub_number, parent_act_id, act_type, \
         giver_name, receiver_name, created_at_utc, updated_at_utc, version, \
         handover_date_utc) \
         VALUES (3, 7, NULL, NULL, 'handover', 'Сидоров С.С.', 'Кузнецов К.К.', \
         ?1, ?1, 1, ?1)",
        [now + 20],
    )
    .expect("seed second handover act");

    // Apply the REAL V042 SQL.
    conn.execute_batch(V042_SQL).expect("apply V042 migration");

    // (a) + (b): number is now TEXT storage holding the exact original
    // digits; id / sub_number / parent_act_id survive unchanged.
    type ActNumberRow = (i64, String, String, Option<i64>, Option<i64>);
    let mut stmt = conn
        .prepare(
            "SELECT id, typeof(number), number, sub_number, parent_act_id \
             FROM acts ORDER BY id",
        )
        .expect("prepare");
    let rows: Vec<ActNumberRow> = stmt
        .query_map([], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
        })
        .expect("query_map")
        .map(|r| r.expect("row"))
        .collect();

    assert_eq!(
        rows,
        vec![
            (1, "text".to_string(), "42".to_string(), None, None),
            (2, "text".to_string(), "42".to_string(), Some(1), Some(1)),
            (3, "text".to_string(), "7".to_string(), None, None),
        ],
        "number must become TEXT storage class holding the exact original \
         digits; id / sub_number / parent_act_id must survive unchanged \
         (FK targets intact)"
    );

    // (c) idx_acts_number_sub_unique exists after the rebuild...
    let index_exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master \
             WHERE type = 'index' AND name = 'idx_acts_number_sub_unique'",
            [],
            |r| r.get(0),
        )
        .expect("query sqlite_master for index");
    assert_eq!(
        index_exists, 1,
        "idx_acts_number_sub_unique must exist after the rebuild"
    );

    // ...and actually rejects a live duplicate: number='42', sub_number=NULL
    // collides with row 1 (COALESCE(sub_number, 0) = 0 for both).
    let dup_result = conn.execute(
        "INSERT INTO acts (number, sub_number, parent_act_id, act_type, \
         giver_name, receiver_name, created_at_utc, updated_at_utc, version, \
         handover_date_utc) \
         VALUES ('42', NULL, NULL, 'handover', 'Дубликат Д.Д.', 'Тестов Т.Т.', \
         ?1, ?1, 1, ?1)",
        [now + 30],
    );
    assert!(
        dup_result.is_err(),
        "duplicate (number, sub_number) must be rejected by the recreated \
         unique index"
    );
}

#[test]
fn user_version_is_42_after_migration() {
    let (conn, _guard) = fresh_conn();
    create_pre_v042_acts_table(&conn);
    conn.execute_batch(V042_SQL).expect("apply V042 migration");

    let version: i64 = conn
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .expect("read user_version");
    assert_eq!(version, 42);
}
