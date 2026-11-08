//! Phase 41.9 Plan 03: ключи смешанного списка, корневые контейнеры, строки
//! контейнеров и прямые дети группы (GRD-07, GRD-08, GRD-10, GRD-11).
//! Имена вымышленные (репозиторий публичный). Префикс тестов: `container_entries_`.

use rusqlite::{params, Connection};
use trackly_core::domain::devices::{ContainerRow, DeviceFilter, DeviceNew, EntryKey, EntryKind};
use trackly_core::ports::devices::DeviceRepository;
use trackly_infra::repos::SqliteDeviceRepository;
use trackly_infra::test_support::test_db;

const NOW: i64 = 1_700_000_000;

fn add_device(conn: &mut Connection, type_id: i64, name: &str, inv: &str) -> i64 {
    let status: i64 = conn
        .query_row(
            "SELECT id FROM device_statuses WHERE code = 'на_складе'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    SqliteDeviceRepository
        .create(
            conn,
            &DeviceNew {
                type_id,
                name: name.to_string(),
                inventory_no: Some(inv.to_string()),
                serial_no: None,
                model: None,
                specs: None,
                kit: None,
                state: None,
                place_id: None,
                status_id: status,
            },
            NOW,
        )
        .unwrap()
}

fn add_type(conn: &Connection, code: &str, behavior: &str) -> i64 {
    conn.execute(
        "INSERT INTO group_types (code, name, behavior, created_at_utc, updated_at_utc) \
         VALUES (?1, ?1, ?2, ?3, ?3)",
        params![code, behavior, NOW],
    )
    .unwrap();
    conn.last_insert_rowid()
}

fn add_group(
    conn: &Connection,
    type_id: i64,
    name: &str,
    seq: i64,
    parent: Option<i64>,
    anchor: Option<i64>,
    deleted: bool,
) -> i64 {
    conn.execute(
        "INSERT INTO groups (type_id, name, seq, parent_group_id, anchor_device_id, \
         deleted_at_utc, version, created_at_utc, updated_at_utc) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, ?7, ?7)",
        params![
            type_id,
            name,
            seq,
            parent,
            anchor,
            deleted.then_some(NOW),
            NOW
        ],
    )
    .unwrap();
    conn.last_insert_rowid()
}

fn join(conn: &Connection, device: i64, group: i64) {
    conn.execute(
        "INSERT INTO group_devices (device_id, group_id, added_at_utc) VALUES (?1, ?2, ?3)",
        params![device, group, NOW],
    )
    .unwrap();
}

struct Fx {
    del_member: i64,
    mon: i64,
    ups: i64,
    printer_member: i64,
    printer_usb: i64,
    sysblock: i64,
    drive: i64,
    same_name_device: i64,
    arm: i64,
    arm_anchored: i64,
    arm_nested: i64,
    arm_empty: i64,
    arm_nested_empty: i64,
    arm_deleted: i64,
}

fn fixture(conn: &mut Connection) -> Fx {
    let t_cont = add_type(conn, "t_cont", "container");
    let t_rep = add_type(conn, "t_rep", "repair");
    let ups = add_device(conn, 1, "ИБП #1", "ИНВ-000001");
    let mon = add_device(conn, 1, "Монитор #2", "ИНВ-000002");
    let printer_member = add_device(conn, 2, "Принтер #5", "ИНВ-000005");
    let printer_usb = add_device(conn, 2, "Принтер #6", "ИНВ-000006");
    let sysblock = add_device(conn, 1, "Системный блок #4", "ИНВ-000004");
    let drive = add_device(conn, 1, "Накопитель #7", "ИНВ-000007");
    let nested_member = add_device(conn, 1, "Клавиатура #9", "ИНВ-000009");
    let same_name_device = add_device(conn, 1, "АРМ #3", "ИНВ-000010");
    let del_member = add_device(conn, 1, "Мышь #11", "ИНВ-000011");
    let in_empty = add_device(conn, 1, "Кабель #12", "ИНВ-000012");
    conn.execute(
        "UPDATE devices SET deleted_at_utc = ?1 WHERE id = ?2",
        params![NOW, in_empty],
    )
    .unwrap();

    let arm = add_group(conn, t_cont, "АРМ #3", 1, None, None, false);
    join(conn, mon, arm);
    join(conn, printer_member, arm);
    // Вложенная якорная группа: якорь sysblock, член drive.
    let arm_anchored = add_group(conn, t_rep, "Разбор 2", 2, Some(arm), Some(sysblock), false);
    join(conn, sysblock, arm_anchored);
    join(conn, drive, arm_anchored);
    // Вложенный АРМ без якоря с одним членом.
    let arm_nested = add_group(conn, t_cont, "Стойка #8", 3, Some(arm), None, false);
    join(conn, nested_member, arm_nested);
    // Вложенный пустой АРМ (единственный член удалён).
    let arm_nested_empty = add_group(conn, t_cont, "Пустая стойка", 4, Some(arm), None, false);
    join(conn, in_empty, arm_nested_empty);
    // Корневой пустой АРМ.
    let arm_empty = add_group(conn, t_cont, "Пустой АРМ", 5, None, None, false);
    // Удалённый АРМ с членом.
    let arm_deleted = add_group(conn, t_cont, "Удалённый АРМ", 6, None, None, true);
    join(conn, del_member, arm_deleted);

    Fx {
        del_member,
        mon,
        ups,
        printer_member,
        printer_usb,
        sysblock,
        drive,
        same_name_device,
        arm,
        arm_anchored,
        arm_nested,
        arm_empty,
        arm_nested_empty,
        arm_deleted,
    }
}

fn hide_filter() -> DeviceFilter {
    DeviceFilter {
        hide_anchor_members: true,
        ..DeviceFilter::default()
    }
}

fn dev(id: i64) -> EntryKey {
    EntryKey {
        kind: EntryKind::Device,
        id,
    }
}

fn cont(id: i64) -> EntryKey {
    EntryKey {
        kind: EntryKind::Container,
        id,
    }
}

fn all_keys(conn: &Connection, filter: &DeviceFilter, size: i64) -> (Vec<EntryKey>, u64) {
    let repo = SqliteDeviceRepository;
    let mut out = Vec::new();
    let (first, total) = repo.list_entry_keys(conn, filter, size, 0).unwrap();
    out.extend(first);
    while (out.len() as u64) < total {
        let (page, t) = repo
            .list_entry_keys(conn, filter, size, out.len() as i64)
            .unwrap();
        assert_eq!(t, total);
        assert!(!page.is_empty(), "page ended before total");
        out.extend(page);
    }
    (out, total)
}

#[test]
fn container_entries_keys_order_and_exclusions() {
    let (mut conn, _dir) = test_db();
    let fx = fixture(&mut conn);
    let (keys, total) = all_keys(&conn, &hide_filter(), 100);
    assert_eq!(total as usize, keys.len());
    // Видимые устройства: ИБП, одноимённое «АРМ #3», принтер вне групп, член удалённого
    // АРМ; контейнер — один.
    // Члены АРМ скрыты (включая якорь вложенной группы). Порядок: name BINARY, kind, id.
    assert_eq!(
        keys,
        vec![
            dev(fx.same_name_device),
            cont(fx.arm),
            dev(fx.ups),
            dev(fx.del_member),
            dev(fx.printer_usb),
        ]
    );
    for excluded in [
        fx.arm_empty,
        fx.arm_nested,
        fx.arm_nested_empty,
        fx.arm_deleted,
    ] {
        assert!(!keys.contains(&cont(excluded)), "group {excluded} leaked");
    }
    let _ = (
        fx.mon,
        fx.printer_member,
        fx.sysblock,
        fx.drive,
        fx.arm_anchored,
    );
}

#[test]
fn container_entries_pages_sum_to_total_without_gaps() {
    let (mut conn, _dir) = test_db();
    let _ = fixture(&mut conn);
    let (full, total) = all_keys(&conn, &hide_filter(), 100);
    for size in [1, 2] {
        let (paged, t) = all_keys(&conn, &hide_filter(), size);
        assert_eq!(t, total);
        assert_eq!(paged, full, "size {size}");
    }
}

#[test]
fn container_entries_no_container_rows_without_switch() {
    let (mut conn, _dir) = test_db();
    let fx = fixture(&mut conn);
    let repo = SqliteDeviceRepository;
    let off = DeviceFilter::default();
    let with_status = DeviceFilter {
        status_id: Some(1),
        ..hide_filter()
    };
    let only = DeviceFilter {
        anchors_only: true,
        ..hide_filter()
    };
    let member_of = DeviceFilter {
        member_of_group_id: Some(fx.arm),
        ..hide_filter()
    };
    for f in [off, with_status, only, member_of] {
        let (keys, total) = repo.list_entry_keys(&conn, &f, 100, 0).unwrap();
        assert!(keys.iter().all(|k| k.kind == EntryKind::Device));
        let (rows, list_total) = repo
            .list(
                &conn,
                &f,
                &trackly_core::domain::devices::Pagination {
                    offset: 0,
                    limit: 100,
                },
            )
            .unwrap();
        assert_eq!(total, list_total);
        let ids: Vec<i64> = keys.iter().map(|k| k.id).collect();
        let list_ids: Vec<i64> = rows.iter().map(|r| r.id).collect();
        assert_eq!(ids, list_ids);
    }
}

#[test]
fn container_entries_root_containers_counts() {
    let (mut conn, _dir) = test_db();
    let fx = fixture(&mut conn);
    let rows = SqliteDeviceRepository.list_root_containers(&conn).unwrap();
    assert_eq!(rows.len(), 1);
    let r: &ContainerRow = &rows[0];
    assert_eq!(r.id, fx.arm);
    assert_eq!(r.name, "АРМ #3");
    // mon, принтер, sysblock (якорь), drive, клавиатура вложенной стойки.
    assert_eq!(r.device_count, 5);
}

#[test]
fn container_entries_children_direct_levels() {
    let (mut conn, _dir) = test_db();
    let fx = fixture(&mut conn);
    let repo = SqliteDeviceRepository;
    let (keys, total) = repo.group_children_keys(&conn, fx.arm, 100, 0).unwrap();
    assert_eq!(total as usize, keys.len());
    // Монитор #2, Принтер #5 (член, D-15), Системный блок #4 (якорь вложенной), Стойка #8.
    assert_eq!(
        keys,
        vec![
            dev(fx.mon),
            dev(fx.printer_member),
            dev(fx.sysblock),
            cont(fx.arm_nested),
        ]
    );
    assert!(!keys.contains(&dev(fx.printer_usb)));
    assert!(!keys.contains(&dev(fx.drive)));
    assert!(!keys.contains(&cont(fx.arm_nested_empty)));
    assert!(!keys.contains(&cont(fx.arm_anchored)));
}

#[test]
fn container_entries_children_limit_offset_without_repeats() {
    let (mut conn, _dir) = test_db();
    let fx = fixture(&mut conn);
    let repo = SqliteDeviceRepository;
    let (all, total) = repo.group_children_keys(&conn, fx.arm, 100, 0).unwrap();
    let (a, _) = repo.group_children_keys(&conn, fx.arm, 2, 0).unwrap();
    let (b, t2) = repo.group_children_keys(&conn, fx.arm, 2, 2).unwrap();
    assert_eq!(t2, total);
    let mut joined = a;
    joined.extend(b);
    assert_eq!(joined, all);
}

#[test]
fn container_entries_children_hide_deleted_child() {
    let (mut conn, _dir) = test_db();
    let fx = fixture(&mut conn);
    conn.execute(
        "UPDATE devices SET deleted_at_utc = ?1 WHERE id = ?2",
        params![NOW, fx.mon],
    )
    .unwrap();
    let (keys, _) = SqliteDeviceRepository
        .group_children_keys(&conn, fx.arm, 100, 0)
        .unwrap();
    assert!(!keys.contains(&dev(fx.mon)));
}

#[test]
fn container_entries_rows_order_and_counts() {
    let (mut conn, _dir) = test_db();
    let fx = fixture(&mut conn);
    let repo = SqliteDeviceRepository;
    assert!(repo.container_rows(&conn, &[]).unwrap().is_empty());
    let rows = repo
        .container_rows(&conn, &[fx.arm_nested, 9999, fx.arm, fx.arm_deleted])
        .unwrap();
    let got: Vec<(i64, i64)> = rows.iter().map(|r| (r.id, r.device_count)).collect();
    assert_eq!(got, vec![(fx.arm_nested, 1), (fx.arm, 5)]);
}
