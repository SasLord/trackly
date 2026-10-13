//! Phase 41.1 Plan 12: контракт списка «Устройств» для якорной строки (GRD-01/02/03).
//!
//! Фикстура расходится до и после реализации: якорь A (группа G1), члены m1/m2,
//! член n1 вложенной группы G2 (родитель G1), обычное устройство p. Режимы запроса:
//! без флагов, со скрытием, `anchors_only`, `exclude_anchors` в свёртке,
//! `member_of_group_id`, поиск. Имена и номера вымышленные (репозиторий публичный).
//!
//! Префикс тестов: `anchor_list_`.

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

async fn seed_device(svc: &DeviceService, name: &str, inv: &str) -> i64 {
    let (name, inv) = (name.to_string(), inv.to_string());
    svc.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO devices (type_id, name, inventory_number, status_id, \
                 created_at_utc, updated_at_utc, version) \
                 VALUES (1, ?1, ?2, 1, ?3, ?3, 1)",
                params![name, inv, NOW],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed device")
}

async fn exec(svc: &DeviceService, sql: &'static str, p: Vec<i64>) {
    svc.writer
        .execute(move |conn| {
            conn.execute(sql, rusqlite::params_from_iter(p.iter()))
                .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("exec");
}

async fn seed_group(svc: &DeviceService, seq: i64, parent: Option<i64>) -> i64 {
    svc.writer
        .execute(move |conn| {
            let type_id: i64 = conn
                .query_row(
                    "SELECT id FROM group_types WHERE behavior = 'substitute' LIMIT 1",
                    [],
                    |r| r.get(0),
                )
                .or_else(|_| {
                    conn.execute(
                        "INSERT INTO group_types (code, name, behavior, created_at_utc, \
                         updated_at_utc) VALUES ('anchor_list_t', 'Тип', 'substitute', ?1, ?1)",
                        params![NOW],
                    )?;
                    Ok::<_, rusqlite::Error>(conn.last_insert_rowid())
                })
                .map_err(map_rusqlite)?;
            conn.execute(
                "INSERT INTO groups (type_id, name, seq, parent_group_id, version, \
                 created_at_utc, updated_at_utc) VALUES (?1, ?2, ?3, ?4, 1, ?5, ?5)",
                params![type_id, format!("Подмена {seq}"), seq, parent, NOW],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed group")
}

async fn fixture() -> Fx {
    let (writer, readers, dir) = test_writer_and_readers();
    let clock: Arc<dyn Clock + Send + Sync> = Arc::new(SystemClock);
    let svc = DeviceService::new(writer, readers, clock);

    let anchor = seed_device(&svc, "Системный блок #4", "ИНВ-000001").await;
    let m1 = seed_device(&svc, "Системный блок #4", "ИНВ-000002").await;
    let m2 = seed_device(&svc, "Планка памяти", "ИНВ-000003").await;
    let n1 = seed_device(&svc, "Планка памяти", "ИНВ-000004").await;
    let plain = seed_device(&svc, "Мышь", "ИНВ-000005").await;
    let g1 = seed_group(&svc, 1, None).await;
    let g2 = seed_group(&svc, 2, Some(g1)).await;
    exec(
        &svc,
        "UPDATE groups SET anchor_device_id = ?1 WHERE id = ?2",
        vec![anchor, g1],
    )
    .await;
    for (d, g) in [(anchor, g1), (m1, g1), (m2, g1), (n1, g2)] {
        exec(
            &svc,
            "INSERT INTO group_devices (device_id, group_id, added_at_utc) \
             VALUES (?1, ?2, 1700000000)",
            vec![d, g],
        )
        .await;
    }
    Fx {
        svc,
        _dir: dir,
        anchor,
        m1,
        m2,
        n1,
        plain,
        g1,
        g2,
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
async fn anchor_list_without_flags_returns_all_five() {
    timed!(async {
        let f = fixture().await;
        let r = f.svc.list(DeviceFilter::default(), page()).await.unwrap();
        assert_eq!(
            sorted_ids(&r.items),
            sorted(vec![f.anchor, f.m1, f.m2, f.n1, f.plain])
        );
        assert_eq!(r.total, 5);
        assert_eq!(r.hidden_member_count, 0, "K без флага равен 0");
    });
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn anchor_list_hide_shows_anchor_and_plain_with_k() {
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
        assert_eq!(sorted_ids(&r.items), sorted(vec![f.anchor, f.plain]));
        assert_eq!(r.total, 2, "total совпадает с числом строк");
        assert_eq!(r.hidden_member_count, 3, "K = m1, m2, n1");
    });
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn anchor_list_anchors_only() {
    timed!(async {
        let f = fixture().await;
        let r = f
            .svc
            .list(
                DeviceFilter {
                    anchors_only: true,
                    ..Default::default()
                },
                page(),
            )
            .await
            .unwrap();
        assert_eq!(sorted_ids(&r.items), vec![f.anchor]);
        assert_eq!(r.total, 1);
    });
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn anchor_list_search_bypasses_hiding_by_construction() {
    timed!(async {
        let f = fixture().await;
        let r = f
            .svc
            .search("ИНВ-000002".to_string(), page())
            .await
            .unwrap();
        assert_eq!(
            sorted_ids(&r.items),
            vec![f.m1],
            "скрытый член найден поиском"
        );
        assert_eq!(r.hidden_member_count, 0);
        // Член вложенной группы — тоже.
        let r = f
            .svc
            .search("ИНВ-000004".to_string(), page())
            .await
            .unwrap();
        assert_eq!(sorted_ids(&r.items), vec![f.n1]);
    });
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn anchor_list_grouped_exclude_anchors_keeps_anchor_out_of_collapse() {
    timed!(async {
        let f = fixture().await;
        let ids_of = |groups: &[trackly_app::dto::device::DeviceGroup], name: &str| -> Vec<i64> {
            let g = groups
                .iter()
                .find(|g| g.repr.name == name)
                .unwrap_or_else(|| panic!("нет набора {name}"));
            sorted(g.ids.iter().map(|i| i64::from(*i)).collect())
        };
        // Без флага: якорь и m1 свёрнуты в один набор (прежнее поведение, пикеры).
        let plain = f
            .svc
            .list_grouped(DeviceFilter::default(), page())
            .await
            .unwrap();
        assert_eq!(
            ids_of(&plain, "Системный блок #4"),
            sorted(vec![f.anchor, f.m1])
        );
        // С флагом: якорь вне свёртки ни как представитель, ни как член.
        let excl = f
            .svc
            .list_grouped(
                DeviceFilter {
                    exclude_anchors: true,
                    ..Default::default()
                },
                page(),
            )
            .await
            .unwrap();
        assert_eq!(ids_of(&excl, "Системный блок #4"), vec![f.m1]);
        assert!(excl.iter().all(|g| g.repr.id != f.anchor));
        // Пикер акта (group_by_condition=true) без флагов видит якорь.
        let picker = f
            .svc
            .list_grouped(
                DeviceFilter {
                    group_by_condition: true,
                    ..Default::default()
                },
                page(),
            )
            .await
            .unwrap();
        assert_eq!(
            ids_of(&picker, "Системный блок #4"),
            sorted(vec![f.anchor, f.m1])
        );
    });
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn anchor_list_member_of_group_returns_subtree_without_anchor() {
    timed!(async {
        let f = fixture().await;
        let r = f
            .svc
            .list(
                DeviceFilter {
                    member_of_group_id: Some(f.g1),
                    ..Default::default()
                },
                page(),
            )
            .await
            .unwrap();
        assert_eq!(sorted_ids(&r.items), sorted(vec![f.m1, f.m2, f.n1]));
        let nested = f
            .svc
            .list(
                DeviceFilter {
                    member_of_group_id: Some(f.g2),
                    ..Default::default()
                },
                page(),
            )
            .await
            .unwrap();
        assert_eq!(sorted_ids(&nested.items), vec![f.n1]);
    });
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn anchor_list_rows_are_enriched_with_group_anchor_and_origin() {
    timed!(async {
        let f = fixture().await;
        let r = f.svc.list(DeviceFilter::default(), page()).await.unwrap();
        let by = |id: i64| r.items.iter().find(|d| d.id == id).unwrap().clone();

        let anchor = by(f.anchor);
        assert_eq!(anchor.anchor_group_id, Some(f.g1));
        assert_eq!(anchor.anchor_group_name.as_deref(), Some("Подмена 1"));
        assert_eq!(anchor.anchor_member_count, Some(3), "m1, m2, n1 без якоря");

        let m1 = by(f.m1);
        assert_eq!(m1.group_id, Some(f.g1));
        assert_eq!(m1.group_name.as_deref(), Some("Подмена 1"));
        assert!(m1.anchor_group_id.is_none() && m1.anchor_member_count.is_none());
        assert_eq!(by(f.n1).group_id, Some(f.g2), "вложенная группа");

        let plain = by(f.plain);
        assert!(plain.group_id.is_none() && plain.group_name.is_none());
        assert!(plain.anchor_group_id.is_none() && plain.origin_group_id.is_none());

        // get и list_by_ids обогащены так же.
        let got = f.svc.get(f.anchor).await.unwrap();
        assert_eq!(got.anchor_member_count, Some(3));
        let many = f.svc.list_by_ids(vec![f.m1, f.plain]).await.unwrap();
        assert_eq!(
            many.iter().find(|d| d.id == f.m1).unwrap().group_id,
            Some(f.g1)
        );

        // repr свёрнутого набора тоже обогащён.
        let groups = f
            .svc
            .list_grouped(DeviceFilter::default(), page())
            .await
            .unwrap();
        let sys = groups
            .iter()
            .find(|g| g.repr.name == "Системный блок #4")
            .unwrap();
        assert_eq!(sys.repr.group_id, Some(f.g1));
    });
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn anchor_list_origin_name_present_then_none_after_group_removed() {
    timed!(async {
        let f = fixture().await;
        let origin = seed_group(&f.svc, 3, None).await;
        exec(
            &f.svc,
            "UPDATE devices SET origin_group_id = ?1 WHERE id = ?2",
            vec![origin, f.plain],
        )
        .await;
        let got = f.svc.get(f.plain).await.unwrap();
        assert_eq!(got.origin_group_id, Some(origin));
        assert_eq!(got.origin_group_name.as_deref(), Some("Подмена 3"));

        // Физическое удаление группы: FK ON DELETE SET NULL.
        exec(&f.svc, "DELETE FROM groups WHERE id = ?1", vec![origin]).await;
        let got = f.svc.get(f.plain).await.unwrap();
        assert!(got.origin_group_id.is_none());
        assert!(got.origin_group_name.is_none());
    });
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn anchor_list_soft_deleted_member_is_not_counted_anywhere() {
    timed!(async {
        let f = fixture().await;
        exec(
            &f.svc,
            "UPDATE devices SET deleted_at_utc = 1700000100 WHERE id = ?1",
            vec![f.n1],
        )
        .await;
        let hid = f
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
        assert_eq!(hid.hidden_member_count, 2, "K без удалённого n1");
        let all = f.svc.list(DeviceFilter::default(), page()).await.unwrap();
        assert!(!all.items.iter().any(|d| d.id == f.n1));
        let a = all.items.iter().find(|d| d.id == f.anchor).unwrap();
        assert_eq!(a.anchor_member_count, Some(2));
    });
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn anchor_list_export_csv_ignores_hiding_flags() {
    timed!(async {
        let f = fixture().await;
        let csv = f
            .svc
            .export_csv(DeviceFilter {
                hide_anchor_members: true,
                anchors_only: true,
                exclude_anchors: true,
                member_of_group_id: Some(f.g2),
                ..Default::default()
            })
            .await
            .unwrap();
        for inv in [
            "ИНВ-000001",
            "ИНВ-000002",
            "ИНВ-000003",
            "ИНВ-000004",
            "ИНВ-000005",
        ] {
            assert!(csv.contains(inv), "выгрузка потеряла {inv}");
        }
    });
}
