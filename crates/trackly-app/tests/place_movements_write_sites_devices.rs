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

// ---------------------------------------------------------------------------
// Plan 40-21: printer -> attached cartridges place cascade
// ---------------------------------------------------------------------------

/// Сеет принтер (type_id=2) напрямую через SQL, мимо `PrinterService` — мимикрия
/// `cartridges_lifecycle.rs::seed_printer_device` в этом файле.
async fn seed_printer_device(writer: &WriterHandle, name: &str) -> (i64, i64) {
    let name = name.to_string();
    writer
        .execute(move |conn| {
            let tx = conn.transaction().map_err(|e| AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            tx.execute(
                "INSERT INTO devices \
                 (type_id, name, status_id, version, created_at_utc, updated_at_utc) \
                 VALUES (2, ?1, 2, 1, ?2, ?2)",
                rusqlite::params![name, 1_700_000_000_i64],
            )
            .map_err(|e| AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            let id = tx.last_insert_rowid();
            tx.commit().map_err(|e| AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            Ok((id, 1_i64))
        })
        .await
        .expect("seed printer device")
}

/// Сеет картридж напрямую через SQL, привязанный к принтеру `printer_id`
/// с местом `place_id` — обходит `CartridgeService`, т.к. этот файл тестирует
/// только device-семейство write-путей.
async fn seed_cartridge_attached_to_printer(
    writer: &WriterHandle,
    printer_id: i64,
    place_id: i64,
) -> i64 {
    writer
        .execute(move |conn| {
            let tx = conn.transaction().map_err(|e| AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            // Минимальная модель-заглушка (FK model_id -> cartridge_models).
            tx.execute(
                "INSERT INTO cartridge_models \
                 (brand, model, kind_id, created_at_utc, updated_at_utc, version) \
                 VALUES ('HP', 'CE285A', 1, ?1, ?1, 1)",
                rusqlite::params![1_700_000_000_i64],
            )
            .map_err(|e| AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            let model_id = tx.last_insert_rowid();
            tx.execute(
                "INSERT INTO cartridges \
                 (model_id, code, status_id, state_id, place_id, current_printer_device_id, \
                  created_at_utc, updated_at_utc, version) \
                 VALUES (?1, 'C-0001', 2, 1, ?2, ?3, ?4, ?4, 1)",
                rusqlite::params![model_id, place_id, printer_id, 1_700_000_000_i64],
            )
            .map_err(|e| AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            let cartridge_id = tx.last_insert_rowid();
            tx.commit().map_err(|e| AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            Ok(cartridge_id)
        })
        .await
        .expect("seed cartridge attached to printer")
}

async fn cartridge_place_and_version(
    writer: &WriterHandle,
    cartridge_id: i64,
) -> (Option<i64>, i64) {
    writer
        .execute(move |conn| {
            conn.query_row(
                "SELECT place_id, version FROM cartridges WHERE id = ?1",
                rusqlite::params![cartridge_id],
                |r| Ok((r.get::<_, Option<i64>>(0)?, r.get::<_, i64>(1)?)),
            )
            .map_err(|e| AppError::Internal {
                source_chain: format!("{e}"),
            })
        })
        .await
        .expect("read cartridge place/version")
}

async fn count_cartridge_movements(
    readers: Arc<trackly_infra::db::pools::ReaderPool>,
    entity_id: i64,
) -> i64 {
    tokio::task::spawn_blocking(move || {
        let conn = readers.acquire();
        conn.query_row(
            "SELECT COUNT(*) FROM place_movements WHERE entity_type='cartridge' AND entity_id=?1",
            rusqlite::params![entity_id],
            |r| r.get(0),
        )
    })
    .await
    .expect("spawn_blocking")
    .expect("count cartridge place_movements")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn update_cascades_place_to_attached_cartridges() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let place_a = seed_place(&svc.writer, "Каб. 401").await;
        let place_b = seed_place(&svc.writer, "Каб. 402").await;
        let manager = seed_manager_caller(&svc.writer).await;
        let (printer_id, printer_version) =
            seed_printer_device(&svc.writer, "Pantum BM5100ADN").await;
        let cartridge_id =
            seed_cartridge_attached_to_printer(&svc.writer, printer_id, place_a).await;

        svc.update(
            &manager,
            printer_id,
            printer_version,
            DevicePatch {
                place_id: Some(Some(place_b)),
                ..Default::default()
            },
        )
        .await
        .expect("update printer place A -> B");

        let (cart_place, cart_version) =
            cartridge_place_and_version(&svc.writer, cartridge_id).await;
        assert_eq!(
            cart_place,
            Some(place_b),
            "картридж должен переехать вместе с принтером"
        );
        assert_eq!(cart_version, 2, "version картриджа должна увеличиться на 1");

        let count = count_cartridge_movements(svc.readers.clone(), cartridge_id).await;
        assert_eq!(
            count, 1,
            "должна быть ровно одна новая запись place_movements для картриджа, получили {count}"
        );

        let (from_place_id, to_place_id, source, note): (i64, i64, String, Option<String>) =
            tokio::task::spawn_blocking({
                let readers = svc.readers.clone();
                move || {
                    let conn = readers.acquire();
                    conn.query_row(
                        "SELECT from_place_id, to_place_id, source, note FROM place_movements \
                         WHERE entity_type='cartridge' AND entity_id=?1",
                        rusqlite::params![cartridge_id],
                        |r| {
                            Ok((
                                r.get::<_, i64>(0)?,
                                r.get::<_, i64>(1)?,
                                r.get::<_, String>(2)?,
                                r.get::<_, Option<String>>(3)?,
                            ))
                        },
                    )
                }
            })
            .await
            .expect("spawn_blocking")
            .expect("query cartridge movement row");

        assert_eq!(from_place_id, place_a);
        assert_eq!(to_place_id, place_b);
        assert_eq!(source, "manual");
        assert!(
            note.as_deref()
                .unwrap_or_default()
                .contains("вместе с принтером"),
            "note должна упоминать «вместе с принтером», получили {note:?}"
        );
    })
    .await
    .expect("update_cascades_place_to_attached_cartridges exceeded 30 s budget");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn update_with_no_place_change_does_not_touch_cartridges() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let place_a = seed_place(&svc.writer, "Каб. 501").await;
        let manager = seed_manager_caller(&svc.writer).await;
        let (printer_id, printer_version) =
            seed_printer_device(&svc.writer, "Kyocera ECOSYS").await;
        let cartridge_id =
            seed_cartridge_attached_to_printer(&svc.writer, printer_id, place_a).await;

        // Меняем только status_id — место принтера не трогаем.
        svc.update(
            &manager,
            printer_id,
            printer_version,
            DevicePatch {
                status_id: Some(2),
                ..Default::default()
            },
        )
        .await
        .expect("update printer status only");

        let (cart_place, cart_version) =
            cartridge_place_and_version(&svc.writer, cartridge_id).await;
        assert_eq!(
            cart_place,
            Some(place_a),
            "место картриджа не должно меняться"
        );
        assert_eq!(cart_version, 1, "version картриджа не должна увеличиваться");

        let count = count_cartridge_movements(svc.readers.clone(), cartridge_id).await;
        assert_eq!(
            count, 0,
            "не должно быть новых записей place_movements для картриджа, получили {count}"
        );
    })
    .await
    .expect("update_with_no_place_change_does_not_touch_cartridges exceeded 30 s budget");
}

// ---------------------------------------------------------------------------
// Plan 40-28 (CR-03, 40-VERIFICATION.md gap 1): clearing a printer's place
// (Some -> None) must NOT touch attached cartridges' places.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn update_clearing_printer_place_does_not_touch_cartridges() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let place_a = seed_place(&svc.writer, "Каб. 601").await;
        let manager = seed_manager_caller(&svc.writer).await;
        let (printer_id, printer_version) =
            seed_printer_device(&svc.writer, "Xerox Phaser 3320").await;

        // Принтер сеется без места (place_id = NULL). Первое присвоение места
        // (NULL -> place_a) — обычный переезд принтера на A, version становится 2.
        let dto = svc
            .update(
                &manager,
                printer_id,
                printer_version,
                DevicePatch {
                    place_id: Some(Some(place_a)),
                    ..Default::default()
                },
            )
            .await
            .expect("update printer NULL -> place A");

        // Картридж сеется уже прикреплённым к принтеру с местом = A — имитирует
        // состояние ПОСЛЕ реального каскада (which the first update above did not
        // itself trigger, since no cartridge was attached yet).
        let cartridge_id =
            seed_cartridge_attached_to_printer(&svc.writer, printer_id, place_a).await;

        // Очищаем место принтера: place A -> None. `after.place_id == None`.
        svc.update(
            &manager,
            printer_id,
            dto.version,
            DevicePatch {
                place_id: Some(None),
                ..Default::default()
            },
        )
        .await
        .expect("update printer place A -> None (clear)");

        let (cart_place, cart_version) =
            cartridge_place_and_version(&svc.writer, cartridge_id).await;
        assert_eq!(
            cart_place,
            Some(place_a),
            "CR-03: очистка места принтера НЕ должна трогать место картриджа"
        );
        assert_eq!(
            cart_version, 1,
            "CR-03: version картриджа НЕ должна увеличиваться при очистке места принтера"
        );

        let count = count_cartridge_movements(svc.readers.clone(), cartridge_id).await;
        assert_eq!(
            count, 0,
            "CR-03: очистка места принтера не должна писать place_movements для картриджа, получили {count}"
        );
    })
    .await
    .expect("update_clearing_printer_place_does_not_touch_cartridges exceeded 30 s budget");
}
