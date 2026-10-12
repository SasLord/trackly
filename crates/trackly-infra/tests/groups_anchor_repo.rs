//! Integration tests: примитивы якоря и происхождения в `SqliteGroupRepository`
//! (Phase 41.1 Plan 06, GRD-02 / GRD-03 / GRP-10). Вымышленные имена и данные.

use rusqlite::{params, Connection};
use trackly_core::domain::groups::{GroupNew, GroupRow};
use trackly_infra::repos::SqliteGroupRepository;
use trackly_infra::test_support::test_db;

const NOW: i64 = 1_700_000_000;

fn insert_type(conn: &Connection, code: &str, name: &str, behavior: &str) -> i64 {
    conn.execute(
        "INSERT INTO group_types (code, name, behavior, created_at_utc, updated_at_utc)
         VALUES (?1, ?2, ?3, ?4, ?4)",
        params![code, name, behavior, NOW],
    )
    .expect("insert type");
    conn.last_insert_rowid()
}

fn insert_device(conn: &Connection, id: i64, name: &str) {
    conn.execute(
        "INSERT INTO devices (id, type_id, name, inventory_number, status_id, created_at_utc, updated_at_utc)
         VALUES (?1, 1, ?2, ?3, 2, ?4, ?4)",
        params![id, name, format!("ИНВ-{id:06}"), NOW],
    )
    .expect("insert device");
}

fn soft_delete_device(conn: &Connection, id: i64) {
    conn.execute(
        "UPDATE devices SET deleted_at_utc = ?2 WHERE id = ?1",
        params![id, NOW],
    )
    .expect("soft delete device");
}

fn make_group(
    conn: &mut Connection,
    repo: &SqliteGroupRepository,
    type_id: i64,
    name: &str,
) -> GroupRow {
    let tx = conn.transaction().unwrap();
    let seq = repo.next_seq_in_tx(&tx, type_id).unwrap();
    let id = repo
        .insert_group_in_tx(
            &tx,
            &GroupNew {
                type_id,
                name: name.to_string(),
                seq,
                place_id: None,
            },
            NOW,
        )
        .unwrap();
    let row = repo.get_group_in_tx(&tx, id).unwrap();
    tx.commit().unwrap();
    row
}

fn add_device(conn: &mut Connection, repo: &SqliteGroupRepository, group: i64, device: i64) {
    let tx = conn.transaction().unwrap();
    repo.add_device_in_tx(&tx, group, device, NOW).unwrap();
    tx.commit().unwrap();
}

fn origin_in_db(conn: &Connection, device_id: i64) -> Option<i64> {
    conn.query_row(
        "SELECT origin_group_id FROM devices WHERE id = ?1",
        params![device_id],
        |r| r.get(0),
    )
    .unwrap()
}

#[test]
fn anchor_group_of_device_finds_only_the_anchor() {
    let (mut conn, _g) = test_db();
    let repo = SqliteGroupRepository;
    let t = insert_type(&conn, "workstation", "АРМ", "container");
    insert_device(&conn, 101, "Системный блок #4");
    insert_device(&conn, 102, "Монитор #2");
    insert_device(&conn, 103, "Планка памяти");
    let g = make_group(&mut conn, &repo, t, "АРМ #1");
    {
        let tx = conn.transaction().unwrap();
        repo.set_anchor_in_tx(&tx, g.id, 101).unwrap();
        tx.commit().unwrap();
    }
    add_device(&mut conn, &repo, g.id, 101);
    add_device(&mut conn, &repo, g.id, 102);

    let tx = conn.transaction().unwrap();
    let found = repo.anchor_group_of_device_in_tx(&tx, 101).unwrap();
    assert_eq!(found.map(|r| r.id), Some(g.id), "якорь находит свою группу");
    assert!(
        repo.anchor_group_of_device_in_tx(&tx, 102)
            .unwrap()
            .is_none(),
        "член группы, не якорь"
    );
    assert!(
        repo.anchor_group_of_device_in_tx(&tx, 103)
            .unwrap()
            .is_none(),
        "обычное устройство"
    );
}

#[test]
fn same_device_cannot_anchor_two_groups() {
    let (mut conn, _g) = test_db();
    let repo = SqliteGroupRepository;
    let t = insert_type(&conn, "workstation", "АРМ", "container");
    insert_device(&conn, 101, "Системный блок #4");
    let g1 = make_group(&mut conn, &repo, t, "АРМ #1");
    let g2 = make_group(&mut conn, &repo, t, "АРМ #2");

    let tx = conn.transaction().unwrap();
    repo.set_anchor_in_tx(&tx, g1.id, 101).unwrap();
    let second = repo.set_anchor_in_tx(&tx, g2.id, 101);
    assert!(second.is_err(), "уникальный индекс idx_groups_anchor");
}

#[test]
fn set_anchor_does_not_bump_version() {
    let (mut conn, _g) = test_db();
    let repo = SqliteGroupRepository;
    let t = insert_type(&conn, "workstation", "АРМ", "container");
    insert_device(&conn, 101, "Системный блок #4");
    let g = make_group(&mut conn, &repo, t, "АРМ #1");

    let tx = conn.transaction().unwrap();
    repo.set_anchor_in_tx(&tx, g.id, 101).unwrap();
    let after = repo.get_group_in_tx(&tx, g.id).unwrap();
    assert_eq!(
        after.version, g.version,
        "CAS открытой карточки не ломается"
    );
    assert_eq!(after.updated_at_utc, g.updated_at_utc);
}

#[test]
fn origin_is_written_only_while_null() {
    let (mut conn, _g) = test_db();
    let repo = SqliteGroupRepository;
    let t = insert_type(&conn, "workstation", "АРМ", "container");
    insert_device(&conn, 103, "Планка памяти");
    let g1 = make_group(&mut conn, &repo, t, "АРМ #1");
    let g2 = make_group(&mut conn, &repo, t, "АРМ #2");

    {
        let tx = conn.transaction().unwrap();
        assert!(repo
            .set_device_origin_if_null_in_tx(&tx, 103, g1.id)
            .unwrap());
        tx.commit().unwrap();
    }
    assert_eq!(origin_in_db(&conn, 103), Some(g1.id));

    {
        let tx = conn.transaction().unwrap();
        assert!(
            !repo
                .set_device_origin_if_null_in_tx(&tx, 103, g2.id)
                .unwrap(),
            "повторная запись не проходит"
        );
        tx.commit().unwrap();
    }
    assert_eq!(
        origin_in_db(&conn, 103),
        Some(g1.id),
        "D-33: не перезатёрто"
    );
}

#[test]
fn count_live_parts_excludes_anchor_and_soft_deleted() {
    let (mut conn, _g) = test_db();
    let repo = SqliteGroupRepository;
    let t = insert_type(&conn, "workstation", "АРМ", "container");
    insert_device(&conn, 101, "Системный блок #4");
    insert_device(&conn, 102, "Монитор #2");
    insert_device(&conn, 103, "Планка памяти");
    insert_device(&conn, 104, "Планка памяти удалённая");
    let g = make_group(&mut conn, &repo, t, "АРМ #1");
    {
        let tx = conn.transaction().unwrap();
        repo.set_anchor_in_tx(&tx, g.id, 101).unwrap();
        tx.commit().unwrap();
    }
    for d in [101, 102, 103, 104] {
        add_device(&mut conn, &repo, g.id, d);
    }
    soft_delete_device(&conn, 104);

    let tx = conn.transaction().unwrap();
    assert_eq!(repo.count_live_parts_in_tx(&tx, g.id).unwrap(), 2);
}

#[test]
fn count_live_parts_of_plain_group_counts_everyone() {
    let (mut conn, _g) = test_db();
    let repo = SqliteGroupRepository;
    let t = insert_type(&conn, "workstation", "АРМ", "container");
    insert_device(&conn, 102, "Монитор #2");
    let g = make_group(&mut conn, &repo, t, "АРМ #1");
    add_device(&mut conn, &repo, g.id, 102);

    let tx = conn.transaction().unwrap();
    assert_eq!(
        repo.count_live_parts_in_tx(&tx, g.id).unwrap(),
        1,
        "у группы без якоря подзапрос даёт NULL, IS NOT считает устройство деталью"
    );
}

#[test]
fn get_group_reports_anchor_id_and_name() {
    let (mut conn, _g) = test_db();
    let repo = SqliteGroupRepository;
    let t = insert_type(&conn, "workstation", "АРМ", "container");
    insert_device(&conn, 101, "Системный блок #4");
    let anchored = make_group(&mut conn, &repo, t, "АРМ #1");
    let plain = make_group(&mut conn, &repo, t, "АРМ #2");
    assert_eq!(plain.anchor_device_id, None);
    assert_eq!(plain.anchor_device_name, None);

    let tx = conn.transaction().unwrap();
    repo.set_anchor_in_tx(&tx, anchored.id, 101).unwrap();
    let row = repo.get_group_in_tx(&tx, anchored.id).unwrap();
    assert_eq!(row.anchor_device_id, Some(101));
    assert_eq!(row.anchor_device_name.as_deref(), Some("Системный блок #4"));
    let plain_row = repo.get_group_in_tx(&tx, plain.id).unwrap();
    assert_eq!(
        (plain_row.anchor_device_id, plain_row.anchor_device_name),
        (None, None)
    );
}
