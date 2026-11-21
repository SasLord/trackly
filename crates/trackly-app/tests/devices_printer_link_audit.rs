//! quick 261121-hed, INT-30: смена IP, community или USB-хоста принтера через
//! `DeviceService::update` пишет ровно одну строку `audit_log`
//! `action = 'custom:printer_connection'` с payload только по изменившимся полям.
//! IP и хост — «было/стало», community — только факт смены (значение — секрет SNMP
//! и не попадает в журнал). Сохранение без изменения этих полей строки не пишет.
//!
//! Приватность (CLAUDE.md): только вымышленные названия («Принтер А», «Рабочая
//! станция А/Б»), вымышленный community «s3cr3t-comm».

use std::sync::Arc;
use std::time::Duration;

use rusqlite::params;

use trackly_app::dto::device::{DeviceDto, DeviceNew, DevicePatch};
use trackly_app::dto::number_template::{NumberFieldInput, TemplateContextDto};
use trackly_app::dto::printer::PrinterCreateBlockDto;
use trackly_app::services::DeviceService;
use trackly_core::auth::Identity;
use trackly_core::primitives::clock::Clock;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::test_support::test_writer_and_readers;

const DEVICE_TYPE_ID: i64 = 1;
const PRINTER_TYPE_ID: i64 = 2;
const SECRET: &str = "s3cr3t-comm";

fn make_service() -> (DeviceService, tempfile::TempDir) {
    let (writer, readers, dir) = test_writer_and_readers();
    let clock: Arc<dyn Clock + Send + Sync> = Arc::new(SystemClock);
    (DeviceService::new(writer, readers, clock), dir)
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

async fn create(
    svc: &DeviceService,
    name: &str,
    type_id: i64,
    block: Option<PrinterCreateBlockDto>,
) -> DeviceDto {
    svc.create_single_with_number_check_with_printer(
        minimal_new(name, type_id),
        empty_number_input(),
        TemplateContextDto::PrinterCreate,
        block,
    )
    .await
    .expect("create")
    .expect_created("create")
}

async fn printer(svc: &DeviceService, ip: Option<&str>, host: Option<i64>) -> DeviceDto {
    create(
        svc,
        "Принтер А",
        PRINTER_TYPE_ID,
        Some(PrinterCreateBlockDto {
            ip_address: ip.map(str::to_string),
            community: None,
            usb_host_device_id: host,
        }),
    )
    .await
}

async fn update(svc: &DeviceService, dto: &DeviceDto, patch: DevicePatch) -> DeviceDto {
    svc.update(&admin_caller(), dto.id, dto.version, patch)
        .await
        .expect("update")
        .expect_created("update")
}

/// payload_json строк `custom:printer_connection` устройства, по порядку.
async fn connection_rows(svc: &DeviceService, device_id: i64) -> Vec<serde_json::Value> {
    let readers = svc.readers.clone();
    tokio::task::spawn_blocking(move || {
        let conn = readers.acquire();
        let mut stmt = conn
            .prepare(
                "SELECT payload_json FROM audit_log WHERE entity_type = 'device' \
                 AND entity_id = ?1 AND action = 'custom:printer_connection' ORDER BY id",
            )
            .unwrap();
        stmt.query_map(params![device_id], |r| r.get::<_, String>(0))
            .unwrap()
            .map(|r| serde_json::from_str(&r.unwrap()).unwrap())
            .collect()
    })
    .await
    .unwrap()
}

/// Все текстовые поля всех строк audit_log устройства одной строкой — для поиска утечки.
async fn audit_dump(svc: &DeviceService, device_id: i64) -> String {
    let readers = svc.readers.clone();
    tokio::task::spawn_blocking(move || {
        let conn = readers.acquire();
        let mut stmt = conn
            .prepare(
                "SELECT action || '|' || COALESCE(before_json,'') || '|' || \
                 COALESCE(after_json,'') || '|' || COALESCE(payload_json,'') \
                 FROM audit_log WHERE entity_type = 'device' AND entity_id = ?1 \
                 AND action = 'custom:printer_connection'",
            )
            .unwrap();
        stmt.query_map(params![device_id], |r| r.get::<_, String>(0))
            .unwrap()
            .map(|r| r.unwrap())
            .collect::<Vec<_>>()
            .join("\n")
    })
    .await
    .unwrap()
}

async fn update_rows(svc: &DeviceService, device_id: i64) -> i64 {
    let readers = svc.readers.clone();
    tokio::task::spawn_blocking(move || {
        let conn = readers.acquire();
        conn.query_row(
            "SELECT COUNT(*) FROM audit_log WHERE entity_type = 'device' AND entity_id = ?1 \
             AND action = 'update'",
            params![device_id],
            |r| r.get(0),
        )
        .unwrap()
    })
    .await
    .unwrap()
}

async fn run<F: std::future::Future<Output = ()>>(f: F) {
    tokio::time::timeout(Duration::from_secs(30), f)
        .await
        .expect("test timed out");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn ip_change_writes_one_from_to_row() {
    run(async {
        let (svc, _d) = make_service();
        let p = printer(&svc, Some("10.0.0.5"), None).await;
        update(
            &svc,
            &p,
            DevicePatch {
                printer_ip_address: Some(Some("10.0.0.6".into())),
                ..Default::default()
            },
        )
        .await;
        let rows = connection_rows(&svc, p.id).await;
        assert_eq!(rows.len(), 1, "{rows:?}");
        assert_eq!(
            rows[0],
            serde_json::json!({ "ip_address": { "from": "10.0.0.5", "to": "10.0.0.6" } })
        );
        assert_eq!(
            update_rows(&svc, p.id).await,
            1,
            "обычная строка update пишется"
        );
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn usb_host_change_and_clear_are_recorded() {
    run(async {
        let (svc, _d) = make_service();
        let a = create(&svc, "Рабочая станция А", DEVICE_TYPE_ID, None).await;
        let b = create(&svc, "Рабочая станция Б", DEVICE_TYPE_ID, None).await;
        let p = printer(&svc, None, Some(a.id)).await;
        let p = update(
            &svc,
            &p,
            DevicePatch {
                usb_host_device_id: Some(Some(b.id)),
                ..Default::default()
            },
        )
        .await;
        update(
            &svc,
            &p,
            DevicePatch {
                usb_host_device_id: Some(None),
                ..Default::default()
            },
        )
        .await;
        let rows = connection_rows(&svc, p.id).await;
        assert_eq!(rows.len(), 2, "{rows:?}");
        assert_eq!(
            rows[0],
            serde_json::json!({ "usb_host_device_id": { "from": a.id, "to": b.id } })
        );
        assert_eq!(
            rows[1],
            serde_json::json!({ "usb_host_device_id": { "from": b.id, "to": null } })
        );
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn community_change_records_fact_without_value() {
    run(async {
        let (svc, _d) = make_service();
        let p = printer(&svc, Some("10.0.0.5"), None).await;
        update(
            &svc,
            &p,
            DevicePatch {
                printer_community: Some(SECRET.into()),
                ..Default::default()
            },
        )
        .await;
        let rows = connection_rows(&svc, p.id).await;
        assert_eq!(rows.len(), 1, "{rows:?}");
        assert_eq!(
            rows[0],
            serde_json::json!({ "community": { "changed": true } })
        );
        let dump = audit_dump(&svc, p.id).await;
        assert!(
            !dump.contains(SECRET),
            "значение community в аудите: {dump}"
        );
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn ip_and_community_in_one_save_give_one_row_with_two_keys() {
    run(async {
        let (svc, _d) = make_service();
        let p = printer(&svc, Some("10.0.0.5"), None).await;
        update(
            &svc,
            &p,
            DevicePatch {
                printer_ip_address: Some(Some("10.0.0.7".into())),
                printer_community: Some(SECRET.into()),
                ..Default::default()
            },
        )
        .await;
        let rows = connection_rows(&svc, p.id).await;
        assert_eq!(rows.len(), 1, "{rows:?}");
        assert_eq!(
            rows[0],
            serde_json::json!({
                "ip_address": { "from": "10.0.0.5", "to": "10.0.0.7" },
                "community": { "changed": true }
            })
        );
        assert!(!audit_dump(&svc, p.id).await.contains(SECRET));
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn unchanged_connection_writes_no_row() {
    run(async {
        let (svc, _d) = make_service();
        let a = create(&svc, "Рабочая станция А", DEVICE_TYPE_ID, None).await;
        let p = printer(&svc, Some("10.0.0.5"), Some(a.id)).await;
        // Те же значения IP/хоста (+ правка имени) — строки подключения нет.
        let p = update(
            &svc,
            &p,
            DevicePatch {
                name: Some("Принтер А (2 этаж)".into()),
                printer_ip_address: Some(Some("10.0.0.5".into())),
                usb_host_device_id: Some(Some(a.id)),
                ..Default::default()
            },
        )
        .await;
        // Только имя.
        update(
            &svc,
            &p,
            DevicePatch {
                name: Some("Принтер А".into()),
                ..Default::default()
            },
        )
        .await;
        assert!(connection_rows(&svc, p.id).await.is_empty());
        assert_eq!(update_rows(&svc, p.id).await, 2);
    })
    .await;
}
