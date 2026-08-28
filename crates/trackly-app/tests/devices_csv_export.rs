//! Интеграционные тесты экспорта устройств в CSV.
//!
//! Проверяем `DeviceService::export_csv`:
//! - UTF-8 BOM (EF BB BF) в начале строки
//! - Разделитель `;` (не `,`)
//! - Русские заголовки (включая колонку «Место», Phase 39)
//! - Round-trip кириллицы через export после create
//! - D-27/Phase 39: в колонке «Место» печатается ПОЛНЫЙ путь места из
//!   `place_full_paths` («Здание А / 2 этаж / Кабинет 214», разделитель
//!   ровно `' / '` — см. `migrations/V037__places.sql`), а не имя листа и
//!   не пустая строка.
//! - D-24/Phase 39.1 Plan 03: даже когда у места установлен
//!   `path_variant_override = 'ends'` (короткий путь отличался бы от
//!   полного), колонка «Место» остаётся на ПОЛНОМ пути — `export_csv`
//!   никогда не подключается к `place_effective_variant`/`place_path_short`.
//!
//! Каждый тест обёрнут в `tokio::time::timeout(30s)`.
//!
//! Все названия мест и устройств ВЫМЫШЛЕННЫЕ (жёсткое условие приватности:
//! репозиторий публичный).

use std::sync::Arc;
use std::time::Duration;

use trackly_app::dto::device::{DeviceFilter, DeviceNew, Pagination};
use trackly_app::services::DeviceService;
use trackly_core::domain::places::{PlaceKind, PlaceNew};
use trackly_core::ports::places::PlaceRepository;
use trackly_core::primitives::clock::Clock;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::repos::SqlitePlaceRepository;
use trackly_infra::test_support::test_writer_and_readers;

fn make_service() -> (DeviceService, tempfile::TempDir) {
    let (writer, readers, dir) = test_writer_and_readers();
    let clock: Arc<dyn Clock + Send + Sync> = Arc::new(SystemClock);
    let svc = DeviceService::new(writer, readers, clock);
    (svc, dir)
}

/// Создаёт узел дерева мест напрямую через `SqlitePlaceRepository` на writer'е
/// сервиса — тот же приём, что и `seed_place` в `devices_csv_import.rs`
/// (D-18: место создаётся только явным вызовом, ни один write-path устройств
/// его не заводит неявно).
async fn seed_place(
    svc: &DeviceService,
    parent_id: Option<i64>,
    kind: PlaceKind,
    name: &str,
) -> i64 {
    let name = name.to_string();
    svc.writer
        .execute(move |conn| {
            let repo = SqlitePlaceRepository;
            repo.create(
                conn,
                &PlaceNew {
                    parent_id,
                    kind,
                    name: name.clone(),
                    level: None,
                    is_storage: false,
                    sort_order: None,
                    notes: None,
                },
                1_700_000_000,
            )
        })
        .await
        .expect("seed place")
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
            place_id: None,
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
        // Phase 39: колонка места переименована в «Место» (было «Расположение»)
        // и печатает полный путь дерева — заголовок обязан присутствовать
        // ВСЕГДА, даже на пустой БД.
        assert!(
            first_line.contains("Место"),
            "заголовок должен содержать 'Место': {first_line}"
        );
    })
    .await
    .expect("timeout");
}

/// Заголовок «Место» стоит между «Состояние» и «Статус» — порядок колонок
/// важен, потому что импорт (`devices_csv_import.rs`) сопоставляет колонки
/// по имени заголовка, а пользователь редактирует выгрузку в Excel.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn export_header_has_mesto_column_in_expected_position() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();

        let csv = svc
            .export_csv(DeviceFilter::default())
            .await
            .expect("export should succeed");

        let without_bom = csv.trim_start_matches('\u{FEFF}');
        let first_line = without_bom
            .lines()
            .next()
            .unwrap_or("")
            .trim_end_matches('\r');
        let headers: Vec<&str> = first_line.split(';').collect();

        let idx = headers
            .iter()
            .position(|h| *h == "Место")
            .unwrap_or_else(|| panic!("нет колонки «Место» в заголовке: {headers:?}"));
        assert_eq!(
            headers.get(idx - 1).copied(),
            Some("Состояние"),
            "перед «Место» должна идти «Состояние»: {headers:?}"
        );
        assert_eq!(
            headers.get(idx + 1).copied(),
            Some("Статус"),
            "после «Место» должен идти «Статус»: {headers:?}"
        );
    })
    .await
    .expect("timeout");
}

/// D-27/Phase 39: устройство, лежащее в узле 3-го уровня, экспортируется с
/// ПОЛНЫМ путём («Здание А / 2 этаж / Кабинет 214»), а не с именем листа.
/// Разделитель ровно `' / '` — как в `place_full_paths` (V037).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn export_place_column_carries_full_tree_path() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();

        let building = seed_place(&svc, None, PlaceKind::Building, "Здание А").await;
        let floor = seed_place(&svc, Some(building), PlaceKind::Floor, "2 этаж").await;
        let room = seed_place(&svc, Some(floor), PlaceKind::Room, "Кабинет 214").await;

        svc.create(DeviceNew {
            type_id: 1,
            name: "Ноутбук А-214".to_string(),
            inventory_no: Some("INV-PLACE-001".to_string()),
            serial_no: None,
            model: None,
            specs: None,
            kit: None,
            state: None,
            place_id: Some(room),
            status_id: 1,
        })
        .await
        .expect("create device in nested place");

        let csv = svc
            .export_csv(DeviceFilter::default())
            .await
            .expect("export should succeed");

        let without_bom = csv.trim_start_matches('\u{FEFF}');
        let mut lines = without_bom.lines().filter(|l| !l.trim().is_empty());
        let header: Vec<String> = lines
            .next()
            .expect("header row")
            .trim_end_matches('\r')
            .split(';')
            .map(|s| s.to_string())
            .collect();
        let data: Vec<String> = lines
            .next()
            .expect("одна строка данных")
            .trim_end_matches('\r')
            .split(';')
            .map(|s| s.to_string())
            .collect();

        let idx = header
            .iter()
            .position(|h| h == "Место")
            .unwrap_or_else(|| panic!("нет колонки «Место»: {header:?}"));

        assert_eq!(
            data.get(idx).map(String::as_str),
            Some("Здание А / 2 этаж / Кабинет 214"),
            "колонка «Место» должна содержать полный путь дерева, а не имя \
             листа и не пустую строку. header={header:?} data={data:?}"
        );
    })
    .await
    .expect("timeout");
}

/// Устройство БЕЗ места экспортируется с пустой ячейкой «Место» — колонка не
/// сдвигается и не подставляет мусор (все остальные тесты этого файла
/// сеют `place_id: None`, но ни один не смотрел на саму ячейку).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn export_place_column_empty_when_device_has_no_place() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();

        svc.create(DeviceNew {
            type_id: 1,
            name: "Ноутбук без места".to_string(),
            inventory_no: None,
            serial_no: None,
            model: None,
            specs: None,
            kit: None,
            state: None,
            place_id: None,
            status_id: 1,
        })
        .await
        .expect("create device without place");

        let csv = svc
            .export_csv(DeviceFilter::default())
            .await
            .expect("export should succeed");

        let without_bom = csv.trim_start_matches('\u{FEFF}');
        let mut lines = without_bom.lines().filter(|l| !l.trim().is_empty());
        let header: Vec<String> = lines
            .next()
            .expect("header row")
            .trim_end_matches('\r')
            .split(';')
            .map(|s| s.to_string())
            .collect();
        let data: Vec<String> = lines
            .next()
            .expect("одна строка данных")
            .trim_end_matches('\r')
            .split(';')
            .map(|s| s.to_string())
            .collect();

        let idx = header.iter().position(|h| h == "Место").expect("«Место»");
        assert_eq!(
            data.get(idx).map(String::as_str),
            Some(""),
            "без места ячейка «Место» должна быть пустой: {data:?}"
        );
        assert_eq!(
            data.len(),
            header.len(),
            "число ячеек данных должно совпадать с числом заголовков"
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
            place_id: None,
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
            place_id: None,
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
                place_id: None,
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

/// D-24 (Phase 39.1 Plan 03): устройство лежит в месте с явным
/// `path_variant_override = 'ends'` (короткий путь "Здание А // 1-05"
/// отличался бы от полного) — колонка «Место» в CSV всё равно печатает
/// ПОЛНЫЙ путь ("Здание А / 1 этаж / 1-05"), а не сокращённый. Доказывает
/// отсутствие регрессии, а не просто "тест проходит потому что код не
/// менялся" — сокращение реально было бы видно, если бы `export_csv`
/// случайно начал использовать `place_path_short`.
///
/// Нет ещё сервисного метода для установки `path_variant_override`
/// (Phase 39.1 Plan 04+) — override ставится напрямую SQL-ом через
/// `svc.writer`, тем же приёмом, что `place_effective_variant.rs`.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn export_csv_place_column_stays_full_path_not_shortened() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();

        let building = seed_place(&svc, None, PlaceKind::Building, "Здание А").await;
        let floor = seed_place(&svc, Some(building), PlaceKind::Floor, "1 этаж").await;
        let room = seed_place(&svc, Some(floor), PlaceKind::Room, "1-05").await;

        // Явный override 'ends' на самом месте — короткий путь был бы
        // "Здание А // 1-05", отличным от полного.
        svc.writer
            .execute(move |conn| {
                conn.execute(
                    "UPDATE places SET path_variant_override = 'ends' WHERE id = ?1",
                    rusqlite::params![room],
                )
                .map_err(|e| trackly_core::error::AppError::Internal {
                    source_chain: format!("set path_variant_override in test: {e}"),
                })?;
                Ok(())
            })
            .await
            .expect("set path_variant_override");

        svc.create(DeviceNew {
            type_id: 1,
            name: "Ноутбук D-24".to_string(),
            inventory_no: Some("INV-D24-001".to_string()),
            serial_no: None,
            model: None,
            specs: None,
            kit: None,
            state: None,
            place_id: Some(room),
            status_id: 1,
        })
        .await
        .expect("create device in overridden place");

        let csv = svc
            .export_csv(DeviceFilter::default())
            .await
            .expect("export should succeed");

        let without_bom = csv.trim_start_matches('\u{FEFF}');
        let mut lines = without_bom.lines().filter(|l| !l.trim().is_empty());
        let header: Vec<String> = lines
            .next()
            .expect("header row")
            .trim_end_matches('\r')
            .split(';')
            .map(|s| s.to_string())
            .collect();
        let data: Vec<String> = lines
            .next()
            .expect("одна строка данных")
            .trim_end_matches('\r')
            .split(';')
            .map(|s| s.to_string())
            .collect();

        let idx = header
            .iter()
            .position(|h| h == "Место")
            .unwrap_or_else(|| panic!("нет колонки «Место»: {header:?}"));

        assert_eq!(
            data.get(idx).map(String::as_str),
            Some("Здание А / 1 этаж / 1-05"),
            "колонка «Место» должна остаться ПОЛНЫМ путём, даже когда \
             у места есть path_variant_override='ends' (D-24). \
             header={header:?} data={data:?}"
        );
        assert_ne!(
            data.get(idx).map(String::as_str),
            Some("Здание А // 1-05"),
            "колонка «Место» НЕ должна содержать сокращённую форму (D-24 регресс)"
        );
    })
    .await
    .expect("timeout");
}
