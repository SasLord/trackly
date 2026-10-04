//! Integration tests: `SqliteGroupRepository` на реальной БД (Phase 41 Plan 06,
//! GRP-04/05/06/08/10). Вымышленные имена и данные; фикстуры вставляются прямым SQL.

use rusqlite::{params, Connection};
use trackly_core::domain::groups::{GroupNew, GroupRow, GroupValueRow};
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

// ---- Задача 2: значения, производные принтеры, счётчики дерева ----------------

fn insert_property(conn: &Connection, type_id: i64, name: &str, data_type: &str) -> i64 {
    conn.execute(
        "INSERT INTO group_type_properties (type_id, name, data_type, created_at_utc, updated_at_utc)
         VALUES (?1, ?2, ?3, ?4, ?4)",
        params![type_id, name, data_type, NOW],
    )
    .expect("insert property");
    conn.last_insert_rowid()
}

fn insert_printer(conn: &Connection, device_id: i64, usb_host: Option<i64>) {
    conn.execute(
        "INSERT INTO printers (device_id, usb_host_device_id, created_at_utc, updated_at_utc)
         VALUES (?1, ?2, ?3, ?3)",
        params![device_id, usb_host, NOW],
    )
    .expect("insert printer");
}

fn text_value(group: i64, property: i64, position: i64, text: &str) -> GroupValueRow {
    GroupValueRow {
        group_id: group,
        property_id: property,
        position,
        value_text: Some(text.to_string()),
        value_ref: None,
        is_primary: false,
    }
}

fn ref_value(group: i64, property: i64, position: i64, r: i64, primary: bool) -> GroupValueRow {
    GroupValueRow {
        group_id: group,
        property_id: property,
        position,
        value_text: None,
        value_ref: Some(r),
        is_primary: primary,
    }
}

#[test]
fn replace_values_swaps_set_and_empty_clears() {
    let (mut conn, _g) = test_db();
    let repo = SqliteGroupRepository;
    let t = insert_type(&conn, "workstation", "АРМ", "container");
    let p_text = insert_property(&conn, t, "Примечание", "text");
    let p_other = insert_property(&conn, t, "Кабинет", "text");
    let g = make_group(&mut conn, &repo, t, "АРМ #1", None);

    let tx = conn.transaction().unwrap();
    repo.replace_property_values_in_tx(
        &tx,
        g.id,
        p_text,
        &[text_value(g.id, p_text, 0, "старое")],
        NOW,
    )
    .unwrap();
    repo.replace_property_values_in_tx(
        &tx,
        g.id,
        p_other,
        &[text_value(g.id, p_other, 0, "101")],
        NOW,
    )
    .unwrap();
    tx.commit().unwrap();

    let tx = conn.transaction().unwrap();
    repo.replace_property_values_in_tx(
        &tx,
        g.id,
        p_text,
        &[text_value(g.id, p_text, 0, "новое")],
        NOW + 1,
    )
    .unwrap();
    tx.commit().unwrap();
    let vals = repo.list_values(&conn, g.id).unwrap();
    assert_eq!(vals.len(), 2, "чужое свойство не затронуто");
    let text = vals.iter().find(|v| v.property_id == p_text).unwrap();
    assert_eq!(text.value_text.as_deref(), Some("новое"));
    assert_eq!(
        vals.iter()
            .find(|v| v.property_id == p_other)
            .unwrap()
            .value_text
            .as_deref(),
        Some("101")
    );

    // Пустой набор -> строки нет.
    let tx = conn.transaction().unwrap();
    repo.replace_property_values_in_tx(&tx, g.id, p_text, &[], NOW + 2)
        .unwrap();
    tx.commit().unwrap();
    let vals = repo.list_values(&conn, g.id).unwrap();
    assert_eq!(vals.len(), 1);
    assert_eq!(vals[0].property_id, p_other);
}

#[test]
fn list_values_orders_by_property_then_position() {
    let (mut conn, _g) = test_db();
    let repo = SqliteGroupRepository;
    let t = insert_type(&conn, "workstation", "АРМ", "container");
    let p1 = insert_property(&conn, t, "Пользователи", "users");
    let p2 = insert_property(&conn, t, "Устройства", "device_refs");
    let g = make_group(&mut conn, &repo, t, "АРМ #1", None);
    insert_device(&conn, 1, "А");
    insert_device(&conn, 2, "Б");
    insert_device(&conn, 3, "В");

    let tx = conn.transaction().unwrap();
    // Вставляем в «неправильном» порядке: сначала p2, позиции вразнобой.
    repo.replace_property_values_in_tx(
        &tx,
        g.id,
        p2,
        &[
            ref_value(g.id, p2, 1, 3, false),
            ref_value(g.id, p2, 0, 2, true),
        ],
        NOW,
    )
    .unwrap();
    repo.replace_property_values_in_tx(&tx, g.id, p1, &[ref_value(g.id, p1, 0, 1, true)], NOW)
        .unwrap();
    tx.commit().unwrap();

    let got: Vec<(i64, i64, Option<i64>)> = repo
        .list_values(&conn, g.id)
        .unwrap()
        .iter()
        .map(|v| (v.property_id, v.position, v.value_ref))
        .collect();
    assert_eq!(
        got,
        vec![(p1, 0, Some(1)), (p2, 0, Some(2)), (p2, 1, Some(3))]
    );
}

#[test]
fn two_primary_values_conflict_and_old_values_survive() {
    let (mut conn, _g) = test_db();
    let repo = SqliteGroupRepository;
    let t = insert_type(&conn, "workstation", "АРМ", "container");
    let p = insert_property(&conn, t, "Пользователи", "users");
    let g = make_group(&mut conn, &repo, t, "АРМ #1", None);
    insert_device(&conn, 1, "А");
    insert_device(&conn, 2, "Б");
    insert_device(&conn, 3, "В");

    let tx = conn.transaction().unwrap();
    repo.replace_property_values_in_tx(&tx, g.id, p, &[ref_value(g.id, p, 0, 1, true)], NOW)
        .unwrap();
    tx.commit().unwrap();

    let tx = conn.transaction().unwrap();
    let err = repo
        .replace_property_values_in_tx(
            &tx,
            g.id,
            p,
            &[
                // Первые две вставки проходят, третья нарушает uq_gpv_primary: без отката
                // к savepoint в БД остались бы две строки вместо одной прежней.
                ref_value(g.id, p, 0, 2, false),
                ref_value(g.id, p, 1, 1, true),
                ref_value(g.id, p, 2, 3, true),
            ],
            NOW + 1,
        )
        .unwrap_err();
    assert!(matches!(err, AppError::Conflict { .. }), "{err:?}");
    // Savepoint откатил частичную замену: прежнее значение на месте, транзакция жива.
    let vals = repo.list_values(&tx, g.id).unwrap();
    assert_eq!(vals.len(), 1, "частичная замена откатана");
    assert_eq!(vals[0].value_ref, Some(1));
    assert!(vals[0].is_primary);
    tx.commit().unwrap();
}

#[test]
fn values_cascade_when_group_deleted() {
    let (mut conn, _g) = test_db();
    let repo = SqliteGroupRepository;
    let t = insert_type(&conn, "workstation", "АРМ", "container");
    let p = insert_property(&conn, t, "Примечание", "text");
    let g1 = make_group(&mut conn, &repo, t, "АРМ #1", None);
    let g2 = make_group(&mut conn, &repo, t, "АРМ #2", None);
    let tx = conn.transaction().unwrap();
    repo.replace_property_values_in_tx(&tx, g1.id, p, &[text_value(g1.id, p, 0, "а")], NOW)
        .unwrap();
    repo.replace_property_values_in_tx(&tx, g2.id, p, &[text_value(g2.id, p, 0, "б")], NOW)
        .unwrap();
    repo.delete_group_in_tx(&tx, g1.id).unwrap();
    tx.commit().unwrap();
    assert!(repo.list_values(&conn, g1.id).unwrap().is_empty());
    assert_eq!(repo.list_values(&conn, g2.id).unwrap().len(), 1);
}

fn ids(rows: &[trackly_core::domain::groups::PrinterRefRow]) -> Vec<i64> {
    rows.iter().map(|r| r.device_id).collect()
}

#[test]
fn usb_printers_come_from_composition_including_nested_group() {
    let (mut conn, _g) = test_db();
    let repo = SqliteGroupRepository;
    let t = insert_type(&conn, "workstation", "АРМ", "container");
    let root = make_group(&mut conn, &repo, t, "Корень", None);
    let nested = make_group(&mut conn, &repo, t, "Вложенная", None);
    let stranger = make_group(&mut conn, &repo, t, "Чужая", None);
    set_parent(&mut conn, &repo, nested.id, Some(root.id)).unwrap();
    // Хосты: 1 — в корне, 2 — во вложенной, 3 — в чужой группе, 4 — вне групп.
    for (id, name) in [
        (1, "Хост корня"),
        (2, "Хост вложенной"),
        (3, "Хост чужой"),
        (4, "Хост без группы"),
    ] {
        insert_device(&conn, id, name);
    }
    add_device(&mut conn, &repo, root.id, 1);
    add_device(&mut conn, &repo, nested.id, 2);
    add_device(&mut conn, &repo, stranger.id, 3);
    // Принтеры: 11 -> хост 1 (корень), 12 -> хост 2 (вложенная), 13 -> хост 3 (чужая),
    // 14 -> хост 4 (вне групп), 15 — без USB-хоста (сетевой).
    for (id, name, host) in [
        (11, "Принтер А", Some(1)),
        (12, "Принтер Б", Some(2)),
        (13, "Принтер В", Some(3)),
        (14, "Принтер Г", Some(4)),
        (15, "Принтер Д", None),
    ] {
        insert_device(&conn, id, name);
        insert_printer(&conn, id, host);
    }

    let found = repo.usb_printers_for_group(&conn, root.id).unwrap();
    assert_eq!(
        ids(&found),
        vec![11, 12],
        "корень + вложенная, без чужой группы"
    );
    assert_eq!(found[0].name, "Принтер А");
    assert_eq!(found[0].inventory_number.as_deref(), Some("ИНВ-000011"));
    assert_eq!(
        ids(&repo.usb_printers_for_group(&conn, nested.id).unwrap()),
        vec![12]
    );
    assert_eq!(
        ids(&repo.usb_printers_for_group(&conn, stranger.id).unwrap()),
        vec![13]
    );

    // Мягко удалённый принтер исключается; мягко удалённый хост выпадает из состава.
    soft_delete_device(&conn, 11);
    assert_eq!(
        ids(&repo.usb_printers_for_group(&conn, root.id).unwrap()),
        vec![12]
    );
    soft_delete_device(&conn, 2);
    assert!(repo
        .usb_printers_for_group(&conn, root.id)
        .unwrap()
        .is_empty());
}

#[test]
fn ref_devices_follow_device_refs_and_overlap_with_usb() {
    let (mut conn, _g) = test_db();
    let repo = SqliteGroupRepository;
    let t = insert_type(&conn, "workstation", "АРМ", "container");
    let p_refs = insert_property(&conn, t, "Принтеры", "device_refs");
    let p_users = insert_property(&conn, t, "Пользователи", "users");
    let g = make_group(&mut conn, &repo, t, "АРМ #1", None);
    for (id, name) in [
        (1, "Хост"),
        (11, "Принтер А"),
        (12, "Принтер Б"),
        (13, "Принтер В"),
    ] {
        insert_device(&conn, id, name);
    }
    add_device(&mut conn, &repo, g.id, 1);
    insert_printer(&conn, 11, Some(1)); // USB-привязан
    insert_printer(&conn, 12, None);

    let tx = conn.transaction().unwrap();
    // 11 — и USB, и явная ссылка; 12 — только ссылка; 13 — ссылка, затем мягко удалён.
    repo.replace_property_values_in_tx(
        &tx,
        g.id,
        p_refs,
        &[
            ref_value(g.id, p_refs, 0, 11, true),
            ref_value(g.id, p_refs, 1, 12, false),
            ref_value(g.id, p_refs, 2, 13, false),
        ],
        NOW,
    )
    .unwrap();
    // Ссылка свойства, которое вызывающий не запрашивает, в ответ не попадает.
    repo.replace_property_values_in_tx(
        &tx,
        g.id,
        p_users,
        &[ref_value(g.id, p_users, 0, 1, true)],
        NOW,
    )
    .unwrap();
    tx.commit().unwrap();
    soft_delete_device(&conn, 13);

    let refs = repo.ref_devices_for_group(&conn, g.id, &[p_refs]).unwrap();
    assert_eq!(ids(&refs), vec![11, 12]);
    assert!(repo
        .ref_devices_for_group(&conn, g.id, &[])
        .unwrap()
        .is_empty());
    assert_eq!(
        ids(&repo
            .ref_devices_for_group(&conn, g.id, &[p_refs, p_users])
            .unwrap()),
        vec![11, 12, 1],
        "несколько свойств, порядок по имени («Хост» после принтеров)"
    );

    // Принтер 11 виден обоими источниками — дедупликация на стороне сервиса (план 12).
    let usb = repo.usb_printers_for_group(&conn, g.id).unwrap();
    assert_eq!(ids(&usb), vec![11]);
    assert!(ids(&refs).contains(&11));
}

#[test]
fn tree_counts_include_nested_and_group_counts_by_type() {
    let (mut conn, _g) = test_db();
    let repo = SqliteGroupRepository;
    let t1 = insert_type(&conn, "rack", "Стойка", "container");
    let t2 = insert_type(&conn, "workstation", "АРМ", "container");
    let root = make_group(&mut conn, &repo, t1, "Корень", None);
    let nested = make_group(&mut conn, &repo, t2, "Вложенная", None);
    let deep = make_group(&mut conn, &repo, t2, "Глубокая", None);
    let empty = make_group(&mut conn, &repo, t2, "Пустая", None);
    set_parent(&mut conn, &repo, nested.id, Some(root.id)).unwrap();
    set_parent(&mut conn, &repo, deep.id, Some(nested.id)).unwrap();
    for id in 1..=7 {
        insert_device(&conn, id, &format!("Устройство {id}"));
    }
    for id in 1..=2 {
        add_device(&mut conn, &repo, root.id, id);
    }
    for id in 3..=5 {
        add_device(&mut conn, &repo, nested.id, id);
    }
    add_device(&mut conn, &repo, deep.id, 6);
    add_device(&mut conn, &repo, deep.id, 7);
    soft_delete_device(&conn, 7);

    let counts = repo.tree_counts(&conn).unwrap();
    let get = |id: i64| counts.iter().find(|(g, _)| *g == id).map(|(_, c)| *c);
    assert_eq!(get(deep.id), Some(1), "7 мягко удалено");
    assert_eq!(get(nested.id), Some(4), "3 свои + 1 глубокая");
    assert_eq!(get(root.id), Some(6), "2 свои + 4 вложенных");
    assert_eq!(get(empty.id), Some(0), "пустая группа присутствует с нулём");
    assert_eq!(counts.len(), 4, "по строке на группу, без дублей");

    assert_eq!(
        repo.group_counts_by_type(&conn).unwrap(),
        vec![(t1, 1), (t2, 3)]
    );
}
