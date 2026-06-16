// audit_log.action for cartridge install = 'custom:install' (verified in CartridgeTransitionOp::audit_action)
//
//! Backup service integration test scaffold — Phase 7 Plan 01 (RED).
//!
//! Covers SET-05 (manual backup) and SET-05 auto-backup:
//!   - Manual backup copies DB file to configured backup_folder
//!   - SQLite integrity_check is run on the copy before declaring success
//!   - UNC paths (\\server\share) are rejected with a validation error
//!   - Retention policy prunes oldest copies when count exceeds retention limit
//!
//! Implemented in plan 06 (BackupService::backup_now / apply_retention).

use std::time::Duration;

/// Verify that a manual backup creates a file in backup_folder and passes integrity_check.
///
/// RED: BackupService does not exist yet — todo!() causes runtime failure.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn backup_manual_creates_file_and_passes_integrity_check() {
    tokio::time::timeout(Duration::from_secs(30), async {
        todo!("implement in plan 06")
    })
    .await
    .expect("backup_manual_creates_file_and_passes_integrity_check budget")
}

/// Verify that UNC backup_folder paths are rejected.
///
/// Per PATTERNS.md: UNC path = starts with r"\\" (two backslashes).
/// Decision recorded: Plan 01-02 — "UNC rejection via simple starts_with(r"\\\\") prefix check".
///
/// RED: BackupService does not exist yet — todo!() causes runtime failure.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn backup_unc_path_rejected() {
    tokio::time::timeout(Duration::from_secs(30), async {
        todo!("implement in plan 06")
    })
    .await
    .expect("backup_unc_path_rejected budget")
}
