//! Phase 41.1 Plan 22 (GRD-05, D-37) — мягко удалённое устройство-принтер не
//! должно оставаться «призраком» в разделе «Принтеры».
//!
//! `printers` не имеет `deleted_at_utc` (мягкое удаление живёт на `devices`),
//! поэтому до этого плана список, его счётчик, три счётчика дашборда и
//! выборка поллера продолжали видеть принтер удалённого устройства.
//!
//! Фикстура РАСХОДЯЩАЯСЯ (иначе тест вакуумный): два принтера, один мягко
//! удалён сервисным путём (`DeviceService::delete_soft`) и имеет активный
//! alert 'offline'; живой — без alert'а. Каждое утверждение отличает
//! «до фикса» от «после фикса».
//!
//! Тесты:
//!   1. список и его total — только живой принтер;
//!   2. printer_total / printer_offline / printer_problematic — без скрытого;
//!   3. выборка поллера (`poll_all`) не опрашивает скрытый принтер;
//!   4. после снятия `deleted_at_utc` строка возвращается с прежними
//!      SNMP-настройками (строка `printers` не пересоздавалась);
//!   5. `PrinterRepository::get` скрытого принтера по-прежнему работает;
//!   6. проверка дубликата IP при обнаружении (следствие D-37).
//!
//! Данные вымышленные (CLAUDE.md, приватность): названия «Принтер А»/«Принтер Б»,
//! IP — фикстуры `MockSnmpClient` (192.168.1.100 / .101), community — «секрет-а».

use std::sync::Arc;
use std::time::Duration;

use rusqlite::params;

use trackly_app::dto::device::DeviceNew;
use trackly_app::dto::number_template::{NumberFieldInput, TemplateContextDto};
use trackly_app::dto::printer::PrinterCreateBlockDto;
use trackly_app::services::dashboard_service::DashboardService;
use trackly_app::services::{DeviceService, PrinterService};
use trackly_core::auth::Identity;
use trackly_core::domain::printers::{Pagination, PrinterFilter};
use trackly_core::ports::printers::PrinterRepository;
use trackly_core::primitives::clock::Clock;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::repos::printers_sqlite::SqlitePrinterRepository;
use trackly_infra::test_support::test_writer_and_readers;
use trackly_infra::AppConfig;

const DEVICE_TYPE_ID: i64 = 1;
const PRINTER_TYPE_ID: i64 = 2;

/// IP мягко удалённого принтера (фикстура `MockSnmpClient`: статус ok).
const HIDDEN_IP: &str = "192.168.1.100";
/// IP живого принтера (фикстура `MockSnmpClient`: статус warning).
const LIVE_IP: &str = "192.168.1.101";
const HIDDEN_COMMUNITY: &str = "секрет-а";

struct Fixture {
    devices: DeviceService,
    printers: PrinterService,
    dashboard: DashboardService,
    _dir: tempfile::TempDir,
    hidden_printer_id: i64,
    hidden_device_id: i64,
    hidden_device_version: i64,
    live_printer_id: i64,
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

async fn create_printer_device(
    devices: &DeviceService,
    name: &str,
    ip: &str,
    community: &str,
) -> trackly_app::dto::device::DeviceDto {
    devices
        .create_single_with_number_check_with_printer(
            minimal_new(name, PRINTER_TYPE_ID),
            empty_number_input(),
            TemplateContextDto::PrinterCreate,
            Some(PrinterCreateBlockDto {
                ip_address: Some(ip.to_string()),
                community: Some(community.to_string()),
            }),
        )
        .await
        .expect("create printer device")
        .expect_created("create printer device")
}

async fn printer_id_for_device(devices: &DeviceService, device_id: i64) -> i64 {
    let readers = devices.readers.clone();
    tokio::task::spawn_blocking(move || {
        readers
            .acquire()
            .query_row(
                "SELECT id FROM printers WHERE device_id = ?1",
                params![device_id],
                |r| r.get(0),
            )
            .expect("printers row for device")
    })
    .await
    .expect("spawn_blocking")
}

/// Два принтера: скрытый (сетевой, с USB-хостом, профилем OID и непубличной
/// community) и живой (сетевой). Скрытый УЖЕ мягко удалён через DeviceService.
async fn setup_deleted_hidden() -> Fixture {
    let f = setup_alive().await;
    f.devices
        .delete_soft(f.hidden_device_id, f.hidden_device_version)
        .await
        .expect("soft-delete hidden printer device");
    f
}

/// Те же два принтера, но оба ещё живы.
async fn setup_alive() -> Fixture {
    let (writer, readers, dir) = test_writer_and_readers();
    let clock: Arc<dyn Clock + Send + Sync> = Arc::new(SystemClock);

    let devices = DeviceService::new(writer.clone(), readers.clone(), clock.clone());
    let (ws_tx, _) = tokio::sync::broadcast::channel::<trackly_app::dto::printer::WsEvent>(16);
    let (poll_tx, _poll_rx) = tokio::sync::mpsc::channel::<i64>(16);
    let snmp: Arc<dyn trackly_core::ports::snmp::SnmpClient + Send + Sync> =
        Arc::new(trackly_infra::snmp::mock::MockSnmpClient::default_fixtures());
    let printers = PrinterService::new(
        writer.clone(),
        readers.clone(),
        clock.clone(),
        snmp,
        poll_tx,
        Arc::new(ws_tx),
    );
    let dashboard = DashboardService::new(
        writer.clone(),
        readers.clone(),
        clock.clone(),
        Arc::new(AppConfig::default()),
    );

    let host = devices
        .create_single_with_number_check_with_printer(
            minimal_new("Рабочая станция А", DEVICE_TYPE_ID),
            empty_number_input(),
            TemplateContextDto::DeviceCreate,
            None,
        )
        .await
        .expect("create usb host device")
        .expect_created("create usb host device");

    let hidden = create_printer_device(&devices, "Принтер А", HIDDEN_IP, HIDDEN_COMMUNITY).await;
    let live = create_printer_device(&devices, "Принтер Б", LIVE_IP, "public").await;

    let hidden_printer_id = printer_id_for_device(&devices, hidden.id).await;
    let live_printer_id = printer_id_for_device(&devices, live.id).await;

    // SNMP-настройки скрытого принтера, которые обязаны пережить удаление.
    let host_id = host.id;
    writer
        .execute(move |conn| {
            conn.execute(
                "UPDATE printers \
                    SET oid_profile_id = (SELECT MIN(id) FROM oid_profiles), \
                        usb_host_device_id = ?1 \
                  WHERE id = ?2",
                params![host_id, hidden_printer_id],
            )
            .map_err(trackly_infra::error_conversions::map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("set snmp settings on hidden printer");

    Fixture {
        devices,
        printers,
        dashboard,
        _dir: dir,
        hidden_printer_id,
        hidden_device_id: hidden.id,
        hidden_device_version: hidden.version,
        live_printer_id,
    }
}

async fn insert_offline_alert(f: &Fixture, printer_id: i64) {
    f.devices
        .writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO printer_alerts \
                   (printer_id, alert_type, first_seen_utc, last_seen_utc) \
                 VALUES (?1, 'offline', 1, 1)",
                params![printer_id],
            )
            .map_err(trackly_infra::error_conversions::map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("insert offline alert");
}

async fn restore_device(f: &Fixture) {
    let id = f.hidden_device_id;
    f.devices
        .writer
        .execute(move |conn| {
            conn.execute(
                "UPDATE devices SET deleted_at_utc = NULL WHERE id = ?1",
                params![id],
            )
            .map_err(trackly_infra::error_conversions::map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("restore device");
}

async fn list_ids(f: &Fixture) -> (Vec<i64>, i64) {
    let resp = f
        .printers
        .list(
            PrinterFilter::default(),
            Pagination {
                offset: 0,
                limit: 1000,
            },
        )
        .await
        .expect("printers list");
    let mut ids: Vec<i64> = resp.items.iter().map(|p| p.id).collect();
    ids.sort();
    (ids, resp.total)
}

async fn reading_count(f: &Fixture, printer_id: i64) -> i64 {
    let readers = f.devices.readers.clone();
    tokio::task::spawn_blocking(move || {
        readers
            .acquire()
            .query_row(
                "SELECT COUNT(*) FROM printer_readings WHERE printer_id = ?1",
                params![printer_id],
                |r| r.get(0),
            )
            .expect("count readings")
    })
    .await
    .expect("spawn_blocking")
}

/// (id, version, community, oid_profile_id, usb_host_device_id) строки printers.
async fn printer_snapshot(
    f: &Fixture,
    printer_id: i64,
) -> (i64, i64, String, Option<i64>, Option<i64>) {
    let readers = f.devices.readers.clone();
    tokio::task::spawn_blocking(move || {
        readers
            .acquire()
            .query_row(
                "SELECT id, version, community, oid_profile_id, usb_host_device_id \
                   FROM printers WHERE id = ?1",
                params![printer_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
            )
            .expect("printers snapshot")
    })
    .await
    .expect("spawn_blocking")
}

/// Ровно та выборка, что делает проверка дубликата IP в `PrinterService::discover`.
async fn ips_seen_by_discover(f: &Fixture) -> Vec<String> {
    let readers = f.devices.readers.clone();
    tokio::task::spawn_blocking(move || {
        let conn = readers.acquire();
        let filter = PrinterFilter {
            status: None,
            search: None,
        };
        let page = Pagination {
            offset: 0,
            limit: 1000,
        };
        let (rows, _) = SqlitePrinterRepository
            .list(&conn, &filter, &page)
            .expect("repo list");
        rows.into_iter().filter_map(|r| r.ip_address).collect()
    })
    .await
    .expect("spawn_blocking")
}

// ---------------------------------------------------------------------------
// Тест 1: список и его total
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn printers_soft_delete_list_and_total_hide_deleted() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let f = setup_deleted_hidden().await;
        let (ids, total) = list_ids(&f).await;
        assert_eq!(
            ids,
            vec![f.live_printer_id],
            "в списке должен остаться только живой принтер"
        );
        assert_eq!(total, 1, "total списка не должен считать скрытый принтер");
    })
    .await
    .expect("printers_soft_delete_list_and_total_hide_deleted exceeded 30 s budget");
}

// ---------------------------------------------------------------------------
// Тест 2: три счётчика дашборда (alert у скрытого принтера не засчитывается)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn printers_soft_delete_dashboard_counters_ignore_deleted() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let f = setup_deleted_hidden().await;
        insert_offline_alert(&f, f.hidden_printer_id).await;

        let dto = f
            .dashboard
            .get_all_widgets(&Identity::trusted_admin(), None)
            .await
            .expect("widgets");
        // printer_online = printer_total - printer_offline: при total == 1 и
        // offline == 0 online == 1 (без фильтра total == 2, online == 1 при
        // offline == 1 — поэтому проверяем online вместе с offline).
        assert_eq!(
            dto.printer_offline, 0,
            "printer_offline: alert скрытого не считать"
        );
        assert_eq!(
            dto.printer_problematic, 0,
            "printer_problematic: alert скрытого не считать"
        );
        assert_eq!(
            dto.printer_online, 1,
            "printer_online = printer_total(1) - printer_offline(0)"
        );

        restore_device(&f).await;
        let dto = f
            .dashboard
            .get_all_widgets(&Identity::trusted_admin(), None)
            .await
            .expect("widgets after restore");
        assert_eq!(
            dto.printer_offline, 1,
            "после возврата alert снова считается"
        );
        assert_eq!(dto.printer_problematic, 1);
        assert_eq!(
            dto.printer_online, 1,
            "printer_total(2) - printer_offline(1)"
        );
    })
    .await
    .expect("printers_soft_delete_dashboard_counters_ignore_deleted exceeded 30 s budget");
}

// ---------------------------------------------------------------------------
// Тест 2b: printer_total отдельно (printer_online при offline == 0)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn printers_soft_delete_dashboard_total_ignores_deleted() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let f = setup_deleted_hidden().await;
        // alert'ов нет: printer_online == printer_total напрямую.
        let dto = f
            .dashboard
            .get_all_widgets(&Identity::trusted_admin(), None)
            .await
            .expect("widgets");
        assert_eq!(
            dto.printer_online, 1,
            "printer_total не должен считать скрытый"
        );

        restore_device(&f).await;
        let dto = f
            .dashboard
            .get_all_widgets(&Identity::trusted_admin(), None)
            .await
            .expect("widgets after restore");
        assert_eq!(dto.printer_online, 2, "после возврата printer_total == 2");
    })
    .await
    .expect("printers_soft_delete_dashboard_total_ignores_deleted exceeded 30 s budget");
}

// ---------------------------------------------------------------------------
// Тест 3: выборка поллера
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn printers_soft_delete_poller_skips_deleted() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let f = setup_deleted_hidden().await;
        f.printers.poll_all().await;
        assert_eq!(
            reading_count(&f, f.hidden_printer_id).await,
            0,
            "скрытый принтер поллер опрашивать не должен (community уходила бы в сеть)"
        );
        assert!(
            reading_count(&f, f.live_printer_id).await > 0,
            "живой принтер поллером опрошен (SNMP-мок пишет показания)"
        );
    })
    .await
    .expect("printers_soft_delete_poller_skips_deleted exceeded 30 s budget");
}

// ---------------------------------------------------------------------------
// Тест 3b (T-41.1-22-02): опрос ПО ТРЕБОВАНИЮ тоже не ходит к скрытому
// ---------------------------------------------------------------------------
//
// Периодический поллер берёт выборку из отфильтрованного `list`, а
// `poll_single` получает id от вызывающего (`printers_refresh`, оба
// транспорта). Пока он резолвил id через неотфильтрованный `get`, держатель
// `ReadPrinters` мог заставить нас отправить SNMP с сохранённой community на
// адрес уже удалённого принтера. Мутация: вернуть `repo.get` вместо
// `repo.get_live` в `PrinterService::poll_single` — тест краснеет.

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn printers_soft_delete_on_demand_poll_skips_deleted() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let f = setup_deleted_hidden().await;

        f.printers.poll_single(f.hidden_printer_id).await;
        assert_eq!(
            reading_count(&f, f.hidden_printer_id).await,
            0,
            "опрос по требованию скрытого принтера не должен слать SNMP (community уходила бы в сеть)"
        );

        // Вакуумность исключена: тот же вызов на живом принтере пишет показание.
        f.printers.poll_single(f.live_printer_id).await;
        assert!(
            reading_count(&f, f.live_printer_id).await > 0,
            "живой принтер по требованию опрашивается (SNMP-мок пишет показания)"
        );
    })
    .await
    .expect("printers_soft_delete_on_demand_poll_skips_deleted exceeded 30 s budget");
}

// ---------------------------------------------------------------------------
// Тест 4: возврат с теми же SNMP-настройками, строка printers не пересоздана
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn printers_soft_delete_restore_returns_same_snmp_settings() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let f = setup_alive().await;
        let before = printer_snapshot(&f, f.hidden_printer_id).await;
        assert_eq!(before.2, HIDDEN_COMMUNITY);
        assert!(before.3.is_some(), "oid_profile_id задан до удаления");
        assert!(before.4.is_some(), "usb_host_device_id задан до удаления");

        f.devices
            .delete_soft(f.hidden_device_id, f.hidden_device_version)
            .await
            .expect("soft-delete");
        let during = printer_snapshot(&f, f.hidden_printer_id).await;
        assert_eq!(during, before, "мягкое удаление не трогает строку printers");

        restore_device(&f).await;
        let after = printer_snapshot(&f, f.hidden_printer_id).await;
        assert_eq!(
            after, before,
            "id, version, community, oid_profile_id, usb_host_device_id сохранились"
        );

        let (ids, total) = list_ids(&f).await;
        let mut expected = vec![f.hidden_printer_id, f.live_printer_id];
        expected.sort();
        assert_eq!(ids, expected, "после возврата оба принтера в списке");
        assert_eq!(total, 2);

        f.printers.poll_all().await;
        assert!(
            reading_count(&f, f.hidden_printer_id).await > 0,
            "после возврата принтер снова попадает в выборку поллера"
        );
    })
    .await
    .expect("printers_soft_delete_restore_returns_same_snmp_settings exceeded 30 s budget");
}

// ---------------------------------------------------------------------------
// Тест 5: get не фильтруется
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn printers_soft_delete_get_still_returns_deleted() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let f = setup_deleted_hidden().await;
        let dto = f
            .printers
            .get(f.hidden_printer_id)
            .await
            .expect("get по id скрытого принтера не должен ломаться");
        assert_eq!(dto.id, f.hidden_printer_id);
        assert_eq!(dto.ip_address.as_deref(), Some(HIDDEN_IP));
    })
    .await
    .expect("printers_soft_delete_get_still_returns_deleted exceeded 30 s budget");
}

// ---------------------------------------------------------------------------
// Тест 6: проверка дубликата IP при обнаружении (следствие D-37)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn printers_soft_delete_discover_duplicate_check_ignores_deleted() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let f = setup_deleted_hidden().await;

        // Общий источник проверки дубликата — repo.list с аргументами discover.
        let seen = ips_seen_by_discover(&f).await;
        assert!(
            !seen.iter().any(|ip| ip == HIDDEN_IP),
            "IP скрытого принтера не дубликат для обнаружения"
        );
        assert!(seen.iter().any(|ip| ip == LIVE_IP));

        // Сквозная проверка через discover по диапазону из одного IP (мок знает его).
        let found = f
            .printers
            .discover(HIDDEN_IP, HIDDEN_IP, "public", &Identity::trusted_admin())
            .await
            .expect("discover");
        assert_eq!(found.len(), 1);
        assert!(
            !found[0].is_duplicate,
            "обнаружение предлагает IP мягко удалённого принтера снова"
        );

        restore_device(&f).await;
        let seen = ips_seen_by_discover(&f).await;
        assert!(
            seen.iter().any(|ip| ip == HIDDEN_IP),
            "после возврата IP снова виден"
        );
        let found = f
            .printers
            .discover(HIDDEN_IP, HIDDEN_IP, "public", &Identity::trusted_admin())
            .await
            .expect("discover after restore");
        assert!(found[0].is_duplicate, "после возврата IP снова дубликат");
    })
    .await
    .expect("printers_soft_delete_discover_duplicate_check_ignores_deleted exceeded 30 s budget");
}

// ---------------------------------------------------------------------------
// Тест 7 (Phase 41.13, PGR-04, D-08): имя и инв. № USB-хоста в DTO принтера
// ---------------------------------------------------------------------------

/// Фикстура расходящаяся: у скрытого принтера хост («Рабочая станция А», инв.
/// «ИНВ-114» сеется здесь) есть, у живого — нет. Имя принтера («Принтер А»)
/// отличается от имени хоста, поэтому JOIN не по той колонке даёт другую
/// строку, а не ту же самую.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn printers_get_returns_usb_host_name_and_inventory_number() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let f = setup_alive().await;

        // (1) id хоста скрытого принтера + инвентарный номер хосту.
        let printer_id = f.hidden_printer_id;
        let host_id: i64 = {
            let readers = f.devices.readers.clone();
            tokio::task::spawn_blocking(move || {
                readers
                    .acquire()
                    .query_row(
                        "SELECT usb_host_device_id FROM printers WHERE id = ?1",
                        params![printer_id],
                        |r| r.get(0),
                    )
                    .expect("usb_host_device_id скрытого принтера")
            })
            .await
            .expect("spawn_blocking")
        };
        f.devices
            .writer
            .execute(move |conn| {
                conn.execute(
                    "UPDATE devices SET inventory_number = 'ИНВ-114' WHERE id = ?1",
                    params![host_id],
                )
                .map_err(trackly_infra::error_conversions::map_rusqlite)?;
                Ok(())
            })
            .await
            .expect("seed host inventory number");

        // (2) оба канала чтения несут имя и инв. № хоста.
        let by_id = f.printers.get(f.hidden_printer_id).await.expect("get");
        assert_eq!(by_id.usb_host_device_id, Some(host_id));
        assert_eq!(by_id.usb_host_name.as_deref(), Some("Рабочая станция А"));
        assert_eq!(by_id.usb_host_inventory_no.as_deref(), Some("ИНВ-114"));
        let by_device = f
            .printers
            .get_by_device_id(f.hidden_device_id)
            .await
            .expect("get_by_device_id");
        assert_eq!(
            by_device.usb_host_name.as_deref(),
            Some("Рабочая станция А")
        );
        assert_eq!(by_device.usb_host_inventory_no.as_deref(), Some("ИНВ-114"));

        // (3) принтер без хоста: оба поля None.
        let live = f.printers.get(f.live_printer_id).await.expect("get live");
        assert_eq!(live.usb_host_device_id, None);
        assert_eq!(live.usb_host_name, None);
        assert_eq!(live.usb_host_inventory_no, None);

        // (3б) хост без инвентарного номера: имя есть, номер None.
        f.devices
            .writer
            .execute(move |conn| {
                conn.execute(
                    "UPDATE devices SET inventory_number = NULL WHERE id = ?1",
                    params![host_id],
                )
                .map_err(trackly_infra::error_conversions::map_rusqlite)?;
                Ok(())
            })
            .await
            .expect("clear host inventory number");
        let no_inv = f
            .printers
            .get(f.hidden_printer_id)
            .await
            .expect("get no inv");
        assert_eq!(no_inv.usb_host_name.as_deref(), Some("Рабочая станция А"));
        assert_eq!(no_inv.usb_host_inventory_no, None);
        f.devices
            .writer
            .execute(move |conn| {
                conn.execute(
                    "UPDATE devices SET inventory_number = 'ИНВ-114' WHERE id = ?1",
                    params![host_id],
                )
                .map_err(trackly_infra::error_conversions::map_rusqlite)?;
                Ok(())
            })
            .await
            .expect("restore host inventory number");

        // (4) мягко удалённый хост: подпись остаётся (D-37).
        let host = f.devices.get(host_id).await.expect("get host device");
        f.devices
            .delete_soft(host_id, host.version)
            .await
            .expect("soft-delete usb host device");
        let after = f
            .printers
            .get(f.hidden_printer_id)
            .await
            .expect("get after host delete");
        assert_eq!(after.usb_host_device_id, Some(host_id));
        assert_eq!(after.usb_host_name.as_deref(), Some("Рабочая станция А"));
        assert_eq!(after.usb_host_inventory_no.as_deref(), Some("ИНВ-114"));
    })
    .await
    .expect("printers_get_returns_usb_host_name_and_inventory_number exceeded 30 s budget");
}
