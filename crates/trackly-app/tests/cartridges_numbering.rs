//! Cartridge auto-code numbering integration tests — Plan 04-03 (GREEN phase).
//!
//! Wave 0 scaffold: tests compile with todo!() bodies.
//! CartridgeService will be wired in plan 04-03; this file is the RED gate.
//!
//! Covers:
//!   - 50 concurrent creates produce 50 unique codes in C-NNNNNN format.
//!   - Counter is never lost on UNIQUE collision: retry loop increments again.

use std::time::Duration;

use trackly_infra::test_support::test_writer_and_readers;

/// Placeholder until CartridgeService is implemented in plan 04-03.
#[allow(dead_code)]
fn make_cartridge_service() -> tempfile::TempDir {
    let (_writer, _readers, _dir) = test_writer_and_readers();
    todo!("CartridgeService will be wired in plan 04-03")
}

/// Spawn 50 concurrent writer.execute closures, each doing:
///   BEGIN IMMEDIATE → increment cartridge_seq → INSERT cartridges
/// Verify: all 50 codes are unique and have format C-NNNNNN.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_50_unique_codes() {
    tokio::time::timeout(Duration::from_secs(30), async {
        todo!("implement in plan 04-03 GREEN phase")
    })
    .await
    .expect("concurrent_50_unique_codes budget")
}

/// Verify that when a code would collide, the counter is incremented again
/// and the counter value is never lost (no gap that loses the increment).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn collision_retry_does_not_lose_counter() {
    tokio::time::timeout(Duration::from_secs(30), async {
        todo!("implement in plan 04-03 GREEN phase")
    })
    .await
    .expect("collision_retry_does_not_lose_counter budget")
}
