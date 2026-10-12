//! Integration tests: `status_code`, `origin_group_id` на строке устройства на
//! всех путях чтения и status-only запись `set_status_in_tx`
//! (Phase 41.1 Plan 07, GRD-04 / GRD-03). Вымышленные имена и данные.

use rusqlite::{params, Connection};
use trackly_core::domain::devices::{DeviceFilter, DeviceNew, Pagination};
use trackly_core::domain::groups::GroupNew;
use trackly_core::error::AppError;
use trackly_core::ports::devices::DeviceRepository;
use trackly_infra::repos::{SqliteDeviceRepository, SqliteGroupRepository};
use trackly_infra::test_support::test_db;

const NOW: i64 = 1_700_000_000;

fn status_id(conn: &Connection, code: &str) -> i64 {
    conn.query_row(
        "SELECT id FROM device_statuses WHERE code = ?1",
        params![code],
        |r| r.get(0),
    )
    .expect("status by code")
}

fn new_device(name: &str, inv: Option<&str>, status_id: i64) -> DeviceNew {
    DeviceNew {
        type_id: 1,
        name: name.to_string(),
        inventory_no: inv.map(str::to_string),
        serial_no: None,
        model: None,
        specs: None,
        kit: None,
        state: None,
        place_id: None,
        status_id,
    }
}

fn page() -> Pagination {
    Pagination {
        offset: 0,
        limit: 50,
    }
}

fn make_group(conn: &mut Connection) -> i64 {
    conn.execute(
        "INSERT INTO group_types (code, name, behavior, created_at_utc, updated_at_utc)
         VALUES ('t_origin', 'Тип', 'repair', ?1, ?1)",
        params![NOW],
    )
    .expect("insert type");
    let type_id = conn.last_insert_rowid();
    let repo = SqliteGroupRepository;
    let tx = conn.transaction().unwrap();
    let seq = repo.next_seq_in_tx(&tx, type_id).unwrap();
    let id = repo
        .insert_group_in_tx(
            &tx,
            &GroupNew {
                type_id,
                name: "Разбор А".to_string(),
                seq,
                place_id: None,
            },
            NOW,
        )
        .unwrap();
    tx.commit().unwrap();
    id
}

/// Test 1: get / get_in_tx / list / search_fts / list_by_ids несут code статуса.
#[test]
fn every_read_path_carries_status_code() {
    let (mut conn, _dir) = test_db();
    let repo = SqliteDeviceRepository;
    let mut ids = Vec::new();
    for (i, code) in ["на_складе", "в_работе", "на_ремонте", "списано"]
        .iter()
        .enumerate()
    {
        let sid = status_id(&conn, code);
        let inv = format!("ИНВ-{:06}", i + 1);
        let id = repo
            .create(&mut conn, &new_device("Ноутбук", Some(&inv), sid), NOW)
            .expect("create");
        ids.push((id, *code));
    }

    for (id, code) in &ids {
        assert_eq!(repo.get(&conn, *id).unwrap().status_code, *code, "get");
    }
    {
        let tx = conn.transaction().unwrap();
        for (id, code) in &ids {
            assert_eq!(
                repo.get_in_tx(&tx, *id).unwrap().status_code,
                *code,
                "get_in_tx"
            );
        }
    }

    let (rows, _) = repo
        .list(
            &conn,
            &DeviceFilter {
                include_deleted: true,
                ..DeviceFilter::default()
            },
            &page(),
        )
        .unwrap();
    assert_eq!(rows.len(), 4);
    for r in &rows {
        let expected = ids.iter().find(|(i, _)| *i == r.id).unwrap().1;
        assert_eq!(r.status_code, expected, "list");
    }

    let (hits, _) = repo.search_fts(&conn, "Ноутбук", &page()).unwrap();
    assert_eq!(hits.len(), 4, "search_fts");
    for r in &hits {
        let expected = ids.iter().find(|(i, _)| *i == r.id).unwrap().1;
        assert_eq!(r.status_code, expected, "search_fts");
    }

    let all: Vec<i64> = ids.iter().map(|(i, _)| *i).collect();
    let by_ids = repo.list_by_ids(&conn, &all).unwrap();
    assert_eq!(by_ids.len(), 4);
    for r in &by_ids {
        let expected = ids.iter().find(|(i, _)| *i == r.id).unwrap().1;
        assert_eq!(r.status_code, expected, "list_by_ids");
    }
}

/// Test 2: origin_group_id NULL по умолчанию и читается с колонки на всех путях.
#[test]
fn origin_group_id_defaults_null_and_reads_column() {
    let (mut conn, _dir) = test_db();
    let repo = SqliteDeviceRepository;
    let sid = status_id(&conn, "на_складе");
    let id = repo
        .create(
            &mut conn,
            &new_device("Ноутбук", Some("ИНВ-000001"), sid),
            NOW,
        )
        .unwrap();
    assert_eq!(repo.get(&conn, id).unwrap().origin_group_id, None);

    let gid = make_group(&mut conn);
    conn.execute(
        "UPDATE devices SET origin_group_id = ?2 WHERE id = ?1",
        params![id, gid],
    )
    .unwrap();

    assert_eq!(repo.get(&conn, id).unwrap().origin_group_id, Some(gid));
    let (rows, _) = repo.list(&conn, &DeviceFilter::default(), &page()).unwrap();
    assert_eq!(rows[0].origin_group_id, Some(gid), "list");
    let (hits, _) = repo.search_fts(&conn, "Ноутбук", &page()).unwrap();
    assert_eq!(hits[0].origin_group_id, Some(gid), "search_fts");
    assert_eq!(
        repo.list_by_ids(&conn, &[id]).unwrap()[0].origin_group_id,
        Some(gid),
        "list_by_ids"
    );
}

/// Test 3: set_status_in_tx меняет статус и version, не трогает place_id.
#[test]
fn set_status_in_tx_changes_status_only() {
    let (mut conn, _dir) = test_db();
    let repo = SqliteDeviceRepository;
    conn.execute(
        "INSERT INTO places (kind, name, created_at_utc, updated_at_utc)
         VALUES ('room', 'Кабинет 1', ?1, ?1)",
        params![NOW],
    )
    .expect("insert place");
    let place = conn.last_insert_rowid();
    let sid = status_id(&conn, "в_работе");
    let mut dn = new_device("Ноутбук", Some("ИНВ-000001"), sid);
    dn.place_id = Some(place);
    let id = repo.create(&mut conn, &dn, NOW).unwrap();
    let before = repo.get(&conn, id).unwrap();

    let target = status_id(&conn, "на_списание");
    let tx = conn.transaction().unwrap();
    repo.set_status_in_tx(&tx, id, target, NOW + 100).unwrap();
    tx.commit().unwrap();

    let after = repo.get(&conn, id).unwrap();
    assert_eq!(after.status_id, target);
    assert_eq!(after.status_code, "на_списание");
    assert_eq!(after.version, before.version + 1);
    assert_eq!(after.updated_at_utc, NOW + 100);
    assert_eq!(after.place_id, Some(place), "место не тронуто");
}

#[test]
fn set_status_in_tx_rejects_missing_and_soft_deleted() {
    let (mut conn, _dir) = test_db();
    let repo = SqliteDeviceRepository;
    let sid = status_id(&conn, "на_складе");
    let id = repo
        .create(
            &mut conn,
            &new_device("Ноутбук", Some("ИНВ-000001"), sid),
            NOW,
        )
        .unwrap();
    conn.execute(
        "UPDATE devices SET deleted_at_utc = ?2 WHERE id = ?1",
        params![id, NOW],
    )
    .unwrap();
    let target = status_id(&conn, "на_списание");
    let tx = conn.transaction().unwrap();
    assert!(matches!(
        repo.set_status_in_tx(&tx, id, target, NOW),
        Err(AppError::NotFound { .. })
    ));
    assert!(matches!(
        repo.set_status_in_tx(&tx, 999_999, target, NOW),
        Err(AppError::NotFound { .. })
    ));
}

/// Test 4: представитель list_grouped несёт непустой status_code.
#[test]
fn list_grouped_repr_carries_status_code() {
    let (mut conn, _dir) = test_db();
    let repo = SqliteDeviceRepository;
    let sid = status_id(&conn, "в_работе");
    for _ in 0..2 {
        repo.create(&mut conn, &new_device("Кабель", None, sid), NOW)
            .unwrap();
    }
    let groups = repo
        .list_grouped(&conn, &DeviceFilter::default(), &page())
        .unwrap();
    assert!(!groups.is_empty(), "тест не вакуумен: группы есть");
    for g in &groups {
        assert_eq!(g.repr.status_code, "в_работе");
        assert_eq!(g.repr.origin_group_id, None);
    }
}
