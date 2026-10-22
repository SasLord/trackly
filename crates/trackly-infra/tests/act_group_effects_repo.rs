//! Integration test: методы `SqliteGroupRepository` для `act_group_effects`
//! (V049, Phase 41.2 Plan 02, D-08 / D-09) и читатели снимка групп для
//! планировщика актов. БД поднята настоящим раннером миграций.
//! Только вымышленные данные.

use rusqlite::{params, Connection, TransactionBehavior};
use tempfile::TempDir;

use trackly_core::domain::groups::{ActGroupEffectKind, ActGroupEffectNew};
use trackly_infra::db::migrations;
use trackly_infra::db::pragmas::apply_writer_pragmas;
use trackly_infra::repos::SqliteGroupRepository;

const NOW: i64 = 1_700_000_000;

fn fresh_db() -> (Connection, TempDir) {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("act-group-effects-repo.db");
    let mut conn = Connection::open(&path).expect("open");
    apply_writer_pragmas(&conn).expect("writer pragmas");
    migrations::run(&mut conn).expect("run migrations");
    (conn, dir)
}

/// Тип группы, две живые группы (id 1 и 2), мягко удалённая группа (id 3),
/// три живых устройства и одно мягко удалённое. Членство: устройства 1 и 2 в
/// группе 1, устройство 3 в группе 2, удалённое устройство 4 в группе 1,
/// устройство 5 (живое) в мягко удалённой группе 3.
fn seed(conn: &Connection) {
    conn.execute_batch(&format!(
        "INSERT INTO places (id, parent_id, kind, name, is_storage, created_at_utc, updated_at_utc)
           VALUES (1, NULL, 'building', 'Склад А', 1, {NOW}, {NOW});
         INSERT INTO group_types (id, code, name, behavior, created_at_utc, updated_at_utc)
           VALUES (100, 'workstation_test', 'АРМ', 'container', {NOW}, {NOW});
         INSERT INTO groups (id, type_id, name, seq, place_id, created_at_utc, updated_at_utc)
           VALUES (1, 100, 'АРМ #1', 1, 1, {NOW}, {NOW}),
                  (2, 100, 'АРМ #2', 2, NULL, {NOW}, {NOW});
         INSERT INTO groups (id, type_id, name, seq, place_id, created_at_utc, updated_at_utc, deleted_at_utc)
           VALUES (3, 100, 'АРМ #3', 3, NULL, {NOW}, {NOW}, {NOW});
         INSERT INTO devices (id, type_id, name, inventory_number, status_id, place_id,
                              created_at_utc, updated_at_utc)
           VALUES (1, 1, 'Монитор #1', 'ИНВ-000001', 2, 1, {NOW}, {NOW}),
                  (2, 1, 'Монитор #2', 'ИНВ-000002', 2, 1, {NOW}, {NOW}),
                  (3, 1, 'Планка памяти', 'ИНВ-000003', 2, 1, {NOW}, {NOW}),
                  (5, 1, 'Клавиатура', 'ИНВ-000005', 2, 1, {NOW}, {NOW});
         INSERT INTO devices (id, type_id, name, inventory_number, status_id, place_id,
                              created_at_utc, updated_at_utc, deleted_at_utc)
           VALUES (4, 1, 'Списанный блок', 'ИНВ-000004', 2, 1, {NOW}, {NOW}, {NOW});
         INSERT INTO group_devices (device_id, group_id, added_at_utc)
           VALUES (1, 1, {NOW}), (2, 1, {NOW}), (3, 2, {NOW}), (4, 1, {NOW}), (5, 3, {NOW});"
    ))
    .expect("seed");
}

fn effect(act_id: i64, kind: ActGroupEffectKind, group_id: i64, label: &str) -> ActGroupEffectNew {
    ActGroupEffectNew {
        act_id,
        kind,
        group_id,
        group_label: label.to_string(),
        device_id: None,
        from_place_id: None,
        to_place_id: None,
        from_parent_group_id: None,
        created_at_utc: NOW,
    }
}

#[test]
fn insert_and_list_return_rows_in_id_order_with_all_fields() {
    let (mut conn, _guard) = fresh_db();
    seed(&conn);
    let repo = SqliteGroupRepository;
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .expect("tx");

    let mut release = effect(7, ActGroupEffectKind::Release, 1, "АРМ #1");
    release.device_id = Some(2);
    let mut moved = effect(7, ActGroupEffectKind::Move, 2, "АРМ #2");
    moved.from_place_id = Some(1);
    moved.to_place_id = Some(9);
    let mut detach = effect(7, ActGroupEffectKind::Detach, 2, "АРМ #2");
    detach.from_parent_group_id = Some(1);

    let id1 = repo.insert_act_group_effect_in_tx(&tx, &release).unwrap();
    let id2 = repo.insert_act_group_effect_in_tx(&tx, &moved).unwrap();
    let id3 = repo.insert_act_group_effect_in_tx(&tx, &detach).unwrap();
    assert!(id1 < id2 && id2 < id3);

    let rows = repo.list_act_group_effects_in_tx(&tx, 7).unwrap();
    assert_eq!(rows.len(), 3);
    assert_eq!(
        rows.iter().map(|r| r.id).collect::<Vec<_>>(),
        vec![id1, id2, id3]
    );
    assert_eq!(rows[0].kind, ActGroupEffectKind::Release);
    assert_eq!(rows[0].device_id, Some(2));
    assert_eq!(rows[0].group_label, "АРМ #1");
    assert_eq!(rows[1].kind, ActGroupEffectKind::Move);
    assert_eq!(
        (rows[1].from_place_id, rows[1].to_place_id),
        (Some(1), Some(9))
    );
    assert_eq!(rows[2].kind, ActGroupEffectKind::Detach);
    assert_eq!(rows[2].from_parent_group_id, Some(1));
    assert_eq!(rows[2].created_at_utc, NOW);
    assert!(rows.iter().all(|r| r.act_id == 7));
}

#[test]
fn list_skips_rows_with_unknown_kind() {
    let (mut conn, _guard) = fresh_db();
    seed(&conn);
    conn.execute(
        "INSERT INTO act_group_effects (act_id, kind, group_id, group_label, created_at_utc)
         VALUES (7, 'future_kind', 1, 'АРМ #1', ?1)",
        params![NOW],
    )
    .unwrap();
    let repo = SqliteGroupRepository;
    let tx = conn.transaction().expect("tx");
    repo.insert_act_group_effect_in_tx(&tx, &effect(7, ActGroupEffectKind::Detach, 2, "АРМ #2"))
        .unwrap();
    let rows = repo.list_act_group_effects_in_tx(&tx, 7).unwrap();
    assert_eq!(rows.len(), 1, "unknown kind must be skipped, not fail");
    assert_eq!(rows[0].kind, ActGroupEffectKind::Detach);
}

#[test]
fn delete_by_act_removes_only_that_act_and_returns_count() {
    let (mut conn, _guard) = fresh_db();
    seed(&conn);
    let repo = SqliteGroupRepository;
    let tx = conn.transaction().expect("tx");
    repo.insert_act_group_effect_in_tx(&tx, &effect(7, ActGroupEffectKind::Release, 1, "АРМ #1"))
        .unwrap();
    repo.insert_act_group_effect_in_tx(&tx, &effect(7, ActGroupEffectKind::Move, 1, "АРМ #1"))
        .unwrap();
    let other = repo
        .insert_act_group_effect_in_tx(&tx, &effect(8, ActGroupEffectKind::Release, 2, "АРМ #2"))
        .unwrap();

    assert_eq!(repo.delete_act_group_effects_in_tx(&tx, 7).unwrap(), 2);
    assert!(repo
        .list_act_group_effects_in_tx(&tx, 7)
        .unwrap()
        .is_empty());
    let left = repo.list_act_group_effects_in_tx(&tx, 8).unwrap();
    assert_eq!(left.len(), 1);
    assert_eq!(left[0].id, other);

    repo.delete_act_group_effect_in_tx(&tx, other).unwrap();
    assert!(repo
        .list_act_group_effects_in_tx(&tx, 8)
        .unwrap()
        .is_empty());
    assert_eq!(repo.delete_act_group_effects_in_tx(&tx, 8).unwrap(), 0);
}

#[test]
fn effect_survives_hard_delete_of_group_and_keeps_label() {
    let (mut conn, _guard) = fresh_db();
    seed(&conn);
    conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
    let repo = SqliteGroupRepository;
    {
        let tx = conn.transaction().expect("tx");
        repo.insert_act_group_effect_in_tx(
            &tx,
            &effect(7, ActGroupEffectKind::Release, 2, "АРМ #2"),
        )
        .unwrap();
        tx.commit().unwrap();
    }
    // Группа 2 содержит устройство 3 (CASCADE на group_devices) и не якорь.
    conn.execute("DELETE FROM groups WHERE id = 2", []).unwrap();
    let still: i64 = conn
        .query_row("SELECT COUNT(*) FROM groups WHERE id = 2", [], |r| r.get(0))
        .unwrap();
    assert_eq!(still, 0, "group must be physically gone");

    let tx = conn.transaction().expect("tx");
    let rows = repo.list_act_group_effects_in_tx(&tx, 7).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].group_id, 2);
    assert_eq!(rows[0].group_label, "АРМ #2");
}

#[test]
fn live_groups_for_plan_excludes_soft_deleted_group() {
    let (conn, _guard) = fresh_db();
    seed(&conn);
    let repo = SqliteGroupRepository;
    let groups = repo.list_live_groups_for_plan(&conn).unwrap();
    assert_eq!(groups.iter().map(|g| g.id).collect::<Vec<_>>(), vec![1, 2]);
    assert_eq!(groups[0].name, "АРМ #1");
    assert_eq!(groups[0].place_id, Some(1));
    assert_eq!(groups[1].place_id, None);
    assert_eq!(groups[0].version, 1);
    assert_eq!(groups[0].parent_group_id, None);
    assert_eq!(groups[0].anchor_device_id, None);
    // Фикстура содержит мягко удалённую группу 3, иначе проверка вакуумна.
    let soft_deleted: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM groups WHERE deleted_at_utc IS NOT NULL",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(soft_deleted, 1);
}

#[test]
fn live_memberships_exclude_deleted_group_and_deleted_device() {
    let (conn, _guard) = fresh_db();
    seed(&conn);
    let repo = SqliteGroupRepository;
    // Обе «мёртвые» пары присутствуют в group_devices: (4, 1) — удалённое
    // устройство, (5, 3) — удалённая группа.
    let raw: i64 = conn
        .query_row("SELECT COUNT(*) FROM group_devices", [], |r| r.get(0))
        .unwrap();
    assert_eq!(raw, 5);
    let pairs = repo.list_live_memberships(&conn).unwrap();
    assert_eq!(pairs, vec![(1, 1), (2, 1), (3, 2)]);
}

#[test]
fn snapshot_readers_work_inside_a_transaction() {
    let (mut conn, _guard) = fresh_db();
    seed(&conn);
    let repo = SqliteGroupRepository;
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .expect("tx");
    assert_eq!(repo.list_live_groups_for_plan(&tx).unwrap().len(), 2);
    assert_eq!(repo.list_live_memberships(&tx).unwrap().len(), 3);
}
