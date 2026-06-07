//! Cartridge CRUD integration tests — Plan 04-03 (GREEN phase).
//!
//! Covers: create + auto-code, custom code, get-404, soft-delete, counts,
//! rejects_invalid_custom_code (empty or >32 chars or ctrl chars → AppError::Validation).

use std::sync::Arc;
use std::time::Duration;

use trackly_core::error::AppError;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::test_support::test_writer_and_readers;

use trackly_app::dto::cartridge::CartridgeCreateDto;
use trackly_app::services::CartridgeService;

/// Set up a fresh CartridgeService backed by an in-memory migrated DB.
fn make_cartridge_service() -> (CartridgeService, tempfile::TempDir) {
    let (writer, readers, dir) = test_writer_and_readers();
    let clock = Arc::new(SystemClock);
    let svc = CartridgeService::new(writer, readers, clock);
    (svc, dir)
}

/// Seed a cartridge_model and return its id.
async fn seed_model(svc: &CartridgeService) -> i64 {
    let model = svc
        .model_create(trackly_app::dto::cartridge::CartridgeModelCreateDto {
            brand: "Pantum".into(),
            model: "TL-5120X".into(),
            kind_id: 1,
            color: Some("Чёрный".into()),
            notes: None,
            compatibility: vec![],
        })
        .await
        .expect("seed_model");
    model.id
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_cartridge_assigns_auto_code() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_cartridge_service();
        let model_id = seed_model(&svc).await;

        let dto = svc
            .create(CartridgeCreateDto {
                model_id,
                code_override: None,
                state_id: Some(1),
                location: Some("Склад".into()),
                notes: None,
            })
            .await
            .expect("create auto");

        // Code must start with "C-" and be unique.
        assert!(
            dto.code.starts_with("C-"),
            "auto-code must start with C-: {}",
            dto.code
        );
        assert_eq!(dto.model_id, model_id);
        assert_eq!(dto.status_id, 1); // На складе
    })
    .await
    .expect("create_cartridge_assigns_auto_code budget")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_cartridge_custom_code() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_cartridge_service();
        let model_id = seed_model(&svc).await;

        let dto = svc
            .create(CartridgeCreateDto {
                model_id,
                code_override: Some("BARCODE-42".into()),
                state_id: None,
                location: None,
                notes: None,
            })
            .await
            .expect("create custom");

        assert_eq!(dto.code, "BARCODE-42");
    })
    .await
    .expect("create_cartridge_custom_code budget")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn get_returns_404_for_missing() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_cartridge_service();
        let err = svc.get(99999).await.expect_err("should be NotFound");
        assert!(
            matches!(err, AppError::NotFound { .. }),
            "expected NotFound, got {:?}",
            err
        );
    })
    .await
    .expect("get_returns_404_for_missing budget")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn soft_delete_hides_item() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_cartridge_service();
        let model_id = seed_model(&svc).await;

        let dto = svc
            .create(CartridgeCreateDto {
                model_id,
                code_override: None,
                state_id: None,
                location: None,
                notes: None,
            })
            .await
            .expect("create");

        svc.delete(dto.id, dto.version).await.expect("delete");

        // After soft-delete, get should return NotFound.
        let err = svc.get(dto.id).await.expect_err("should be hidden");
        assert!(
            matches!(err, AppError::NotFound { .. }),
            "expected NotFound after delete, got {:?}",
            err
        );
    })
    .await
    .expect("soft_delete_hides_item budget")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn counts_by_status() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_cartridge_service();
        let model_id = seed_model(&svc).await;

        // Create 2 cartridges on-stock.
        for _ in 0..2 {
            svc.create(CartridgeCreateDto {
                model_id,
                code_override: None,
                state_id: Some(1),
                location: None,
                notes: None,
            })
            .await
            .expect("create");
        }

        let counts = svc.status_counts().await.expect("counts");
        assert_eq!(counts.all, 2);
        assert_eq!(counts.in_stock, 2);
        assert_eq!(counts.in_use, 0);
    })
    .await
    .expect("counts_by_status budget")
}

/// Verify that create with an empty code_override, one longer than 32 chars,
/// or one containing a control character returns AppError::Validation.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn rejects_invalid_custom_code() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_cartridge_service();
        let model_id = seed_model(&svc).await;

        // (a) empty string
        let result = svc
            .create(CartridgeCreateDto {
                model_id,
                code_override: Some("".into()),
                ..Default::default()
            })
            .await;
        assert!(
            matches!(result, Err(AppError::Validation { ref field, .. }) if field == "code_override"),
            "empty string must return Validation(code_override), got: {:?}",
            result
        );

        // (b) string longer than 32 chars (33 x's)
        let result = svc
            .create(CartridgeCreateDto {
                model_id,
                code_override: Some("x".repeat(33)),
                ..Default::default()
            })
            .await;
        assert!(
            matches!(result, Err(AppError::Validation { ref field, .. }) if field == "code_override"),
            ">32 chars must return Validation(code_override), got: {:?}",
            result
        );

        // (c) string with a control character (tab = U+0009 < U+0020)
        let result = svc
            .create(CartridgeCreateDto {
                model_id,
                code_override: Some("C\x09ode".into()),
                ..Default::default()
            })
            .await;
        assert!(
            matches!(result, Err(AppError::Validation { ref field, .. }) if field == "code_override"),
            "ctrl char must return Validation(code_override), got: {:?}",
            result
        );
    })
    .await
    .expect("rejects_invalid_custom_code budget")
}
