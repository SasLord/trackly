//! Integration test: `V047__device_statuses_lifecycle.sql`
//! (Phase 41.1 Plan 01, GRD-04), прогнанный через НАСТОЯЩИЙ раннер миграций
//! так же, как `AppCtx::build` обновляет существующую БД:
//! `run_up_to(46)` -> посев -> `run`.
//!
//! Миграция чисто аддитивная (урок V042: перестройка таблицы в
//! refinery-транзакции стирала `act_items`). Тесты фиксируют: порядок
//! жизненного цикла по `sort_order`, переименование по `code`, нетронутые
//! `devices.status_id`, отсутствие потерь данных. Только вымышленные данные.

use rusqlite::Connection;
use tempfile::TempDir;

use trackly_infra::db::migrations;
use trackly_infra::db::pragmas::apply_writer_pragmas;

const NOW: i64 = 1_700_000_000;

fn conn_at_v046() -> (Connection, TempDir) {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("device-statuses-migration.db");
    let mut conn = Connection::open(&path).expect("open");
    apply_writer_pragmas(&conn).expect("writer pragmas");
    let report = migrations::run_up_to(&mut conn, 46).expect("run migrations up to V046");
    assert_eq!(report.schema_version, 46, "fixture must be a real V046 DB");
    (conn, dir)
}

/// Посев на V046: устройство в статусе 3 («На ремонте») и строка act_items.
fn seed(conn: &Connection) {
    conn.execute_batch(&format!(
        "INSERT INTO users (id, login, full_name, role, created_at_utc, updated_at_utc)
           VALUES (1, 'ivanov', 'Иванов И.И.', 'admin', {NOW}, {NOW});
         INSERT INTO places (id, parent_id, kind, name, is_storage, created_at_utc, updated_at_utc)
           VALUES (1, NULL, 'building', 'Склад А', 1, {NOW}, {NOW}),
                  (2, NULL, 'building', 'Кабинет 101', 0, {NOW}, {NOW});
         INSERT INTO devices (id, type_id, name, inventory_number, status_id, place_id,
                              created_at_utc, updated_at_utc)
           VALUES (1, 1, 'Ноутбук', 'ИНВ-000001', 3, 2, {NOW}, {NOW});
         INSERT INTO acts (id, number, sub_number, parent_act_id, act_type, giver_name,
                           receiver_name, created_at_utc, updated_at_utc, version,
                           handover_date_utc)
           VALUES (1, '42', NULL, NULL, 'handover', 'Иванов И.И.', 'Петров П.П.', {NOW}, {NOW}, 1, {NOW});
         INSERT INTO act_items (id, act_id, device_id, quantity) VALUES (1, 1, 1, 1);"
    ))
    .expect("seed V046 data");
}

fn count(conn: &Connection, sql: &str) -> i64 {
    conn.query_row(sql, [], |r| r.get(0)).expect("count query")
}

fn upgraded() -> (Connection, TempDir) {
    let (mut conn, guard) = conn_at_v046();
    seed(&conn);
    migrations::run(&mut conn).expect("upgrade V046 -> latest");
    (conn, guard)
}

#[test]
fn v047_statuses_ordered_by_lifecycle_sort_order() {
    let (conn, _guard) = upgraded();
    let mut stmt = conn
        .prepare("SELECT code FROM device_statuses ORDER BY sort_order")
        .expect("prepare");
    let codes: Vec<String> = stmt
        .query_map([], |r| r.get::<_, String>(0))
        .expect("query")
        .map(|r| r.expect("row"))
        .collect();
    assert_eq!(
        codes,
        vec![
            "на_складе",
            "в_работе",
            "на_ремонте",
            "в_ремонте",
            "на_списание",
            "списано"
        ]
    );
}

#[test]
fn v047_renamed_status_keeps_id_code_and_device_link() {
    let (conn, _guard) = upgraded();
    let (id, name): (i64, String) = conn
        .query_row(
            "SELECT id, name FROM device_statuses WHERE code = 'на_ремонте'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .expect("status by code");
    assert_eq!(name, "Ожидает ремонта");
    assert_eq!(id, 3, "id of the renamed status must not change");
    let status_id: i64 = conn
        .query_row("SELECT status_id FROM devices WHERE id = 1", [], |r| {
            r.get(0)
        })
        .expect("device status");
    assert_eq!(status_id, 3, "existing devices.status_id must be untouched");
}

#[test]
fn v047_new_statuses_have_expected_codes_and_distinct_ids() {
    let (conn, _guard) = upgraded();
    let repair: (i64, String) = conn
        .query_row(
            "SELECT id, name FROM device_statuses WHERE code = 'в_ремонте'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .expect("в_ремонте");
    let writeoff: (i64, String) = conn
        .query_row(
            "SELECT id, name FROM device_statuses WHERE code = 'на_списание'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .expect("на_списание");
    assert_eq!(repair.1, "В ремонте");
    assert_eq!(writeoff.1, "На списание");
    assert_ne!(repair.0, writeoff.0);
    assert!(
        repair.0 > 4 && writeoff.0 > 4,
        "new ids go after the seeded 1..4"
    );
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM device_statuses"), 6);
}

#[test]
fn v047_preserves_rows_and_is_idempotent() {
    let (mut conn, _guard) = conn_at_v046();
    seed(&conn);
    let items_before = count(&conn, "SELECT COUNT(*) FROM act_items");
    let devices_before = count(&conn, "SELECT COUNT(*) FROM devices");
    assert!(items_before > 0 && devices_before > 0);

    let report = migrations::run(&mut conn).expect("upgrade V046 -> latest");
    assert_eq!(report.schema_version, migrations::max_known_version());
    assert!(migrations::max_known_version() >= 47);

    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM act_items"),
        items_before,
        "act_items must survive"
    );
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM devices"),
        devices_before,
        "devices must survive"
    );
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM pragma_foreign_key_check"),
        0,
        "foreign_key_check must be empty"
    );

    let again = migrations::run(&mut conn).expect("second run");
    assert_eq!(again.schema_version, report.schema_version);
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM device_statuses"), 6);
}
