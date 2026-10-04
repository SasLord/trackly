//! Integration tests: `SqliteGroupTypeRepository` на реальной БД (Phase 41 Plan 04,
//! GRP-01/02/03). Вымышленные имена и данные.

use rusqlite::{params, Connection};
use trackly_core::domain::groups::{GroupBehavior, GroupTypeNew, GroupTypePatch};
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
