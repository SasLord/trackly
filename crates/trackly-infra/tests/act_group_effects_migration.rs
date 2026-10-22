//! Integration test: `V049__act_group_effects.sql`
//! (Phase 41.2 Plan 02, GAM-02 / GAM-03, D-08 / D-09), прогнанный через
//! НАСТОЯЩИЙ раннер: `run_up_to(48)` -> посев -> `run`.
//!
//! Миграция обязана быть чисто аддитивной: канарейка `act_items` доказывает,
//! что урок V042 (перестройка родителя стирает детей) не повторён.
//! Только вымышленные данные.

use rusqlite::{params, Connection};
use tempfile::TempDir;

use trackly_infra::db::migrations;
use trackly_infra::db::pragmas::apply_writer_pragmas;

const NOW: i64 = 1_700_000_000;

fn conn_at_v048() -> (Connection, TempDir) {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("act-group-effects-migration.db");
    let mut conn = Connection::open(&path).expect("open");
    apply_writer_pragmas(&conn).expect("writer pragmas");
    let report = migrations::run_up_to(&mut conn, 48).expect("run migrations up to V048");
    assert_eq!(report.schema_version, 48, "fixture must be a real V048 DB");
    (conn, dir)
}

fn seed(conn: &Connection) {
    conn.execute_batch(&format!(
        "INSERT INTO users (id, login, full_name, role, created_at_utc, updated_at_utc)
           VALUES (1, 'petrov', 'Петров П.П.', 'admin', {NOW}, {NOW});
         INSERT INTO places (id, parent_id, kind, name, is_storage, created_at_utc, updated_at_utc)
           VALUES (1, NULL, 'building', 'Склад А', 1, {NOW}, {NOW});
         INSERT INTO devices (id, type_id, name, inventory_number, status_id, place_id,
                              created_at_utc, updated_at_utc)
           VALUES (1, 1, 'Монитор #2', 'ИНВ-000001', 2, 1, {NOW}, {NOW});
         INSERT INTO acts (id, number, sub_number, parent_act_id, act_type, giver_name,
                           receiver_name, created_at_utc, updated_at_utc, version,
                           handover_date_utc)
           VALUES (1, '42', NULL, NULL, 'handover', 'Петров П.П.', 'Иванов И.И.', {NOW}, {NOW}, 1, {NOW});
         INSERT INTO act_items (id, act_id, device_id, quantity) VALUES (1, 1, 1, 1);"
    ))
    .expect("seed V048 data");
}

fn upgraded() -> (Connection, TempDir) {
    let (mut conn, guard) = conn_at_v048();
    seed(&conn);
    migrations::run(&mut conn).expect("upgrade V048 -> latest");
    conn.execute_batch("PRAGMA foreign_keys = ON;")
        .expect("enable fk");
    (conn, guard)
}

fn count(conn: &Connection, sql: &str) -> i64 {
    conn.query_row(sql, [], |r| r.get(0)).expect("count query")
}

#[test]
fn v049_creates_table_and_index() {
    let (conn, _guard) = upgraded();
    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'act_group_effects'"
        ),
        1
    );
    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'index' AND name = 'idx_act_group_effects_act'"
        ),
        1
    );
    let uv: i64 = conn
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .expect("user_version");
    assert!(uv >= 49, "user_version must be at least 49, got {uv}");
}

#[test]
fn v049_table_has_no_foreign_keys_and_no_check() {
    let (conn, _guard) = upgraded();
    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) FROM pragma_foreign_key_list('act_group_effects')"
        ),
        0,
        "act_group_effects must not carry foreign keys (D-09)"
    );
    let sql: String = conn
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'act_group_effects'",
            [],
            |r| r.get(0),
        )
        .expect("table sql");
    assert!(
        !sql.to_uppercase().contains("CHECK"),
        "kind token must be validated in Rust, not by SQL CHECK: {sql}"
    );
}

#[test]
fn v049_canary_act_items_survive_migration() {
    let (conn, _guard) = upgraded();
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM act_items"), 1);
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM acts"), 1);
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM devices"), 1);
}

#[test]
fn v049_effect_with_unknown_parents_is_accepted_under_fk_on() {
    let (conn, _guard) = upgraded();
    conn.execute(
        "INSERT INTO act_group_effects
           (act_id, kind, group_id, group_label, device_id, created_at_utc)
         VALUES (?1, 'release', ?2, 'Группа без родителя', ?3, ?4)",
        params![987_654, 123_456, 555_555, NOW],
    )
    .expect("effect without parent rows must insert with foreign_keys = ON");
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM act_group_effects"), 1);
}
