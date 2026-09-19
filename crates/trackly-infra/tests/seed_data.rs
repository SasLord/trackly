//! Integration test: lookup tables seeded per D-Migrations-01.
//!
//! Asserts the EXACT seed rows for `device_types`, `device_statuses`,
//! `cartridge_states`, `cartridge_statuses`, and `counters`. Cyrillic
//! strings are compared as `String` (not `Cow`) so any silent encoding
//! drift fails loudly.

use trackly_infra::test_support::test_db;

fn select_names(conn: &rusqlite::Connection, table: &str) -> Vec<String> {
    let sql = format!("SELECT name FROM {table} ORDER BY id");
    let mut stmt = conn.prepare(&sql).expect("prepare");
    let rows = stmt
        .query_map([], |r| r.get::<_, String>(0))
        .expect("query_map");
    rows.collect::<Result<Vec<_>, _>>().expect("collect")
}

#[test]
fn device_types_seed_matches_d_migrations_01() {
    let (conn, _guard) = test_db();
    let got = select_names(&conn, "device_types");
    assert_eq!(got, vec!["Устройство".to_string(), "Принтер".to_string()]);
}

#[test]
fn device_statuses_seed_matches_d_migrations_01() {
    let (conn, _guard) = test_db();
    let got = select_names(&conn, "device_statuses");
    assert_eq!(
        got,
        vec![
            "На складе".to_string(),
            "В работе".to_string(),
            "На ремонте".to_string(),
            "Списано".to_string(),
        ]
    );
}

#[test]
fn cartridge_states_seed_matches_d_migrations_01() {
    let (conn, _guard) = test_db();
    let got = select_names(&conn, "cartridge_states");
    assert_eq!(
        got,
        vec![
            // kind 1 (картриджи) — состояние заряда
            "Полный".to_string(),
            "Частичный".to_string(),
            "Пустой".to_string(),
            // kind 2 (фотобарабаны) — состояние (V017)
            "Новый".to_string(),
            "Изношенный".to_string(),
            "Отработанный".to_string(),
        ]
    );
}

#[test]
fn cartridge_statuses_seed_matches_d_migrations_01() {
    let (conn, _guard) = test_db();
    let got = select_names(&conn, "cartridge_statuses");
    assert_eq!(
        got,
        vec![
            "На складе".to_string(),
            "В работе".to_string(),
            "На заправке".to_string(),
            "Списано".to_string(),
        ]
    );
}

// `counters_seeded_with_act_number_and_cartridge_seq` (asserted the
// `counters` table was seeded with `act_number`/`cartridge_seq`/`drum_seq`)
// was removed here (Phase 40.2 Plan 06, NUM-13/NUM-14): `V041__number_templates.sql`
// (Plan 01) already unconditionally `DROP TABLE counters` — the table this
// test queried has not existed since V041, for ANY of its three counters,
// not just `act_number`. The act-numbering half of this obsolete assertion
// is fully superseded by `number_templates_migration.rs::v041_drops_counters_table`
// (asserts the table is gone) and `number_templates_migration.rs::v041_seeds_expected_templates_and_contexts`
// (asserts the replacement `number_templates`/`number_template_contexts` seed
// rows). The cartridge_seq/drum_seq half — Plan 06's own tracked gap, since
// those two counters were still read via `cartridges_sqlite.rs
// ::assign_code_in_tx` against the same now-nonexistent table — is now
// CLOSED by Plan 07 (NUM-13): `assign_code_in_tx` no longer reads/increments
// any `counters` row at all (explicit code only); `increment_counter_in_tx`
// itself was deleted as part of that same change.
