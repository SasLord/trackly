//! Cartridge FTS + LIKE search integration tests — Plan 04-03 (GREEN phase).
//!
//! Wave 0 scaffold: tests compile with todo!() bodies.
//! CartridgeService will be wired in plan 04-03; this file is the RED gate.
//!
//! Covers:
//!   - search_by_code: LIKE match on C-NNNNNN code
//!   - search_by_model_brand: FTS/LIKE on cartridge_models.brand + model
//!   - search_by_location: LIKE on cartridges.location
//!   - empty_query_returns_all: empty / whitespace-only query falls back to list
//!
//! FTS search path: FTS MATCH UNION LIKE CTE query (D-Search-01).
//! cartridges_fts fields: code, location, holder_name (V012 + V016 triggers).

use std::time::Duration;

use trackly_infra::test_support::test_writer_and_readers;

/// Placeholder until CartridgeService is implemented in plan 04-03.
#[allow(dead_code)]
fn make_cartridge_service() -> tempfile::TempDir {
    let (_writer, _readers, _dir) = test_writer_and_readers();
    todo!("CartridgeService will be wired in plan 04-03")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn search_by_code() {
    tokio::time::timeout(Duration::from_secs(30), async {
        todo!("implement in plan 04-03 GREEN phase")
    })
    .await
    .expect("search_by_code budget")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn search_by_model_brand() {
    tokio::time::timeout(Duration::from_secs(30), async {
        todo!("implement in plan 04-03 GREEN phase")
    })
    .await
    .expect("search_by_model_brand budget")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn search_by_location() {
    tokio::time::timeout(Duration::from_secs(30), async {
        todo!("implement in plan 04-03 GREEN phase")
    })
    .await
    .expect("search_by_location budget")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn empty_query_returns_all() {
    tokio::time::timeout(Duration::from_secs(30), async {
        todo!("implement in plan 04-03 GREEN phase")
    })
    .await
    .expect("empty_query_returns_all budget")
}
