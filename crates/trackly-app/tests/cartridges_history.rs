//! Cartridge audit-log history integration tests — Plan 04-03 (GREEN phase).
//!
//! Covers (D-History-01, CART-11):
//!   - history_returns_audit_entries_for_cartridge: get_history returns all
//!     audit_log rows for the given cartridge (entity_type='cartridge').
//!   - history_is_chronological: entries are ordered by created_at_utc DESC
//!     (covered by idx_audit_log_entity from V012).

use std::sync::Arc;
use std::time::Duration;

use trackly_infra::clock_impl::SystemClock;
use trackly_infra::test_support::test_writer_and_readers;

use trackly_app::dto::cartridge::{
    CartridgeCreateDto, CartridgeModelCreateDto, CartridgeTransitionPayload,
};
use trackly_app::services::CartridgeService;

fn make_cartridge_service() -> (CartridgeService, tempfile::TempDir) {
    let (writer, readers, dir) = test_writer_and_readers();
    let clock = Arc::new(SystemClock);
    let svc = CartridgeService::new(writer, readers, clock);
    (svc, dir)
}

async fn seed_and_create(svc: &CartridgeService) -> trackly_app::dto::cartridge::CartridgeDto {
    let model_id = svc
        .model_create(CartridgeModelCreateDto {
            brand: "HistoryBrand".into(),
            model: "HM-001".into(),
            kind_id: 1,
            color: None,
            notes: None,
            compatibility: vec![],
        })
        .await
        .expect("seed model")
        .id;

    svc.create(CartridgeCreateDto {
        model_id,
        code_override: None,
        state_id: Some(1),
        place_id: None,
        notes: None,
    })
    .await
    .expect("create cartridge")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn history_returns_audit_entries_for_cartridge() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_cartridge_service();
        let cart = seed_and_create(&svc).await;

        // Create action produces an audit entry.
        let history_after_create = svc
            .get_history(cart.id)
            .await
            .expect("history after create");
        assert!(
            !history_after_create.is_empty(),
            "history must not be empty after create"
        );

        // Transition should add more entries.
        let in_use = svc
            .transition(CartridgeTransitionPayload::Install {
                cartridge_id: cart.id,
                version: cart.version,
                date_utc: 1_700_000_000,
                given_by_name: "A".into(),
                given_to_name: "B".into(),
                place_id: None,
                printer_device_id: None,
                previous_cartridge_state_id: None,
                previous_cartridge_place_id: None,
            })
            .await
            .expect("install");

        let history_after_install = svc
            .get_history(cart.id)
            .await
            .expect("history after install");
        assert!(
            history_after_install.len() > history_after_create.len(),
            "history must grow after transition"
        );

        // Verify all entries are for this cartridge (entity_type='cartridge' filtered by id)
        // — the action field reflects what happened.
        let has_install = history_after_install
            .iter()
            .any(|e| e.action == "custom:install");
        assert!(has_install, "history must contain custom:install action");

        let _ = in_use;
    })
    .await
    .expect("history_returns_audit_entries_for_cartridge budget")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn history_is_chronological() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_cartridge_service();
        let cart = seed_and_create(&svc).await;

        // Install (adds audit entry).
        let in_use = svc
            .transition(CartridgeTransitionPayload::Install {
                cartridge_id: cart.id,
                version: cart.version,
                date_utc: 1_700_000_001,
                given_by_name: "A".into(),
                given_to_name: "B".into(),
                place_id: None,
                printer_device_id: None,
                previous_cartridge_state_id: None,
                previous_cartridge_place_id: None,
            })
            .await
            .expect("install");

        // Return to stock (adds another audit entry).
        let _returned = svc
            .transition(CartridgeTransitionPayload::ReturnToStock {
                cartridge_id: in_use.id,
                version: in_use.version,
                state_id: 3,
                place_id: None,
                notes: None,
            })
            .await
            .expect("return to stock");

        let history = svc.get_history(cart.id).await.expect("history");
        assert!(
            history.len() >= 2,
            "need at least 2 entries for chronological check"
        );

        // Entries must be ordered newest first (created_at_utc DESC).
        for window in history.windows(2) {
            assert!(
                window[0].created_at_utc >= window[1].created_at_utc,
                "history must be newest-first: {} >= {}",
                window[0].created_at_utc,
                window[1].created_at_utc
            );
        }
    })
    .await
    .expect("history_is_chronological budget")
}
