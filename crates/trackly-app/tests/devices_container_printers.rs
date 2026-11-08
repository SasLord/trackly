//! Phase 41.9 Plan 04: принтеры в общем списке «Устройства» (D-15, D-16, D-18).
//!
//! Данные вымышленные. Префикс тестов: `container_printers_`.
//!
//! Фикстура: АРМ «АРМ #3» с членами «Рабочая станция #1» (тип 1) и «Принтер #5»
//! (тип 2, USB-хост — станция, член группы). Вне группы: «Принтер #6» (USB-хост —
//! та же станция, членом НЕ является), «Принтер #7» (без строки принтера в
//! `printers`) и «Монитор #1» (добавленный тип id 3). «Принтер #7» в статусе 2.

use std::sync::Arc;
use std::time::Duration;

use rusqlite::params;
use trackly_app::dto::device::{DeviceEntryDto, DeviceFilter, Pagination};
use trackly_app::services::DeviceService;
use trackly_core::primitives::clock::Clock;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::error_conversions::map_rusqlite;
use trackly_infra::test_support::test_writer_and_readers;

const NOW: i64 = 1_700_000_000;

struct Fx {
    svc: DeviceService,
    _dir: tempfile::TempDir,
    c1: i64,
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

/// `type_id: None` — явно: сервер не должен навешивать умолчание по типу.
fn hide_all_types() -> DeviceFilter {
    DeviceFilter {
        type_id: None,
        hide_anchor_members: true,
        ..DeviceFilter::default()
    }
}

async fn seed_device(
    svc: &DeviceService,
    type_id: i64,
    status_id: i64,
    name: &str,
    inv: &str,
) -> i64 {
    let (name, inv) = (name.to_string(), inv.to_string());
    svc.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO devices (type_id, name, inventory_number, status_id, \
                 created_at_utc, updated_at_utc, version) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?5, 1)",
                params![type_id, name, inv, status_id, NOW],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed device")
}

async fn fixture() -> Fx {
    let (writer, readers, dir) = test_writer_and_readers();
    let clock: Arc<dyn Clock + Send + Sync> = Arc::new(SystemClock);
    let svc = DeviceService::new(writer, readers, clock);

    let c1 = svc
        .writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO device_types (id, name) VALUES (3, 'Монитор')",
                [],
            )
            .map_err(map_rusqlite)?;
            conn.execute(
                "INSERT INTO group_types (code, name, behavior, created_at_utc, \
                 updated_at_utc) VALUES ('container_printers_arm', 'АРМ', 'container', ?1, ?1)",
                params![NOW],
            )
            .map_err(map_rusqlite)?;
            let t = conn.last_insert_rowid();
            conn.execute(
                "INSERT INTO groups (type_id, name, seq, version, created_at_utc, \
                 updated_at_utc) VALUES (?1, 'АРМ #3', 1, 1, ?2, ?2)",
                params![t, NOW],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed group");

    let host = seed_device(&svc, 1, 1, "Рабочая станция #1", "ИНВ-000001").await;
    let p_member = seed_device(&svc, 2, 1, "Принтер #5", "ИНВ-000002").await;
    let p_usb = seed_device(&svc, 2, 1, "Принтер #6", "ИНВ-000003").await;
    let _p_free = seed_device(&svc, 2, 2, "Принтер #7", "ИНВ-000004").await;
    let _monitor = seed_device(&svc, 3, 1, "Монитор #1", "ИНВ-000005").await;

    svc.writer
        .execute(move |conn| {
            for d in [host, p_member] {
                conn.execute(
                    "INSERT INTO group_devices (device_id, group_id, added_at_utc) \
                     VALUES (?1, ?2, ?3)",
                    params![d, c1, NOW],
                )
                .map_err(map_rusqlite)?;
            }
            for p in [p_member, p_usb] {
                conn.execute(
                    "INSERT INTO printers (device_id, usb_host_device_id, created_at_utc, \
                     updated_at_utc) VALUES (?1, ?2, ?3, ?3)",
                    params![p, host, NOW],
                )
                .map_err(map_rusqlite)?;
            }
            Ok(())
        })
        .await
        .expect("seed links");
    Fx { svc, _dir: dir, c1 }
}

fn device_names(rows: &[DeviceEntryDto]) -> Vec<String> {
    rows.iter()
        .filter_map(|e| match e {
            DeviceEntryDto::Device { device } => Some(device.name.clone()),
            DeviceEntryDto::Container { .. } => None,
        })
        .collect()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn container_printers_member_hidden_outside_visible_inside() {
    timed!(async {
        let f = fixture().await;
        let r = f
            .svc
            .list_entries(hide_all_types(), page(0, 100))
            .await
            .unwrap();
        let names = device_names(&r.rows);
        assert!(
            !names.contains(&"Принтер #5".to_string()),
            "член скрыт снаружи (D-15)"
        );
        assert!(
            names.contains(&"Принтер #7".to_string()),
            "принтер без группы — строкой"
        );
        assert!(
            names.contains(&"Монитор #1".to_string()),
            "тип id 3 виден при type_id = null"
        );
        // Скрыты: станция и Принтер #5; строк: Принтер #6, #7, Монитор, АРМ.
        assert_eq!(r.hidden_member_count, 2, "K считает принтера-члена");
        assert_eq!(r.total, 4, "total включает принтеры");
        let inside = f.svc.group_children(f.c1, page(0, 100)).await.unwrap();
        assert!(device_names(&inside.rows).contains(&"Принтер #5".to_string()));
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn container_printers_usb_host_without_membership_stays_separate_row() {
    timed!(async {
        let f = fixture().await;
        let r = f
            .svc
            .list_entries(hide_all_types(), page(0, 100))
            .await
            .unwrap();
        let outside = device_names(&r.rows);
        assert!(
            outside.contains(&"Принтер #6".to_string()),
            "USB-хост без членства — отдельная строка (D-16)"
        );
        let inside = f.svc.group_children(f.c1, page(0, 100)).await.unwrap();
        let inside = device_names(&inside.rows);
        assert!(
            !inside.contains(&"Принтер #6".to_string()),
            "в составе не-член отсутствует"
        );
        assert!(
            inside.contains(&"Принтер #5".to_string()),
            "USB-хост + членство — в составе"
        );
        assert!(!outside.contains(&"Принтер #5".to_string()));
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn container_printers_type_filter_is_opt_in_control() {
    timed!(async {
        let f = fixture().await;
        let only_devices = f
            .svc
            .list_entries(
                DeviceFilter {
                    type_id: Some(1),
                    ..hide_all_types()
                },
                page(0, 100),
            )
            .await
            .unwrap();
        let names = device_names(&only_devices.rows);
        assert!(
            names.iter().all(|n| !n.starts_with("Принтер")),
            "при type_id = Some(1) принтеров нет: фильтр опт-ин, умолчания нет"
        );
        let all = f
            .svc
            .list_entries(hide_all_types(), page(0, 100))
            .await
            .unwrap();
        assert!(device_names(&all.rows)
            .iter()
            .any(|n| n.starts_with("Принтер")));
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn container_printers_status_tab_counts_equal_all_tab_rows() {
    timed!(async {
        let f = fixture().await;
        let counts = f.svc.status_counts().await.unwrap();
        let sum: u64 = counts.iter().map(|c| c.count).sum();
        let mut rows = 0u64;
        let mut offset = 0;
        loop {
            let r = f
                .svc
                .list_entries(
                    DeviceFilter {
                        type_id: None,
                        ..DeviceFilter::default()
                    },
                    page(offset, 2),
                )
                .await
                .unwrap();
            rows += r.rows.len() as u64;
            offset += 2;
            if !r.has_more {
                break;
            }
        }
        assert_eq!(sum, 5);
        assert_eq!(rows, sum, "вкладка «Все» = сумма счётчиков статусов");
    })
}
