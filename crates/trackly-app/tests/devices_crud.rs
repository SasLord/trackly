//! Интеграционные тесты CRUD-операций `DeviceService`.
//!
//! RED phase (Plan 03 Task 1): тесты написаны ДО реализации.
//! После реализации (GREEN) все тесты проходят.
//!
//! Каждый тест обёрнут в `tokio::time::timeout(30s)` — защита от Linux-CI
//! deadlock (PATTERNS.md §Pattern 4).

use std::sync::Arc;
use std::time::Duration;

use trackly_app::dto::device::{DeviceFilter, DeviceNew, DevicePatch, Pagination};
use trackly_app::services::DeviceService;
use trackly_core::error::AppError;
use trackly_core::primitives::clock::Clock;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::test_support::test_writer_and_readers;

/// Создаёт тестовый `DeviceService` поверх свежего tempfile DB.
fn make_service() -> (DeviceService, tempfile::TempDir) {
    let (writer, readers, dir) = test_writer_and_readers();
    let clock: Arc<dyn Clock + Send + Sync> = Arc::new(SystemClock);
    let svc = DeviceService::new(writer, readers, clock);
    (svc, dir)
}

/// DeviceNew с минимальными обязательными полями.
fn minimal_new(name: &str) -> DeviceNew {
    DeviceNew {
        type_id: 1,
        name: name.to_string(),
        inventory_no: None,
        serial_no: None,
        model: None,
        specs: None,
        kit: None,
        state: None,
        location: None,
        location_id: None,
        status_id: 1,
    }
}

// ---------------------------------------------------------------------------
// create_inserts_device_and_audit_log
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_inserts_device_and_audit_log() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let new = minimal_new("Ноутбук Lenovo");
        let dto = svc.create(new).await.expect("create device");

        assert!(dto.id > 0, "id должен быть > 0, получили {}", dto.id);
        assert_eq!(dto.version, 1);
        assert!(dto.created_at_utc > 0);
        assert_eq!(dto.name, "Ноутбук Lenovo");
        assert_eq!(dto.type_id, 1);
        assert_eq!(dto.status_id, 1);

        // Проверяем audit_log через reader.
        let readers = svc.readers.clone();
        let entity_id = dto.id;
        let count: i64 = tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            conn.query_row(
                "SELECT COUNT(*) FROM audit_log WHERE entity_type='device' AND entity_id=?1 AND action='create'",
                rusqlite::params![entity_id],
                |r| r.get(0),
            )
        })
        .await
        .expect("spawn_blocking")
        .expect("count audit_log");

        assert_eq!(count, 1, "должна быть ровно одна запись audit_log create, получили {count}");
    })
    .await
    .expect("create_inserts_device_and_audit_log exceeded 30 s budget");
}

// ---------------------------------------------------------------------------
// create_rejects_empty_name
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_rejects_empty_name() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let new = minimal_new("");
        let err = svc.create(new).await.expect_err("должно вернуть ошибку для пустого name");
        match err {
            AppError::Validation { field, message } => {
                assert_eq!(field, "name");
                assert!(
                    message.starts_with("Наименование"),
                    "message должен начинаться с 'Наименование', получили: {message}"
                );
            }
            other => panic!("ожидали AppError::Validation, получили {other:?}"),
        }
    })
    .await
    .expect("create_rejects_empty_name exceeded 30 s budget");
}

// ---------------------------------------------------------------------------
// create_rejects_missing_type
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_rejects_missing_type() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let mut new = minimal_new("Тест");
        new.type_id = 0; // невалидный type_id
        let err = svc.create(new).await.expect_err("должно вернуть ошибку для type_id=0");
        match err {
            AppError::Validation { field, .. } => {
                assert_eq!(field, "type_id");
            }
            AppError::Conflict { .. } => {
                // FK violation — тоже допустимо
            }
            other => panic!("ожидали Validation или Conflict, получили {other:?}"),
        }
    })
    .await
    .expect("create_rejects_missing_type exceeded 30 s budget");
}

// ---------------------------------------------------------------------------
// update_succeeds_with_correct_version
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn update_succeeds_with_correct_version() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let dto = svc.create(minimal_new("Ноутбук Lenovo")).await.expect("create");

        let patch = DevicePatch {
            name: Some("Ноутбук Lenovo X1".to_string()),
            ..Default::default()
        };
        let updated = svc
            .update(dto.id, dto.version, patch)
            .await
            .expect("update");
        assert_eq!(updated.version, 2);
        assert_eq!(updated.name, "Ноутбук Lenovo X1");

        // audit_log 'update'
        let readers = svc.readers.clone();
        let entity_id = dto.id;
        let count: i64 = tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            conn.query_row(
                "SELECT COUNT(*) FROM audit_log WHERE entity_type='device' AND entity_id=?1 AND action='update'",
                rusqlite::params![entity_id],
                |r| r.get(0),
            )
        })
        .await
        .expect("spawn_blocking")
        .expect("count audit_log update");
        assert_eq!(count, 1, "одна запись audit_log update, получили {count}");
    })
    .await
    .expect("update_succeeds_with_correct_version exceeded 30 s budget");
}

// ---------------------------------------------------------------------------
// update_returns_optimistic_lock_mismatch_on_stale_version
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn update_returns_optimistic_lock_mismatch_on_stale_version() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let dto = svc.create(minimal_new("Тест OLM")).await.expect("create");
        assert_eq!(dto.version, 1);

        // Первый update — успешен, version становится 2.
        let patch = DevicePatch {
            name: Some("Тест OLM v2".to_string()),
            ..Default::default()
        };
        svc.update(dto.id, 1, patch.clone()).await.expect("first update");

        // Второй update со старой version=1 — должен вернуть OptimisticLockMismatch.
        let err = svc
            .update(dto.id, 1, patch)
            .await
            .expect_err("должно вернуть ошибку со stale version");
        match err {
            AppError::OptimisticLockMismatch {
                entity,
                id,
                expected,
                actual,
            } => {
                assert_eq!(entity, "device");
                assert_eq!(id, dto.id);
                assert_eq!(expected, 1);
                assert_eq!(actual, 2);
            }
            other => panic!("ожидали OptimisticLockMismatch, получили {other:?}"),
        }
    })
    .await
    .expect("update_returns_optimistic_lock_mismatch_on_stale_version exceeded 30 s budget");
}

// ---------------------------------------------------------------------------
// delete_soft_marks_deleted_at_utc
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn delete_soft_marks_deleted_at_utc() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let dto = svc.create(minimal_new("На удаление")).await.expect("create");

        svc.delete_soft(dto.id, dto.version).await.expect("delete_soft");

        // list не возвращает удалённые
        let filter = DeviceFilter::default();
        let page = Pagination::default();
        let resp = svc.list(filter, page).await.expect("list after delete");
        assert!(
            resp.items.is_empty(),
            "список должен быть пустым после soft-delete, получили {} элементов",
            resp.items.len()
        );

        // audit_log 'delete'
        let readers = svc.readers.clone();
        let entity_id = dto.id;
        let count: i64 = tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            conn.query_row(
                "SELECT COUNT(*) FROM audit_log WHERE entity_type='device' AND entity_id=?1 AND action='delete'",
                rusqlite::params![entity_id],
                |r| r.get(0),
            )
        })
        .await
        .expect("spawn_blocking")
        .expect("count audit_log delete");
        assert_eq!(count, 1, "одна запись audit_log delete, получили {count}");
    })
    .await
    .expect("delete_soft_marks_deleted_at_utc exceeded 30 s budget");
}

// ---------------------------------------------------------------------------
// list_returns_only_non_deleted
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn list_returns_only_non_deleted() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();

        let d1 = svc.create(minimal_new("Устройство 1")).await.expect("create 1");
        let d2 = svc.create(minimal_new("Устройство 2")).await.expect("create 2");
        svc.create(minimal_new("Устройство 3")).await.expect("create 3");

        // Удалим первые два
        svc.delete_soft(d1.id, d1.version).await.expect("delete 1");
        svc.delete_soft(d2.id, d2.version).await.expect("delete 2");

        let filter = DeviceFilter::default();
        let page = Pagination::default();
        let resp = svc.list(filter, page).await.expect("list");
        assert_eq!(resp.items.len(), 1, "должно быть 1 устройство, получили {}", resp.items.len());
        assert_eq!(resp.total, 1);
    })
    .await
    .expect("list_returns_only_non_deleted exceeded 30 s budget");
}

// ---------------------------------------------------------------------------
// list_with_status_filter
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn list_with_status_filter() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();

        svc.create(minimal_new("Устройство status=1")).await.expect("create status=1");
        let mut new2 = minimal_new("Устройство status=2");
        new2.status_id = 2;
        svc.create(new2).await.expect("create status=2");
        let mut new3 = minimal_new("Устройство status=2 second");
        new3.status_id = 2;
        svc.create(new3).await.expect("create status=2 second");

        let filter = DeviceFilter {
            status_id: Some(2),
            ..Default::default()
        };
        let page = Pagination::default();
        let resp = svc.list(filter, page).await.expect("list");
        assert_eq!(
            resp.items.len(),
            2,
            "с фильтром status_id=2 должно быть 2 устройства, получили {}",
            resp.items.len()
        );
        for item in &resp.items {
            assert_eq!(item.status_id, 2);
        }
    })
    .await
    .expect("list_with_status_filter exceeded 30 s budget");
}

// ---------------------------------------------------------------------------
// list_with_type_id_filter
// ---------------------------------------------------------------------------
//
// Verifies the /devices vs /printers section split:
// - type_id=1 ("Устройство") — shown in /devices UI section
// - type_id=2 ("Принтер")   — shown in /printers UI section (Phase 6)
// - type_id=None             — returns all (admin/internal use)

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn list_with_type_id_filter() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();

        // Create one device with type_id=1 (Устройство, seeded in V001)
        let d1 = svc.create(minimal_new("Ноутбук Lenovo")).await.expect("create type=1");
        assert_eq!(d1.type_id, 1);

        // Create one device with type_id=2 (Принтер, seeded in V001)
        let mut new2 = minimal_new("HP LaserJet Pro");
        new2.type_id = 2;
        let d2 = svc.create(new2).await.expect("create type=2");
        assert_eq!(d2.type_id, 2);

        // Filter by type_id=1: only Ноутбук Lenovo returned
        let filter_type1 = DeviceFilter {
            type_id: Some(1),
            ..Default::default()
        };
        let resp1 = svc.list(filter_type1, Pagination::default()).await.expect("list type=1");
        assert_eq!(
            resp1.items.len(),
            1,
            "type_id=1 фильтр должен вернуть 1 устройство, получили {}",
            resp1.items.len()
        );
        assert_eq!(resp1.items[0].name, "Ноутбук Lenovo");
        assert_eq!(resp1.items[0].type_id, 1);

        // Filter by type_id=2: only HP LaserJet Pro returned
        let filter_type2 = DeviceFilter {
            type_id: Some(2),
            ..Default::default()
        };
        let resp2 = svc.list(filter_type2, Pagination::default()).await.expect("list type=2");
        assert_eq!(
            resp2.items.len(),
            1,
            "type_id=2 фильтр должен вернуть 1 устройство, получили {}",
            resp2.items.len()
        );
        assert_eq!(resp2.items[0].name, "HP LaserJet Pro");
        assert_eq!(resp2.items[0].type_id, 2);

        // Filter by type_id=None: both returned
        let filter_all = DeviceFilter::default(); // type_id: None
        let resp_all = svc.list(filter_all, Pagination::default()).await.expect("list all");
        assert_eq!(
            resp_all.items.len(),
            2,
            "без фильтра type_id должно быть 2 устройства, получили {}",
            resp_all.items.len()
        );
        assert_eq!(resp_all.total, 2);
    })
    .await
    .expect("list_with_type_id_filter exceeded 30 s budget");
}

// ---------------------------------------------------------------------------
// create_persists_serial_number
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_persists_serial_number() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();

        let new = DeviceNew {
            type_id: 1,
            name: "Ноутбук Lenovo".to_string(),
            inventory_no: None,
            serial_no: Some("SN-XYZ-001".to_string()),
            model: None,
            specs: None,
            kit: None,
            state: None,
            location: None,
            location_id: None,
            status_id: 1,
        };

        let dto = svc.create(new).await.expect("create device with serial_no");

        // Round-trip: читаем только что созданное устройство
        let fetched = svc.get(dto.id).await.expect("get by id");
        assert_eq!(
            fetched.serial_no,
            Some("SN-XYZ-001".to_string()),
            "serial_no должен сохраниться и вернуться как есть, получили {:?}",
            fetched.serial_no
        );

        // create() возвращает DeviceDto с serial_no — проверяем сразу
        assert_eq!(
            dto.serial_no,
            Some("SN-XYZ-001".to_string()),
            "create() должен вернуть dto с serial_no, получили {:?}",
            dto.serial_no
        );
    })
    .await
    .expect("create_persists_serial_number exceeded 30 s budget");
}

// ---------------------------------------------------------------------------
// create_persists_inventory_and_serial_together
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_persists_inventory_and_serial_together() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();

        let new = DeviceNew {
            type_id: 1,
            name: "Сервер Dell".to_string(),
            inventory_no: Some("ИНВ-000007".to_string()),
            serial_no: Some("DELLSN-20240001".to_string()),
            model: Some("PowerEdge R640".to_string()),
            specs: None,
            kit: None,
            state: None,
            location: None,
            location_id: None,
            status_id: 1,
        };

        let dto = svc.create(new).await.expect("create device with both numbers");

        assert_eq!(
            dto.inventory_no,
            Some("ИНВ-000007".to_string()),
            "inventory_no должен сохраниться"
        );
        assert_eq!(
            dto.serial_no,
            Some("DELLSN-20240001".to_string()),
            "serial_no должен сохраниться"
        );

        // Verify via get() for full round-trip
        let fetched = svc.get(dto.id).await.expect("get by id");
        assert_eq!(fetched.inventory_no, dto.inventory_no);
        assert_eq!(fetched.serial_no, dto.serial_no);
    })
    .await
    .expect("create_persists_inventory_and_serial_together exceeded 30 s budget");
}

// ---------------------------------------------------------------------------
// update_second_save_after_successful_first_uses_new_version
//
// Regression guard for the symptom: user edits device, clicks Save (success,
// v1→v2), edits again within the same modal session without page reload,
// clicks Save again — the second save must succeed (v2→v3), NOT fail with
// OptimisticLockMismatch because the form still held v1.
//
// The frontend fix (currentVersion = updated.version) ensures this; the
// backend contract itself is correct and has always worked — this test
// documents the expected call sequence.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn update_second_save_after_successful_first_uses_new_version() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let dto = svc.create(minimal_new("Тест версий")).await.expect("create");
        assert_eq!(dto.version, 1);

        // First update: expected_version=1 → succeeds, returns v2.
        let patch1 = DevicePatch {
            name: Some("Тест версий v2".to_string()),
            ..Default::default()
        };
        let v2 = svc.update(dto.id, 1, patch1).await.expect("first update");
        assert_eq!(v2.version, 2, "first update must return version=2");

        // Second update using the REFRESHED version: expected_version=2 → succeeds, returns v3.
        let patch2 = DevicePatch {
            name: Some("Тест версий v3".to_string()),
            ..Default::default()
        };
        let v3 = svc
            .update(dto.id, v2.version, patch2)
            .await
            .expect("second update must succeed when using refreshed version");
        assert_eq!(v3.version, 3, "second update must return version=3");
        assert_eq!(v3.name, "Тест версий v3");
    })
    .await
    .expect("update_second_save_after_successful_first_uses_new_version exceeded 30 s budget");
}

// ---------------------------------------------------------------------------
// state_hints_returns_six_russian_strings
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn state_hints_returns_six_russian_strings() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let hints = svc.state_hints();
        assert_eq!(hints.len(), 6, "должно быть 6 state hints, получили {}", hints.len());
        assert_eq!(hints[0], "Новое");
        // Все строки непустые
        for h in &hints {
            assert!(!h.is_empty());
        }
    })
    .await
    .expect("state_hints_returns_six_russian_strings exceeded 30 s budget");
}
