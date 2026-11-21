//! Integration test: V040 `place_movements` schema-only migration correctness
//! (Phase 40 Plan 01, HST-01).
//!
//! Mirrors `migration_idempotency.rs`'s harness pattern (fresh tempfile DB, apply
//! pragmas + all embedded migrations, then inspect the resulting schema) rather than
//! its assertions — this file asserts the table, its 8 indexes (5 + 2 from V046 + 1 from V056), and the
//! empty-on-fresh-DB invariant (D-02: no backfill from `audit_log`). V056 (quick 261121-hed,
//! NEW-3): колонка `request_id` и перенос прежних ремонтных строк — на настоящей БД V055.

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

#[test]
fn place_movements_v040_creates_table_and_indexes() {
    let dir = TempDir::new().expect("tempdir");
    let conn = fresh_migrated_db(&dir, "place-movements-schema.db");

    let table_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'place_movements'",
            [],
            |r| r.get(0),
        )
        .expect("query sqlite_master for place_movements table");
    assert_eq!(
        table_count, 1,
        "place_movements table must exist exactly once"
    );

    let index_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'index' AND tbl_name = 'place_movements'",
            [],
            |r| r.get(0),
        )
        .expect("query sqlite_master for place_movements indexes");
    assert_eq!(
        index_count, 8,
        "place_movements must have exactly 8 indexes (5 from V040 + 2 from V046 + 1 from V056), got {index_count}"
    );

    let user_version: i64 = conn
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .expect("read user_version");
    assert!(
        user_version >= 40,
        "user_version must be at least 40, got {user_version}"
    );
}

#[test]
fn place_movements_starts_empty() {
    let dir = TempDir::new().expect("tempdir");
    let conn = fresh_migrated_db(&dir, "place-movements-empty.db");

    let row_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM place_movements", [], |r| r.get(0))
        .expect("query place_movements row count");
    assert_eq!(
        row_count, 0,
        "place_movements must start empty (D-02: no backfill from audit_log)"
    );
}

#[test]
fn place_movements_migration_reruns_idempotently() {
    let dir = TempDir::new().expect("tempdir");
    let db_path = dir.path().join("place-movements-idempotent.db");

    let mut conn = Connection::open(&db_path).expect("open");
    pragmas::apply_writer_pragmas(&conn).expect("pragmas");

    let report1 = migrations::run(&mut conn).expect("run 1");
    let version_after_first = report1.schema_version;

    // Re-run against the same connection: must be a no-op, no error.
    let report2 = migrations::run(&mut conn).expect("run 2");
    assert_eq!(
        report2.applied_count, 0,
        "second run must apply zero migrations"
    );
    assert_eq!(
        report2.schema_version, version_after_first,
        "user_version must not change on idempotent re-run"
    );

    // Independent fresh apply against a second connection must land on the same version.
    let dir2 = TempDir::new().expect("tempdir 2");
    let conn2 = fresh_migrated_db(&dir2, "place-movements-idempotent-2.db");
    let version2: i64 = conn2
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .expect("read user_version 2");
    assert_eq!(
        version2, version_after_first as i64,
        "independent fresh migration run must reach the same schema version"
    );
}

// ---------------------------------------------------------------------------
// V056 (quick 261121-hed, NEW-3): place_movements.request_id + перенос прежних
// ремонтных строк. Настоящая БД V055 через тот же раннер, что у AppCtx::build.
// Только вымышленные данные.
// ---------------------------------------------------------------------------

const NOW: i64 = 1_700_000_000;

fn v055_seeded() -> (Connection, TempDir) {
    let dir = TempDir::new().expect("tempdir");
    let mut conn = Connection::open(dir.path().join("pm-v056.db")).expect("open");
    pragmas::apply_writer_pragmas(&conn).expect("pragmas");
    let report = migrations::run_up_to(&mut conn, 55).expect("up to V055");
    assert_eq!(report.schema_version, 55, "fixture must be a real V055 DB");
    conn.execute_batch(&format!(
        "INSERT INTO users (id, login, full_name, role, created_at_utc, updated_at_utc)
           VALUES (1, 'sidorov', 'Сидоров С.С.', 'admin', {NOW}, {NOW});
         INSERT INTO places (id, parent_id, kind, name, is_storage, created_at_utc, updated_at_utc)
           VALUES (1, NULL, 'building', 'Склад А', 1, {NOW}, {NOW}),
                  (2, NULL, 'building', 'Кабинет 12', 0, {NOW}, {NOW});
         INSERT INTO devices (id, type_id, name, inventory_number, status_id, place_id,
                              created_at_utc, updated_at_utc)
           VALUES (1, 1, 'Принтер #1', 'ИНВ-000001', 2, 1, {NOW}, {NOW});
         -- 1: ремонтная заявка; 2: заявка другого типа.
         INSERT INTO requests (id, request_type, status, requested_by_user_id, printer_device_id,
                               created_at_utc, updated_at_utc)
           VALUES (1, 'repair', 'completed', 1, 1, {NOW}, {NOW}),
                  (2, 'free_form', 'completed', 1, NULL, {NOW}, {NOW});"
    ))
    .expect("seed");
    (conn, dir)
}

fn insert_movement(conn: &Connection, id: i64, entity_type: &str, note: Option<&str>) {
    conn.execute(
        "INSERT INTO place_movements (id, entity_type, entity_id, from_place_id, from_place_path,
            to_place_id, to_place_path, source, note, user_id, created_at_utc)
         VALUES (?1, ?2, 1, 2, 'Кабинет 12', 1, 'Склад А', 'manual', ?3, 1, ?4)",
        rusqlite::params![id, entity_type, note, NOW],
    )
    .expect("insert movement");
}

fn movement(conn: &Connection, id: i64) -> (String, Option<i64>, Option<String>) {
    conn.query_row(
        "SELECT source, request_id, note FROM place_movements WHERE id = ?1",
        [id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )
    .unwrap_or_else(|e| panic!("movement {id}: {e}"))
}

#[test]
fn v056_adds_request_id_and_backfills_only_exact_repair_rows() {
    let (mut conn, _g) = v055_seeded();
    // 10: прежняя ремонтная строка устройства — переносится.
    insert_movement(&conn, 10, "device", Some("по заявке №1"));
    // 11: заявка №2 существует, но не ремонтная — не трогается.
    insert_movement(&conn, 11, "device", Some("по заявке №2"));
    // 12: заявки №99 нет — не трогается.
    insert_movement(&conn, 12, "device", Some("по заявке №99"));
    // 13: ручная пометка с другим текстом — не трогается (не LIKE).
    insert_movement(&conn, 13, "device", Some("по заявке №1, срочно"));
    // 14: каскад картриджа «вместе с принтером» — не переносится.
    insert_movement(&conn, 14, "cartridge", Some("вместе с принтером"));
    // 15: картриджная строка с ремонтным текстом — только устройства.
    insert_movement(&conn, 15, "cartridge", Some("по заявке №1"));
    // 16: без примечания.
    insert_movement(&conn, 16, "device", None);

    let report = migrations::run(&mut conn).expect("migrate to head");
    assert!(report.schema_version >= 56, "{}", report.schema_version);
    let uv: i64 = conn
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .unwrap();
    assert_eq!(uv, i64::from(report.schema_version));

    let has_col: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('place_movements') WHERE name = 'request_id'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(has_col, 1, "колонка request_id должна появиться");

    assert_eq!(movement(&conn, 10), ("repair".to_string(), Some(1), None));
    assert_eq!(
        movement(&conn, 11),
        ("manual".to_string(), None, Some("по заявке №2".to_string()))
    );
    assert_eq!(
        movement(&conn, 12),
        (
            "manual".to_string(),
            None,
            Some("по заявке №99".to_string())
        )
    );
    assert_eq!(
        movement(&conn, 13),
        (
            "manual".to_string(),
            None,
            Some("по заявке №1, срочно".to_string())
        )
    );
    assert_eq!(
        movement(&conn, 14),
        (
            "manual".to_string(),
            None,
            Some("вместе с принтером".to_string())
        )
    );
    assert_eq!(
        movement(&conn, 15),
        ("manual".to_string(), None, Some("по заявке №1".to_string()))
    );
    assert_eq!(movement(&conn, 16), ("manual".to_string(), None, None));
}

#[test]
fn v056_file_ends_with_user_version_56() {
    let sql = include_str!("../../../migrations/V056__place_movements_request_id.sql");
    let last = sql
        .lines()
        .rev()
        .find(|l| !l.trim().is_empty())
        .expect("non-empty migration");
    assert_eq!(last.trim(), "PRAGMA user_version = 56;");
}
