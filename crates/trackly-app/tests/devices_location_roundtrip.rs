//! Интеграционные тесты round-trip location через таблицу `locations`.
//!
//! Каждый тест обёрнут в `tokio::time::timeout(30s)` — защита от CI deadlock.

use std::sync::Arc;
use std::time::Duration;

use trackly_app::dto::device::{DeviceNew, DevicePatch};
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

fn new_with_location(name: &str, location: &str) -> DeviceNew {
    DeviceNew {
        type_id: 1,
        name: name.to_string(),
        inventory_no: None,
        serial_no: None,
        model: None,
        specs: None,
        kit: None,
        state: None,
        location: if location.is_empty() { None } else { Some(location.to_string()) },
        location_id: None,
        status_id: 1,
    }
}

// ---------------------------------------------------------------------------
// create_with_location_persists_round_trip
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_with_location_persists_round_trip() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let new = new_with_location("Ноутбук Lenovo", "Склад A");
        let dto = svc.create(new).await.expect("create device");

        assert_eq!(dto.location.as_deref(), Some("Склад A"), "location должен вернуться как строка");
        assert!(dto.location_id.is_some(), "location_id должен быть заполнен");

        // Re-fetch to confirm persistence.
        let fetched = svc.get(dto.id).await.expect("get device");
        assert_eq!(fetched.location.as_deref(), Some("Склад A"), "после re-fetch location должен совпадать");
    })
    .await
    .expect("create_with_location_persists_round_trip exceeded 30 s budget");
}

// ---------------------------------------------------------------------------
// create_with_same_location_reuses_id
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_with_same_location_reuses_id() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();

        let a = svc.create(new_with_location("Устройство A", "Склад A")).await.expect("create A");
        let b = svc.create(new_with_location("Устройство B", "Склад A")).await.expect("create B");

        // Same location string → same location_id (INSERT OR IGNORE reuses existing row).
        assert_eq!(
            a.location_id, b.location_id,
            "одинаковая строка расположения должна давать одинаковый location_id: A={:?}, B={:?}",
            a.location_id, b.location_id
        );
        assert_eq!(a.location.as_deref(), Some("Склад A"));
        assert_eq!(b.location.as_deref(), Some("Склад A"));
    })
    .await
    .expect("create_with_same_location_reuses_id exceeded 30 s budget");
}

// ---------------------------------------------------------------------------
// update_changes_location_creates_new_locations_row
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn update_changes_location_creates_new_locations_row() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();

        let dto = svc.create(new_with_location("Ноутбук", "Склад A")).await.expect("create");
        let original_location_id = dto.location_id;

        let patch = DevicePatch {
            location: Some(Some("Офис 305".to_string())),
            ..Default::default()
        };
        let updated = svc.update(dto.id, dto.version, patch).await.expect("update");

        assert_eq!(updated.location.as_deref(), Some("Офис 305"), "location должен обновиться");
        assert_ne!(
            updated.location_id, original_location_id,
            "новая строка расположения должна дать новый location_id"
        );

        // Verify the updated location appears in autocomplete (device now linked to "Офис 305").
        // "Склад A" is NOT checked here: the device moved away from it, so no device links to it,
        // and location autocomplete only surfaces locations that have associated devices.
        let locs = svc
            .autocomplete("location".to_string(), "".to_string(), None, None)
            .await
            .expect("autocomplete location");
        assert!(locs.contains(&"Офис 305".to_string()), "Офис 305 должен появиться в autocomplete");
    })
    .await
    .expect("update_changes_location_creates_new_locations_row exceeded 30 s budget");
}

// ---------------------------------------------------------------------------
// create_with_empty_location_keeps_null
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_with_empty_location_keeps_null() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();

        let new = new_with_location("Устройство без расположения", "");
        let dto = svc.create(new).await.expect("create");

        assert!(dto.location_id.is_none(), "location_id должен быть None при пустой строке");
        assert!(dto.location.is_none(), "location должен быть None при пустой строке");
    })
    .await
    .expect("create_with_empty_location_keeps_null exceeded 30 s budget");
}

// ---------------------------------------------------------------------------
// autocomplete_location_returns_from_locations_table
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn autocomplete_location_returns_from_locations_table() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();

        svc.create(new_with_location("Устройство 1", "Склад A")).await.expect("create 1");
        svc.create(new_with_location("Устройство 2", "Склад B")).await.expect("create 2");
        svc.create(new_with_location("Устройство 3", "Офис 305")).await.expect("create 3");

        // All locations with prefix "Скла"
        let results = svc
            .autocomplete("location".to_string(), "Скла".to_string(), None, None)
            .await
            .expect("autocomplete location prefix");

        assert_eq!(results.len(), 2, "должно вернуть 2 склада, получили {results:?}");
        assert!(results.contains(&"Склад A".to_string()));
        assert!(results.contains(&"Склад B".to_string()));
        assert!(!results.contains(&"Офис 305".to_string()));
    })
    .await
    .expect("autocomplete_location_returns_from_locations_table exceeded 30 s budget");
}

// ---------------------------------------------------------------------------
// autocomplete_location_filtered_by_ctx_status_id_via_locations_table
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn autocomplete_location_filtered_by_ctx_status_id_via_locations_table() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();

        // Device A: location="Склад A", status_id=1.
        svc.create(DeviceNew {
            type_id: 1,
            name: "Устройство 1".to_string(),
            inventory_no: None,
            serial_no: None,
            model: None,
            specs: None,
            kit: None,
            state: None,
            location: Some("Склад A".to_string()),
            location_id: None,
            status_id: 1,
        }).await.expect("create 1");

        // Device B: location="Офис 305", status_id=2.
        svc.create(DeviceNew {
            type_id: 1,
            name: "Устройство 2".to_string(),
            inventory_no: None,
            serial_no: None,
            model: None,
            specs: None,
            kit: None,
            state: None,
            location: Some("Офис 305".to_string()),
            location_id: None,
            status_id: 2,
        }).await.expect("create 2");

        // ctx_status_id=1 → only "Склад A"
        let results = svc
            .autocomplete("location".to_string(), "".to_string(), None, Some(1))
            .await
            .expect("autocomplete location ctx_status_id=1");

        assert_eq!(results.len(), 1, "ctx_status_id=1: ожидаем 1 результат, получили {results:?}");
        assert_eq!(results[0], "Склад A");

        // ctx_status_id=2 → only "Офис 305"
        let results2 = svc
            .autocomplete("location".to_string(), "".to_string(), None, Some(2))
            .await
            .expect("autocomplete location ctx_status_id=2");

        assert_eq!(results2.len(), 1, "ctx_status_id=2: ожидаем 1 результат, получили {results2:?}");
        assert_eq!(results2[0], "Офис 305");
    })
    .await
    .expect("autocomplete_location_filtered_by_ctx_status_id_via_locations_table exceeded 30 s budget");
}
