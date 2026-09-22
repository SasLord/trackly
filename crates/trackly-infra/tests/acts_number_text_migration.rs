//! Integration test: `V042__acts_number_text.sql` — `acts.number` INTEGER →
//! TEXT rebuild (Phase 40.2 Plan 06, NUM-14), exercised through the REAL
//! migration runner (`trackly_infra::db::migrations`), exactly as
//! `AppCtx::build` runs it on an upgrade:
//!
//!   1. `apply_writer_pragmas` (`foreign_keys = ON`, WAL, ...),
//!   2. `migrations::run_up_to(41)` — a genuine pre-V042 database,
//!   3. seed acts + every FK child of `acts` (`act_items` CASCADE, a return
//!      act's `parent_act_id` RESTRICT, `place_movements.act_id` SET NULL),
//!   4. `migrations::run` — the remaining migrations (V042+) through the
//!      same refinery pipeline, one transaction per file.
//!
//! BE-CR-01 (code review 40.2): `PRAGMA foreign_keys = OFF` inside a
//! migration file is a no-op (refinery wraps each file in a transaction and
//! SQLite ignores the pragma there), so with FKs left ON the rebuild's
//! `DROP TABLE acts` cascade-deleted every `act_items` row, nulled
//! `place_movements.act_id` and failed outright when a return existed. The
//! runner now disables FKs on the connection before refinery starts. These
//! tests pin zero data loss and preserved links. Fictional names only.

use rusqlite::{params, Connection};
use tempfile::TempDir;

use trackly_infra::db::migrations;
use trackly_infra::db::pragmas::apply_writer_pragmas;

const NOW: i64 = 1_700_000_000;

fn conn_at_v041() -> (Connection, TempDir) {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("acts-number-text-migration.db");
    let mut conn = Connection::open(&path).expect("open");
    apply_writer_pragmas(&conn).expect("writer pragmas");
    let report = migrations::run_up_to(&mut conn, 41).expect("run migrations up to V041");
    assert_eq!(report.schema_version, 41, "fixture must be a real V041 DB");
    (conn, dir)
}

/// Seed a realistic pre-V042 data set with every FK child of `acts`
/// populated. Returns nothing — ids are fixed so assertions can name them.
fn seed_acts_with_children(conn: &Connection) {
    conn.execute_batch(&format!(
        "INSERT INTO users (id, login, full_name, role, created_at_utc, updated_at_utc)
           VALUES (1, 'ivanov', 'Иванов И.И.', 'admin', {NOW}, {NOW});
         INSERT INTO places (id, parent_id, kind, name, is_storage, created_at_utc, updated_at_utc)
           VALUES (1, NULL, 'building', 'Склад А', 1, {NOW}, {NOW}),
                  (2, NULL, 'building', 'Корпус Б', 0, {NOW}, {NOW});
         INSERT INTO devices (id, type_id, name, inventory_number, status_id, place_id,
                              created_at_utc, updated_at_utc)
           VALUES (1, 1, 'Ноутбук', 'ИНВ-000001', 2, 2, {NOW}, {NOW}),
                  (2, 1, 'Монитор', 'ИНВ-000002', 2, 2, {NOW}, {NOW});

         -- Handover #42, its partial return (42в), and an unrelated handover #7.
         INSERT INTO acts (id, number, sub_number, parent_act_id, act_type, giver_name,
                           receiver_name, created_at_utc, updated_at_utc, version,
                           handover_date_utc)
           VALUES (1, 42, NULL, NULL, 'handover', 'Иванов И.И.', 'Петров П.П.', {NOW}, {NOW}, 1, {NOW}),
                  (2, 42, 1, 1, 'return', 'Петров П.П.', 'Иванов И.И.', {NOW}, {NOW}, 1, {NOW}),
                  (3, 7, NULL, NULL, 'handover', 'Сидоров С.С.', 'Кузнецов К.К.', {NOW}, {NOW}, 1, {NOW});

         INSERT INTO act_items (id, act_id, device_id, quantity)
           VALUES (1, 1, 1, 1),
                  (2, 1, 2, 1),
                  (3, 2, 1, 1),
                  (4, 3, 2, 1);

         INSERT INTO place_movements (id, entity_type, entity_id, from_place_id, from_place_path,
                                      to_place_id, to_place_path, source, act_id, user_id,
                                      created_at_utc)
           VALUES (1, 'device', 1, 1, 'Склад А', 2, 'Корпус Б', 'act', 1, 1, {NOW}),
                  (2, 'device', 1, 2, 'Корпус Б', 1, 'Склад А', 'act', 2, 1, {NOW}),
                  (3, 'device', 2, 1, 'Склад А', 2, 'Корпус Б', 'act', 3, 1, {NOW});"
    ))
    .expect("seed pre-V042 acts + children");
}

fn act_items(conn: &Connection) -> Vec<(i64, i64, i64)> {
    let mut stmt = conn
        .prepare("SELECT id, act_id, device_id FROM act_items ORDER BY id")
        .expect("prepare act_items");
    stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .expect("query act_items")
        .map(|r| r.expect("row"))
        .collect()
}

fn movement_links(conn: &Connection) -> Vec<(i64, Option<i64>)> {
    let mut stmt = conn
        .prepare("SELECT id, act_id FROM place_movements ORDER BY id")
        .expect("prepare place_movements");
    stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .expect("query place_movements")
        .map(|r| r.expect("row"))
        .collect()
}

#[test]
fn upgrade_through_real_runner_keeps_every_act_child_row_and_link() {
    let (mut conn, _guard) = conn_at_v041();
    seed_acts_with_children(&conn);

    let items_before = act_items(&conn);
    let links_before = movement_links(&conn);
    assert_eq!(items_before.len(), 4);

    let report = migrations::run(&mut conn).expect(
        "upgrade V041 -> latest must succeed even with a return act \
         (parent_act_id ON DELETE RESTRICT) present",
    );
    assert_eq!(report.schema_version, migrations::max_known_version());

    // Zero data loss in the CASCADE child.
    assert_eq!(
        act_items(&conn),
        items_before,
        "act_items must survive the acts rebuild untouched (ON DELETE CASCADE must not fire)"
    );
    // Links preserved in the SET NULL child.
    assert_eq!(
        movement_links(&conn),
        links_before,
        "place_movements.act_id must keep pointing at the same acts (ON DELETE SET NULL must not fire)"
    );

    // FK enforcement is back ON after the run, and the DB is consistent.
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

    // The FKs of the children now resolve against the REBUILT acts table:
    // deleting the parent handover is still RESTRICTed by its return.
    let restricted = conn.execute("DELETE FROM acts WHERE id = 1", []);
    assert!(
        restricted.is_err(),
        "returns' parent_act_id must still RESTRICT deleting the parent after the rebuild"
    );
}

#[test]
fn migrates_integer_number_to_text_preserving_ids_and_unique_index() {
    let (mut conn, _guard) = conn_at_v041();
    seed_acts_with_children(&conn);
    migrations::run(&mut conn).expect("upgrade");

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
        "number must become TEXT holding the exact original digits; ids / \
         sub_number / parent_act_id unchanged"
    );

    // The recreated unique index rejects a live duplicate (number, sub_number).
    let dup = conn.execute(
        "INSERT INTO acts (number, sub_number, parent_act_id, act_type, giver_name, \
         receiver_name, created_at_utc, updated_at_utc, version, handover_date_utc) \
         VALUES ('42', NULL, NULL, 'handover', 'Дубликат Д.Д.', 'Тестов Т.Т.', ?1, ?1, 1, ?1)",
        params![NOW + 30],
    );
    assert!(
        dup.is_err(),
        "duplicate (number, sub_number) must be rejected"
    );
}

#[test]
fn upgrade_of_empty_v041_db_reaches_latest() {
    let (mut conn, _guard) = conn_at_v041();
    let report = migrations::run(&mut conn).expect("upgrade empty V041 DB");
    assert_eq!(report.schema_version, migrations::max_known_version());
    assert_eq!(
        report.applied_count,
        (migrations::max_known_version() - 41) as usize
    );
}

/// Without any return act the old bug was SILENT: the upgrade "succeeded"
/// while `DROP TABLE acts` cascade-deleted every `act_items` row. Pin that
/// the items survive when nothing would make the migration fail loudly.
#[test]
fn upgrade_without_returns_does_not_silently_cascade_act_items() {
    let (mut conn, _guard) = conn_at_v041();
    seed_acts_with_children(&conn);
    conn.execute_batch(
        "DELETE FROM place_movements WHERE act_id = 2;
         DELETE FROM act_items WHERE act_id = 2;
         DELETE FROM acts WHERE id = 2;",
    )
    .expect("drop the return act from the fixture");
    let items_before = act_items(&conn);
    let links_before = movement_links(&conn);
    assert_eq!(items_before.len(), 3);

    migrations::run(&mut conn).expect("upgrade");

    assert_eq!(act_items(&conn), items_before, "no act_items may be lost");
    assert_eq!(
        movement_links(&conn),
        links_before,
        "no act links may be nulled"
    );
}

/// BE-WR-11: the rebuild must carry the AUTOINCREMENT high-water mark over,
/// so an id of a physically deleted act is never handed out again.
#[test]
fn rebuild_keeps_autoincrement_high_water_mark() {
    let (mut conn, _guard) = conn_at_v041();
    seed_acts_with_children(&conn);
    conn.execute_batch(&format!(
        "INSERT INTO acts (id, number, act_type, giver_name, receiver_name,
                           created_at_utc, updated_at_utc, version, handover_date_utc)
           VALUES (10, 99, 'handover', 'Иванов И.И.', 'Петров П.П.', {NOW}, {NOW}, 1, {NOW});
         DELETE FROM acts WHERE id = 10;"
    ))
    .expect("create and physically delete act #10");

    migrations::run(&mut conn).expect("upgrade");

    conn.execute(
        "INSERT INTO acts (number, act_type, giver_name, receiver_name, \
         created_at_utc, updated_at_utc, version, handover_date_utc) \
         VALUES ('100', 'handover', 'Иванов И.И.', 'Петров П.П.', ?1, ?1, 1, ?1)",
        params![NOW],
    )
    .expect("insert new act");
    assert_eq!(
        conn.last_insert_rowid(),
        11,
        "the new act must not reuse id 10 of the physically deleted act"
    );
}
