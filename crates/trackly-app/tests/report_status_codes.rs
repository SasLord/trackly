//! Отчёты устройств «В работе» / «На складе» подбирают статус по `code`, а не по
//! русскому имени (Phase 41.1, D-03).
//!
//! Тест не вакуумен: перед вызовом отчёта статусы `в_работе` и `на_складе`
//! ПЕРЕИМЕНОВАНЫ в `device_statuses`. Реализация с `WHERE name = 'В работе'`
//! после переименования не находит статус и возвращает ноль строк.
//! Имена в данных вымышленные.

use rusqlite::params;
use trackly_app::context::AppCtx;
use trackly_app::dto::reports::{PeriodDto, ReportFilter};
use trackly_infra::error_conversions::map_rusqlite;

const RENAMED_IN_USE: &str = "Рабочее состояние (переим.)";
const RENAMED_IN_STOCK: &str = "Хранение (переим.)";

async fn make_test_ctx() -> (AppCtx, tempfile::TempDir) {
    let dir = tempfile::TempDir::new().expect("tempdir");
    let paths =
        trackly_infra::Paths::resolve_for_exe_dir(dir.path().to_path_buf()).expect("resolve paths");
    let config = trackly_infra::AppConfig::default();
    let log_guard = trackly_app::logging::init(&paths, &config).or_else(|_| {
        let (_nb, guard) = tracing_appender::non_blocking(std::io::sink());
        Ok::<_, anyhow::Error>(guard)
    });
    let log_guard = log_guard.expect("log guard");
    let ctx = AppCtx::build(paths, config, log_guard)
        .await
        .expect("build ctx");
    (ctx, dir)
}

/// Два устройства (в работе / на складе), статусы выставлены по `code`,
/// затем оба статуса переименованы. Возвращает (id в работе, id на складе).
async fn seed_and_rename(ctx: &AppCtx) -> (i64, i64) {
    ctx.writer
        .execute(move |conn| {
            let status_by_code = |code: &str| -> rusqlite::Result<i64> {
                conn.query_row(
                    "SELECT id FROM device_statuses WHERE code = ?1",
                    params![code],
                    |r| r.get(0),
                )
            };
            let in_use = status_by_code("в_работе").map_err(map_rusqlite)?;
            let in_stock = status_by_code("на_складе").map_err(map_rusqlite)?;
            conn.execute(
                "INSERT INTO places (kind, name, parent_id, is_storage, created_at_utc, \
                 updated_at_utc, version) VALUES ('room', 'Склад А', NULL, 0, 1700000000, 1700000000, 1)",
                [],
            )
            .map_err(map_rusqlite)?;
            let place = conn.last_insert_rowid();
            let mut ids = Vec::new();
            for (name, inv, status) in [
                ("Ноутбук Иванова И.И.", "INV-RPT-1", in_use),
                ("Монитор Петрова П.П.", "INV-RPT-2", in_stock),
            ] {
                conn.execute(
                    "INSERT INTO devices (type_id, name, inventory_number, place_id, status_id, \
                     created_at_utc, updated_at_utc, version) \
                     VALUES (1, ?1, ?2, ?3, ?4, 1700000000, 1700000000, 1)",
                    params![name, inv, place, status],
                )
                .map_err(map_rusqlite)?;
                ids.push(conn.last_insert_rowid());
            }
            conn.execute(
                "UPDATE device_statuses SET name = ?1 WHERE code = 'в_работе'",
                params![RENAMED_IN_USE],
            )
            .map_err(map_rusqlite)?;
            conn.execute(
                "UPDATE device_statuses SET name = ?1 WHERE code = 'на_складе'",
                params![RENAMED_IN_STOCK],
            )
            .map_err(map_rusqlite)?;
            Ok((ids[0], ids[1]))
        })
        .await
        .expect("seed devices and rename statuses")
}

fn period_all() -> PeriodDto {
    PeriodDto {
        mode: "year".to_string(),
        year: Some(2026),
        month: None,
        date_from: None,
        date_to: None,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn device_reports_find_rows_after_status_rename() {
    let (ctx, _dir) = make_test_ctx().await;
    let (in_use_id, in_stock_id) = seed_and_rename(&ctx).await;

    let in_use = ctx
        .reports
        .list_device_in_use(ReportFilter::default())
        .await
        .expect("in_use");
    assert_eq!(
        in_use.total, 1,
        "«В работе» должен найти устройство по code"
    );
    assert_eq!(in_use.rows[0].id, in_use_id);

    let in_stock = ctx
        .reports
        .list_device_in_stock(ReportFilter::default())
        .await
        .expect("in_stock");
    assert_eq!(
        in_stock.total, 1,
        "«На складе» должен найти устройство по code"
    );
    assert_eq!(in_stock.rows[0].id, in_stock_id);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn device_report_counts_survive_status_rename() {
    let (ctx, _dir) = make_test_ctx().await;
    seed_and_rename(&ctx).await;

    let counts = ctx
        .reports
        .get_report_counts("devices", ReportFilter::default(), period_all(), false)
        .await
        .expect("counts");
    let get = |key: &str| {
        counts
            .counts
            .iter()
            .find(|c| c.key == key)
            .unwrap_or_else(|| panic!("нет счётчика {key}"))
            .count
    };
    assert_eq!(get("in_use"), 1, "счётчик in_use после переименования");
    assert_eq!(get("in_stock"), 1, "счётчик in_stock после переименования");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn report_rows_show_the_new_status_name() {
    let (ctx, _dir) = make_test_ctx().await;
    seed_and_rename(&ctx).await;

    let in_use = ctx
        .reports
        .list_device_in_use(ReportFilter::default())
        .await
        .expect("in_use");
    assert_eq!(
        in_use.rows[0].status_name.as_deref(),
        Some(RENAMED_IN_USE),
        "имя в строке отчёта берётся из device_statuses.name через JOIN"
    );
    let in_stock = ctx
        .reports
        .list_device_in_stock(ReportFilter::default())
        .await
        .expect("in_stock");
    assert_eq!(
        in_stock.rows[0].status_name.as_deref(),
        Some(RENAMED_IN_STOCK)
    );
}
