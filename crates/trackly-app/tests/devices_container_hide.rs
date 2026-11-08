//! Phase 41.9 Plan 01: скрытие членов корневых контейнерных групп (GRD-08, D-12..D-15).
//!
//! Фикстура (имена вымышленные, репозиторий публичный): корневой контейнер C1
//! «АРМ #3» с членами m1, m2 и принтером p1; вложенная якорная группа G2
//! (якорь A2) с членом n1; вложенный контейнер C2 с членом n2; корневая якорная
//! группа G3 (якорь A3) с членом x; обычное устройство и принтер вне групп;
//! мягко удалённый контейнер C4 с членом d1; корневая группа behavior 'teardown'
//! без якоря с членом t1.
//!
//! Префикс тестов: `container_hide_`.

use std::sync::Arc;
use std::time::Duration;

use rusqlite::params;
use trackly_app::dto::device::{DeviceDto, DeviceFilter, Pagination};
use trackly_app::services::DeviceService;
use trackly_core::primitives::clock::Clock;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::error_conversions::map_rusqlite;
use trackly_infra::test_support::test_writer_and_readers;

const NOW: i64 = 1_700_000_000;

struct Fx {
    svc: DeviceService,
    _dir: tempfile::TempDir,
    hidden: Vec<i64>,
    visible: Vec<i64>,
    t1: i64,
    a3: i64,
}

fn page() -> Pagination {
    Pagination {
        offset: 0,
        limit: 100,
    }
}

async fn seed_device(svc: &DeviceService, type_id: i64, name: &str, inv: &str) -> i64 {
    let (name, inv) = (name.to_string(), inv.to_string());
    svc.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO devices (type_id, name, inventory_number, status_id, \
                 created_at_utc, updated_at_utc, version) \
                 VALUES (?1, ?2, ?3, 1, ?4, ?4, 1)",
                params![type_id, name, inv, NOW],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed device")
}

async fn seed_type(svc: &DeviceService, code: &'static str, behavior: &'static str) -> i64 {
    svc.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO group_types (code, name, behavior, created_at_utc, \
                 updated_at_utc) VALUES (?1, ?1, ?2, ?3, ?3)",
                params![code, behavior, NOW],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed group type")
}

async fn seed_group(
    svc: &DeviceService,
    type_id: i64,
    seq: i64,
    parent: Option<i64>,
    anchor: Option<i64>,
    deleted: bool,
) -> i64 {
    svc.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO groups (type_id, name, seq, parent_group_id, anchor_device_id, \
                 deleted_at_utc, version, created_at_utc, updated_at_utc) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, ?7, ?7)",
                params![
                    type_id,
                    format!("Группа {seq}"),
                    seq,
                    parent,
                    anchor,
                    if deleted { Some(NOW) } else { None },
                    NOW
                ],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed group")
}

async fn member(svc: &DeviceService, device: i64, group: i64) {
    svc.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO group_devices (device_id, group_id, added_at_utc) \
                 VALUES (?1, ?2, ?3)",
                params![device, group, NOW],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("member");
}

async fn fixture() -> Fx {
    let (writer, readers, dir) = test_writer_and_readers();
    let clock: Arc<dyn Clock + Send + Sync> = Arc::new(SystemClock);
    let svc = DeviceService::new(writer, readers, clock);

    let container = seed_type(&svc, "container_hide_arm", "container").await;
    let teardown = seed_type(&svc, "container_hide_td", "teardown").await;
    let anchored = seed_type(&svc, "container_hide_sub", "substitute").await;

    let m1 = seed_device(&svc, 1, "Монитор #2", "ИНВ-000001").await;
    let m2 = seed_device(&svc, 1, "ИБП #1", "ИНВ-000002").await;
    let p1 = seed_device(&svc, 2, "Принтер #1", "ИНВ-000003").await;
    let a2 = seed_device(&svc, 1, "Системный блок #4", "ИНВ-000004").await;
    let n1 = seed_device(&svc, 1, "Мышь #1", "ИНВ-000005").await;
    let n2 = seed_device(&svc, 1, "Клавиатура #1", "ИНВ-000006").await;
    let a3 = seed_device(&svc, 1, "Системный блок #5", "ИНВ-000007").await;
    let x = seed_device(&svc, 1, "Планка памяти #1", "ИНВ-000008").await;
    let plain = seed_device(&svc, 1, "Мышь #2", "ИНВ-000009").await;
    let p2 = seed_device(&svc, 2, "Принтер #2", "ИНВ-000010").await;
    let d1 = seed_device(&svc, 1, "Кабель #1", "ИНВ-000011").await;
    let t1 = seed_device(&svc, 1, "Кабель #2", "ИНВ-000012").await;

    let c1 = seed_group(&svc, container, 1, None, None, false).await;
    let g2 = seed_group(&svc, anchored, 2, Some(c1), Some(a2), false).await;
    let c2 = seed_group(&svc, container, 3, Some(c1), None, false).await;
    let g3 = seed_group(&svc, anchored, 4, None, Some(a3), false).await;
    let c4 = seed_group(&svc, container, 5, None, None, true).await;
    let td = seed_group(&svc, teardown, 6, None, None, false).await;

    for (d, g) in [
        (m1, c1),
        (m2, c1),
        (p1, c1),
        (a2, g2),
        (n1, g2),
        (n2, c2),
        (a3, g3),
        (x, g3),
        (d1, c4),
        (t1, td),
    ] {
        member(&svc, d, g).await;
    }

    Fx {
        svc,
        _dir: dir,
        hidden: vec![m1, m2, p1, a2, n1, n2, x],
        visible: vec![a3, plain, p2, d1, t1],
        t1,
        a3,
    }
}

fn sorted_ids(items: &[DeviceDto]) -> Vec<i64> {
    let mut v: Vec<i64> = items.iter().map(|d| d.id).collect();
    v.sort_unstable();
    v
}

fn sorted(mut v: Vec<i64>) -> Vec<i64> {
    v.sort_unstable();
    v
}

macro_rules! timed {
    ($body:expr) => {
        tokio::time::timeout(Duration::from_secs(30), $body)
            .await
            .expect("таймаут 30 с")
    };
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn container_hide_list_hides_root_container_members_and_counts_k() {
    timed!(async {
        let f = fixture().await;
        let r = f
            .svc
            .list(
                DeviceFilter {
                    hide_anchor_members: true,
                    ..Default::default()
                },
                page(),
            )
            .await
            .unwrap();
        assert_eq!(sorted_ids(&r.items), sorted(f.visible.clone()));
        assert_eq!(r.total, f.visible.len() as u64);
        assert_eq!(r.hidden_member_count, f.hidden.len() as i64);
    });
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn container_hide_without_flag_shows_all_devices() {
    timed!(async {
        let f = fixture().await;
        let r = f.svc.list(DeviceFilter::default(), page()).await.unwrap();
        let mut all = f.hidden.clone();
        all.extend(f.visible.iter().copied());
        assert_eq!(all.len(), 12);
        assert_eq!(sorted_ids(&r.items), sorted(all));
        assert_eq!(r.hidden_member_count, 0, "K без флага равен 0");
    });
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn container_hide_root_anchor_stays_visible_nested_anchor_hidden() {
    timed!(async {
        let f = fixture().await;
        let r = f
            .svc
            .list(
                DeviceFilter {
                    hide_anchor_members: true,
                    ..Default::default()
                },
                page(),
            )
            .await
            .unwrap();
        let ids = sorted_ids(&r.items);
        assert!(ids.contains(&f.a3), "якорь корневой якорной группы виден");
        // a2 (якорь вложенной группы в АРМ) — четвёртый в списке скрытых.
        assert!(!ids.contains(&f.hidden[3]), "якорь вложенной группы скрыт");
    });
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn container_hide_non_container_behavior_not_seeded() {
    timed!(async {
        let f = fixture().await;
        let r = f
            .svc
            .list(
                DeviceFilter {
                    hide_anchor_members: true,
                    ..Default::default()
                },
                page(),
            )
            .await
            .unwrap();
        assert!(sorted_ids(&r.items).contains(&f.t1), "член teardown виден");
    });
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn container_hide_grouped_excludes_hidden_members() {
    timed!(async {
        let f = fixture().await;
        let hidden_filter = DeviceFilter {
            hide_anchor_members: true,
            ..Default::default()
        };
        let groups = f.svc.list_grouped(hidden_filter, page()).await.unwrap();
        let mut seen: Vec<i64> = groups
            .iter()
            .flat_map(|g| g.ids.iter().map(|i| i64::from(*i)))
            .collect();
        seen.sort_unstable();
        assert_eq!(seen, sorted(f.visible.clone()));
        let all = f
            .svc
            .list_grouped(DeviceFilter::default(), page())
            .await
            .unwrap();
        let total: usize = all.iter().map(|g| g.ids.len()).sum();
        assert_eq!(total, 12, "без флага свёртка видит все устройства");
    });
}
