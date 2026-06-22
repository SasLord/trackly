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
    CartridgeCreateDto, CartridgeFilter, CartridgeModelCreateDto, CartridgeTransitionPayload,
    Pagination,
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

async fn create_stock_cartridge(
    svc: &CartridgeService,
    model_id: i64,
) -> trackly_app::dto::cartridge::CartridgeDto {
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

/// Same as `create_stock_cartridge`, but with an explicit `state_id`
/// (Plan 12-01: installable_only filter tests need 1/2/3 charge states).
async fn create_stock_cartridge_with_state(
    svc: &CartridgeService,
    model_id: i64,
    state_id: i64,
) -> trackly_app::dto::cartridge::CartridgeDto {
    svc.create(CartridgeCreateDto {
        model_id,
        code_override: None,
        state_id: Some(state_id),
        location: Some("Склад".into()),
        notes: None,
    })
    .await
    .expect("create cartridge with state")
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
        assert!(
            returned.holder_name.is_none(),
            "holder_name must be cleared"
        );
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
        assert!(
            !history.is_empty(),
            "history must not be empty after transition"
        );

        // At least one entry has action containing "custom:" (transition action pattern)
        let has_custom = history.iter().any(|e| e.action.contains("custom:"));
        assert!(
            has_custom,
            "transition audit entry must contain 'custom:': {:?}",
            history
        );

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
        assert!(
            history2.len() > history.len(),
            "history must grow after each transition"
        );
    })
    .await
    .expect("all_transitions_write_audit_log budget")
}

/// Plan 12-01 (D-01): `installable_only: true` keeps only state_id IN (1, 2)
/// — Полный/Частичный — on stock cartridges, excluding state_id 3 (Пустой).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn installable_filters_to_full_and_partial_charge() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_cartridge_service();
        let model_id = seed_model(&svc).await;
        create_stock_cartridge_with_state(&svc, model_id, 1).await; // Полный
        create_stock_cartridge_with_state(&svc, model_id, 2).await; // Частичный
        create_stock_cartridge_with_state(&svc, model_id, 3).await; // Пустой

        let result = svc
            .list(
                CartridgeFilter {
                    status_id: Some(1),
                    installable_only: true,
                    ..Default::default()
                },
                Pagination::default(),
            )
            .await
            .expect("list installable_only=true");

        assert_eq!(
            result.items.len(),
            2,
            "only state_id IN (1, 2) cartridges must be returned, got: {:?}",
            result.items
        );
        assert!(
            result
                .items
                .iter()
                .all(|c| c.state_id == Some(1) || c.state_id == Some(2)),
            "every returned cartridge must have state_id 1 or 2: {:?}",
            result.items
        );
    })
    .await
    .expect("installable_filters_to_full_and_partial_charge budget")
}

/// Plan 12-01 (D-01): `installable_only: false` (default) returns all charge states.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn installable_only_false_returns_all() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_cartridge_service();
        let model_id = seed_model(&svc).await;
        create_stock_cartridge_with_state(&svc, model_id, 1).await;
        create_stock_cartridge_with_state(&svc, model_id, 2).await;
        create_stock_cartridge_with_state(&svc, model_id, 3).await;

        let result = svc
            .list(
                CartridgeFilter {
                    status_id: Some(1),
                    installable_only: false,
                    ..Default::default()
                },
                Pagination::default(),
            )
            .await
            .expect("list installable_only=false");

        assert_eq!(
            result.items.len(),
            3,
            "installable_only=false must return all charge states: {:?}",
            result.items
        );
    })
    .await
    .expect("installable_only_false_returns_all budget")
}

/// Plan 12-01 (D-01/DISC-01): `installable_only: true` combined with `model_id`
/// narrows to the requested model; `model_id: None` must not filter at all.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn installable_only_respects_model_filter() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_cartridge_service();
        let model_a = seed_model(&svc).await;
        let model_b = svc
            .model_create(CartridgeModelCreateDto {
                brand: "Kyocera".into(),
                model: "TK-1170".into(),
                kind_id: 1,
                color: Some("Чёрный".into()),
                notes: None,
                compatibility: vec![],
            })
            .await
            .expect("seed model B")
            .id;

        create_stock_cartridge_with_state(&svc, model_a, 1).await;
        create_stock_cartridge_with_state(&svc, model_b, 1).await;

        // With model_id set: only the matching model's cartridge comes back.
        let scoped = svc
            .list(
                CartridgeFilter {
                    status_id: Some(1),
                    installable_only: true,
                    model_id: Some(model_a),
                    ..Default::default()
                },
                Pagination::default(),
            )
            .await
            .expect("list installable_only + model_id");

        assert_eq!(
            scoped.items.len(),
            1,
            "model_id must narrow installable_only results: {:?}",
            scoped.items
        );
        assert_eq!(scoped.items[0].model_id, model_a);

        // With model_id: None — installable_only alone must not be scoped by model.
        let unscoped = svc
            .list(
                CartridgeFilter {
                    status_id: Some(1),
                    installable_only: true,
                    model_id: None,
                    ..Default::default()
                },
                Pagination::default(),
            )
            .await
            .expect("list installable_only, model_id=None");

        assert_eq!(
            unscoped.items.len(),
            2,
            "model_id: None must not filter by model: {:?}",
            unscoped.items
        );
    })
    .await
    .expect("installable_only_respects_model_filter budget")
}

/// Plan 12-01 (DISC-02): empty result set is Ok, not an error.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn installable_only_empty_result_is_ok_not_error() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_cartridge_service();
        let model_id = seed_model(&svc).await;
        // Only an empty-charge cartridge on stock — installable_only must exclude it.
        create_stock_cartridge_with_state(&svc, model_id, 3).await;

        let result = svc
            .list(
                CartridgeFilter {
                    status_id: Some(1),
                    installable_only: true,
                    ..Default::default()
                },
                Pagination::default(),
            )
            .await
            .expect("list must be Ok even with empty result");

        assert_eq!(result.items.len(), 0, "no installable cartridges expected");
        assert_eq!(result.total, 0, "total must be 0, not an error");
    })
    .await
    .expect("installable_only_empty_result_is_ok_not_error budget")
}
