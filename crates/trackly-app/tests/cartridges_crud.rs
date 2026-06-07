//! Cartridge CRUD integration tests — Plan 04-03 (GREEN phase).
//!
//! Wave 0 scaffold: tests compile with todo!() bodies.
//! CartridgeService will be wired in plan 04-03; this file is the RED gate.
//!
//! Covers: create + auto-code, custom code, get-404, soft-delete, counts,
//! rejects_invalid_custom_code (empty or >32 chars → AppError::Validation).

use std::time::Duration;

use trackly_infra::test_support::test_writer_and_readers;

/// Placeholder until CartridgeService is implemented in plan 04-03.
#[allow(dead_code)]
fn make_cartridge_service() -> tempfile::TempDir {
    let (_writer, _readers, _dir) = test_writer_and_readers();
    todo!("CartridgeService will be wired in plan 04-03")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_cartridge_assigns_auto_code() {
    tokio::time::timeout(Duration::from_secs(30), async {
        todo!("implement in plan 04-03 GREEN phase")
    })
    .await
    .expect("create_cartridge_assigns_auto_code budget")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_cartridge_custom_code() {
    tokio::time::timeout(Duration::from_secs(30), async {
        todo!("implement in plan 04-03 GREEN phase")
    })
    .await
    .expect("create_cartridge_custom_code budget")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn get_returns_404_for_missing() {
    tokio::time::timeout(Duration::from_secs(30), async {
        todo!("implement in plan 04-03 GREEN phase")
    })
    .await
    .expect("get_returns_404_for_missing budget")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn soft_delete_hides_item() {
    tokio::time::timeout(Duration::from_secs(30), async {
        todo!("implement in plan 04-03 GREEN phase")
    })
    .await
    .expect("soft_delete_hides_item budget")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn counts_by_status() {
    tokio::time::timeout(Duration::from_secs(30), async {
        todo!("implement in plan 04-03 GREEN phase")
    })
    .await
    .expect("counts_by_status budget")
}

/// Verify that create with an empty code_override or one longer than 32 chars
/// returns AppError::Validation (GREEN impl in plan 04-03 Task 1: validate_create).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn rejects_invalid_custom_code() {
    tokio::time::timeout(Duration::from_secs(30), async {
        todo!("implement in plan 04-03 GREEN phase")
    })
    .await
    .expect("rejects_invalid_custom_code budget")
}
