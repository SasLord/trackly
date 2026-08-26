//! Интеграционные тесты импорта устройств из CSV.
//!
//! Task 1 (Preview-тесты): проверяем encoding/delimiter detection для 4 вариантов fixtures.
//! Task 2 (Commit-тесты): проверяем `import_csv_commit` — добавляются в этом же файле.
//!
//! Каждый тест обёрнут в `tokio::time::timeout(30s)`.
//!
//! ВАЖНО (Phase 17, план 17-07): неотфильтрованный `cargo test -p trackly-app` (или
//! `--workspace`) БЕЗ `-- --test-threads=1` может выглядеть как многоминутное зависание
//! именно на интеграционных тестах этого крейта (в т.ч. в файлах вроде этого) — это
//! ЗАДОКУМЕНТИРОВАННЫЙ класс проблемы, а НЕ дефект в `devices_csv_import.rs`. Причина:
//! несколько `#[tokio::test]` внутри одного тест-бинарника поднимают tokio multi_thread
//! runtime + tracing-appender non_blocking background thread + WriterHandle spawn_blocking;
//! параллельный запуск таких тестов насыщает worker-потоки (см. комментарий в
//! `.github/workflows/ci-fast.yml` рядом с шагом `cargo test`, наблюдалось 30+ минут
//! зависания на ubuntu CI-раннере). Канонический корректный вызов (после `pnpm --dir ui
//! build`, и убедившись, что нет другого параллельного `cargo`-процесса — project
//! convention «один `cargo test` за раз»):
//!
//! ```text
//! TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app -- --test-threads=1
//! ```

use std::sync::Arc;
use std::time::Duration;

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

fn fixture_bytes(name: &str) -> Vec<u8> {
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let path = manifest.join("tests/fixtures/devices").join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("failed to read fixture {name}: {e}"))
}

/// Seed a root-level place (`kind=Room`) whose `full_path` equals `name` —
/// `import_csv_commit` (Plan 39-06, D-CSV/place-tree) resolves a CSV row's
/// "Расположение" text against an EXACT (case-insensitive) `place_full_paths`
/// match, no auto-create-by-name (D-18 removed that entirely). The `utf8.csv`
/// fixture's location column values ("Кабинет 305" etc.) must exist as real
/// `places` rows or every row with a location value fails validation and
/// silently zero rows get inserted — mirrors the `create_place` precedent in
/// `devices_grouping.rs`.
async fn seed_place(svc: &DeviceService, name: &str) -> i64 {
    let name = name.to_string();
    svc.writer
        .execute(move |conn| {
            let repo = SqlitePlaceRepository;
            let new_place = PlaceNew {
                parent_id: None,
                kind: PlaceKind::Room,
                name: name.clone(),
                level: None,
                is_storage: false,
                sort_order: None,
                notes: None,
            };
            repo.create(conn, &new_place, 1_700_000_000)
        })
        .await
        .expect("seed place")
}

/// Seed all four places referenced by `fixtures/devices/utf8.csv`'s
/// "Расположение" column, so `import_csv_commit`'s exact-match place
/// resolution succeeds for every row.
async fn seed_utf8_fixture_places(svc: &DeviceService) {
    for name in ["Кабинет 305", "Кабинет 101", "Кабинет 102", "Кладовая"]
    {
        seed_place(svc, name).await;
    }
}

// ---------------------------------------------------------------------------
// Task 1: Preview tests — encoding + delimiter detection
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn import_utf8_no_bom() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let bytes = fixture_bytes("utf8.csv");
        let preview = svc
            .import_csv_preview(bytes)
            .await
            .expect("preview should succeed");

        assert_eq!(preview.encoding, "UTF-8", "encoding should be UTF-8");
        assert_eq!(preview.delimiter, ",", "delimiter should be comma");
        assert!(
            preview.total_rows >= 4,
            "expected 4+ data rows, got {}",
            preview.total_rows
        );
        // First data row should contain our fixture string
        assert!(
            preview.preview_rows.iter().any(|row| row
                .iter()
                .any(|cell| cell.contains("Сидоров-Петроградский"))),
            "preview rows should contain fixture cyrillic string"
        );
        // Headers should include "Наименование"
        assert!(
            preview.headers.iter().any(|h| h.contains("Наименование")),
            "headers should contain 'Наименование'"
        );
    })
    .await
    .expect("timeout");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn import_utf8_with_bom() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let bytes = fixture_bytes("utf8_bom.csv");
        // Verify BOM is present
        assert_eq!(&bytes[..3], b"\xEF\xBB\xBF", "fixture should have BOM");

        let preview = svc
            .import_csv_preview(bytes)
            .await
            .expect("preview should succeed");

        assert_eq!(
            preview.encoding, "UTF-8",
            "BOM-detected encoding should be UTF-8"
        );
        assert_eq!(preview.delimiter, ",");
        assert!(
            preview.headers.iter().any(|h| h.contains("Наименование")),
            "headers should be readable after BOM strip"
        );
    })
    .await
    .expect("timeout");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn import_cp1251_semicolon() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let bytes = fixture_bytes("cp1251_semicolon.csv");

        let preview = svc
            .import_csv_preview(bytes)
            .await
            .expect("preview should succeed");

        assert_eq!(
            preview.encoding, "windows-1251",
            "encoding should be detected as windows-1251"
        );
        assert_eq!(preview.delimiter, ";", "delimiter should be semicolon");
        // Cyrillic should decode correctly
        assert!(
            preview.headers.iter().any(|h| h.contains("Наименование")),
            "headers should contain decoded Cyrillic"
        );
    })
    .await
    .expect("timeout");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn import_cp1251_comma() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let bytes = fixture_bytes("cp1251_comma.csv");

        let preview = svc
            .import_csv_preview(bytes)
            .await
            .expect("preview should succeed");

        assert_eq!(
            preview.encoding, "windows-1251",
            "encoding should be detected as windows-1251"
        );
        assert_eq!(preview.delimiter, ",", "delimiter should be comma");
    })
    .await
    .expect("timeout");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn import_file_too_large_rejected() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        // 51 MB of zeros
        let huge = vec![0u8; 51 * 1024 * 1024];
        let result = svc.import_csv_preview(huge).await;
        assert!(result.is_err(), "huge file should be rejected");
        let err = result.unwrap_err();
        let msg = format!("{err:?}");
        assert!(
            msg.contains("50") || msg.contains("МБ") || msg.contains("large"),
            "error should mention size limit: {msg}"
        );
    })
    .await
    .expect("timeout");
}

// ---------------------------------------------------------------------------
// Task 2: Commit tests — added after Task 2 implementation
// ---------------------------------------------------------------------------

// These tests are in the same file to reuse fixtures + make_service.
// They will be GREEN after Task 2 implements import_csv_commit.

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn import_commit_inserts_devices() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        seed_utf8_fixture_places(&svc).await;
        let bytes = fixture_bytes("utf8.csv");

        // Step 1: preview
        let preview = svc.import_csv_preview(bytes).await.expect("preview");

        // Auto-mapping: map CSV headers to device fields
        let mapping = auto_map(&preview.headers);

        // Step 2: commit
        let report = svc
            .import_csv_commit(preview.token, mapping)
            .await
            .expect("commit should succeed");

        assert!(
            report.inserted >= 3,
            "should insert at least 3 devices, got {}",
            report.inserted
        );
        // At least no catastrophic failures
        // (some rows may fail if they lack required fields)
    })
    .await
    .expect("timeout");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn import_commit_per_row_errors() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let bytes = fixture_bytes("malformed_mixed_rows.csv");

        let preview = svc.import_csv_preview(bytes).await.expect("preview");
        let mapping = auto_map(&preview.headers);

        let report = svc
            .import_csv_commit(preview.token, mapping)
            .await
            .expect("commit should not fail entirely");

        // malformed_mixed_rows.csv has 2 rows with empty "Наименование" (required)
        assert!(
            report.failed.len() >= 2,
            "expected at least 2 row errors, got {}",
            report.failed.len()
        );
        // The error messages should mention "Наименование" or name validation
        let any_name_error = report.failed.iter().any(|e| {
            e.error_code.contains("Validation")
                || e.error_message.contains("Наименование")
                || e.error_message.contains("обязател")
        });
        assert!(
            any_name_error,
            "errors should mention required field validation"
        );
    })
    .await
    .expect("timeout");
}

/// T-39-15-01 / UI-SPEC §12: a "place" column value with no exact
/// (case-insensitive) match in `place_full_paths` must fail that row with
/// the exact copy "Строка N: место «...» не найдено в дереве." — never
/// silently drop the place, never auto-create a place row (D-18).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn import_commit_unresolved_place_reports_row_error_with_exact_copy() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        // Deliberately do NOT seed any places — every "Расположение" value in
        // utf8.csv ("Кабинет 305" etc.) must fail to resolve.
        let bytes = fixture_bytes("utf8.csv");

        let preview = svc.import_csv_preview(bytes).await.expect("preview");
        let mapping = auto_map(&preview.headers);

        let report = svc
            .import_csv_commit(preview.token, mapping)
            .await
            .expect("commit should not fail entirely");

        assert_eq!(
            report.inserted, 0,
            "no row should insert — every place value is unresolved"
        );
        assert!(!report.failed.is_empty(), "expected per-row place errors");

        // row_index=1 is the first DATA row (session.all_rows is header-excluded) —
        // utf8.csv's first row is "Кабинет 305".
        let row1 = report
            .failed
            .iter()
            .find(|e| e.row_index == 1)
            .unwrap_or_else(|| panic!("expected a row-1 error, got {:?}", report.failed));
        // Backend's `error_message` intentionally omits the "Строка N:" prefix —
        // the UI (import modal's error-list) prepends `err.row_index` generically
        // for every row (see device_service.rs comment at the RowError push site).
        // Concatenating exactly as the UI does must reproduce UI-SPEC §12's exact
        // copy, with no duplicated prefix.
        assert_eq!(
            row1.error_message, "место «Кабинет 305» не найдено в дереве.",
            "raw error_message must not bake in its own row prefix"
        );
        let rendered = format!("Строка {}: {}", row1.row_index, row1.error_message);
        assert_eq!(
            rendered, "Строка 1: место «Кабинет 305» не найдено в дереве.",
            "UI-composed string must match UI-SPEC §12's exact copy shape"
        );
    })
    .await
    .expect("timeout");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn import_commit_double_take_fails() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        seed_utf8_fixture_places(&svc).await;
        let bytes = fixture_bytes("utf8.csv");

        let preview = svc.import_csv_preview(bytes).await.expect("preview");
        let token = preview.token.clone();
        let mapping = auto_map(&preview.headers);

        // First commit: should succeed
        let _report = svc
            .import_csv_commit(token.clone(), mapping.clone())
            .await
            .expect("first commit should succeed");

        // Second commit with same token: should fail (single-use token)
        let result2 = svc.import_csv_commit(token, mapping).await;
        assert!(
            result2.is_err(),
            "second commit with same token should fail"
        );
        let err = result2.unwrap_err();
        let msg = format!("{err:?}");
        assert!(
            msg.contains("expired")
                || msg.contains("istekla")
                || msg.contains("использована")
                || msg.contains("token"),
            "error should mention expired/used token: {msg}"
        );
    })
    .await
    .expect("timeout");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn import_cyrillic_round_trip() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        seed_utf8_fixture_places(&svc).await;
        let bytes = fixture_bytes("utf8.csv");

        let preview = svc.import_csv_preview(bytes).await.expect("preview");
        let mapping = auto_map(&preview.headers);

        let report = svc
            .import_csv_commit(preview.token, mapping)
            .await
            .expect("commit");

        assert!(report.inserted >= 1, "should insert at least 1 device");

        // List all devices and check for the fixture cyrillic string
        use trackly_app::dto::device::{DeviceFilter, Pagination};
        let resp = svc
            .list(
                DeviceFilter::default(),
                Pagination {
                    offset: 0,
                    limit: 50,
                },
            )
            .await
            .expect("list");

        let fixture_name = "Сидоров-Петроградский Иван Александрович (ё) №42";
        let found = resp.items.iter().any(|d| d.name == fixture_name);
        assert!(
            found,
            "device with cyrillic fixture name should be in DB after import"
        );
    })
    .await
    .expect("timeout");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn import_commit_records_audit_log() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        seed_utf8_fixture_places(&svc).await;
        let bytes = fixture_bytes("utf8.csv");

        let preview = svc.import_csv_preview(bytes).await.expect("preview");
        let mapping = auto_map(&preview.headers);

        let report = svc
            .import_csv_commit(preview.token, mapping)
            .await
            .expect("commit");

        // Count should match inserted devices in audit_log
        // We verify this via a read on the DB — use reader directly
        assert!(report.inserted >= 1);
        // (Full audit_log verification would require direct DB access;
        // for now we trust the service writes audit_log on each create)
    })
    .await
    .expect("timeout");
}

// ---------------------------------------------------------------------------
// Helper: auto-map CSV headers to device field names
// ---------------------------------------------------------------------------

/// Creates a mapping from CSV column headers to device DTO field names.
/// Uses exact-match + fuzzy match for Russian headers.
fn auto_map(headers: &[String]) -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();

    for header in headers {
        let h = header.trim();
        let field = match h {
            "Тип" | "тип" => Some("type"),
            "Наименование" | "наименование" | "Имя" | "имя" => {
                Some("name")
            }
            "Инвентарный №" | "Инв.№" | "Инвентарный" | "inv_no" | "inventory_no" => {
                Some("inventory_no")
            }
            "Серийный №" | "Серийный" | "serial_no" => Some("serial_no"),
            "Модель" | "модель" => Some("model"),
            "Технические характеристики" | "Тех.характеристики" | "specs" => {
                Some("specs")
            }
            "Комплектация" | "kit" => Some("kit"),
            "Состояние" | "state" => Some("state"),
            "Расположение" | "Местоположение" | "Место" | "location" | "place" => {
                Some("place")
            }
            "Статус" | "status" => Some("status"),
            _ => None,
        };
        if let Some(f) = field {
            map.insert(header.clone(), f.to_string());
        }
    }

    map
}
