//! Интеграционные тесты автодополнения устройств (Plan 04, Task 1).
//!
//! Тесты покрывают:
//! - DISTINCT возврат значений
//! - Контекстный фильтр по Наименованию (DEV-09)
//! - Ограничение 30 результатов
//! - Сортировка ASC
//! - Whitelist-защита от SQL injection через field (T-02-04-02)

use std::sync::Arc;
use std::time::Duration;

use trackly_app::dto::device::DeviceNew;
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

fn minimal_new(name: &str) -> DeviceNew {
    DeviceNew {
        type_id: 1,
        name: name.to_string(),
        inventory_no: None,
        serial_no: None,
        model: None,
        specs: None,
        kit: None,
        state: None,
        location_id: None,
        status_id: 1,
    }
}

// ---------------------------------------------------------------------------
// autocomplete_name_returns_distinct
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn autocomplete_name_returns_distinct() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();

        // 3 devices named "Принтер", 2 named "Сканер".
        for _ in 0..3 {
            svc.create(minimal_new("Принтер")).await.expect("create Принтер");
        }
        for _ in 0..2 {
            svc.create(minimal_new("Сканер")).await.expect("create Сканер");
        }

        let results = svc
            .autocomplete("name".to_string(), "При".to_string(), None)
            .await
            .expect("autocomplete name");

        // DISTINCT: should return ["Принтер"] only once.
        assert_eq!(results.len(), 1, "DISTINCT — должен вернуть 1 значение 'Принтер', получили {results:?}");
        assert_eq!(results[0], "Принтер");
    })
    .await
    .expect("autocomplete_name_returns_distinct exceeded 30s");
}

// ---------------------------------------------------------------------------
// autocomplete_model_with_context
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn autocomplete_model_with_context() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();

        // Device A: name="Принтер HP", model="LaserJet 1020"
        let mut a = minimal_new("Принтер HP");
        a.model = Some("LaserJet 1020".to_string());
        svc.create(a).await.expect("create A");

        // Device B: name="Принтер HP", model="LaserJet 2055"
        let mut b = minimal_new("Принтер HP");
        b.model = Some("LaserJet 2055".to_string());
        svc.create(b).await.expect("create B");

        // Device C: name="Сканер", model="ScanJet G3110"
        let mut c = minimal_new("Сканер");
        c.model = Some("ScanJet G3110".to_string());
        svc.create(c).await.expect("create C");

        // Autocomplete model with ctx_name="Принтер HP" — should return only LaserJet models.
        let results = svc
            .autocomplete(
                "model".to_string(),
                "Las".to_string(),
                Some("Принтер HP".to_string()),
            )
            .await
            .expect("autocomplete model contextual");

        assert_eq!(results.len(), 2, "contextual: должны вернуться 2 LaserJet модели, получили {results:?}");
        assert!(results.contains(&"LaserJet 1020".to_string()));
        assert!(results.contains(&"LaserJet 2055".to_string()));
        assert!(
            !results.contains(&"ScanJet G3110".to_string()),
            "ScanJet не должен попадать в contextual autocomplete по Принтер HP"
        );
    })
    .await
    .expect("autocomplete_model_with_context exceeded 30s");
}

// ---------------------------------------------------------------------------
// autocomplete_limit_30
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn autocomplete_limit_30() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let (svc, _dir) = make_service();

        // Create 35 devices with unique names.
        for i in 0..35u32 {
            svc.create(minimal_new(&format!("Устройство {i:03}")))
                .await
                .expect("create");
        }

        let results = svc
            .autocomplete("name".to_string(), "".to_string(), None)
            .await
            .expect("autocomplete name all");

        assert_eq!(results.len(), 30, "лимит автодополнения — 30 результатов, получили {}", results.len());
    })
    .await
    .expect("autocomplete_limit_30 exceeded 60s");
}

// ---------------------------------------------------------------------------
// autocomplete_sorted_asc
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn autocomplete_sorted_asc() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();

        svc.create(minimal_new("Сканер")).await.expect("create Сканер");
        svc.create(minimal_new("Мышь")).await.expect("create Мышь");
        svc.create(minimal_new("Клавиатура")).await.expect("create Клавиатура");

        let results = svc
            .autocomplete("name".to_string(), "".to_string(), None)
            .await
            .expect("autocomplete sorted");

        // Must be in ascending order.
        let sorted = {
            let mut v = results.clone();
            v.sort();
            v
        };
        assert_eq!(results, sorted, "результаты автодополнения должны быть отсортированы ASC");
    })
    .await
    .expect("autocomplete_sorted_asc exceeded 30s");
}

// ---------------------------------------------------------------------------
// autocomplete_invalid_field_rejected (T-02-04-02)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn autocomplete_invalid_field_rejected() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();

        // status_id is not a whitelisted autocomplete field.
        let err = svc
            .autocomplete("status_id".to_string(), "".to_string(), None)
            .await
            .expect_err("должно вернуть ошибку для неразрешённого поля 'status_id'");

        match err {
            trackly_core::error::AppError::Validation { field, message } => {
                assert_eq!(field, "field");
                assert!(
                    message.contains("поддерживаемые") || message.contains("Поддерживаемые") || message.contains("Неподдерживаемое"),
                    "message должен указывать на неподдерживаемое поле, получили: {message}"
                );
            }
            other => panic!("ожидали AppError::Validation, получили {other:?}"),
        }
    })
    .await
    .expect("autocomplete_invalid_field_rejected exceeded 30s");
}

// ---------------------------------------------------------------------------
// autocomplete_no_context_returns_all_matching
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn autocomplete_no_context_returns_all_matching() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();

        let mut a = minimal_new("Принтер HP");
        a.model = Some("LaserJet 1020".to_string());
        svc.create(a).await.expect("create A");

        let mut b = minimal_new("Сканер Canon");
        b.model = Some("LaserJet Canon".to_string()); // same prefix, different device
        svc.create(b).await.expect("create B");

        // Without context, should return both LaserJet models.
        let results = svc
            .autocomplete("model".to_string(), "Las".to_string(), None)
            .await
            .expect("autocomplete no context");

        assert_eq!(results.len(), 2, "без контекста должны вернуться все совпадения");
    })
    .await
    .expect("autocomplete_no_context_returns_all_matching exceeded 30s");
}

// ---------------------------------------------------------------------------
// autocomplete_specs_field
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn autocomplete_specs_field() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();

        let mut a = minimal_new("Ноутбук");
        a.specs = Some("Intel Core i7, 16GB RAM".to_string());
        svc.create(a).await.expect("create");

        let results = svc
            .autocomplete("specs".to_string(), "Intel".to_string(), None)
            .await
            .expect("autocomplete specs");

        assert_eq!(results.len(), 1, "должно найти по specs (notes в БД)");
        assert!(results[0].contains("Intel"));
    })
    .await
    .expect("autocomplete_specs_field exceeded 30s");
}
