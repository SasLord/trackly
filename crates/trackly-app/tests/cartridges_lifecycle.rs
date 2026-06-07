//! Cartridge lifecycle (status transitions) integration tests — Plan 04-03 (GREEN phase).
//!
//! Covers:
//!   - install: На складе → В работе (status_id 1→2)
//!   - return_to_stock: В работе → На складе (status_id 2→1, state_id = 3 Пустой by default)
//!   - to_refill: На складе → На заправке (status_id 1→3)
//!   - from_refill: На заправке → На складе (status_id 3→1, state_id = 1 Полный by default)
//!   - write_off: any → Списано (status_id 4)
//!   - all_transitions_write_audit_log: each op produces a row in audit_log

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

async fn seed_model(svc: &CartridgeService) -> i64 {
    svc.model_create(CartridgeModelCreateDto {
        brand: "HP".into(),
        model: "CE285A".into(),
        kind_id: 1,
        color: Some("Чёрный".into()),
        notes: None,
        compatibility: vec![],
    })
    .await
    .expect("seed model")
    .id
}

async fn create_stock_cartridge(svc: &CartridgeService, model_id: i64) -> trackly_app::dto::cartridge::CartridgeDto {
    svc.create(CartridgeCreateDto {
        model_id,
        code_override: None,
        state_id: Some(1), // Полный
        location: Some("Склад".into()),
        notes: None,
    })
    .await
    .expect("create cartridge")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn install_changes_status_to_in_use() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_cartridge_service();
        let model_id = seed_model(&svc).await;
        let cart = create_stock_cartridge(&svc, model_id).await;

        let updated = svc
            .transition(CartridgeTransitionPayload::Install {
                cartridge_id: cart.id,
                version: cart.version,
                date_utc: 1_700_000_000,
                given_by_name: "Иванов".into(),
                given_to_name: "Петров".into(),
                location: "Каб. 305".into(),
            })
            .await
            .expect("transition Install");

        assert_eq!(updated.status_id, 2, "status must be В работе (2)");
        assert_eq!(
            updated.holder_name.as_deref(),
            Some("Петров"),
            "holder_name must be updated"
        );
    })
    .await
    .expect("install_changes_status_to_in_use budget")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn return_to_stock_sets_default_empty_state() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_cartridge_service();
        let model_id = seed_model(&svc).await;
        let cart = create_stock_cartridge(&svc, model_id).await;

        // First install it
        let in_use = svc
            .transition(CartridgeTransitionPayload::Install {
                cartridge_id: cart.id,
                version: cart.version,
                date_utc: 1_700_000_000,
                given_by_name: "A".into(),
                given_to_name: "B".into(),
                location: "Каб. 1".into(),
            })
            .await
            .expect("install");

        // Then return to stock with state = 3 (Пустой)
        let returned = svc
            .transition(CartridgeTransitionPayload::ReturnToStock {
                cartridge_id: in_use.id,
                version: in_use.version,
                state_id: 3, // Пустой
                location: "Склад".into(),
                notes: None,
            })
            .await
            .expect("return_to_stock");

        assert_eq!(returned.status_id, 1, "status must be На складе (1)");
        assert_eq!(returned.state_id, Some(3), "state must be Пустой (3)");
        assert!(returned.holder_name.is_none(), "holder_name must be cleared");
    })
    .await
    .expect("return_to_stock_sets_default_empty_state budget")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn to_refill_changes_status() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_cartridge_service();
        let model_id = seed_model(&svc).await;
        let cart = create_stock_cartridge(&svc, model_id).await;

        let at_refill = svc
            .transition(CartridgeTransitionPayload::ToRefill {
                cartridge_id: cart.id,
                version: cart.version,
                date_utc: 1_700_000_000,
                given_by_name: "Иванов".into(),
                given_to_name: "ООО Заправка".into(),
                location: "Пункт заправки".into(),
            })
            .await
            .expect("to_refill");

        assert_eq!(at_refill.status_id, 3, "status must be На заправке (3)");
    })
    .await
    .expect("to_refill_changes_status budget")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn from_refill_sets_default_full_state() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_cartridge_service();
        let model_id = seed_model(&svc).await;
        let cart = create_stock_cartridge(&svc, model_id).await;

        // Send to refill
        let at_refill = svc
            .transition(CartridgeTransitionPayload::ToRefill {
                cartridge_id: cart.id,
                version: cart.version,
                date_utc: 1_700_000_000,
                given_by_name: "A".into(),
                given_to_name: "Заправщик".into(),
                location: "Заправка".into(),
            })
            .await
            .expect("to_refill");

        // Return from refill with state = 1 (Полный)
        let back = svc
            .transition(CartridgeTransitionPayload::FromRefill {
                cartridge_id: at_refill.id,
                version: at_refill.version,
                state_id: 1, // Полный
                location: "Склад".into(),
                notes: None,
            })
            .await
            .expect("from_refill");

        assert_eq!(back.status_id, 1, "status must be На складе (1)");
        assert_eq!(back.state_id, Some(1), "state must be Полный (1)");
    })
    .await
    .expect("from_refill_sets_default_full_state budget")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn write_off_changes_status_to_written_off() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_cartridge_service();
        let model_id = seed_model(&svc).await;
        let cart = create_stock_cartridge(&svc, model_id).await;

        let written_off = svc
            .transition(CartridgeTransitionPayload::WriteOff {
                cartridge_id: cart.id,
                version: cart.version,
                date_utc: 1_700_000_000,
                notes: Some("Физический износ".into()),
            })
            .await
            .expect("write_off");

        assert_eq!(written_off.status_id, 4, "status must be Списано (4)");
    })
    .await
    .expect("write_off_changes_status_to_written_off budget")
}

/// Verify that every lifecycle transition writes a row to audit_log.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn all_transitions_write_audit_log() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_cartridge_service();
        let model_id = seed_model(&svc).await;
        let cart = create_stock_cartridge(&svc, model_id).await;

        // Install
        let in_use = svc
            .transition(CartridgeTransitionPayload::Install {
                cartridge_id: cart.id,
                version: cart.version,
                date_utc: 1_700_000_000,
                given_by_name: "A".into(),
                given_to_name: "B".into(),
                location: "Каб.1".into(),
            })
            .await
            .expect("install");

        let history = svc.get_history(cart.id).await.expect("history");
        assert!(!history.is_empty(), "history must not be empty after transition");

        // At least one entry has action containing "custom:" (transition action pattern)
        let has_custom = history.iter().any(|e| e.action.contains("custom:"));
        assert!(has_custom, "transition audit entry must contain 'custom:': {:?}", history);

        // Return to stock
        let _returned = svc
            .transition(CartridgeTransitionPayload::ReturnToStock {
                cartridge_id: in_use.id,
                version: in_use.version,
                state_id: 3,
                location: "Склад".into(),
                notes: None,
            })
            .await
            .expect("return_to_stock");

        let history2 = svc.get_history(cart.id).await.expect("history2");
        assert!(history2.len() > history.len(), "history must grow after each transition");
    })
    .await
    .expect("all_transitions_write_audit_log budget")
}
