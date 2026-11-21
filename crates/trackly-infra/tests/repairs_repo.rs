//! Integration tests: `SqliteRepairRepository` (Phase 41.4 Plan 02).
//! Свежая БД после полного набора миграций. Только вымышленные данные.

use rusqlite::{params, Connection};
use trackly_core::domain::repairs::RepairVisitNew;
use trackly_core::error::AppError;
use trackly_infra::repos::SqliteRepairRepository;
use trackly_infra::test_support::test_db;

const NOW: i64 = 1_700_000_000;

fn repo() -> SqliteRepairRepository {
    SqliteRepairRepository
}

fn add_user(conn: &Connection, login: &str, name: &str) -> i64 {
    conn.execute(
        "INSERT INTO users (login, full_name, role, created_at_utc, updated_at_utc) \
         VALUES (?1, ?2, 'employee', ?3, ?3)",
        params![login, name, NOW],
    )
    .unwrap();
    conn.last_insert_rowid()
}

fn add_place(conn: &Connection, name: &str) -> i64 {
    conn.execute(
        "INSERT INTO places (parent_id, kind, name, version, created_at_utc, updated_at_utc) \
         VALUES (NULL, 'room', ?1, 1, ?2, ?2)",
        params![name, NOW],
    )
    .unwrap();
    conn.last_insert_rowid()
}

fn add_device(conn: &Connection, type_name: &str, name: &str, inv: &str) -> i64 {
    conn.execute(
        "INSERT INTO devices (type_id, name, inventory_number, status_id, version, \
         created_at_utc, updated_at_utc) \
         VALUES ((SELECT id FROM device_types WHERE name = ?1), ?2, ?3, \
                 (SELECT id FROM device_statuses WHERE code = 'на_складе'), 1, ?4, ?4)",
        params![type_name, name, inv, NOW],
    )
    .unwrap();
    conn.last_insert_rowid()
}

fn add_printer_usb(conn: &Connection, device: i64, host: Option<i64>) {
    conn.execute(
        "INSERT INTO printers (device_id, usb_host_device_id, created_at_utc, updated_at_utc) \
         VALUES (?1, ?2, ?3, ?3)",
        params![device, host, NOW],
    )
    .unwrap();
}

fn add_request(
    conn: &Connection,
    user: i64,
    device: Option<i64>,
    status: &str,
    deleted: Option<i64>,
) -> Result<i64, rusqlite::Error> {
    conn.execute(
        "INSERT INTO requests (request_type, status, requested_by_user_id, printer_device_id, \
         created_at_utc, updated_at_utc, deleted_at_utc) VALUES ('repair', ?1, ?2, ?3, ?4, ?4, ?5)",
        params![status, user, device, NOW, deleted],
    )?;
    Ok(conn.last_insert_rowid())
}

fn visit_new(request_id: i64, device_id: i64, place: i64) -> RepairVisitNew {
    // Номер акта уникален на уровне БД (V052) — каждому визиту свой.
    static NEXT_ACT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    let act = NEXT_ACT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    RepairVisitNew {
        request_id,
        device_id,
        service_place_id: place,
        from_place_id: None,
        sent_at_utc: NOW,
        giver_name: "Иванов И.И.".into(),
        receiver_name: "Петров П.П.".into(),
        fault_description: "Не включается".into(),
        act_number: format!("РЕМ-{act:06}"),
        created_by_user_id: None,
    }
}

#[test]
fn open_predicate_by_status_and_deletion() {
    let (conn, _g) = test_db();
    let u = add_user(&conn, "u1", "Сидоров С.С.");
    for (i, status) in ["open", "in_progress", "in_repair"].iter().enumerate() {
        let d = add_device(
            &conn,
            "Принтер",
            &format!("Принтер {i}"),
            &format!("INV-A{i}"),
        );
        let id = add_request(&conn, u, Some(d), status, None).unwrap();
        assert_eq!(repo().open_repair_request_id(&conn, d).unwrap(), Some(id));
    }
    for (i, status) in ["completed", "rejected", "cancelled"].iter().enumerate() {
        let d = add_device(
            &conn,
            "Принтер",
            &format!("Закрытый {i}"),
            &format!("INV-B{i}"),
        );
        add_request(&conn, u, Some(d), status, None).unwrap();
        assert_eq!(repo().open_repair_request_id(&conn, d).unwrap(), None);
    }
    let d = add_device(&conn, "Принтер", "Удалённый", "INV-C");
    add_request(&conn, u, Some(d), "open", Some(NOW)).unwrap();
    assert_eq!(repo().open_repair_request_id(&conn, d).unwrap(), None);
}

#[test]
fn open_requests_for_devices_diverging_fixture() {
    let (conn, _g) = test_db();
    let u = add_user(&conn, "u1", "Сидоров С.С.");
    let busy = add_device(&conn, "Принтер", "Занятый", "INV-1");
    let free1 = add_device(&conn, "Принтер", "Свободный 1", "INV-2");
    let free2 = add_device(&conn, "Принтер", "Закрытый", "INV-3");
    let rid = add_request(&conn, u, Some(busy), "in_repair", None).unwrap();
    add_request(&conn, u, Some(free2), "completed", None).unwrap();
    let got = repo()
        .open_repair_requests_for_devices(&conn, &[busy, free1, free2])
        .unwrap();
    assert_eq!(got, vec![(busy, rid)]);
    assert!(repo()
        .open_repair_requests_for_devices(&conn, &[])
        .unwrap()
        .is_empty());
}

#[test]
fn set_device_conflict_on_second_open_repair() {
    let (mut conn, _g) = test_db();
    let u = add_user(&conn, "u1", "Сидоров С.С.");
    let d = add_device(&conn, "Принтер", "Принтер", "INV-1");
    add_request(&conn, u, Some(d), "open", None).unwrap();
    let r2 = add_request(&conn, u, None, "open", None).unwrap();
    let tx = conn.transaction().unwrap();
    let err = repo()
        .set_request_device_in_tx(&tx, r2, d, NOW)
        .unwrap_err();
    assert!(matches!(err, AppError::Conflict { .. }), "{err:?}");
}

#[test]
fn visit_seq_and_single_open_visit() {
    let (mut conn, _g) = test_db();
    let u = add_user(&conn, "u1", "Сидоров С.С.");
    let d = add_device(&conn, "Принтер", "Принтер", "INV-1");
    let p = add_place(&conn, "Сервис");
    let rid = add_request(&conn, u, Some(d), "open", None).unwrap();
    let tx = conn.transaction().unwrap();
    repo()
        .insert_visit_in_tx(&tx, &visit_new(rid, d, p), NOW)
        .unwrap();
    // второй открытый заход отклоняется
    let err = repo()
        .insert_visit_in_tx(&tx, &visit_new(rid, d, p), NOW)
        .unwrap_err();
    assert!(matches!(err, AppError::Conflict { .. }), "{err:?}");
    drop(tx); // откат: ни один заход не сохранён
    assert_eq!(repo().count_visits(&conn, rid).unwrap(), 0);

    let tx = conn.transaction().unwrap();
    let a = repo()
        .insert_visit_in_tx(&tx, &visit_new(rid, d, p), NOW)
        .unwrap();
    repo()
        .close_visit_in_tx(
            &tx,
            a,
            1,
            NOW + 1,
            Some(p),
            Some("Петров П.П."),
            Some("Замена"),
            Some(150_050),
            NOW + 1,
        )
        .unwrap();
    let b = repo()
        .insert_visit_in_tx(&tx, &visit_new(rid, d, p), NOW + 2)
        .unwrap();
    repo()
        .close_visit_in_tx(&tx, b, 1, NOW + 3, None, None, None, None, NOW + 3)
        .unwrap();
    let c = repo()
        .insert_visit_in_tx(&tx, &visit_new(rid, d, p), NOW + 4)
        .unwrap();
    tx.commit().unwrap();

    let visits = repo().list_visits_for_request(&conn, rid).unwrap();
    assert_eq!(
        visits.iter().map(|v| v.seq).collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
    assert_eq!(visits[0].cost_kopecks, Some(150_050));
    assert_eq!(repo().count_visits(&conn, rid).unwrap(), 3);
    assert_eq!(
        repo()
            .open_visit_for_request(&conn, rid)
            .unwrap()
            .unwrap()
            .id,
        c
    );
}

#[test]
fn close_visit_stale_version_is_lock_mismatch() {
    let (mut conn, _g) = test_db();
    let u = add_user(&conn, "u1", "Сидоров С.С.");
    let d = add_device(&conn, "Принтер", "Принтер", "INV-1");
    let p = add_place(&conn, "Сервис");
    let rid = add_request(&conn, u, Some(d), "open", None).unwrap();
    let tx = conn.transaction().unwrap();
    let v = repo()
        .insert_visit_in_tx(&tx, &visit_new(rid, d, p), NOW)
        .unwrap();
    let err = repo()
        .close_visit_in_tx(&tx, v, 7, NOW, None, None, None, None, NOW)
        .unwrap_err();
    assert!(
        matches!(
            err,
            AppError::OptimisticLockMismatch {
                expected: 7,
                actual: 1,
                ..
            }
        ),
        "{err:?}"
    );
    // уже закрытый заход с верной версией — Conflict
    repo()
        .close_visit_in_tx(&tx, v, 1, NOW, None, None, None, None, NOW)
        .unwrap();
    let err = repo()
        .close_visit_in_tx(&tx, v, 2, NOW, None, None, None, None, NOW)
        .unwrap_err();
    assert!(matches!(err, AppError::Conflict { .. }), "{err:?}");
}

#[test]
fn request_status_cas() {
    let (mut conn, _g) = test_db();
    let u = add_user(&conn, "u1", "Сидоров С.С.");
    let d = add_device(&conn, "Принтер", "Принтер", "INV-1");
    let rid = add_request(&conn, u, Some(d), "open", None).unwrap();
    let tx = conn.transaction().unwrap();
    repo()
        .set_request_status_in_tx(&tx, rid, 1, "in_repair", Some(u), NOW)
        .unwrap();
    let err = repo()
        .set_request_status_in_tx(&tx, rid, 1, "completed", None, NOW)
        .unwrap_err();
    assert!(
        matches!(err, AppError::OptimisticLockMismatch { actual: 2, .. }),
        "{err:?}"
    );
    repo()
        .set_repair_fields_in_tx(&tx, rid, None, true, NOW)
        .unwrap();
    assert_eq!(repo().get_repair_fields(&tx, rid).unwrap(), (None, true));
}

#[test]
fn participants_idempotent_and_scans_skip_deleted() {
    let (mut conn, _g) = test_db();
    let u = add_user(&conn, "u1", "Сидоров С.С.");
    let u2 = add_user(&conn, "u2", "Кузнецов К.К.");
    let d = add_device(&conn, "Принтер", "Принтер", "INV-1");
    let p = add_place(&conn, "Сервис");
    let rid = add_request(&conn, u, Some(d), "open", None).unwrap();
    let tx = conn.transaction().unwrap();
    repo().add_participant_in_tx(&tx, rid, u2, NOW).unwrap();
    repo().add_participant_in_tx(&tx, rid, u2, NOW + 1).unwrap();
    assert_eq!(repo().participant_user_ids(&tx, rid).unwrap(), vec![u2]);

    let v = repo()
        .insert_visit_in_tx(&tx, &visit_new(rid, d, p), NOW)
        .unwrap();
    for name in ["a.pdf", "b.pdf"] {
        tx.execute(
            "INSERT INTO attachments (parent_kind, parent_id, original_name, mime, size_bytes, \
             content_hash, created_at_utc) VALUES ('request', ?1, ?2, 'application/pdf', 1, ?2, ?3)",
            params![rid, name, NOW],
        )
        .unwrap();
        let aid = tx.last_insert_rowid();
        repo().link_scan_in_tx(&tx, v, aid, NOW).unwrap();
        repo().link_scan_in_tx(&tx, v, aid, NOW).unwrap();
    }
    assert_eq!(repo().scan_count_for_visit(&tx, v).unwrap(), 2);
    tx.execute(
        "UPDATE attachments SET deleted_at_utc = ?1 WHERE original_name = 'b.pdf'",
        params![NOW],
    )
    .unwrap();
    assert_eq!(repo().scan_count_for_visit(&tx, v).unwrap(), 1);
    assert_eq!(
        repo().scan_counts_for_request(&tx, rid).unwrap(),
        vec![(v, 1)]
    );
}

#[test]
fn settings_upsert_twice() {
    let (conn, _g) = test_db();
    assert_eq!(repo().read_setting(&conn, "repair.test_key").unwrap(), None);
    repo()
        .upsert_setting(&conn, "repair.test_key", "a", NOW)
        .unwrap();
    repo()
        .upsert_setting(&conn, "repair.test_key", "b", NOW + 1)
        .unwrap();
    assert_eq!(
        repo()
            .read_setting(&conn, "repair.test_key")
            .unwrap()
            .as_deref(),
        Some("b")
    );
    assert!(repo().device_status_id_by_code(&conn, "в_ремонте").unwrap() > 0);
}

#[test]
fn employee_device_ids_scope_and_negative() {
    let (conn, _g) = test_db();
    let a = add_user(&conn, "ua", "Алексеев А.А.");
    let b = add_user(&conn, "ub", "Борисов Б.Б.");
    let pc_a = add_device(&conn, "Устройство", "АРМ А", "INV-A");
    let pc_b = add_device(&conn, "Устройство", "АРМ Б", "INV-B");
    let prn_usb = add_device(&conn, "Принтер", "USB-принтер А", "INV-PA");
    add_printer_usb(&conn, prn_usb, Some(pc_a));
    let prn_other = add_device(&conn, "Принтер", "Принтер Б", "INV-PB");
    add_printer_usb(&conn, prn_other, Some(pc_b));

    conn.execute(
        "INSERT INTO group_types (code, name, behavior, created_at_utc, updated_at_utc) \
         VALUES ('arm_t', 'АРМ-тест', 'workstation', ?1, ?1)",
        params![NOW],
    )
    .unwrap();
    let t = conn.last_insert_rowid();
    conn.execute(
        "INSERT INTO group_type_properties (type_id, name, data_type, created_at_utc, updated_at_utc) \
         VALUES (?1, 'Пользователи', 'users', ?2, ?2)",
        params![t, NOW],
    )
    .unwrap();
    let prop = conn.last_insert_rowid();
    let mut gids = Vec::new();
    for (seq, user, pc) in [(1, a, pc_a), (2, b, pc_b)] {
        conn.execute(
            "INSERT INTO groups (type_id, name, seq, version, created_at_utc, updated_at_utc) \
             VALUES (?1, ?2, ?3, 1, ?4, ?4)",
            params![t, format!("Комплект {seq}"), seq, NOW],
        )
        .unwrap();
        let g = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO group_devices (device_id, group_id, added_at_utc) VALUES (?1, ?2, ?3)",
            params![pc, g, NOW],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO group_property_values (group_id, property_id, value_ref, updated_at_utc) \
             VALUES (?1, ?2, ?3, ?4)",
            params![g, prop, user, NOW],
        )
        .unwrap();
        gids.push(g);
    }

    let got = repo().employee_device_ids(&conn, a).unwrap();
    assert!(got.contains(&pc_a));
    assert!(got.contains(&prn_usb));
    assert!(
        !got.contains(&pc_b),
        "чужое устройство не должно попасть в область"
    );
    assert!(
        !got.contains(&prn_other),
        "чужой принтер не должен попасть в область"
    );
    assert!(repo()
        .employee_device_ids(&conn, 999_999)
        .unwrap()
        .is_empty());
}

#[test]
fn history_requests_by_visit_device_without_duplicates_or_deleted() {
    let (mut conn, _g) = test_db();
    let u = add_user(&conn, "ivanov", "Иванов И.И.");
    let place = add_place(&conn, "Сервис");
    let d1 = add_device(&conn, "Принтер", "Принтер А", "INV-1");
    let d2 = add_device(&conn, "Принтер", "Принтер Б", "INV-2");
    // req_a ссылается на d2 в requests, но заходы у неё по d1: берём по заходам.
    let req_a = add_request(&conn, u, Some(d2), "completed", None).unwrap();
    let req_b = add_request(&conn, u, Some(d1), "in_repair", None).unwrap();
    let req_deleted = add_request(&conn, u, Some(d1), "completed", Some(NOW)).unwrap();
    let _req_empty = add_request(&conn, u, Some(d2), "open", None).unwrap();
    conn.execute(
        "UPDATE requests SET created_at_utc = ?2 WHERE id = ?1",
        params![req_b, NOW + 100],
    )
    .unwrap();
    let tx = conn.transaction().unwrap();
    for req in [req_a, req_a, req_b, req_deleted] {
        let v = repo()
            .insert_visit_in_tx(&tx, &visit_new(req, d1, place), NOW)
            .unwrap_or_else(|_| {
                panic!("insert visit for {req}");
            });
        // закрываем, чтобы следующий заход той же заявки не упёрся в «один открытый»
        if req == req_a {
            repo()
                .close_visit_in_tx(&tx, v, 1, NOW + 1, Some(place), None, None, None, NOW + 1)
                .unwrap();
        }
    }
    tx.commit().unwrap();
    let rows = repo()
        .list_repair_requests_with_visits_for_device(&conn, d1)
        .unwrap();
    let ids: Vec<i64> = rows.iter().map(|x| x.id).collect();
    assert_eq!(
        ids,
        vec![req_b, req_a],
        "новые первыми, без дублей и удалённых"
    );
    assert_eq!(rows[0].requester_name.as_deref(), Some("Иванов И.И."));
    assert!(repo()
        .list_repair_requests_with_visits_for_device(&conn, d2)
        .unwrap()
        .is_empty());
}

// ---------------------------------------------------------- employee_device_origins

fn add_group(conn: &Connection, type_id: i64, name: &str, parent: Option<i64>) -> i64 {
    let seq: i64 = conn
        .query_row("SELECT COALESCE(MAX(seq),0)+1 FROM groups", [], |r| {
            r.get(0)
        })
        .unwrap();
    conn.execute(
        "INSERT INTO groups (type_id, name, seq, parent_group_id, version, created_at_utc, updated_at_utc) \
         VALUES (?1, ?2, ?3, ?4, 1, ?5, ?5)",
        params![type_id, name, seq, parent, NOW],
    )
    .unwrap();
    conn.last_insert_rowid()
}

fn add_member(conn: &Connection, group: i64, device: i64) {
    conn.execute(
        "INSERT INTO group_devices (device_id, group_id, added_at_utc) VALUES (?1, ?2, ?3)",
        params![device, group, NOW],
    )
    .unwrap();
}

fn add_prop_value(conn: &Connection, group: i64, prop: i64, value_ref: i64) {
    conn.execute(
        "INSERT INTO group_property_values (group_id, property_id, value_ref, updated_at_utc) \
         VALUES (?1, ?2, ?3, ?4)",
        params![group, prop, value_ref, NOW],
    )
    .unwrap();
}

struct OriginFixture {
    user_a: i64,
    user_b: i64,
    pc_a: i64,
    sub_dev: i64,
    prn_link: i64,
    prn_usb: i64,
    prn_both: i64,
    prn_shared: i64,
    foreign_dev: i64,
    g1: i64,
    g2: i64,
}

fn origin_fixture(conn: &Connection) -> OriginFixture {
    let user_a = add_user(conn, "ua", "Иванов И.И.");
    let user_b = add_user(conn, "ub", "Петров П.П.");
    let pc_a = add_device(conn, "Устройство", "Системник 1", "INV-1");
    let sub_dev = add_device(conn, "Устройство", "Монитор вложенный", "INV-2");
    let prn_link = add_device(conn, "Принтер", "Принтер по ссылке", "INV-3");
    let prn_usb = add_device(conn, "Принтер", "Принтер USB", "INV-4");
    let prn_both = add_device(conn, "Принтер", "Принтер и член и USB", "INV-5");
    let prn_shared = add_device(conn, "Принтер", "Принтер 2-го этажа", "INV-6");
    let foreign_dev = add_device(conn, "Устройство", "Чужой АРМ", "INV-7");
    add_printer_usb(conn, prn_link, None);
    add_printer_usb(conn, prn_usb, Some(pc_a));
    add_printer_usb(conn, prn_both, Some(pc_a));
    add_printer_usb(conn, prn_shared, None);

    conn.execute(
        "INSERT INTO group_types (code, name, behavior, created_at_utc, updated_at_utc) \
         VALUES ('arm_t', 'АРМ-тест', 'workstation', ?1, ?1)",
        params![NOW],
    )
    .unwrap();
    let t = conn.last_insert_rowid();
    let prop = |name: &str, dt: &str| -> i64 {
        conn.execute(
            "INSERT INTO group_type_properties (type_id, name, data_type, created_at_utc, updated_at_utc) \
             VALUES (?1, ?2, ?3, ?4, ?4)",
            params![t, name, dt, NOW],
        )
        .unwrap();
        conn.last_insert_rowid()
    };
    let p_users = prop("Пользователи", "users");
    let p_refs = prop("Устройства", "device_refs");

    let g1 = add_group(conn, t, "АРМ #1", None);
    let sub = add_group(conn, t, "Системный блок", Some(g1));
    let g2 = add_group(conn, t, "АРМ #2", None);
    let g_foreign = add_group(conn, t, "АРМ чужой", None);
    add_prop_value(conn, g1, p_users, user_a);
    add_prop_value(conn, g2, p_users, user_a);
    add_prop_value(conn, g_foreign, p_users, user_b);
    add_prop_value(conn, g1, p_refs, prn_link);
    add_member(conn, g1, pc_a);
    add_member(conn, sub, sub_dev);
    add_member(conn, g1, prn_both);
    add_member(conn, g1, prn_shared);
    add_prop_value(conn, g2, p_refs, prn_shared);
    add_member(conn, g_foreign, foreign_dev);
    OriginFixture {
        user_a,
        user_b,
        pc_a,
        sub_dev,
        prn_link,
        prn_usb,
        prn_both,
        prn_shared,
        foreign_dev,
        g1,
        g2,
    }
}

#[test]
fn employee_device_origins_equals_scope_invariant() {
    let (conn, _g) = test_db();
    let f = origin_fixture(&conn);
    let ids: std::collections::BTreeSet<i64> = repo()
        .employee_device_ids(&conn, f.user_a)
        .unwrap()
        .into_iter()
        .collect();
    let got: std::collections::BTreeSet<i64> = repo()
        .employee_device_origins(&conn, f.user_a)
        .unwrap()
        .iter()
        .map(|r| r.device_id)
        .collect();
    assert_eq!(got, ids);
    // несущая проба: пусто не проходит, есть устройства всех трёх веток, чужого нет
    assert!(got.contains(&f.pc_a) && got.contains(&f.sub_dev)); // состав
    assert!(got.contains(&f.prn_link)); // ссылка
    assert!(got.contains(&f.prn_usb)); // USB
    assert!(!got.contains(&f.foreign_dev));
}

#[test]
fn employee_device_origins_names_bound_group_not_subgroup() {
    let (conn, _g) = test_db();
    let f = origin_fixture(&conn);
    let rows = repo().employee_device_origins(&conn, f.user_a).unwrap();
    let r = rows.iter().find(|r| r.device_id == f.sub_dev).unwrap();
    assert_eq!(r.group_name, "АРМ #1");
    assert_eq!(r.group_id, f.g1);
    assert!(rows.iter().all(|r| r.group_name != "Системный блок"));
}

#[test]
fn employee_device_origins_sources_and_priority() {
    use trackly_infra::repos::repairs_sqlite::DeviceOriginSource as S;
    let (conn, _g) = test_db();
    let f = origin_fixture(&conn);
    let rows = repo().employee_device_origins(&conn, f.user_a).unwrap();
    let srcs = |d: i64| -> Vec<S> {
        let mut v: Vec<S> = rows
            .iter()
            .filter(|r| r.device_id == d)
            .map(|r| r.source)
            .collect();
        v.sort_by_key(|s| s.rank());
        v
    };
    assert_eq!(srcs(f.prn_both), vec![S::Usb, S::Member]);
    assert_eq!(srcs(f.prn_usb), vec![S::Usb]);
    assert_eq!(srcs(f.prn_link), vec![S::Link]);
    assert!(S::Usb.rank() < S::Link.rank() && S::Link.rank() < S::Member.rank());
    let shared: Vec<i64> = rows
        .iter()
        .filter(|r| r.device_id == f.prn_shared)
        .map(|r| r.group_id)
        .collect();
    assert_eq!(shared.len(), 2);
    assert!(shared.contains(&f.g1) && shared.contains(&f.g2));
}

#[test]
fn employee_device_origins_exclusions_and_has_groups() {
    let (conn, _g) = test_db();
    let f = origin_fixture(&conn);
    // удалённое устройство и удалённая группа выпадают
    conn.execute(
        "UPDATE devices SET deleted_at_utc = ?2 WHERE id = ?1",
        params![f.pc_a, NOW],
    )
    .unwrap();
    // Удаление группы жёсткое (INT-12, quick 261121-hei): значения свойств и
    // членство g2 уходят каскадом.
    conn.execute("DELETE FROM groups WHERE id = ?1", params![f.g2])
        .unwrap();
    let rows = repo().employee_device_origins(&conn, f.user_a).unwrap();
    assert!(rows.iter().all(|r| r.device_id != f.pc_a));
    assert!(rows.iter().all(|r| r.group_id != f.g2));
    assert!(rows.iter().all(|r| r.device_id != f.foreign_dev));
    // пользователь без групп / несуществующий
    let c = add_user(&conn, "uc", "Сидоров С.С.");
    assert!(repo().employee_device_origins(&conn, c).unwrap().is_empty());
    assert!(!repo().employee_has_groups(&conn, c).unwrap());
    assert!(!repo().employee_has_groups(&conn, 999_999).unwrap());
    assert!(repo().employee_has_groups(&conn, f.user_a).unwrap());
    assert!(repo().employee_has_groups(&conn, f.user_b).unwrap());
    // группа без устройств всё равно считается группой
    conn.execute("DELETE FROM group_devices", []).unwrap();
    assert!(repo().employee_has_groups(&conn, f.user_b).unwrap());
}

#[test]
fn employee_device_ids_keeps_written_off_device() {
    use trackly_core::domain::repairs::DEVICE_STATUS_CODE_WRITTEN_OFF;
    assert_eq!(DEVICE_STATUS_CODE_WRITTEN_OFF, "списано");
    let (conn, _g) = test_db();
    let f = origin_fixture(&conn);
    conn.execute(
        "UPDATE devices SET status_id = (SELECT id FROM device_statuses WHERE code = ?2) WHERE id = ?1",
        params![f.pc_a, DEVICE_STATUS_CODE_WRITTEN_OFF],
    )
    .unwrap();
    assert!(repo()
        .employee_device_ids(&conn, f.user_a)
        .unwrap()
        .contains(&f.pc_a));
}
