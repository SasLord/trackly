//! Phase 40.3 Plan 04 (NUM-06/NUM-08) — regression-тест на
//! `DeviceService::create_single_with_number_check_with_printer`: printer-блок
//! (IP/SNMP community) должен создавать строку `printers` СРАЗУ в ТОЙ ЖЕ
//! writer-транзакции, что и запись устройства — не через второй follow-up
//! запрос.
//!
//! Три сценария (`<behavior>` плана):
//!   1. С printer-блоком — `printers.ip_address`/`community` совпадают с
//!      переданными значениями.
//!   2. Без printer-блока — дефолты не меняются (`ip_address = NULL`,
//!      `community = 'public'`).
//!   3. Printer-блок на устройстве НЕ типа «Принтер» — явная
//!      `AppError::Validation`, а не тихое игнорирование.
//!
//! Каждый тест обёрнут в `tokio::time::timeout(30s)` (PATTERNS.md §Pattern 4),
//! паттерн харнесса — `devices_crud.rs`/`devices_type_conversion.rs`.
//!
//! Фиктивные данные (приватность — CLAUDE.md): IP из документационного
//! диапазона RFC 5737 (`192.0.2.0/24`), вымышленные названия устройств.

use std::sync::Arc;
use std::time::Duration;

use rusqlite::params;

use trackly_app::dto::device::DeviceNew;
use trackly_app::dto::number_template::{NumberFieldInput, TemplateContextDto};
use trackly_app::dto::printer::PrinterCreateBlockDto;
use trackly_app::services::DeviceService;
use trackly_core::error::AppError;
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

/// Пустой номер (без шаблона) — достаточен для проверки printer-блока
/// изолированно от номерной цепочки (D-01/NUM-11 покрыты в
/// `devices_numbering.rs`).
fn empty_number_input() -> NumberFieldInput {
    NumberFieldInput {
        value: String::new(),
        template_id: None,
        confirm_mismatch: false,
        confirm_script_mix: false,
    }
}

/// Прочитать `printers.ip_address`/`community` для `device_id` напрямую SQL —
/// доказывает, что строка видна СРАЗУ после успешного `Ok` (атомарность
/// одной транзакции, не два последовательных запроса).
async fn fetch_printer_ip_community(
    svc: &DeviceService,
    device_id: i64,
) -> (Option<String>, String) {
    let readers = svc.readers.clone();
    tokio::task::spawn_blocking(move || {
        let conn = readers.acquire();
        conn.query_row(
            "SELECT ip_address, community FROM printers WHERE device_id = ?1",
            params![device_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .expect("printers row must exist immediately after create Ok")
    })
    .await
    .expect("spawn_blocking")
}

// ---------------------------------------------------------------------------
// Тест 1: с printer-блоком — ip/community записаны в той же транзакции
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn printer_block_writes_ip_and_community_atomically() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();

        let printer_block = PrinterCreateBlockDto {
            ip_address: Some("192.0.2.50".to_string()),
            community: Some("private".to_string()),
        };

        let outcome = svc
            .create_single_with_number_check_with_printer(
                minimal_new("Test Printer 1", PRINTER_TYPE_ID),
                empty_number_input(),
                TemplateContextDto::PrinterCreate,
                Some(printer_block),
            )
            .await
            .expect("create_single_with_number_check_with_printer with printer block");
        let dto = outcome.expect_created("printer_block_writes_ip_and_community_atomically");

        // Строка `printers` видна СРАЗУ после успешного Ok — доказывает
        // атомарность (одна транзакция, не два запроса).
        let (ip_address, community) = fetch_printer_ip_community(&svc, dto.id).await;
        assert_eq!(
            ip_address.as_deref(),
            Some("192.0.2.50"),
            "printer block ip_address must be persisted in the same transaction"
        );
        assert_eq!(
            community, "private",
            "printer block community must override the 'public' default"
        );
    })
    .await
    .expect("printer_block_writes_ip_and_community_atomically exceeded 30 s budget");
}

// ---------------------------------------------------------------------------
// Тест 2: без printer-блока — дефолты не меняются (регрессия)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn no_printer_block_keeps_existing_defaults() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();

        let outcome = svc
            .create_single_with_number_check_with_printer(
                minimal_new("Test Printer 2", PRINTER_TYPE_ID),
                empty_number_input(),
                TemplateContextDto::PrinterCreate,
                None,
            )
            .await
            .expect("create_single_with_number_check_with_printer without printer block");
        let dto = outcome.expect_created("no_printer_block_keeps_existing_defaults");

        let (ip_address, community) = fetch_printer_ip_community(&svc, dto.id).await;
        assert_eq!(
            ip_address, None,
            "without a printer block, ip_address must stay NULL (unchanged default)"
        );
        assert_eq!(
            community, "public",
            "without a printer block, community must stay 'public' (unchanged default)"
        );
    })
    .await
    .expect("no_printer_block_keeps_existing_defaults exceeded 30 s budget");
}

// ---------------------------------------------------------------------------
// Тест 3: printer-блок на НЕ-принтере — явная ошибка валидации
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn printer_block_on_non_printer_type_is_rejected() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();

        let printer_block = PrinterCreateBlockDto {
            ip_address: Some("192.0.2.51".to_string()),
            community: None,
        };

        let err = svc
            .create_single_with_number_check_with_printer(
                minimal_new("Test Device 1", DEVICE_TYPE_ID),
                empty_number_input(),
                TemplateContextDto::DeviceCreate,
                Some(printer_block),
            )
            .await
            .expect_err(
                "a printer block on a non-printer type_id must be explicitly rejected, \
                 not silently ignored",
            );

        match err {
            AppError::Validation { field, .. } => {
                assert_eq!(
                    field, "printer",
                    "validation error must name the 'printer' field"
                );
            }
            other => panic!("expected AppError::Validation, got {other:?}"),
        }
    })
    .await
    .expect("printer_block_on_non_printer_type_is_rejected exceeded 30 s budget");
}
