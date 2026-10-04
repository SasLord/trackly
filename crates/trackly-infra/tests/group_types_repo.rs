//! Integration tests: `SqliteGroupTypeRepository` на реальной БД (Phase 41 Plan 04,
//! GRP-01/02/03). Вымышленные имена и данные.

use rusqlite::{params, Connection};
use trackly_core::domain::groups::{
    GroupBehavior, GroupTypeNew, GroupTypePatch, PropertyDataType, PropertyNew, PropertyPatch,
};
use trackly_core::error::AppError;
use trackly_core::ports::group_types::GroupTypeRepository;
use trackly_infra::repos::SqliteGroupTypeRepository;
use trackly_infra::test_support::test_db;

const NOW: i64 = 1_700_000_000;

fn new_type(code: &str, name: &str, behavior: GroupBehavior) -> GroupTypeNew {
    GroupTypeNew {
        code: code.to_string(),
        name: name.to_string(),
        behavior,
        is_builtin: false,
        sort_order: 0,
        quick_action_enabled: false,
        quick_action_label: None,
    }
}

fn builtin_seed() -> Vec<GroupTypeNew> {
    vec![
        GroupTypeNew {
            code: "workstation".into(),
            name: "АРМ".into(),
            behavior: GroupBehavior::Container,
            is_builtin: true,
            sort_order: 10,
            quick_action_enabled: true,
            quick_action_label: Some("Сформировать группу".into()),
        },
        GroupTypeNew {
            code: "system_unit".into(),
            name: "Системный блок".into(),
            behavior: GroupBehavior::Substitute,
            is_builtin: true,
            sort_order: 20,
            quick_action_enabled: true,
            quick_action_label: Some("Замещение группой".into()),
        },
    ]
}

fn type_name(conn: &Connection, code: &str) -> String {
    conn.query_row(
        "SELECT name FROM group_types WHERE code = ?1",
        params![code],
        |r| r.get(0),
    )
    .unwrap()
}

#[test]
fn create_type_sets_version_one_and_duplicate_code_conflicts() {
    let (mut conn, _g) = test_db();
    let repo = SqliteGroupTypeRepository;
    let id = repo
        .create_type(
            &mut conn,
            &new_type("rack", "Стойка", GroupBehavior::Container),
            NOW,
        )
        .unwrap();
    let row = repo.get_type(&conn, id).unwrap();
    assert_eq!(row.version, 1);
    assert_eq!(row.code, "rack");
    assert_eq!(row.behavior, "container");
    assert_eq!(row.created_at_utc, NOW);

    let dup = repo.create_type(
        &mut conn,
        &new_type("rack", "Другая стойка", GroupBehavior::Container),
        NOW,
    );
    assert!(matches!(dup, Err(AppError::Conflict { .. })), "{dup:?}");
    assert_eq!(
        repo.get_type_by_code(&conn, "rack").unwrap().unwrap().id,
        id
    );
    assert!(repo.get_type_by_code(&conn, "nope").unwrap().is_none());
}

#[test]
fn update_type_cas_not_found_and_version_increment() {
    let (mut conn, _g) = test_db();
    let repo = SqliteGroupTypeRepository;
    let id = repo
        .create_type(
            &mut conn,
            &new_type("rack", "Стойка", GroupBehavior::Container),
            NOW,
        )
        .unwrap();

    let patch = GroupTypePatch {
        name: Some("Шкаф".into()),
        sort_order: Some(7),
        quick_action_enabled: Some(true),
        quick_action_label: Some(Some("Собрать".into())),
    };
    let upd = repo.update_type(&mut conn, id, 1, &patch, NOW + 5).unwrap();
    assert_eq!(upd.name, "Шкаф");
    assert_eq!(upd.sort_order, 7);
    assert!(upd.quick_action_enabled);
    assert_eq!(upd.quick_action_label.as_deref(), Some("Собрать"));
    assert_eq!(upd.version, 2);
    assert_eq!(upd.updated_at_utc, NOW + 5);
    // code/behavior не тронуты
    assert_eq!(upd.code, "rack");
    assert_eq!(upd.behavior, "container");

    // Чужая версия.
    let stale = repo.update_type(&mut conn, id, 1, &GroupTypePatch::default(), NOW);
    match stale {
        Err(AppError::OptimisticLockMismatch {
            expected, actual, ..
        }) => {
            assert_eq!((expected, actual), (1, 2));
        }
        other => panic!("ожидали OptimisticLockMismatch, получили {other:?}"),
    }
    // Несуществующий id.
    let missing = repo.update_type(&mut conn, 9_999, 1, &GroupTypePatch::default(), NOW);
    assert!(
        matches!(missing, Err(AppError::NotFound { .. })),
        "{missing:?}"
    );

    // Явный сброс подписи (Some(None)) отличим от «не передано» (None).
    let keep = repo
        .update_type(&mut conn, id, 2, &GroupTypePatch::default(), NOW)
        .unwrap();
    assert_eq!(keep.quick_action_label.as_deref(), Some("Собрать"));
    let cleared = repo
        .update_type(
            &mut conn,
            id,
            3,
            &GroupTypePatch {
                quick_action_label: Some(None),
                ..Default::default()
            },
            NOW,
        )
        .unwrap();
    assert_eq!(cleared.quick_action_label, None);
}

#[test]
fn trigger_rejects_direct_code_and_behavior_update() {
    let (mut conn, _g) = test_db();
    let repo = SqliteGroupTypeRepository;
    let id = repo
        .create_type(
            &mut conn,
            &new_type("rack", "Стойка", GroupBehavior::Container),
            NOW,
        )
        .unwrap();

    let err = conn
        .execute(
            "UPDATE group_types SET code = 'x' WHERE id = ?1",
            params![id],
        )
        .unwrap_err();
    let mapped = trackly_infra::error_conversions::map_rusqlite(err);
    assert!(matches!(mapped, AppError::Conflict { .. }), "{mapped:?}");
    let code: String = conn
        .query_row(
            "SELECT code FROM group_types WHERE id = ?1",
            params![id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(code, "rack", "строка не должна измениться");

    let err = conn
        .execute(
            "UPDATE group_types SET behavior = 'teardown' WHERE id = ?1",
            params![id],
        )
        .unwrap_err();
    let mapped = trackly_infra::error_conversions::map_rusqlite(err);
    assert!(matches!(mapped, AppError::Conflict { .. }), "{mapped:?}");
    let behavior: String = conn
        .query_row(
            "SELECT behavior FROM group_types WHERE id = ?1",
            params![id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(behavior, "container");

    // Обновление через репозиторий триггер не задевает.
    repo.update_type(
        &mut conn,
        id,
        1,
        &GroupTypePatch {
            name: Some("Шкаф".into()),
            ..Default::default()
        },
        NOW,
    )
    .unwrap();
}

#[test]
fn delete_type_and_count_groups() {
    let (mut conn, _g) = test_db();
    let repo = SqliteGroupTypeRepository;
    let id = repo
        .create_type(
            &mut conn,
            &new_type("rack", "Стойка", GroupBehavior::Container),
            NOW,
        )
        .unwrap();
    assert_eq!(repo.count_groups_of_type(&conn, id).unwrap(), 0);

    conn.execute(
        "INSERT INTO groups (type_id, name, seq, created_at_utc, updated_at_utc)
         VALUES (?1, 'Стойка #1', 1, ?2, ?2), (?1, 'Стойка #2', 2, ?2, ?2)",
        params![id, NOW],
    )
    .unwrap();
    assert_eq!(repo.count_groups_of_type(&conn, id).unwrap(), 2);

    // FK RESTRICT: тип с группами удалить нельзя.
    let blocked = repo.delete_type(&mut conn, id);
    assert!(
        matches!(blocked, Err(AppError::Conflict { .. })),
        "{blocked:?}"
    );
    assert!(repo.get_type(&conn, id).is_ok());

    conn.execute("DELETE FROM groups WHERE type_id = ?1", params![id])
        .unwrap();
    repo.delete_type(&mut conn, id).unwrap();
    assert!(matches!(
        repo.get_type(&conn, id),
        Err(AppError::NotFound { .. })
    ));
    assert!(matches!(
        repo.delete_type(&mut conn, id),
        Err(AppError::NotFound { .. })
    ));
}

#[test]
fn list_types_ordered_by_sort_order_then_id() {
    let (mut conn, _g) = test_db();
    let repo = SqliteGroupTypeRepository;
    let mut b = new_type("b", "Б", GroupBehavior::Container);
    b.sort_order = 2;
    let mut a = new_type("a", "А", GroupBehavior::Container);
    a.sort_order = 1;
    let mut c = new_type("c", "В", GroupBehavior::Container);
    c.sort_order = 2;
    repo.create_type(&mut conn, &b, NOW).unwrap();
    repo.create_type(&mut conn, &a, NOW).unwrap();
    repo.create_type(&mut conn, &c, NOW).unwrap();
    let codes: Vec<String> = repo
        .list_types(&conn)
        .unwrap()
        .into_iter()
        .map(|t| t.code)
        .collect();
    assert_eq!(codes, vec!["a", "b", "c"]);
}

#[test]
fn seed_inserts_missing_and_is_idempotent() {
    let (mut conn, _g) = test_db();
    let repo = SqliteGroupTypeRepository;
    let seed = builtin_seed();

    let tx = conn.transaction().unwrap();
    assert_eq!(repo.seed_builtin_types_in_tx(&tx, &seed, NOW).unwrap(), 2);
    tx.commit().unwrap();

    let before = repo.list_types(&conn).unwrap();
    assert_eq!(before.len(), 2);

    let tx = conn.transaction().unwrap();
    assert_eq!(
        repo.seed_builtin_types_in_tx(&tx, &seed, NOW + 100)
            .unwrap(),
        0
    );
    tx.commit().unwrap();
    assert_eq!(repo.list_types(&conn).unwrap(), before, "строки неизменны");
}

#[test]
fn seed_does_not_overwrite_admin_rename() {
    let (mut conn, _g) = test_db();
    let repo = SqliteGroupTypeRepository;
    let seed = builtin_seed();

    let tx = conn.transaction().unwrap();
    repo.seed_builtin_types_in_tx(&tx, &seed, NOW).unwrap();
    tx.commit().unwrap();
    let original = repo
        .get_type_by_code(&conn, "workstation")
        .unwrap()
        .unwrap();
    assert_eq!(original.name, "АРМ");

    // Администратор переименовывает в имя, отличное от сидового.
    repo.update_type(
        &mut conn,
        original.id,
        original.version,
        &GroupTypePatch {
            name: Some("Рабочее место".into()),
            ..Default::default()
        },
        NOW + 10,
    )
    .unwrap();
    assert_eq!(type_name(&conn, "workstation"), "Рабочее место");

    let tx = conn.transaction().unwrap();
    assert_eq!(
        repo.seed_builtin_types_in_tx(&tx, &seed, NOW + 20).unwrap(),
        0
    );
    tx.commit().unwrap();

    let after = repo
        .get_type_by_code(&conn, "workstation")
        .unwrap()
        .unwrap();
    assert_eq!(after.name, "Рабочее место", "имя пережило повторный засев");
    assert_eq!(after.id, original.id, "id не меняется");
    let n: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM group_types WHERE code = 'workstation'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(n, 1);
}

// ---- свойства -------------------------------------------------------------

fn make_type(conn: &mut Connection, code: &str) -> i64 {
    SqliteGroupTypeRepository
        .create_type(conn, &new_type(code, code, GroupBehavior::Container), NOW)
        .unwrap()
}

fn prop(type_id: i64, name: &str, data_type: PropertyDataType) -> PropertyNew {
    PropertyNew {
        type_id,
        name: name.to_string(),
        data_type,
        is_required: false,
        show_on_map: false,
    }
}

fn make_prop(conn: &mut Connection, type_id: i64, name: &str) -> i64 {
    SqliteGroupTypeRepository
        .create_property(conn, &prop(type_id, name, PropertyDataType::Text), NOW)
        .unwrap()
}

fn make_group(conn: &Connection, type_id: i64, seq: i64, name: &str) -> i64 {
    conn.execute(
        "INSERT INTO groups (type_id, name, seq, created_at_utc, updated_at_utc)
         VALUES (?1, ?2, ?3, ?4, ?4)",
        params![type_id, name, seq, NOW],
    )
    .unwrap();
    conn.last_insert_rowid()
}

fn sort_orders(conn: &Connection, type_id: i64) -> Vec<(i64, i64)> {
    let mut stmt = conn
        .prepare(
            "SELECT id, sort_order FROM group_type_properties
             WHERE type_id = ?1 ORDER BY id",
        )
        .unwrap();
    stmt.query_map(params![type_id], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap()
}

#[test]
fn create_property_appends_after_hidden_and_duplicate_name_conflicts() {
    let (mut conn, _g) = test_db();
    let repo = SqliteGroupTypeRepository;
    let t = make_type(&mut conn, "rack");
    let p0 = make_prop(&mut conn, t, "Хост");
    let p1 = make_prop(&mut conn, t, "IP");
    assert_eq!(repo.get_property(&conn, p0).unwrap().sort_order, 0);
    assert_eq!(repo.get_property(&conn, p1).unwrap().sort_order, 1);

    // Скрытое свойство хранит место: новое идёт после него.
    repo.archive_property(&mut conn, p1, NOW + 1).unwrap();
    let p2 = make_prop(&mut conn, t, "MAC");
    let row = repo.get_property(&conn, p2).unwrap();
    assert_eq!(row.sort_order, 2);
    assert_eq!(row.version, 1);
    assert_eq!(row.data_type, "text");

    // Живой дубликат имени.
    let dup = repo.create_property(&mut conn, &prop(t, "Хост", PropertyDataType::Ip), NOW);
    assert!(matches!(dup, Err(AppError::Conflict { .. })), "{dup:?}");

    // То же имя в другом типе — можно.
    let t2 = make_type(&mut conn, "desk");
    repo.create_property(&mut conn, &prop(t2, "Хост", PropertyDataType::Text), NOW)
        .unwrap();

    // Скрытое имя освобождается, а возврат скрытого при занятом имени — Conflict.
    let again = repo
        .create_property(&mut conn, &prop(t, "IP", PropertyDataType::Ip), NOW)
        .unwrap();
    assert!(again > p2);
    let blocked = repo.unarchive_property(&mut conn, p1, NOW + 2);
    assert!(
        matches!(blocked, Err(AppError::Conflict { .. })),
        "{blocked:?}"
    );
    assert!(repo
        .get_property(&conn, p1)
        .unwrap()
        .archived_at_utc
        .is_some());
}

#[test]
fn list_properties_hides_archived_unless_requested() {
    let (mut conn, _g) = test_db();
    let repo = SqliteGroupTypeRepository;
    let t = make_type(&mut conn, "rack");
    let a = make_prop(&mut conn, t, "А");
    let b = make_prop(&mut conn, t, "Б");
    let c = make_prop(&mut conn, t, "В");
    repo.archive_property(&mut conn, b, NOW + 1).unwrap();

    let live: Vec<i64> = repo
        .list_properties(&conn, t, false)
        .unwrap()
        .iter()
        .map(|p| p.id)
        .collect();
    assert_eq!(live, vec![a, c]);
    let all: Vec<i64> = repo
        .list_properties(&conn, t, true)
        .unwrap()
        .iter()
        .map(|p| p.id)
        .collect();
    assert_eq!(all, vec![a, b, c]);

    // Другой тип не подмешивается.
    let t2 = make_type(&mut conn, "desk");
    make_prop(&mut conn, t2, "Г");
    assert_eq!(repo.list_properties(&conn, t, true).unwrap().len(), 3);

    // Возврат из скрытия.
    repo.unarchive_property(&mut conn, b, NOW + 2).unwrap();
    assert!(repo
        .get_property(&conn, b)
        .unwrap()
        .archived_at_utc
        .is_none());
    assert_eq!(repo.list_properties(&conn, t, false).unwrap().len(), 3);
}

#[test]
fn update_property_cas_and_fields() {
    let (mut conn, _g) = test_db();
    let repo = SqliteGroupTypeRepository;
    let t = make_type(&mut conn, "rack");
    let p = make_prop(&mut conn, t, "Хост");

    let upd = repo
        .update_property(
            &mut conn,
            p,
            1,
            &PropertyPatch {
                name: Some("Имя хоста".into()),
                data_type: Some(PropertyDataType::Ip),
                is_required: Some(true),
                show_on_map: Some(true),
            },
            NOW + 9,
        )
        .unwrap();
    assert_eq!(upd.name, "Имя хоста");
    assert_eq!(upd.data_type, "ip");
    assert!(upd.is_required && upd.show_on_map);
    assert_eq!(upd.version, 2);
    assert_eq!(upd.updated_at_utc, NOW + 9);

    // Пустой патч ничего не меняет, кроме версии.
    let same = repo
        .update_property(&mut conn, p, 2, &PropertyPatch::default(), NOW)
        .unwrap();
    assert_eq!(same.name, "Имя хоста");
    assert!(same.is_required);

    let stale = repo.update_property(&mut conn, p, 1, &PropertyPatch::default(), NOW);
    assert!(
        matches!(
            stale,
            Err(AppError::OptimisticLockMismatch {
                expected: 1,
                actual: 3,
                ..
            })
        ),
        "{stale:?}"
    );
    let missing = repo.update_property(&mut conn, 9_999, 1, &PropertyPatch::default(), NOW);
    assert!(
        matches!(missing, Err(AppError::NotFound { .. })),
        "{missing:?}"
    );

    // Переименование в занятое живое имя.
    make_prop(&mut conn, t, "Модель");
    let clash = repo.update_property(
        &mut conn,
        p,
        3,
        &PropertyPatch {
            name: Some("Модель".into()),
            ..Default::default()
        },
        NOW,
    );
    assert!(matches!(clash, Err(AppError::Conflict { .. })), "{clash:?}");
}

#[test]
fn delete_property_hard_cascades_values() {
    let (mut conn, _g) = test_db();
    let repo = SqliteGroupTypeRepository;
    let t = make_type(&mut conn, "rack");
    let p = make_prop(&mut conn, t, "Хост");
    let g = make_group(&conn, t, 1, "Стойка #1");
    conn.execute(
        "INSERT INTO group_property_values (group_id, property_id, value_text, updated_at_utc)
         VALUES (?1, ?2, 'host-a', ?3)",
        params![g, p, NOW],
    )
    .unwrap();

    repo.delete_property_hard(&mut conn, p).unwrap();
    let left: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM group_property_values WHERE property_id = ?1",
            params![p],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(left, 0, "значения удаляются каскадом");
    assert!(matches!(
        repo.get_property(&conn, p),
        Err(AppError::NotFound { .. })
    ));
    assert!(matches!(
        repo.delete_property_hard(&mut conn, p),
        Err(AppError::NotFound { .. })
    ));
}

#[test]
fn filled_group_count_counts_distinct_groups_not_rows() {
    let (mut conn, _g) = test_db();
    let repo = SqliteGroupTypeRepository;
    let t = make_type(&mut conn, "rack");
    let p = make_prop(&mut conn, t, "Принтеры");
    assert_eq!(repo.filled_group_count(&conn, p).unwrap(), 0);

    let g1 = make_group(&conn, t, 1, "АРМ #1");
    let g2 = make_group(&conn, t, 2, "АРМ #2");
    let _g3 = make_group(&conn, t, 3, "АРМ #3");
    // Три строки значения у двух групп (две ссылки у g1): ответ 2, не 3.
    for (g, r, primary) in [(g1, 11, 1), (g1, 12, 0), (g2, 11, 1)] {
        conn.execute(
            "INSERT INTO group_property_values
               (group_id, property_id, value_ref, is_primary, updated_at_utc)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![g, p, r, primary, NOW],
        )
        .unwrap();
    }
    let rows: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM group_property_values WHERE property_id = ?1",
            params![p],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(rows, 3);
    assert_eq!(repo.filled_group_count(&conn, p).unwrap(), 2);
}

#[test]
fn groups_missing_required_finds_only_groups_without_value() {
    let (mut conn, _g) = test_db();
    let repo = SqliteGroupTypeRepository;
    let t = make_type(&mut conn, "rack");
    let other_t = make_type(&mut conn, "desk");
    let p_text = make_prop(&mut conn, t, "Хост");
    let p_ref = make_prop(&mut conn, t, "Пользователи");

    let missing = make_group(&conn, t, 3, "АРМ #3");
    let with_text = make_group(&conn, t, 1, "АРМ #1");
    let with_ref = make_group(&conn, t, 2, "АРМ #2");
    let _foreign = make_group(&conn, other_t, 1, "Стол #1");
    conn.execute(
        "INSERT INTO group_property_values (group_id, property_id, value_text, updated_at_utc)
         VALUES (?1, ?2, 'host-a', ?3)",
        params![with_text, p_text, NOW],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO group_property_values
           (group_id, property_id, value_ref, is_primary, updated_at_utc)
         VALUES (?1, ?2, 42, 1, ?3)",
        params![with_ref, p_ref, NOW],
    )
    .unwrap();

    // Текстовое свойство: нарушители — группы без текста (#2 и #3), по seq.
    let v = repo.groups_missing_required(&conn, t, p_text).unwrap();
    assert_eq!(
        v,
        vec![
            (with_ref, "АРМ #2".to_string()),
            (missing, "АРМ #3".to_string())
        ]
    );
    // Свойство-ссылка: с value_ref группа не нарушитель, с текстом по другому свойству — нарушитель.
    let v = repo.groups_missing_required(&conn, t, p_ref).unwrap();
    assert_eq!(
        v,
        vec![
            (with_text, "АРМ #1".to_string()),
            (missing, "АРМ #3".to_string())
        ]
    );
}

#[test]
fn reorder_properties_assigns_positions_atomically() {
    let (mut conn, _g) = test_db();
    let repo = SqliteGroupTypeRepository;
    let t = make_type(&mut conn, "rack");
    let other_t = make_type(&mut conn, "desk");
    let p1 = make_prop(&mut conn, t, "А");
    let p2 = make_prop(&mut conn, t, "Б");
    let p3 = make_prop(&mut conn, t, "В");
    let foreign = make_prop(&mut conn, other_t, "Г");
    let before = sort_orders(&conn, t);
    assert_eq!(before, vec![(p1, 0), (p2, 1), (p3, 2)]);
    let foreign_before = sort_orders(&conn, other_t);

    // Чужой id: отказ, порядок прежний.
    let bad = repo.reorder_properties(&mut conn, t, &[p3, p1, foreign], NOW);
    assert!(
        matches!(&bad, Err(AppError::Validation { field, .. }) if field == "ordered_ids"),
        "{bad:?}"
    );
    assert_eq!(sort_orders(&conn, t), before);
    assert_eq!(sort_orders(&conn, other_t), foreign_before);

    // Неполный список и дубликат.
    assert!(matches!(
        repo.reorder_properties(&mut conn, t, &[p3, p1], NOW),
        Err(AppError::Validation { .. })
    ));
    assert!(matches!(
        repo.reorder_properties(&mut conn, t, &[p1, p1, p2], NOW),
        Err(AppError::Validation { .. })
    ));
    assert_eq!(sort_orders(&conn, t), before);

    // Верный порядок.
    repo.reorder_properties(&mut conn, t, &[p3, p1, p2], NOW + 3)
        .unwrap();
    assert_eq!(sort_orders(&conn, t), vec![(p1, 1), (p2, 2), (p3, 0)]);
    let listed: Vec<i64> = repo
        .list_properties(&conn, t, false)
        .unwrap()
        .iter()
        .map(|p| p.id)
        .collect();
    assert_eq!(listed, vec![p3, p1, p2]);

    // Скрытое свойство в сортировке не участвует.
    repo.archive_property(&mut conn, p2, NOW + 4).unwrap();
    repo.reorder_properties(&mut conn, t, &[p1, p3], NOW + 5)
        .unwrap();
    let listed: Vec<i64> = repo
        .list_properties(&conn, t, false)
        .unwrap()
        .iter()
        .map(|p| p.id)
        .collect();
    assert_eq!(listed, vec![p1, p3]);
}
