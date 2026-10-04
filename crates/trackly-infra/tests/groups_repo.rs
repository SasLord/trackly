//! Integration tests: `SqliteGroupRepository` на реальной БД (Phase 41 Plan 06,
//! GRP-04/05/06/08/10). Вымышленные имена и данные; фикстуры вставляются прямым SQL.

use rusqlite::{params, Connection};
use trackly_core::domain::groups::{GroupNew, GroupRow};
use trackly_core::error::AppError;
use trackly_core::ports::groups::GroupRepository;
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

fn insert_place(conn: &Connection, id: i64, name: &str) {
    conn.execute(
        "INSERT INTO places (id, parent_id, kind, name, is_storage, created_at_utc, updated_at_utc)
         VALUES (?1, NULL, 'building', ?2, 0, ?3, ?3)",
        params![id, name, NOW],
    )
    .expect("insert place");
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

/// Создаёт группу через репозиторий с очередным seq.
fn make_group(
    conn: &mut Connection,
    repo: &SqliteGroupRepository,
    type_id: i64,
    name: &str,
    place_id: Option<i64>,
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
                place_id,
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

fn set_parent(
    conn: &mut Connection,
    repo: &SqliteGroupRepository,
    id: i64,
    parent: Option<i64>,
) -> Result<GroupRow, AppError> {
    let version = repo.get_group(conn, id).unwrap().version;
    let tx = conn.transaction().unwrap();
    let res = repo.set_parent_in_tx(&tx, id, version, parent, NOW);
    if res.is_ok() {
        tx.commit().unwrap();
    }
    res
}

fn parent_in_db(conn: &Connection, id: i64) -> Option<i64> {
    conn.query_row(
        "SELECT parent_group_id FROM groups WHERE id = ?1",
        params![id],
        |r| r.get(0),
    )
    .unwrap()
}

#[test]
fn next_seq_uses_column_not_name() {
    let (mut conn, _g) = test_db();
    let repo = SqliteGroupRepository;
    let t = insert_type(&conn, "workstation", "АРМ", "container");

    {
        let tx = conn.transaction().unwrap();
        assert_eq!(repo.next_seq_in_tx(&tx, t).unwrap(), 1, "пустой тип");
    }
    let g1 = make_group(&mut conn, &repo, t, "АРМ #1", None);
    let g2 = make_group(&mut conn, &repo, t, "АРМ #2", None);
    assert_eq!((g1.seq, g2.seq), (1, 2));
    {
        let tx = conn.transaction().unwrap();
        assert_eq!(repo.next_seq_in_tx(&tx, t).unwrap(), 3, "после двух групп");
    }

    // Переименование первой в «АРМ #99» не влияет на нумерацию.
    let tx = conn.transaction().unwrap();
    repo.rename_group_in_tx(&tx, g1.id, g1.version, "АРМ #99", NOW)
        .unwrap();
    tx.commit().unwrap();
    let g3 = make_group(&mut conn, &repo, t, "Рабочее место бухгалтера", None);
    assert_eq!(g3.seq, 3, "имя не влияет на seq");

    // Другой тип нумеруется независимо.
    let t2 = insert_type(&conn, "rack", "Стойка", "container");
    assert_eq!(make_group(&mut conn, &repo, t2, "Стойка А", None).seq, 1);

    // Удаление последней группы возвращает её seq (принятое поведение MAX+1).
    let tx = conn.transaction().unwrap();
    repo.delete_group_in_tx(&tx, g3.id).unwrap();
    assert_eq!(repo.next_seq_in_tx(&tx, t).unwrap(), 3);
    tx.commit().unwrap();
}

#[test]
fn rename_cas_rejects_stale_version() {
    let (mut conn, _g) = test_db();
    let repo = SqliteGroupRepository;
    let t = insert_type(&conn, "rack", "Стойка", "container");
    let g = make_group(&mut conn, &repo, t, "Стойка А", None);
    let tx = conn.transaction().unwrap();
    let renamed = repo
        .rename_group_in_tx(&tx, g.id, g.version, "Стойка Б", NOW + 5)
        .unwrap();
    assert_eq!(renamed.name, "Стойка Б");
    assert_eq!(renamed.version, g.version + 1);
    let stale = repo.rename_group_in_tx(&tx, g.id, g.version, "Стойка В", NOW + 6);
    assert!(matches!(
        stale,
        Err(AppError::OptimisticLockMismatch { .. })
    ));
    let missing = repo.rename_group_in_tx(&tx, 9999, 1, "Х", NOW);
    assert!(matches!(missing, Err(AppError::NotFound { .. })));
}

#[test]
fn device_can_belong_to_only_one_group() {
    let (mut conn, _g) = test_db();
    let repo = SqliteGroupRepository;
    let t = insert_type(&conn, "workstation", "АРМ", "container");
    let a = make_group(&mut conn, &repo, t, "АРМ #1", None);
    let b = make_group(&mut conn, &repo, t, "АРМ #2", None);
    insert_device(&conn, 1, "Монитор");

    add_device(&mut conn, &repo, a.id, 1);

    let tx = conn.transaction().unwrap();
    let err = repo.add_device_in_tx(&tx, b.id, 1, NOW).unwrap_err();
    match err {
        AppError::Conflict { reason } => assert!(reason.contains('1'), "reason: {reason}"),
        other => panic!("ожидали Conflict, получили {other:?}"),
    }
    let of = repo.group_of_device_in_tx(&tx, 1).unwrap().unwrap();
    assert_eq!(of.id, a.id, "прежняя группа не изменилась");
    drop(tx);

    // Прямой INSERT в обход предпроверки тоже отклоняется PK.
    let direct = conn.execute(
        "INSERT INTO group_devices (device_id, group_id, added_at_utc) VALUES (1, ?1, ?2)",
        params![b.id, NOW],
    );
    assert!(direct.is_err(), "PK group_devices.device_id — бэкстоп");

    // remove_device освобождает устройство для другой группы.
    let tx = conn.transaction().unwrap();
    assert!(repo.remove_device_in_tx(&tx, 1).unwrap());
    assert!(!repo.remove_device_in_tx(&tx, 1).unwrap());
    repo.add_device_in_tx(&tx, b.id, 1, NOW).unwrap();
    assert_eq!(
        repo.group_of_device_in_tx(&tx, 1).unwrap().unwrap().id,
        b.id
    );
}

#[test]
fn locked_group_requires_place() {
    let (mut conn, _g) = test_db();
    let repo = SqliteGroupRepository;
    insert_place(&conn, 1, "Корпус А");
    let t = insert_type(&conn, "workstation", "АРМ", "container");
    let placed = make_group(&mut conn, &repo, t, "АРМ с местом", Some(1));
    let unplaced = make_group(&mut conn, &repo, t, "АРМ без места", None);
    insert_device(&conn, 1, "Системный блок");
    insert_device(&conn, 2, "Клавиатура");
    insert_device(&conn, 3, "Мышь");
    add_device(&mut conn, &repo, placed.id, 1);
    add_device(&mut conn, &repo, unplaced.id, 2);

    let tx = conn.transaction().unwrap();
    let locked = repo.locked_group_for_device_in_tx(&tx, 1).unwrap();
    assert_eq!(
        locked.map(|g| g.id),
        Some(placed.id),
        "группа с местом -> Some"
    );
    assert!(
        repo.locked_group_for_device_in_tx(&tx, 2)
            .unwrap()
            .is_none(),
        "группа без места -> None (запрет спит)"
    );
    assert!(
        repo.locked_group_for_device_in_tx(&tx, 3)
            .unwrap()
            .is_none(),
        "вне групп -> None"
    );
    // Но членство без места по-прежнему видно.
    assert_eq!(
        repo.group_of_device_in_tx(&tx, 2).unwrap().map(|g| g.id),
        Some(unplaced.id)
    );
}

#[test]
fn cycle_detection_and_unchanged_parent() {
    let (mut conn, _g) = test_db();
    let repo = SqliteGroupRepository;
    let t = insert_type(&conn, "rack", "Стойка", "container");
    let a = make_group(&mut conn, &repo, t, "A", None);
    let b = make_group(&mut conn, &repo, t, "B", None);
    let c = make_group(&mut conn, &repo, t, "C", None);
    let lone = make_group(&mut conn, &repo, t, "Одиночка", None);
    set_parent(&mut conn, &repo, b.id, Some(a.id)).unwrap();
    set_parent(&mut conn, &repo, c.id, Some(b.id)).unwrap();

    {
        let tx = conn.transaction().unwrap();
        assert!(
            repo.would_create_cycle_in_tx(&tx, a.id, a.id).unwrap(),
            "сама в себя"
        );
        assert!(
            repo.would_create_cycle_in_tx(&tx, a.id, c.id).unwrap(),
            "A под потомка C"
        );
        assert!(
            repo.would_create_cycle_in_tx(&tx, a.id, b.id).unwrap(),
            "A под непосредственного потомка"
        );
        assert!(
            !repo.would_create_cycle_in_tx(&tx, a.id, lone.id).unwrap(),
            "несвязанная"
        );
        assert!(
            !repo.would_create_cycle_in_tx(&tx, c.id, a.id).unwrap(),
            "вверх по цепочке — не цикл"
        );
    }

    let before = repo.get_group(&conn, a.id).unwrap();
    for target in [a.id, c.id] {
        let err = set_parent(&mut conn, &repo, a.id, Some(target)).unwrap_err();
        match err {
            AppError::Validation { field, message } => {
                assert_eq!(field, "parent_group_id");
                assert_eq!(
                    message,
                    "Нельзя вложить группу в саму себя или в свою вложенную группу."
                );
            }
            other => panic!("ожидали Validation, получили {other:?}"),
        }
        assert_eq!(
            parent_in_db(&conn, a.id),
            None,
            "parent_group_id не изменился"
        );
    }
    let after = repo.get_group(&conn, a.id).unwrap();
    assert_eq!(
        after.version, before.version,
        "версия не выросла при отказе"
    );
    assert_eq!(after.updated_at_utc, before.updated_at_utc);

    // Корректная смена и вынос в корень работают.
    set_parent(&mut conn, &repo, c.id, Some(lone.id)).unwrap();
    assert_eq!(parent_in_db(&conn, c.id), Some(lone.id));
    set_parent(&mut conn, &repo, c.id, None).unwrap();
    assert_eq!(parent_in_db(&conn, c.id), None);
}

#[test]
fn teardown_group_cannot_be_parent() {
    let (mut conn, _g) = test_db();
    let repo = SqliteGroupRepository;
    let rack = insert_type(&conn, "rack", "Стойка", "container");
    let td = insert_type(&conn, "dismantle", "Разбор", "teardown");
    let child = make_group(&mut conn, &repo, rack, "Стойка А", None);
    let teardown = make_group(&mut conn, &repo, td, "Разбор #1", None);

    let err = set_parent(&mut conn, &repo, child.id, Some(teardown.id)).unwrap_err();
    match err {
        AppError::Validation { field, message } => {
            assert_eq!(field, "parent_group_id");
            assert_eq!(
                message,
                "Группа типа «Разбор» не может содержать другие группы."
            );
        }
        other => panic!("ожидали Validation, получили {other:?}"),
    }
    assert_eq!(parent_in_db(&conn, child.id), None);

    // Сам teardown может быть вложен в контейнер (проверка касается только родителя).
    set_parent(&mut conn, &repo, teardown.id, Some(child.id)).unwrap();
    assert_eq!(parent_in_db(&conn, teardown.id), Some(child.id));
}

#[test]
fn subtree_device_ids_include_nested_and_skip_soft_deleted() {
    let (mut conn, _g) = test_db();
    let repo = SqliteGroupRepository;
    let t = insert_type(&conn, "rack", "Стойка", "container");
    let root = make_group(&mut conn, &repo, t, "Корень", None);
    let nested = make_group(&mut conn, &repo, t, "Вложенная", None);
    let other = make_group(&mut conn, &repo, t, "Чужая", None);
    set_parent(&mut conn, &repo, nested.id, Some(root.id)).unwrap();
    for id in 1..=8 {
        insert_device(&conn, id, &format!("Устройство {id}"));
    }
    for id in 1..=4 {
        add_device(&mut conn, &repo, root.id, id);
    }
    for id in 5..=6 {
        add_device(&mut conn, &repo, nested.id, id);
    }
    add_device(&mut conn, &repo, other.id, 7);
    add_device(&mut conn, &repo, other.id, 8);

    let tx = conn.transaction().unwrap();
    assert_eq!(
        repo.subtree_device_ids_in_tx(&tx, root.id).unwrap(),
        vec![1, 2, 3, 4, 5, 6]
    );
    assert_eq!(
        repo.subtree_group_ids_in_tx(&tx, root.id).unwrap(),
        vec![root.id, nested.id]
    );
    assert_eq!(
        repo.subtree_device_ids_in_tx(&tx, nested.id).unwrap(),
        vec![5, 6]
    );
    drop(tx);

    soft_delete_device(&conn, 2);
    let tx = conn.transaction().unwrap();
    assert_eq!(
        repo.subtree_device_ids_in_tx(&tx, root.id).unwrap(),
        vec![1, 3, 4, 5, 6],
        "мягко удалённое устройство не входит в состав"
    );
}

#[test]
fn root_group_id_walks_up_chain() {
    let (mut conn, _g) = test_db();
    let repo = SqliteGroupRepository;
    let t = insert_type(&conn, "rack", "Стойка", "container");
    let a = make_group(&mut conn, &repo, t, "A", None);
    let b = make_group(&mut conn, &repo, t, "B", None);
    let c = make_group(&mut conn, &repo, t, "C", None);
    set_parent(&mut conn, &repo, b.id, Some(a.id)).unwrap();
    set_parent(&mut conn, &repo, c.id, Some(b.id)).unwrap();

    let tx = conn.transaction().unwrap();
    assert_eq!(repo.root_group_id_in_tx(&tx, c.id).unwrap(), a.id);
    assert_eq!(repo.root_group_id_in_tx(&tx, b.id).unwrap(), a.id);
    assert_eq!(
        repo.root_group_id_in_tx(&tx, a.id).unwrap(),
        a.id,
        "для корня — он сам"
    );
    assert!(matches!(
        repo.root_group_id_in_tx(&tx, 9999),
        Err(AppError::NotFound { .. })
    ));
}

#[test]
fn delete_group_cascades_members_and_detaches_children() {
    let (mut conn, _g) = test_db();
    let repo = SqliteGroupRepository;
    insert_place(&conn, 1, "Корпус А");
    let t = insert_type(&conn, "rack", "Стойка", "container");
    let root = make_group(&mut conn, &repo, t, "Корень", Some(1));
    let nested = make_group(&mut conn, &repo, t, "Вложенная", Some(1));
    set_parent(&mut conn, &repo, nested.id, Some(root.id)).unwrap();
    insert_device(&conn, 1, "Сервер");
    add_device(&mut conn, &repo, root.id, 1);
    conn.execute("UPDATE devices SET place_id = 1 WHERE id = 1", [])
        .unwrap();

    let tx = conn.transaction().unwrap();
    repo.delete_group_in_tx(&tx, root.id).unwrap();
    tx.commit().unwrap();

    let members: i64 = conn
        .query_row("SELECT COUNT(*) FROM group_devices", [], |r| r.get(0))
        .unwrap();
    assert_eq!(members, 0, "строки group_devices исчезли");
    assert_eq!(
        parent_in_db(&conn, nested.id),
        None,
        "вложенная осталась в корне"
    );
    assert!(repo.get_group(&conn, nested.id).is_ok());
    assert!(matches!(
        repo.get_group(&conn, root.id),
        Err(AppError::NotFound { .. })
    ));
    let place: Option<i64> = conn
        .query_row("SELECT place_id FROM devices WHERE id = 1", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(
        place,
        Some(1),
        "devices.place_id бывшего члена не изменился"
    );

    let tx = conn.transaction().unwrap();
    assert!(matches!(
        repo.delete_group_in_tx(&tx, root.id),
        Err(AppError::NotFound { .. })
    ));
}

#[test]
fn set_subtree_place_updates_all_nested_groups_only() {
    let (mut conn, _g) = test_db();
    let repo = SqliteGroupRepository;
    insert_place(&conn, 1, "Корпус А");
    insert_place(&conn, 2, "Корпус Б");
    let t = insert_type(&conn, "rack", "Стойка", "container");
    let root = make_group(&mut conn, &repo, t, "Корень", None);
    let nested = make_group(&mut conn, &repo, t, "Вложенная", None);
    let other = make_group(&mut conn, &repo, t, "Чужая", Some(2));
    set_parent(&mut conn, &repo, nested.id, Some(root.id)).unwrap();
    let nested_before = repo.get_group(&conn, nested.id).unwrap();

    let tx = conn.transaction().unwrap();
    let n = repo
        .set_subtree_place_in_tx(&tx, root.id, Some(1), NOW + 10)
        .unwrap();
    tx.commit().unwrap();
    assert_eq!(n, 2);
    let nested_after = repo.get_group(&conn, nested.id).unwrap();
    assert_eq!(nested_after.place_id, Some(1));
    assert_eq!(nested_after.version, nested_before.version + 1);
    assert_eq!(nested_after.updated_at_utc, NOW + 10);
    assert_eq!(repo.get_group(&conn, root.id).unwrap().place_id, Some(1));
    assert_eq!(
        repo.get_group(&conn, other.id).unwrap().place_id,
        Some(2),
        "чужая не тронута"
    );

    let tx = conn.transaction().unwrap();
    repo.set_subtree_place_in_tx(&tx, root.id, None, NOW + 11)
        .unwrap();
    tx.commit().unwrap();
    assert_eq!(repo.get_group(&conn, nested.id).unwrap().place_id, None);
}

#[test]
fn reads_groups_for_devices_children_and_list() {
    let (mut conn, _g) = test_db();
    let repo = SqliteGroupRepository;
    let t1 = insert_type(&conn, "rack", "Стойка", "container");
    let t2 = insert_type(&conn, "workstation", "АРМ", "container");
    let a = make_group(&mut conn, &repo, t1, "Стойка А", None);
    let b = make_group(&mut conn, &repo, t2, "АРМ #1", None);
    let c = make_group(&mut conn, &repo, t2, "АРМ #2", None);
    set_parent(&mut conn, &repo, b.id, Some(a.id)).unwrap();
    set_parent(&mut conn, &repo, c.id, Some(a.id)).unwrap();
    for id in 1..=3 {
        insert_device(&conn, id, &format!("Устройство {id}"));
    }
    add_device(&mut conn, &repo, a.id, 1);
    add_device(&mut conn, &repo, b.id, 2);

    let pairs = repo.groups_for_devices(&conn, &[1, 2, 3]).unwrap();
    let got: Vec<(i64, i64)> = pairs.iter().map(|(d, g)| (*d, g.id)).collect();
    assert_eq!(
        got,
        vec![(1, a.id), (2, b.id)],
        "устройство 3 вне групп — не в ответе"
    );
    assert!(repo.groups_for_devices(&conn, &[]).unwrap().is_empty());

    let list: Vec<i64> = repo
        .list_groups(&conn)
        .unwrap()
        .iter()
        .map(|g| g.id)
        .collect();
    assert_eq!(list, vec![a.id, b.id, c.id], "ORDER BY type_id, seq");

    let kids: Vec<i64> = repo
        .direct_child_groups(&conn, a.id)
        .unwrap()
        .iter()
        .map(|g| g.id)
        .collect();
    assert_eq!(kids, vec![b.id, c.id]);
    assert!(repo.direct_child_groups(&conn, b.id).unwrap().is_empty());
}
