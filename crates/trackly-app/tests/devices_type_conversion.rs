//! Quick 260820-rdj: интеграционные тесты атомарной синхронизации `printers`
//! при конверсии типа устройства (Устройство ⇄ Принтер) через
//! `DeviceService::create/update/bulk_create`.
//!
//! Каждый тест обёрнут в `tokio::time::timeout(30s)` — защита от deadlock
//! (PATTERNS.md §Pattern 4), паттерн харнесса — `devices_crud.rs`.
//!
//! Фиктивные имена устройств (приватность — CLAUDE.md): "Test Printer 1",
//! "Test Device 1" и т.п. — никаких реальных данных организации.

use std::sync::Arc;
use std::time::Duration;

use rusqlite::params;

use trackly_app::dto::device::{DeviceNew, DevicePatch};
use trackly_app::services::DeviceService;
use trackly_core::auth::Identity;
use trackly_core::primitives::clock::Clock;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::test_support::test_writer_and_readers;

const DEVICE_TYPE_ID: i64 = 1;
const PRINTER_TYPE_ID: i64 = 2;

/// Создаёт тестовый `DeviceService` поверх свежего tempfile DB.
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

/// `DeviceNew` с минимальными обязательными полями и заданным `type_id`.
fn minimal_new(name: &str, type_id: i64) -> DeviceNew {
    DeviceNew {
        type_id,
        name: name.to_string(),
        inventory_no: None,
        serial_no: None,
        model: None,
        specs: None,
        kit: None,
        state: None,
        place_id: None,
        status_id: 1,
    }
}

/// `DevicePatch`, меняющий только `type_id` (остальные поля — no-op `None`).
fn type_patch(type_id: Option<i64>) -> DevicePatch {
    DevicePatch {
        type_id,
        name: None,
        inventory_no: None,
        serial_no: None,
        model: None,
        specs: None,
        kit: None,
        state: None,
        place_id: None,
        status_id: None,
    }
}

/// Прочитать строку `printers` (ip_address, community, snmp_version) для device_id,
/// если она есть.
async fn fetch_printer_row(
    svc: &DeviceService,
    device_id: i64,
) -> Option<(Option<String>, String, String)> {
    let readers = svc.readers.clone();
    tokio::task::spawn_blocking(move || {
        let conn = readers.acquire();
        conn.query_row(
            "SELECT ip_address, community, snmp_version FROM printers WHERE device_id = ?1",
            params![device_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .ok()
    })
    .await
    .expect("spawn_blocking")
}

async fn printers_count_for_device(svc: &DeviceService, device_id: i64) -> i64 {
    let readers = svc.readers.clone();
    tokio::task::spawn_blocking(move || {
        let conn = readers.acquire();
        conn.query_row(
            "SELECT COUNT(*) FROM printers WHERE device_id = ?1",
            params![device_id],
            |r| r.get(0),
        )
        .expect("count printers")
    })
    .await
    .expect("spawn_blocking")
}

/// Сидит `printer_readings`/`printer_alerts` для printer_id (напрямую через writer,
/// имитирует историю мониторинга перед downgrade-конверсией).
async fn seed_monitoring_history(svc: &DeviceService, printer_id: i64) {
    svc.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO printer_readings (printer_id, ts_utc, toner_levels, page_count, status) \
                 VALUES (?1, ?2, '{\"black\":{\"level\":42}}', 100, 'ok')",
                params![printer_id, 1_700_000_000_i64],
            )
            .expect("seed reading");
            conn.execute(
                "INSERT INTO printer_alerts (printer_id, alert_type, first_seen_utc, last_seen_utc) \
                 VALUES (?1, 'offline', ?2, ?2)",
                params![printer_id, 1_700_000_000_i64],
            )
            .expect("seed alert");
            Ok(())
        })
        .await
        .expect("seed monitoring history");
}

async fn printer_id_for_device(svc: &DeviceService, device_id: i64) -> i64 {
    let readers = svc.readers.clone();
    tokio::task::spawn_blocking(move || {
        let conn = readers.acquire();
        conn.query_row(
            "SELECT id FROM printers WHERE device_id = ?1",
            params![device_id],
            |r| r.get(0),
        )
        .expect("printer id")
    })
    .await
    .expect("spawn_blocking")
}

async fn readings_and_alerts_count(svc: &DeviceService, printer_id: i64) -> (i64, i64) {
    let readers = svc.readers.clone();
    tokio::task::spawn_blocking(move || {
        let conn = readers.acquire();
        let readings: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM printer_readings WHERE printer_id = ?1",
                params![printer_id],
                |r| r.get(0),
            )
            .expect("count readings");
        let alerts: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM printer_alerts WHERE printer_id = ?1",
                params![printer_id],
                |r| r.get(0),
            )
            .expect("count alerts");
        (readings, alerts)
    })
    .await
    .expect("spawn_blocking")
}

// ---------------------------------------------------------------------------
// create() с type_id=2 → строка printers с дефолтами
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_with_printer_type_creates_printers_row_with_defaults() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let dto = svc
            .create(minimal_new("Test Printer 1", PRINTER_TYPE_ID))
            .await
            .expect("create printer-type device");

        let row = fetch_printer_row(&svc, dto.id)
            .await
            .expect("printers row must exist");
        assert_eq!(row.0, None, "ip_address must default to NULL");
        assert_eq!(row.1, "public", "community must default to 'public'");
        assert_eq!(row.2, "v2c", "snmp_version must default to 'v2c'");
    })
    .await
    .expect("test timed out");
}

// ---------------------------------------------------------------------------
// create() с type_id=1 (default) → НЕТ строки printers
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_with_device_type_creates_no_printers_row() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let dto = svc
            .create(minimal_new("Test Device 1", DEVICE_TYPE_ID))
            .await
            .expect("create device-type device");

        assert_eq!(
            printers_count_for_device(&svc, dto.id).await,
            0,
            "device-type devices must not get a printers row"
        );
    })
    .await
    .expect("test timed out");
}

// ---------------------------------------------------------------------------
// update() 1→2 → строка printers появляется
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn update_upgrade_device_to_printer_creates_printers_row() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let dto = svc
            .create(minimal_new("Test Device 2", DEVICE_TYPE_ID))
            .await
            .expect("create device-type device");
        assert_eq!(printers_count_for_device(&svc, dto.id).await, 0);

        let updated = svc
            .update(
                &admin_caller(),
                dto.id,
                dto.version,
                type_patch(Some(PRINTER_TYPE_ID)),
            )
            .await
            .expect("upgrade to printer");

        assert_eq!(updated.type_id, PRINTER_TYPE_ID);
        assert_eq!(
            printers_count_for_device(&svc, dto.id).await,
            1,
            "printers row must appear after upgrade"
        );
    })
    .await
    .expect("test timed out");
}

// ---------------------------------------------------------------------------
// update() 2→1 с существующей историей мониторинга → cascade delete
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn update_downgrade_printer_to_device_cascades_monitoring_history() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let dto = svc
            .create(minimal_new("Test Printer 2", PRINTER_TYPE_ID))
            .await
            .expect("create printer-type device");

        let printer_id = printer_id_for_device(&svc, dto.id).await;
        seed_monitoring_history(&svc, printer_id).await;
        let (readings_before, alerts_before) = readings_and_alerts_count(&svc, printer_id).await;
        assert_eq!(readings_before, 1, "reading should be seeded");
        assert_eq!(alerts_before, 1, "alert should be seeded");

        let updated = svc
            .update(
                &admin_caller(),
                dto.id,
                dto.version,
                type_patch(Some(DEVICE_TYPE_ID)),
            )
            .await
            .expect("downgrade to device");

        assert_eq!(updated.type_id, DEVICE_TYPE_ID);
        assert_eq!(
            printers_count_for_device(&svc, dto.id).await,
            0,
            "printers row must be gone after downgrade"
        );
        let (readings_after, alerts_after) = readings_and_alerts_count(&svc, printer_id).await;
        assert_eq!(readings_after, 0, "printer_readings must cascade-delete");
        assert_eq!(alerts_after, 0, "printer_alerts must cascade-delete");
    })
    .await
    .expect("test timed out");
}

// ---------------------------------------------------------------------------
// update() без смены типа, вызванный дважды подряд → идемпотентность
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn update_without_type_change_called_twice_stays_idempotent() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let dto = svc
            .create(minimal_new("Test Printer 3", PRINTER_TYPE_ID))
            .await
            .expect("create printer-type device");
        assert_eq!(printers_count_for_device(&svc, dto.id).await, 1);

        // Первый update() без смены типа (patch.type_id = None).
        let updated1 = svc
            .update(&admin_caller(), dto.id, dto.version, type_patch(None))
            .await
            .expect("first no-op-type update");
        assert_eq!(updated1.type_id, PRINTER_TYPE_ID);
        assert_eq!(
            printers_count_for_device(&svc, dto.id).await,
            1,
            "still exactly one printers row after first update"
        );

        // Второй update() без смены типа — не должен создать дубликат.
        let updated2 = svc
            .update(
                &admin_caller(),
                updated1.id,
                updated1.version,
                type_patch(None),
            )
            .await
            .expect("second no-op-type update");
        assert_eq!(updated2.type_id, PRINTER_TYPE_ID);
        assert_eq!(
            printers_count_for_device(&svc, dto.id).await,
            1,
            "no duplicate printers row after second update"
        );
    })
    .await
    .expect("test timed out");
}

// ---------------------------------------------------------------------------
// bulk_create() с type_id=2, count=3 → 3 строки printers
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn bulk_create_with_printer_type_creates_printers_row_per_device() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let dtos = svc
            .bulk_create(minimal_new("Test Printer Bulk", PRINTER_TYPE_ID), 3)
            .await
            .expect("bulk create printers");

        assert_eq!(dtos.len(), 3);
        for dto in &dtos {
            assert_eq!(
                printers_count_for_device(&svc, dto.id).await,
                1,
                "each bulk-created printer-type device must get exactly one printers row"
            );
        }
    })
    .await
    .expect("test timed out");
}
