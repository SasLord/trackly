//! Phase 41.8 Plan 20 (D-28): создание устройства с `origin_group_id` вводит его в
//! состав группы разбора той же транзакцией писателя; отказ ввода откатывает создание.
//! Реальный `AppCtx` на временном каталоге; имена вымышленные.

use rusqlite::params;

use trackly_app::context::AppCtx;
use trackly_app::dto::device::DeviceNew;
use trackly_core::error::AppError;
use trackly_infra::error_conversions::map_rusqlite;

async fn make_ctx() -> (AppCtx, tempfile::TempDir) {
    let dir = tempfile::TempDir::new().expect("tempdir");
    let paths = trackly_infra::Paths::resolve_for_exe_dir(dir.path().to_path_buf()).expect("paths");
    let config = trackly_infra::AppConfig::default();
    let log_guard = trackly_app::logging::init(&paths, &config).unwrap_or_else(|_| {
        let (_nb, guard) = tracing_appender::non_blocking(std::io::sink());
        guard
    });
    let ctx = AppCtx::build(paths, config, log_guard).await.expect("ctx");
    (ctx, dir)
}

async fn seed_teardown(ctx: &AppCtx, name: &str, seq: i64) -> i64 {
    let name = name.to_string();
    ctx.writer
        .execute(move |conn| {
            let t: i64 = conn
                .query_row(
                    "SELECT id FROM group_types WHERE behavior = 'teardown' LIMIT 1",
                    [],
                    |r| r.get(0),
                )
                .map_err(map_rusqlite)?;
            conn.execute(
                "INSERT INTO groups (type_id, name, seq, created_at_utc, updated_at_utc, version) \
                 VALUES (?1, ?2, ?3, 1700000000, 1700000000, 1)",
                params![t, name, seq],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed teardown")
}

async fn seed_container(ctx: &AppCtx, name: &str, seq: i64) -> i64 {
    let name = name.to_string();
    ctx.writer
        .execute(move |conn| {
            let t: i64 = conn
                .query_row(
                    "SELECT id FROM group_types WHERE behavior != 'teardown' LIMIT 1",
                    [],
                    |r| r.get(0),
                )
                .map_err(map_rusqlite)?;
            conn.execute(
                "INSERT INTO groups (type_id, name, seq, created_at_utc, updated_at_utc, version) \
                 VALUES (?1, ?2, ?3, 1700000000, 1700000000, 1)",
                params![t, name, seq],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed container")
}

async fn status_id(ctx: &AppCtx, code: &'static str) -> i64 {
    ctx.writer
        .execute(move |conn| {
            conn.query_row(
                "SELECT id FROM device_statuses WHERE code = ?1",
                params![code],
                |r| r.get(0),
            )
            .map_err(map_rusqlite)
        })
        .await
        .expect("status id")
}

async fn scalar(ctx: &AppCtx, sql: &'static str, p: Vec<rusqlite::types::Value>) -> i64 {
    ctx.writer
        .execute(move |conn| {
            conn.query_row(sql, rusqlite::params_from_iter(p), |r| r.get::<_, i64>(0))
                .map_err(map_rusqlite)
        })
        .await
        .expect("scalar")
}

fn part(name: &str, status_id: i64, origin: Option<i64>) -> DeviceNew {
    DeviceNew {
        type_id: 1,
        name: name.to_string(),
        inventory_no: None,
        serial_no: None,
        model: None,
        specs: None,
        kit: None,
        state: None,
        place_id: None,
        status_id,
        origin_group_id: origin,
    }
}

async fn members_of(ctx: &AppCtx, g: i64) -> i64 {
    scalar(
        ctx,
        "SELECT COUNT(*) FROM group_devices WHERE group_id = ?1",
        vec![rusqlite::types::Value::Integer(g)],
    )
    .await
}

async fn device_count_named(ctx: &AppCtx, name: &str) -> i64 {
    scalar(
        ctx,
        "SELECT COUNT(*) FROM devices WHERE name = ?1",
        vec![rusqlite::types::Value::Text(name.to_string())],
    )
    .await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn d28_create_with_origin_joins_group_and_audits() {
    let (ctx, _d) = make_ctx().await;
    let td = seed_teardown(&ctx, "Разбор #7", 7).await;
    let st = status_id(&ctx, "в_работе").await;
    let before = scalar(
        &ctx,
        "SELECT COUNT(*) FROM audit_log WHERE action = 'add_devices'",
        vec![],
    )
    .await;
    ctx.devices
        .create(part("Блок питания", st, Some(td)))
        .await
        .expect("create");
    assert_eq!(
        members_of(&ctx, td).await,
        1,
        "деталь должна быть в составе"
    );
    let after = scalar(
        &ctx,
        "SELECT COUNT(*) FROM audit_log WHERE action = 'add_devices'",
        vec![],
    )
    .await;
    assert_eq!(after, before + 1, "одна запись аудита ввода");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn d28_bulk_create_with_origin_joins_all() {
    let (ctx, _d) = make_ctx().await;
    let td = seed_teardown(&ctx, "Разбор #7", 7).await;
    let st = status_id(&ctx, "в_работе").await;
    let made = ctx
        .devices
        .bulk_create(part("Планка памяти", st, Some(td)), 3)
        .await
        .expect("bulk");
    assert_eq!(made.len(), 3);
    assert_eq!(members_of(&ctx, td).await, 3);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn d28_refused_join_rolls_back_the_device() {
    let (ctx, _d) = make_ctx().await;
    let td = seed_teardown(&ctx, "Разбор #7", 7).await;
    let written_off = status_id(&ctx, "списано").await;
    let err = ctx
        .devices
        .create(part("Кулер списанный", written_off, Some(td)))
        .await
        .expect_err("ввод должен отказать по статусу");
    assert!(matches!(err, AppError::Validation { .. }), "{err:?}");
    assert_eq!(device_count_named(&ctx, "Кулер списанный").await, 0);
    assert_eq!(members_of(&ctx, td).await, 0);

    let err = ctx
        .devices
        .bulk_create(part("Кулер пакетный", written_off, Some(td)), 2)
        .await
        .expect_err("bulk откат");
    assert!(matches!(err, AppError::Validation { .. }), "{err:?}");
    assert_eq!(device_count_named(&ctx, "Кулер пакетный").await, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn d28_non_teardown_origin_is_rejected_and_nothing_created() {
    let (ctx, _d) = make_ctx().await;
    let arm = seed_container(&ctx, "АРМ #5", 5).await;
    let st = status_id(&ctx, "в_работе").await;
    let err = ctx
        .devices
        .create(part("SSD 256 ГБ", st, Some(arm)))
        .await
        .expect_err("не разбор");
    assert!(matches!(err, AppError::Validation { .. }), "{err:?}");
    assert_eq!(device_count_named(&ctx, "SSD 256 ГБ").await, 0);
}
