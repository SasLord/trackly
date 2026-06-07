//! Cartridge audit-log history integration tests — Plan 04-03 (GREEN phase).
//!
//! Wave 0 scaffold: tests compile with todo!() bodies.
//! CartridgeService will be wired in plan 04-03; this file is the RED gate.
//!
//! Covers (D-History-01, CART-11):
//!   - history_returns_audit_entries_for_cartridge: get_history returns all
//!     audit_log rows for the given cartridge (entity_type='cartridge').
//!   - history_is_chronological: entries are ordered by created_at_utc DESC
//!     (covered by idx_audit_log_entity from V012).

use std::time::Duration;

use trackly_infra::test_support::test_writer_and_readers;

/// Placeholder until CartridgeService is implemented in plan 04-03.
#[allow(dead_code)]
fn make_cartridge_service() -> tempfile::TempDir {
    let (_writer, _readers, _dir) = test_writer_and_readers();
    todo!("CartridgeService will be wired in plan 04-03")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn history_returns_audit_entries_for_cartridge() {
    tokio::time::timeout(Duration::from_secs(30), async {
        todo!("implement in plan 04-03 GREEN phase")
    })
    .await
    .expect("history_returns_audit_entries_for_cartridge budget")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn history_is_chronological() {
    tokio::time::timeout(Duration::from_secs(30), async {
        todo!("implement in plan 04-03 GREEN phase")
    })
    .await
    .expect("history_is_chronological budget")
}
