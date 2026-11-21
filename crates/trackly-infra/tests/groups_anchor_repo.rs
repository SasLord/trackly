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

fn set_origin(conn: &Connection, device: i64, group: Option<i64>) {
    conn.execute(
        "UPDATE devices SET origin_group_id = ?2 WHERE id = ?1",
        params![device, group],
    )
    .expect("set origin");
}

/// Фикстура: разбор + четыре различающихся случая происхождения.
fn ghost_fixture(conn: &mut Connection, repo: &SqliteGroupRepository) -> (GroupRow, GroupRow) {
    let t = insert_type(conn, "workstation", "АРМ", "container");
    let razbor = make_group(conn, repo, t, "Разбор #7");
    let arm = make_group(conn, repo, t, "АРМ #3");
    insert_device(conn, 201, "SSD 256 ГБ"); // origin + выведен (состоит в АРМ #3)
    insert_device(conn, 202, "Планка памяти"); // origin + удалён
    insert_device(conn, 203, "Монитор #2"); // origin, но ещё член разбора
    insert_device(conn, 204, "Мышь"); // чужой origin
    insert_device(conn, 205, "Клавиатура"); // пустой origin
    insert_device(conn, 206, "Блок питания"); // origin + никуда не входит
    for d in [201, 202, 203, 206] {
        set_origin(conn, d, Some(razbor.id));
    }
    set_origin(conn, 204, Some(arm.id));
    add_device(conn, repo, arm.id, 201);
    add_device(conn, repo, razbor.id, 203);
    soft_delete_device(conn, 202);
    (razbor, arm)
}

#[test]
fn ghost_devices_include_removed_and_deleted_but_not_members_or_foreign() {
    let (mut conn, _g) = test_db();
    let repo = SqliteGroupRepository;
    let (razbor, arm) = ghost_fixture(&mut conn, &repo);

    let rows = repo.ghost_devices(&conn, razbor.id).unwrap();
    let ids: Vec<i64> = rows.iter().map(|r| r.device_id).collect();
    // живые раньше удалённых; внутри — по имени (бинарное сравнение: латиница < кириллица)
    assert_eq!(ids, vec![201, 206, 202]);
    assert!(!rows[0].is_deleted && !rows[1].is_deleted && rows[2].is_deleted);
    assert_eq!(rows[0].current_group_id, Some(arm.id));
    assert_eq!(rows[0].current_group_name.as_deref(), Some("АРМ #3"));
    assert_eq!(rows[1].current_group_id, None);
    assert_eq!(rows[0].name, "SSD 256 ГБ");
    assert_eq!(rows[0].inventory_number.as_deref(), Some("ИНВ-000201"));
}

#[test]
fn count_origin_ghosts_counts_all_including_deleted() {
    let (mut conn, _g) = test_db();
    let repo = SqliteGroupRepository;
    let (razbor, arm) = ghost_fixture(&mut conn, &repo);

    let tx = conn.transaction().unwrap();
    assert_eq!(
        repo.count_origin_ghosts_in_tx(&tx, razbor.id).unwrap(),
        3,
        "выведенный + удалённый + бесхозный; член разбора и чужой origin не считаются"
    );
    assert_eq!(repo.count_origin_ghosts_in_tx(&tx, arm.id).unwrap(), 1);
}

#[test]
fn member_counters_ignore_ghosts() {
    let (mut conn, _g) = test_db();
    let repo = SqliteGroupRepository;
    let (razbor, _arm) = ghost_fixture(&mut conn, &repo);

    let members = repo.member_devices(&conn, razbor.id).unwrap();
    assert_eq!(members.len(), 1, "D-11: в составе только живой член 203");
    assert_eq!(members[0].device_id, 203);
}

/// Страж архитектуры (Phase 41.8, D-09): запрет вложения «Разбора» живёт в сервисе;
/// репозиторный `set_parent_in_tx` им пользуется откат акта и обязан вкладывать как раньше.
#[test]
fn repo_set_parent_still_nests_teardown_group() {
    let (mut conn, _g) = test_db();
    let repo = SqliteGroupRepository;
    let t = insert_type(&conn, "workstation", "АРМ", "container");
    let td_t = insert_type(&conn, "teardown", "Разбор", "teardown");
    let parent = make_group(&mut conn, &repo, t, "АРМ #3");
    let td = make_group(&mut conn, &repo, td_t, "Разбор #7");
    let tx = conn.transaction().unwrap();
    repo.set_parent_in_tx(&tx, td.id, td.version, Some(parent.id), NOW)
        .expect("репозиторий не знает про запрет D-09");
    assert_eq!(
        repo.get_group_in_tx(&tx, td.id).unwrap().parent_group_id,
        Some(parent.id)
    );
}
