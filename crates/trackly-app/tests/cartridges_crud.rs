//! Cartridge CRUD integration tests — Plan 04-03 (GREEN phase).
//!
//! Covers: create + auto-code, custom code, get-404, soft-delete, counts,
//! rejects_invalid_custom_code (empty or >32 chars or ctrl chars → AppError::Validation).
//!
//! Plan 12-05 originally added `printer_compatib*` tests covering a
//! per-device printer/model junction table (D-11/D-12/D-13/D-14, GAP-12-02).
//! Plan 13-01/13-02 (V032) replaced that junction table with a single
//! `cartridge_model_compatibility.printer_name` column matched
//! case-insensitively against `devices.name` — the tests below were updated
//! to seed compatibility via `CartridgeService::model_create`'s
//! `compatibility: Vec<String>` field instead of the removed junction-table
//! repository methods. Plan 13-05 replaced the leftover round-trip test
//! (which still exercised those removed methods) with
//! `printer_compatib_case_insensitive_match`, covering the case/whitespace
//! comparison semantics (D-03) the round-trip test never verified.

use std::sync::Arc;
use std::time::Duration;

use trackly_core::auth::{Identity, Role};
use trackly_core::error::AppError;
use trackly_core::primitives::clock::Clock;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::db::writer_worker::WriterHandle;
use trackly_infra::test_support::test_writer_and_readers;

use trackly_app::dto::cartridge::{CartridgeCreateDto, CartridgeFilter, Pagination};
use trackly_app::services::CartridgeService;

/// `Identity::trusted_admin()` — desktop unlocked mode (D-Desktop-01),
/// `user_id: None`. Used for pre-existing call sites that don't assert on
/// `audit_log.user_id`.
fn admin_caller() -> Identity {
    Identity::trusted_admin()
}

/// Seed a real `users` row (FK target for `audit_log.user_id`) and return its
/// id. Invented name — privacy gate (CLAUDE.md hard constraint, no real ФИО).
async fn seed_manager_user(writer: &WriterHandle) -> i64 {
    let now = SystemClock.unix_seconds();
    writer
        .execute(move |conn| {
            let tx = conn.transaction().map_err(|e| AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            tx.execute(
                "INSERT INTO users \
                 (login, full_name, password_hash, role, ad_user, is_active, \
                  created_at_utc, updated_at_utc, version) \
                 VALUES ('sidorov.ss', 'Сидоров С.С.', NULL, 'manager', 0, 1, ?1, ?1, 1)",
                rusqlite::params![now],
            )
            .map_err(|e| AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            let user_id = tx.last_insert_rowid();
            tx.commit().map_err(|e| AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            Ok(user_id)
        })
        .await
        .expect("seed manager user")
}

/// Set up a fresh CartridgeService backed by an in-memory migrated DB.
fn make_cartridge_service() -> (CartridgeService, tempfile::TempDir) {
    let (writer, readers, dir) = test_writer_and_readers();
    let clock = Arc::new(SystemClock);
    let svc = CartridgeService::new(writer, readers, clock);
    (svc, dir)
}

/// Seed a cartridge_model and return its id.
async fn seed_model(svc: &CartridgeService) -> i64 {
    let model = svc
        .model_create(trackly_app::dto::cartridge::CartridgeModelCreateDto {
            brand: "Pantum".into(),
            model: "TL-5120X".into(),
            kind_id: 1,
            color: Some("Чёрный".into()),
            notes: None,
            compatibility: vec![],
        })
        .await
        .expect("seed_model");
    model.id
}

/// Set up a fresh CartridgeService PLUS raw writer/readers handles, for tests
/// that need to seed a `devices` row (type_id=2, Принтер) directly
/// (`printer_compatib*` tests).
fn make_cartridge_service_with_handles() -> (
    CartridgeService,
    Arc<trackly_infra::db::writer_worker::WriterHandle>,
    Arc<trackly_infra::db::pools::ReaderPool>,
    tempfile::TempDir,
) {
    let (writer, readers, dir) = test_writer_and_readers();
    let clock = Arc::new(SystemClock);
    let svc = CartridgeService::new(writer.clone(), readers.clone(), clock);
    (svc, writer, readers, dir)
}

/// Seed a printer device (devices.type_id=2) and return its id.
async fn seed_printer_device(
    writer: &Arc<trackly_infra::db::writer_worker::WriterHandle>,
    name: &str,
) -> i64 {
    let name = name.to_string();
    writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO devices (type_id, name, status_id, created_at_utc, updated_at_utc, version) \
                 VALUES (2, ?1, 1, 1700000000, 1700000000, 1)",
                rusqlite::params![name],
            )
            .map_err(trackly_infra::error_conversions::map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed_printer_device")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_cartridge_assigns_auto_code() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_cartridge_service();
        let model_id = seed_model(&svc).await;

        let dto = svc
            .create(CartridgeCreateDto {
                model_id,
                code_override: None,
                state_id: Some(1),
                place_id: None,
                notes: None,
            })
            .await
            .expect("create auto");

        // Code must start with "C-" and be unique.
        assert!(
            dto.code.starts_with("C-"),
            "auto-code must start with C-: {}",
            dto.code
        );
        assert_eq!(dto.model_id, model_id);
        assert_eq!(dto.status_id, 1); // На складе
    })
    .await
    .expect("create_cartridge_assigns_auto_code budget")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_cartridge_custom_code() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_cartridge_service();
        let model_id = seed_model(&svc).await;

        let dto = svc
            .create(CartridgeCreateDto {
                model_id,
                code_override: Some("BARCODE-42".into()),
                state_id: None,
                place_id: None,
                notes: None,
            })
            .await
            .expect("create custom");

        assert_eq!(dto.code, "BARCODE-42");
    })
    .await
    .expect("create_cartridge_custom_code budget")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn get_returns_404_for_missing() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_cartridge_service();
        let err = svc.get(99999).await.expect_err("should be NotFound");
        assert!(
            matches!(err, AppError::NotFound { .. }),
            "expected NotFound, got {:?}",
            err
        );
    })
    .await
    .expect("get_returns_404_for_missing budget")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn soft_delete_hides_item() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_cartridge_service();
        let model_id = seed_model(&svc).await;

        let dto = svc
            .create(CartridgeCreateDto {
                model_id,
                code_override: None,
                state_id: None,
                place_id: None,
                notes: None,
            })
            .await
            .expect("create");

        svc.delete(dto.id, dto.version).await.expect("delete");

        // After soft-delete, get should return NotFound.
        let err = svc.get(dto.id).await.expect_err("should be hidden");
        assert!(
            matches!(err, AppError::NotFound { .. }),
            "expected NotFound after delete, got {:?}",
            err
        );
    })
    .await
    .expect("soft_delete_hides_item budget")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn counts_by_status() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_cartridge_service();
        let model_id = seed_model(&svc).await;

        // Create 2 cartridges on-stock.
        for _ in 0..2 {
            svc.create(CartridgeCreateDto {
                model_id,
                code_override: None,
                state_id: Some(1),
                place_id: None,
                notes: None,
            })
            .await
            .expect("create");
        }

        let counts = svc.status_counts().await.expect("counts");
        assert_eq!(counts.all, 2);
        assert_eq!(counts.in_stock, 2);
        assert_eq!(counts.in_use, 0);
    })
    .await
    .expect("counts_by_status budget")
}

/// Verify that create with an empty code_override, one longer than 32 chars,
/// or one containing a control character returns AppError::Validation.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn rejects_invalid_custom_code() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_cartridge_service();
        let model_id = seed_model(&svc).await;

        // (a) empty string
        let result = svc
            .create(CartridgeCreateDto {
                model_id,
                code_override: Some("".into()),
                ..Default::default()
            })
            .await;
        assert!(
            matches!(result, Err(AppError::Validation { ref field, .. }) if field == "code_override"),
            "empty string must return Validation(code_override), got: {:?}",
            result
        );

        // (b) string longer than 32 chars (33 x's)
        let result = svc
            .create(CartridgeCreateDto {
                model_id,
                code_override: Some("x".repeat(33)),
                ..Default::default()
            })
            .await;
        assert!(
            matches!(result, Err(AppError::Validation { ref field, .. }) if field == "code_override"),
            ">32 chars must return Validation(code_override), got: {:?}",
            result
        );

        // (c) string with a control character (tab = U+0009 < U+0020)
        let result = svc
            .create(CartridgeCreateDto {
                model_id,
                code_override: Some("C\x09ode".into()),
                ..Default::default()
            })
            .await;
        assert!(
            matches!(result, Err(AppError::Validation { ref field, .. }) if field == "code_override"),
            "ctrl char must return Validation(code_override), got: {:?}",
            result
        );
    })
    .await
    .expect("rejects_invalid_custom_code budget")
}

// ---------------------------------------------------------------------------
// Plan 12-05 (rewired by 13-01/13-02, V032): single-column printer-name
// compatibility (D-11..D-14), matched case-insensitively against the
// printer device's `devices.name`.
// ---------------------------------------------------------------------------

/// Test 1: cartridges.list() with compatible_with_printer_device_id narrows
/// to the linked model when a link exists (D-13). Model B has a
/// *non-matching* compatibility entry (not an empty list — an empty list
/// would pass through unfiltered per D-05) so it is properly excluded.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn printer_compatib_list_narrows_to_linked_model() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, writer, _readers, _dir) = make_cartridge_service_with_handles();
        let printer_name = "Принтер тест";
        let device_id = seed_printer_device(&writer, printer_name).await;

        let model_a = svc
            .model_create(trackly_app::dto::cartridge::CartridgeModelCreateDto {
                brand: "Pantum".into(),
                model: "TL-5120X".into(),
                kind_id: 1,
                color: Some("Чёрный".into()),
                notes: None,
                compatibility: vec![printer_name.to_string()],
            })
            .await
            .expect("seed model A")
            .id;
        let model_b = svc
            .model_create(trackly_app::dto::cartridge::CartridgeModelCreateDto {
                brand: "Kyocera".into(),
                model: "TK-1200".into(),
                kind_id: 1,
                color: Some("Чёрный".into()),
                notes: None,
                compatibility: vec!["Другой принтер".into()],
            })
            .await
            .expect("seed model B")
            .id;

        // In-stock, full-charge cartridges for both models.
        svc.create(CartridgeCreateDto {
            model_id: model_a,
            code_override: None,
            state_id: Some(1),
            place_id: None,
            notes: None,
        })
        .await
        .expect("create cartridge A");
        svc.create(CartridgeCreateDto {
            model_id: model_b,
            code_override: None,
            state_id: Some(1),
            place_id: None,
            notes: None,
        })
        .await
        .expect("create cartridge B");

        let resp = svc
            .list(
                CartridgeFilter {
                    kind_id: Some(1),
                    installable_only: true,
                    compatible_with_printer_device_id: Some(device_id),
                    ..Default::default()
                },
                Pagination::default(),
            )
            .await
            .expect("list with compatibility filter");

        assert_eq!(
            resp.items.len(),
            1,
            "only model A's cartridge should be returned, got: {:?}",
            resp.items
        );
        assert_eq!(resp.items[0].model_id, model_a);
    })
    .await
    .expect("printer_compatib_list_narrows_to_linked_model budget")
}

/// Test 2: zero links configured for the device → list() with
/// compatible_with_printer_device_id returns ALL kind_id=1 in-stock
/// cartridges unfiltered (D-14 "not configured = no narrowing").
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn printer_compatib_unconfigured_device_does_not_narrow() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, writer, _readers, _dir) = make_cartridge_service_with_handles();
        let device_id = seed_printer_device(&writer, "Принтер без связей").await;

        let model_a = seed_model(&svc).await;
        let model_b = svc
            .model_create(trackly_app::dto::cartridge::CartridgeModelCreateDto {
                brand: "Kyocera".into(),
                model: "TK-1200".into(),
                kind_id: 1,
                color: Some("Чёрный".into()),
                notes: None,
                compatibility: vec![],
            })
            .await
            .expect("seed model B")
            .id;

        svc.create(CartridgeCreateDto {
            model_id: model_a,
            code_override: None,
            state_id: Some(1),
            place_id: None,
            notes: None,
        })
        .await
        .expect("create cartridge A");
        svc.create(CartridgeCreateDto {
            model_id: model_b,
            code_override: None,
            state_id: Some(1),
            place_id: None,
            notes: None,
        })
        .await
        .expect("create cartridge B");

        // No links inserted for device_id at all.

        let filtered = svc
            .list(
                CartridgeFilter {
                    kind_id: Some(1),
                    installable_only: true,
                    compatible_with_printer_device_id: Some(device_id),
                    ..Default::default()
                },
                Pagination::default(),
            )
            .await
            .expect("list with unconfigured compatibility filter");

        let unfiltered = svc
            .list(
                CartridgeFilter {
                    kind_id: Some(1),
                    installable_only: true,
                    compatible_with_printer_device_id: None,
                    ..Default::default()
                },
                Pagination::default(),
            )
            .await
            .expect("list without compatibility filter");

        assert_eq!(
            filtered.items.len(),
            unfiltered.items.len(),
            "unconfigured device must not narrow the result set (D-14)"
        );
        assert_eq!(filtered.total, unfiltered.total);
        assert_eq!(filtered.items.len(), 2, "both models' cartridges expected");
    })
    .await
    .expect("printer_compatib_unconfigured_device_does_not_narrow budget")
}

/// Test 3 (Plan 13-05): seeding a model's compatibility with a printer name
/// that differs from the linked device's `devices.name` only by case and
/// surrounding whitespace still narrows the cartridge-list filter to that
/// model — confirms the case-insensitive + TRIM comparison (D-03).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn printer_compatib_case_insensitive_match() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, writer, _readers, _dir) = make_cartridge_service_with_handles();
        let device_id = seed_printer_device(&writer, "HP LaserJet M404").await;

        let model_a = svc
            .model_create(trackly_app::dto::cartridge::CartridgeModelCreateDto {
                brand: "HP".into(),
                model: "CF258A".into(),
                kind_id: 1,
                color: Some("Чёрный".into()),
                notes: None,
                // Different case + leading/trailing whitespace vs. the
                // device's exact name "HP LaserJet M404" — D-03 normalises
                // via LOWER(TRIM(...)) on both sides, which strips
                // leading/trailing whitespace and folds case, but does NOT
                // collapse interior whitespace runs.
                compatibility: vec!["  hp laserjet m404  ".into()],
            })
            .await
            .expect("seed model A")
            .id;

        svc.create(CartridgeCreateDto {
            model_id: model_a,
            code_override: None,
            state_id: Some(1),
            place_id: None,
            notes: None,
        })
        .await
        .expect("create cartridge A");

        let resp = svc
            .list(
                CartridgeFilter {
                    kind_id: Some(1),
                    installable_only: true,
                    compatible_with_printer_device_id: Some(device_id),
                    ..Default::default()
                },
                Pagination::default(),
            )
            .await
            .expect("list with compatibility filter");

        assert_eq!(
            resp.items.len(),
            1,
            "case/whitespace-insensitive match must still narrow to model A, got: {:?}",
            resp.items
        );
        assert_eq!(resp.items[0].model_id, model_a);
    })
    .await
    .expect("printer_compatib_case_insensitive_match budget")
}

/// Seed a device with an explicit `type_id` and name (covers non-printer rows
/// and duplicate / soft-deleted printers that `seed_printer_device` cannot
/// express). Mirrors `seed_printer_device`'s INSERT shape.
async fn seed_device_typed(
    writer: &Arc<trackly_infra::db::writer_worker::WriterHandle>,
    type_id: i64,
    name: &str,
    deleted: bool,
) -> i64 {
    let name = name.to_string();
    writer
        .execute(move |conn| {
            let deleted_at: Option<i64> = if deleted { Some(1_700_000_000) } else { None };
            conn.execute(
                "INSERT INTO devices (type_id, name, status_id, created_at_utc, updated_at_utc, version, deleted_at_utc) \
                 VALUES (?1, ?2, 1, 1700000000, 1700000000, 1, ?3)",
                rusqlite::params![type_id, name, deleted_at],
            )
            .map_err(trackly_infra::error_conversions::map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed_device_typed")
}

/// GAP-1 (SPEC-13-R3): `suggest_compat_printer` autocomplete sources DISTINCT
/// names strictly from live printer devices (type_id=2, not soft-deleted),
/// prefix-matched. A non-printer device with a matching name must NOT appear,
/// duplicate printer names collapse to one entry, and soft-deleted printers
/// are excluded.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn suggest_compat_printer_returns_distinct_printer_names() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, writer, _readers, _dir) = make_cartridge_service_with_handles();

        // Two DISTINCT-collapsible duplicate printers (same name, type_id=2).
        seed_device_typed(&writer, 2, "HP LaserJet M404", false).await;
        seed_device_typed(&writer, 2, "HP LaserJet M404", false).await;
        // A second, distinct matching printer.
        seed_device_typed(&writer, 2, "HP LaserJet M507", false).await;
        // A non-printer device whose name matches the prefix — must be excluded.
        seed_device_typed(&writer, 1, "HP Switch HX100", false).await;
        // A soft-deleted printer matching the prefix — must be excluded.
        seed_device_typed(&writer, 2, "HP DeletedJet", true).await;
        // A live printer that does NOT match the prefix — must be excluded.
        seed_device_typed(&writer, 2, "Canon i-SENSYS", false).await;

        let got = svc
            .suggest_compat_printer("HP ".into())
            .await
            .expect("suggest_compat_printer");

        // Only live printer (type_id=2) names matching the prefix, DISTINCT,
        // ordered by name ASC.
        assert_eq!(
            got,
            vec![
                "HP LaserJet M404".to_string(),
                "HP LaserJet M507".to_string(),
            ],
            "expected only DISTINCT live-printer names matching prefix 'HP '; got {got:?}"
        );

        // Non-matching / empty-result prefix returns an empty set.
        let none = svc
            .suggest_compat_printer("Zzz".into())
            .await
            .expect("suggest_compat_printer non-matching");
        assert!(
            none.is_empty(),
            "non-matching prefix must return empty, got {none:?}"
        );
    })
    .await
    .expect("suggest_compat_printer_returns_distinct_printer_names budget")
}

// ---------------------------------------------------------------------------
// update — caller threading + before-fetch (Plan 40-04, Pitfall 1/2)
// ---------------------------------------------------------------------------

/// Seed a real `places` row — FK-valid `place_id` (V038 `REFERENCES places(id)`).
async fn seed_place(svc: &CartridgeService, name: &str) -> i64 {
    let name = name.to_string();
    svc.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO places (kind, name, is_storage, created_at_utc, updated_at_utc, version) \
                 VALUES ('room', ?1, 0, ?2, ?2, 1)",
                rusqlite::params![name, 1_700_000_000_i64],
            )
            .map_err(|e| AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed place")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn update_stores_real_caller_user_id_in_audit_log() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_cartridge_service();
        let model_id = seed_model(&svc).await;
        let place_a = seed_place(&svc, "Склад").await;
        let place_b = seed_place(&svc, "Каб. 101").await;

        let dto = svc
            .create(CartridgeCreateDto {
                model_id,
                code_override: None,
                state_id: None,
                place_id: Some(place_a),
                notes: None,
            })
            .await
            .expect("create cartridge");

        let manager_user_id = seed_manager_user(&svc.writer).await;
        let manager = Identity {
            user_id: Some(manager_user_id),
            role: Role::Manager,
        };

        // Real before/after place_id change — exercises the before-fetch
        // (Pitfall 2) this task adds alongside caller threading.
        svc.update(&manager, dto.id, dto.version, Some(place_b), None)
            .await
            .expect("update with manager caller");

        // audit_log.user_id должен равняться caller.user_id — реальный
        // менеджер, не хардкод NULL (Pitfall 1).
        let readers = svc.readers.clone();
        let entity_id = dto.id;
        let user_id: Option<i64> = tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            conn.query_row(
                "SELECT user_id FROM audit_log WHERE entity_type='cartridge' AND entity_id=?1 AND action='update'",
                rusqlite::params![entity_id],
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
    .expect("update_stores_real_caller_user_id_in_audit_log exceeded 30 s budget");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn update_with_trusted_admin_caller_stores_null_user_id() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_cartridge_service();
        let model_id = seed_model(&svc).await;
        let dto = svc
            .create(CartridgeCreateDto {
                model_id,
                code_override: None,
                state_id: None,
                place_id: None,
                notes: None,
            })
            .await
            .expect("create cartridge");

        let admin = admin_caller();
        svc.update(&admin, dto.id, dto.version, None, Some("заметка".into()))
            .await
            .expect("update with admin caller");

        let readers = svc.readers.clone();
        let entity_id = dto.id;
        let user_id: Option<i64> = tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            conn.query_row(
                "SELECT user_id FROM audit_log WHERE entity_type='cartridge' AND entity_id=?1 AND action='update'",
                rusqlite::params![entity_id],
                |r| r.get(0),
            )
        })
        .await
        .expect("spawn_blocking")
        .expect("query audit_log user_id");

        assert_eq!(
            user_id, None,
            "trusted_admin caller has no user_id — audit_log.user_id остаётся NULL (unchanged behavior)"
        );
    })
    .await
    .expect("update_with_trusted_admin_caller_stores_null_user_id exceeded 30 s budget");
}
