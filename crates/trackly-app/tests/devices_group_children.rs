//! Phase 41.9 Plan 04: состав группы (прямые дети), GRD-10, D-11, D-15, D-16, D-17.
//!
//! Данные вымышленные. Префикс тестов: `group_children_`.
//!
//! Фикстура: АРМ «АРМ #3» с членами «Монитор #2» и принтером «Принтер #1»;
//! вложенная якорная группа (якорь «Корпус #4», член «Мышь #1»); вложенный АРМ
//! «АРМ #7» (член «Клавиатура #1»); пустой вложенный АРМ «АРМ #8». Принтер
//! «Принтер #2» связан с АРМ только USB-хостом (хост — «Монитор #2») и членом
//! группы не является.

use std::sync::Arc;
use std::time::Duration;

use rusqlite::params;
use trackly_app::dto::device::{DeviceEntryDto, Pagination};
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
    g2: i64,
    c2: i64,
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
    name: &str,
    seq: i64,
    parent: Option<i64>,
    anchor: Option<i64>,
    deleted: bool,
) -> i64 {
    let name = name.to_string();
    svc.writer
        .execute(move |conn| {
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

async fn printer_row(svc: &DeviceService, device: i64, usb_host: i64) {
    svc.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO printers (device_id, usb_host_device_id, created_at_utc, \
                 updated_at_utc) VALUES (?1, ?2, ?3, ?3)",
                params![device, usb_host, NOW],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("printer row");
}

async fn fixture() -> Fx {
    let (writer, readers, dir) = test_writer_and_readers();
    let clock: Arc<dyn Clock + Send + Sync> = Arc::new(SystemClock);
    let svc = DeviceService::new(writer, readers, clock);
    let container = seed_type(&svc, "group_children_arm", "container").await;
    let anchored = seed_type(&svc, "group_children_sub", "substitute").await;

    let monitor = seed_device(&svc, 1, "Монитор #2", "ИНВ-000001").await;
    let p1 = seed_device(&svc, 2, "Принтер #1", "ИНВ-000002").await;
    let p2 = seed_device(&svc, 2, "Принтер #2", "ИНВ-000003").await;
    let a2 = seed_device(&svc, 1, "Корпус #4", "ИНВ-000004").await;
    let n1 = seed_device(&svc, 1, "Мышь #1", "ИНВ-000005").await;
    let n2 = seed_device(&svc, 1, "Клавиатура #1", "ИНВ-000006").await;

    let c1 = seed_group(&svc, container, "АРМ #3", 1, None, None, false).await;
    let g2 = seed_group(&svc, anchored, "Группа якоря", 2, Some(c1), Some(a2), false).await;
    let c2 = seed_group(&svc, container, "АРМ #7", 3, Some(c1), None, false).await;
    let _c3 = seed_group(&svc, container, "АРМ #8", 4, Some(c1), None, false).await;

    for (d, g) in [(monitor, c1), (p1, c1), (a2, g2), (n1, g2), (n2, c2)] {
        member(&svc, d, g).await;
    }
    printer_row(&svc, p1, monitor).await;
    printer_row(&svc, p2, monitor).await;
    Fx {
        svc,
        _dir: dir,
        c1,
        g2,
        c2,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn group_children_direct_children_only() {
    timed!(async {
        let f = fixture().await;
        let r = f.svc.group_children(f.c1, page(0, 100)).await.unwrap();
        let mut names = Vec::new();
        for row in &r.rows {
            match row {
                DeviceEntryDto::Device { device } => {
                    if device.name == "Корпус #4" {
                        assert_eq!(device.anchor_group_id, Some(f.g2), "якорь вложенной группы");
                    }
                    names.push(("device", device.name.clone()));
                }
                DeviceEntryDto::Container { container } => {
                    assert_eq!(container.group_id, f.c2);
                    assert_eq!(container.device_count, 1);
                    names.push(("container", container.name.clone()));
                }
            }
        }
        assert_eq!(
            names,
            vec![
                ("container", "АРМ #7".to_string()),
                ("device", "Корпус #4".to_string()),
                ("device", "Монитор #2".to_string()),
                ("device", "Принтер #1".to_string()),
            ],
            "пустой вложенный АРМ #8 и принтер-не-член отсутствуют"
        );
        assert_eq!(r.total, 4);
        assert!(!r.has_more);
        assert_eq!(r.hidden_member_count, 0);
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn group_children_pagination_total_and_has_more() {
    timed!(async {
        let f = fixture().await;
        let first = f.svc.group_children(f.c1, page(0, 2)).await.unwrap();
        assert_eq!(first.rows.len(), 2);
        assert_eq!(first.total, 4);
        assert!(first.has_more);
        let second = f.svc.group_children(f.c1, page(2, 2)).await.unwrap();
        assert_eq!(second.rows.len(), 2);
        assert!(!second.has_more);
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn group_children_limit_over_200_is_validation_error() {
    timed!(async {
        let f = fixture().await;
        let err = f.svc.group_children(f.c1, page(0, 201)).await.unwrap_err();
        assert!(matches!(err, AppError::Validation { .. }));
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn group_children_deleted_or_missing_group_is_empty() {
    timed!(async {
        let f = fixture().await;
        let gid = f.c1;
        f.svc
            .writer
            .execute(move |conn| {
                conn.execute(
                    "UPDATE groups SET deleted_at_utc = ?2 WHERE id = ?1",
                    params![gid, NOW],
                )
                .map_err(map_rusqlite)?;
                Ok(())
            })
            .await
            .expect("soft delete");
        let deleted = f.svc.group_children(f.c1, page(0, 100)).await.unwrap();
        assert!(deleted.rows.is_empty(), "удалённая группа — пустой ответ");
        assert_eq!(deleted.total, 0);
        let missing = f.svc.group_children(9_999, page(0, 100)).await.unwrap();
        assert!(missing.rows.is_empty());
        assert_eq!(missing.total, 0);
    })
}
