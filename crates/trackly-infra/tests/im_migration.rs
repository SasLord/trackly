//! Integration test: `V058__im_core.sql` и `V059__idempotency_keys.sql`
//! (Phase 43 Plan 01, DLV-02/DLV-03). Только вымышленные данные.

use rusqlite::Connection;
use trackly_infra::test_support::test_db;

const NOW: i64 = 1_700_000_000;

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

fn last_line(file: &str) -> String {
    let sql = std::fs::read_to_string(format!(
        "{}/../../migrations/{file}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    sql.trim_end().lines().last().unwrap().trim().to_string()
}

fn seed_user(conn: &Connection) {
    conn.execute_batch(&format!(
        "INSERT INTO users (id, login, full_name, role, created_at_utc, updated_at_utc)
           VALUES (1, 'petrov', 'Петров П.П.', 'admin', {NOW}, {NOW});"
    ))
    .unwrap();
}

fn insert_request(conn: &Connection, client_id: Option<&str>) -> rusqlite::Result<usize> {
    conn.execute(
        "INSERT INTO requests (request_type, status, requested_by_user_id, description,
                               created_at_utc, updated_at_utc, client_request_id)
         VALUES ('free_form', 'open', 1, 'тест', ?1, ?1, ?2)",
        rusqlite::params![NOW, client_id],
    )
}

fn insert_chat(
    conn: &Connection,
    request_id: i64,
    client_id: Option<&str>,
) -> rusqlite::Result<usize> {
    conn.execute(
        "INSERT INTO chat_messages (request_id, author_user_id, body, created_at_utc, client_msg_id)
         VALUES (?1, 1, 'привет', ?2, ?3)",
        rusqlite::params![request_id, NOW, client_id],
    )
}

fn insert_im(conn: &Connection, seq: i64, client_id: Option<&str>) -> rusqlite::Result<usize> {
    conn.execute(
        "INSERT INTO im_messages (conversation_id, seq, rev, author_user_id, client_msg_id,
                                  body_json, body_text, created_at_utc)
         VALUES (1, ?1, ?1, 1, ?2, '{}', 'x', ?3)",
        rusqlite::params![seq, client_id, NOW],
    )
}

#[test]
fn v058_creates_im_tables_with_columns() {
    let (conn, _d) = test_db();
    let v: i64 = conn
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .unwrap();
    assert!(v >= 59, "user_version {v}");
    assert_eq!(
        cols(&conn, "im_conversations"),
        [
            "id",
            "kind",
            "title",
            "dialog_key",
            "created_by_user_id",
            "created_at_utc",
            "last_seq"
        ]
    );
    assert_eq!(
        cols(&conn, "im_members"),
        [
            "conversation_id",
            "user_id",
            "role",
            "joined_at_utc",
            "left_at_utc",
            "left_seq",
            "visible_from_seq",
            "last_read_seq",
            "muted"
        ]
    );
    let m = cols(&conn, "im_messages");
    for c in [
        "seq",
        "rev",
        "client_msg_id",
        "body_json",
        "body_text",
        "reply_to_id",
        "purged_at_utc",
    ] {
        assert!(m.iter().any(|x| x == c), "нет колонки {c}");
    }
}

#[test]
fn v058_unique_constraints_on_im_messages() {
    let (conn, _d) = test_db();
    seed_user(&conn);
    conn.execute(
        "INSERT INTO im_conversations (id, kind, created_by_user_id, created_at_utc)
         VALUES (1, 'room', 1, ?1)",
        [NOW],
    )
    .unwrap();
    insert_im(&conn, 1, Some("c-1")).unwrap();
    // тот же seq — отказ
    assert!(insert_im(&conn, 1, Some("c-2")).is_err(), "дубль seq");
    // тот же client id у того же автора — отказ
    assert!(insert_im(&conn, 2, Some("c-1")).is_err(), "дубль client id");
    // две строки с NULL проходят
    insert_im(&conn, 3, None).unwrap();
    insert_im(&conn, 4, None).unwrap();
}

#[test]
fn v059_partial_unique_indexes() {
    let (conn, _d) = test_db();
    seed_user(&conn);
    insert_request(&conn, None).unwrap();
    insert_request(&conn, None).unwrap();
    insert_request(&conn, Some("req-a")).unwrap();
    assert!(
        insert_request(&conn, Some("req-a")).is_err(),
        "дубль заявки"
    );
    insert_request(&conn, Some("req-b")).unwrap();

    // чат: NULL-ы проходят
    insert_chat(&conn, 1, None).unwrap();
    insert_chat(&conn, 1, None).unwrap();
    insert_chat(&conn, 1, Some("m-1")).unwrap();
    assert!(
        insert_chat(&conn, 1, Some("m-1")).is_err(),
        "дубль сообщения"
    );
    // тот же id в другой заявке проходит
    insert_chat(&conn, 2, Some("m-1")).unwrap();
}

#[test]
fn migration_files_end_with_user_version() {
    assert_eq!(last_line("V058__im_core.sql"), "PRAGMA user_version = 58;");
    assert_eq!(
        last_line("V059__idempotency_keys.sql"),
        "PRAGMA user_version = 59;"
    );
}
