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
//!   * n12    — `printers.version` растёт ровно на 1 при записи сети.
//!   * quick 261121-heg — история опросов `printers_get_readings` (N2) и
//!     пометка мягко удалённого USB-хоста `usb_host_deleted` (INT-36в).
//!
//! (n13 — прямой путь создания принтера — удалён вместе с маршрутом, quick 261121-4nz.)
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
use trackly_app::dto::printer::PrinterCreateBlockDto;
use trackly_app::services::DeviceService;
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
// quick 261121-heg (N2): история опросов принтера `printers_get_readings`.
// UI («История статусов» в карточке принтера) звал команду с Фазы 6, её не
// было — ошибка глушилась, лента всегда пустая.
// ---------------------------------------------------------------------------

async fn make_app_ctx() -> (trackly_app::context::AppCtx, tempfile::TempDir) {
    let dir = tempfile::TempDir::new().expect("tempdir");
    let paths = trackly_infra::Paths::resolve_for_exe_dir(dir.path().to_path_buf()).expect("paths");
    let config = trackly_infra::AppConfig::default();
    let log_guard = trackly_app::logging::init(&paths, &config)
        .or_else(|_| {
            let (_nb, guard) = tracing_appender::non_blocking(std::io::sink());
            Ok::<_, anyhow::Error>(guard)
        })
        .expect("log guard");
    let ctx = trackly_app::context::AppCtx::build(paths, config, log_guard)
        .await
        .expect("ctx");
    (ctx, dir)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn printers_get_readings_newest_first_camel_case_and_not_found() {
    use trackly_app::tauri_cmds::printers::build_printers_get_readings;

    tokio::time::timeout(Duration::from_secs(30), async {
        let (ctx, _dir) = make_app_ctx().await;
        let printer =
            create_printer(&ctx.devices, "Принтер А", Some("192.0.2.80"), None, None).await;
        let device_id = printer.id;
        let printer_id: i64 = {
            let readers = ctx.readers.clone();
            tokio::task::spawn_blocking(move || {
                readers
                    .acquire()
                    .query_row(
                        "SELECT id FROM printers WHERE device_id = ?1",
                        params![device_id],
                        |r| r.get(0),
                    )
                    .expect("printers.id")
            })
            .await
            .expect("spawn_blocking")
        };

        // Живой AppCtx поднимает мониторинг (SNMP-мок): он может дописать своё
        // показание с текущим временем. Засеянные строки — в будущем, чтобы
        // они гарантированно были новейшими; общий порядок проверяется ниже.
        let base = 4_000_000_000_i64;
        ctx.writer
            .execute(move |conn| {
                // Порядок вставки нарочно не совпадает с порядком ts_utc.
                for (ts, pages, status) in [
                    (base + 200, Some(300_i64), "ok"),
                    (base + 100, Some(200_i64), "warning"),
                    (base + 300, None, "offline"),
                ] {
                    conn.execute(
                        "INSERT INTO printer_readings \
                         (printer_id, ts_utc, toner_levels, page_count, status) \
                         VALUES (?1, ?2, '{\"black\":{\"pct\":40}}', ?3, ?4)",
                        params![printer_id, ts, pages, status],
                    )
                    .expect("seed reading");
                }
                Ok(())
            })
            .await
            .expect("seed readings");

        let admin = admin_caller();
        let rows = build_printers_get_readings(&ctx, &admin, printer_id)
            .await
            .expect("readings");
        let ts: Vec<i64> = rows.iter().map(|r| r.ts_utc).collect();
        assert_eq!(
            ts[..3],
            [base + 300, base + 200, base + 100],
            "новейшие сверху"
        );
        assert!(
            ts.windows(2).all(|w| w[0] >= w[1]),
            "вся лента по убыванию ts_utc: {ts:?}"
        );
        assert_eq!(rows[0].page_count, None);
        assert_eq!(rows[0].status, "offline");

        // Форма JSON ровно как ui/src/bindings-phase6.ts PrinterReadingDto.
        let json = serde_json::to_value(&rows[1]).expect("json");
        let mut keys: Vec<&str> = json
            .as_object()
            .expect("object")
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            vec![
                "id",
                "pageCount",
                "printerId",
                "status",
                "tonerLevelsJson",
                "tsUtc"
            ]
        );
        assert_eq!(json["printerId"], printer_id);
        assert_eq!(json["pageCount"], 300);
        assert_eq!(json["tonerLevelsJson"], "{\"black\":{\"pct\":40}}");

        // Несуществующий printers.id -> NotFound (как у printers_get).
        let missing = build_printers_get_readings(&ctx, &admin, printer_id + 10_000).await;
        assert!(
            matches!(missing, Err(AppError::NotFound { .. })),
            "чужой id -> NotFound, получено {missing:?}"
        );
    })
    .await
    .expect("printers_get_readings exceeded 30 s budget");
}

// ---------------------------------------------------------------------------
// quick 261121-heg (INT-36в): мягко удалённый USB-хост помечен в выборке
// принтера (`usb_host_deleted`), подпись хоста при этом остаётся (D-37).
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn usb_host_deleted_flag_follows_soft_delete_of_host() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (ctx, _dir) = make_app_ctx().await;
        let gone = create_device(&ctx.devices, "Рабочая станция А", DEVICE_TYPE_ID).await;
        let alive = create_device(&ctx.devices, "Рабочая станция Б", DEVICE_TYPE_ID).await;
        let p_gone = create_printer(&ctx.devices, "Принтер А", None, None, Some(gone.id)).await;
        let p_alive = create_printer(&ctx.devices, "Принтер Б", None, None, Some(alive.id)).await;
        let p_none =
            create_printer(&ctx.devices, "Принтер В", Some("192.0.2.81"), None, None).await;

        // ПРЕДУСЛОВИЕ: до удаления ни один хост не помечен.
        let before = ctx.printers.get_by_device_id(p_gone.id).await.expect("get");
        assert_eq!(
            before.usb_host_device_id,
            Some(gone.id),
            "ПРЕДУСЛОВИЕ: хост задан"
        );
        assert!(!before.usb_host_deleted, "ПРЕДУСЛОВИЕ: хост жив");

        let host_id = gone.id;
        ctx.writer
            .execute(move |conn| {
                conn.execute(
                    "UPDATE devices SET deleted_at_utc = 1700000000 WHERE id = ?1",
                    params![host_id],
                )
                .expect("soft delete host");
                Ok(())
            })
            .await
            .expect("soft delete");

        let after = ctx.printers.get_by_device_id(p_gone.id).await.expect("get");
        assert!(after.usb_host_deleted, "мягко удалённый хост помечен");
        assert_eq!(
            after.usb_host_name.as_deref(),
            Some("Рабочая станция А"),
            "подпись хоста остаётся (D-37)"
        );
        let json = serde_json::to_value(&after).expect("json");
        assert_eq!(json["usbHostDeleted"], true, "camelCase-ключ для UI");

        let alive_dto = ctx
            .printers
            .get_by_device_id(p_alive.id)
            .await
            .expect("get");
        assert!(!alive_dto.usb_host_deleted, "живой хост не помечен");
        let none_dto = ctx.printers.get_by_device_id(p_none.id).await.expect("get");
        assert!(!none_dto.usb_host_deleted, "без хоста — false");
    })
    .await
    .expect("usb_host_deleted exceeded 30 s budget");
}
