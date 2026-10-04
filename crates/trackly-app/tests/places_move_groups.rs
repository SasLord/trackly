//! Phase 41 Plan 15 (D-23, GRP-07): массовый перенос содержимого места
//! (`PlaceService::move_subtree_contents`, write-site S2) и группы.
//!
//! Группа с местом едет целиком через `move_group_in_tx` в той же внешней
//! транзакции, остальной состав едет за ней, а её устройства исключены из цикла
//! одиночного переноса — иначе у каждого было бы ДВЕ строки журнала. Группа без
//! места («запрет спит», D-21) устройства не запирает: оно едет как обычное.
//!
//! Фикстуры расходятся (место группы против места участника, член вне
//! содержимого, группа без места), поэтому тесты не вакуумны. Реальный `AppCtx`
//! на временном каталоге; имена вымышленные.
//!
//! Префиксы: `subtree_group_`, `subtree_sleeping_`, `subtree_atomic_`.

use std::time::Duration;

use rusqlite::params;
use trackly_app::context::AppCtx;
use trackly_app::dto::auth::UserNew;
use trackly_core::auth::{Identity, Role};
use trackly_infra::error_conversions::map_rusqlite;

async fn make_test_ctx() -> (AppCtx, tempfile::TempDir) {
    let dir = tempfile::TempDir::new().expect("tempdir");
    let paths =
        trackly_infra::Paths::resolve_for_exe_dir(dir.path().to_path_buf()).expect("resolve paths");
    let config = trackly_infra::AppConfig::default();
    let log_guard = trackly_app::logging::init(&paths, &config).or_else(|_| {
        let (_nb, guard) = tracing_appender::non_blocking(std::io::sink());
        Ok::<_, anyhow::Error>(guard)
    });
    let ctx = AppCtx::build(paths, config, log_guard.expect("log guard"))
        .await
        .expect("build ctx");
    (ctx, dir)
}

async fn manager(ctx: &AppCtx) -> Identity {
    let dto = ctx
        .auth
        .create_user(
            UserNew {
                login: "manager_subtree_groups".to_string(),
                full_name: "Петров П.П.".to_string(),
                password: "password123".to_string(),
                role: "manager".to_string(),
                email: None,
            },
            &Identity::trusted_admin(),
        )
        .await
        .expect("create manager");
    Identity {
        user_id: Some(dto.id),
        role: Role::Manager,
    }
}

async fn workstation_type(ctx: &AppCtx) -> i64 {
    ctx.group_types
        .list_types(&Identity::trusted_admin(), false)
        .await
        .expect("list_types")
        .into_iter()
        .find(|t| t.code == "workstation")
        .expect("тип workstation")
        .id
}

async fn seed_place(ctx: &AppCtx, name: &str) -> i64 {
    let name = name.to_string();
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO places (kind, name, is_storage, created_at_utc, updated_at_utc, version) \
                 VALUES ('room', ?1, 0, 1700000000, 1700000000, 1)",
                params![name],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed place")
}

async fn seed_group(
    ctx: &AppCtx,
    type_id: i64,
    name: &str,
    seq: i64,
    place: Option<i64>,
    parent: Option<i64>,
) -> i64 {
    let name = name.to_string();
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO groups (type_id, name, seq, place_id, parent_group_id, \
                 created_at_utc, updated_at_utc, version) \
                 VALUES (?1, ?2, ?3, ?4, ?5, 1700000000, 1700000000, 1)",
                params![type_id, name, seq, place, parent],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed group")
}

/// Устройство (`type_id` 1 — обычное, 2 — принтер) на месте, при необходимости член группы.
async fn seed_device(
    ctx: &AppCtx,
    type_id: i64,
    name: &str,
    place: i64,
    group: Option<i64>,
) -> i64 {
    let name = name.to_string();
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO devices (type_id, name, place_id, status_id, version, \
                 created_at_utc, updated_at_utc) VALUES (?1, ?2, ?3, 1, 1, 1700000000, 1700000000)",
                params![type_id, name, place],
            )
            .map_err(map_rusqlite)?;
            let id = conn.last_insert_rowid();
            if let Some(g) = group {
                conn.execute(
                    "INSERT INTO group_devices (device_id, group_id, added_at_utc) \
                     VALUES (?1, ?2, 1700000000)",
                    params![id, g],
                )
                .map_err(map_rusqlite)?;
            }
            Ok(id)
        })
        .await
        .expect("seed device")
}

fn int(v: i64) -> rusqlite::types::Value {
    rusqlite::types::Value::Integer(v)
}

async fn scalar(ctx: &AppCtx, sql: &'static str, p: Vec<rusqlite::types::Value>) -> i64 {
    ctx.writer
        .execute(move |conn| {
            conn.query_row(sql, rusqlite::params_from_iter(p), |r| r.get::<_, i64>(0))
                .map_err(map_rusqlite)
        })
        .await
        .expect("scalar query")
}

async fn opt_scalar(
    ctx: &AppCtx,
    sql: &'static str,
    p: Vec<rusqlite::types::Value>,
) -> Option<i64> {
    ctx.writer
        .execute(move |conn| {
            conn.query_row(sql, rusqlite::params_from_iter(p), |r| {
                r.get::<_, Option<i64>>(0)
            })
            .map_err(map_rusqlite)
        })
        .await
        .expect("opt scalar query")
}

async fn device_place(ctx: &AppCtx, id: i64) -> Option<i64> {
    opt_scalar(
        ctx,
        "SELECT place_id FROM devices WHERE id = ?1",
        vec![int(id)],
    )
    .await
}

async fn group_place(ctx: &AppCtx, id: i64) -> Option<i64> {
    opt_scalar(
        ctx,
        "SELECT place_id FROM groups WHERE id = ?1",
        vec![int(id)],
    )
    .await
}

async fn device_rows(ctx: &AppCtx, id: i64) -> i64 {
    scalar(
        ctx,
        "SELECT COUNT(*) FROM place_movements WHERE entity_type = 'device' AND entity_id = ?1",
        vec![int(id)],
    )
    .await
}

async fn all_movement_rows(ctx: &AppCtx) -> i64 {
    scalar(ctx, "SELECT COUNT(*) FROM place_movements", vec![]).await
}

async fn all_audit_rows(ctx: &AppCtx) -> i64 {
    scalar(ctx, "SELECT COUNT(*) FROM audit_log", vec![]).await
}

async fn rows_with_source(ctx: &AppCtx, source: &'static str) -> i64 {
    scalar(
        ctx,
        "SELECT COUNT(*) FROM place_movements WHERE source = ?1",
        vec![rusqlite::types::Value::Text(source.to_string())],
    )
    .await
}

async fn rows_in_batches(ctx: &AppCtx) -> i64 {
    scalar(
        ctx,
        "SELECT COUNT(*) FROM place_movements WHERE batch_id IS NOT NULL",
        vec![],
    )
    .await
}

async fn distinct_batches(ctx: &AppCtx) -> i64 {
    scalar(
        ctx,
        "SELECT COUNT(DISTINCT batch_id) FROM place_movements WHERE batch_id IS NOT NULL",
        vec![],
    )
    .await
}

// ---------------------------------------------------------------------------
// D-23: группа едет целиком, без дублей журнала
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn subtree_group_moves_whole_group_without_duplicate_journal_rows() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let (ctx, _dir) = make_test_ctx().await;
        let mgr = manager(&ctx).await;
        let arm = workstation_type(&ctx).await;

        let place_a = seed_place(&ctx, "Склад А").await;
        let place_b = seed_place(&ctx, "Склад Б").await;

        // «АРМ #3»: 4 прямых устройства + вложенная группа с 2 устройствами.
        let root_group = seed_group(&ctx, arm, "АРМ #3", 3, Some(place_a), None).await;
        let nested_group =
            seed_group(&ctx, arm, "АРМ #4", 4, Some(place_a), Some(root_group)).await;
        let mut group_devices = Vec::new();
        for i in 1..=4 {
            group_devices.push(
                seed_device(
                    &ctx,
                    1,
                    &format!("Системный блок {i}"),
                    place_a,
                    Some(root_group),
                )
                .await,
            );
        }
        for i in 1..=2 {
            group_devices.push(
                seed_device(
                    &ctx,
                    1,
                    &format!("Монитор {i}"),
                    place_a,
                    Some(nested_group),
                )
                .await,
            );
        }
        // Два одиночных устройства того же склада.
        let single_1 = seed_device(&ctx, 1, "Ноутбук Одиночный 1", place_a, None).await;
        let single_2 = seed_device(&ctx, 2, "Принтер Одиночный 2", place_a, None).await;

        let moved = ctx
            .places
            .move_subtree_contents(&mgr, place_a, place_b, None)
            .await
            .expect("bulk move");
        assert_eq!(
            moved, 8,
            "возвращаемое число = все элементы содержимого, как прежде"
        );

        for d in group_devices.iter().chain([&single_1, &single_2]) {
            assert_eq!(device_place(&ctx, *d).await, Some(place_b));
        }
        assert_eq!(group_place(&ctx, root_group).await, Some(place_b));
        assert_eq!(
            group_place(&ctx, nested_group).await,
            Some(place_b),
            "вложенная группа денормализована вместе с корнем"
        );

        // Ни у одного устройства группы не две строки журнала.
        for d in &group_devices {
            assert_eq!(
                device_rows(&ctx, *d).await,
                1,
                "устройство группы {d}: ровно одна строка журнала, не две"
            );
        }
        // 1 строка группы + 6 строк устройств с одним batch_id + 2 manual = 9.
        assert_eq!(all_movement_rows(&ctx).await, 9);
        assert_eq!(rows_in_batches(&ctx).await, 7);
        assert_eq!(
            distinct_batches(&ctx).await,
            1,
            "один пакет на корневую группу"
        );
        assert_eq!(rows_with_source(&ctx, "manual").await, 2);
        assert_eq!(rows_with_source(&ctx, "group").await, 7);
        // Одиночные — manual и вне пакета.
        for d in [single_1, single_2] {
            assert_eq!(
                scalar(
                    &ctx,
                    "SELECT COUNT(*) FROM place_movements WHERE entity_type = 'device' \
                     AND entity_id = ?1 AND source = 'manual' AND batch_id IS NULL",
                    vec![int(d)]
                )
                .await,
                1
            );
        }
    })
    .await
    .expect("test exceeded 60 s budget");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn subtree_group_pulls_member_from_another_place_along() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let (ctx, _dir) = make_test_ctx().await;
        let mgr = manager(&ctx).await;
        let arm = workstation_type(&ctx).await;

        let place_a = seed_place(&ctx, "Склад А").await;
        let place_b = seed_place(&ctx, "Склад Б").await;
        let place_c = seed_place(&ctx, "Кабинет В").await;

        let group = seed_group(&ctx, arm, "АРМ #7", 7, Some(place_a), None).await;
        let inside = seed_device(&ctx, 1, "Системный блок А", place_a, Some(group)).await;
        // Второй член группы лежит ВНЕ содержимого склада А.
        let outside = seed_device(&ctx, 1, "Монитор А", place_c, Some(group)).await;

        ctx.places
            .move_subtree_contents(&mgr, place_a, place_b, None)
            .await
            .expect("bulk move");

        assert_eq!(group_place(&ctx, group).await, Some(place_b));
        assert_eq!(device_place(&ctx, inside).await, Some(place_b));
        assert_eq!(
            device_place(&ctx, outside).await,
            Some(place_b),
            "переносится ВСЯ группа: член вне содержимого тоже получает целевое место"
        );
        assert_eq!(device_rows(&ctx, inside).await, 1);
        assert_eq!(device_rows(&ctx, outside).await, 1);
        // Группа + два устройства, всё в одном пакете, ни одной manual-строки.
        assert_eq!(all_movement_rows(&ctx).await, 3);
        assert_eq!(rows_in_batches(&ctx).await, 3);
        assert_eq!(rows_with_source(&ctx, "manual").await, 0);
    })
    .await
    .expect("test exceeded 60 s budget");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn subtree_group_two_root_groups_make_two_batches() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let (ctx, _dir) = make_test_ctx().await;
        let mgr = manager(&ctx).await;
        let arm = workstation_type(&ctx).await;

        let place_a = seed_place(&ctx, "Склад А").await;
        let place_b = seed_place(&ctx, "Склад Б").await;

        let g1 = seed_group(&ctx, arm, "АРМ #1", 1, Some(place_a), None).await;
        let g2 = seed_group(&ctx, arm, "АРМ #2", 2, Some(place_a), None).await;
        for i in 1..=2 {
            seed_device(&ctx, 1, &format!("Блок 1-{i}"), place_a, Some(g1)).await;
            seed_device(&ctx, 1, &format!("Блок 2-{i}"), place_a, Some(g2)).await;
        }

        ctx.places
            .move_subtree_contents(&mgr, place_a, place_b, None)
            .await
            .expect("bulk move");

        assert_eq!(group_place(&ctx, g1).await, Some(place_b));
        assert_eq!(group_place(&ctx, g2).await, Some(place_b));
        assert_eq!(
            distinct_batches(&ctx).await,
            2,
            "по пакету на корневую группу"
        );
        assert_eq!(all_movement_rows(&ctx).await, 6);
    })
    .await
    .expect("test exceeded 60 s budget");
}

// ---------------------------------------------------------------------------
// D-21: группа без места устройство не запирает
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn subtree_sleeping_group_member_moves_as_ordinary_device() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let (ctx, _dir) = make_test_ctx().await;
        let mgr = manager(&ctx).await;
        let arm = workstation_type(&ctx).await;

        let place_a = seed_place(&ctx, "Склад А").await;
        let place_b = seed_place(&ctx, "Склад Б").await;

        let sleeping = seed_group(&ctx, arm, "АРМ #9", 9, None, None).await;
        let member = seed_device(&ctx, 1, "Системный блок Б", place_a, Some(sleeping)).await;
        let single = seed_device(&ctx, 1, "Ноутбук Б", place_a, None).await;

        let moved = ctx
            .places
            .move_subtree_contents(&mgr, place_a, place_b, None)
            .await
            .expect("bulk move");
        assert_eq!(moved, 2);

        assert_eq!(device_place(&ctx, member).await, Some(place_b));
        assert_eq!(device_place(&ctx, single).await, Some(place_b));
        assert_eq!(
            group_place(&ctx, sleeping).await,
            None,
            "группа без места остаётся без места"
        );
        assert_eq!(device_rows(&ctx, member).await, 1);
        assert_eq!(
            scalar(
                &ctx,
                "SELECT COUNT(*) FROM place_movements WHERE entity_type = 'device' \
                 AND entity_id = ?1 AND source = 'manual' AND batch_id IS NULL",
                vec![int(member)]
            )
            .await,
            1,
            "член спящей группы — обычная manual-строка вне пакета"
        );
        assert_eq!(all_movement_rows(&ctx).await, 2);
        assert_eq!(rows_in_batches(&ctx).await, 0);
        assert_eq!(
            scalar(
                &ctx,
                "SELECT COUNT(*) FROM place_movements WHERE entity_type = 'group'",
                vec![]
            )
            .await,
            0
        );
    })
    .await
    .expect("test exceeded 60 s budget");
}

// ---------------------------------------------------------------------------
// Атомарность: один conn.transaction() на весь перенос
// ---------------------------------------------------------------------------

/// Сбой на одиночном устройстве: ни группа, ни устройства, ни журнал не меняются.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn subtree_atomic_single_device_failure_leaves_everything_untouched() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let (ctx, _dir) = make_test_ctx().await;
        let mgr = manager(&ctx).await;
        let arm = workstation_type(&ctx).await;

        let place_a = seed_place(&ctx, "Склад А").await;
        let place_b = seed_place(&ctx, "Склад Б").await;

        let group = seed_group(&ctx, arm, "АРМ #5", 5, Some(place_a), None).await;
        let member = seed_device(&ctx, 1, "Системный блок В", place_a, Some(group)).await;
        let single = seed_device(&ctx, 1, "Ноутбук В", place_a, None).await;

        ctx.writer
            .execute(move |conn| {
                conn.execute(
                    &format!(
                        "CREATE TRIGGER fail_subtree_single BEFORE UPDATE OF place_id ON devices \
                         WHEN NEW.id = {single} \
                         BEGIN SELECT RAISE(ABORT, 'boom single'); END;"
                    ),
                    [],
                )
                .map_err(map_rusqlite)?;
                Ok(())
            })
            .await
            .expect("install trigger");

        let movements_before = all_movement_rows(&ctx).await;
        let audit_before = all_audit_rows(&ctx).await;

        let err = ctx
            .places
            .move_subtree_contents(&mgr, place_a, place_b, None)
            .await
            .expect_err("перенос должен упасть");
        assert!(format!("{err:?}").contains("boom single"), "{err:?}");

        assert_eq!(group_place(&ctx, group).await, Some(place_a));
        assert_eq!(device_place(&ctx, member).await, Some(place_a));
        assert_eq!(device_place(&ctx, single).await, Some(place_a));
        assert_eq!(all_movement_rows(&ctx).await, movements_before);
        assert_eq!(all_audit_rows(&ctx).await, audit_before);
    })
    .await
    .expect("test exceeded 60 s budget");
}

/// Сбой на члене группы ПОСЛЕ того, как одиночные устройства уже обновлены в
/// транзакции: откатываются и одиночные, и место группы, и журнал, и аудит.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn subtree_atomic_group_failure_rolls_back_single_devices_too() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let (ctx, _dir) = make_test_ctx().await;
        let mgr = manager(&ctx).await;
        let arm = workstation_type(&ctx).await;

        let place_a = seed_place(&ctx, "Склад А").await;
        let place_b = seed_place(&ctx, "Склад Б").await;

        let group = seed_group(&ctx, arm, "АРМ #6", 6, Some(place_a), None).await;
        let first_member = seed_device(&ctx, 1, "Системный блок Г1", place_a, Some(group)).await;
        let failing_member = seed_device(&ctx, 1, "Системный блок Г2", place_a, Some(group)).await;
        let single_1 = seed_device(&ctx, 1, "Ноутбук Г1", place_a, None).await;
        let single_2 = seed_device(&ctx, 2, "Принтер Г2", place_a, None).await;

        ctx.writer
            .execute(move |conn| {
                conn.execute(
                    &format!(
                        "CREATE TRIGGER fail_subtree_member BEFORE UPDATE OF place_id ON devices \
                         WHEN NEW.id = {failing_member} \
                         BEGIN SELECT RAISE(ABORT, 'boom member'); END;"
                    ),
                    [],
                )
                .map_err(map_rusqlite)?;
                Ok(())
            })
            .await
            .expect("install trigger");

        let movements_before = all_movement_rows(&ctx).await;
        let audit_before = all_audit_rows(&ctx).await;

        let err = ctx
            .places
            .move_subtree_contents(&mgr, place_a, place_b, None)
            .await
            .expect_err("перенос должен упасть");
        assert!(format!("{err:?}").contains("boom member"), "{err:?}");

        assert_eq!(group_place(&ctx, group).await, Some(place_a));
        for d in [first_member, failing_member, single_1, single_2] {
            assert_eq!(
                device_place(&ctx, d).await,
                Some(place_a),
                "устройство {d} не должно сохранить частичное состояние"
            );
        }
        assert_eq!(all_movement_rows(&ctx).await, movements_before);
        assert_eq!(all_audit_rows(&ctx).await, audit_before);
    })
    .await
    .expect("test exceeded 60 s budget");
}
