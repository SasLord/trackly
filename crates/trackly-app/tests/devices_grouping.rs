//! Интеграционные тесты группировки не-уникальных устройств (Plan 04, Task 1).
//!
//! Тесты покрывают:
//! - Сжатие одинаковых устройств в группу с count
//! - Уникальные устройства (с inventory/serial) НЕ попадают в группированный список
//! - empty-string нормализация (Pitfall #12)
//! - Фильтр по статусу внутри групп

use std::sync::Arc;
use std::time::Duration;

use trackly_app::dto::device::{DeviceFilter, DeviceNew, Pagination};
use trackly_app::services::DeviceService;
use trackly_core::primitives::clock::Clock;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::test_support::test_writer_and_readers;

fn make_service() -> (DeviceService, tempfile::TempDir) {
    let (writer, readers, dir) = test_writer_and_readers();
    let clock: Arc<dyn Clock + Send + Sync> = Arc::new(SystemClock);
    let svc = DeviceService::new(writer, readers, clock);
    (svc, dir)
}

fn non_unique_device(name: &str, status_id: i64) -> DeviceNew {
    DeviceNew {
        type_id: 1,
        name: name.to_string(),
        inventory_no: None, // no unique identifiers
        serial_no: None,
        model: Some("Model X".to_string()),
        specs: None,
        kit: None,
        state: None,
        location: None,
        location_id: None,
        status_id,
    }
}

// ---------------------------------------------------------------------------
// grouping_collapses_non_unique
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn grouping_collapses_non_unique() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();

        // Create 5 identical devices without inventory_no/serial_no.
        for _ in 0..5 {
            svc.create(non_unique_device("Бумага A4", 1))
                .await
                .expect("create");
        }

        let filter = DeviceFilter::default();
        let page = Pagination { offset: 0, limit: 50 };
        let groups = svc.list_grouped(filter, page).await.expect("list_grouped");

        assert_eq!(groups.len(), 1, "5 одинаковых устройств должны схлопнуться в 1 группу");
        assert_eq!(groups[0].count, 5, "count должен быть 5");
        assert_eq!(groups[0].ids.len(), 5, "ids должен содержать 5 элементов");
    })
    .await
    .expect("grouping_collapses_non_unique exceeded 30s");
}

// ---------------------------------------------------------------------------
// grouping_groups_devices_with_same_name_even_if_inventory_set
// ---------------------------------------------------------------------------
// New behaviour: devices with inventory_no ARE included in groups when they
// share the same (name, model, ...) key. The old test "grouping_keeps_unique_separate"
// assumed the opposite — it has been replaced by this test.

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn grouping_groups_devices_with_same_name_even_if_inventory_set() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();

        // 3 devices with the same name but different inventory_no values.
        for i in 1..=3i32 {
            let mut new = non_unique_device("Ноутбук Lenovo X1", 1);
            new.inventory_no = Some(format!("LEN-{i:03}"));
            svc.create(new).await.expect("create with inv");
        }

        let filter = DeviceFilter::default();
        let page = Pagination { offset: 0, limit: 50 };
        let groups = svc.list_grouped(filter, page).await.expect("list_grouped");

        // All 3 share the same (name, model, ...) key → 1 group with count=3.
        assert_eq!(
            groups.len(),
            1,
            "3 устройства с одинаковым именем должны схлопнуться в 1 группу, получили {} групп",
            groups.len()
        );
        assert_eq!(groups[0].count, 3, "count в группе должен быть 3");
        assert_eq!(groups[0].ids.len(), 3, "ids должен содержать 3 элемента");
    })
    .await
    .expect("grouping_groups_devices_with_same_name_even_if_inventory_set exceeded 30s");
}

// ---------------------------------------------------------------------------
// grouping_singleton_included (count == 1 device appears as its own group)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn grouping_singleton_included() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();

        // A single device with a unique inventory_no — should appear as a group of count=1.
        let mut new = non_unique_device("Монитор Dell U2722D", 1);
        new.inventory_no = Some("MON-001".to_string());
        svc.create(new).await.expect("create singleton");

        let filter = DeviceFilter::default();
        let page = Pagination { offset: 0, limit: 50 };
        let groups = svc.list_grouped(filter, page).await.expect("list_grouped");

        assert_eq!(
            groups.len(), 1,
            "singleton устройство должно появляться как группа count=1, получили {} групп",
            groups.len()
        );
        assert_eq!(groups[0].count, 1, "count группы должен быть 1");
        assert_eq!(groups[0].ids.len(), 1, "ids должен содержать 1 элемент");
    })
    .await
    .expect("grouping_singleton_included exceeded 30s");
}

// ---------------------------------------------------------------------------
// grouping_handles_empty_string_as_null (Pitfall #12)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn grouping_handles_empty_string_as_null() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();

        // Device with inventory_no = "" (empty string).
        // Backend normalizes empty → NULL on INSERT, so it should appear in grouped list.
        let mut new = non_unique_device("Карандаш", 1);
        new.inventory_no = Some("".to_string()); // empty string — normalized to NULL
        svc.create(new).await.expect("create with empty inventory_no");

        let filter = DeviceFilter::default();
        let page = Pagination { offset: 0, limit: 50 };
        let groups = svc.list_grouped(filter, page).await.expect("list_grouped");

        assert_eq!(
            groups.len(),
            1,
            "устройство с пустым inventory_no должно попадать в группу (Pitfall #12)"
        );
    })
    .await
    .expect("grouping_handles_empty_string_as_null exceeded 30s");
}

// ---------------------------------------------------------------------------
// grouping_with_status_filter
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn grouping_with_status_filter() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();

        // Create 3 devices with status=1 (На складе).
        for _ in 0..3 {
            svc.create(non_unique_device("Флешка 16GB", 1))
                .await
                .expect("create status=1");
        }
        // Create 2 devices with status=2 (В работе).
        for _ in 0..2 {
            svc.create(non_unique_device("Флешка 16GB", 2))
                .await
                .expect("create status=2");
        }

        // Filter by status_id=2 — only 2 devices with status В работе.
        let filter = DeviceFilter {
            status_id: Some(2),
            ..Default::default()
        };
        let page = Pagination { offset: 0, limit: 50 };
        let groups = svc.list_grouped(filter, page).await.expect("list_grouped with status");

        assert_eq!(groups.len(), 1, "должна быть 1 группа с status=2");
        assert_eq!(groups[0].count, 2, "count в группе должен быть 2");
    })
    .await
    .expect("grouping_with_status_filter exceeded 30s");
}

// ---------------------------------------------------------------------------
// status_counts_returns_correct_counts
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn status_counts_returns_correct_counts() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();

        // Create 3 devices with status=1, 2 with status=2.
        for _ in 0..3 {
            svc.create(non_unique_device("Тест", 1)).await.expect("create status=1");
        }
        for _ in 0..2 {
            svc.create(non_unique_device("Тест", 2)).await.expect("create status=2");
        }

        let counts = svc.status_counts().await.expect("status_counts");
        let map: std::collections::HashMap<i64, u64> =
            counts.iter().map(|sc| (sc.status_id, sc.count)).collect();

        assert_eq!(map.get(&1), Some(&3), "status_id=1 должен иметь count=3");
        assert_eq!(map.get(&2), Some(&2), "status_id=2 должен иметь count=2");
    })
    .await
    .expect("status_counts_returns_correct_counts exceeded 30s");
}

// ---------------------------------------------------------------------------
// list_by_ids_returns_correct_devices
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn list_by_ids_returns_correct_devices() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();

        let d1 = svc.create(non_unique_device("Устройство 1", 1)).await.expect("create 1");
        let d2 = svc.create(non_unique_device("Устройство 2", 1)).await.expect("create 2");
        let d3 = svc.create(non_unique_device("Устройство 3", 1)).await.expect("create 3");

        let ids = vec![d1.id, d3.id]; // skip d2
        let result = svc.list_by_ids(ids).await.expect("list_by_ids");

        assert_eq!(result.len(), 2, "должно вернуть 2 устройства");
        let names: Vec<&str> = result.iter().map(|d| d.name.as_str()).collect();
        assert!(names.contains(&"Устройство 1"));
        assert!(names.contains(&"Устройство 3"));
        assert!(!names.contains(&"Устройство 2"));

        let _ = d2; // suppress unused warning
    })
    .await
    .expect("list_by_ids_returns_correct_devices exceeded 30s");
}

// ---------------------------------------------------------------------------
// grouping_singleton_includes_inventory_and_serial_no
// ---------------------------------------------------------------------------
// Regression: list_grouped SQL previously omitted inventory_number/serial_number
// from SELECT, so repr.inventory_no and repr.serial_no were always None.
// For count==1 groups the frontend renders a plain DeviceListRow, so «—» was
// shown instead of the actual values.

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn grouping_singleton_includes_inventory_and_serial_no() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();

        let mut new = non_unique_device("Singleton X", 1);
        new.inventory_no = Some("INV-1".to_string());
        new.serial_no = Some("SN-1".to_string());
        svc.create(new).await.expect("create singleton");

        let filter = DeviceFilter::default();
        let page = Pagination { offset: 0, limit: 50 };
        let groups = svc.list_grouped(filter, page).await.expect("list_grouped");

        assert_eq!(groups.len(), 1, "должна быть 1 группа (singleton)");
        let g = &groups[0];
        assert_eq!(g.count, 1, "count должен быть 1");
        assert_eq!(
            g.repr.inventory_no.as_deref(),
            Some("INV-1"),
            "repr.inventory_no должен быть INV-1, получен {:?}",
            g.repr.inventory_no
        );
        assert_eq!(
            g.repr.serial_no.as_deref(),
            Some("SN-1"),
            "repr.serial_no должен быть SN-1, получен {:?}",
            g.repr.serial_no
        );
    })
    .await
    .expect("grouping_singleton_includes_inventory_and_serial_no exceeded 30s");
}

// ---------------------------------------------------------------------------
// grouping_collapsed_group_aggregates_inv_serial_safely
// ---------------------------------------------------------------------------
// For count>1 groups the UI hides inv/serial via colspan. The aggregated value
// must be Some (not None) — the guard here is just that the column is present
// and non-null for at least one device.

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn grouping_collapsed_group_aggregates_inv_serial_safely() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();

        for letter in &["A", "B", "C"] {
            let mut new = non_unique_device("Multi", 1);
            new.inventory_no = Some(letter.to_string());
            svc.create(new).await.expect("create");
        }

        let filter = DeviceFilter::default();
        let page = Pagination { offset: 0, limit: 50 };
        let groups = svc.list_grouped(filter, page).await.expect("list_grouped");

        assert_eq!(groups.len(), 1, "3 одинаковых устройства → 1 группа");
        let g = &groups[0];
        assert_eq!(g.count, 3, "count должен быть 3");
        // MAX("A","B","C") = "C" in SQLite; just assert Some (not None).
        assert!(
            g.repr.inventory_no.is_some(),
            "repr.inventory_no должен быть Some для группы с inv_no у членов, получен None"
        );
    })
    .await
    .expect("grouping_collapsed_group_aggregates_inv_serial_safely exceeded 30s");
}

// ---------------------------------------------------------------------------
// grouping_multiple_distinct_groups
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn grouping_multiple_distinct_groups() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();

        // 3 devices named "Флешка 8GB".
        for _ in 0..3 {
            svc.create(non_unique_device("Флешка 8GB", 1)).await.expect("create");
        }
        // 2 devices named "Флешка 16GB".
        for _ in 0..2 {
            svc.create(non_unique_device("Флешка 16GB", 1)).await.expect("create");
        }

        let filter = DeviceFilter::default();
        let page = Pagination { offset: 0, limit: 50 };
        let groups = svc.list_grouped(filter, page).await.expect("list_grouped");

        assert_eq!(groups.len(), 2, "должно быть 2 группы (8GB и 16GB)");
        let counts: std::collections::HashMap<&str, u64> = groups
            .iter()
            .map(|g| (g.repr.name.as_str(), g.count))
            .collect();
        assert_eq!(counts.get("Флешка 8GB"), Some(&3));
        assert_eq!(counts.get("Флешка 16GB"), Some(&2));
    })
    .await
    .expect("grouping_multiple_distinct_groups exceeded 30s");
}
