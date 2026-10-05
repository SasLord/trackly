//! Phase 41 Plan 30 (W-B03): отчёт «Перемещения» держит потолок 1000 строк,
//! «старые первыми». Усечение не должно быть тихим: `total` — истинное число
//! подходящих записей, а печать и CSV несут текст уведомления. Реальный
//! `AppCtx` на временном каталоге; данные вымышленные.
//!
//! Префикс тестов: `report_trunc_`.

use trackly_app::context::AppCtx;
use trackly_app::dto::reports::{PeriodDto, ReportFilter};
use trackly_app::services::report_service::movements_truncation_notice;
use trackly_app::tauri_cmds::reports::{
    build_reports_export_csv, build_reports_export_pdf, build_reports_get_report_counts,
    build_reports_list_movements,
};
use trackly_core::auth::Identity;
use trackly_infra::error_conversions::map_rusqlite;

const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../ui/scripts/fixtures/report-truncation/cases.json"
);

async fn build_ctx_in(dir: &std::path::Path) -> anyhow::Result<AppCtx> {
    let paths = trackly_infra::Paths::resolve_for_exe_dir(dir.to_path_buf())?;
    let config = trackly_infra::AppConfig::default();
    let log_guard = trackly_app::logging::init(&paths, &config).or_else(|_| {
        let (_nb, guard) = tracing_appender::non_blocking(std::io::sink());
        Ok::<_, anyhow::Error>(guard)
    })?;
    AppCtx::build(paths, config, log_guard).await
}

async fn make_test_ctx() -> (AppCtx, tempfile::TempDir) {
    let dir = tempfile::TempDir::new().expect("tempdir");
    let ctx = build_ctx_in(dir.path()).await.expect("build ctx");
    (ctx, dir)
}

fn admin() -> Identity {
    Identity::trusted_admin()
}

fn wide_period() -> PeriodDto {
    PeriodDto {
        mode: "range".to_string(),
        year: None,
        month: None,
        date_from: Some("2000-01-01".to_string()),
        date_to: Some("2099-12-31".to_string()),
    }
}

fn int(v: i64) -> rusqlite::types::Value {
    rusqlite::types::Value::Integer(v)
}

async fn exec(ctx: &AppCtx, sql: &'static str, p: Vec<rusqlite::types::Value>) {
    ctx.writer
        .execute(move |conn| {
            conn.execute(sql, rusqlite::params_from_iter(p))
                .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("exec");
}

async fn seed_place(ctx: &AppCtx, name: &str) -> i64 {
    let name = name.to_string();
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO places (kind, name, parent_id, is_storage, created_at_utc, \
                 updated_at_utc, version) VALUES ('room', ?1, NULL, 0, 1700000000, 1700000000, 1)",
                rusqlite::params![name],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed place")
}

async fn seed_device(ctx: &AppCtx, name: &str, inv: &str, type_id: i64, place: i64) -> i64 {
    let (name, inv) = (name.to_string(), inv.to_string());
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO devices (type_id, name, inventory_number, serial_number, place_id, \
                 status_id, created_at_utc, updated_at_utc, version) \
                 VALUES (?1, ?2, ?3, ?3, ?4, 1, 1700000000, 1700000000, 1)",
                rusqlite::params![type_id, name, inv, place],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed device")
}

/// `count` перемещений устройства `device` из `from` в `to`; `created_at_utc`
/// возрастает от `start_ts` (внутри `wide_period()`). Одним `INSERT ... SELECT`.
async fn seed_movements(ctx: &AppCtx, device: i64, from: i64, to: i64, count: i64, start_ts: i64) {
    exec(
        ctx,
        "WITH RECURSIVE n(i) AS (SELECT 0 UNION ALL SELECT i + 1 FROM n WHERE i + 1 < ?5) \
         INSERT INTO place_movements (entity_type, entity_id, from_place_id, from_place_path, \
         to_place_id, to_place_path, source, created_at_utc) \
         SELECT 'device', ?1, ?2, 'Склад А', ?3, 'Склад Б', 'manual', ?4 + i FROM n",
        vec![int(device), int(from), int(to), int(start_ts), int(count)],
    )
    .await;
}

struct Seeded {
    place_a: i64,
    place_b: i64,
}

/// Два склада и устройство типа 1; `to_b` перемещений А -> Б.
async fn seed_basic(ctx: &AppCtx, to_b: i64) -> Seeded {
    let place_a = seed_place(ctx, "Склад А").await;
    let place_b = seed_place(ctx, "Склад Б").await;
    let d = seed_device(ctx, "Ноутбук", "INV-T-1", 1, place_a).await;
    seed_movements(ctx, d, place_a, place_b, to_b, 1_700_000_000).await;
    Seeded { place_a, place_b }
}

fn golden_cases() -> Vec<serde_json::Value> {
    let raw = std::fs::read_to_string(FIXTURE).expect("fixture cases.json");
    serde_json::from_str(&raw).expect("fixture json")
}

/// Ожидаемый текст кейса (1000, 1005) — ЛИТЕРАЛ из фикстуры, а не результат функции.
fn fixture_notice_1000_of_1005() -> String {
    golden_cases()
        .iter()
        .find(|c| c["shown"] == 1000 && c["total"] == 1005)
        .and_then(|c| c["expected"].as_str())
        .expect("кейс 1000/1005 в фикстуре")
        .to_string()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn report_trunc_total_counts_rows_beyond_limit() {
    let (ctx, _dir) = make_test_ctx().await;
    seed_basic(&ctx, 1005).await;

    let resp = build_reports_list_movements(&ctx, &admin(), ReportFilter::default(), wide_period())
        .await
        .expect("movements report");
    assert_eq!(resp.rows.len(), 1000, "потолок строк сохранён");
    assert_eq!(
        resp.total, 1005,
        "total — истинное число подходящих записей"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn report_trunc_under_limit_total_equals_rows() {
    // Охранный тест: зелёный и на старом коде — защищает от ложного усечения.
    let (ctx, _dir) = make_test_ctx().await;
    seed_basic(&ctx, 3).await;
    let resp = build_reports_list_movements(&ctx, &admin(), ReportFilter::default(), wide_period())
        .await
        .expect("movements report");
    assert_eq!((resp.rows.len(), resp.total), (3, 3));

    let (ctx2, _dir2) = make_test_ctx().await;
    seed_basic(&ctx2, 1000).await;
    let resp =
        build_reports_list_movements(&ctx2, &admin(), ReportFilter::default(), wide_period())
            .await
            .expect("movements report");
    assert_eq!(
        (resp.rows.len(), resp.total),
        (1000, 1000),
        "ровно на потолке"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn report_trunc_total_respects_filters() {
    // 1001 перемещение в Склад Б + 4 в Склад В: фильтр «Куда» = Б -> total 1001.
    let (ctx, _dir) = make_test_ctx().await;
    let s = seed_basic(&ctx, 1001).await;
    let place_c = seed_place(&ctx, "Склад В").await;
    let d = seed_device(&ctx, "Монитор", "INV-T-2", 1, s.place_a).await;
    seed_movements(&ctx, d, s.place_a, place_c, 4, 1_700_100_000).await;

    let filter = ReportFilter {
        to_place_id: Some(s.place_b),
        ..ReportFilter::default()
    };
    let resp = build_reports_list_movements(&ctx, &admin(), filter, wide_period())
        .await
        .expect("movements report");
    assert_eq!(resp.rows.len(), 1000);
    assert_eq!(resp.total, 1001, "счёт использует те же CTE и условия");

    // Вариант 2: фильтр по типу устройства (join к devices): 1001 строка у
    // устройства типа 1 и 4 строки у устройства типа 2.
    let (ctx2, _dir2) = make_test_ctx().await;
    let s2 = seed_basic(&ctx2, 1001).await;
    let d2 = seed_device(&ctx2, "Принтер", "INV-T-3", 2, s2.place_a).await;
    seed_movements(&ctx2, d2, s2.place_a, s2.place_b, 4, 1_700_100_000).await;
    let filter = ReportFilter {
        type_id: Some(1),
        ..ReportFilter::default()
    };
    let resp = build_reports_list_movements(&ctx2, &admin(), filter, wide_period())
        .await
        .expect("movements report");
    assert_eq!(resp.rows.len(), 1000);
    assert_eq!(resp.total, 1001, "фильтр type_id учтён в счётном запросе");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn report_trunc_counts_badge_uses_total() {
    let (ctx, _dir) = make_test_ctx().await;
    seed_basic(&ctx, 1005).await;
    let counts = build_reports_get_report_counts(
        &ctx,
        &admin(),
        "movements".to_string(),
        ReportFilter::default(),
        wide_period(),
    )
    .await
    .expect("counts");
    assert_eq!(counts.counts.len(), 1);
    assert_eq!(counts.counts[0].key, "all");
    assert_eq!(
        counts.counts[0].count, 1005,
        "значок — истинное число, не 1000"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn report_trunc_csv_has_notice_row() {
    let (ctx, _dir) = make_test_ctx().await;
    seed_basic(&ctx, 1005).await;
    let bytes = build_reports_export_csv(
        &ctx,
        &admin(),
        "movements".to_string(),
        ReportFilter::default(),
        Some(wide_period()),
    )
    .await
    .expect("csv");
    let text = String::from_utf8(bytes).expect("utf8");
    let text = text.trim_start_matches('\u{FEFF}');
    let lines: Vec<&str> = text.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(
        lines.len(),
        1 + 1000 + 1,
        "шапка + 1000 строк + строка уведомления"
    );
    let last_first_cell = lines.last().unwrap().split(';').next().unwrap();
    assert_eq!(last_first_cell, fixture_notice_1000_of_1005());
    let notices = lines
        .iter()
        .filter(|l| l.starts_with("Показано записей"))
        .count();
    assert_eq!(notices, 1, "уведомление ровно одно");

    // Без усечения лишней строки нет.
    let (ctx2, _dir2) = make_test_ctx().await;
    seed_basic(&ctx2, 3).await;
    let bytes = build_reports_export_csv(
        &ctx2,
        &admin(),
        "movements".to_string(),
        ReportFilter::default(),
        Some(wide_period()),
    )
    .await
    .expect("csv");
    let text = String::from_utf8(bytes).expect("utf8");
    let lines: Vec<&str> = text.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 1 + 3, "шапка + 3 строки, без уведомления");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn report_trunc_print_summary_has_notice() {
    let (ctx, _dir) = make_test_ctx().await;
    let s = seed_basic(&ctx, 1005).await;
    let notice = fixture_notice_1000_of_1005();

    let html = build_reports_export_pdf(
        &ctx,
        &admin(),
        "movements".to_string(),
        ReportFilter::default(),
        Some(wide_period()),
    )
    .await
    .expect("html");
    assert!(html.contains(&notice), "уведомление в печати без фильтров");

    // С активным фильтром «Куда» — и сводка фильтра, и уведомление.
    let filter = ReportFilter {
        to_place_id: Some(s.place_b),
        ..ReportFilter::default()
    };
    let html = build_reports_export_pdf(
        &ctx,
        &admin(),
        "movements".to_string(),
        filter,
        Some(wide_period()),
    )
    .await
    .expect("html");
    assert!(html.contains("Куда: Склад Б"));
    assert!(html.contains(&notice));

    // Без усечения уведомления нет.
    let (ctx2, _dir2) = make_test_ctx().await;
    seed_basic(&ctx2, 3).await;
    let html = build_reports_export_pdf(
        &ctx2,
        &admin(),
        "movements".to_string(),
        ReportFilter::default(),
        Some(wide_period()),
    )
    .await
    .expect("html");
    assert!(!html.contains("Показано записей"));
}

#[test]
fn report_trunc_notice_matches_golden_fixture() {
    let cases = golden_cases();
    assert!(cases.len() >= 6, "фикстура не должна быть пустой");
    for c in &cases {
        let shown = c["shown"].as_u64().expect("shown") as usize;
        let total = c["total"].as_i64().expect("total");
        let expected = c["expected"].as_str().map(str::to_string);
        assert_eq!(
            movements_truncation_notice(shown, total),
            expected,
            "кейс {}",
            c["name"]
        );
    }
}
