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

use rusqlite::params;
use serde_json::Value;
use trackly_core::auth::{Identity, Role};
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::db::writer_worker::WriterHandle;
use trackly_infra::error_conversions::map_rusqlite;
use trackly_infra::test_support::test_writer_and_readers;

use trackly_app::dto::cartridge::{
    CartridgeCreateDto, CartridgeFilter, CartridgeModelCreateDto, CartridgeTransitionPayload,
    Pagination,
};
use trackly_app::services::CartridgeService;

/// `Identity::trusted_admin()` — unlocked-desktop identity (D-Desktop-01),
/// `user_id: None`. Used for pre-existing call sites that don't assert on
/// `audit_log.user_id` (Plan 40-04 caller-threading, mirrors device_service's
/// `admin_caller()` precedent from Plan 40-03).
fn admin_caller() -> Identity {
    Identity::trusted_admin()
}

/// Seed a real `users` row (FK target for `audit_log.user_id`) and return its
/// id. Invented name — privacy gate (CLAUDE.md hard constraint, no real ФИО).
async fn seed_manager_user(writer: &WriterHandle) -> i64 {
    let now = 1_700_000_000_i64;
    writer
        .execute(move |conn| {
            let tx = conn.transaction().map_err(map_rusqlite)?;
            tx.execute(
                "INSERT INTO users \
                 (login, full_name, password_hash, role, ad_user, is_active, \
                  created_at_utc, updated_at_utc, version) \
                 VALUES ('kuznetsov.kk', 'Кузнецов К.К.', NULL, 'manager', 0, 1, ?1, ?1, 1)",
                params![now],
            )
            .map_err(map_rusqlite)?;
            let user_id = tx.last_insert_rowid();
            tx.commit().map_err(map_rusqlite)?;
            Ok(user_id)
        })
        .await
        .expect("seed manager user")
}

fn make_cartridge_service() -> (CartridgeService, tempfile::TempDir) {
    let (writer, readers, dir) = test_writer_and_readers();
    let clock = Arc::new(SystemClock);
    let svc = CartridgeService::new(writer, readers, clock);
    (svc, dir)
}

/// Seed a printer device (type_id=2, see `acts_clone_handover.rs::seed_device`
/// for the type_id=1 analog) — Plan 12-06.
async fn seed_printer_device(svc: &CartridgeService, name: &str) -> i64 {
    let name = name.to_string();
    svc.writer
        .execute(move |conn| {
            let tx = conn.transaction().map_err(map_rusqlite)?;
            tx.execute(
                "INSERT INTO devices \
                 (type_id, name, status_id, version, created_at_utc, updated_at_utc) \
                 VALUES (2, ?1, 2, 1, ?2, ?2)",
                params![name, 1_700_000_000_i64],
            )
            .map_err(map_rusqlite)?;
            let id = tx.last_insert_rowid();
            tx.commit().map_err(map_rusqlite)?;
            Ok(id)
        })
        .await
        .expect("seed printer device")
}

/// Same as `seed_printer_device`, but the printer is seeded WITH its own
/// `place_id` set (Plan 40-28, CR-02 real-flow regression test) — mirrors a
/// printer whose physical location is already known, as opposed to
/// `seed_printer_device`'s NULL default.
async fn seed_printer_device_with_place(svc: &CartridgeService, name: &str, place_id: i64) -> i64 {
    let name = name.to_string();
    svc.writer
        .execute(move |conn| {
            let tx = conn.transaction().map_err(map_rusqlite)?;
            tx.execute(
                "INSERT INTO devices \
                 (type_id, name, status_id, place_id, version, created_at_utc, updated_at_utc) \
                 VALUES (2, ?1, 2, ?2, 1, ?3, ?3)",
                params![name, place_id, 1_700_000_000_i64],
            )
            .map_err(map_rusqlite)?;
            let id = tx.last_insert_rowid();
            tx.commit().map_err(map_rusqlite)?;
            Ok(id)
        })
        .await
        .expect("seed printer device with place")
}

/// Seed a real `places` row — FK-valid `place_id` for tests that assert an
/// actual place value (`places.id` carries a `REFERENCES places(id)` FK,
/// V038, enforced in the test harness). Mirrors the `seed_place()` precedent
/// from Plan 09's `cartridges_sqlite.rs` inline test module.
async fn seed_place(svc: &CartridgeService, name: &str) -> i64 {
    let name = name.to_string();
    svc.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO places (kind, name, is_storage, created_at_utc, updated_at_utc, version) \
                 VALUES ('room', ?1, 0, ?2, ?2, 1)",
                params![name, 1_700_000_000_i64],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed place")
}

/// Seed a real `places` row with `is_storage=1` — a складское place, as
/// opposed to `seed_place()` above which always writes `is_storage=0`
/// (Plan 40-22, UAT-40 gap "return-to-stock-empty-place-field": the
/// last-known-storage-place fallback only considers `is_storage=1` places).
async fn seed_storage_place(svc: &CartridgeService, name: &str) -> i64 {
    let name = name.to_string();
    svc.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO places (kind, name, is_storage, created_at_utc, updated_at_utc, version) \
                 VALUES ('room', ?1, 1, ?2, ?2, 1)",
                params![name, 1_700_000_000_i64],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed storage place")
}

/// Read `current_printer_device_id` for a cartridge directly — `CartridgeDto`
/// does not expose this field yet (Plan 12-06 is backend-only; frontend wiring
/// is a future plan).
async fn current_printer_device_id_of(svc: &CartridgeService, cartridge_id: i64) -> Option<i64> {
    svc.writer
        .execute(move |conn| {
            conn.query_row(
                "SELECT current_printer_device_id FROM cartridges WHERE id = ?1",
                params![cartridge_id],
                |r| r.get::<_, Option<i64>>(0),
            )
            .map_err(map_rusqlite)
        })
        .await
        .expect("read current_printer_device_id")
}

/// Read `(status_id, state_id, place_id, holder_name, current_printer_device_id)`
/// for a cartridge directly — used by the auto-return assertions.
#[allow(clippy::type_complexity)]
async fn cartridge_snapshot(
    svc: &CartridgeService,
    cartridge_id: i64,
) -> (i64, Option<i64>, Option<i64>, Option<String>, Option<i64>) {
    svc.writer
        .execute(move |conn| {
            conn.query_row(
                "SELECT status_id, state_id, place_id, holder_name, current_printer_device_id \
                 FROM cartridges WHERE id = ?1",
                params![cartridge_id],
                |r| {
                    Ok((
                        r.get::<_, i64>(0)?,
                        r.get::<_, Option<i64>>(1)?,
                        r.get::<_, Option<i64>>(2)?,
                        r.get::<_, Option<String>>(3)?,
                        r.get::<_, Option<i64>>(4)?,
                    ))
                },
            )
            .map_err(map_rusqlite)
        })
        .await
        .expect("read cartridge snapshot")
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
        place_id: None,
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
        place_id: None,
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
            .transition(
                &admin_caller(),
                CartridgeTransitionPayload::Install {
                    cartridge_id: cart.id,
                    version: cart.version,
                    date_utc: 1_700_000_000,
                    given_by_name: "Иванов".into(),
                    given_to_name: "Петров".into(),
                    place_id: None,
                    printer_device_id: None,
                    previous_cartridge_state_id: None,
                    previous_cartridge_place_id: None,
                },
            )
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
            .transition(
                &admin_caller(),
                CartridgeTransitionPayload::Install {
                    cartridge_id: cart.id,
                    version: cart.version,
                    date_utc: 1_700_000_000,
                    given_by_name: "A".into(),
                    given_to_name: "B".into(),
                    place_id: None,
                    printer_device_id: None,
                    previous_cartridge_state_id: None,
                    previous_cartridge_place_id: None,
                },
            )
            .await
            .expect("install");

        // Then return to stock with state = 3 (Пустой)
        let returned = svc
            .transition(
                &admin_caller(),
                CartridgeTransitionPayload::ReturnToStock {
                    cartridge_id: in_use.id,
                    version: in_use.version,
                    state_id: 3, // Пустой
                    place_id: None,
                    notes: None,
                },
            )
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
            .transition(
                &admin_caller(),
                CartridgeTransitionPayload::ToRefill {
                    cartridge_id: cart.id,
                    version: cart.version,
                    date_utc: 1_700_000_000,
                    given_by_name: "Иванов".into(),
                    given_to_name: "ООО Заправка".into(),
                    place_id: None,
                },
            )
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
            .transition(
                &admin_caller(),
                CartridgeTransitionPayload::ToRefill {
                    cartridge_id: cart.id,
                    version: cart.version,
                    date_utc: 1_700_000_000,
                    given_by_name: "A".into(),
                    given_to_name: "Заправщик".into(),
                    place_id: None,
                },
            )
            .await
            .expect("to_refill");

        // Return from refill with state = 1 (Полный)
        let back = svc
            .transition(
                &admin_caller(),
                CartridgeTransitionPayload::FromRefill {
                    cartridge_id: at_refill.id,
                    version: at_refill.version,
                    state_id: 1, // Полный
                    place_id: None,
                    notes: None,
                },
            )
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
            .transition(
                &admin_caller(),
                CartridgeTransitionPayload::WriteOff {
                    cartridge_id: cart.id,
                    version: cart.version,
                    date_utc: 1_700_000_000,
                    notes: Some("Физический износ".into()),
                },
            )
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
            .transition(
                &admin_caller(),
                CartridgeTransitionPayload::Install {
                    cartridge_id: cart.id,
                    version: cart.version,
                    date_utc: 1_700_000_000,
                    given_by_name: "A".into(),
                    given_to_name: "B".into(),
                    place_id: None,
                    printer_device_id: None,
                    previous_cartridge_state_id: None,
                    previous_cartridge_place_id: None,
                },
            )
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
            .transition(
                &admin_caller(),
                CartridgeTransitionPayload::ReturnToStock {
                    cartridge_id: in_use.id,
                    version: in_use.version,
                    state_id: 3,
                    place_id: None,
                    notes: None,
                },
            )
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

/// CR-01 regression: `installable_only: true` must be kind-aware. Photo-drums
/// (kind_id=2) use charge states 4=Новый/5=Изношенный/6=Отработанный, not the
/// cartridge-only states 1/2. A state_id=4 drum on stock must be returned by
/// the install picker; a state_id=6 (Отработанный, already refused at install
/// time) drum must be excluded.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn installable_only_includes_new_drum_excludes_spent_drum() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_cartridge_service();
        let drum_model_id = svc
            .model_create(CartridgeModelCreateDto {
                brand: "Kyocera".into(),
                model: "DK-1170".into(),
                kind_id: 2, // Фотобарабан
                color: None,
                notes: None,
                compatibility: vec![],
            })
            .await
            .expect("seed drum model")
            .id;

        create_stock_cartridge_with_state(&svc, drum_model_id, 4).await; // Новый
        create_stock_cartridge_with_state(&svc, drum_model_id, 6).await; // Отработанный

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
            .expect("list installable_only=true for drums");

        assert_eq!(
            result.items.len(),
            1,
            "only the state_id=4 (Новый) drum must be installable, got: {:?}",
            result.items
        );
        assert_eq!(
            result.items[0].state_id,
            Some(4),
            "the returned drum must be the Новый (state_id=4) one: {:?}",
            result.items
        );
        assert!(
            result.items.iter().all(|c| c.state_id != Some(6)),
            "Отработанный (state_id=6) drum must never be installable: {:?}",
            result.items
        );
    })
    .await
    .expect("installable_only_includes_new_drum_excludes_spent_drum budget")
}

// ---------------------------------------------------------------------------
// Plan 12-06 (D-16..D-19, GAP-12-03): printer link + auto-return.
// ---------------------------------------------------------------------------

/// Test 1 (D-19): installing with `printer_device_id: Some(pid)` writes
/// `cartridges.current_printer_device_id = pid`.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn install_with_printer_sets_current_printer_device_id() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_cartridge_service();
        let model_id = seed_model(&svc).await;
        let printer_id = seed_printer_device(&svc, "Pantum BM5100ADN").await;
        let cart = create_stock_cartridge(&svc, model_id).await;

        let installed = svc
            .transition(
                &admin_caller(),
                CartridgeTransitionPayload::Install {
                    cartridge_id: cart.id,
                    version: cart.version,
                    date_utc: 1_700_000_000,
                    given_by_name: "Иванов".into(),
                    given_to_name: "Петров".into(),
                    place_id: None,
                    printer_device_id: Some(printer_id),
                    previous_cartridge_state_id: None,
                    previous_cartridge_place_id: None,
                },
            )
            .await
            .expect("install with printer");

        assert_eq!(installed.status_id, 2, "status must be В работе (2)");
        let linked = current_printer_device_id_of(&svc, installed.id).await;
        assert_eq!(
            linked,
            Some(printer_id),
            "current_printer_device_id must equal the target printer's id"
        );
    })
    .await
    .expect("install_with_printer_sets_current_printer_device_id budget")
}

/// Test 2 (D-16/D-17): installing cartridge B into a printer that already has
/// cartridge A "В работе" auto-returns A to stock (status=1, state=3 Пустой,
/// place_id=NULL, current_printer_device_id=NULL, holder_name=NULL) within the
/// SAME `transition()` call that installs B.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn install_auto_returns_previous_cartridge_in_same_printer() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_cartridge_service();
        let model_id = seed_model(&svc).await;
        let printer_id = seed_printer_device(&svc, "Pantum BM5100ADN").await;

        let cart_a = create_stock_cartridge(&svc, model_id).await;
        let cart_b = create_stock_cartridge(&svc, model_id).await;

        // Install A into the printer first.
        let a_installed = svc
            .transition(
                &admin_caller(),
                CartridgeTransitionPayload::Install {
                    cartridge_id: cart_a.id,
                    version: cart_a.version,
                    date_utc: 1_700_000_000,
                    given_by_name: "Иванов".into(),
                    given_to_name: "Петров".into(),
                    place_id: None,
                    printer_device_id: Some(printer_id),
                    previous_cartridge_state_id: None,
                    previous_cartridge_place_id: None,
                },
            )
            .await
            .expect("install A");
        assert_eq!(a_installed.status_id, 2);

        // Install B into the SAME printer — must auto-return A.
        let b_installed = svc
            .transition(
                &admin_caller(),
                CartridgeTransitionPayload::Install {
                    cartridge_id: cart_b.id,
                    version: cart_b.version,
                    date_utc: 1_700_000_100,
                    given_by_name: "Сидоров".into(),
                    given_to_name: "Кузнецов".into(),
                    place_id: None,
                    printer_device_id: Some(printer_id),
                    previous_cartridge_state_id: None,
                    previous_cartridge_place_id: None,
                },
            )
            .await
            .expect("install B into same printer");

        // B is now the printer's current cartridge.
        assert_eq!(b_installed.status_id, 2, "B must be В работе (2)");
        let b_linked = current_printer_device_id_of(&svc, b_installed.id).await;
        assert_eq!(b_linked, Some(printer_id));

        // A was auto-returned to stock — all in the ONE transition() call above.
        let (a_status, a_state, a_place_id, a_holder, a_printer) =
            cartridge_snapshot(&svc, a_installed.id).await;
        assert_eq!(a_status, 1, "A must be На складе (1) after auto-return");
        assert_eq!(a_state, Some(3), "A's state must default to Пустой (3)");
        assert_eq!(
            a_place_id, None,
            "A's place_id must be cleared to NULL when the cartridge has no prior \
             storage-place movement history (create_stock_cartridge seeds place_id=None \
             and A was never moved into a storage place before this install, so the \
             Plan 40-22 last-known-storage-place fallback has nothing to derive)"
        );
        assert_eq!(a_holder, None, "A's holder_name must be cleared");
        assert_eq!(
            a_printer, None,
            "A's current_printer_device_id must be cleared"
        );
    })
    .await
    .expect("install_auto_returns_previous_cartridge_in_same_printer budget")
}

/// Test 3 (D-18): installing into a printer that NEVER had a cartridge causes
/// no side effects on an unrelated cartridge.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn install_into_empty_printer_has_no_side_effects() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_cartridge_service();
        let model_id = seed_model(&svc).await;
        let printer_id = seed_printer_device(&svc, "Kyocera ECOSYS").await;

        let cart_c = create_stock_cartridge(&svc, model_id).await;
        // Unrelated cartridge, never touched by this printer.
        let unrelated = create_stock_cartridge(&svc, model_id).await;

        let installed = svc
            .transition(
                &admin_caller(),
                CartridgeTransitionPayload::Install {
                    cartridge_id: cart_c.id,
                    version: cart_c.version,
                    date_utc: 1_700_000_000,
                    given_by_name: "Иванов".into(),
                    given_to_name: "Петров".into(),
                    place_id: None,
                    printer_device_id: Some(printer_id),
                    previous_cartridge_state_id: None,
                    previous_cartridge_place_id: None,
                },
            )
            .await
            .expect("install C into empty printer");

        assert_eq!(installed.status_id, 2);
        let linked = current_printer_device_id_of(&svc, installed.id).await;
        assert_eq!(linked, Some(printer_id));

        // Unrelated cartridge is untouched — still На складе with no printer link.
        let (u_status, _u_state, _u_place_id, u_holder, u_printer) =
            cartridge_snapshot(&svc, unrelated.id).await;
        assert_eq!(u_status, 1, "unrelated cartridge must remain На складе (1)");
        assert_eq!(u_holder, None);
        assert_eq!(u_printer, None);
    })
    .await
    .expect("install_into_empty_printer_has_no_side_effects budget")
}

/// Test 4 (backward-compat, D-08): `printer_device_id: None` performs the
/// status transition exactly as before — no printer lookup, no auto-return.
/// This is a regression guard duplicating `install_changes_status_to_in_use`'s
/// assertions plus an explicit check that current_printer_device_id stays NULL.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn install_without_printer_device_id_has_no_side_effects() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_cartridge_service();
        let model_id = seed_model(&svc).await;
        let cart = create_stock_cartridge(&svc, model_id).await;

        let updated = svc
            .transition(
                &admin_caller(),
                CartridgeTransitionPayload::Install {
                    cartridge_id: cart.id,
                    version: cart.version,
                    date_utc: 1_700_000_000,
                    given_by_name: "Иванов".into(),
                    given_to_name: "Петров".into(),
                    place_id: None,
                    printer_device_id: None,
                    previous_cartridge_state_id: None,
                    previous_cartridge_place_id: None,
                },
            )
            .await
            .expect("install without printer_device_id");

        assert_eq!(updated.status_id, 2, "status must be В работе (2)");
        assert_eq!(updated.holder_name.as_deref(), Some("Петров"));
        let linked = current_printer_device_id_of(&svc, updated.id).await;
        assert_eq!(
            linked, None,
            "current_printer_device_id must stay NULL when no printer is supplied"
        );
    })
    .await
    .expect("install_without_printer_device_id_has_no_side_effects budget")
}

/// Test 5 (D-17, audit; GAP-12-12 extended): after the auto-return scenario,
/// the previous cartridge's audit_log entry carries
/// `action = 'custom:return_to_stock'` AND its `payload_json` records an
/// INVERTED actor relative to the new install (B): B's given_to_name
/// ("Кузнецов", the recipient of the new cartridge) is the one who hands
/// back A, so A's payload must have `given_by_name == "Кузнецов"`; B's
/// given_by_name ("Сидоров", issuer/warehouse) receives A back, so A's
/// payload must have `given_to_name == "Сидоров"`.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn auto_return_writes_return_to_stock_audit_entry() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_cartridge_service();
        let model_id = seed_model(&svc).await;
        let printer_id = seed_printer_device(&svc, "HP LaserJet").await;

        let cart_a = create_stock_cartridge(&svc, model_id).await;
        let cart_b = create_stock_cartridge(&svc, model_id).await;

        let a_installed = svc
            .transition(&admin_caller(), CartridgeTransitionPayload::Install {
                cartridge_id: cart_a.id,
                version: cart_a.version,
                date_utc: 1_700_000_000,
                given_by_name: "Иванов".into(),
                given_to_name: "Петров".into(),
                place_id: None,
                printer_device_id: Some(printer_id),
                previous_cartridge_state_id: None,
                previous_cartridge_place_id: None,
            })
            .await
            .expect("install A");

        svc.transition(&admin_caller(), CartridgeTransitionPayload::Install {
            cartridge_id: cart_b.id,
            version: cart_b.version,
            date_utc: 1_700_000_100,
            given_by_name: "Сидоров".into(),
            given_to_name: "Кузнецов".into(),
            place_id: None,
            printer_device_id: Some(printer_id),
            previous_cartridge_state_id: None,
            previous_cartridge_place_id: None,
        })
        .await
        .expect("install B auto-returns A");

        let a_history = svc.get_history(a_installed.id).await.expect("A history");
        let return_entry = a_history
            .iter()
            .find(|e| e.action == "custom:return_to_stock")
            .expect("A's audit history must contain a custom:return_to_stock entry");

        let payload: Value = serde_json::from_str(
            return_entry
                .payload_json
                .as_deref()
                .expect("return_to_stock entry must have payload_json"),
        )
        .expect("payload_json must parse as JSON");

        assert_eq!(
            payload.get("given_by_name").and_then(Value::as_str),
            Some("Кузнецов"),
            "given_by_name must be B's given_to_name (recipient hands A back): {payload:?}"
        );
        assert_eq!(
            payload.get("given_to_name").and_then(Value::as_str),
            Some("Сидоров"),
            "given_to_name must be B's given_by_name (issuer/warehouse receives A back): {payload:?}"
        );
    })
    .await
    .expect("auto_return_writes_return_to_stock_audit_entry budget")
}

/// Test 6 (GAP-12-12 round-trip): install with `printer_device_id` sets
/// `current_printer_device_id`; a subsequent direct `ReturnToStock` of that
/// same cartridge clears the link (NULL). Proves the binding is symmetric
/// across the full install→return lifecycle, not just on auto-return.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn return_to_stock_clears_current_printer_device_id() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_cartridge_service();
        let model_id = seed_model(&svc).await;
        let printer_id = seed_printer_device(&svc, "Canon iR").await;
        let cart = create_stock_cartridge(&svc, model_id).await;

        let installed = svc
            .transition(
                &admin_caller(),
                CartridgeTransitionPayload::Install {
                    cartridge_id: cart.id,
                    version: cart.version,
                    date_utc: 1_700_000_000,
                    given_by_name: "Иванов".into(),
                    given_to_name: "Петров".into(),
                    place_id: None,
                    printer_device_id: Some(printer_id),
                    previous_cartridge_state_id: None,
                    previous_cartridge_place_id: None,
                },
            )
            .await
            .expect("install with printer");

        let linked = current_printer_device_id_of(&svc, installed.id).await;
        assert_eq!(
            linked,
            Some(printer_id),
            "current_printer_device_id must be set after install"
        );

        let returned = svc
            .transition(
                &admin_caller(),
                CartridgeTransitionPayload::ReturnToStock {
                    cartridge_id: installed.id,
                    version: installed.version,
                    state_id: 3,
                    place_id: None,
                    notes: None,
                },
            )
            .await
            .expect("direct return to stock");
        assert_eq!(returned.status_id, 1, "status must be На складе (1)");

        let unlinked = current_printer_device_id_of(&svc, installed.id).await;
        assert_eq!(
            unlinked, None,
            "current_printer_device_id must be cleared after direct return"
        );
    })
    .await
    .expect("return_to_stock_clears_current_printer_device_id budget")
}

// ---------------------------------------------------------------------------
// Plan 12-09 (D-16, GAP-12-03 frontend close): previous-cartridge overrides.
// ---------------------------------------------------------------------------

/// Test 1 (D-16 override): installing with explicit
/// `previous_cartridge_state_id`/`previous_cartridge_place_id` overrides the
/// auto-return's hardcoded defaults — the previous cartridge ends up with the
/// USER-supplied charge state and place, not 3 (Пустой)/NULL.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn install_auto_return_uses_previous_cartridge_overrides_when_present() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_cartridge_service();
        let model_id = seed_model(&svc).await;
        let printer_id = seed_printer_device(&svc, "Pantum BM5100ADN").await;
        // FK-valid place row for the previous_cartridge_place_id override
        // below (places.id has a REFERENCES places(id) FK, V038) — mirrors
        // the seed_place() precedent from Plan 09.
        let override_place_id = seed_place(&svc, "Кабинет 5").await;

        let cart_a = create_stock_cartridge(&svc, model_id).await;
        let cart_b = create_stock_cartridge(&svc, model_id).await;

        // Install A into the printer first.
        let a_installed = svc
            .transition(
                &admin_caller(),
                CartridgeTransitionPayload::Install {
                    cartridge_id: cart_a.id,
                    version: cart_a.version,
                    date_utc: 1_700_000_000,
                    given_by_name: "Иванов".into(),
                    given_to_name: "Петров".into(),
                    place_id: None,
                    printer_device_id: Some(printer_id),
                    previous_cartridge_state_id: None,
                    previous_cartridge_place_id: None,
                },
            )
            .await
            .expect("install A");

        // Install B into the SAME printer with explicit overrides for A's
        // auto-return — state_id=1 (Полный), place_id=override_place_id.
        let b_installed = svc
            .transition(
                &admin_caller(),
                CartridgeTransitionPayload::Install {
                    cartridge_id: cart_b.id,
                    version: cart_b.version,
                    date_utc: 1_700_000_100,
                    given_by_name: "Сидоров".into(),
                    given_to_name: "Кузнецов".into(),
                    place_id: None,
                    printer_device_id: Some(printer_id),
                    previous_cartridge_state_id: Some(1),
                    previous_cartridge_place_id: Some(override_place_id),
                },
            )
            .await
            .expect("install B into same printer with overrides");

        assert_eq!(b_installed.status_id, 2, "B must be В работе (2)");

        // A was auto-returned with the USER-supplied overrides, not the
        // hardcoded defaults (state_id=3, place_id=NULL).
        let (a_status, a_state, a_place_id, a_holder, a_printer) =
            cartridge_snapshot(&svc, a_installed.id).await;
        assert_eq!(a_status, 1, "A must be На складе (1) after auto-return");
        assert_eq!(
            a_state,
            Some(1),
            "A's state must be the overridden Полный (1), not the default Пустой (3)"
        );
        assert_eq!(
            a_place_id,
            Some(override_place_id),
            "A's place_id must be the overridden value, not cleared to NULL"
        );
        assert_eq!(a_holder, None, "A's holder_name must still be cleared");
        assert_eq!(
            a_printer, None,
            "A's current_printer_device_id must still be cleared"
        );
    })
    .await
    .expect("install_auto_return_uses_previous_cartridge_overrides_when_present budget")
}

/// Test 2 (D-16 backward-compat): when `previous_cartridge_state_id`/
/// `previous_cartridge_place_id` are both `None`, the auto-return falls back
/// to 12-06's original hardcoded defaults (state_id=3 Пустой, place_id=NULL)
/// when the cartridge has no prior storage-place movement history —
/// proves this widening does not regress 12-06's own behavior. A's
/// `place_id` stays `None` here because `create_stock_cartridge` seeds
/// place_id=None and the printer in this test also has no place, so there
/// is nothing for the Plan 40-22 last-known-storage-place fallback to
/// derive (see `install_auto_return_falls_back_to_last_known_storage_place`
/// below for the fallback's positive case).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn install_auto_return_falls_back_to_defaults_when_overrides_absent() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_cartridge_service();
        let model_id = seed_model(&svc).await;
        let printer_id = seed_printer_device(&svc, "Kyocera ECOSYS").await;

        let cart_a = create_stock_cartridge(&svc, model_id).await;
        let cart_b = create_stock_cartridge(&svc, model_id).await;

        let a_installed = svc
            .transition(
                &admin_caller(),
                CartridgeTransitionPayload::Install {
                    cartridge_id: cart_a.id,
                    version: cart_a.version,
                    date_utc: 1_700_000_000,
                    given_by_name: "Иванов".into(),
                    given_to_name: "Петров".into(),
                    place_id: None,
                    printer_device_id: Some(printer_id),
                    previous_cartridge_state_id: None,
                    previous_cartridge_place_id: None,
                },
            )
            .await
            .expect("install A");

        let b_installed = svc
            .transition(
                &admin_caller(),
                CartridgeTransitionPayload::Install {
                    cartridge_id: cart_b.id,
                    version: cart_b.version,
                    date_utc: 1_700_000_100,
                    given_by_name: "Сидоров".into(),
                    given_to_name: "Кузнецов".into(),
                    place_id: None,
                    printer_device_id: Some(printer_id),
                    previous_cartridge_state_id: None,
                    previous_cartridge_place_id: None,
                },
            )
            .await
            .expect("install B into same printer without overrides");

        assert_eq!(b_installed.status_id, 2);

        let (a_status, a_state, a_place_id, _a_holder, _a_printer) =
            cartridge_snapshot(&svc, a_installed.id).await;
        assert_eq!(a_status, 1, "A must be На складе (1) after auto-return");
        assert_eq!(
            a_state,
            Some(3),
            "A's state must fall back to the default Пустой (3) when no override given"
        );
        assert_eq!(
            a_place_id, None,
            "A's place_id must fall back to the default NULL when no override given \
             and the cartridge has no prior storage-place movement history"
        );
    })
    .await
    .expect("install_auto_return_falls_back_to_defaults_when_overrides_absent budget")
}

/// Test 3 (Plan 40-22, UAT-40 gap "return-to-stock-empty-place-field"): when
/// `previous_cartridge_place_id` is `None` AND the returned cartridge DOES
/// have prior storage-place movement history, the auto-return derives its
/// `place_id` from that history instead of clearing it to NULL — the
/// positive case for `last_known_storage_place_in_tx`, complementing the
/// no-history case covered by
/// `install_auto_return_falls_back_to_defaults_when_overrides_absent` above.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn install_auto_return_falls_back_to_last_known_storage_place() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_cartridge_service();
        let model_id = seed_model(&svc).await;
        let printer_id = seed_printer_device(&svc, "Kyocera ECOSYS").await;

        // A складское place (is_storage=1) — the place the fallback must
        // derive — and a second, non-storage place used only as the
        // synthetic movement's `from_place_id` (FK-valid, arbitrary origin).
        let storage_place_id = seed_storage_place(&svc, "Склад расходников").await;
        let other_place_id = seed_place(&svc, "Каб. 310").await;

        let cart_a = create_stock_cartridge(&svc, model_id).await;
        let cart_b = create_stock_cartridge(&svc, model_id).await;

        // Imitate A having previously sat in the storage place, before
        // either of this test's Install calls — a direct SQL insert into
        // place_movements (not via the service), created_at_utc earlier
        // than the first Install below.
        svc.writer
            .execute(move |conn| {
                conn.execute(
                    "INSERT INTO place_movements \
                     (entity_type, entity_id, from_place_id, from_place_path, \
                      to_place_id, to_place_path, source, note, act_id, user_id, \
                      actor_name_snapshot, created_at_utc) \
                     VALUES ('cartridge', ?1, ?2, 'Каб. 310', ?3, 'Склад расходников', \
                             'manual', 'seeded for test', NULL, NULL, NULL, ?4)",
                    params![
                        cart_a.id,
                        other_place_id,
                        storage_place_id,
                        1_699_999_000_i64
                    ],
                )
                .map_err(map_rusqlite)?;
                Ok(())
            })
            .await
            .expect("seed prior storage-place movement for A");

        // Install A into the printer — place_id stays whatever create_stock_cartridge
        // seeded (None); the synthetic movement above is A's ONLY place_movements row.
        let a_installed = svc
            .transition(
                &admin_caller(),
                CartridgeTransitionPayload::Install {
                    cartridge_id: cart_a.id,
                    version: cart_a.version,
                    date_utc: 1_700_000_000,
                    given_by_name: "Иванов".into(),
                    given_to_name: "Петров".into(),
                    place_id: None,
                    printer_device_id: Some(printer_id),
                    previous_cartridge_state_id: None,
                    previous_cartridge_place_id: None,
                },
            )
            .await
            .expect("install A");

        // Install B into the SAME printer, again without an explicit
        // previous_cartridge_place_id override — must auto-return A using
        // the last known storage place, not NULL.
        let b_installed = svc
            .transition(
                &admin_caller(),
                CartridgeTransitionPayload::Install {
                    cartridge_id: cart_b.id,
                    version: cart_b.version,
                    date_utc: 1_700_000_100,
                    given_by_name: "Сидоров".into(),
                    given_to_name: "Кузнецов".into(),
                    place_id: None,
                    printer_device_id: Some(printer_id),
                    previous_cartridge_state_id: None,
                    previous_cartridge_place_id: None,
                },
            )
            .await
            .expect("install B into same printer without overrides");

        assert_eq!(b_installed.status_id, 2, "B must be В работе (2)");

        let (a_status, a_state, a_place_id, _a_holder, _a_printer) =
            cartridge_snapshot(&svc, a_installed.id).await;
        assert_eq!(a_status, 1, "A must be На складе (1) after auto-return");
        assert_eq!(
            a_state,
            Some(3),
            "A's state must still default to Пустой (3) — this test only covers place_id"
        );
        assert_eq!(
            a_place_id,
            Some(storage_place_id),
            "A's place_id must fall back to its last known storage place, not NULL, \
             when the cartridge has prior place_movements history into an is_storage=1 place"
        );
    })
    .await
    .expect("install_auto_return_falls_back_to_last_known_storage_place budget")
}

/// Test 4 (Plan 40-28, CR-02 / 40-VERIFICATION.md gap 2): the SAME positive
/// fallback as `install_auto_return_falls_back_to_last_known_storage_place`
/// above, but driven ENTIRELY through `CartridgeService` — NO raw SQL seed of
/// `place_movements`. This is the exact real lifecycle the old single-query
/// fallback (`to_place_id` only) could never satisfy: a cartridge's FIRST
/// place assignment (create at storage, D-06) never produces a movement row,
/// so the only row the real flow below ever writes for A is the Install
/// S(storage) -> Q(printer's own, non-storage place) — a `from_place_id`
/// hit, not a `to_place_id` hit. Before the fix this test reproduces the
/// UAT-16 defect (A's place_id resolves to `None` instead of the storage
/// place); after the fix it passes via the `from_place_id`/`p_from.is_storage`
/// branch of `last_known_storage_place_in_tx`'s first query.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn install_auto_return_falls_back_via_real_service_flow_no_hand_seed() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_cartridge_service();
        let model_id = seed_model(&svc).await;

        // Склад (is_storage=1) — where both cartridges are created — and a
        // normal, non-storage place standing in for the printer's own
        // physical location (Каб. 210, say) — D-13 resolves Install's
        // place_id from THIS place, never from the storage place.
        let storage_place_id = seed_storage_place(&svc, "Склад расходников 2").await;
        let printer_place_id = seed_place(&svc, "Каб. 210").await;
        let printer_id =
            seed_printer_device_with_place(&svc, "Brother HL-2240", printer_place_id).await;

        // (4) A created directly AT the storage place — D-06: first
        // assignment never writes a place_movements row, so A has ZERO
        // history entering this test.
        let cart_a = svc
            .create(CartridgeCreateDto {
                model_id,
                code_override: None,
                state_id: Some(1),
                place_id: Some(storage_place_id),
                notes: None,
            })
            .await
            .expect("create cartridge A at storage");

        // (5) Install A into the printer with NO explicit place_id — D-13
        // resolves it from the printer's own (non-storage) place. This is
        // the ONLY place_movements row A ever gets: from_place_id=storage
        // (is_storage=1), to_place_id=printer_place (is_storage=0).
        let a_installed = svc
            .transition(
                &admin_caller(),
                CartridgeTransitionPayload::Install {
                    cartridge_id: cart_a.id,
                    version: cart_a.version,
                    date_utc: 1_700_000_000,
                    given_by_name: "Иванов".into(),
                    given_to_name: "Петров".into(),
                    place_id: None,
                    printer_device_id: Some(printer_id),
                    previous_cartridge_state_id: None,
                    previous_cartridge_place_id: None,
                },
            )
            .await
            .expect("install A (D-13 resolves place from printer)");

        // (6) B created at the same storage place.
        let cart_b = svc
            .create(CartridgeCreateDto {
                model_id,
                code_override: None,
                state_id: Some(1),
                place_id: Some(storage_place_id),
                notes: None,
            })
            .await
            .expect("create cartridge B at storage");

        // (7) Install B into the SAME printer, again with no explicit
        // previous_cartridge_place_id — triggers A's auto-return with no
        // override, exercising the fallback chain end-to-end.
        let b_installed = svc
            .transition(
                &admin_caller(),
                CartridgeTransitionPayload::Install {
                    cartridge_id: cart_b.id,
                    version: cart_b.version,
                    date_utc: 1_700_000_100,
                    given_by_name: "Сидоров".into(),
                    given_to_name: "Кузнецов".into(),
                    place_id: None,
                    printer_device_id: Some(printer_id),
                    previous_cartridge_state_id: None,
                    previous_cartridge_place_id: None,
                },
            )
            .await
            .expect("install B into same printer, no explicit previous_cartridge_place_id");

        assert_eq!(b_installed.status_id, 2, "B must be В работе (2)");

        let (a_status, _a_state, a_place_id, _a_holder, _a_printer) =
            cartridge_snapshot(&svc, a_installed.id).await;
        assert_eq!(a_status, 1, "A must be На складе (1) after auto-return");
        assert_eq!(
            a_place_id,
            Some(storage_place_id),
            "A's place_id must resolve to its last known STORAGE place via the \
             from_place_id branch of the real S->Q install movement — driven entirely \
             through CartridgeService, no hand-seeded place_movements row"
        );
    })
    .await
    .expect("install_auto_return_falls_back_via_real_service_flow_no_hand_seed budget")
}

// ---------------------------------------------------------------------------
// transition — caller threading (Plan 40-04, Pitfall 1/3)
// ---------------------------------------------------------------------------

/// Main mutation: a plain Install (no auto-return triggered) must store the
/// real caller's user_id on its own audit_log row, not a hard-coded NULL.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn transition_stores_real_caller_user_id_on_main_mutation_audit_log() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_cartridge_service();
        let model_id = seed_model(&svc).await;
        let cart = create_stock_cartridge(&svc, model_id).await;

        let manager_user_id = seed_manager_user(&svc.writer).await;
        let manager = Identity {
            user_id: Some(manager_user_id),
            role: Role::Manager,
        };

        let installed = svc
            .transition(
                &manager,
                CartridgeTransitionPayload::Install {
                    cartridge_id: cart.id,
                    version: cart.version,
                    date_utc: 1_700_000_000,
                    given_by_name: "Иванов".into(),
                    given_to_name: "Петров".into(),
                    place_id: None,
                    printer_device_id: None,
                    previous_cartridge_state_id: None,
                    previous_cartridge_place_id: None,
                },
            )
            .await
            .expect("transition Install with manager caller");

        let readers = svc.readers.clone();
        let entity_id = installed.id;
        let user_id: Option<i64> = tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            conn.query_row(
                "SELECT user_id FROM audit_log \
                 WHERE entity_type='cartridge' AND entity_id=?1 AND action='custom:install'",
                params![entity_id],
                |r| r.get(0),
            )
        })
        .await
        .expect("spawn_blocking")
        .expect("query audit_log user_id");

        assert_eq!(
            user_id, manager.user_id,
            "audit_log.user_id должен совпадать с caller.user_id реального менеджера"
        );
    })
    .await
    .expect("transition_stores_real_caller_user_id_on_main_mutation_audit_log budget")
}

/// Pitfall 3 (RESEARCH.md): the nested auto-return branch inside
/// `transition_in_tx` writes its OWN audit_log row for the PREVIOUSLY
/// installed cartridge — a separate entity, a separate call site. Both the
/// main mutation's row AND the auto-returned cartridge's row must carry the
/// real caller's user_id.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn transition_stores_real_caller_user_id_on_auto_return_audit_log() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_cartridge_service();
        let model_id = seed_model(&svc).await;
        let printer_id = seed_printer_device(&svc, "Pantum BM5100ADN").await;

        let cart_a = create_stock_cartridge(&svc, model_id).await;
        let cart_b = create_stock_cartridge(&svc, model_id).await;

        let manager_user_id = seed_manager_user(&svc.writer).await;
        let manager = Identity {
            user_id: Some(manager_user_id),
            role: Role::Manager,
        };

        // Install A into the printer first.
        let a_installed = svc
            .transition(
                &manager,
                CartridgeTransitionPayload::Install {
                    cartridge_id: cart_a.id,
                    version: cart_a.version,
                    date_utc: 1_700_000_000,
                    given_by_name: "Иванов".into(),
                    given_to_name: "Петров".into(),
                    place_id: None,
                    printer_device_id: Some(printer_id),
                    previous_cartridge_state_id: None,
                    previous_cartridge_place_id: None,
                },
            )
            .await
            .expect("install A with manager caller");

        // Install B into the SAME printer — auto-returns A within the SAME
        // transition() call, using the SAME manager caller.
        let b_installed = svc
            .transition(
                &manager,
                CartridgeTransitionPayload::Install {
                    cartridge_id: cart_b.id,
                    version: cart_b.version,
                    date_utc: 1_700_000_100,
                    given_by_name: "Сидоров".into(),
                    given_to_name: "Кузнецов".into(),
                    place_id: None,
                    printer_device_id: Some(printer_id),
                    previous_cartridge_state_id: None,
                    previous_cartridge_place_id: None,
                },
            )
            .await
            .expect("install B with manager caller (triggers auto-return of A)");

        let readers = svc.readers.clone();
        let a_id = a_installed.id;
        let b_id = b_installed.id;

        // B's own audit row (the main mutation this transition() call made).
        let b_user_id: Option<i64> = tokio::task::spawn_blocking({
            let readers = readers.clone();
            move || {
                let conn = readers.acquire();
                conn.query_row(
                    "SELECT user_id FROM audit_log \
                     WHERE entity_type='cartridge' AND entity_id=?1 AND action='custom:install'",
                    params![b_id],
                    |r| r.get(0),
                )
            }
        })
        .await
        .expect("spawn_blocking")
        .expect("query audit_log user_id for B (main mutation)");

        // A's auto-return audit row — a SEPARATE entity, SEPARATE call site
        // (Pitfall 3), written by the nested branch inside transition_in_tx.
        let a_user_id: Option<i64> = tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            conn.query_row(
                "SELECT user_id FROM audit_log \
                 WHERE entity_type='cartridge' AND entity_id=?1 AND action='custom:return_to_stock'",
                params![a_id],
                |r| r.get(0),
            )
        })
        .await
        .expect("spawn_blocking")
        .expect("query audit_log user_id for A (auto-return)");

        assert_eq!(
            b_user_id, manager.user_id,
            "main mutation's audit_log.user_id must be the real manager caller"
        );
        assert_eq!(
            a_user_id, manager.user_id,
            "auto-return's audit_log.user_id must ALSO be the real manager caller (Pitfall 3)"
        );
    })
    .await
    .expect("transition_stores_real_caller_user_id_on_auto_return_audit_log budget")
}

// ---------------------------------------------------------------------------
// Plan 40-21: explicit cartridge place backfills printer's place
// ---------------------------------------------------------------------------

/// Read `devices.place_id` directly — used by the backfill assertions below.
async fn device_place_id(svc: &CartridgeService, device_id: i64) -> Option<i64> {
    svc.writer
        .execute(move |conn| {
            conn.query_row(
                "SELECT place_id FROM devices WHERE id = ?1",
                params![device_id],
                |r| r.get::<_, Option<i64>>(0),
            )
            .map_err(map_rusqlite)
        })
        .await
        .expect("read device place_id")
}

/// Test 3 (Phase 40-21, gap item 4): Install with an explicit cartridge place
/// into a printer that has no place yet backfills `devices.place_id` for that
/// printer from the cartridge's explicit place.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn install_with_explicit_place_backfills_printer_without_place() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_cartridge_service();
        let model_id = seed_model(&svc).await;
        let printer_id = seed_printer_device(&svc, "Pantum BM5100ADN").await;
        let place_id = seed_place(&svc, "Каб. 601").await;
        let cart = create_stock_cartridge(&svc, model_id).await;

        assert_eq!(
            device_place_id(&svc, printer_id).await,
            None,
            "precondition: printer has no place yet"
        );

        svc.transition(
            &admin_caller(),
            CartridgeTransitionPayload::Install {
                cartridge_id: cart.id,
                version: cart.version,
                date_utc: 1_700_000_000,
                given_by_name: "Иванов".into(),
                given_to_name: "Петров".into(),
                place_id: Some(place_id),
                printer_device_id: Some(printer_id),
                previous_cartridge_state_id: None,
                previous_cartridge_place_id: None,
            },
        )
        .await
        .expect("install with explicit place into placeless printer");

        assert_eq!(
            device_place_id(&svc, printer_id).await,
            Some(place_id),
            "printer's place должен быть заполнен по месту установленного картриджа"
        );
    })
    .await
    .expect("install_with_explicit_place_backfills_printer_without_place budget")
}

/// Test 4 (Phase 40-21, gap item 4): Install with an explicit cartridge place
/// into a printer that ALREADY has a place does NOT overwrite the printer's
/// existing place.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn install_with_explicit_place_does_not_override_printer_with_existing_place() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_cartridge_service();
        let model_id = seed_model(&svc).await;
        let printer_id = seed_printer_device(&svc, "Kyocera ECOSYS").await;
        let place_x = seed_place(&svc, "Каб. 701").await;
        let place_y = seed_place(&svc, "Каб. 702").await;
        let cart = create_stock_cartridge(&svc, model_id).await;

        // Printer already has place X.
        svc.writer
            .execute(move |conn| {
                conn.execute(
                    "UPDATE devices SET place_id = ?1 WHERE id = ?2",
                    params![place_x, printer_id],
                )
                .map_err(map_rusqlite)
            })
            .await
            .expect("seed printer place X");

        svc.transition(
            &admin_caller(),
            CartridgeTransitionPayload::Install {
                cartridge_id: cart.id,
                version: cart.version,
                date_utc: 1_700_000_000,
                given_by_name: "Иванов".into(),
                given_to_name: "Петров".into(),
                place_id: Some(place_y),
                printer_device_id: Some(printer_id),
                previous_cartridge_state_id: None,
                previous_cartridge_place_id: None,
            },
        )
        .await
        .expect("install with explicit place Y into printer already at X");

        assert_eq!(
            device_place_id(&svc, printer_id).await,
            Some(place_x),
            "обратная запись не должна перезаписывать уже существующее место принтера"
        );
    })
    .await
    .expect("install_with_explicit_place_does_not_override_printer_with_existing_place budget")
}

// ---------------------------------------------------------------------------
// operation_default_place — Plan 40-30 (HST-01, UAT3-01)
// ---------------------------------------------------------------------------

/// Регрессионный тест на ФАКТИЧЕСКОЕ (не идеализированное) поведение
/// `from_refill`-ветки `operation_default_place`: она переиспользует
/// `last_known_storage_place_in_tx` (CR-02, план 40-28), а этот резолвер
/// смотрит на самое СВЕЖЕЕ движение, затрагивающее складское место — не
/// обязательно место "до отправки на заправку". Поле «Место» картриджа
/// редактируется через `CartridgeService::update` без гейта по статусу, и
/// пункт меню «Редактировать» доступен во всех статусах
/// (`CartridgeContextMenu.svelte`), включая «На заправке». Если оператор
/// вручную поменяет место, пока картридж числится на заправке, дефолт
/// НЕ должен отразить эту правку — дефолт `from_refill` привязан к движению
/// `ToRefill` (UAT3-01a, gap-closure round 3), а ручное редактирование места
/// пишет отдельное движение без `TO_REFILL_MOVEMENT_NOTE`, значит на дефолт
/// не влияет.
///
/// Раунд 2 (план 40-30) закреплял ПРОТИВОПОЛОЖНОЕ поведение этим же тестом
/// (тогда — `..._reflects_manual_edit_during_refill`, дефолт совпадал с
/// правкой B), потому что резолвер `from_refill` тогда переиспользовал
/// `last_known_storage_place_in_tx` — "последнее известное складское место"
/// вообще, без привязки к конкретному переходу `ToRefill`. Живой UAT
/// (UAT3-01a) показал, что это была не просто другая, а НЕВЕРНАЯ трактовка
/// вопроса «место до отправки на заправку»: тот же резолвер на месте
/// заправки, помеченном `is_storage = 1`, возвращал саму заправку. Новый
/// резолвер `place_before_last_to_refill` отвечает только на вопрос "куда
/// был отправлен ПОСЛЕДНИЙ ToRefill" и потому игнорирует более поздние
/// ручные правки места, не связанные с самой отправкой на заправку — это
/// осознанно более узкое и корректное поведение для этого дефолта.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn operation_default_place_from_refill_ignores_manual_edit_during_refill() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_cartridge_service();
        let model_id = seed_model(&svc).await;
        let place_a = seed_storage_place(&svc, "Склад А").await;
        let place_b = seed_storage_place(&svc, "Склад Б").await;
        let place_r = seed_place(&svc, "Заправка").await;

        let cart = create_stock_cartridge(&svc, model_id).await;

        // 1. Место картриджа -> A (обычный ручной edit, картридж на складе).
        let after_a = svc
            .update(&admin_caller(), cart.id, cart.version, Some(place_a), None)
            .await
            .expect("update place to A");

        // 2. Отправить на заправку с местом заправки R — статус становится
        //    3 «На заправке», место R.
        let after_to_refill = svc
            .transition(
                &admin_caller(),
                CartridgeTransitionPayload::ToRefill {
                    cartridge_id: cart.id,
                    version: after_a.version,
                    date_utc: 1_700_000_000,
                    given_by_name: "Иванов".into(),
                    given_to_name: "Петров".into(),
                    place_id: Some(place_r),
                },
            )
            .await
            .expect("transition to_refill");
        assert_eq!(
            after_to_refill.status_id, 3,
            "картридж должен быть На заправке"
        );
        assert_eq!(after_to_refill.place_id, Some(place_r));

        // 3. Картридж ВСЁ ЕЩЁ «На заправке» (никакого перехода статуса) —
        //    ручное редактирование места через ту же форму, что доступна из
        //    «Редактировать» в любом статусе. Место становится B.
        let after_manual_edit = svc
            .update(
                &admin_caller(),
                cart.id,
                after_to_refill.version,
                Some(place_b),
                None,
            )
            .await
            .expect("manual place edit while On Refill");
        assert_eq!(
            after_manual_edit.status_id, 3,
            "статус не должен меняться от update()"
        );
        assert_eq!(after_manual_edit.place_id, Some(place_b));

        // 4. Дефолт from_refill должен вернуть A (место до отправки на
        //    заправку, зафиксированное самим движением ToRefill), а НЕ B
        //    (более поздний ручной edit места, не связанный с ToRefill).
        let default_place = svc
            .operation_default_place("from_refill", Some(cart.id))
            .await
            .expect("operation_default_place from_refill");
        assert_eq!(
            default_place,
            Some(place_a),
            "дефолт должен отражать место до отправки на заправку (A) из движения ToRefill, \
             а не более поздний несвязанный ручной edit (B)"
        );
    })
    .await
    .expect("operation_default_place_from_refill_ignores_manual_edit_during_refill budget")
}

/// `from_refill` дефолт через реальный поток `CartridgeService` — без
/// ручного посева `place_movements`. Картридж создаётся на складском месте
/// A, отправляется на заправку в НЕ-складское место, затем дефолт должен
/// вернуть A (единственное складское место в истории картриджа).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn operation_default_place_from_refill_resolves_via_real_service_flow() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_cartridge_service();
        let model_id = seed_model(&svc).await;
        let place_a = seed_storage_place(&svc, "Склад А").await;
        let refill_place = seed_place(&svc, "Заправка").await;

        let cart = svc
            .create(CartridgeCreateDto {
                model_id,
                code_override: None,
                state_id: Some(1),
                place_id: Some(place_a),
                notes: None,
            })
            .await
            .expect("create cartridge at storage A");

        let after_to_refill = svc
            .transition(
                &admin_caller(),
                CartridgeTransitionPayload::ToRefill {
                    cartridge_id: cart.id,
                    version: cart.version,
                    date_utc: 1_700_000_000,
                    given_by_name: "Иванов".into(),
                    given_to_name: "Петров".into(),
                    place_id: Some(refill_place),
                },
            )
            .await
            .expect("transition to_refill");
        assert_eq!(after_to_refill.place_id, Some(refill_place));

        let default_place = svc
            .operation_default_place("from_refill", Some(cart.id))
            .await
            .expect("operation_default_place from_refill");
        assert_eq!(
            default_place,
            Some(place_a),
            "должен вернуться единственный кандидат — складское место A, откуда отправили на заправку"
        );
    })
    .await
    .expect("operation_default_place_from_refill_resolves_via_real_service_flow budget")
}

/// UAT3-01a (gap-closure round 3, живой UAT): регрессия ровно на сценарии,
/// который провалился вживую — место заправки САМО помечено `is_storage = 1`
/// (естественно, если картриджи физически лежат в пункте заправки; ничем в
/// UI не запрещено). Картридж лежит на складе A, отправляется на заправку R
/// (тоже складское место), дефолт `from_refill` обязан вернуть A (место ДО
/// отправки), а НЕ R (саму заправку). Через реальный поток
/// `CartridgeService`, без ручного посева `place_movements` сырым SQL —
/// раунд 1 этой фазы провалился ровно на таком посеве: тест был зелёный, а
/// дефект жил.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn operation_default_place_from_refill_prefers_pre_refill_place_when_refill_place_is_storage_too(
) {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_cartridge_service();
        let model_id = seed_model(&svc).await;
        let place_a = seed_storage_place(&svc, "Склад А").await;
        // Место заправки помечено is_storage = 1 — ровно ветка, которая
        // провалилась вживую (UAT3-01a).
        let refill_place = seed_storage_place(&svc, "Заправка (склад)").await;

        let cart = svc
            .create(CartridgeCreateDto {
                model_id,
                code_override: None,
                state_id: Some(1),
                place_id: Some(place_a),
                notes: None,
            })
            .await
            .expect("create cartridge at storage A");

        let after_to_refill = svc
            .transition(
                &admin_caller(),
                CartridgeTransitionPayload::ToRefill {
                    cartridge_id: cart.id,
                    version: cart.version,
                    date_utc: 1_700_000_000,
                    given_by_name: "Иванов".into(),
                    given_to_name: "Петров".into(),
                    place_id: Some(refill_place),
                },
            )
            .await
            .expect("transition to_refill");
        assert_eq!(after_to_refill.place_id, Some(refill_place));

        let default_place = svc
            .operation_default_place("from_refill", Some(cart.id))
            .await
            .expect("operation_default_place from_refill");
        assert_eq!(
            default_place,
            Some(place_a),
            "дефолт должен вернуть место ДО отправки на заправку (A), а не саму заправку (R), \
             даже если место заправки тоже помечено is_storage = 1 (UAT3-01a)"
        );
    })
    .await
    .expect(
        "operation_default_place_from_refill_prefers_pre_refill_place_when_refill_place_is_storage_too budget",
    )
}

/// `to_refill` дефолт через реальный поток `CartridgeService` — без ручного
/// посева `place_movements`. Два картриджа отправлены на заправку с целевым
/// местом B, третий — с целевым местом C; самое частое место (B, 2>1)
/// должно победить.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn operation_default_place_to_refill_resolves_via_real_service_flow() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_cartridge_service();
        let model_id = seed_model(&svc).await;
        let place_source = seed_storage_place(&svc, "Склад источник").await;
        let place_b = seed_place(&svc, "Заправка Б").await;
        let place_c = seed_place(&svc, "Заправка В").await;

        for target in [place_b, place_b, place_c] {
            let cart = svc
                .create(CartridgeCreateDto {
                    model_id,
                    code_override: None,
                    state_id: Some(1),
                    place_id: Some(place_source),
                    notes: None,
                })
                .await
                .expect("create cartridge at source storage");

            svc.transition(
                &admin_caller(),
                CartridgeTransitionPayload::ToRefill {
                    cartridge_id: cart.id,
                    version: cart.version,
                    date_utc: 1_700_000_000,
                    given_by_name: "Иванов".into(),
                    given_to_name: "Петров".into(),
                    place_id: Some(target),
                },
            )
            .await
            .expect("transition to_refill");
        }

        let default_place = svc
            .operation_default_place("to_refill", None)
            .await
            .expect("operation_default_place to_refill");
        assert_eq!(
            default_place,
            Some(place_b),
            "B получил 2 отправки против 1 у C — B должен победить"
        );
    })
    .await
    .expect("operation_default_place_to_refill_resolves_via_real_service_flow budget")
}
