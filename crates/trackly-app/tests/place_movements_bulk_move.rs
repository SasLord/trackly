//! Wave 0 integration coverage: D-28 "Перенести всё содержимое в…" bulk move
//! (Plan 40-13, HST-01, seventh `place_movements` write site).
//!
//! Covers:
//! - a successful multi-item move (2 devices — one a "printer" kind, one a
//!   plain device — plus 1 cartridge, all nested under the root) relocates
//!   every item to the target place and records exactly one `place_movements`
//!   row per item, all inside one transaction, `source='manual'`
//! - `Action::MutateDevices` + `Action::MutateCartridges` both gate the call
//!   (D-13): an Employee identity is rejected on BOTH the Tauri path
//!   (`build_places_move_subtree_contents` → `Err(AppError::Forbidden)`) and
//!   the real HTTP path (`POST /api/v1/places_move_subtree_contents` with an
//!   Employee session cookie → `403 Forbidden`) — before any row is touched
//! - atomicity-on-failure: a `BEFORE UPDATE` trigger injects a simulated
//!   failure on the LAST item in the walk order (a cartridge — `device` rows
//!   are unioned before `cartridge` rows in `list_subtree_contents`), proving
//!   the whole call rolls back, not just the failing item (mirrors the WR-05
//!   lesson from Phase 39.2 — atomicity proven by fault injection, not
//!   declared in a comment)
//!
//! Harness mirrors `role_endpoint_matrix.rs` (full `AppCtx` + real HTTP
//! router + programmatic session cookie) for the role-gate cases, and
//! `place_movements_write_sites_devices.rs` / `_cartridges.rs` (direct
//! service calls, raw-SQL seeding) for the success/atomicity cases — real
//! tempfile SQLite DB, invented place/device/cartridge names only (CLAUDE.md
//! privacy gate).

use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use rusqlite::params;
use serde_json::json;
use time::{Duration as TimeDuration, OffsetDateTime};
use tower::ServiceExt;
use tower_sessions::session::{Id, Record};
use tower_sessions::SessionStore;

use trackly_app::context::AppCtx;
use trackly_app::dto::auth::UserNew;
use trackly_app::dto::cartridge::{CartridgeCreateDto, CartridgeModelCreateDto};
use trackly_app::http::auth::SessionIdentity;
use trackly_app::http::build_router;
use trackly_app::server::rusqlite_session_store::RusqliteSessionStore;
use trackly_app::tauri_cmds::places::build_places_move_subtree_contents;
use trackly_core::auth::{Identity, Role};
use trackly_core::error::AppError;
use trackly_infra::error_conversions::map_rusqlite;

/// Построить тестовый `AppCtx` — клон `role_endpoint_matrix.rs::make_test_ctx`.
async fn make_test_ctx() -> anyhow::Result<(AppCtx, tempfile::TempDir)> {
    let dir = tempfile::TempDir::new()?;
    let dir_path = dir.path().to_path_buf();
    let paths = trackly_infra::Paths::resolve_for_exe_dir(dir_path)?;
    let config = trackly_infra::AppConfig::default();
    let log_guard = trackly_app::logging::init(&paths, &config).or_else(|_| {
        let (_nb, guard) = tracing_appender::non_blocking(std::io::sink());
        Ok::<_, anyhow::Error>(guard)
    })?;
    let ctx = AppCtx::build(paths, config, log_guard).await?;
    Ok((ctx, dir))
}

/// Создаёт реального пользователя (FK-цель `place_movements.user_id`) и
/// возвращает `Identity`. Вымышленное имя (CLAUDE.md privacy gate).
async fn create_identity(ctx: &AppCtx, login: &str, full_name: &str, role: Role) -> Identity {
    let role_str = match role {
        Role::Admin => "admin",
        Role::Manager => "manager",
        Role::Employee => "employee",
    };
    let dto = ctx
        .auth
        .create_user(
            UserNew {
                login: login.to_string(),
                full_name: full_name.to_string(),
                password: "password123".to_string(),
                role: role_str.to_string(),
                email: None,
            },
            &Identity::trusted_admin(),
        )
        .await
        .expect("create test user");
    Identity {
        user_id: Some(dto.id),
        role,
    }
}

/// Сеет строку `places` напрямую. Invented names only.
async fn seed_place(ctx: &AppCtx, name: &str) -> i64 {
    let name = name.to_string();
    ctx.writer
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

/// Сеет нижнее место с `parent_id` — для проверки, что `move_subtree_contents`
/// walks the NESTED subtree (nested=true, D-28 "и всех вложенных местах"),
/// not just the root.
async fn seed_child_place(ctx: &AppCtx, name: &str, parent_id: i64) -> i64 {
    let name = name.to_string();
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO places (kind, name, parent_id, is_storage, created_at_utc, updated_at_utc, version) \
                 VALUES ('room', ?1, ?2, 0, ?3, ?3, 1)",
                params![name, parent_id, 1_700_000_000_i64],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed child place")
}

/// Сеет устройство (`type_id=1`, обычное устройство) в заданном месте.
async fn seed_device_at_place(ctx: &AppCtx, name: &str, place_id: i64) -> i64 {
    let name = name.to_string();
    ctx.writer
        .execute(move |conn| {
            let tx = conn.transaction().map_err(map_rusqlite)?;
            tx.execute(
                "INSERT INTO devices \
                 (type_id, name, status_id, place_id, version, created_at_utc, updated_at_utc) \
                 VALUES (1, ?1, 1, ?2, 1, ?3, ?3)",
                params![name, place_id, 1_700_000_000_i64],
            )
            .map_err(map_rusqlite)?;
            let id = tx.last_insert_rowid();
            tx.commit().map_err(map_rusqlite)?;
            Ok(id)
        })
        .await
        .expect("seed device")
}

/// Сеет принтер (`type_id=2`) в заданном месте — `list_subtree_contents`
/// classifies this as `kind='printer'`, walked identically to `'device'`.
async fn seed_printer_at_place(ctx: &AppCtx, name: &str, place_id: i64) -> i64 {
    let name = name.to_string();
    ctx.writer
        .execute(move |conn| {
            let tx = conn.transaction().map_err(map_rusqlite)?;
            tx.execute(
                "INSERT INTO devices \
                 (type_id, name, status_id, place_id, version, created_at_utc, updated_at_utc) \
                 VALUES (2, ?1, 1, ?2, 1, ?3, ?3)",
                params![name, place_id, 1_700_000_000_i64],
            )
            .map_err(map_rusqlite)?;
            let id = tx.last_insert_rowid();
            tx.commit().map_err(map_rusqlite)?;
            Ok(id)
        })
        .await
        .expect("seed printer")
}

async fn seed_cartridge_model(ctx: &AppCtx) -> i64 {
    ctx.cartridges
        .model_create(CartridgeModelCreateDto {
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

async fn seed_cartridge_at_place(ctx: &AppCtx, model_id: i64, place_id: i64) -> i64 {
    ctx.cartridges
        .create(CartridgeCreateDto {
            model_id,
            code_override: None,
            state_id: Some(1),
            place_id: Some(place_id),
            notes: None,
        })
        .await
        .expect("seed cartridge")
        .id
}

async fn device_place_id(ctx: &AppCtx, device_id: i64) -> Option<i64> {
    let readers = ctx.readers.clone();
    tokio::task::spawn_blocking(move || {
        let conn = readers.acquire();
        conn.query_row(
            "SELECT place_id FROM devices WHERE id = ?1",
            params![device_id],
            |r| r.get::<_, Option<i64>>(0),
        )
    })
    .await
    .expect("spawn_blocking")
    .expect("query device place_id")
}

async fn cartridge_place_id(ctx: &AppCtx, cartridge_id: i64) -> Option<i64> {
    let readers = ctx.readers.clone();
    tokio::task::spawn_blocking(move || {
        let conn = readers.acquire();
        conn.query_row(
            "SELECT place_id FROM cartridges WHERE id = ?1",
            params![cartridge_id],
            |r| r.get::<_, Option<i64>>(0),
        )
    })
    .await
    .expect("spawn_blocking")
    .expect("query cartridge place_id")
}

async fn count_movements(ctx: &AppCtx, entity_type: &str, entity_id: i64) -> i64 {
    let readers = ctx.readers.clone();
    let entity_type = entity_type.to_string();
    tokio::task::spawn_blocking(move || {
        let conn = readers.acquire();
        conn.query_row(
            "SELECT COUNT(*) FROM place_movements WHERE entity_type=?1 AND entity_id=?2",
            params![entity_type, entity_id],
            |r| r.get(0),
        )
    })
    .await
    .expect("spawn_blocking")
    .expect("count place_movements")
}

/// Создать сессию программно в `RusqliteSessionStore`, вернуть cookie строку
/// (обходит `GovernorLayer` на `/auth_login`, недоступный peer IP в тестах —
/// клон `role_endpoint_matrix.rs::create_session_cookie`).
async fn create_session_cookie(
    store: &RusqliteSessionStore,
    user_id: i64,
    role: Role,
) -> anyhow::Result<String> {
    let session_id = Id::default();
    let si = SessionIdentity {
        user_id: Some(user_id),
        role: role.as_str().to_string(),
    };
    let mut record = Record {
        id: session_id,
        data: Default::default(),
        expiry_date: OffsetDateTime::now_utc() + TimeDuration::days(1),
    };
    record
        .data
        .insert("identity".to_string(), serde_json::to_value(&si)?);
    store.create(&mut record).await?;
    Ok(format!("id={session_id}"))
}

// ---------------------------------------------------------------------------
// place_movements_bulk_move_success
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn place_movements_bulk_move_success() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (ctx, _dir) = make_test_ctx().await.expect("make_test_ctx");
        let manager = create_identity(&ctx, "manager_bulk", "Петров П.П.", Role::Manager).await;

        let root = seed_place(&ctx, "Каб. 401 (переезд)").await;
        let child = seed_child_place(&ctx, "Каб. 401 / шкаф", root).await;
        let target = seed_place(&ctx, "Склад-2").await;

        // Root itself + nested child — D-28 "и всех вложенных местах".
        let device_id = seed_device_at_place(&ctx, "Ноутбук Dell", root).await;
        let printer_id = seed_printer_at_place(&ctx, "Kyocera ECOSYS", child).await;

        let model_id = seed_cartridge_model(&ctx).await;
        let cartridge_id = seed_cartridge_at_place(&ctx, model_id, child).await;

        let moved = ctx
            .places
            .move_subtree_contents(&manager, root, target, Some("Переезд кабинета".to_string()))
            .await
            .expect("move_subtree_contents succeeds");

        assert_eq!(moved, 3, "must report exactly 3 items moved");

        assert_eq!(device_place_id(&ctx, device_id).await, Some(target));
        assert_eq!(device_place_id(&ctx, printer_id).await, Some(target));
        assert_eq!(cartridge_place_id(&ctx, cartridge_id).await, Some(target));

        assert_eq!(count_movements(&ctx, "device", device_id).await, 1);
        assert_eq!(count_movements(&ctx, "device", printer_id).await, 1);
        assert_eq!(count_movements(&ctx, "cartridge", cartridge_id).await, 1);
    })
    .await
    .expect("place_movements_bulk_move_success exceeded 30 s budget");
}

// ---------------------------------------------------------------------------
// place_movements_bulk_move_already_at_target_skips_movement_row
// ---------------------------------------------------------------------------

/// D-04: an item already AT `target_place_id` still gets its (no-op) UPDATE
/// but `record_movement_if_applicable`'s own guard skips the movement row —
/// the overall call still succeeds and counts the item. `target` is nested
/// UNDER `root` so it is itself part of root's subtree walk (nested=true).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn place_movements_bulk_move_already_at_target_skips_movement_row() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (ctx, _dir) = make_test_ctx().await.expect("make_test_ctx");
        let manager =
            create_identity(&ctx, "manager_bulk_noop", "Сидоров С.С.", Role::Manager).await;

        let root = seed_place(&ctx, "Каб. 501").await;
        let target = seed_child_place(&ctx, "Каб. 501 / ниша", root).await;

        let device_at_root = seed_device_at_place(&ctx, "Монитор Samsung", root).await;
        let device_at_target = seed_device_at_place(&ctx, "Клавиатура", target).await;

        let moved = ctx
            .places
            .move_subtree_contents(&manager, root, target, None)
            .await
            .expect("move_subtree_contents succeeds");

        assert_eq!(moved, 2, "both items in root's subtree are counted");
        assert_eq!(device_place_id(&ctx, device_at_root).await, Some(target));
        assert_eq!(device_place_id(&ctx, device_at_target).await, Some(target));
        assert_eq!(
            count_movements(&ctx, "device", device_at_root).await,
            1,
            "device_at_root had a real place change -> one movement row"
        );
        assert_eq!(
            count_movements(&ctx, "device", device_at_target).await,
            0,
            "D-04: device_at_target's place never actually changed -> zero movement rows"
        );
    })
    .await
    .expect("place_movements_bulk_move_already_at_target_skips_movement_row exceeded 30 s budget");
}

// ---------------------------------------------------------------------------
// place_movements_bulk_move_employee_forbidden_tauri
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn place_movements_bulk_move_employee_forbidden_tauri() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (ctx, _dir) = make_test_ctx().await.expect("make_test_ctx");
        let employee =
            create_identity(&ctx, "employee_bulk", "Кузнецов К.К.", Role::Employee).await;

        let root = seed_place(&ctx, "Каб. 601").await;
        let target = seed_place(&ctx, "Склад-4").await;
        seed_device_at_place(&ctx, "Принтер HP", root).await;

        let result = build_places_move_subtree_contents(&ctx, &employee, root, target, None).await;
        assert!(
            matches!(result, Err(AppError::Forbidden)),
            "Employee must be denied bulk move (Tauri path): {result:?}"
        );

        // No row must have moved.
        assert_eq!(count_movements(&ctx, "device", root).await, 0);
    })
    .await
    .expect("place_movements_bulk_move_employee_forbidden_tauri exceeded 30 s budget");
}

// ---------------------------------------------------------------------------
// place_movements_bulk_move_employee_forbidden_http
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn place_movements_bulk_move_employee_forbidden_http() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (ctx, _dir) = make_test_ctx().await.expect("make_test_ctx");
        let employee_dto = ctx
            .auth
            .create_user(
                UserNew {
                    login: "employee_bulk_http".to_string(),
                    full_name: "Николаев Н.Н.".to_string(),
                    password: "password123".to_string(),
                    role: "employee".to_string(),
                    email: None,
                },
                &Identity::trusted_admin(),
            )
            .await
            .expect("create employee_bulk_http");

        let root = seed_place(&ctx, "Каб. 701").await;
        let target = seed_place(&ctx, "Склад-5").await;

        let session_store = RusqliteSessionStore::new(ctx.writer.clone(), ctx.readers.clone());
        let employee_cookie =
            create_session_cookie(&session_store, employee_dto.id, Role::Employee)
                .await
                .expect("create employee session");

        let app = build_router(&ctx, session_store);

        let body = json!({
            "rootId": root,
            "targetPlaceId": target,
            "note": null,
        });
        let req = Request::builder()
            .method("POST")
            .uri("/api/v1/places_move_subtree_contents")
            .header("content-type", "application/json")
            .header("cookie", employee_cookie)
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();

        let res = app.oneshot(req).await.unwrap();
        assert_eq!(
            res.status(),
            StatusCode::FORBIDDEN,
            "Employee session must be denied bulk move over HTTP"
        );
    })
    .await
    .expect("place_movements_bulk_move_employee_forbidden_http exceeded 30 s budget");
}

// ---------------------------------------------------------------------------
// place_movements_bulk_move_atomicity_on_failure
// ---------------------------------------------------------------------------

/// WR-05-style fault injection (Phase 39.2): a `BEFORE UPDATE` trigger raises
/// `ABORT` when the LAST item in the walk order (a cartridge — `device` rows
/// are unioned before `cartridge` rows by `list_subtree_contents`) is
/// updated. Proves the whole call rolls back — the device's already-applied
/// UPDATE + movement INSERT inside the SAME transaction are undone too, not
/// just the failing cartridge.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn place_movements_bulk_move_atomicity_on_failure() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (ctx, _dir) = make_test_ctx().await.expect("make_test_ctx");
        let manager =
            create_identity(&ctx, "manager_bulk_fail", "Смирнов С.А.", Role::Manager).await;

        let root = seed_place(&ctx, "Каб. 801").await;
        let target = seed_place(&ctx, "Склад-6").await;

        let device_id = seed_device_at_place(&ctx, "Системный блок", root).await;
        let model_id = seed_cartridge_model(&ctx).await;
        let cartridge_id = seed_cartridge_at_place(&ctx, model_id, root).await;

        // Inject a fault: any UPDATE touching this specific cartridge id aborts.
        ctx.writer
            .execute(move |conn| {
                conn.execute(
                    &format!(
                        "CREATE TRIGGER fail_bulk_move_cartridge \
                         BEFORE UPDATE ON cartridges \
                         WHEN NEW.id = {cartridge_id} \
                         BEGIN SELECT RAISE(ABORT, 'simulated bulk-move failure'); END;"
                    ),
                    [],
                )
                .map_err(map_rusqlite)?;
                Ok(())
            })
            .await
            .expect("install fault-injection trigger");

        let result = ctx
            .places
            .move_subtree_contents(&manager, root, target, None)
            .await;
        assert!(
            result.is_err(),
            "bulk move must fail when the cartridge UPDATE aborts: {result:?}"
        );

        // Atomicity: the device — processed and committed-within-tx BEFORE
        // the cartridge in the walk order — must have been rolled back too.
        assert_eq!(
            device_place_id(&ctx, device_id).await,
            Some(root),
            "device's UPDATE must be rolled back, not partially applied"
        );
        assert_eq!(
            cartridge_place_id(&ctx, cartridge_id).await,
            Some(root),
            "cartridge never moved (it's the one that failed)"
        );
        assert_eq!(
            count_movements(&ctx, "device", device_id).await,
            0,
            "device's movement INSERT must be rolled back — zero partial state"
        );
        assert_eq!(count_movements(&ctx, "cartridge", cartridge_id).await, 0);
    })
    .await
    .expect("place_movements_bulk_move_atomicity_on_failure exceeded 30 s budget");
}
