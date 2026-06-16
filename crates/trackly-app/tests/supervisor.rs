// audit_log.action for cartridge install = 'custom:install' (verified in CartridgeTransitionOp::audit_action)
//
//! Supervisor (scheduled task runner) integration test scaffold — Phase 7 Plan 01 (RED).
//!
//! Covers SET-06 (auto-backup schedule):
//!   - Catch-up semantics: if a scheduled backup was overdue at startup, it fires immediately
//!   - Tasks run in the background and do not block the Tauri event loop
//!   - Schedule intervals are enforced (a task due in the future does NOT fire early)
//!
//! Implemented in plan 06 (BackupSupervisor / ScheduledTaskRunner).

use std::time::Duration;

/// Verify that an overdue scheduled task fires immediately on supervisor startup.
///
/// RED: BackupSupervisor does not exist yet — todo!() causes runtime failure.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn supervisor_overdue_task_fires_on_startup() {
    tokio::time::timeout(Duration::from_secs(30), async {
        todo!("implement in plan 06")
    })
    .await
    .expect("supervisor_overdue_task_fires_on_startup budget")
}

/// Verify that a future task does not fire before its scheduled time.
///
/// RED: BackupSupervisor does not exist yet — todo!() causes runtime failure.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn supervisor_future_task_does_not_fire_early() {
    tokio::time::timeout(Duration::from_secs(30), async {
        todo!("implement in plan 06")
    })
    .await
    .expect("supervisor_future_task_does_not_fire_early budget")
}
