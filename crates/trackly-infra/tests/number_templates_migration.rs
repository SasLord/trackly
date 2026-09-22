//! Integration test: V041 `number_templates` + `number_template_contexts`
//! schema, seed data, and `counters` removal correctness (Phase 40.2 Plan 01,
//! NUM-01/NUM-13).
//!
//! Mirrors `place_movements_migration.rs`'s harness pattern (fresh tempfile
//! DB, apply pragmas + all embedded migrations, then inspect the resulting
//! schema/data) — same `fresh_migrated_db` helper, not a new harness.

use rusqlite::Connection;
use tempfile::TempDir;

use trackly_infra::db::{migrations, pragmas};

/// Fresh DB with all embedded migrations applied.
fn fresh_migrated_db(dir: &TempDir, file_name: &str) -> Connection {
    let db_path = dir.path().join(file_name);
    let mut conn = Connection::open(&db_path).expect("open");
    pragmas::apply_writer_pragmas(&conn).expect("pragmas");
    migrations::run(&mut conn).expect("run migrations");
    conn
}

/// Test 1: fresh DB seeds exactly the prescribed 3 templates and 5 contexts.
#[test]
fn v041_seeds_expected_templates_and_contexts() {
    let dir = TempDir::new().expect("tempdir");
    let conn = fresh_migrated_db(&dir, "number-templates-seed.db");

    let mut stmt = conn
        .prepare("SELECT type, mask FROM number_templates ORDER BY type")
        .expect("prepare templates query");
    let rows: Vec<(String, String)> = stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
        .expect("query templates")
        .map(|r| r.expect("row"))
        .collect();
    assert_eq!(
        rows,
        vec![
            ("act_number".to_string(), "[X]".to_string()),
            ("cartridge_code".to_string(), "C-[XXXX]".to_string()),
            ("drum_code".to_string(), "D-[XXXX]".to_string()),
        ],
        "number_templates must contain exactly the 3 seeded rows (D-16)"
    );

    let mut ctx_stmt = conn
        .prepare(
            "SELECT ntc.context, nt.type, nt.mask \
             FROM number_template_contexts ntc \
             LEFT JOIN number_templates nt ON nt.id = ntc.template_id \
             ORDER BY ntc.context",
        )
        .expect("prepare contexts query");
    let ctx_rows: Vec<(String, Option<String>, Option<String>)> = ctx_stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, Option<String>>(1)?,
                r.get::<_, Option<String>>(2)?,
            ))
        })
        .expect("query contexts")
        .map(|r| r.expect("row"))
        .collect();
    assert_eq!(
        ctx_rows,
        vec![
            (
                "act_create".to_string(),
                Some("act_number".to_string()),
                Some("[X]".to_string())
            ),
            (
                "cartridge_create".to_string(),
                Some("cartridge_code".to_string()),
                Some("C-[XXXX]".to_string())
            ),
            ("device_create".to_string(), None, None),
            (
                "drum_create".to_string(),
                Some("drum_code".to_string()),
                Some("D-[XXXX]".to_string())
            ),
            ("printer_create".to_string(), None, None),
        ],
        "number_template_contexts must map exactly as prescribed by D-13/D-16"
    );
}

/// Test 2: `counters` no longer exists as a table after migration (D-15).
#[test]
fn v041_drops_counters_table() {
    let dir = TempDir::new().expect("tempdir");
    let conn = fresh_migrated_db(&dir, "number-templates-no-counters.db");

    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'counters'",
            [],
            |r| r.get(0),
        )
        .expect("query sqlite_master for counters table");
    assert_eq!(count, 0, "counters table must not exist after V041 (D-15)");
}

/// Test 3: `number_template_contexts.template_id` really is `ON DELETE SET
/// NULL` at the schema level — deleting the referenced template must null
/// out the context row, not error or cascade-delete (D-17 / T-40.2-01).
#[test]
fn v041_context_template_id_set_null_on_delete() {
    let dir = TempDir::new().expect("tempdir");
    let conn = fresh_migrated_db(&dir, "number-templates-fk-set-null.db");

    let before: Option<i64> = conn
        .query_row(
            "SELECT template_id FROM number_template_contexts WHERE context = 'act_create'",
            [],
            |r| r.get(0),
        )
        .expect("read template_id before delete");
    assert!(
        before.is_some(),
        "act_create must have a template assigned before the delete"
    );

    conn.execute("DELETE FROM number_templates WHERE type = 'act_number'", [])
        .expect("delete act_number template");

    let after: Option<i64> = conn
        .query_row(
            "SELECT template_id FROM number_template_contexts WHERE context = 'act_create'",
            [],
            |r| r.get(0),
        )
        .expect("read template_id after delete");
    assert_eq!(
        after, None,
        "ON DELETE SET NULL must null out template_id, not leave a dangling id or error"
    );
}

/// Test 4: re-running migrations against the same connection is a no-op
/// (mirrors `place_movements_migration_reruns_idempotently`).
#[test]
fn v041_migration_reruns_idempotently() {
    let dir = TempDir::new().expect("tempdir");
    let db_path = dir.path().join("number-templates-idempotent.db");

    let mut conn = Connection::open(&db_path).expect("open");
    pragmas::apply_writer_pragmas(&conn).expect("pragmas");

    let report1 = migrations::run(&mut conn).expect("run 1");
    let version_after_first = report1.schema_version;
    assert!(
        version_after_first >= 41,
        "user_version must be at least 41, got {version_after_first}"
    );

    let report2 = migrations::run(&mut conn).expect("run 2");
    assert_eq!(
        report2.applied_count, 0,
        "second run must apply zero migrations"
    );
    assert_eq!(
        report2.schema_version, version_after_first,
        "user_version must not change on idempotent re-run"
    );

    // number_templates content must be unaffected by the re-run (no
    // duplicate seed rows from a migration re-applying).
    let template_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM number_templates", [], |r| r.get(0))
        .expect("count number_templates rows");
    assert_eq!(
        template_count, 3,
        "number_templates must still contain exactly 3 rows after idempotent re-run"
    );
}
