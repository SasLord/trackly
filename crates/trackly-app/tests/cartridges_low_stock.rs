//! Cartridge low-stock query integration tests — Plan 04-03 (GREEN phase).
//!
//! Wave 0 scaffold: tests compile with todo!() bodies.
//! CartridgeService will be wired in plan 04-03; this file is the RED gate.
//!
//! Covers (D-LowStock-01, CART-12):
//!   - low_stock_returns_model_below_threshold: models with fewer than threshold
//!     fully-charged cartridges on-stock appear in the low_stock result.
//!   - full_stock_not_in_low_stock: models with enough stock are excluded.
//!   - threshold_read_from_app_settings: the threshold is read from
//!     app_settings WHERE key='low_stock_threshold' (seeded to '2' in V016).

use std::time::Duration;

use trackly_infra::test_support::test_writer_and_readers;

/// Placeholder until CartridgeService is implemented in plan 04-03.
#[allow(dead_code)]
fn make_cartridge_service() -> tempfile::TempDir {
    let (_writer, _readers, _dir) = test_writer_and_readers();
    todo!("CartridgeService will be wired in plan 04-03")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn low_stock_returns_model_below_threshold() {
    tokio::time::timeout(Duration::from_secs(30), async {
        todo!("implement in plan 04-03 GREEN phase")
    })
    .await
    .expect("low_stock_returns_model_below_threshold budget")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn full_stock_not_in_low_stock() {
    tokio::time::timeout(Duration::from_secs(30), async {
        todo!("implement in plan 04-03 GREEN phase")
    })
    .await
    .expect("full_stock_not_in_low_stock budget")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn threshold_read_from_app_settings() {
    tokio::time::timeout(Duration::from_secs(30), async {
        todo!("implement in plan 04-03 GREEN phase")
    })
    .await
    .expect("threshold_read_from_app_settings budget")
}
