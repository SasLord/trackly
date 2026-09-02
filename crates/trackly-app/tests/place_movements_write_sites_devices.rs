//! Wave 0 integration coverage: device family write site for `place_movements`
//! (Plan 40-07, HST-01).
//!
//! Verifies `device_service::update` calls
//! `SqlitePlaceMovementsRepository::record_movement_if_applicable` correctly:
//! - a real place->place change records exactly one manual-source row (D-27)
//! - a status/notes-only edit (place unchanged) records zero rows (D-04)
//! - a first-time place assignment (NULL -> place) records zero rows (D-06)
//!
//! Harness mirrors `devices_crud.rs` / `cartridges_crud.rs::seed_place` — real
//! tempfile SQLite DB via `test_writer_and_readers`, invented place/device names only
//! (CLAUDE.md privacy gate).

use std::sync::Arc;
use std::time::Duration;

use trackly_app::dto::device::{DeviceNew, DevicePatch};
use trackly_app::services::DeviceService;
use trackly_core::auth::{Identity, Role};
use trackly_core::error::AppError;
use trackly_core::primitives::clock::Clock;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::db::writer_worker::WriterHandle;
use trackly_infra::test_support::test_writer_and_readers;

/// Создаёт тестовый `DeviceService` поверх свежего tempfile DB.
fn make_service() -> (DeviceService, tempfile::TempDir) {
    let (writer, readers, dir) = test_writer_and_readers();
    let clock: Arc<dyn Clock + Send + Sync> = Arc::new(SystemClock);
    let svc = DeviceService::new(writer, readers, clock);
    (svc, dir)
}

/// Сеет реальную строку `users` (FK-цель для `place_movements.user_id`) и
/// возвращает `Identity` менеджера. Вымышленное имя — privacy gate (CLAUDE.md).
async fn seed_manager_caller(writer: &WriterHandle) -> Identity {
    let now = SystemClock.unix_seconds();
    let user_id = writer
        .execute(move |conn| {
            let tx = conn.transaction().map_err(|e| AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            tx.execute(
                "INSERT INTO users \
                 (login, full_name, password_hash, role, ad_user, is_active, \
                  created_at_utc, updated_at_utc, version) \
                 VALUES ('ivanov.ii', 'Иванов И.И.', NULL, 'manager', 0, 1, ?1, ?1, 1)",
                rusqlite::params![now],
            )
            .map_err(|e| AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            let user_id = tx.last_insert_rowid();
            tx.commit().map_err(|e| AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            Ok(user_id)
        })
        .await
        .expect("seed manager user");
    Identity {
        user_id: Some(user_id),
        role: Role::Manager,
    }
}

/// Сеет строку `places` напрямую (мимо PlaceService — как в `cartridges_crud.rs`).
async fn seed_place(writer: &WriterHandle, name: &str) -> i64 {
    let name = name.to_string();
    writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO places (kind, name, is_storage, created_at_utc, updated_at_utc, version) \
                 VALUES ('room', ?1, 0, ?2, ?2, 1)",
                rusqlite::params![name, 1_700_000_000_i64],
            )
            .map_err(|e| AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed place")
}

fn minimal_new(name: &str, place_id: Option<i64>) -> DeviceNew {
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

async fn count_movements(
    readers: Arc<trackly_infra::db::pools::ReaderPool>,
    entity_id: i64,
) -> i64 {
    tokio::task::spawn_blocking(move || {
        let conn = readers.acquire();
        conn.query_row(
            "SELECT COUNT(*) FROM place_movements WHERE entity_type='device' AND entity_id=?1",
            rusqlite::params![entity_id],
            |r| r.get(0),
        )
    })
    .await
    .expect("spawn_blocking")
    .expect("count place_movements")
}

// ---------------------------------------------------------------------------
// place_movements_manual_device
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn place_movements_manual_device() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let place_a = seed_place(&svc.writer, "Каб. 101").await;
        let place_b = seed_place(&svc.writer, "Склад").await;
        let manager = seed_manager_caller(&svc.writer).await;

        let dto = svc
            .create(minimal_new("Ноутбук Dell", Some(place_a)))
            .await
            .expect("create device at place A");

        svc.update(
            &manager,
            dto.id,
            dto.version,
            DevicePatch {
                type_id: None,
                name: None,
                inventory_no: None,
                serial_no: None,
                model: None,
                specs: None,
                kit: None,
                state: None,
                place_id: Some(Some(place_b)),
                status_id: None,
            },
        )
        .await
        .expect("update device place A -> B");

        let readers = svc.readers.clone();
        let entity_id = dto.id;
        let row: (String, i64, i64, String, Option<i64>) = tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            conn.query_row(
                "SELECT source, from_place_id, to_place_id, entity_type, user_id \
                 FROM place_movements WHERE entity_type='device' AND entity_id=?1",
                rusqlite::params![entity_id],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, i64>(1)?,
                        r.get::<_, i64>(2)?,
                        r.get::<_, String>(3)?,
                        r.get::<_, Option<i64>>(4)?,
                    ))
                },
            )
        })
        .await
        .expect("spawn_blocking")
        .expect("query place_movements row");

        let (source, from_place_id, to_place_id, entity_type, user_id) = row;
        assert_eq!(
            source, "manual",
            "D-27: manual edit flow -> source='manual'"
        );
        assert_eq!(from_place_id, place_a);
        assert_eq!(to_place_id, place_b);
        assert_eq!(entity_type, "device");
        assert_eq!(
            user_id, manager.user_id,
            "place_movements.user_id должен совпадать с caller.user_id"
        );

        let count = count_movements(svc.readers.clone(), dto.id).await;
        assert_eq!(
            count, 1,
            "должна быть ровно одна запись place_movements, получили {count}"
        );
    })
    .await
    .expect("place_movements_manual_device exceeded 30 s budget");
}

// ---------------------------------------------------------------------------
// place_movements_manual_device_status_only_noop
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn place_movements_manual_device_status_only_noop() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let place_a = seed_place(&svc.writer, "Каб. 202").await;
        let manager = seed_manager_caller(&svc.writer).await;

        let dto = svc
            .create(minimal_new("Принтер HP", Some(place_a)))
            .await
            .expect("create device at place A");

        // Меняем только status_id — place_id не тронут (D-04: не движение).
        svc.update(
            &manager,
            dto.id,
            dto.version,
            DevicePatch {
                type_id: None,
                name: None,
                inventory_no: None,
                serial_no: None,
                model: None,
                specs: None,
                kit: None,
                state: None,
                place_id: None,
                status_id: Some(1),
            },
        )
        .await
        .expect("update device status only");

        let count = count_movements(svc.readers.clone(), dto.id).await;
        assert_eq!(
            count, 0,
            "status-only правка не должна создавать place_movements (D-04), получили {count}"
        );
    })
    .await
    .expect("place_movements_manual_device_status_only_noop exceeded 30 s budget");
}

// ---------------------------------------------------------------------------
// place_movements_manual_device_first_assignment_noop
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn place_movements_manual_device_first_assignment_noop() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let place_a = seed_place(&svc.writer, "Каб. 303").await;
        let manager = seed_manager_caller(&svc.writer).await;

        let dto = svc
            .create(minimal_new("Монитор Samsung", None))
            .await
            .expect("create device with no place");

        // Первое присвоение места (NULL -> place_a) — D-06: не движение.
        svc.update(
            &manager,
            dto.id,
            dto.version,
            DevicePatch {
                type_id: None,
                name: None,
                inventory_no: None,
                serial_no: None,
                model: None,
                specs: None,
                kit: None,
                state: None,
                place_id: Some(Some(place_a)),
                status_id: None,
            },
        )
        .await
        .expect("update device first place assignment");

        let count = count_movements(svc.readers.clone(), dto.id).await;
        assert_eq!(
            count, 0,
            "первое присвоение места не должно создавать place_movements (D-06), получили {count}"
        );
    })
    .await
    .expect("place_movements_manual_device_first_assignment_noop exceeded 30 s budget");
}
