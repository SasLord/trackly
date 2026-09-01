//! Интеграционные тесты round-trip `place_id`/`full_path` через дерево мест
//! (`places`/`place_full_paths`, Phase 39 — заменяет свободнотекстовое
//! `location`/`locations`, D-18: caller передаёт уже разрешённый `place_id`,
//! ни один путь записи устройства больше не создаёт место неявно по строке).
//!
//! Каждый тест обёрнут в `tokio::time::timeout(30s)` — защита от CI deadlock.
//!
//! Только вымышленные названия мест ("Здание А", "Офис 305") — никогда
//! реальные данные организации, по жёсткому условию приватности проекта.

use std::sync::Arc;
use std::time::Duration;

use trackly_app::dto::device::{DeviceNew, DevicePatch};
use trackly_app::services::DeviceService;
use trackly_core::auth::Identity;
use trackly_core::domain::places::{PlaceKind, PlaceNew};
use trackly_core::error::AppError;
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

/// «Доверенный администратор» — десктоп unlocked mode (D-Desktop-01).
fn admin_caller() -> Identity {
    Identity::trusted_admin()
}

/// Создаёт корневое место (kind=Building) напрямую через `SqlitePlaceRepository`
/// на writer-соединении сервиса (D-18: только явное Admin-действие создаёт место,
/// ни один device-write-путь не делает этого неявно).
async fn create_place(svc: &DeviceService, name: &str) -> i64 {
    let name = name.to_string();
    svc.writer
        .execute(move |conn| {
            let repo = SqlitePlaceRepository;
            let new_place = PlaceNew {
                parent_id: None,
                kind: PlaceKind::Building,
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

fn new_with_place_id(name: &str, place_id: Option<i64>) -> DeviceNew {
    DeviceNew {
        type_id: 1,
        name: name.to_string(),
        inventory_no: None,
        serial_no: None,
        model: None,
        specs: None,
        kit: None,
        state: None,
        place_id,
        status_id: 1,
    }
}

// ---------------------------------------------------------------------------
// create_with_place_persists_round_trip
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_with_place_persists_round_trip() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let place_id = create_place(&svc, "Склад A").await;

        let new = new_with_place_id("Ноутбук Lenovo", Some(place_id));
        let dto = svc.create(new).await.expect("create device");

        assert_eq!(dto.place_id, Some(place_id), "place_id должен сохраниться");
        assert_eq!(
            dto.full_path.as_deref(),
            Some("Склад A"),
            "full_path должен резолвиться из place_full_paths"
        );

        // Re-fetch to confirm persistence.
        let fetched = svc.get(dto.id).await.expect("get device");
        assert_eq!(fetched.place_id, Some(place_id));
        assert_eq!(
            fetched.full_path.as_deref(),
            Some("Склад A"),
            "после re-fetch full_path должен совпадать"
        );
    })
    .await
    .expect("create_with_place_persists_round_trip exceeded 30 s budget");
}

// ---------------------------------------------------------------------------
// update_changes_place_id_round_trips
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn update_changes_place_id_round_trips() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let place_a = create_place(&svc, "Склад A").await;
        let place_b = create_place(&svc, "Офис 305").await;

        let dto = svc
            .create(new_with_place_id("Ноутбук", Some(place_a)))
            .await
            .expect("create");
        assert_eq!(dto.place_id, Some(place_a));

        let patch = DevicePatch {
            place_id: Some(Some(place_b)),
            ..Default::default()
        };
        let updated = svc
            .update(&admin_caller(), dto.id, dto.version, patch)
            .await
            .expect("update");

        assert_eq!(
            updated.place_id,
            Some(place_b),
            "place_id должен обновиться на новое место"
        );
        assert_eq!(
            updated.full_path.as_deref(),
            Some("Офис 305"),
            "full_path должен отражать новое место сразу, без переиндексации"
        );
    })
    .await
    .expect("update_changes_place_id_round_trips exceeded 30 s budget");
}

// ---------------------------------------------------------------------------
// create_with_no_place_keeps_null
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_with_no_place_keeps_null() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();

        let new = new_with_place_id("Устройство без места", None);
        let dto = svc.create(new).await.expect("create");

        assert!(
            dto.place_id.is_none(),
            "place_id должен быть None (D-07: место опционально)"
        );
        assert!(
            dto.full_path.is_none(),
            "full_path должен быть None без места"
        );
    })
    .await
    .expect("create_with_no_place_keeps_null exceeded 30 s budget");
}

// ---------------------------------------------------------------------------
// create_with_nonexistent_place_id_is_rejected
// ---------------------------------------------------------------------------

/// D-18 / T-39-06-01: несуществующий `place_id` отклоняется FK-констрейнтом
/// (`ON DELETE RESTRICT`) — никакой путь записи не создаёт место неявно.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_with_nonexistent_place_id_is_rejected() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();

        let new = new_with_place_id("Устройство с несуществующим местом", Some(999_999));
        let result = svc.create(new).await;

        assert!(
            matches!(result, Err(AppError::Conflict { .. })),
            "должен быть отклонён FK-констрейнтом, получили {result:?}"
        );
    })
    .await
    .expect("create_with_nonexistent_place_id_is_rejected exceeded 30 s budget");
}
