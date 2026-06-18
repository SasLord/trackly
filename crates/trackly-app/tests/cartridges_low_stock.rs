//! Cartridge low-stock query integration tests — Plan 04-03 (GREEN phase).
//!
//! Covers (D-LowStock-01, CART-12):
//!   - low_stock_returns_model_below_threshold: models with fewer than threshold
//!     fully-charged cartridges on-stock appear in the low_stock result.
//!   - full_stock_not_in_low_stock: models with enough stock are excluded.
//!   - threshold_read_from_app_settings: the threshold is read from
//!     app_settings WHERE key='low_stock_threshold' (seeded to '2' in V016).

use std::sync::Arc;
use std::time::Duration;

use trackly_infra::clock_impl::SystemClock;
use trackly_infra::test_support::test_writer_and_readers;

use trackly_app::dto::cartridge::{CartridgeCreateDto, CartridgeModelCreateDto};
use trackly_app::services::CartridgeService;

fn make_cartridge_service() -> (CartridgeService, tempfile::TempDir) {
    let (writer, readers, dir) = test_writer_and_readers();
    let clock = Arc::new(SystemClock);
    let svc = CartridgeService::new(writer, readers, clock);
    (svc, dir)
}

async fn seed_model(svc: &CartridgeService, brand: &str) -> i64 {
    svc.model_create(CartridgeModelCreateDto {
        brand: brand.into(),
        model: "TestModel".into(),
        kind_id: 1,
        color: None,
        notes: None,
        compatibility: vec![],
    })
    .await
    .expect("seed model")
    .id
}

/// Create `n` in-stock, fully-charged (state=1) cartridges for a model.
async fn create_full_stock(svc: &CartridgeService, model_id: i64, n: usize) {
    for _ in 0..n {
        svc.create(CartridgeCreateDto {
            model_id,
            code_override: None,
            state_id: Some(1), // Полный
            location: Some("Склад".into()),
            notes: None,
        })
        .await
        .expect("create full stock");
    }
}

/// The default threshold from app_settings is 2.
/// A model with 1 full-stock cartridge is below the threshold.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn low_stock_returns_model_below_threshold() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_cartridge_service();
        let model_id = seed_model(&svc, "LowStockBrand").await;

        // 1 full cartridge — below threshold of 2.
        create_full_stock(&svc, model_id, 1).await;

        let items = svc.low_stock().await.expect("low_stock");
        assert!(
            items.iter().any(|i| i.model_id == model_id),
            "model with 1 cartridge must be in low_stock result"
        );
    })
    .await
    .expect("low_stock_returns_model_below_threshold budget")
}

/// A model with 2 or more full-stock cartridges must NOT appear in low_stock.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn full_stock_not_in_low_stock() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_cartridge_service();
        let model_id = seed_model(&svc, "FullStockBrand").await;

        // 2 full cartridges — exactly at threshold, so NOT below.
        create_full_stock(&svc, model_id, 2).await;

        let items = svc.low_stock().await.expect("low_stock");
        assert!(
            !items.iter().any(|i| i.model_id == model_id),
            "model with 2 cartridges (at threshold) must NOT be in low_stock result"
        );
    })
    .await
    .expect("full_stock_not_in_low_stock budget")
}

/// Verify the threshold comes from app_settings and affects the result.
/// Default is 2 (seeded in V016). Create 1 cartridge and verify it appears.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn threshold_read_from_app_settings() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_cartridge_service();
        let model_id = seed_model(&svc, "ThresholdBrand").await;

        // 1 cartridge — below threshold=2.
        let cart = svc
            .create(CartridgeCreateDto {
                model_id,
                code_override: None,
                state_id: Some(1), // Полный
                location: None,
                notes: None,
            })
            .await
            .expect("create");

        let items = svc.low_stock().await.expect("low_stock");
        let item = items.iter().find(|i| i.model_id == model_id);
        assert!(item.is_some(), "model must be in low_stock");
        let item = item.unwrap();
        assert_eq!(item.count, 1, "count must be 1");
        assert_eq!(
            item.threshold, 2,
            "threshold must be 2 (from app_settings V016 seed)"
        );

        let _ = cart;
    })
    .await
    .expect("threshold_read_from_app_settings budget")
}
