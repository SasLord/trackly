//! Интеграционные тесты экспорта устройств в CSV.
//!
//! Проверяем `DeviceService::export_csv`:
//! - UTF-8 BOM (EF BB BF) в начале строки
//! - Разделитель `;` (не `,`)
//! - Русские заголовки
//! - Round-trip кириллицы через export после create
//!
//! Каждый тест обёрнут в `tokio::time::timeout(30s)`.

use std::sync::Arc;
use std::time::Duration;

use trackly_app::dto::device::{DeviceFilter, DeviceNew, Pagination};
use trackly_app::services::DeviceService;
use trackly_core::primitives::clock::Clock;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::test_support::test_writer_and_readers;

fn make_service() -> (DeviceService, tempfile::TempDir) {
    let (writer, readers, dir) = test_writer_and_readers();
    let clock: Arc<dyn Clock + Send + Sync> = Arc::new(SystemClock);
    let svc = DeviceService::new(writer, readers, clock);
    (svc, dir)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn export_starts_with_bom() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();

        let csv = svc
            .export_csv(DeviceFilter::default())
            .await
            .expect("export should succeed");

        let bytes: Vec<u8> = csv.bytes().take(3).collect();
        assert_eq!(
            bytes,
            vec![0xEF, 0xBB, 0xBF],
            "CSV должен начинаться с UTF-8 BOM (EF BB BF), got: {bytes:?}"
        );
    })
    .await
    .expect("timeout");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn export_uses_semicolon_delimiter() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();

        // Create a device so there's at least one data row.
        svc.create(DeviceNew {
            type_id: 1,
            name: "Тестовый принтер".to_string(),
            inventory_no: Some("INV-EXPORT-001".to_string()),
            serial_no: None,
            model: Some("TestModel".to_string()),
            specs: None,
            kit: None,
            state: None,
            location: None,
            location_id: None,
            status_id: 1,
        })
        .await
        .expect("create device");

        let csv = svc
            .export_csv(DeviceFilter::default())
            .await
            .expect("export should succeed");

        // Skip BOM (3 bytes / 3 chars), then check first data row
        let without_bom = csv.trim_start_matches('\u{FEFF}');
        assert!(
            without_bom.contains(';'),
            "CSV должен использовать ';' как разделитель:\n{without_bom}"
        );
        assert!(
            !without_bom.contains(','),
            "CSV НЕ должен содержать ',' в заголовках/данных:\n{without_bom}"
        );
    })
    .await
    .expect("timeout");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn export_headers_russian() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();

        let csv = svc
            .export_csv(DeviceFilter::default())
            .await
            .expect("export should succeed");

        let without_bom = csv.trim_start_matches('\u{FEFF}');
        let first_line = without_bom.lines().next().unwrap_or("");

        assert!(
            first_line.contains("Тип"),
            "заголовок должен содержать 'Тип': {first_line}"
        );
        assert!(
            first_line.contains("Наименование"),
            "заголовок должен содержать 'Наименование': {first_line}"
        );
        assert!(
            first_line.contains("Инвентарный №"),
            "заголовок должен содержать 'Инвентарный №': {first_line}"
        );
        assert!(
            first_line.contains("Статус"),
            "заголовок должен содержать 'Статус': {first_line}"
        );
    })
    .await
    .expect("timeout");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn export_cyrillic_roundtrip() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();

        let fixture_name = "Сидоров-Петроградский Иван Александрович (ё) №42";

        svc.create(DeviceNew {
            type_id: 1,
            name: fixture_name.to_string(),
            inventory_no: None,
            serial_no: None,
            model: None,
            specs: None,
            kit: None,
            state: None,
            location: None,
            location_id: None,
            status_id: 1,
        })
        .await
        .expect("create device with cyrillic name");

        let csv = svc
            .export_csv(DeviceFilter::default())
            .await
            .expect("export should succeed");

        assert!(
            csv.contains(fixture_name),
            "CSV должен содержать кириллическое имя '{fixture_name}'"
        );

        // Verify raw UTF-8 encoding is intact by checking for the BOM prefix.
        let bytes: Vec<u8> = csv.bytes().take(3).collect();
        assert_eq!(bytes, vec![0xEF, 0xBB, 0xBF], "BOM должен присутствовать");
    })
    .await
    .expect("timeout");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn export_empty_database_only_headers() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();

        let csv = svc
            .export_csv(DeviceFilter::default())
            .await
            .expect("export empty DB should succeed");

        let without_bom = csv.trim_start_matches('\u{FEFF}');
        let lines: Vec<&str> = without_bom
            .lines()
            .filter(|l| !l.trim().is_empty())
            .collect();

        // Only one line (headers), no data rows.
        assert_eq!(
            lines.len(),
            1,
            "пустая БД: должна быть только строка заголовков, got {lines:?}"
        );
    })
    .await
    .expect("timeout");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn export_formula_injection_prevention() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();

        // Device name starting with '=' — should be prefixed with ' to prevent formula injection.
        svc.create(DeviceNew {
            type_id: 1,
            name: "=SUM(1+1)".to_string(),
            inventory_no: None,
            serial_no: None,
            model: None,
            specs: None,
            kit: None,
            state: None,
            location: None,
            location_id: None,
            status_id: 1,
        })
        .await
        .expect("create device with formula name");

        let csv = svc
            .export_csv(DeviceFilter::default())
            .await
            .expect("export should succeed");

        // The raw '=SUM...' should NOT appear verbatim — must be prefixed with apostrophe.
        assert!(
            !csv.contains(";=SUM"),
            "formula injection: '=SUM...' должен быть заэкранирован (апостроф), csv:\n{csv}"
        );
        assert!(
            csv.contains("'=SUM"),
            "formula injection: '=SUM...' должен быть заэкранирован апострофом в CSV"
        );
    })
    .await
    .expect("timeout");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn export_list_all_created_devices() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();

        // Create 3 devices.
        for i in 1..=3 {
            svc.create(DeviceNew {
                type_id: 1,
                name: format!("Устройство {i}"),
                inventory_no: Some(format!("INV-00{i}")),
                serial_no: None,
                model: None,
                specs: None,
                kit: None,
                state: None,
                location: None,
                location_id: None,
                status_id: 1,
            })
            .await
            .expect("create device");
        }

        let csv = svc
            .export_csv(DeviceFilter::default())
            .await
            .expect("export should succeed");

        // BOM + 1 header row + 3 data rows = 4 non-empty lines.
        let without_bom = csv.trim_start_matches('\u{FEFF}');
        let lines: Vec<&str> = without_bom
            .lines()
            .filter(|l| !l.trim().is_empty())
            .collect();

        assert_eq!(
            lines.len(),
            4,
            "ожидаем 4 строки (заголовок + 3 данных), got {lines:?}"
        );

        // Verify filter: status_id=1 returns all 3.
        let list_resp = svc
            .list(
                DeviceFilter {
                    status_id: Some(1),
                    ..Default::default()
                },
                Pagination {
                    offset: 0,
                    limit: 10,
                },
            )
            .await
            .expect("list");
        assert_eq!(list_resp.total, 3);
    })
    .await
    .expect("timeout");
}
