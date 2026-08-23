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
use trackly_core::domain::places::{PlaceKind, PlaceNew};
use trackly_core::ports::places::PlaceRepository;
use trackly_core::primitives::clock::Clock;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::repos::SqlitePlaceRepository;
use trackly_infra::test_support::test_writer_and_readers;

fn make_service() -> (DeviceService, tempfile::TempDir) {
    let (writer, readers, dir) = test_writer_and_readers();
    let clock: Arc<dyn Clock + Send + Sync> = Arc::new(SystemClock);
    let svc = DeviceService::new(writer, readers, clock);
    (svc, dir)
}

/// Создаёт корневое место (kind=Room) напрямую через `SqlitePlaceRepository`
/// на writer-соединении сервиса — фикстура-заместитель прежнего свободнотекстового
/// `location`, невозможного больше на write-пути (D-18).
async fn create_place(svc: &DeviceService, name: &str) -> i64 {
    let name = name.to_string();
    svc.writer
        .execute(move |conn| {
            let repo = SqlitePlaceRepository;
            let new_place = PlaceNew {
                parent_id: None,
                kind: PlaceKind::Room,
                name: name.clone(),
                level: None,
                is_storage: false,
                sort_order: None,
                notes: None,
            };
            repo.create(conn, &new_place, 1_700_000_000)
        })
        .await
        .expect("create place")
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
        place_id: None,
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
        let page = Pagination {
            offset: 0,
            limit: 50,
        };
        let groups = svc.list_grouped(filter, page).await.expect("list_grouped");

        assert_eq!(
            groups.len(),
            1,
            "5 одинаковых устройств должны схлопнуться в 1 группу"
        );
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
        let page = Pagination {
            offset: 0,
            limit: 50,
        };
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
        let page = Pagination {
            offset: 0,
            limit: 50,
        };
        let groups = svc.list_grouped(filter, page).await.expect("list_grouped");

        assert_eq!(
            groups.len(),
            1,
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
        svc.create(new)
            .await
            .expect("create with empty inventory_no");

        let filter = DeviceFilter::default();
        let page = Pagination {
            offset: 0,
            limit: 50,
        };
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
        let page = Pagination {
            offset: 0,
            limit: 50,
        };
        let groups = svc
            .list_grouped(filter, page)
            .await
            .expect("list_grouped with status");

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
            svc.create(non_unique_device("Тест", 1))
                .await
                .expect("create status=1");
        }
        for _ in 0..2 {
            svc.create(non_unique_device("Тест", 2))
                .await
                .expect("create status=2");
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

        let d1 = svc
            .create(non_unique_device("Устройство 1", 1))
            .await
            .expect("create 1");
        let d2 = svc
            .create(non_unique_device("Устройство 2", 1))
            .await
            .expect("create 2");
        let d3 = svc
            .create(non_unique_device("Устройство 3", 1))
            .await
            .expect("create 3");

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
        let page = Pagination {
            offset: 0,
            limit: 50,
        };
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
        let page = Pagination {
            offset: 0,
            limit: 50,
        };
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
            svc.create(non_unique_device("Флешка 8GB", 1))
                .await
                .expect("create");
        }
        // 2 devices named "Флешка 16GB".
        for _ in 0..2 {
            svc.create(non_unique_device("Флешка 16GB", 1))
                .await
                .expect("create");
        }

        let filter = DeviceFilter::default();
        let page = Pagination {
            offset: 0,
            limit: 50,
        };
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

// ---------------------------------------------------------------------------
// grouping_groups_devices_with_same_name_and_different_status
// ---------------------------------------------------------------------------
// Round 8: group key relaxed to (type_id, name). Two devices with the same
// Наименование but different status_id must collapse into one group.

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn grouping_groups_devices_with_same_name_and_different_status() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();

        // One monitor with status=1 (На складе), another with status=2 (В работе).
        svc.create(non_unique_device("Монитор", 1))
            .await
            .expect("create status=1");
        svc.create(non_unique_device("Монитор", 2))
            .await
            .expect("create status=2");

        let filter = DeviceFilter::default();
        let page = Pagination {
            offset: 0,
            limit: 50,
        };
        let groups = svc.list_grouped(filter, page).await.expect("list_grouped");

        assert_eq!(
            groups.len(),
            1,
            "два монитора с разными статусами должны схлопнуться в 1 группу, получили {} групп",
            groups.len()
        );
        assert_eq!(groups[0].count, 2, "count в группе должен быть 2");
        assert_eq!(groups[0].ids.len(), 2, "ids должен содержать 2 элемента");
    })
    .await
    .expect("grouping_groups_devices_with_same_name_and_different_status exceeded 30s");
}

// ---------------------------------------------------------------------------
// grouping_groups_devices_with_same_name_and_different_location
// ---------------------------------------------------------------------------
// Two devices with the same Наименование but different locations must group.

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn grouping_groups_devices_with_same_name_and_different_location() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();

        let place_a = create_place(&svc, "Кабинет 305").await;
        let place_b = create_place(&svc, "Склад").await;

        let mut d1 = non_unique_device("Монитор Dell", 1);
        d1.place_id = Some(place_a);

        let mut d2 = non_unique_device("Монитор Dell", 1);
        d2.place_id = Some(place_b);

        svc.create(d1).await.expect("create place=Кабинет 305");
        svc.create(d2).await.expect("create place=Склад");

        let filter = DeviceFilter::default();
        let page = Pagination {
            offset: 0,
            limit: 50,
        };
        let groups = svc.list_grouped(filter, page).await.expect("list_grouped");

        assert_eq!(
            groups.len(),
            1,
            "два монитора с разными локациями должны схлопнуться в 1 группу, получили {} групп",
            groups.len()
        );
        assert_eq!(groups[0].count, 2, "count в группе должен быть 2");
    })
    .await
    .expect("grouping_groups_devices_with_same_name_and_different_location exceeded 30s");
}

// ---------------------------------------------------------------------------
// grouping_groups_devices_with_same_name_and_model_ignores_condition
// ---------------------------------------------------------------------------
// D-05 (Phase 18): two devices with the same Наименование+model but different
// condition/state must now COLLAPSE into ONE group — condition is no longer
// part of the true-branch group key (model is). condition_distinct_count
// signals the mixed condition for frontend drill-in (D-07).
// Was: grouping_groups_devices_with_same_name_and_different_condition
// (asserted the pre-Phase-18 condition-splits-groups behaviour, DEF-2B/ITEM-1).

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn grouping_groups_devices_with_same_name_and_model_ignores_condition() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();

        let mut d1 = non_unique_device("Клавиатура", 1);
        d1.state = Some("Новое".to_string());

        let mut d2 = non_unique_device("Клавиатура", 1);
        d2.state = Some("Б/У".to_string());

        svc.create(d1).await.expect("create condition=Новое");
        svc.create(d2).await.expect("create condition=Б/У");

        let filter = DeviceFilter {
            group_by_condition: true,
            ..Default::default()
        };
        let page = Pagination {
            offset: 0,
            limit: 50,
        };
        let groups = svc.list_grouped(filter, page).await.expect("list_grouped");

        assert_eq!(
            groups.len(),
            1,
            "D-05: одинаковый name+model, разный condition → ОДНА группа, получили {} групп",
            groups.len()
        );
        assert_eq!(groups[0].count, 2, "count группы должен быть 2");
        assert_eq!(
            groups[0].condition_distinct_count, 2,
            "condition_distinct_count должен сигнализировать смешанный condition (D-07)"
        );
    })
    .await
    .expect("grouping_groups_devices_with_same_name_and_model_ignores_condition exceeded 30s");
}

// ---------------------------------------------------------------------------
// model_key_splits_groups_condition_does_not (D-04/D-05, Phase 18)
// ---------------------------------------------------------------------------
// Два устройства с одинаковым name+model, но разным condition → ОДНА группа
// (condition больше не входит в ключ группировки true-ветки), condition_distinct_count
// сигнализирует о смешанном condition для drill-in (D-07).
// Was: condition_key_splits_groups (asserted old condition-key-splits behaviour).

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn model_key_splits_groups_condition_does_not() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();

        let mut d1 = non_unique_device("DVD-ROM ASUS", 1);
        d1.model = Some("BW-16D1HT".to_string());
        d1.state = Some("Новое".to_string());

        let mut d2 = non_unique_device("DVD-ROM ASUS", 1);
        d2.model = Some("BW-16D1HT".to_string());
        d2.state = Some("Хорошее".to_string());

        svc.create(d1).await.expect("create condition=Новое");
        svc.create(d2).await.expect("create condition=Хорошее");

        let filter = DeviceFilter {
            group_by_condition: true,
            ..Default::default()
        };
        let page = Pagination {
            offset: 0,
            limit: 50,
        };
        let groups = svc.list_grouped(filter, page).await.expect("list_grouped");

        assert_eq!(
            groups.len(),
            1,
            "D-05: одинаковый name+model, разный condition → ОДНА группа, получили {} групп",
            groups.len()
        );
        assert_eq!(groups[0].count, 2, "count группы должен быть 2");
        assert_eq!(
            groups[0].condition_distinct_count, 2,
            "condition_distinct_count должен сигнализировать смешанный condition (D-07)"
        );
    })
    .await
    .expect("model_key_splits_groups_condition_does_not exceeded 30s");
}

// ---------------------------------------------------------------------------
// device_with_condition helper
// ---------------------------------------------------------------------------

fn device_with_condition(name: &str, condition: &str) -> DeviceNew {
    DeviceNew {
        type_id: 1,
        name: name.to_string(),
        inventory_no: None,
        serial_no: None,
        model: Some("Model X".to_string()),
        specs: None,
        kit: None,
        state: Some(condition.to_string()),
        place_id: None,
        status_id: 1,
    }
}

// ---------------------------------------------------------------------------
// grouping_page_mode_ignores_condition (ITEM-1a)
// ---------------------------------------------------------------------------
// group_by_condition=false + два устройства с разным condition → 1 группа,
// condition_distinct_count == 2.

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn grouping_page_mode_ignores_condition() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();

        svc.create(device_with_condition("Клавиатура", "Новое"))
            .await
            .expect("create condition=Новое");
        svc.create(device_with_condition("Клавиатура", "Б/У"))
            .await
            .expect("create condition=Б/У");

        let filter = DeviceFilter {
            group_by_condition: false,
            ..Default::default()
        };
        let page = Pagination {
            offset: 0,
            limit: 50,
        };
        let groups = svc.list_grouped(filter, page).await.expect("list_grouped");

        assert_eq!(
            groups.len(),
            1,
            "ITEM-1a: group_by_condition=false → разные condition схлопываются в 1 группу, получили {} групп",
            groups.len()
        );
        assert_eq!(
            groups[0].condition_distinct_count, 2,
            "ITEM-1a: condition_distinct_count должен быть 2 для смешанной группы"
        );
    })
    .await
    .expect("grouping_page_mode_ignores_condition exceeded 30s");
}

// ---------------------------------------------------------------------------
// grouping_act_form_groups_by_name_and_model_not_condition (D-04/D-05, Phase 18)
// ---------------------------------------------------------------------------
// group_by_condition=true + два устройства с одинаковым name+model (оба None),
// но разным condition → ОДНА группа (condition больше не входит в ключ).
// Was: grouping_act_form_keeps_condition_split (asserted old 2-groups behaviour).

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn grouping_act_form_groups_by_name_and_model_not_condition() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();

        svc.create(device_with_condition("Клавиатура", "Новое"))
            .await
            .expect("create condition=Новое");
        svc.create(device_with_condition("Клавиатура", "Б/У"))
            .await
            .expect("create condition=Б/У");

        let filter = DeviceFilter {
            group_by_condition: true,
            ..Default::default()
        };
        let page = Pagination {
            offset: 0,
            limit: 50,
        };
        let groups = svc.list_grouped(filter, page).await.expect("list_grouped");

        assert_eq!(
            groups.len(),
            1,
            "D-05: одинаковый name+model (оба None), разный condition → ОДНА группа, получили {} групп",
            groups.len()
        );
        assert_eq!(groups[0].count, 2, "count группы должен быть 2");
    })
    .await
    .expect("grouping_act_form_groups_by_name_and_model_not_condition exceeded 30s");
}

// ---------------------------------------------------------------------------
// grouping_condition_distinct_count_mixed (ITEM-1c)
// ---------------------------------------------------------------------------
// group_by_condition=false + два устройства (Новое + Б/У) → condition_distinct_count > 1.

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn grouping_condition_distinct_count_mixed() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();

        svc.create(device_with_condition("Монитор", "Новое"))
            .await
            .expect("create condition=Новое");
        svc.create(device_with_condition("Монитор", "Б/У"))
            .await
            .expect("create condition=Б/У");

        let filter = DeviceFilter {
            group_by_condition: false,
            ..Default::default()
        };
        let page = Pagination {
            offset: 0,
            limit: 50,
        };
        let groups = svc.list_grouped(filter, page).await.expect("list_grouped");

        assert_eq!(
            groups.len(),
            1,
            "ITEM-1c: одна смешанная группа ожидается, получили {} групп",
            groups.len()
        );
        assert!(
            groups[0].condition_distinct_count > 1,
            "ITEM-1c: condition_distinct_count должен быть > 1 для смешанной группы, получено {}",
            groups[0].condition_distinct_count
        );
    })
    .await
    .expect("grouping_condition_distinct_count_mixed exceeded 30s");
}

// ---------------------------------------------------------------------------
// grouping_condition_distinct_count_counts_null_as_distinct (WR-04)
// ---------------------------------------------------------------------------
// SQLite's COUNT(DISTINCT x) ignores NULL — a group with одно устройство БЕЗ
// condition (NULL) и одно С condition="Новое" раньше сообщал
// condition_distinct_count=1, что подавляло D-07 drill-in и клонировало
// смешанную группу как однородную. COALESCE(d.condition, ' ') в SQL должен
// считать NULL отдельным бакетом → distinct_count == 2 для такой группы.
// Проверяем оба режима (true-branch с model-ключом и false-branch).

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn grouping_condition_distinct_count_counts_null_as_distinct() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();

        // non_unique_device: state=None → condition IS NULL в БД.
        svc.create(non_unique_device("Проектор", 1))
            .await
            .expect("create condition=NULL");
        // device_with_condition("Проектор", "Новое"): тот же name + model
        // ("Model X"), condition="Новое" — должны схлопнуться в одну группу.
        svc.create(device_with_condition("Проектор", "Новое"))
            .await
            .expect("create condition=Новое");

        let page = Pagination {
            offset: 0,
            limit: 50,
        };

        // true-branch (group_by_condition=true, D-04/D-05 model-key grouping) —
        // основной путь для drill-in в форме акта.
        let filter_true = DeviceFilter {
            group_by_condition: true,
            ..Default::default()
        };
        let groups_true = svc
            .list_grouped(filter_true, page)
            .await
            .expect("list_grouped (group_by_condition=true)");
        assert_eq!(
            groups_true.len(),
            1,
            "WR-04: одинаковый name+model (NULL + Новое condition) → ОДНА группа, получили {} групп",
            groups_true.len()
        );
        assert_eq!(
            groups_true[0].condition_distinct_count, 2,
            "WR-04: condition_distinct_count должен считать NULL как отдельный бакет (true-branch), получено {}",
            groups_true[0].condition_distinct_count
        );

        // false-branch (group_by_condition=false, DevicesPage grouping).
        let filter_false = DeviceFilter {
            group_by_condition: false,
            ..Default::default()
        };
        let groups_false = svc
            .list_grouped(filter_false, page)
            .await
            .expect("list_grouped (group_by_condition=false)");
        assert_eq!(
            groups_false.len(),
            1,
            "WR-04: false-branch тоже должен схлопнуть в ОДНУ группу, получили {} групп",
            groups_false.len()
        );
        assert_eq!(
            groups_false[0].condition_distinct_count, 2,
            "WR-04: condition_distinct_count должен считать NULL как отдельный бакет (false-branch), получено {}",
            groups_false[0].condition_distinct_count
        );
    })
    .await
    .expect("grouping_condition_distinct_count_counts_null_as_distinct exceeded 30s");
}

// ---------------------------------------------------------------------------
// condition_key_same_condition_collapses (DEF-2B)
// ---------------------------------------------------------------------------
// Два устройства с одинаковым name + одинаковым condition → одна группа count=2.

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn condition_key_same_condition_collapses() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();

        let mut d1 = non_unique_device("DVD-ROM ASUS", 1);
        d1.state = Some("Новое".to_string());

        let mut d2 = non_unique_device("DVD-ROM ASUS", 1);
        d2.state = Some("Новое".to_string());

        svc.create(d1).await.expect("create condition=Новое (1)");
        svc.create(d2).await.expect("create condition=Новое (2)");

        let filter = DeviceFilter::default();
        let page = Pagination {
            offset: 0,
            limit: 50,
        };
        let groups = svc.list_grouped(filter, page).await.expect("list_grouped");

        assert_eq!(
            groups.len(),
            1,
            "одинаковый condition → одна группа, получили {} групп",
            groups.len()
        );
        assert_eq!(groups[0].count, 2, "count группы должен быть 2");
    })
    .await
    .expect("condition_key_same_condition_collapses exceeded 30s");
}

// ---------------------------------------------------------------------------
// grouping_true_branch_splits_by_model (D-05, Phase 18)
// ---------------------------------------------------------------------------
// Два устройства с одинаковым name, но РАЗНЫМ model, group_by_condition=true
// → 2 отдельные группы (модель — часть ключа группировки true-ветки).

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn grouping_true_branch_splits_by_model() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();

        let mut d1 = non_unique_device("Принтер HP", 1);
        d1.model = Some("M404".to_string());

        let mut d2 = non_unique_device("Принтер HP", 1);
        d2.model = Some("M405".to_string());

        svc.create(d1).await.expect("create model=M404");
        svc.create(d2).await.expect("create model=M405");

        let filter = DeviceFilter {
            group_by_condition: true,
            ..Default::default()
        };
        let page = Pagination {
            offset: 0,
            limit: 50,
        };
        let groups = svc.list_grouped(filter, page).await.expect("list_grouped");

        assert_eq!(
            groups.len(),
            2,
            "D-05: одинаковое name, разный model → 2 группы (НЕ схлопывать), получили {} групп",
            groups.len()
        );
        assert!(
            groups.iter().all(|g| g.count == 1),
            "каждая model-группа должна содержать ровно 1 устройство"
        );
    })
    .await
    .expect("grouping_true_branch_splits_by_model exceeded 30s");
}

// ---------------------------------------------------------------------------
// grouping_true_branch_sorts_by_count_desc (D-04, Phase 18)
// ---------------------------------------------------------------------------
// 3 разноимённых группы с count 1/5/3, group_by_condition=true → порядок
// групп строго по убыванию count (5, затем 3, затем 1), не по алфавиту.

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn grouping_true_branch_sorts_by_count_desc() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();

        // "Алоэ" (count=1) would sort first alphabetically, but must sort LAST by count.
        svc.create(non_unique_device("Алоэ", 1))
            .await
            .expect("create count=1 group");
        for _ in 0..5 {
            svc.create(non_unique_device("Яблоко", 1))
                .await
                .expect("create count=5 group");
        }
        for _ in 0..3 {
            svc.create(non_unique_device("Вишня", 1))
                .await
                .expect("create count=3 group");
        }

        let filter = DeviceFilter {
            group_by_condition: true,
            ..Default::default()
        };
        let page = Pagination {
            offset: 0,
            limit: 50,
        };
        let groups = svc.list_grouped(filter, page).await.expect("list_grouped");

        assert_eq!(groups.len(), 3, "должно быть 3 группы");
        let counts: Vec<u64> = groups.iter().map(|g| g.count).collect();
        assert_eq!(
            counts,
            vec![5u64, 3, 1],
            "D-04: порядок групп должен быть строго по убыванию count, получили {counts:?}"
        );
    })
    .await
    .expect("grouping_true_branch_sorts_by_count_desc exceeded 30s");
}

// ---------------------------------------------------------------------------
// grouping_true_branch_filters_by_name_text (AUTO-03, Phase 18)
// ---------------------------------------------------------------------------
// name_prefix="Lenovo" (group_by_condition=true) → возвращает только группу,
// чьё наименование матчится FTS5-токеном "Lenovo".

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn grouping_true_branch_filters_by_name_text() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();

        svc.create(non_unique_device("Ноутбук Lenovo X1", 1))
            .await
            .expect("create Lenovo");
        svc.create(non_unique_device("Монитор Dell", 1))
            .await
            .expect("create Dell");

        let filter = DeviceFilter {
            group_by_condition: true,
            name_prefix: Some("Lenovo".to_string()),
            ..Default::default()
        };
        let page = Pagination {
            offset: 0,
            limit: 50,
        };
        let groups = svc.list_grouped(filter, page).await.expect("list_grouped");

        assert_eq!(
            groups.len(),
            1,
            "AUTO-03: фильтр 'Lenovo' должен вернуть только 1 группу, получили {} групп",
            groups.len()
        );
        assert_eq!(groups[0].repr.name, "Ноутбук Lenovo X1");
    })
    .await
    .expect("grouping_true_branch_filters_by_name_text exceeded 30s");
}

// ---------------------------------------------------------------------------
// grouping_true_branch_filters_by_inventory_and_serial (AUTO-03, Phase 18)
// ---------------------------------------------------------------------------
// Текстовый фильтр не ограничен именем — доказывает матч по inventory_number.

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn grouping_true_branch_filters_by_inventory_and_serial() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();

        let mut d = non_unique_device("Сервер Dell", 1);
        d.inventory_no = Some("INV-777".to_string());
        svc.create(d).await.expect("create with inv INV-777");

        svc.create(non_unique_device("Другое устройство", 1))
            .await
            .expect("create unrelated device");

        let filter = DeviceFilter {
            group_by_condition: true,
            name_prefix: Some("INV-777".to_string()),
            ..Default::default()
        };
        let page = Pagination {
            offset: 0,
            limit: 50,
        };
        let groups = svc.list_grouped(filter, page).await.expect("list_grouped");

        assert_eq!(
            groups.len(),
            1,
            "AUTO-03: фильтр по инвентарному № должен найти устройство, получили {} групп",
            groups.len()
        );
        assert_eq!(groups[0].repr.name, "Сервер Dell");
    })
    .await
    .expect("grouping_true_branch_filters_by_inventory_and_serial exceeded 30s");
}

// ---------------------------------------------------------------------------
// grouping_true_branch_query_sanitizes_special_chars (T-18-01)
// ---------------------------------------------------------------------------
// FTS5 спецсимволы во входном тексте не должны вызывать Err/панику —
// build_fts_query sanitizer превращает их в безопасные литералы.
// Образец: devices_search.rs::search_quotes_user_input_with_special_chars.

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn grouping_true_branch_query_sanitizes_special_chars() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();

        svc.create(non_unique_device("Принтер", 1))
            .await
            .expect("create");

        let page = Pagination {
            offset: 0,
            limit: 50,
        };

        let tricky_queries = [
            "(AND OR)",
            "NOT foo",
            "\"unmatched quote",
            "NEAR(x y)",
            "foo*bar",
        ];

        for q in &tricky_queries {
            let filter = DeviceFilter {
                group_by_condition: true,
                name_prefix: Some(q.to_string()),
                ..Default::default()
            };
            let result = svc.list_grouped(filter, page).await;
            assert!(
                result.is_ok(),
                "T-18-01: запрос '{q}' должен выполниться без ошибки FTS5 синтаксиса, получили: {result:?}"
            );
        }
    })
    .await
    .expect("grouping_true_branch_query_sanitizes_special_chars exceeded 30s");
}
