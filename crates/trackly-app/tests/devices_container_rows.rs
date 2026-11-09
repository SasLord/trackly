//! Phase 41.9 Plan 04: смешанный список устройств и АРМ, блок АРМ и поиск
//! (GRD-07, GRD-08, GRD-10, D-04, D-13, D-14, D-17).
//!
//! Данные вымышленные (репозиторий публичный). Префикс тестов: `container_rows_`.
//!
//! Фикстура: корневой АРМ «АРМ #3» (члены «Монитор #2», «ИБП #1») с вложенным
//! АРМ «АРМ #7» (член «Клавиатура #1»); корневой АРМ «Системный блок #4» (член
//! «Кабель #1»); пустой «АРМ #9»; «АРМ #10» только с пустым вложенным «АРМ #11»;
//! якорная группа с якорем «Корпус #5» и членом «Планка #1»; обычные устройства
//! «Мышь #2» и одноимённое с АРМ устройство «АРМ #3» (создаётся ПОСЛЕДНИМ, id
//! устройства больше id группы — порядок `name, kind, id` отличим от `name, id`).

use std::sync::Arc;
use std::time::Duration;

use rusqlite::params;
use trackly_app::dto::device::{DeviceEntryDto, DeviceFilter, Pagination};
use trackly_app::services::DeviceService;
use trackly_core::error::AppError;
use trackly_core::primitives::clock::Clock;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::error_conversions::map_rusqlite;
use trackly_infra::test_support::test_writer_and_readers;

const NOW: i64 = 1_700_000_000;

struct Fx {
    svc: DeviceService,
    _dir: tempfile::TempDir,
    c1: i64,
    c3: i64,
}

macro_rules! timed {
    ($body:expr) => {
        tokio::time::timeout(Duration::from_secs(60), $body)
            .await
            .expect("таймаут 60 с")
    };
}

fn page(offset: u64, limit: u64) -> Pagination {
    Pagination { offset, limit }
}

fn hide() -> DeviceFilter {
    DeviceFilter {
        hide_anchor_members: true,
        ..DeviceFilter::default()
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
    name: &str,
    seq: i64,
    parent: Option<i64>,
    anchor: Option<i64>,
) -> i64 {
    let name = name.to_string();
    svc.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO groups (type_id, name, seq, parent_group_id, anchor_device_id, \
                 version, created_at_utc, updated_at_utc) \
                 VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6, ?6)",
                params![type_id, name, seq, parent, anchor, NOW],
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

fn new_service() -> (DeviceService, tempfile::TempDir) {
    let (writer, readers, dir) = test_writer_and_readers();
    let clock: Arc<dyn Clock + Send + Sync> = Arc::new(SystemClock);
    (DeviceService::new(writer, readers, clock), dir)
}

async fn fixture() -> Fx {
    let (svc, dir) = new_service();
    let container = seed_type(&svc, "container_rows_arm", "container").await;
    let anchored = seed_type(&svc, "container_rows_sub", "substitute").await;

    // Группы первыми: id группы «АРМ #3» меньше id одноимённого устройства.
    let c1 = seed_group(&svc, container, "АРМ #3", 1, None, None).await;
    let c2 = seed_group(&svc, container, "АРМ #7", 2, Some(c1), None).await;
    let c3 = seed_group(&svc, container, "Системный блок #4", 3, None, None).await;
    let _c4 = seed_group(&svc, container, "АРМ #9", 4, None, None).await;
    let c5 = seed_group(&svc, container, "АРМ #10", 5, None, None).await;
    let _c6 = seed_group(&svc, container, "АРМ #11", 6, Some(c5), None).await;

    let m1 = seed_device(&svc, "Монитор #2", "ИНВ-000001").await;
    let m2 = seed_device(&svc, "ИБП #1", "ИНВ-000002").await;
    let n2 = seed_device(&svc, "Клавиатура #1", "ИНВ-000003").await;
    let k1 = seed_device(&svc, "Кабель #1", "ИНВ-000004").await;
    let a3 = seed_device(&svc, "Корпус #5", "ИНВ-000005").await;
    let x = seed_device(&svc, "Планка #1", "ИНВ-000006").await;
    let _plain = seed_device(&svc, "Мышь #2", "ИНВ-000007").await;
    let _same_name = seed_device(&svc, "АРМ #3", "ИНВ-000008").await;
    let g3 = seed_group(&svc, anchored, "Группа якоря", 7, None, Some(a3)).await;

    for (d, g) in [(m1, c1), (m2, c1), (n2, c2), (k1, c3), (a3, g3), (x, g3)] {
        member(&svc, d, g).await;
    }
    Fx {
        svc,
        _dir: dir,
        c1,
        c3,
    }
}

fn label(e: &DeviceEntryDto) -> (&'static str, String) {
    match e {
        DeviceEntryDto::Device { device } => ("device", device.name.clone()),
        DeviceEntryDto::Container { container } => ("container", container.name.clone()),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn container_rows_order_and_totals_with_same_name() {
    timed!(async {
        let f = fixture().await;
        let r = f.svc.list_entries(hide(), page(0, 100)).await.unwrap();
        let got: Vec<_> = r.rows.iter().map(label).collect();
        assert_eq!(
            got,
            vec![
                ("device", "АРМ #3".to_string()),
                ("container", "АРМ #3".to_string()),
                ("device", "Корпус #5".to_string()),
                ("device", "Мышь #2".to_string()),
                ("container", "Системный блок #4".to_string()),
            ],
            "порядок name, kind, id; пустые и вложенные АРМ отсутствуют (D-17)"
        );
        assert_eq!(r.total, 5);
        assert!(!r.has_more);
        // Скрыты: Монитор, ИБП, Клавиатура, Кабель, Планка.
        assert_eq!(r.hidden_member_count, 5);
        let DeviceEntryDto::Container { container } = &r.rows[1] else {
            panic!("вторая строка — АРМ")
        };
        assert_eq!(container.group_id, f.c1);
        assert_eq!(container.device_count, 3, "члены с вложенными группами");
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn container_rows_pages_sum_to_total_without_repeats() {
    timed!(async {
        let f = fixture().await;
        let full = f.svc.list_entries(hide(), page(0, 100)).await.unwrap();
        for size in [1u64, 2] {
            let mut seen = Vec::new();
            let mut offset = 0;
            loop {
                let r = f
                    .svc
                    .list_entries(hide(), page(offset, size))
                    .await
                    .unwrap();
                assert_eq!(r.total, full.total);
                seen.extend(r.rows.iter().map(label));
                offset += size;
                if !r.has_more {
                    break;
                }
            }
            let expected: Vec<_> = full.rows.iter().map(label).collect();
            assert_eq!(seen.len() as u64, full.total, "сумма страниц = total");
            assert_eq!(seen, expected, "без пропусков и повторов, тот же порядок");
        }
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn container_rows_absent_without_flag_or_with_status() {
    timed!(async {
        let f = fixture().await;
        let plain = f
            .svc
            .list_entries(DeviceFilter::default(), page(0, 100))
            .await
            .unwrap();
        assert!(plain
            .rows
            .iter()
            .all(|e| matches!(e, DeviceEntryDto::Device { .. })));
        assert_eq!(plain.hidden_member_count, 0);
        assert_eq!(plain.total, 8, "все живые устройства");

        let with_status = f
            .svc
            .list_entries(
                DeviceFilter {
                    status_id: Some(1),
                    ..hide()
                },
                page(0, 100),
            )
            .await
            .unwrap();
        assert!(with_status
            .rows
            .iter()
            .all(|e| matches!(e, DeviceEntryDto::Device { .. })));
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn container_rows_badge_equals_k_for_single_container() {
    timed!(async {
        let (svc, _dir) = new_service();
        let container = seed_type(&svc, "container_rows_arm", "container").await;
        let c1 = seed_group(&svc, container, "АРМ #3", 1, None, None).await;
        let c2 = seed_group(&svc, container, "АРМ #7", 2, Some(c1), None).await;
        let a = seed_device(&svc, "Монитор #2", "ИНВ-000001").await;
        let b = seed_device(&svc, "ИБП #1", "ИНВ-000002").await;
        let c = seed_device(&svc, "Клавиатура #1", "ИНВ-000003").await;
        member(&svc, a, c1).await;
        member(&svc, b, c1).await;
        member(&svc, c, c2).await;
        let r = svc.list_containers(hide(), None).await.unwrap();
        assert_eq!(r.containers.len(), 1);
        assert_eq!(r.containers[0].device_count, r.hidden_member_count);
        assert_eq!(r.hidden_member_count, 3);
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn container_rows_list_containers_search() {
    timed!(async {
        let f = fixture().await;
        let all = f.svc.list_containers(hide(), None).await.unwrap();
        let ids: Vec<i64> = all.containers.iter().map(|c| c.group_id).collect();
        assert_eq!(ids, vec![f.c1, f.c3], "корневые непустые по name, id");
        assert_eq!(all.hidden_member_count, 5);
        assert!(!all.truncated);

        let arm = f
            .svc
            .list_containers(hide(), Some("арм".into()))
            .await
            .unwrap();
        assert_eq!(arm.containers.len(), 1);
        assert_eq!(arm.containers[0].name, "АРМ #3");

        let block = f
            .svc
            .list_containers(hide(), Some("блок".into()))
            .await
            .unwrap();
        assert_eq!(
            block
                .containers
                .iter()
                .map(|c| c.group_id)
                .collect::<Vec<_>>(),
            vec![f.c3],
            "«Системный блок #4» — группа без якоря"
        );

        let none = f
            .svc
            .list_containers(hide(), Some("#".into()))
            .await
            .unwrap();
        assert!(none.containers.is_empty(), "нет токенов — пусто");

        let no_hide = f
            .svc
            .list_containers(DeviceFilter::default(), None)
            .await
            .unwrap();
        assert_eq!(
            no_hide.hidden_member_count, 0,
            "K только при включённом скрытии"
        );
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn container_rows_block_is_truncated_with_flag() {
    timed!(async {
        let (svc, _dir) = new_service();
        let container = seed_type(&svc, "container_rows_arm", "container").await;
        svc.writer
            .execute(move |conn| {
                for i in 0..1001_i64 {
                    conn.execute(
                        "INSERT INTO groups (type_id, name, seq, version, created_at_utc, \
                         updated_at_utc) VALUES (?1, ?2, ?3, 1, ?4, ?4)",
                        params![container, format!("АРМ #{i}"), i + 1, NOW],
                    )
                    .map_err(map_rusqlite)?;
                    let gid = conn.last_insert_rowid();
                    conn.execute(
                        "INSERT INTO devices (type_id, name, inventory_number, status_id, \
                         created_at_utc, updated_at_utc, version) \
                         VALUES (1, ?1, ?2, 1, ?3, ?3, 1)",
                        params![format!("Мышь #{i}"), format!("ИНВ-{i}"), NOW],
                    )
                    .map_err(map_rusqlite)?;
                    let did = conn.last_insert_rowid();
                    conn.execute(
                        "INSERT INTO group_devices (device_id, group_id, added_at_utc) \
                         VALUES (?1, ?2, ?3)",
                        params![did, gid, NOW],
                    )
                    .map_err(map_rusqlite)?;
                }
                Ok(())
            })
            .await
            .expect("bulk seed");
        let r = svc.list_containers(hide(), None).await.unwrap();
        assert_eq!(r.containers.len(), 1000);
        assert!(r.truncated, "усечение не молчаливое");
    })
}

/// Фильтр ровно в том виде, в каком его шлёт клиент в режиме поиска
/// (кейс `deviceListFlags containers: как flat, поиск` в фикстуре device-list-mode).
fn client_search_filter(overrides: serde_json::Value) -> DeviceFilter {
    let mut v = serde_json::json!({
        "type_id": null,
        "place_id": null,
        "status_id": null,
        "state": null,
        "name_prefix": null,
        "include_deleted": false,
        "group_by_condition": false,
        "hide_anchor_members": false,
        "anchors_only": false,
        "exclude_anchors": false
    });
    for (k, val) in overrides.as_object().unwrap() {
        v[k] = val.clone();
    }
    serde_json::from_value(v).expect("фильтр клиента")
}

/// G-3 (D-14): поиск шлёт hide_anchor_members=false, но строки АРМ по имени нужны.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn container_rows_search_with_client_flags() {
    timed!(async {
        let f = fixture().await;
        let flt = || client_search_filter(serde_json::json!({}));
        let arm = f
            .svc
            .list_containers(flt(), Some("арм".into()))
            .await
            .unwrap();
        assert_eq!(arm.containers.len(), 1);
        assert_eq!(arm.containers[0].name, "АРМ #3");
        assert_eq!(arm.hidden_member_count, 0, "в поиске K не включается");

        let block = f
            .svc
            .list_containers(flt(), Some("блок".into()))
            .await
            .unwrap();
        assert_eq!(block.containers.len(), 1);
        assert_eq!(block.containers[0].name, "Системный блок #4");

        let none = f
            .svc
            .list_containers(flt(), Some("#".into()))
            .await
            .unwrap();
        assert!(none.containers.is_empty());

        // Без запроса (или пробелы) при hide=false строк АРМ нет.
        for q in [None, Some("   ".to_string())] {
            let r = f.svc.list_containers(flt(), q).await.unwrap();
            assert!(r.containers.is_empty());
        }

        // Вкладка статуса, якорный запрос и «состав группы» строк АРМ не дают.
        for o in [
            serde_json::json!({"status_id": 1}),
            serde_json::json!({"anchors_only": true}),
            serde_json::json!({"member_of_group_id": f.c1}),
        ] {
            let r = f
                .svc
                .list_containers(client_search_filter(o), Some("арм".into()))
                .await
                .unwrap();
            assert!(r.containers.is_empty());
        }
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn container_rows_limit_over_200_is_validation_error() {
    timed!(async {
        let f = fixture().await;
        let err = f.svc.list_entries(hide(), page(0, 201)).await.unwrap_err();
        assert!(matches!(err, AppError::Validation { .. }));
    })
}
