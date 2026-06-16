// audit_log.action for cartridge install = 'custom:install' (verified in CartridgeTransitionOp::audit_action)
//
//! Organisation settings integration test scaffold — Phase 7 Plan 01 (RED).
//!
//! Covers SET-01 (org data save/load round-trip) and SET-02 (logo BLOB upload/delete).
//!
//! Key invariants:
//!   - Only one row can exist in org_settings (CHECK id = 1)
//!   - Save updates the existing row (never inserts a second)
//!   - Logo stored as raw bytes; retrieved as Vec<u8> + mime type
//!   - OrgSettingsDto.has_logo is false when logo_blob IS NULL
//!
//! Implemented in plan 05 (OrgSettingsService::save / logo_save / logo_delete).

use std::time::Duration;

/// Verify that OrgPatch is persisted and retrieved as OrgSettingsDto.
///
/// RED: OrgSettingsService does not exist yet — todo!() causes runtime failure.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn org_settings_save_and_load_round_trip() {
    tokio::time::timeout(Duration::from_secs(30), async {
        todo!("implement in plan 05")
    })
    .await
    .expect("org_settings_save_and_load_round_trip budget")
}

/// Verify that logo BLOB upload sets has_logo=true and can be cleared.
///
/// RED: OrgSettingsService does not exist yet — todo!() causes runtime failure.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn org_logo_save_and_delete() {
    tokio::time::timeout(Duration::from_secs(30), async {
        todo!("implement in plan 05")
    })
    .await
    .expect("org_logo_save_and_delete budget")
}
