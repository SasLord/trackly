//! Integration test: `V043__cartridges_code_unique_live.sql` — `cartridges.code`
//! uniqueness rebuild, column-level `UNIQUE` -> partial `UNIQUE INDEX ...
//! WHERE deleted_at_utc IS NULL` (Phase 40.2 Plan 07, NUM-09), exercised
//! through the REAL migration runner on a genuine pre-V043 database:
//! `apply_writer_pragmas` -> `migrations::run_up_to(42)` -> seed ->
//! `migrations::run`.
//!
//! BE-CR-02 (code review 40.2): with FKs effectively ON inside refinery's
//! per-file transaction, `DROP TABLE cartridges` failed with `FOREIGN KEY
//! constraint failed` as soon as any request had `completed_cartridge_id`
//! set (a completed "cartridge replace" request — a normal Phase 12 flow),
//! and the app refused to start. The runner now disables FKs on the
//! connection before refinery starts; these tests pin that the upgrade
//! succeeds and the request keeps its link. Fictional names only.
//!
//! Continuity note (NUM-13, D-16): "next code after C-0042 is C-0043" is
//! proven in `trackly-app`'s `cartridges_numbering.rs`; this file proves the
//! prerequisite that codes physically survive V043 unchanged.

use rusqlite::{params, Connection};
use tempfile::TempDir;

use trackly_infra::db::migrations;
use trackly_infra::db::pragmas::apply_writer_pragmas;

const NOW: i64 = 1_700_000_000;

fn conn_at_v042() -> (Connection, TempDir) {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("cartridges-code-unique-migration.db");
    let mut conn = Connection::open(&path).expect("open");
    apply_writer_pragmas(&conn).expect("writer pragmas");
    let report = migrations::run_up_to(&mut conn, 42).expect("run migrations up to V042");
    assert_eq!(report.schema_version, 42, "fixture must be a real V042 DB");
    conn.execute_batch(&format!(
        "INSERT INTO users (id, login, full_name, role, created_at_utc, updated_at_utc)
           VALUES (1, 'petrov', 'Петров П.П.', 'employee', {NOW}, {NOW});
         INSERT INTO cartridge_models (id, brand, model, created_at_utc, updated_at_utc)
           VALUES (1, 'Pantum', 'TL-5120X', {NOW}, {NOW});"
    ))
    .expect("seed lookup rows");
    (conn, dir)
}

fn insert_cartridge(conn: &Connection, id: i64, code: &str, deleted_at_utc: Option<i64>) {
    conn.execute(
        "INSERT INTO cartridges (id, code, model_id, status_id, created_at_utc, \
         updated_at_utc, deleted_at_utc, version) \
         VALUES (?1, ?2, 1, 1, ?3, ?3, ?4, 1)",
        params![id, code, NOW, deleted_at_utc],
    )
    .unwrap_or_else(|e| panic!("insert cartridge id={id} code={code}: {e}"));
}

#[test]
fn upgrade_succeeds_with_completed_request_and_keeps_its_cartridge_link() {
    let (mut conn, _guard) = conn_at_v042();
    insert_cartridge(&conn, 1, "C-0001", None);
    insert_cartridge(&conn, 2, "C-0002", None);
    conn.execute(
        "INSERT INTO requests (id, request_type, status, requested_by_user_id, \
         cartridge_model_id, created_at_utc, updated_at_utc, completed_cartridge_id) \
         VALUES (1, 'cartridge_replace', 'completed', 1, 1, ?1, ?1, 2)",
        params![NOW],
    )
    .expect("seed completed cartridge_replace request");

    migrations::run(&mut conn)
        .expect("upgrade V042 -> latest must succeed with requests.completed_cartridge_id set");

    let link: Option<i64> = conn
        .query_row(
            "SELECT completed_cartridge_id FROM requests WHERE id = 1",
            [],
            |r| r.get(0),
        )
        .expect("read request link");
    assert_eq!(
        link,
        Some(2),
        "the request must still point at cartridge #2"
    );

    let fk_on: i64 = conn
        .pragma_query_value(None, "foreign_keys", |r| r.get(0))
        .expect("read foreign_keys");
    assert_eq!(fk_on, 1, "runner must re-enable foreign keys");
    let violations: i64 = conn
        .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |r| {
            r.get(0)
        })
        .expect("foreign_key_check");
    assert_eq!(violations, 0);

    // The FK now resolves against the REBUILT cartridges table.
    let dangling = conn.execute(
        "UPDATE requests SET completed_cartridge_id = 999 WHERE id = 1",
        [],
    );
    assert!(
        dangling.is_err(),
        "requests.completed_cartridge_id must still be enforced after the rebuild"
    );
}

#[test]
fn soft_deleted_code_frees_up_but_live_duplicate_still_rejected() {
    let (mut conn, _guard) = conn_at_v042();
    // Created live with C-0042 and later soft-deleted: under the OLD global
    // UNIQUE(code) the code stayed reserved forever.
    insert_cartridge(&conn, 1, "C-0042", Some(NOW + 5));
    migrations::run(&mut conn).expect("upgrade");

    let reuse = conn.execute(
        "INSERT INTO cartridges (code, model_id, status_id, created_at_utc, \
         updated_at_utc, deleted_at_utc, version) VALUES ('C-0042', 1, 1, ?1, ?1, NULL, 1)",
        params![NOW + 100],
    );
    assert!(
        reuse.is_ok(),
        "reusing a soft-deleted code for a new LIVE row must succeed: {reuse:?}"
    );

    let dup = conn.execute(
        "INSERT INTO cartridges (code, model_id, status_id, created_at_utc, \
         updated_at_utc, deleted_at_utc, version) VALUES ('C-0042', 1, 1, ?1, ?1, NULL, 1)",
        params![NOW + 200],
    );
    assert!(
        dup.is_err(),
        "two LIVE cartridges with the same code must be rejected by idx_cartridges_code_live"
    );
}

#[test]
fn existing_codes_survive_migration_unchanged_continuity_prerequisite() {
    let (mut conn, _guard) = conn_at_v042();
    let seeded_codes: Vec<String> = (1..=42).map(|n| format!("C-{n:04}")).collect();
    for (idx, code) in seeded_codes.iter().enumerate() {
        insert_cartridge(&conn, (idx + 1) as i64, code, None);
    }
    migrations::run(&mut conn).expect("upgrade");

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
        "C-0001..C-0042 must survive V043 byte-for-byte and in the same order"
    );
}

#[test]
fn fts_triggers_and_secondary_indexes_survive_the_rebuild() {
    let (mut conn, _guard) = conn_at_v042();
    insert_cartridge(&conn, 1, "C-0001", None);
    migrations::run(&mut conn).expect("upgrade");

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
        "FTS sync triggers must be recreated after the rebuild"
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
        ]
    );

    // FTS still finds the pre-existing row (id preserved 1:1).
    let hits: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM cartridges_fts WHERE cartridges_fts MATCH '\"C-0001\"'",
            [],
            |r| r.get(0),
        )
        .expect("fts query");
    assert_eq!(hits, 1);
}

/// BE-WR-11: the rebuild must carry the AUTOINCREMENT high-water mark over.
#[test]
fn rebuild_keeps_autoincrement_high_water_mark() {
    let (mut conn, _guard) = conn_at_v042();
    insert_cartridge(&conn, 1, "C-0001", None);
    insert_cartridge(&conn, 20, "C-0020", None);
    conn.execute("DELETE FROM cartridges WHERE id = 20", [])
        .expect("physically delete cartridge #20");

    migrations::run(&mut conn).expect("upgrade");

    conn.execute(
        "INSERT INTO cartridges (code, model_id, status_id, created_at_utc, \
         updated_at_utc, version) VALUES ('C-0002', 1, 1, ?1, ?1, 1)",
        params![NOW],
    )
    .expect("insert new cartridge");
    assert_eq!(
        conn.last_insert_rowid(),
        21,
        "the new cartridge must not reuse id 20 of the physically deleted row"
    );
}
