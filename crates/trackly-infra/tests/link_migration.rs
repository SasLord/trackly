//! Integration test: `V060__link_devices.sql` (Phase 44 Plan 02, D-03/D-12/D-13).
//! Только вымышленные данные.

use rusqlite::Connection;
use tempfile::TempDir;

use trackly_infra::db::migrations;
use trackly_infra::db::pragmas::apply_writer_pragmas;

const NOW: i64 = 1_700_000_000;

fn count(conn: &Connection, sql: &str) -> i64 {
    conn.query_row(sql, [], |r| r.get(0)).expect("count")
}

fn open_v60(dir: &TempDir) -> Connection {
    let mut conn = Connection::open(dir.path().join("link-mig.db")).expect("open");
    apply_writer_pragmas(&conn).expect("pragmas");
    migrations::run_up_to(&mut conn, 59).expect("up to V059");
    conn.execute_batch(&format!(
        "INSERT INTO users (id, login, full_name, role, created_at_utc, updated_at_utc)
           VALUES (1, 'ivanov', 'Иванов И.И.', 'admin', {NOW}, {NOW});"
    ))
    .expect("seed");
    migrations::run_up_to(&mut conn, 60).expect("up to V060");
    conn
}

#[test]
fn v060_creates_tables_indexes_and_keeps_data() {
    let dir = TempDir::new().unwrap();
    let conn = open_v60(&dir);
    assert_eq!(count(&conn, "PRAGMA user_version"), 60);
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM users"), 1);
    for t in ["link_devices", "link_logins", "link_invites"] {
        assert_eq!(
            count(
                &conn,
                &format!("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='{t}'")
            ),
            1,
            "{t}"
        );
    }
    for i in ["idx_link_logins_user", "idx_link_logins_device"] {
        assert_eq!(
            count(
                &conn,
                &format!("SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='{i}'")
            ),
            1,
            "{i}"
        );
    }
}

#[test]
fn device_id_is_primary_key() {
    let dir = TempDir::new().unwrap();
    let conn = open_v60(&dir);
    let ins = "INSERT INTO link_devices (device_id, secret_hash, hostname, first_seen_utc)
               VALUES ('d-1', x'01', 'PC-EXAMPLE-01', 1700000000)";
    conn.execute_batch(ins).unwrap();
    assert!(conn.execute_batch(ins).is_err());
}

#[test]
fn hash_columns_unique_and_sso_tokens_nullable() {
    let dir = TempDir::new().unwrap();
    let conn = open_v60(&dir);
    let inv = |h: &str| {
        conn.execute_batch(&format!(
            "INSERT INTO link_invites (code_hash, created_by_user_id, created_at_utc, expires_at_utc)
             VALUES (x'{h}', 1, {NOW}, {NOW})"
        ))
    };
    inv("AA").unwrap();
    assert!(inv("AA").is_err());
    let login = |h: &str| {
        conn.execute_batch(&format!(
            "INSERT INTO link_logins (device_id, user_id, method, token_hash, created_at_utc)
             VALUES ('d-1', 1, 'sso', {h}, {NOW})"
        ))
    };
    login("NULL").unwrap();
    login("NULL").unwrap();
    login("x'BB'").unwrap();
    assert!(login("x'BB'").is_err());
    assert!(conn
        .execute_batch(&format!(
            "INSERT INTO link_logins (device_id, user_id, method, created_at_utc)
             VALUES ('d-1', 1, 'bogus', {NOW})"
        ))
        .is_err());
}

#[test]
fn migration_file_ends_with_user_version_and_is_additive() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../migrations/V060__link_devices.sql");
    let sql = std::fs::read_to_string(path).expect("read");
    let last = sql
        .lines()
        .map(str::trim)
        .rfind(|l| !l.is_empty() && !l.starts_with("--"))
        .unwrap();
    assert_eq!(last, "PRAGMA user_version = 60;");
    let body: String = sql
        .lines()
        .filter(|l| !l.trim_start().starts_with("--"))
        .collect::<Vec<_>>()
        .join("\n")
        .to_uppercase();
    assert!(!body.contains("DROP") && !body.contains("REFERENCES"));
}
