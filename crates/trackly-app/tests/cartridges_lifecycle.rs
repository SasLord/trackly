//! Cartridge lifecycle (status transitions) integration tests — Plan 04-03 (GREEN phase).
//!
//! Wave 0 scaffold: tests compile with todo!() bodies.
//! CartridgeService will be wired in plan 04-03; this file is the RED gate.
//!
//! Covers:
//!   - install: На складе → В работе (status_id 1→2)
//!   - return_to_stock: В работе → На складе (status_id 2→1, state_id = 3 Пустой by default)
//!   - to_refill: На складе → На заправке (status_id 1→3)
//!   - from_refill: На заправке → На складе (status_id 3→1, state_id = 1 Полный by default)
//!   - write_off: any → Списано (status_id 4)
//!   - all_transitions_write_audit_log: each op produces a row in audit_log

use std::time::Duration;

use trackly_infra::test_support::test_writer_and_readers;

/// Placeholder until CartridgeService is implemented in plan 04-03.
#[allow(dead_code)]
fn make_cartridge_service() -> tempfile::TempDir {
    let (_writer, _readers, _dir) = test_writer_and_readers();
    todo!("CartridgeService will be wired in plan 04-03")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn install_changes_status_to_in_use() {
    tokio::time::timeout(Duration::from_secs(30), async {
        todo!("implement in plan 04-03 GREEN phase")
    })
    .await
    .expect("install_changes_status_to_in_use budget")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn return_to_stock_sets_default_empty_state() {
    tokio::time::timeout(Duration::from_secs(30), async {
        todo!("implement in plan 04-03 GREEN phase")
    })
    .await
    .expect("return_to_stock_sets_default_empty_state budget")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn to_refill_changes_status() {
    tokio::time::timeout(Duration::from_secs(30), async {
        todo!("implement in plan 04-03 GREEN phase")
    })
    .await
    .expect("to_refill_changes_status budget")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn from_refill_sets_default_full_state() {
    tokio::time::timeout(Duration::from_secs(30), async {
        todo!("implement in plan 04-03 GREEN phase")
    })
    .await
    .expect("from_refill_sets_default_full_state budget")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn write_off_changes_status_to_written_off() {
    tokio::time::timeout(Duration::from_secs(30), async {
        todo!("implement in plan 04-03 GREEN phase")
    })
    .await
    .expect("write_off_changes_status_to_written_off budget")
}

/// Verify that every lifecycle transition writes a row to audit_log
/// with entity_type='cartridge' and the appropriate action code.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn all_transitions_write_audit_log() {
    tokio::time::timeout(Duration::from_secs(30), async {
        todo!("implement in plan 04-03 GREEN phase")
    })
    .await
    .expect("all_transitions_write_audit_log budget")
}
