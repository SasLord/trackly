//! Integration test: `V054__request_chat.sql` (Phase 41.5 Plan 01, MSG-02/MSG-04).
//! Только вымышленные данные.

use rusqlite::Connection;
use tempfile::TempDir;

use trackly_infra::db::migrations;
use trackly_infra::db::pragmas::apply_writer_pragmas;

const NOW: i64 = 1_700_000_000;

fn count(conn: &Connection, sql: &str) -> i64 {
    conn.query_row(sql, [], |r| r.get(0)).expect("count")
}

fn cols(conn: &Connection, table: &str) -> Vec<String> {
    conn.prepare(&format!(
        "SELECT name FROM pragma_table_info('{table}') ORDER BY cid"
    ))
    .unwrap()
    .query_map([], |r| r.get(0))
    .unwrap()
    .map(|r| r.unwrap())
    .collect()
}

fn migrated() -> (Connection, TempDir) {
    let dir = TempDir::new().expect("tempdir");
    let mut conn = Connection::open(dir.path().join("chat-mig.db")).expect("open");
    apply_writer_pragmas(&conn).expect("pragmas");
    migrations::run_up_to(&mut conn, 53).expect("up to V053");
    conn.execute_batch(&format!(
        "INSERT INTO users (id, login, full_name, role, created_at_utc, updated_at_utc)
           VALUES (1, 'petrov', 'Петров П.П.', 'admin', {NOW}, {NOW});
         INSERT INTO audit_log (entity_type, entity_id, action, user_id, created_at_utc)
           VALUES ('device', 1, 'create', 1, {NOW}), ('device', 1, 'update', 1, {NOW});
         INSERT INTO attachments (parent_kind, parent_id, original_name, mime, size_bytes,
                                  content_hash, uploaded_by_user_id, created_at_utc)
           VALUES ('request', 1, 'a.png', 'image/png', 3, 'abc', 1, {NOW});"
    ))
    .expect("seed");
    let attachments_before = count(&conn, "SELECT COUNT(*) FROM attachments");
    migrations::run(&mut conn).expect("upgrade");
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM attachments"),
        attachments_before
    );
    (conn, dir)
}

#[test]
fn v054_sets_user_version_and_creates_tables() {
    let (conn, _d) = migrated();
    assert!(count(&conn, "PRAGMA user_version") >= 54);
    assert_eq!(
        cols(&conn, "chat_messages"),
        [
            "id",
            "request_id",
            "author_user_id",
            "body",
            "created_at_utc",
            "moved_from_request_id"
        ]
    );
    assert_eq!(
        cols(&conn, "chat_reads"),
        [
            "user_id",
            "request_id",
            "last_message_id",
            "last_audit_id",
            "updated_at_utc"
        ]
    );
    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) FROM pragma_table_info('attachments') WHERE name='message_id'"
        ),
        1
    );
    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) FROM attachments WHERE message_id IS NULL"
        ),
        1
    );
}

#[test]
fn v054_writes_unread_baseline() {
    let (conn, _d) = migrated();
    let max: i64 = count(&conn, "SELECT MAX(id) FROM audit_log");
    let v: String = conn
        .query_row(
            "SELECT value FROM app_settings WHERE key='chat_unread_baseline_audit_id'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(v, max.to_string());
}

#[test]
fn v054_baseline_is_zero_for_empty_audit() {
    let dir = TempDir::new().unwrap();
    let mut conn = Connection::open(dir.path().join("e.db")).unwrap();
    apply_writer_pragmas(&conn).unwrap();
    migrations::run_up_to(&mut conn, 53).unwrap();
    conn.execute("DELETE FROM audit_log", []).unwrap();
    migrations::run(&mut conn).unwrap();
    let v: String = conn
        .query_row(
            "SELECT value FROM app_settings WHERE key='chat_unread_baseline_audit_id'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(v, "0");
}

#[test]
fn v054_indexes_and_pk() {
    let (conn, _d) = migrated();
    for idx in ["idx_chat_messages_request", "idx_attachments_message"] {
        assert_eq!(
            count(
                &conn,
                &format!("SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='{idx}'")
            ),
            1,
            "{idx}"
        );
    }
    let pk: Vec<String> = conn
        .prepare("SELECT name FROM pragma_table_info('chat_reads') WHERE pk > 0 ORDER BY pk")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    assert_eq!(pk, ["user_id", "request_id"]);
}

#[test]
fn v055_creates_chat_mutes_and_ends_with_user_version() {
    let (conn, _d) = migrated();
    assert!(count(&conn, "PRAGMA user_version") >= 55);
    assert_eq!(
        cols(&conn, "chat_mutes"),
        ["user_id", "request_id", "muted_at_utc"]
    );
    let pk: Vec<String> = conn
        .prepare("SELECT name FROM pragma_table_info('chat_mutes') WHERE pk > 0 ORDER BY pk")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    assert_eq!(pk, ["user_id", "request_id"]);

    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../migrations/V055__chat_mutes.sql"
    );
    let sql = std::fs::read_to_string(path).expect("read V055");
    let last = sql.lines().rev().find(|l| !l.trim().is_empty()).unwrap();
    assert_eq!(last.trim(), "PRAGMA user_version = 55;");
    let code: String = sql
        .lines()
        .filter(|l| !l.trim_start().starts_with("--"))
        .collect::<Vec<_>>()
        .join("\n")
        .to_uppercase();
    assert!(!code.contains("DROP"), "V055 must not rebuild tables");
}
