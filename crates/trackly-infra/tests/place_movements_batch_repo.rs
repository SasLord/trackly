//! Integration tests: пакетная запись журнала перемещений (Phase 41 Plan 05, GRP-06).
//!
//! Покрывает `record_batch_movement_if_applicable` (batch_id / entity_label / group_id),
//! регресс одиночного `record_movement_if_applicable` (все три колонки NULL) и чтение
//! через `get_history`. Только вымышленные данные («Склад А», «Склад Б», «АРМ #3»).

use std::time::Duration;

use rusqlite::Connection;
use trackly_core::domain::place_movements::{MovementEntityKind, MovementSource};
use trackly_core::domain::places::{PlaceKind, PlaceNew};
use trackly_core::ports::places::PlaceRepository;
use trackly_infra::repos::{SqlitePlaceMovementsRepository, SqlitePlaceRepository};
use trackly_infra::test_support::test_db;

const NOW: i64 = 1_700_000_000;
const BATCH: &str = "0b9d2f6e-5c1a-4f57-9a3e-3c2d7e8f1a10";
const LABEL: &str = "АРМ #3";
const GROUP_ID: i64 = 77;

fn new_place(name: &str) -> PlaceNew {
    PlaceNew {
        parent_id: None,
        kind: PlaceKind::Building,
        name: name.to_string(),
        level: None,
        is_storage: false,
        sort_order: None,
        notes: None,
    }
}

/// Два разных места: `(склад_а, склад_б)`.
fn seed_places(conn: &mut Connection) -> (i64, i64) {
    let repo = SqlitePlaceRepository;
    let a = repo
        .create(conn, &new_place("Склад А"), NOW)
        .expect("create A");
    let b = repo
        .create(conn, &new_place("Склад Б"), NOW)
        .expect("create B");
    (a, b)
}

#[allow(clippy::too_many_arguments)]
fn record_batch(
    conn: &mut Connection,
    kind: MovementEntityKind,
    entity_id: i64,
    before: Option<i64>,
    after: Option<i64>,
    batch_id: Option<&str>,
    label: Option<&str>,
    group_id: Option<i64>,
) {
    let places_repo = SqlitePlaceRepository;
    let repo = SqlitePlaceMovementsRepository;
    let tx = conn.transaction().expect("tx");
    repo.record_batch_movement_if_applicable(
        &tx,
        &places_repo,
        kind,
        entity_id,
        before,
        after,
        MovementSource::Group,
        None,
        None,
        None,
        NOW,
        batch_id,
        label,
        group_id,
    )
    .expect("record batch");
    tx.commit().expect("commit");
}

fn count(conn: &Connection) -> i64 {
    conn.query_row("SELECT COUNT(*) FROM place_movements", [], |r| r.get(0))
        .expect("count")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn single_record_leaves_batch_columns_null() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (mut conn, _dir) = test_db();
        let (a, b) = seed_places(&mut conn);
        let places_repo = SqlitePlaceRepository;
        let repo = SqlitePlaceMovementsRepository;
        let tx = conn.transaction().expect("tx");
        repo.record_movement_if_applicable(
            &tx,
            &places_repo,
            MovementEntityKind::Device,
            5,
            Some(a),
            Some(b),
            MovementSource::Manual,
            None,
            None,
            None,
            NOW,
        )
        .expect("record");
        tx.commit().expect("commit");

        let rows = repo.get_history(&conn, "device", 5).expect("history");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].batch_id, None);
        assert_eq!(rows[0].entity_label, None);
        assert_eq!(rows[0].group_id, None);
    })
    .await
    .expect("test timed out");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn batch_record_writes_columns_and_reads_them_back() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (mut conn, _dir) = test_db();
        let (a, b) = seed_places(&mut conn);
        record_batch(
            &mut conn,
            MovementEntityKind::Device,
            11,
            Some(a),
            Some(b),
            Some(BATCH),
            Some(LABEL),
            Some(GROUP_ID),
        );
        let repo = SqlitePlaceMovementsRepository;
        let rows = repo.get_history(&conn, "device", 11).expect("history");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].batch_id.as_deref(), Some(BATCH));
        assert_eq!(rows[0].entity_label.as_deref(), Some(LABEL));
        assert_eq!(rows[0].group_id, Some(GROUP_ID));
        assert_eq!(rows[0].source, "group");
        assert_ne!(rows[0].from_place_id, rows[0].to_place_id);
    })
    .await
    .expect("test timed out");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn batch_record_applies_the_same_skip_guard() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (mut conn, _dir) = test_db();
        let (a, b) = seed_places(&mut conn);
        // NULL -> место, место -> NULL, место -> то же место: ни одной строки.
        for (before, after) in [(None, Some(b)), (Some(a), None), (Some(a), Some(a))] {
            record_batch(
                &mut conn,
                MovementEntityKind::Device,
                12,
                before,
                after,
                Some(BATCH),
                Some(LABEL),
                Some(GROUP_ID),
            );
        }
        assert_eq!(count(&conn), 0, "guard is_reportable_place_change");
    })
    .await
    .expect("test timed out");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn batch_of_three_rows_is_split_per_entity_and_shares_label_and_group() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (mut conn, _dir) = test_db();
        let (a, b) = seed_places(&mut conn);
        // Пакет: строка группы + два устройства, один batch_id.
        record_batch(
            &mut conn,
            MovementEntityKind::Group,
            GROUP_ID,
            Some(a),
            Some(b),
            Some(BATCH),
            Some(LABEL),
            Some(GROUP_ID),
        );
        for dev in [21_i64, 22] {
            record_batch(
                &mut conn,
                MovementEntityKind::Device,
                dev,
                Some(a),
                Some(b),
                Some(BATCH),
                Some(LABEL),
                Some(GROUP_ID),
            );
        }
        let repo = SqlitePlaceMovementsRepository;

        let dev = repo
            .get_history(&conn, "device", 21)
            .expect("device history");
        assert_eq!(dev.len(), 1, "get_history устройства — только его строка");
        assert_eq!(dev[0].entity_id, 21);

        let grp = repo
            .get_history(&conn, "group", GROUP_ID)
            .expect("group history");
        assert_eq!(grp.len(), 1, "get_history группы — только строка группы");
        assert_eq!(grp[0].entity_type, "group");

        assert_eq!(count(&conn), 3);
        let distinct: i64 = conn
            .query_row(
                "SELECT COUNT(DISTINCT batch_id || '|' || entity_label || '|' || group_id) \
                 FROM place_movements",
                [],
                |r| r.get(0),
            )
            .expect("distinct");
        assert_eq!(
            distinct, 1,
            "batch_id, entity_label и group_id совпадают во всех строках"
        );
        assert_eq!(dev[0].group_id, grp[0].group_id);
        assert_eq!(dev[0].entity_label, grp[0].entity_label);
    })
    .await
    .expect("test timed out");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn batch_without_group_row_still_carries_group_id() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (mut conn, _dir) = test_db();
        let (a, b) = seed_places(&mut conn);
        // Первое размещение группы: строки самой группы нет (NULL -> место не пишется),
        // у устройств место меняется — group_id всё равно в строке.
        record_batch(
            &mut conn,
            MovementEntityKind::Group,
            GROUP_ID,
            None,
            Some(b),
            Some(BATCH),
            Some(LABEL),
            Some(GROUP_ID),
        );
        record_batch(
            &mut conn,
            MovementEntityKind::Device,
            31,
            Some(a),
            Some(b),
            Some(BATCH),
            Some(LABEL),
            Some(GROUP_ID),
        );
        let repo = SqlitePlaceMovementsRepository;
        assert!(repo
            .get_history(&conn, "group", GROUP_ID)
            .unwrap()
            .is_empty());
        let dev = repo.get_history(&conn, "device", 31).expect("history");
        assert_eq!(dev.len(), 1);
        assert_eq!(dev[0].batch_id.as_deref(), Some(BATCH));
        assert_eq!(dev[0].group_id, Some(GROUP_ID));
        assert_eq!(dev[0].entity_label.as_deref(), Some(LABEL));
    })
    .await
    .expect("test timed out");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn add_devices_row_has_group_id_without_batch_id() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (mut conn, _dir) = test_db();
        let (a, b) = seed_places(&mut conn);
        record_batch(
            &mut conn,
            MovementEntityKind::Device,
            41,
            Some(a),
            Some(b),
            None,
            Some(LABEL),
            Some(GROUP_ID),
        );
        let repo = SqlitePlaceMovementsRepository;
        let dev = repo.get_history(&conn, "device", 41).expect("history");
        assert_eq!(dev.len(), 1);
        assert_eq!(dev[0].batch_id, None);
        assert_eq!(dev[0].group_id, Some(GROUP_ID));
        assert_eq!(dev[0].entity_label.as_deref(), Some(LABEL));
    })
    .await
    .expect("test timed out");
}
