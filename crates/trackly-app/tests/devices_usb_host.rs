//! Phase 41.13 Plan 04 (PGR-03): регрессионные тесты записи USB-хоста принтера
//! через `DeviceService` (создание блоком принтера и редактирование патчем).
//!
//! Что закрепляет файл:
//!   * T1-T3  — создание: хост читается из БД сразу после Ok; несуществующий и
//!     мягко удалённый хост отклоняются и ничего не оставляют в БД (атомарность);
//!   * T4-T5  — редактирование СУЩЕСТВУЮЩЕГО принтера: задать, заменить, сбросить.
//!     Это ловушка exists-гарда `sync_printer_row_in_tx`: ветка создания строки
//!     `printers` на существующей строке молчит, поэтому хост при правке обязан
//!     писаться отдельной операцией;
//!   * T6     — правка названия без поля `usb_host_device_id` хост НЕ трогает.
//!     ВАЖНО: фикстура этого теста несёт НЕпустой хост ДО обновления (и проверяет
//!     это чтением), иначе `NULL -> NULL` ничего не доказывает;
//!   * T7-T9  — самоссылка, несуществующий, мягко удалённый хост и хост на
//!     устройстве не-принтере отклоняются `Validation(usb_host_device_id)`,
//!     прежнее значение остаётся;
//!   * T10    — Принтер -> Устройство -> Принтер: хост пуст (существующее
//!     поведение, закреплено);
//!   * T11    — один патч меняет тип на «Принтер» И задаёт хост (порядок: сначала
//!     sync строки принтера, затем запись хоста).
//!
//! Каждый тест обёрнут в `tokio::time::timeout(30s)` (защита от deadlock).
//!
//! Приватность (CLAUDE.md): только вымышленные названия («Принтер А»,
//! «Рабочая станция А», «Рабочая станция Б»), инв. «ИНВ-114».

use std::sync::Arc;
use std::time::Duration;

use rusqlite::params;

use trackly_app::dto::device::{DeviceDto, DeviceNew, DevicePatch};
use trackly_app::dto::number_template::{NumberFieldInput, TemplateContextDto};
use trackly_app::dto::printer::PrinterCreateBlockDto;
use trackly_app::services::DeviceService;
use trackly_core::auth::Identity;
use trackly_core::error::AppError;
use trackly_core::primitives::clock::Clock;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::test_support::test_writer_and_readers;

const DEVICE_TYPE_ID: i64 = 1;
const PRINTER_TYPE_ID: i64 = 2;
const FIELD: &str = "usb_host_device_id";

fn make_service() -> (DeviceService, tempfile::TempDir) {
    let (writer, readers, dir) = test_writer_and_readers();
    let clock: Arc<dyn Clock + Send + Sync> = Arc::new(SystemClock);
    let svc = DeviceService::new(writer, readers, clock);
    (svc, dir)
}

fn admin_caller() -> Identity {
    Identity::trusted_admin()
}

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

fn empty_number_input() -> NumberFieldInput {
    NumberFieldInput {
        value: String::new(),
        template_id: None,
        confirm_mismatch: false,
        confirm_script_mix: false,
    }
}

/// Создать устройство (без блока принтера) указанного типа.
async fn create_device(svc: &DeviceService, name: &str, type_id: i64) -> DeviceDto {
    svc.create_single_with_number_check_with_printer(
        minimal_new(name, type_id),
        empty_number_input(),
        TemplateContextDto::PrinterCreate,
        None,
    )
    .await
    .expect("create device")
    .expect_created("create device")
}

/// Создать принтер сразу с хостом (блок принтера без IP).
async fn create_printer_with_host(svc: &DeviceService, name: &str, host: i64) -> DeviceDto {
    svc.create_single_with_number_check_with_printer(
        minimal_new(name, PRINTER_TYPE_ID),
        empty_number_input(),
        TemplateContextDto::PrinterCreate,
        Some(PrinterCreateBlockDto {
            ip_address: None,
            community: None,
            usb_host_device_id: Some(host),
        }),
    )
    .await
    .expect("create printer with host")
    .expect_created("create printer with host")
}

/// `printers.usb_host_device_id` для устройства; паникует, если строки нет.
async fn fetch_usb_host(svc: &DeviceService, device_id: i64) -> Option<i64> {
    let readers = svc.readers.clone();
    tokio::task::spawn_blocking(move || {
        let conn = readers.acquire();
        conn.query_row(
            "SELECT usb_host_device_id FROM printers WHERE device_id = ?1",
            params![device_id],
            |r| r.get(0),
        )
        .expect("printers row must exist")
    })
    .await
    .expect("spawn_blocking")
}

async fn count(svc: &DeviceService, sql: &'static str) -> i64 {
    let readers = svc.readers.clone();
    tokio::task::spawn_blocking(move || {
        let conn = readers.acquire();
        conn.query_row(sql, [], |r| r.get(0)).expect("count")
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

async fn ip_address_of(svc: &DeviceService, device_id: i64) -> Option<String> {
    let readers = svc.readers.clone();
    tokio::task::spawn_blocking(move || {
        let conn = readers.acquire();
        conn.query_row(
            "SELECT ip_address FROM printers WHERE device_id = ?1",
            params![device_id],
            |r| r.get(0),
        )
        .expect("printers row")
    })
    .await
    .expect("spawn_blocking")
}

fn host_patch(host: Option<i64>) -> DevicePatch {
    DevicePatch {
        usb_host_device_id: Some(host),
        ..Default::default()
    }
}

async fn update_ok(svc: &DeviceService, dto: &DeviceDto, patch: DevicePatch) -> DeviceDto {
    svc.update(&admin_caller(), dto.id, dto.version, patch)
        .await
        .expect("update must succeed")
        .expect_created("update must succeed")
}

fn assert_host_validation(res: Result<impl std::fmt::Debug, AppError>, what: &str) {
    match res {
        Err(AppError::Validation { field, .. }) => {
            assert_eq!(field, FIELD, "{what}: поле ошибки");
        }
        other => panic!("{what}: expected AppError::Validation, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// T1-T3: создание
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn t1_create_with_host_block_persists_host_and_null_ip() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let host = create_device(&svc, "Рабочая станция А", DEVICE_TYPE_ID).await;
        let printer = create_printer_with_host(&svc, "Принтер А", host.id).await;

        assert_eq!(fetch_usb_host(&svc, printer.id).await, Some(host.id));
        assert_eq!(
            ip_address_of(&svc, printer.id).await,
            None,
            "IP остаётся NULL"
        );
    })
    .await
    .expect("t1 exceeded 30 s budget");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn t2_create_with_missing_host_is_rejected_atomically() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let _anchor = create_device(&svc, "Рабочая станция А", DEVICE_TYPE_ID).await;
        let devices_before = count(&svc, "SELECT COUNT(*) FROM devices").await;
        let printers_before = count(&svc, "SELECT COUNT(*) FROM printers").await;

        let res = svc
            .create_single_with_number_check_with_printer(
                minimal_new("Принтер А", PRINTER_TYPE_ID),
                empty_number_input(),
                TemplateContextDto::PrinterCreate,
                Some(PrinterCreateBlockDto {
                    ip_address: None,
                    community: None,
                    usb_host_device_id: Some(999_999),
                }),
            )
            .await;
        assert_host_validation(res, "несуществующий хост при создании");

        assert_eq!(
            count(&svc, "SELECT COUNT(*) FROM devices").await,
            devices_before,
            "устройство не должно появиться (атомарность)"
        );
        assert_eq!(
            count(&svc, "SELECT COUNT(*) FROM printers").await,
            printers_before,
            "строка printers не должна появиться (атомарность)"
        );
    })
    .await
    .expect("t2 exceeded 30 s budget");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn t3_create_with_soft_deleted_host_is_rejected() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let host = create_device(&svc, "Рабочая станция А", DEVICE_TYPE_ID).await;
        svc.delete_soft(host.id, host.version)
            .await
            .expect("soft delete host");
        let printers_before = count(&svc, "SELECT COUNT(*) FROM printers").await;

        let res = svc
            .create_single_with_number_check_with_printer(
                minimal_new("Принтер А", PRINTER_TYPE_ID),
                empty_number_input(),
                TemplateContextDto::PrinterCreate,
                Some(PrinterCreateBlockDto {
                    ip_address: None,
                    community: None,
                    usb_host_device_id: Some(host.id),
                }),
            )
            .await;
        assert_host_validation(res, "мягко удалённый хост при создании");
        assert_eq!(
            count(&svc, "SELECT COUNT(*) FROM printers").await,
            printers_before,
            "строка printers не должна появиться"
        );
    })
    .await
    .expect("t3 exceeded 30 s budget");
}

// ---------------------------------------------------------------------------
// T4-T6: редактирование
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn t4_update_sets_host_on_existing_printer_row() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let host = create_device(&svc, "Рабочая станция А", DEVICE_TYPE_ID).await;
        let printer = create_device(&svc, "Принтер А", PRINTER_TYPE_ID).await;
        assert_eq!(printers_count_for_device(&svc, printer.id).await, 1);
        assert_eq!(
            fetch_usb_host(&svc, printer.id).await,
            None,
            "до: хоста нет"
        );

        update_ok(&svc, &printer, host_patch(Some(host.id))).await;

        assert_eq!(
            fetch_usb_host(&svc, printer.id).await,
            Some(host.id),
            "хост обязан записаться на существующей строке printers"
        );
    })
    .await
    .expect("t4 exceeded 30 s budget");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn t5_update_replaces_then_resets_host() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let h1 = create_device(&svc, "Рабочая станция А", DEVICE_TYPE_ID).await;
        let h2 = create_device(&svc, "Рабочая станция Б", DEVICE_TYPE_ID).await;
        let printer = create_printer_with_host(&svc, "Принтер А", h1.id).await;
        assert_eq!(fetch_usb_host(&svc, printer.id).await, Some(h1.id));

        let after_replace = update_ok(&svc, &printer, host_patch(Some(h2.id))).await;
        assert_eq!(
            fetch_usb_host(&svc, printer.id).await,
            Some(h2.id),
            "замена"
        );

        update_ok(&svc, &after_replace, host_patch(None)).await;
        assert_eq!(fetch_usb_host(&svc, printer.id).await, None, "сброс null");
    })
    .await
    .expect("t5 exceeded 30 s budget");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn t6_update_without_host_field_does_not_touch_host() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let h1 = create_device(&svc, "Рабочая станция А", DEVICE_TYPE_ID).await;
        // Фикстура НЕ вакуумна: у принтера до обновления стоит НЕПУСТОЙ хост.
        let printer = create_printer_with_host(&svc, "Принтер А", h1.id).await;
        assert_eq!(
            fetch_usb_host(&svc, printer.id).await,
            Some(h1.id),
            "ПРЕДУСЛОВИЕ: хост до обновления непустой"
        );

        let renamed = update_ok(
            &svc,
            &printer,
            DevicePatch {
                name: Some("Принтер А (правка)".into()),
                ..Default::default()
            },
        )
        .await;
        assert_eq!(renamed.name, "Принтер А (правка)");
        assert_eq!(
            fetch_usb_host(&svc, printer.id).await,
            Some(h1.id),
            "правка названия без поля хоста не должна менять хост"
        );

        // Повтор без смены типа (идемпотентный sync) тоже хост не трогает.
        update_ok(
            &svc,
            &renamed,
            DevicePatch {
                type_id: Some(PRINTER_TYPE_ID),
                ..Default::default()
            },
        )
        .await;
        assert_eq!(fetch_usb_host(&svc, printer.id).await, Some(h1.id));
    })
    .await
    .expect("t6 exceeded 30 s budget");
}

// ---------------------------------------------------------------------------
// T7-T9: отказы
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn t7_update_with_self_as_host_is_rejected_and_keeps_old_host() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let h1 = create_device(&svc, "Рабочая станция А", DEVICE_TYPE_ID).await;
        let printer = create_printer_with_host(&svc, "Принтер А", h1.id).await;

        let res = svc
            .update(
                &admin_caller(),
                printer.id,
                printer.version,
                host_patch(Some(printer.id)),
            )
            .await;
        assert_host_validation(res, "самоссылка");
        assert_eq!(fetch_usb_host(&svc, printer.id).await, Some(h1.id));
    })
    .await
    .expect("t7 exceeded 30 s budget");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn t8_update_with_missing_or_soft_deleted_host_is_rejected() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let h1 = create_device(&svc, "Рабочая станция А", DEVICE_TYPE_ID).await;
        let h2 = create_device(&svc, "Рабочая станция Б", DEVICE_TYPE_ID).await;
        let printer = create_printer_with_host(&svc, "Принтер А", h1.id).await;

        let res = svc
            .update(
                &admin_caller(),
                printer.id,
                printer.version,
                host_patch(Some(999_999)),
            )
            .await;
        assert_host_validation(res, "несуществующий хост при правке");
        assert_eq!(fetch_usb_host(&svc, printer.id).await, Some(h1.id));

        svc.delete_soft(h2.id, h2.version)
            .await
            .expect("soft delete h2");
        let res = svc
            .update(
                &admin_caller(),
                printer.id,
                printer.version,
                host_patch(Some(h2.id)),
            )
            .await;
        assert_host_validation(res, "мягко удалённый хост при правке");
        assert_eq!(
            fetch_usb_host(&svc, printer.id).await,
            Some(h1.id),
            "прежний хост остался"
        );
    })
    .await
    .expect("t8 exceeded 30 s budget");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn t9_update_with_host_on_non_printer_is_rejected() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let h1 = create_device(&svc, "Рабочая станция А", DEVICE_TYPE_ID).await;
        let dev = create_device(&svc, "Рабочая станция Б", DEVICE_TYPE_ID).await;

        let res = svc
            .update(
                &admin_caller(),
                dev.id,
                dev.version,
                host_patch(Some(h1.id)),
            )
            .await;
        assert_host_validation(res, "хост на не-принтере");
        assert_eq!(printers_count_for_device(&svc, dev.id).await, 0);
    })
    .await
    .expect("t9 exceeded 30 s budget");
}

// ---------------------------------------------------------------------------
// T10-T11: смена типа
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn t10_printer_device_printer_roundtrip_leaves_host_empty() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let h1 = create_device(&svc, "Рабочая станция А", DEVICE_TYPE_ID).await;
        let printer = create_printer_with_host(&svc, "Принтер А", h1.id).await;
        assert_eq!(
            fetch_usb_host(&svc, printer.id).await,
            Some(h1.id),
            "до: хост есть"
        );

        let as_device = update_ok(
            &svc,
            &printer,
            DevicePatch {
                type_id: Some(DEVICE_TYPE_ID),
                ..Default::default()
            },
        )
        .await;
        assert_eq!(printers_count_for_device(&svc, printer.id).await, 0);

        update_ok(
            &svc,
            &as_device,
            DevicePatch {
                type_id: Some(PRINTER_TYPE_ID),
                ..Default::default()
            },
        )
        .await;
        assert_eq!(
            printers_count_for_device(&svc, printer.id).await,
            1,
            "строка пересоздана"
        );
        assert_eq!(
            fetch_usb_host(&svc, printer.id).await,
            None,
            "после двойной смены типа хост пуст"
        );
    })
    .await
    .expect("t10 exceeded 30 s budget");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn t11_one_patch_converts_to_printer_and_sets_host() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let h1 = create_device(&svc, "Рабочая станция А", DEVICE_TYPE_ID).await;
        let dev = create_device(&svc, "Принтер А", DEVICE_TYPE_ID).await;
        assert_eq!(printers_count_for_device(&svc, dev.id).await, 0);

        update_ok(
            &svc,
            &dev,
            DevicePatch {
                type_id: Some(PRINTER_TYPE_ID),
                usb_host_device_id: Some(Some(h1.id)),
                ..Default::default()
            },
        )
        .await;
        assert_eq!(printers_count_for_device(&svc, dev.id).await, 1);
        assert_eq!(
            fetch_usb_host(&svc, dev.id).await,
            Some(h1.id),
            "строка создаётся sync-ом, затем в неё пишется хост"
        );
    })
    .await
    .expect("t11 exceeded 30 s budget");
}
