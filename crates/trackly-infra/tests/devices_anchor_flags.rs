//! Integration tests: opt-in флаги якорных групп в `DeviceRepository::list` /
//! `list_grouped` и `count_hidden_anchor_members` (Phase 41.1 Plan 12, GRD-02/03).
//! Вымышленные имена и номера.

use rusqlite::{params, Connection};
use trackly_core::domain::devices::{DeviceFilter, DeviceNew, Pagination};
use trackly_core::ports::devices::DeviceRepository;
use trackly_infra::repos::SqliteDeviceRepository;
use trackly_infra::test_support::test_db;

const NOW: i64 = 1_700_000_000;

struct Fixture {
    anchor: i64,
    m1: i64,
    m2: i64,
    n1: i64,
    plain: i64,
    g1: i64,
    g2: i64,
}

fn page() -> Pagination {
    Pagination {
        offset: 0,
        limit: 50,
    }
}

fn add_device(conn: &mut Connection, name: &str, inv: &str) -> i64 {
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
                type_id: 1,
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

fn add_group(conn: &Connection, type_id: i64, seq: i64, parent: Option<i64>) -> i64 {
    conn.execute(
        "INSERT INTO groups (type_id, name, seq, parent_group_id, version, \
         created_at_utc, updated_at_utc) VALUES (?1, ?2, ?3, ?4, 1, ?5, ?5)",
        params![type_id, format!("Разбор {seq}"), seq, parent, NOW],
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

/// Якорь A (G1, он же член G1), m1/m2 в G1, n1 во вложенной G2, обычное p.
/// A и m1 носят одно имя, m2 и n1 — другое: проверка D-11 на реальных наборах.
fn fixture(conn: &mut Connection) -> Fixture {
    conn.execute(
        "INSERT INTO group_types (code, name, behavior, created_at_utc, updated_at_utc) \
         VALUES ('t_anchor_flags', 'Тип', 'repair', ?1, ?1)",
        params![NOW],
    )
    .unwrap();
    let type_id = conn.last_insert_rowid();
    let anchor = add_device(conn, "Системный блок #4", "ИНВ-000001");
    let m1 = add_device(conn, "Системный блок #4", "ИНВ-000002");
    let m2 = add_device(conn, "Монитор #2", "ИНВ-000003");
    let n1 = add_device(conn, "Монитор #2", "ИНВ-000004");
    let plain = add_device(conn, "Мышь", "ИНВ-000005");
    let g1 = add_group(conn, type_id, 1, None);
    let g2 = add_group(conn, type_id, 2, Some(g1));
    conn.execute(
        "UPDATE groups SET anchor_device_id = ?1 WHERE id = ?2",
        params![anchor, g1],
    )
    .unwrap();
    for d in [anchor, m1, m2] {
        join(conn, d, g1);
    }
    join(conn, n1, g2);
    Fixture {
        anchor,
        m1,
        m2,
        n1,
        plain,
        g1,
        g2,
    }
}

fn ids(conn: &Connection, filter: &DeviceFilter) -> (Vec<i64>, u64) {
    let (rows, total) = SqliteDeviceRepository.list(conn, filter, &page()).unwrap();
    let mut v: Vec<i64> = rows.iter().map(|r| r.id).collect();
    v.sort_unstable();
    (v, total)
}

fn sorted(mut v: Vec<i64>) -> Vec<i64> {
    v.sort_unstable();
    v
}

#[test]
fn list_without_flags_returns_everything() {
    let (mut conn, _d) = test_db();
    let f = fixture(&mut conn);
    let (got, total) = ids(&conn, &DeviceFilter::default());
    assert_eq!(got, sorted(vec![f.anchor, f.m1, f.m2, f.n1, f.plain]));
    assert_eq!(total, 5);
}

#[test]
fn hide_anchor_members_keeps_anchor_and_plain_and_total_matches() {
    let (mut conn, _d) = test_db();
    let f = fixture(&mut conn);
    let flt = DeviceFilter {
        hide_anchor_members: true,
        ..Default::default()
    };
    let (got, total) = ids(&conn, &flt);
    assert_eq!(got, sorted(vec![f.anchor, f.plain]));
    assert_eq!(total, 2);
    assert_eq!(
        SqliteDeviceRepository
            .count_hidden_anchor_members(&conn, &flt)
            .unwrap(),
        3
    );
}

#[test]
fn anchors_only_returns_only_anchor() {
    let (mut conn, _d) = test_db();
    let f = fixture(&mut conn);
    let (got, total) = ids(
        &conn,
        &DeviceFilter {
            anchors_only: true,
            ..Default::default()
        },
    );
    assert_eq!(got, vec![f.anchor]);
    assert_eq!(total, 1);
}

#[test]
fn member_of_group_returns_subtree_without_anchor() {
    let (mut conn, _d) = test_db();
    let f = fixture(&mut conn);
    let (got, total) = ids(
        &conn,
        &DeviceFilter {
            member_of_group_id: Some(f.g1),
            ..Default::default()
        },
    );
    assert_eq!(got, sorted(vec![f.m1, f.m2, f.n1]));
    assert_eq!(total, 3);
    let (nested, _) = ids(
        &conn,
        &DeviceFilter {
            member_of_group_id: Some(f.g2),
            ..Default::default()
        },
    );
    assert_eq!(nested, vec![f.n1]);
}

#[test]
fn hidden_count_ignores_soft_deleted_and_respects_status() {
    let (mut conn, _d) = test_db();
    let f = fixture(&mut conn);
    conn.execute(
        "UPDATE devices SET deleted_at_utc = ?1 WHERE id = ?2",
        params![NOW, f.n1],
    )
    .unwrap();
    let flt = DeviceFilter {
        hide_anchor_members: true,
        ..Default::default()
    };
    assert_eq!(
        SqliteDeviceRepository
            .count_hidden_anchor_members(&conn, &flt)
            .unwrap(),
        2
    );
    let other: i64 = conn
        .query_row(
            "SELECT id FROM device_statuses WHERE code = 'в_работе'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let by_status = DeviceFilter {
        hide_anchor_members: true,
        status_id: Some(other),
        ..Default::default()
    };
    assert_eq!(
        SqliteDeviceRepository
            .count_hidden_anchor_members(&conn, &by_status)
            .unwrap(),
        0
    );
}

fn grouped(conn: &Connection, flt: &DeviceFilter) -> Vec<(String, Vec<i64>)> {
    let mut v: Vec<(String, Vec<i64>)> = SqliteDeviceRepository
        .list_grouped(conn, flt, &page())
        .unwrap()
        .into_iter()
        .map(|g| (g.repr.name.clone(), sorted(g.ids)))
        .collect();
    v.sort();
    v
}

/// Все три SQL-ветки `list_grouped` (D-11).
#[test]
fn exclude_anchors_and_hide_apply_in_all_three_grouped_branches() {
    let (mut conn, _d) = test_db();
    let f = fixture(&mut conn);
    let branches: [(&str, DeviceFilter); 3] = [
        ("without_condition", DeviceFilter::default()),
        (
            "by_model_no_query",
            DeviceFilter {
                group_by_condition: true,
                ..Default::default()
            },
        ),
        (
            "by_model_with_query",
            DeviceFilter {
                group_by_condition: true,
                name_prefix: Some("Системный".to_string()),
                ..Default::default()
            },
        ),
    ];
    for (label, base) in branches {
        let plain = grouped(&conn, &base);
        let sys = plain
            .iter()
            .find(|(n, _)| n == "Системный блок #4")
            .unwrap_or_else(|| panic!("{label}: набор без флагов"));
        assert_eq!(sys.1, sorted(vec![f.anchor, f.m1]), "{label}: без флагов");

        let excl = grouped(
            &conn,
            &DeviceFilter {
                exclude_anchors: true,
                ..base.clone()
            },
        );
        let sys = excl
            .iter()
            .find(|(n, _)| n == "Системный блок #4")
            .unwrap_or_else(|| panic!("{label}: набор с exclude"));
        assert_eq!(sys.1, vec![f.m1], "{label}: якорь вне свёртки");

        let hid = grouped(
            &conn,
            &DeviceFilter {
                hide_anchor_members: true,
                ..base.clone()
            },
        );
        let sys = hid
            .iter()
            .find(|(n, _)| n == "Системный блок #4")
            .unwrap_or_else(|| panic!("{label}: набор со скрытием"));
        assert_eq!(sys.1, vec![f.anchor], "{label}: член скрыт, якорь виден");
        assert!(
            hid.iter().all(|(n, _)| n != "Монитор #2"),
            "{label}: члены вложенной группы скрыты"
        );
    }
}
