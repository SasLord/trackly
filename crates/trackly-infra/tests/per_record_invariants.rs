//! Integration test: D-Schema-02/03/04 invariants across every table.
//!
//! For every USER-MUTABLE table: must have the four standard columns
//! (`created_at_utc INTEGER NOT NULL`, `updated_at_utc INTEGER NOT NULL`,
//! `deleted_at_utc INTEGER NULL`, `version INTEGER NOT NULL DEFAULT 1`).
//!
//! For every SYSTEM table (and junction tables): must NOT have
//! `deleted_at_utc` or `version` columns.
//!
//! For every timestamp column anywhere: must use the `_at_utc` suffix
//! and `INTEGER` type (D-Schema-02).

use std::collections::HashMap;

use trackly_infra::test_support::test_db;

const USER_MUTABLE_TABLES: &[&str] = &[
    "users",
    "locations",
    "devices",
    "acts",
    "cartridges",
    "cartridge_models",
    "requests",
    "document_templates",
];

const SYSTEM_TABLES: &[&str] = &[
    "audit_log",
    "counters",
    "sessions",
    "scheduled_tasks",
    "device_types",
    "device_statuses",
    "cartridge_states",
    "cartridge_statuses",
    // Junction tables — also hard-delete invariant.
    "act_items",
    "cartridge_model_compatibility",
];

#[derive(Debug)]
struct ColInfo {
    type_: String,
    not_null: bool,
    dflt_value: Option<String>,
}

fn table_info(conn: &rusqlite::Connection, table: &str) -> HashMap<String, ColInfo> {
    let sql = format!("PRAGMA table_info({table})");
    let mut stmt = conn.prepare(&sql).expect("prepare table_info");
    let rows = stmt
        .query_map([], |r| {
            // PRAGMA table_info columns: cid, name, type, notnull, dflt_value, pk
            let name: String = r.get(1)?;
            let type_: String = r.get(2)?;
            let notnull: i64 = r.get(3)?;
            let dflt: Option<String> = r.get(4)?;
            Ok((
                name,
                ColInfo {
                    type_,
                    not_null: notnull != 0,
                    dflt_value: dflt,
                },
            ))
        })
        .expect("query_map");
    rows.collect::<Result<HashMap<_, _>, _>>().expect("collect")
}

fn assert_has_standard4(conn: &rusqlite::Connection, table: &str) {
    let info = table_info(conn, table);

    let created = info.get("created_at_utc").unwrap_or_else(|| {
        panic!("table `{table}` missing column `created_at_utc`");
    });
    assert!(
        created.type_.eq_ignore_ascii_case("INTEGER"),
        "`{table}.created_at_utc` must be INTEGER, got {}",
        created.type_
    );
    assert!(
        created.not_null,
        "`{table}.created_at_utc` must be NOT NULL"
    );

    let updated = info.get("updated_at_utc").unwrap_or_else(|| {
        panic!("table `{table}` missing column `updated_at_utc`");
    });
    assert!(
        updated.type_.eq_ignore_ascii_case("INTEGER"),
        "`{table}.updated_at_utc` must be INTEGER"
    );
    assert!(
        updated.not_null,
        "`{table}.updated_at_utc` must be NOT NULL"
    );

    let deleted = info.get("deleted_at_utc").unwrap_or_else(|| {
        panic!("table `{table}` missing column `deleted_at_utc`");
    });
    assert!(
        deleted.type_.eq_ignore_ascii_case("INTEGER"),
        "`{table}.deleted_at_utc` must be INTEGER"
    );
    assert!(
        !deleted.not_null,
        "`{table}.deleted_at_utc` must be NULL (nullable)"
    );

    let version = info.get("version").unwrap_or_else(|| {
        panic!("table `{table}` missing column `version`");
    });
    assert!(
        version.type_.eq_ignore_ascii_case("INTEGER"),
        "`{table}.version` must be INTEGER"
    );
    assert!(version.not_null, "`{table}.version` must be NOT NULL");
    let dflt = version
        .dflt_value
        .as_ref()
        .unwrap_or_else(|| panic!("`{table}.version` must have DEFAULT 1"));
    assert_eq!(
        dflt.trim(),
        "1",
        "`{table}.version` DEFAULT should be `1`, got `{dflt}`"
    );
}

fn assert_no_soft_delete_or_version(conn: &rusqlite::Connection, table: &str) {
    let info = table_info(conn, table);
    assert!(
        !info.contains_key("deleted_at_utc"),
        "system/junction table `{table}` MUST NOT have `deleted_at_utc` (D-Schema-03 hard-delete)"
    );
    assert!(
        !info.contains_key("version"),
        "system/junction table `{table}` MUST NOT have `version` (D-Schema-04 user-mutable only)"
    );
}

#[test]
fn user_mutable_tables_have_standard4_columns() {
    let (conn, _guard) = test_db();
    for table in USER_MUTABLE_TABLES {
        assert_has_standard4(&conn, table);
    }
}

#[test]
fn system_and_junction_tables_lack_soft_delete_and_version() {
    let (conn, _guard) = test_db();
    for table in SYSTEM_TABLES {
        assert_no_soft_delete_or_version(&conn, table);
    }
}

#[test]
fn all_timestamp_columns_use_at_utc_suffix_and_integer_type() {
    let (conn, _guard) = test_db();

    // Discover every non-FTS, non-refinery table.
    let mut stmt = conn
        .prepare(
            "SELECT name FROM sqlite_master \
             WHERE type='table' \
               AND name NOT LIKE 'sqlite_%' \
               AND name NOT LIKE '%_fts%' \
               AND name NOT LIKE 'refinery_%'",
        )
        .expect("prepare sqlite_master");
    let tables: Vec<String> = stmt
        .query_map([], |r| r.get(0))
        .expect("query_map")
        .collect::<Result<_, _>>()
        .expect("collect");

    // Forbidden suffixes — anything that looks like a timestamp but doesn't
    // use the `_at_utc` convention.
    fn looks_like_bad_timestamp_column(name: &str) -> bool {
        let n = name.to_ascii_lowercase();
        // _at_utc is OK
        if n.ends_with("_at_utc") {
            return false;
        }
        n.ends_with("_at")
            || n.ends_with("_date")
            || n.ends_with("_time")
            || n.ends_with("_timestamp")
            || n.ends_with("_datetime")
    }

    // Allowlist for columns that look like timestamps but are NOT.
    // - `act_items.condition_at_time` / `complectation_at_time`: TEXT snapshot
    //   columns capturing the device's *state* at the moment the act was
    //   signed — not a timestamp value.
    // - `sessions.expiry_date`: tower-sessions canonical column name; using
    //   their convention keeps the custom SessionStore impl trivial. The
    //   value IS unix seconds in INTEGER so we still assert that below.
    let allowed_non_utc_timestamp_lookalikes: &[(&str, &str)] = &[
        ("act_items", "condition_at_time"),
        ("act_items", "complectation_at_time"),
        ("sessions", "expiry_date"),
    ];

    let mut offenders: Vec<String> = Vec::new();
    for table in &tables {
        let info = table_info(&conn, table);
        for (col_name, col_info) in &info {
            if col_name.to_ascii_lowercase().ends_with("_at_utc") {
                assert!(
                    col_info.type_.eq_ignore_ascii_case("INTEGER"),
                    "`{table}.{col_name}` is a timestamp column — must be INTEGER, got `{}`",
                    col_info.type_
                );
            } else if looks_like_bad_timestamp_column(col_name) {
                let allowed = allowed_non_utc_timestamp_lookalikes
                    .iter()
                    .any(|(t, c)| *t == table.as_str() && *c == col_name.as_str());
                if !allowed {
                    offenders.push(format!("{table}.{col_name} (type={})", col_info.type_));
                }
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "columns that look like timestamps must use `_at_utc` suffix (D-Schema-02). Offenders: {offenders:?}"
    );

    // Sessions.expiry_date IS a real timestamp — assert it's INTEGER even
    // though it doesn't carry the `_at_utc` suffix (tower-sessions convention).
    let sessions_info = table_info(&conn, "sessions");
    let expiry = sessions_info
        .get("expiry_date")
        .expect("sessions.expiry_date should exist");
    assert!(
        expiry.type_.eq_ignore_ascii_case("INTEGER"),
        "sessions.expiry_date must be INTEGER, got `{}`",
        expiry.type_
    );
}
