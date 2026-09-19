//! Integration test: `V043__cartridges_code_unique_live.sql` — `cartridges.code`
//! uniqueness rebuild, column-level `UNIQUE` -> partial `UNIQUE INDEX ...
//! WHERE deleted_at_utc IS NULL` (Phase 40.2 Plan 07, NUM-09 space "в").
//!
//! Mirrors `acts_number_text_migration.rs`'s pattern (itself mirroring
//! `trackly-infra`'s own `v032_data_transform_drops_empty_and_preserves_populated`):
//! stand up a pre-V043-shaped `cartridges` table (same column set as the real
//! cumulative V005+V025+V038 schema, minus FK REFERENCES annotations this
//! test doesn't need enforced) plus the `cartridges_fts` virtual table and
//! its three sync triggers (V038's exact shape — `DROP TABLE cartridges`
//! inside V043 drops them along with the table, so V043 must recreate them;
//! this test proves it did), apply the REAL V043 SQL via `include_str!`, and
//! assert the NEW uniqueness rule plus data preservation.
//!
//! Continuity note (NUM-13, D-16): the actual "next code after C-0042 is
//! C-0043" acceptance criterion is proven in `trackly-app`'s
//! `cartridges_numbering.rs` (Task 3), where `NumberTemplateService` is
//! available. This test only proves the narrower, purely-SQL claim that
//! `cartridges.code` values physically survive V043 unchanged — the
//! prerequisite fact D-16's continuity argument depends on.

use rusqlite::Connection;
use tempfile::TempDir;

use trackly_infra::db::pragmas::apply_writer_pragmas;

/// The actual V043 migration SQL, embedded at compile time so this test
/// exercises the shipped file rather than a hand-copied duplicate.
const V043_SQL: &str = include_str!("../../../migrations/V043__cartridges_code_unique_live.sql");

fn fresh_conn() -> (Connection, TempDir) {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("cartridges-code-unique-migration.db");
    let conn = Connection::open(&path).expect("open");
    apply_writer_pragmas(&conn).expect("writer pragmas");
    (conn, dir)
}

/// Pre-V043 `cartridges` schema — the cumulative shape from V005 (base
/// table) + V025 (`current_printer_device_id`) + V038 (`place_id`, dropped
/// `location`), WITHOUT foreign key REFERENCES annotations (this test only
/// needs the exact column set V043's own INSERT/SELECT column list depends
/// on, not FK enforcement), PLUS the `cartridges_fts` virtual table and its
/// three V038 sync triggers — V043's `DROP TABLE cartridges` drops these
/// triggers along with the table, so the pre-migration fixture must have
/// them present for the "V043 recreates them" assertion to mean anything.
///
/// Deliberately WITHOUT the real pre-V043 column-level `UNIQUE` on `code`:
/// under the real (pre-migration) schema, a live row and a soft-deleted row
/// sharing the same code could never coexist in the first place (the
/// column-level constraint is global, blind to `deleted_at_utc`) — so the
/// scenario this test needs to seed (a soft-deleted row "holding" a code a
/// live row now wants to reuse) has no real pre-migration data shape to
/// reproduce it from. The fixture omits the constraint specifically so the
/// scenario can be seeded directly, then V043 is applied on top exactly as
/// it would run against production data (it does not re-validate existing
/// rows against the new rule — it only rebuilds the table and adds the new
/// partial index), which is the actual thing under test.
fn create_pre_v043_cartridges_table(conn: &Connection) {
    conn.execute_batch(
        "CREATE TABLE cartridges (
            id                          INTEGER PRIMARY KEY AUTOINCREMENT,
            code                        TEXT    NOT NULL,
            model_id                    INTEGER NOT NULL,
            status_id                   INTEGER NOT NULL DEFAULT 1,
            state_id                    INTEGER NULL,
            holder_name                 TEXT    NULL,
            notes                       TEXT    NULL,
            created_at_utc              INTEGER NOT NULL,
            updated_at_utc              INTEGER NOT NULL,
            deleted_at_utc              INTEGER NULL,
            version                     INTEGER NOT NULL DEFAULT 1,
            current_printer_device_id   INTEGER NULL,
            place_id                    INTEGER NULL
         );
         CREATE INDEX idx_cartridges_model ON cartridges(model_id);
         CREATE INDEX idx_cartridges_place ON cartridges(place_id)
           WHERE deleted_at_utc IS NULL AND place_id IS NOT NULL;

         CREATE VIRTUAL TABLE cartridges_fts USING fts5(
           code, holder_name,
           content='cartridges', content_rowid='id',
           tokenize='unicode61 remove_diacritics 2'
         );

         CREATE TRIGGER cartridges_fts_ai
         AFTER INSERT ON cartridges
         WHEN NEW.deleted_at_utc IS NULL
         BEGIN
           INSERT INTO cartridges_fts(rowid, code, holder_name)
           VALUES (NEW.id, NEW.code, NEW.holder_name);
         END;

         CREATE TRIGGER cartridges_fts_ad
         AFTER DELETE ON cartridges
         BEGIN
           INSERT INTO cartridges_fts(cartridges_fts, rowid, code, holder_name)
           VALUES ('delete', OLD.id, OLD.code, OLD.holder_name);
         END;

         CREATE TRIGGER cartridges_fts_au
         AFTER UPDATE ON cartridges
         BEGIN
           INSERT INTO cartridges_fts(cartridges_fts, rowid, code, holder_name)
           VALUES ('delete', OLD.id, OLD.code, OLD.holder_name);
           INSERT INTO cartridges_fts(rowid, code, holder_name)
           SELECT NEW.id, NEW.code, NEW.holder_name
           WHERE NEW.deleted_at_utc IS NULL;
         END;",
    )
    .expect("create pre-V043 cartridges table + fts");
}

fn insert_cartridge(
    conn: &Connection,
    id: i64,
    code: &str,
    deleted_at_utc: Option<i64>,
    now: i64,
) {
    conn.execute(
        "INSERT INTO cartridges (id, code, model_id, status_id, created_at_utc, \
         updated_at_utc, deleted_at_utc, version) \
         VALUES (?1, ?2, 1, 1, ?3, ?3, ?4, 1)",
        rusqlite::params![id, code, now, deleted_at_utc],
    )
    .unwrap_or_else(|e| panic!("insert cartridge id={id} code={code}: {e}"));
}

#[test]
fn soft_deleted_code_frees_up_but_live_duplicate_still_rejected() {
    let (conn, _guard) = fresh_conn();
    create_pre_v043_cartridges_table(&conn);

    let now = 1_700_000_000_i64;

    // Cartridge #1: created live with code C-0042, later soft-deleted
    // (`deleted_at_utc` set after `created_at_utc` — a real lifecycle, not
    // two coexisting rows). Under the OLD column-level `UNIQUE(code)` this
    // single row's code stays reserved forever even after soft-delete —
    // exactly the pain point NUM-09 fixes; seeded directly here (before
    // V043 applies) so the test proves the NEW rule frees it up.
    insert_cartridge(&conn, 1, "C-0042", Some(now + 5), now);

    conn.execute_batch(V043_SQL).expect("apply V043 migration");

    // The rebuilt table (per the real V043 SQL) declares REFERENCES on
    // model_id/place_id/etc against lookup tables this minimal test schema
    // never creates (cartridge_models, places, ...) — irrelevant to what
    // this test is about (the code-uniqueness index), so FK enforcement is
    // turned off for the verification inserts below, same as it implicitly
    // was for every insert against this fixture before migration.
    conn.execute_batch("PRAGMA foreign_keys = OFF;")
        .expect("disable fk for verification inserts");

    // (1) A NEW live cartridge reusing the soft-deleted row's code must
    // succeed — the partial index only sees live rows.
    let result = conn.execute(
        "INSERT INTO cartridges (code, model_id, status_id, created_at_utc, \
         updated_at_utc, deleted_at_utc, version) \
         VALUES ('C-0042', 1, 1, ?1, ?1, NULL, 1)",
        [now + 100],
    );
    assert!(
        result.is_ok(),
        "reusing a soft-deleted cartridge's code for a new LIVE row must succeed \
         under the partial unique index: {result:?}"
    );

    // (2) Two LIVE cartridges with the same code must still be rejected —
    // the index is real and enforced, not accidentally dropped.
    let dup_result = conn.execute(
        "INSERT INTO cartridges (code, model_id, status_id, created_at_utc, \
         updated_at_utc, deleted_at_utc, version) \
         VALUES ('C-0042', 1, 1, ?1, ?1, NULL, 1)",
        [now + 200],
    );
    assert!(
        dup_result.is_err(),
        "two LIVE cartridges with the same code must be rejected by \
         idx_cartridges_code_live"
    );
}

#[test]
fn existing_codes_survive_migration_unchanged_continuity_prerequisite() {
    let (conn, _guard) = fresh_conn();
    create_pre_v043_cartridges_table(&conn);

    let now = 1_700_000_000_i64;
    let seeded_codes: Vec<String> = (1..=42).map(|n| format!("C-{n:04}")).collect();
    for (idx, code) in seeded_codes.iter().enumerate() {
        insert_cartridge(&conn, (idx + 1) as i64, code, None, now + idx as i64);
    }

    conn.execute_batch(V043_SQL).expect("apply V043 migration");

    let mut stmt = conn
        .prepare("SELECT code FROM cartridges ORDER BY id")
        .expect("prepare");
    let survived: Vec<String> = stmt
        .query_map([], |r| r.get::<_, String>(0))
        .expect("query_map")
        .map(|r| r.expect("row"))
        .collect();

    assert_eq!(
        survived, seeded_codes,
        "C-0001..C-0042 must survive V043 byte-for-byte and in the same order — \
         this is the prerequisite fact D-16's NUM-13 continuity argument (next \
         proposed code after V043 is C-0043) depends on. The actual \
         NumberTemplateService::peek_next assertion lives in trackly-app's \
         cartridges_numbering.rs (Task 3), which alone has access to the \
         template/mask machinery this migration-level test does not."
    );
}

#[test]
fn fts_triggers_and_secondary_indexes_survive_the_rebuild() {
    let (conn, _guard) = fresh_conn();
    create_pre_v043_cartridges_table(&conn);
    conn.execute_batch(V043_SQL).expect("apply V043 migration");

    let mut stmt = conn
        .prepare(
            "SELECT name FROM sqlite_master WHERE type = 'trigger' AND tbl_name = 'cartridges' \
             ORDER BY name",
        )
        .expect("prepare");
    let triggers: Vec<String> = stmt
        .query_map([], |r| r.get::<_, String>(0))
        .expect("query_map")
        .map(|r| r.expect("row"))
        .collect();
    assert_eq!(
        triggers,
        vec![
            "cartridges_fts_ad".to_string(),
            "cartridges_fts_ai".to_string(),
            "cartridges_fts_au".to_string(),
        ],
        "FTS sync triggers must be recreated after the rebuild (DROP TABLE \
         cartridges drops them along with the table)"
    );

    let mut idx_stmt = conn
        .prepare(
            "SELECT name FROM sqlite_master WHERE type = 'index' AND tbl_name = 'cartridges' \
             AND name NOT LIKE 'sqlite_autoindex%' ORDER BY name",
        )
        .expect("prepare");
    let indexes: Vec<String> = idx_stmt
        .query_map([], |r| r.get::<_, String>(0))
        .expect("query_map")
        .map(|r| r.expect("row"))
        .collect();
    assert_eq!(
        indexes,
        vec![
            "idx_cartridges_code_live".to_string(),
            "idx_cartridges_model".to_string(),
            "idx_cartridges_place".to_string(),
        ],
        "idx_cartridges_model and idx_cartridges_place must survive the rebuild \
         alongside the new idx_cartridges_code_live"
    );
}

#[test]
fn user_version_is_43_after_migration() {
    let (conn, _guard) = fresh_conn();
    create_pre_v043_cartridges_table(&conn);
    conn.execute_batch(V043_SQL).expect("apply V043 migration");

    let version: i64 = conn
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .expect("read user_version");
    assert_eq!(version, 43);
}
