//! Phase 41.13 Plan 10 (PGR-03, D-13, D-15): регрессионные тесты записи сети
//! принтера (IP-адрес, community) через `DeviceService` и `PrinterService`.
//!
//! Что закрепляет файл:
//!   * n1     — IP при правке СУЩЕСТВУЮЩЕГО принтера (ловушка exists-гарда
//!     `sync_printer_row_in_tx`: на существующей строке тот молчит, поэтому сеть
//!     обязана писаться отдельной операцией `set_network_in_tx`);
//!   * n2     — замена IP и community;
//!   * n3/n3b — СТИРАНИЕ IP (null и пробелы): ip NULL, community = public. Фикстура
//!     несёт непустой IP и нестандартную community ДО обновления (проверено
//!     чтением), иначе `NULL -> NULL` ничего не доказывает;
//!   * n4     — патч без ключей сети IP и community не меняет (предусловие с
//!     непустыми значениями проверено чтением ДО);
//!   * n5     — community без ключа IP: у принтера с IP меняется, у принтера без IP
//!     остаётся public;
//!   * n6/n6b — отказ адреса с портом, IPv6 и пробелами; имя хоста и IPv4 проходят;
//!   * n7     — не-принтер: Validation на непустое, no-op на пустое;
//!   * n8     — один патч «тип -> Принтер» + IP (порядок: сначала строка, потом сеть);
//!   * n9/n10 — IP и USB-хост НЕЗАВИСИМЫ (D-15) на пути обновления. Везде, где
//!     утверждается «хост на месте после записи IP», предусловие «и IP, и хост
//!     непусты» прочитано из БД ДО обновления, иначе тест вакуумен и не краснит
//!     мутанты «сброс хоста при IP» и «отказ при заданном хосте»;
//!   * n11    — создание блоком принтера (канонизация, отказ, хост + IP вместе);
//!   * n12    — `printers.version` растёт ровно на 1 при записи сети;
//!   * n13    — прямой путь `printers_create` (`PrinterService::create_from_device`).
//!
//! Каждый тест обёрнут в `tokio::time::timeout(30s)` (защита от deadlock).
//!
//! Приватность (CLAUDE.md): только вымышленные названия («Принтер А»,
//! «Рабочая станция А»), адреса из документационных диапазонов 192.0.2.0/24,
//! имена хостов `printer.local` и `printer-hall`, community `comm-alpha`/`comm-beta`.

use std::sync::Arc;
use std::time::Duration;

use rusqlite::params;

use trackly_app::dto::device::{DeviceDto, DeviceNew, DevicePatch};
use trackly_app::dto::number_template::{NumberFieldInput, TemplateContextDto};
use trackly_app::dto::printer::{PrinterCreateBlockDto, PrinterCreateDto};
use trackly_app::services::{DeviceService, PrinterService};
use trackly_core::auth::Identity;
use trackly_core::error::AppError;
use trackly_core::primitives::clock::Clock;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::test_support::test_writer_and_readers;

const DEVICE_TYPE_ID: i64 = 1;
const PRINTER_TYPE_ID: i64 = 2;
const IP_FIELD: &str = "printer_ip_address";
const COMMUNITY_FIELD: &str = "printer_community";

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
    create_with_block(svc, name, type_id, None)
        .await
        .expect("create device")
}

async fn create_with_block(
    svc: &DeviceService,
    name: &str,
    type_id: i64,
    block: Option<PrinterCreateBlockDto>,
) -> Result<DeviceDto, AppError> {
    svc.create_single_with_number_check_with_printer(
        minimal_new(name, type_id),
        empty_number_input(),
        TemplateContextDto::PrinterCreate,
        block,
    )
    .await
    .map(|o| o.expect_created("create"))
}

/// Принтер, созданный блоком: IP, community и хост по выбору.
async fn create_printer(
    svc: &DeviceService,
    name: &str,
    ip: Option<&str>,
    community: Option<&str>,
    host: Option<i64>,
) -> DeviceDto {
    create_with_block(
        svc,
        name,
        PRINTER_TYPE_ID,
        Some(PrinterCreateBlockDto {
            ip_address: ip.map(str::to_string),
            community: community.map(str::to_string),
            usb_host_device_id: host,
        }),
    )
    .await
    .expect("create printer")
}

/// Сырая строка `printers`: community через DTO наружу не выходит.
#[derive(Debug, Clone, PartialEq, Eq)]
struct PrinterRow {
    ip: Option<String>,
    community: String,
    host: Option<i64>,
    version: i64,
}

async fn fetch_row(svc: &DeviceService, device_id: i64) -> PrinterRow {
    let readers = svc.readers.clone();
    tokio::task::spawn_blocking(move || {
        let conn = readers.acquire();
        conn.query_row(
            "SELECT ip_address, community, usb_host_device_id, version \
             FROM printers WHERE device_id = ?1",
            params![device_id],
            |r| {
                Ok(PrinterRow {
                    ip: r.get(0)?,
                    community: r.get(1)?,
                    host: r.get(2)?,
                    version: r.get(3)?,
                })
            },
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

fn net_patch(ip: Option<Option<&str>>, community: Option<&str>) -> DevicePatch {
    DevicePatch {
        printer_ip_address: ip.map(|v| v.map(str::to_string)),
        printer_community: community.map(str::to_string),
        ..Default::default()
    }
}

fn ip_only(ip: &str) -> DevicePatch {
    net_patch(Some(Some(ip)), None)
}

fn host_only(host: Option<i64>) -> DevicePatch {
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

async fn update_res(
    svc: &DeviceService,
    dto: &DeviceDto,
    patch: DevicePatch,
) -> Result<DeviceDto, AppError> {
    svc.update(&admin_caller(), dto.id, dto.version, patch)
        .await
        .map(|o| o.expect_created("update"))
}

fn assert_validation(res: Result<impl std::fmt::Debug, AppError>, field: &str, what: &str) {
    match res {
        Err(AppError::Validation { field: f, .. }) => {
            assert_eq!(f, field, "{what}: поле ошибки");
        }
        other => panic!("{what}: expected AppError::Validation, got {other:?}"),
    }
}

/// ПРЕДУСЛОВИЕ «и IP, и хост непусты» — читается из БД, а не выводится из аргументов.
async fn assert_both_set(svc: &DeviceService, id: i64, ip: &str, host: i64, what: &str) {
    let row = fetch_row(svc, id).await;
    assert_eq!(
        row.ip.as_deref(),
        Some(ip),
        "ПРЕДУСЛОВИЕ ({what}): IP непуст"
    );
    assert_eq!(row.host, Some(host), "ПРЕДУСЛОВИЕ ({what}): хост непуст");
}

// ---------------------------------------------------------------------------
// n1-n5: запись, замена, стирание, «не трогать», community
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn n1_update_sets_ip_on_existing_printer_row() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let printer = create_device(&svc, "Принтер А", PRINTER_TYPE_ID).await;
        assert_eq!(printers_count_for_device(&svc, printer.id).await, 1);
        let before = fetch_row(&svc, printer.id).await;
        assert_eq!(before.ip, None, "ПРЕДУСЛОВИЕ: IP пуст");
        assert_eq!(before.community, "public");

        update_ok(&svc, &printer, ip_only("192.0.2.60")).await;

        let after = fetch_row(&svc, printer.id).await;
        assert_eq!(
            after.ip.as_deref(),
            Some("192.0.2.60"),
            "IP обязан записаться на существующей строке printers"
        );
        assert_eq!(after.community, "public", "community осталась public");
    })
    .await
    .expect("n1 exceeded 30 s budget");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn n2_update_replaces_ip_and_community() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let printer = create_printer(
            &svc,
            "Принтер А",
            Some("192.0.2.50"),
            Some("comm-alpha"),
            None,
        )
        .await;
        let before = fetch_row(&svc, printer.id).await;
        assert_eq!(before.ip.as_deref(), Some("192.0.2.50"), "ПРЕДУСЛОВИЕ: IP");
        assert_eq!(before.community, "comm-alpha", "ПРЕДУСЛОВИЕ: community");

        update_ok(
            &svc,
            &printer,
            net_patch(Some(Some("192.0.2.61")), Some("comm-beta")),
        )
        .await;

        let after = fetch_row(&svc, printer.id).await;
        assert_eq!(after.ip.as_deref(), Some("192.0.2.61"));
        assert_eq!(after.community, "comm-beta");
    })
    .await
    .expect("n2 exceeded 30 s budget");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn n3_erase_ip_with_null_resets_community_to_public() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let printer = create_printer(
            &svc,
            "Принтер А",
            Some("192.0.2.50"),
            Some("comm-alpha"),
            None,
        )
        .await;
        let before = fetch_row(&svc, printer.id).await;
        assert_eq!(
            before.ip.as_deref(),
            Some("192.0.2.50"),
            "ПРЕДУСЛОВИЕ: IP до стирания непуст"
        );
        assert_eq!(
            before.community, "comm-alpha",
            "ПРЕДУСЛОВИЕ: community до стирания нестандартная"
        );

        update_ok(&svc, &printer, net_patch(Some(None), None)).await;

        let after = fetch_row(&svc, printer.id).await;
        assert_eq!(after.ip, None, "ip_address IS NULL после стирания");
        assert_eq!(after.community, "public", "community сброшена в public");

        // Стирание с присланной community: итог всё равно public (правило «нет
        // IP — community public»).
        let second = create_printer(
            &svc,
            "Принтер Б",
            Some("192.0.2.51"),
            Some("comm-alpha"),
            None,
        )
        .await;
        assert_eq!(fetch_row(&svc, second.id).await.community, "comm-alpha");
        update_ok(&svc, &second, net_patch(Some(None), Some("comm-beta"))).await;
        let after2 = fetch_row(&svc, second.id).await;
        assert_eq!(after2.ip, None);
        assert_eq!(after2.community, "public");
    })
    .await
    .expect("n3 exceeded 30 s budget");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn n3b_erase_ip_with_blank_string_equals_null() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let printer = create_printer(
            &svc,
            "Принтер А",
            Some("192.0.2.50"),
            Some("comm-alpha"),
            None,
        )
        .await;
        let before = fetch_row(&svc, printer.id).await;
        assert_eq!(before.ip.as_deref(), Some("192.0.2.50"), "ПРЕДУСЛОВИЕ: IP");
        assert_eq!(before.community, "comm-alpha", "ПРЕДУСЛОВИЕ: community");

        update_ok(&svc, &printer, net_patch(Some(Some("   ")), None)).await;

        let after = fetch_row(&svc, printer.id).await;
        assert_eq!(after.ip, None, "строка из пробелов стирает IP");
        assert_eq!(after.community, "public");
    })
    .await
    .expect("n3b exceeded 30 s budget");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn n4_update_without_network_keys_does_not_touch_ip_and_community() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        // Фикстура НЕ вакуумна: оба значения непусты и нестандартны ДО обновления.
        let printer = create_printer(
            &svc,
            "Принтер А",
            Some("192.0.2.50"),
            Some("comm-alpha"),
            None,
        )
        .await;
        let before = fetch_row(&svc, printer.id).await;
        assert_eq!(
            before.ip.as_deref(),
            Some("192.0.2.50"),
            "ПРЕДУСЛОВИЕ: IP непуст"
        );
        assert_eq!(
            before.community, "comm-alpha",
            "ПРЕДУСЛОВИЕ: community непуста"
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

        let after = fetch_row(&svc, printer.id).await;
        assert_eq!(after.ip.as_deref(), Some("192.0.2.50"), "IP не тронут");
        assert_eq!(after.community, "comm-alpha", "community не тронута");
    })
    .await
    .expect("n4 exceeded 30 s budget");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn n5_community_without_ip_key() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();

        // У принтера с IP community меняется, IP прежний.
        let with_ip = create_printer(
            &svc,
            "Принтер А",
            Some("192.0.2.50"),
            Some("comm-alpha"),
            None,
        )
        .await;
        assert_eq!(
            fetch_row(&svc, with_ip.id).await.community,
            "comm-alpha",
            "ПРЕДУСЛОВИЕ"
        );
        let after_first = update_ok(&svc, &with_ip, net_patch(None, Some("comm-beta"))).await;
        let row = fetch_row(&svc, with_ip.id).await;
        assert_eq!(row.community, "comm-beta");
        assert_eq!(row.ip.as_deref(), Some("192.0.2.50"), "IP прежний");

        // Пустая community не меняет прежнюю.
        update_ok(&svc, &after_first, net_patch(None, Some("   "))).await;
        assert_eq!(fetch_row(&svc, with_ip.id).await.community, "comm-beta");

        // У принтера БЕЗ IP присланная community игнорируется: остаётся public.
        let no_ip = create_device(&svc, "Принтер Б", PRINTER_TYPE_ID).await;
        let before = fetch_row(&svc, no_ip.id).await;
        assert_eq!(before.ip, None, "ПРЕДУСЛОВИЕ: IP пуст");
        update_ok(&svc, &no_ip, net_patch(None, Some("comm-beta"))).await;
        let after = fetch_row(&svc, no_ip.id).await;
        assert_eq!(after.ip, None);
        assert_eq!(
            after.community, "public",
            "community без IP остаётся public"
        );
    })
    .await
    .expect("n5 exceeded 30 s budget");
}

// ---------------------------------------------------------------------------
// n6: отклоняемые и допустимые значения
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn n6_update_rejects_port_ipv6_and_inner_whitespace() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let printer = create_printer(
            &svc,
            "Принтер А",
            Some("192.0.2.50"),
            Some("comm-alpha"),
            None,
        )
        .await;
        let before = fetch_row(&svc, printer.id).await;
        assert_eq!(before.ip.as_deref(), Some("192.0.2.50"), "ПРЕДУСЛОВИЕ");
        assert_eq!(before.community, "comm-alpha", "ПРЕДУСЛОВИЕ");
        let devices_before = count(&svc, "SELECT COUNT(*) FROM devices").await;
        let printers_before = count(&svc, "SELECT COUNT(*) FROM printers").await;

        for bad in [
            "192.0.2.10:161",
            "printer.local:9999",
            "::1",
            "2001:db8::1",
            "192.0.2. 1",
            "printer a",
            "192.0.2.\t1",
        ] {
            let res = update_res(&svc, &printer, ip_only(bad)).await;
            assert_validation(res, IP_FIELD, bad);
            let row = fetch_row(&svc, printer.id).await;
            assert_eq!(row.ip.as_deref(), Some("192.0.2.50"), "{bad}: IP на месте");
            assert_eq!(row.community, "comm-alpha", "{bad}: community на месте");
            assert_eq!(
                count(&svc, "SELECT COUNT(*) FROM devices").await,
                devices_before,
                "{bad}: devices"
            );
            assert_eq!(
                count(&svc, "SELECT COUNT(*) FROM printers").await,
                printers_before,
                "{bad}: printers"
            );
        }

        // Атомарность: плохой IP вместе с правкой названия не применяет и название.
        let res = update_res(
            &svc,
            &printer,
            DevicePatch {
                name: Some("Принтер А (не должно записаться)".into()),
                printer_ip_address: Some(Some("192.0.2.10:161".into())),
                ..Default::default()
            },
        )
        .await;
        assert_validation(res, IP_FIELD, "плохой IP вместе с названием");
        let same = svc.get(printer.id).await.expect("get printer device");
        assert_eq!(same.name, "Принтер А", "название не изменилось");
    })
    .await
    .expect("n6 exceeded 30 s budget");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn n6b_update_accepts_host_names_and_ipv4() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let mut printer = create_device(&svc, "Принтер А", PRINTER_TYPE_ID).await;
        for (sent, stored) in [
            ("printer.local", "printer.local"),
            ("printer-hall", "printer-hall"),
            ("192.0.2.63", "192.0.2.63"),
            (" 192.0.2.64 ", "192.0.2.64"),
        ] {
            printer = update_ok(&svc, &printer, ip_only(sent)).await;
            assert_eq!(
                fetch_row(&svc, printer.id).await.ip.as_deref(),
                Some(stored),
                "{sent}: имена хостов и IPv4 обязаны проходить"
            );
        }
    })
    .await
    .expect("n6b exceeded 30 s budget");
}

// ---------------------------------------------------------------------------
// n7-n8: не-принтер и смена типа
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn n7_non_printer_rejects_non_empty_network_values() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let dev = create_device(&svc, "Рабочая станция А", DEVICE_TYPE_ID).await;

        let res = update_res(&svc, &dev, ip_only("192.0.2.65")).await;
        assert_validation(res, IP_FIELD, "IP на не-принтере");

        let res = update_res(&svc, &dev, net_patch(None, Some("comm-alpha"))).await;
        assert_validation(res, COMMUNITY_FIELD, "community на не-принтере");

        // Пустые значения - Ok без изменений.
        let dev = update_ok(&svc, &dev, net_patch(Some(None), None)).await;
        let dev = update_ok(&svc, &dev, net_patch(None, Some(""))).await;
        update_ok(&svc, &dev, net_patch(Some(Some("  ")), None)).await;
        assert_eq!(
            printers_count_for_device(&svc, dev.id).await,
            0,
            "строки printers у не-принтера нет"
        );
    })
    .await
    .expect("n7 exceeded 30 s budget");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn n8_one_patch_converts_to_printer_and_sets_ip() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let dev = create_device(&svc, "Принтер А", DEVICE_TYPE_ID).await;
        assert_eq!(printers_count_for_device(&svc, dev.id).await, 0);

        update_ok(
            &svc,
            &dev,
            DevicePatch {
                type_id: Some(PRINTER_TYPE_ID),
                printer_ip_address: Some(Some("192.0.2.62".into())),
                ..Default::default()
            },
        )
        .await;
        assert_eq!(printers_count_for_device(&svc, dev.id).await, 1);
        assert_eq!(
            fetch_row(&svc, dev.id).await.ip.as_deref(),
            Some("192.0.2.62"),
            "строка создаётся sync-ом, затем в неё пишется сеть"
        );
    })
    .await
    .expect("n8 exceeded 30 s budget");
}

// ---------------------------------------------------------------------------
// n9-n10: IP и USB-хост независимы (D-15)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn n9_ip_and_host_are_independent_on_update() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let h = create_device(&svc, "Рабочая станция А", DEVICE_TYPE_ID).await;
        let h2 = create_device(&svc, "Рабочая станция Б", DEVICE_TYPE_ID).await;

        // (а) у принтера с хостом непустой IP записывается, хост остаётся.
        let a = create_printer(&svc, "Принтер А", None, None, Some(h.id)).await;
        let before = fetch_row(&svc, a.id).await;
        assert_eq!(before.host, Some(h.id), "ПРЕДУСЛОВИЕ (а): хост задан");
        assert_eq!(before.ip, None, "ПРЕДУСЛОВИЕ (а): IP пуст");
        update_ok(&svc, &a, ip_only("192.0.2.66")).await;
        let row = fetch_row(&svc, a.id).await;
        assert_eq!(row.ip.as_deref(), Some("192.0.2.66"), "(а) IP записан");
        assert_eq!(row.host, Some(h.id), "(а) хост ОСТАЛСЯ на месте");

        // (б) патч с обоими ключами записывает оба значения.
        let b = create_printer(&svc, "Принтер Б", None, None, Some(h.id)).await;
        assert_eq!(
            fetch_row(&svc, b.id).await.host,
            Some(h.id),
            "ПРЕДУСЛОВИЕ (б)"
        );
        update_ok(
            &svc,
            &b,
            DevicePatch {
                printer_ip_address: Some(Some("192.0.2.67".into())),
                usb_host_device_id: Some(Some(h2.id)),
                ..Default::default()
            },
        )
        .await;
        let row = fetch_row(&svc, b.id).await;
        assert_eq!(row.ip.as_deref(), Some("192.0.2.67"), "(б) IP");
        assert_eq!(row.host, Some(h2.id), "(б) хост");

        // (в) IP записан, хост явно сброшен в том же патче.
        let c = create_printer(&svc, "Принтер В", None, None, Some(h.id)).await;
        assert_eq!(
            fetch_row(&svc, c.id).await.host,
            Some(h.id),
            "ПРЕДУСЛОВИЕ (в)"
        );
        update_ok(
            &svc,
            &c,
            DevicePatch {
                printer_ip_address: Some(Some("192.0.2.68".into())),
                usb_host_device_id: Some(None),
                ..Default::default()
            },
        )
        .await;
        let row = fetch_row(&svc, c.id).await;
        assert_eq!(row.ip.as_deref(), Some("192.0.2.68"), "(в) IP");
        assert_eq!(row.host, None, "(в) хост сброшен явно");

        // (г) четыре свежих принтера с ОБОИМИ значениями; предусловие читается из БД.
        let g1 = create_printer(&svc, "Принтер Г1", Some("192.0.2.50"), None, Some(h.id)).await;
        assert_both_set(&svc, g1.id, "192.0.2.50", h.id, "г1").await;
        update_ok(&svc, &g1, net_patch(Some(None), None)).await;
        let row = fetch_row(&svc, g1.id).await;
        assert_eq!(row.ip, None, "(г1) IP стёрт");
        assert_eq!(row.host, Some(h.id), "(г1) стирание IP хост НЕ трогает");

        let g2 = create_printer(&svc, "Принтер Г2", Some("192.0.2.51"), None, Some(h.id)).await;
        assert_both_set(&svc, g2.id, "192.0.2.51", h.id, "г2").await;
        update_ok(&svc, &g2, host_only(Some(h2.id))).await;
        let row = fetch_row(&svc, g2.id).await;
        assert_eq!(row.host, Some(h2.id), "(г2) хост заменён");
        assert_eq!(
            row.ip.as_deref(),
            Some("192.0.2.51"),
            "(г2) смена хоста IP не трогает"
        );

        let g3 = create_printer(&svc, "Принтер Г3", Some("192.0.2.52"), None, Some(h.id)).await;
        assert_both_set(&svc, g3.id, "192.0.2.52", h.id, "г3").await;
        update_ok(&svc, &g3, host_only(None)).await;
        let row = fetch_row(&svc, g3.id).await;
        assert_eq!(row.host, None, "(г3) хост сброшен");
        assert_eq!(
            row.ip.as_deref(),
            Some("192.0.2.52"),
            "(г3) сброс хоста IP не трогает"
        );

        let g4 = create_printer(&svc, "Принтер Г4", Some("192.0.2.53"), None, Some(h.id)).await;
        assert_both_set(&svc, g4.id, "192.0.2.53", h.id, "г4").await;
        update_ok(&svc, &g4, ip_only("192.0.2.54")).await;
        let row = fetch_row(&svc, g4.id).await;
        assert_eq!(row.ip.as_deref(), Some("192.0.2.54"), "(г4) IP заменён");
        assert_eq!(row.host, Some(h.id), "(г4) смена IP хост НЕ трогает");

        // (д) оба значения, правка названия без ключей: оба на месте.
        let d = create_printer(
            &svc,
            "Принтер Д",
            Some("192.0.2.55"),
            Some("comm-alpha"),
            Some(h.id),
        )
        .await;
        assert_both_set(&svc, d.id, "192.0.2.55", h.id, "д").await;
        update_ok(
            &svc,
            &d,
            DevicePatch {
                name: Some("Принтер Д (правка)".into()),
                ..Default::default()
            },
        )
        .await;
        let row = fetch_row(&svc, d.id).await;
        assert_eq!(row.ip.as_deref(), Some("192.0.2.55"), "(д) IP на месте");
        assert_eq!(row.host, Some(h.id), "(д) хост на месте");
        assert_eq!(row.community, "comm-alpha", "(д) community на месте");
    })
    .await
    .expect("n9 exceeded 30 s budget");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn n10_host_can_be_set_with_ip_and_after_erasing_ip() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let h1 = create_device(&svc, "Рабочая станция А", DEVICE_TYPE_ID).await;
        let h2 = create_device(&svc, "Рабочая станция Б", DEVICE_TYPE_ID).await;
        let printer = create_printer(&svc, "Принтер А", Some("192.0.2.50"), None, None).await;
        let before = fetch_row(&svc, printer.id).await;
        assert_eq!(
            before.ip.as_deref(),
            Some("192.0.2.50"),
            "ПРЕДУСЛОВИЕ: IP непуст"
        );
        assert_eq!(before.host, None, "ПРЕДУСЛОВИЕ: хоста нет");

        // Патч 1: хост задаётся при НЕПУСТОМ IP.
        let p1 = update_ok(&svc, &printer, host_only(Some(h1.id))).await;
        let row = fetch_row(&svc, printer.id).await;
        assert_eq!(row.host, Some(h1.id), "хост задан при непустом IP");
        assert_eq!(row.ip.as_deref(), Some("192.0.2.50"), "IP на месте");

        // Патч 2: стираем IP.
        let p2 = update_ok(&svc, &p1, net_patch(Some(None), None)).await;
        let row = fetch_row(&svc, printer.id).await;
        assert_eq!(row.ip, None);
        assert_eq!(row.host, Some(h1.id), "стирание IP хост не трогает");

        // Патч 3: хост задаётся после стирания IP.
        update_ok(&svc, &p2, host_only(Some(h2.id))).await;
        assert_eq!(
            fetch_row(&svc, printer.id).await.host,
            Some(h2.id),
            "хост задан после стирания IP и прочитан из БД"
        );
    })
    .await
    .expect("n10 exceeded 30 s budget");
}

// ---------------------------------------------------------------------------
// n11: создание блоком принтера
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn n11_create_block_normalizes_rejects_and_keeps_host_with_ip() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let h = create_device(&svc, "Рабочая станция А", DEVICE_TYPE_ID).await;

        // Пробелы по краям срезаются.
        let p = create_printer(&svc, "Принтер А", Some(" 192.0.2.70 "), None, None).await;
        assert_eq!(
            fetch_row(&svc, p.id).await.ip.as_deref(),
            Some("192.0.2.70")
        );

        // Адрес с портом отклонён и ничего не оставляет.
        let devices_before = count(&svc, "SELECT COUNT(*) FROM devices").await;
        let printers_before = count(&svc, "SELECT COUNT(*) FROM printers").await;
        let res = create_with_block(
            &svc,
            "Принтер Б",
            PRINTER_TYPE_ID,
            Some(PrinterCreateBlockDto {
                ip_address: Some("192.0.2.70:161".into()),
                community: None,
                usb_host_device_id: None,
            }),
        )
        .await;
        assert_validation(res, IP_FIELD, "адрес с портом при создании");
        assert_eq!(
            count(&svc, "SELECT COUNT(*) FROM devices").await,
            devices_before
        );
        assert_eq!(
            count(&svc, "SELECT COUNT(*) FROM printers").await,
            printers_before
        );

        // Имя хоста принимается.
        let named = create_printer(&svc, "Принтер В", Some("printer.local"), None, None).await;
        assert_eq!(
            fetch_row(&svc, named.id).await.ip.as_deref(),
            Some("printer.local")
        );

        // IP и хост вместе: записываются оба (D-15).
        let both = create_printer(
            &svc,
            "Принтер Г",
            Some("192.0.2.71"),
            Some("comm-alpha"),
            Some(h.id),
        )
        .await;
        let row = fetch_row(&svc, both.id).await;
        assert_eq!(row.ip.as_deref(), Some("192.0.2.71"), "IP при хосте");
        assert_eq!(row.host, Some(h.id), "хост при IP");
        assert_eq!(row.community, "comm-alpha");

        // Пустой IP + community: community = public.
        let no_ip = create_printer(&svc, "Принтер Д", Some("  "), Some("comm-alpha"), None).await;
        let row = fetch_row(&svc, no_ip.id).await;
        assert_eq!(row.ip, None);
        assert_eq!(row.community, "public");
    })
    .await
    .expect("n11 exceeded 30 s budget");
}

// ---------------------------------------------------------------------------
// n12: версия строки printers
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn n12_network_write_bumps_printer_version_exactly_once() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let printer = create_printer(
            &svc,
            "Принтер А",
            Some("192.0.2.50"),
            Some("comm-alpha"),
            None,
        )
        .await;
        let v0 = fetch_row(&svc, printer.id).await.version;

        let renamed = update_ok(
            &svc,
            &printer,
            DevicePatch {
                name: Some("Принтер А (правка)".into()),
                ..Default::default()
            },
        )
        .await;
        assert_eq!(
            fetch_row(&svc, printer.id).await.version,
            v0,
            "правка названия без ключей сети версию printers не меняет"
        );

        update_ok(
            &svc,
            &renamed,
            net_patch(Some(Some("192.0.2.61")), Some("comm-beta")),
        )
        .await;
        assert_eq!(
            fetch_row(&svc, printer.id).await.version,
            v0 + 1,
            "запись IP и community одним патчем - ровно один шаг версии"
        );
    })
    .await
    .expect("n12 exceeded 30 s budget");
}

// ---------------------------------------------------------------------------
// n13: прямой путь printers_create
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn n13_printers_create_path_uses_the_same_validator() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (writer, readers, _dir) = test_writer_and_readers();
        let clock: Arc<dyn Clock + Send + Sync> = Arc::new(SystemClock);
        let (ws_tx, _) = tokio::sync::broadcast::channel::<trackly_app::dto::printer::WsEvent>(16);
        let (poll_tx, _poll_rx) = tokio::sync::mpsc::channel::<i64>(16);
        let snmp: Arc<dyn trackly_core::ports::snmp::SnmpClient + Send + Sync> =
            Arc::new(trackly_infra::snmp::mock::MockSnmpClient::default_fixtures());
        let printers = PrinterService::new(
            writer.clone(),
            readers.clone(),
            clock,
            snmp,
            poll_tx,
            Arc::new(ws_tx),
        );
        let caller = admin_caller();

        // Устройство-принтер БЕЗ строки printers: вставляется напрямую.
        let next_device = |name: &'static str| {
            let writer = writer.clone();
            async move {
                writer
                    .execute(move |conn| {
                        conn.execute(
                            "INSERT INTO devices (type_id, name, status_id, created_at_utc, \
                             updated_at_utc, version) VALUES (2, ?1, 1, 1700000000, 1700000000, 1)",
                            params![name],
                        )
                        .map_err(|e| AppError::Internal {
                            source_chain: format!("insert device: {e}"),
                        })?;
                        Ok(conn.last_insert_rowid())
                    })
                    .await
                    .expect("insert device")
            }
        };
        let count_rows = |device_id: i64| {
            let readers = readers.clone();
            async move {
                tokio::task::spawn_blocking(move || {
                    let conn = readers.acquire();
                    conn.query_row(
                        "SELECT COUNT(*) FROM printers WHERE device_id = ?1",
                        params![device_id],
                        |r| r.get::<_, i64>(0),
                    )
                    .expect("count")
                })
                .await
                .expect("spawn_blocking")
            }
        };
        let dto = |device_id: i64, ip: &str| PrinterCreateDto {
            device_id: device_id as i32,
            ip_address: Some(ip.to_string()),
            community_update: None,
            snmp_version: "v2c".to_string(),
            oid_profile_id: None,
            usb_host_device_id: None,
        };

        // Адрес с портом и IPv6 - отказ, строки printers нет.
        for (name, bad) in [
            ("Принтер А", "192.0.2.90:161"),
            ("Принтер Б", "::1"),
            ("Принтер В", "printer a"),
        ] {
            let id = next_device(name).await;
            let res = printers.create_from_device(dto(id, bad), &caller).await;
            assert_validation(res, IP_FIELD, bad);
            assert_eq!(
                count_rows(id).await,
                0,
                "{bad}: строка printers не появилась"
            );
        }

        // IPv4 и имя хоста принимаются как есть.
        for (name, good) in [("Принтер Г", "192.0.2.91"), ("Принтер Д", "printer.local")]
        {
            let id = next_device(name).await;
            let created = printers
                .create_from_device(dto(id, good), &caller)
                .await
                .expect("допустимый адрес принят");
            assert_eq!(created.ip_address.as_deref(), Some(good));
            assert_eq!(count_rows(id).await, 1);
        }

        // Пустая строка - ip_address NULL.
        let id = next_device("Принтер Е").await;
        let created = printers
            .create_from_device(dto(id, ""), &caller)
            .await
            .expect("пустой адрес принят");
        assert_eq!(created.ip_address, None, "пустая строка - NULL");
    })
    .await
    .expect("n13 exceeded 30 s budget");
}
