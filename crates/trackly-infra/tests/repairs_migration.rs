//! Integration test: `V051__repairs.sql` (Phase 41.4 Plan 01), прогон через
//! реальный раннер на настоящей БД V050: `run_up_to(50)` -> наполнение ->
//! `run`. Только вымышленные данные.

use rusqlite::{params, Connection};
use tempfile::TempDir;

use trackly_infra::db::migrations;
use trackly_infra::db::pragmas::apply_writer_pragmas;

const NOW: i64 = 1_700_000_000;

fn count(conn: &Connection, sql: &str) -> i64 {
    conn.query_row(sql, [], |r| r.get(0)).expect("count")
}

/// БД V050 с наполнением; возвращает снимок requests до миграции.
fn seeded_v050() -> (Connection, TempDir) {
    let dir = TempDir::new().expect("tempdir");
    let mut conn = Connection::open(dir.path().join("repairs-mig.db")).expect("open");
    apply_writer_pragmas(&conn).expect("pragmas");
    let report = migrations::run_up_to(&mut conn, 50).expect("up to V050");
    assert_eq!(report.schema_version, 50, "fixture must be a real V050 DB");
    conn.execute_batch(&format!(
        "INSERT INTO users (id, login, full_name, role, created_at_utc, updated_at_utc)
           VALUES (1, 'petrov', 'Петров П.П.', 'admin', {NOW}, {NOW}),
                  (2, 'ivanov', 'Иванов И.И.', 'employee', {NOW}, {NOW});
         INSERT INTO places (id, parent_id, kind, name, is_storage, created_at_utc, updated_at_utc)
           VALUES (1, NULL, 'building', 'Склад А', 1, {NOW}, {NOW}),
                  (2, NULL, 'building', 'Сервис Б', 0, {NOW}, {NOW});
         INSERT INTO devices (id, type_id, name, inventory_number, status_id, place_id,
                              created_at_utc, updated_at_utc)
           VALUES (1, 1, 'Принтер #1', 'ИНВ-000001', 2, 1, {NOW}, {NOW}),
                  (2, 1, 'Принтер #2', 'ИНВ-000002', 2, 1, {NOW}, {NOW});
         INSERT INTO cartridge_models (id, brand, model, created_at_utc, updated_at_utc)
           VALUES (1, 'Pantum', 'TL-5120X', {NOW}, {NOW});
         INSERT INTO cartridges (id, code, model_id, status_id, created_at_utc, updated_at_utc, version)
           VALUES (1, 'C-0001', 1, 1, {NOW}, {NOW}, 1);
         INSERT INTO requests (id, request_type, status, requested_by_user_id, assigned_to_user_id,
                               printer_device_id, cartridge_model_id, description, resolution_notes,
                               created_at_utc, updated_at_utc, version, category_id,
                               completed_cartridge_id, ad_subtype)
           VALUES (1, 'cartridge_replace', 'open', 2, NULL, 1, 1, 'заменить', NULL, {NOW}, {NOW}, 1, NULL, NULL, NULL),
                  (2, 'cartridge_replace', 'completed', 2, 1, 1, 1, 'готово', 'ок', {NOW}, {NOW}, 3, NULL, 1, NULL),
                  (3, 'free_form', 'in_progress', 2, 1, NULL, NULL, 'вопрос', NULL, {NOW}, {NOW}, 2, 1, NULL, NULL),
                  (4, 'ad_register', 'rejected', 2, 1, NULL, NULL, 'учётка', 'нет', {NOW}, {NOW}, 2, NULL, NULL, 'register'),
                  (5, 'free_form', 'cancelled', 2, NULL, NULL, NULL, 'отмена', NULL, {NOW}, {NOW}, 2, 2, NULL, NULL);
         INSERT INTO attachments (id, parent_kind, parent_id, original_name, mime, size_bytes,
                                  content_hash, uploaded_by_user_id, created_at_utc)
           VALUES (1, 'request', 1, 'скан.pdf', 'application/pdf', 10, 'abc', 1, {NOW});
         UPDATE number_templates SET mask = mask;"
    ))
    .expect("seed");
    // Пользовательский шаблон поверх засева; контексты указывают на разные id.
    conn.execute(
        "INSERT INTO number_templates (type, mask, created_at_utc, updated_at_utc, version)
         VALUES ('device_inventory', 'ИНВ-[XXXXXX]', ?1, ?1, 1)",
        params![NOW],
    )
    .unwrap();
    conn.execute(
        "UPDATE number_template_contexts SET template_id =
           (SELECT id FROM number_templates WHERE type='device_inventory')
         WHERE context = 'device_create'",
        [],
    )
    .unwrap();
    (conn, dir)
}

fn contexts(conn: &Connection) -> Vec<(String, Option<i64>)> {
    conn.prepare(
        "SELECT context, template_id FROM number_template_contexts
         WHERE context != 'repair_act_create' ORDER BY context",
    )
    .unwrap()
    .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
    .unwrap()
    .map(|r| r.unwrap())
    .collect()
}

fn requests_dump(conn: &Connection) -> Vec<String> {
    conn.prepare(
        "SELECT id || '|' || request_type || '|' || status || '|' || requested_by_user_id || '|' ||
                IFNULL(assigned_to_user_id,'N') || '|' || IFNULL(printer_device_id,'N') || '|' ||
                IFNULL(cartridge_model_id,'N') || '|' || IFNULL(description,'N') || '|' ||
                IFNULL(resolution_notes,'N') || '|' || created_at_utc || '|' || updated_at_utc || '|' ||
                IFNULL(deleted_at_utc,'N') || '|' || version || '|' || IFNULL(category_id,'N') || '|' ||
                IFNULL(completed_cartridge_id,'N') || '|' || IFNULL(ad_subtype,'N')
         FROM requests ORDER BY id",
    )
    .unwrap()
    .query_map([], |r| r.get(0))
    .unwrap()
    .map(|r| r.unwrap())
    .collect()
}

#[test]
fn v051_upgrade_preserves_data_and_adds_schema() {
    let (mut conn, _g) = seeded_v050();
    let req_before = requests_dump(&conn);
    let cols_before = count(&conn, "SELECT COUNT(*) FROM pragma_table_info('requests')");
    let ctx_before = contexts(&conn);
    let tpl_before: Vec<(i64, String, String)> = conn
        .prepare("SELECT id, type, mask FROM number_templates ORDER BY id")
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    assert_eq!(ctx_before.len(), 5);

    migrations::run(&mut conn).expect("upgrade to V051");

    // (1)
    assert_eq!(count(&conn, "PRAGMA user_version"), 51);
    // (2)
    assert_eq!(requests_dump(&conn), req_before);
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM pragma_table_info('requests')"),
        cols_before + 2
    );
    conn.execute(
        "INSERT INTO requests (request_type, status, requested_by_user_id, created_at_utc, updated_at_utc)
         VALUES ('free_form', 'open', 1, ?1, ?1)",
        params![NOW],
    )
    .unwrap();
    assert_eq!(
        count(&conn, "SELECT MAX(id) FROM requests"),
        6,
        "AUTOINCREMENT не откатился"
    );
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM attachments"), 1);
    // (3)
    assert_eq!(
        contexts(&conn),
        ctx_before,
        "template_id прежних контекстов сохранены"
    );
    for (id, ty, mask) in &tpl_before {
        let n = count(
            &conn,
            &format!("SELECT COUNT(*) FROM number_templates WHERE id={id} AND type='{ty}' AND mask='{mask}'"),
        );
        assert_eq!(n, 1, "шаблон {id} сохранён");
    }
    let dup = conn.execute(
        "INSERT INTO number_templates (type, mask, created_at_utc, updated_at_utc, version)
         VALUES ('act_number', '[X]', 1, 1, 1)",
        [],
    );
    assert!(dup.is_err(), "UNIQUE(type, mask) цел");
    // (4)
    let (ty, mask): (String, String) = conn
        .query_row(
            "SELECT nt.type, nt.mask FROM number_template_contexts c
             JOIN number_templates nt ON nt.id = c.template_id
             WHERE c.context = 'repair_act_create'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .expect("repair_act_create");
    assert_eq!((ty.as_str(), mask.as_str()), ("repair_act", "РЕМ-[XXXXXX]"));
    // (5)
    for idx in [
        "idx_requests_status",
        "idx_requests_assigned",
        "uq_requests_open_repair_device",
    ] {
        assert_eq!(
            count(
                &conn,
                &format!("SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='{idx}'")
            ),
            1,
            "{idx}"
        );
    }
    // (6)
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM pragma_foreign_key_check"),
        0
    );
}

#[test]
fn v051_repair_request_constraints() {
    let (mut conn, _g) = seeded_v050();
    migrations::run(&mut conn).expect("upgrade");
    let ins = |conn: &Connection, ty: &str, st: &str, dev: Option<i64>| {
        conn.execute(
            "INSERT INTO requests (request_type, status, requested_by_user_id, printer_device_id,
                                   created_at_utc, updated_at_utc)
             VALUES (?1, ?2, 1, ?3, ?4, ?4)",
            params![ty, st, dev, NOW],
        )
    };
    // (7)
    ins(&conn, "repair", "in_repair", Some(2)).expect("ремонтная заявка");
    assert!(
        ins(&conn, "repair", "open", Some(2)).is_err(),
        "вторая открытая на то же устройство"
    );
    ins(&conn, "repair", "completed", Some(2)).expect("закрытая не затрагивается");
    ins(&conn, "repair", "open", None).expect("без устройства");
    ins(&conn, "repair", "open", None).expect("без устройства (ещё раз)");
    assert!(ins(&conn, "bogus", "open", None).is_err(), "CHECK типа");

    // (8)
    let rid = conn.last_insert_rowid();
    let visit = |conn: &Connection,
                 rid: i64,
                 seq: i64,
                 returned: Option<i64>,
                 cost: Option<i64>| {
        conn.execute(
            "INSERT INTO repair_visits (request_id, device_id, seq, service_place_id, sent_at_utc,
                returned_at_utc, giver_name, receiver_name, fault_description, cost_kopecks,
                act_number, created_at_utc, updated_at_utc)
             VALUES (?1, 2, ?2, 2, ?3, ?4, 'Иванов И.И.', 'Петров П.П.', 'не печатает', ?5, 'РЕМ-000001', ?3, ?3)",
            params![rid, seq, NOW, returned, cost],
        )
    };
    visit(&conn, rid, 1, None, None).expect("первый заход");
    assert!(
        visit(&conn, rid, 2, None, None).is_err(),
        "второй открытый заход"
    );
    assert!(visit(&conn, rid, 1, Some(NOW), None).is_err(), "дубль seq");
    assert!(
        visit(&conn, rid, 3, Some(NOW), Some(-1)).is_err(),
        "отрицательная стоимость"
    );
    visit(&conn, rid, 3, Some(NOW), Some(150_000)).expect("закрытый заход");

    // (10)
    conn.execute(
        "INSERT INTO request_participants (request_id, user_id, joined_at_utc) VALUES (?1, 2, ?2)",
        params![rid, NOW],
    )
    .expect("участник");
    assert!(conn
        .execute(
            "INSERT INTO request_participants (request_id, user_id, joined_at_utc) VALUES (?1, 2, ?2)",
            params![rid, NOW],
        )
        .is_err());
    conn.execute(
        "INSERT INTO repair_visit_scans (visit_id, attachment_id, linked_at_utc)
         VALUES ((SELECT id FROM repair_visits LIMIT 1), 1, ?1)",
        params![NOW],
    )
    .expect("скан");
}

#[test]
fn v051_hides_repair_category() {
    let (mut conn, _g) = seeded_v050();
    migrations::run(&mut conn).expect("upgrade");
    // (9)
    assert_eq!(
        count(
            &conn,
            "SELECT is_active FROM request_categories WHERE name = 'Ремонт техники'"
        ),
        0
    );
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM request_categories WHERE name != 'Ремонт техники' AND is_active = 0"),
        0
    );
    assert!(
        count(
            &conn,
            "SELECT COUNT(*) FROM request_categories WHERE name != 'Ремонт техники'"
        ) >= 1
    );
}
