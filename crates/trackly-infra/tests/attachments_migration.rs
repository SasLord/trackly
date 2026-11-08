//! Integration test: `V050__attachments.sql` (Phase 41.3 Plan 01, ATT-01, D-01).
//! Только вымышленные данные.

use rusqlite::Connection;
use tempfile::TempDir;

use trackly_infra::db::migrations;
use trackly_infra::db::pragmas::apply_writer_pragmas;

const NOW: i64 = 1_700_000_000;

fn count(conn: &Connection, sql: &str) -> i64 {
    conn.query_row(sql, [], |r| r.get(0)).expect("count")
}

#[test]
fn v050_creates_table_indexes_and_keeps_children() {
    let dir = TempDir::new().expect("tempdir");
    let mut conn = Connection::open(dir.path().join("att-mig.db")).expect("open");
    apply_writer_pragmas(&conn).expect("pragmas");
    migrations::run_up_to(&mut conn, 49).expect("up to V049");
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
    .expect("seed");
    let items_before = count(&conn, "SELECT COUNT(*) FROM act_items");
    let acts_before = count(&conn, "SELECT COUNT(*) FROM acts");
    migrations::run(&mut conn).expect("upgrade");

    assert_eq!(count(&conn, "SELECT COUNT(*) FROM act_items"), items_before);
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM acts"), acts_before);
    // Каждая миграция обязана сама выставлять user_version (D-Migrations-02):
    // без этого версия схемы застревает на 49, и защита от даунгрейда врёт.
    assert_eq!(count(&conn, "PRAGMA user_version"), 50);

    let cols: Vec<String> = conn
        .prepare("SELECT name FROM pragma_table_info('attachments') ORDER BY cid")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    assert_eq!(
        cols,
        [
            "id",
            "parent_kind",
            "parent_id",
            "original_name",
            "mime",
            "size_bytes",
            "content_hash",
            "uploaded_by_user_id",
            "created_at_utc",
            "deleted_at_utc",
            "deleted_by_user_id"
        ]
    );
    for idx in ["idx_attachments_parent", "idx_attachments_hash"] {
        assert_eq!(
            count(
                &conn,
                &format!("SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='{idx}'")
            ),
            1,
            "{idx}"
        );
    }
}
