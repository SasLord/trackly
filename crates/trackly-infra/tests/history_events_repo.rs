//! Интеграционные тесты `SqliteHistoryEventsRepository` и миграции V057 (фаза 41.10-01,
//! HST-05/HST-06). Только вымышленные данные.

use rusqlite::Connection;
use trackly_core::domain::place_movements::{MovementEntityKind, MovementSource};
use trackly_core::domain::places::{PlaceKind, PlaceNew};
use trackly_core::ports::places::PlaceRepository;
use trackly_infra::repos::history_events_sqlite::NewHistoryEvent;
use trackly_infra::repos::{
    SqliteHistoryEventsRepository, SqlitePlaceMovementsRepository, SqlitePlaceRepository,
};
use trackly_infra::test_support::test_db;

const NOW: i64 = 1_700_000_000;

fn ev<'a>(kind: &'a str, subject_type: &'a str, subject_id: i64, at: i64) -> NewHistoryEvent<'a> {
    NewHistoryEvent {
        operation_id: "op-1",
        subject_type,
        subject_id,
        subject_label: Some("АРМ-5"),
        kind,
        source: "manual",
        created_at_utc: at,
        actor_name_snapshot: Some("Иванов И.И."),
        ..Default::default()
    }
}

#[test]
fn v057_creates_table_indexes_and_user_version() {
    let (conn, _dir) = test_db();
    let uv: i64 = conn
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .unwrap();
    assert!(uv >= 57, "{uv}");
    let tables: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='history_events'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(tables, 1);
    let idx: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND tbl_name='history_events' \
             AND name LIKE 'idx_history_events_%'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(idx, 6);
    let cnt: i64 = conn
        .query_row("SELECT COUNT(*) FROM history_events", [], |r| r.get(0))
        .unwrap();
    assert_eq!(cnt, 0, "D-03: бэкфилла нет");
}

#[test]
fn v057_file_ends_with_user_version_57() {
    let sql = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../migrations/V057__history_events.sql"
    ))
    .unwrap();
    assert_eq!(
        sql.trim_end().lines().last().unwrap(),
        "PRAGMA user_version = 57;"
    );
}

#[test]
fn insert_and_list_for_device_orders_desc() {
    let (mut conn, _dir) = test_db();
    let repo = SqliteHistoryEventsRepository;
    let tx = conn.transaction().unwrap();
    let a = repo
        .insert_in_tx(&tx, ev("group_joined", "device", 7, NOW))
        .unwrap();
    let b = repo
        .insert_in_tx(&tx, ev("group_left", "device", 7, NOW))
        .unwrap();
    let c = repo
        .insert_in_tx(&tx, ev("group_joined", "device", 7, NOW + 10))
        .unwrap();
    repo.insert_in_tx(&tx, ev("group_joined", "device", 8, NOW))
        .unwrap();
    tx.commit().unwrap();
    let rows = repo.list_for_device(&conn, 7).unwrap();
    let ids: Vec<i64> = rows.iter().map(|r| r.id).collect();
    assert_eq!(ids, vec![c, b, a]);
    assert_eq!(rows[0].actor_name_snapshot.as_deref(), Some("Иванов И.И."));
}

#[test]
fn list_for_group_covers_subject_to_from_and_skips_status() {
    let (mut conn, _dir) = test_db();
    let repo = SqliteHistoryEventsRepository;
    let tx = conn.transaction().unwrap();
    repo.insert_in_tx(&tx, ev("group_formed", "group", 3, NOW))
        .unwrap();
    let mut joined = ev("group_joined", "device", 1, NOW + 1);
    joined.to_group_id = Some(3);
    repo.insert_in_tx(&tx, joined).unwrap();
    let mut left = ev("group_left", "device", 2, NOW + 2);
    left.from_group_id = Some(3);
    repo.insert_in_tx(&tx, left).unwrap();
    let mut st = ev("status_changed", "device", 1, NOW + 3);
    st.to_group_id = Some(3);
    repo.insert_in_tx(&tx, st).unwrap();
    repo.insert_in_tx(&tx, ev("group_formed", "group", 4, NOW))
        .unwrap();
    tx.commit().unwrap();
    let rows = repo.list_for_group(&conn, 3).unwrap();
    assert_eq!(rows.len(), 3);
    assert!(rows.iter().all(|r| r.kind != "status_changed"));
}

#[test]
fn delete_by_act_id_removes_only_that_act() {
    let (mut conn, _dir) = test_db();
    let repo = SqliteHistoryEventsRepository;
    let tx = conn.transaction().unwrap();
    let mut a = ev("group_joined", "device", 1, NOW);
    a.act_id = Some(10);
    let mut b = ev("group_joined", "device", 1, NOW);
    b.act_id = Some(11);
    repo.insert_in_tx(&tx, a).unwrap();
    repo.insert_in_tx(&tx, b).unwrap();
    repo.insert_in_tx(&tx, ev("group_left", "device", 1, NOW))
        .unwrap();
    assert_eq!(repo.delete_by_act_id_in_tx(&tx, 10).unwrap(), 1);
    tx.commit().unwrap();
    assert_eq!(repo.list_for_device(&conn, 1).unwrap().len(), 2);
}

#[test]
fn unknown_tokens_roundtrip_raw() {
    let (mut conn, _dir) = test_db();
    let repo = SqliteHistoryEventsRepository;
    let tx = conn.transaction().unwrap();
    let mut e = ev("future_kind", "device", 1, NOW);
    e.source = "future_source";
    repo.insert_in_tx(&tx, e).unwrap();
    tx.commit().unwrap();
    let rows = repo.list_for_device(&conn, 1).unwrap();
    assert_eq!(rows[0].kind, "future_kind");
    assert_eq!(rows[0].source, "future_source");
}

fn place(conn: &mut Connection, name: &str) -> i64 {
    SqlitePlaceRepository
        .create(
            conn,
            &PlaceNew {
                parent_id: None,
                kind: PlaceKind::Building,
                name: name.to_string(),
                level: None,
                is_storage: false,
                sort_order: None,
                notes: None,
            },
            NOW,
        )
        .unwrap()
}

#[test]
fn record_movement_returns_movement_id() {
    let (mut conn, _dir) = test_db();
    let a = place(&mut conn, "Здание А");
    let b = place(&mut conn, "Здание Б");
    let repo = SqlitePlaceMovementsRepository;
    let places = SqlitePlaceRepository;
    let tx = conn.transaction().unwrap();
    let call = |before, after| {
        repo.record_movement_if_applicable(
            &tx,
            &places,
            MovementEntityKind::Device,
            1,
            before,
            after,
            MovementSource::Manual,
            None,
            None,
            None,
            NOW,
        )
        .unwrap()
    };
    assert_eq!(call(Some(a), Some(a)), None);
    assert_eq!(call(None, Some(b)), None);
    let id = call(Some(a), Some(b)).expect("Some(id) при реальной смене места");
    let stored: i64 = tx
        .query_row("SELECT id FROM place_movements", [], |r| r.get(0))
        .unwrap();
    assert_eq!(id, stored);
}
