//! Integration test: D-Schema-05 + FOUND-10 — `audit_log` schema + indexes.
//!
//! Asserts the column set and types declared in CONTEXT.md §D-Schema-05,
//! plus the two indexes on (entity_type, entity_id, created_at_utc) and
//! (user_id, created_at_utc).

use std::collections::HashMap;

use trackly_infra::test_support::test_db;

#[derive(Debug)]
struct ColInfo {
    type_: String,
    not_null: bool,
}

fn audit_log_columns(conn: &rusqlite::Connection) -> HashMap<String, ColInfo> {
    let mut stmt = conn
        .prepare("PRAGMA table_info(audit_log)")
        .expect("prepare table_info");
    let rows = stmt
        .query_map([], |r| {
            let name: String = r.get(1)?;
            let type_: String = r.get(2)?;
            let notnull: i64 = r.get(3)?;
            Ok((
                name,
                ColInfo {
                    type_,
                    not_null: notnull != 0,
                },
            ))
        })
        .expect("query_map");
    rows.collect::<Result<_, _>>().expect("collect")
}

#[test]
fn audit_log_has_required_columns_with_correct_types() {
    let (conn, _guard) = test_db();
    let cols = audit_log_columns(&conn);

    // (column, expected_type_prefix, expected_not_null)
    //
    // Note: SQLite's PRAGMA table_info reports `notnull = 0` for INTEGER PRIMARY
    // KEY columns even though they are de-facto NOT NULL (the rowid alias is
    // implicitly non-null). We assert `false` here to match the engine's
    // reported value rather than the conceptual one.
    let expected = [
        ("id", "INTEGER", false),
        ("entity_type", "TEXT", true),
        ("entity_id", "INTEGER", true),
        ("action", "TEXT", true),
        ("user_id", "INTEGER", false),
        ("before_json", "TEXT", false),
        ("after_json", "TEXT", false),
        ("payload_json", "TEXT", false),
        ("created_at_utc", "INTEGER", true),
    ];

    for (name, want_type, want_not_null) in expected {
        let got = cols
            .get(name)
            .unwrap_or_else(|| panic!("audit_log missing column `{name}`"));
        assert!(
            got.type_.eq_ignore_ascii_case(want_type),
            "audit_log.{name} type want `{want_type}`, got `{}`",
            got.type_
        );
        assert_eq!(
            got.not_null, want_not_null,
            "audit_log.{name} not_null want {want_not_null}, got {}",
            got.not_null
        );
    }

    // Hard-delete invariant — no `deleted_at_utc`, no `version`.
    assert!(!cols.contains_key("deleted_at_utc"));
    assert!(!cols.contains_key("version"));
}

#[test]
fn audit_log_has_entity_and_user_indexes() {
    let (conn, _guard) = test_db();

    let mut stmt = conn
        .prepare("PRAGMA index_list(audit_log)")
        .expect("prepare index_list");
    let indexes: Vec<String> = stmt
        .query_map([], |r| r.get::<_, String>(1))
        .expect("query_map")
        .collect::<Result<_, _>>()
        .expect("collect");

    assert!(
        indexes.iter().any(|i| i == "idx_audit_log_entity"),
        "audit_log missing index `idx_audit_log_entity`; got {indexes:?}"
    );
    assert!(
        indexes.iter().any(|i| i == "idx_audit_log_user"),
        "audit_log missing index `idx_audit_log_user`; got {indexes:?}"
    );
}

#[test]
fn idx_audit_log_entity_covers_entity_type_id_created_at_utc() {
    let (conn, _guard) = test_db();

    let mut stmt = conn
        .prepare("PRAGMA index_info(idx_audit_log_entity)")
        .expect("prepare index_info");
    let cols: Vec<String> = stmt
        .query_map([], |r| {
            // (seqno, cid, name)
            r.get::<_, String>(2)
        })
        .expect("query_map")
        .collect::<Result<_, _>>()
        .expect("collect");

    assert_eq!(
        cols,
        vec![
            "entity_type".to_string(),
            "entity_id".to_string(),
            "created_at_utc".to_string(),
        ],
        "idx_audit_log_entity columns out of order or wrong"
    );
}

#[test]
fn idx_audit_log_user_covers_user_id_created_at_utc() {
    let (conn, _guard) = test_db();

    let mut stmt = conn
        .prepare("PRAGMA index_info(idx_audit_log_user)")
        .expect("prepare index_info");
    let cols: Vec<String> = stmt
        .query_map([], |r| r.get::<_, String>(2))
        .expect("query_map")
        .collect::<Result<_, _>>()
        .expect("collect");

    assert_eq!(
        cols,
        vec!["user_id".to_string(), "created_at_utc".to_string()]
    );
}
