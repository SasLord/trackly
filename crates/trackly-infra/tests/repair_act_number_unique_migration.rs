//! Integration test: `V052__repair_act_number_unique.sql` (долг W-1 фазы 41.4).
//!
//! Номер акта ремонта уникален на уровне БД (регистр — с кириллицей — и
//! пробелы по краям не различаются, как в `normalize_number_key`). На старой
//! БД с уже возникшими дублями миграция не падает: самый ранний визит группы
//! номер сохраняет, остальные переименовываются в `ДУБЛЬ-NNNNNN (<исходный>)`
//! с записью в audit_log. Прогон через настоящий раннер на настоящей БД V051.
//! Только вымышленные данные.

use rusqlite::{params, Connection};
use tempfile::TempDir;

use trackly_infra::db::migrations;
use trackly_infra::db::pragmas::apply_writer_pragmas;

const NOW: i64 = 1_700_000_000;

/// Настоящая БД V051: пользователь, два места, устройство и `n` закрытых
/// ремонтных заявок (по одной на будущий визит).
fn v051_with_requests(n: i64) -> (Connection, TempDir) {
    let dir = TempDir::new().expect("tempdir");
    let mut conn = Connection::open(dir.path().join("repair-act-unique.db")).expect("open");
    apply_writer_pragmas(&conn).expect("pragmas");
    let report = migrations::run_up_to(&mut conn, 51).expect("up to V051");
    assert_eq!(report.schema_version, 51, "fixture must be a real V051 DB");
    conn.execute_batch(&format!(
        "INSERT INTO users (id, login, full_name, role, created_at_utc, updated_at_utc)
           VALUES (1, 'petrov', 'Петров П.П.', 'admin', {NOW}, {NOW});
         INSERT INTO places (id, parent_id, kind, name, is_storage, created_at_utc, updated_at_utc)
           VALUES (1, NULL, 'building', 'Склад А', 1, {NOW}, {NOW}),
                  (2, NULL, 'building', 'Сервис Б', 0, {NOW}, {NOW});
         INSERT INTO devices (id, type_id, name, inventory_number, status_id, place_id,
                              created_at_utc, updated_at_utc)
           VALUES (1, 1, 'Принтер #1', 'ИНВ-000001', 2, 1, {NOW}, {NOW});"
    ))
    .expect("seed");
    for _ in 0..n {
        conn.execute(
            "INSERT INTO requests (request_type, status, requested_by_user_id, printer_device_id,
                                   created_at_utc, updated_at_utc)
             VALUES ('repair', 'completed', 1, 1, ?1, ?1)",
            params![NOW],
        )
        .expect("request");
    }
    (conn, dir)
}

fn insert_visit(conn: &Connection, request_id: i64, act_number: &str) -> rusqlite::Result<usize> {
    conn.execute(
        "INSERT INTO repair_visits (request_id, device_id, seq, service_place_id, sent_at_utc,
            returned_at_utc, giver_name, receiver_name, fault_description, act_number,
            created_at_utc, updated_at_utc)
         VALUES (?1, 1, 1, 2, ?2, ?2, 'Иванов И.И.', 'Петров П.П.', 'не печатает', ?3, ?2, ?2)",
        params![request_id, NOW, act_number],
    )
}

fn act_number(conn: &Connection, request_id: i64) -> String {
    conn.query_row(
        "SELECT act_number FROM repair_visits WHERE request_id = ?1",
        [request_id],
        |r| r.get(0),
    )
    .unwrap_or_else(|e| panic!("visit of request {request_id}: {e}"))
}

fn dedup_audit(conn: &Connection) -> Vec<(i64, Option<i64>, String)> {
    conn.prepare(
        "SELECT entity_id, user_id, payload_json FROM audit_log
          WHERE entity_type = 'repair_visit' AND action = 'custom:repair_act_number_dedup'
          ORDER BY entity_id",
    )
    .unwrap()
    .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
    .unwrap()
    .map(|r| r.unwrap())
    .collect()
}

/// Старая БД с дублями: миграция проходит, ранний визит сохраняет номер,
/// остальные переименованы одной сквозной нумерацией, каждое переименование
/// записано в audit_log; после миграции всё уникально.
#[test]
fn v052_renames_existing_duplicates_instead_of_failing() {
    let (mut conn, _g) = v051_with_requests(6);
    // Группа 1: тот же номер, отличаются кириллическим регистром и пробелами.
    insert_visit(&conn, 1, "РЕМ-000001").unwrap();
    insert_visit(&conn, 2, " рем-000001 ").unwrap();
    insert_visit(&conn, 3, "Рем-000001").unwrap();
    // Уникальный — не трогается.
    insert_visit(&conn, 4, "РЕМ-000002").unwrap();
    // Группа 2.
    insert_visit(&conn, 5, "РЕМ-000003").unwrap();
    insert_visit(&conn, 6, "РЕМ-000003").unwrap();

    let report = migrations::run(&mut conn).expect("V052 must not fail on duplicates");
    assert_eq!(report.schema_version, 52);
    assert_eq!(
        conn.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        52
    );

    assert_eq!(act_number(&conn, 1), "РЕМ-000001", "ранний визит не тронут");
    assert_eq!(act_number(&conn, 2), "ДУБЛЬ-000001 (рем-000001)");
    assert_eq!(act_number(&conn, 3), "ДУБЛЬ-000002 (Рем-000001)");
    assert_eq!(act_number(&conn, 4), "РЕМ-000002");
    assert_eq!(act_number(&conn, 5), "РЕМ-000003");
    assert_eq!(act_number(&conn, 6), "ДУБЛЬ-000003 (РЕМ-000003)");

    let audit = dedup_audit(&conn);
    assert_eq!(audit.len(), 3, "по строке аудита на переименование");
    for (_, user, _) in &audit {
        assert_eq!(*user, None, "системное действие");
    }
    let payload: serde_json::Value = serde_json::from_str(&audit[0].2).unwrap();
    assert_eq!(payload["old_number"], "рем-000001");
    assert_eq!(payload["new_number"], "ДУБЛЬ-000001 (рем-000001)");
    assert_eq!(payload["request_id"], 2);
}

/// Чистая БД: ничего не переименовано, аудита нет, индекс уникальный.
#[test]
fn v052_clean_db_untouched_and_index_unique() {
    let (mut conn, _g) = v051_with_requests(2);
    insert_visit(&conn, 1, "РЕМ-000001").unwrap();
    insert_visit(&conn, 2, "РЕМ-000002").unwrap();

    migrations::run(&mut conn).expect("upgrade");

    assert_eq!(act_number(&conn, 1), "РЕМ-000001");
    assert_eq!(act_number(&conn, 2), "РЕМ-000002");
    assert!(dedup_audit(&conn).is_empty());
    let unique: i64 = conn
        .query_row(
            "SELECT \"unique\" FROM pragma_index_list('repair_visits')
              WHERE name = 'idx_repair_visits_act_number'",
            [],
            |r| r.get(0),
        )
        .expect("index exists");
    assert_eq!(unique, 1, "idx_repair_visits_act_number is UNIQUE");
}

/// Вставка второго визита с тем же номером отвергается самой БД — в том числе
/// при другом (кириллическом) регистре и пробелах по краям. Другой номер
/// проходит, так что отказ вызван именно номером.
#[test]
fn v052_db_rejects_second_visit_with_same_act_number() {
    let (mut conn, _g) = v051_with_requests(5);
    migrations::run(&mut conn).expect("upgrade");

    insert_visit(&conn, 1, "РЕМ-000010").expect("первый визит");
    for (rid, dup) in [(2, "РЕМ-000010"), (3, "  рем-000010"), (4, "Рем-000010 ")] {
        let err = insert_visit(&conn, rid, dup).expect_err("дубль номера отвергнут");
        assert!(
            err.to_string().contains("UNIQUE"),
            "отказ по уникальности, а не по другой причине: {err}"
        );
    }
    insert_visit(&conn, 5, "РЕМ-000011").expect("другой номер проходит");
}
