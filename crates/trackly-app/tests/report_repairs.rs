//! Фаза 41.4 план 07 (D-30/D-31): отчёт «Ремонты» — одна строка на заход в
//! сервис. Реальный `AppCtx` на временном каталоге; данные вымышленные.
//!
//! Покрывает: период по дате ухода, итог (заход без стоимости в сумму не
//! входит), CSV (стоимость числом с запятой без пробела тысяч, выравнивание
//! колонок), отсутствие отменённой и мягко удалённой заявок, пустой период,
//! закрытый для сотрудника гейт и живой HTTP-маршрут (`spa_fallback` иначе
//! маскирует отсутствие маршрута).

use axum::body::Body;
use axum::http::Request;
use tower::ServiceExt;

use trackly_app::context::AppCtx;
use trackly_app::dto::reports::{PeriodDto, ReportFilter};
use trackly_app::http::build_router;
use trackly_app::server::rusqlite_session_store::RusqliteSessionStore;
use trackly_app::services::report_service::repairs_total_label;
use trackly_app::tauri_cmds::reports::{build_reports_export_csv, build_reports_list_repairs};
use trackly_core::auth::{Identity, Role};
use trackly_infra::error_conversions::map_rusqlite;

// 2026-03-10 / 2026-03-20 / 2020-01-15, 12:00 UTC.
const MAR_10: i64 = 1_773_144_000;
const MAR_20: i64 = 1_774_008_000;
const JAN_2020: i64 = 1_579_089_600;

async fn make_ctx() -> (AppCtx, tempfile::TempDir) {
    let dir = tempfile::TempDir::new().expect("tempdir");
    let paths = trackly_infra::Paths::resolve_for_exe_dir(dir.path().to_path_buf()).expect("paths");
    let config = trackly_infra::AppConfig::default();
    let log_guard = trackly_app::logging::init(&paths, &config)
        .or_else(|_| {
            let (_nb, guard) = tracing_appender::non_blocking(std::io::sink());
            Ok::<_, anyhow::Error>(guard)
        })
        .expect("log guard");
    let ctx = AppCtx::build(paths, config, log_guard).await.expect("ctx");
    (ctx, dir)
}

fn march() -> PeriodDto {
    PeriodDto {
        mode: "month".to_string(),
        year: Some(2026),
        month: Some(3),
        date_from: None,
        date_to: None,
    }
}

fn admin() -> Identity {
    Identity::trusted_admin()
}

struct Seeded {
    request_with_visits: i64,
}

async fn seed(ctx: &AppCtx) -> Seeded {
    ctx.writer
        .execute(move |conn| {
            let now = 1_700_000_000_i64;
            conn.execute(
                "INSERT INTO users (login, full_name, role, ad_user, created_at_utc, \
                 updated_at_utc, version) VALUES ('ivanov', 'Иванов И.И.', 'employee', 0, ?1, ?1, 1)",
                rusqlite::params![now],
            )
            .map_err(map_rusqlite)?;
            let user = conn.last_insert_rowid();
            conn.execute(
                "INSERT INTO places (kind, name, created_at_utc, updated_at_utc, version) \
                 VALUES ('room', 'Сервис-центр №1', ?1, ?1, 1)",
                rusqlite::params![now],
            )
            .map_err(map_rusqlite)?;
            let service = conn.last_insert_rowid();
            let device = |name: &str, inv: &str| -> Result<i64, trackly_core::error::AppError> {
                conn.execute(
                    "INSERT INTO devices (type_id, name, inventory_number, place_id, status_id, \
                     created_at_utc, updated_at_utc, version) VALUES (1, ?1, ?2, ?3, 1, ?4, ?4, 1)",
                    rusqlite::params![name, inv, service, now],
                )
                .map_err(map_rusqlite)?;
                Ok(conn.last_insert_rowid())
            };
            let d1 = device("Ноутбук тестовый", "ПК-00014")?;
            let d2 = device("Системный блок тестовый", "ПК-00015")?;
            let d3 = device("Монитор тестовый", "ПК-00016")?;
            let d4 = device("Принтер тестовый", "ПК-00017")?;

            let request = |ty_status: &str, dev: i64, deleted: Option<i64>| {
                conn.execute(
                    "INSERT INTO requests (request_type, status, requested_by_user_id, \
                     printer_device_id, created_at_utc, updated_at_utc, deleted_at_utc) \
                     VALUES ('repair', ?1, ?2, ?3, ?4, ?4, ?5)",
                    rusqlite::params![ty_status, user, dev, now, deleted],
                )
                .map_err(map_rusqlite)?;
                Ok::<i64, trackly_core::error::AppError>(conn.last_insert_rowid())
            };
            let visit = |rid: i64,
                         dev: i64,
                         seq: i64,
                         sent: i64,
                         returned: Option<i64>,
                         cost: Option<i64>,
                         fault: &str| {
                conn.execute(
                    "INSERT INTO repair_visits (request_id, device_id, seq, service_place_id, \
                     sent_at_utc, returned_at_utc, giver_name, receiver_name, fault_description, \
                     works_done, cost_kopecks, act_number, created_at_utc, updated_at_utc) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'Иванов И.И.', 'Петров П.П.', ?7, \
                     'замена блока питания', ?8, 'РЕМ-000001', ?5, ?5)",
                    rusqlite::params![rid, dev, seq, service, sent, returned, fault, cost],
                )
                .map_err(map_rusqlite)?;
                Ok::<i64, trackly_core::error::AppError>(conn.last_insert_rowid())
            };

            // Заявка 1: два захода в марте. Первый вернулся (1500,50 ₽, со
            // сканом), второй ещё в сервисе, без стоимости.
            let r1 = request("in_repair", d1, None)?;
            let v1 = visit(r1, d1, 1, MAR_10, Some(MAR_10 + 3600), Some(150_050), "не включается")?;
            visit(r1, d1, 2, MAR_20, None, None, "шумит вентилятор")?;
            conn.execute(
                "INSERT INTO attachments (parent_kind, parent_id, original_name, mime, \
                 size_bytes, content_hash, created_at_utc) \
                 VALUES ('request', ?1, 'scan.pdf', 'application/pdf', 1, 'h1', ?2)",
                rusqlite::params![r1, now],
            )
            .map_err(map_rusqlite)?;
            let att = conn.last_insert_rowid();
            conn.execute(
                "INSERT INTO repair_visit_scans (visit_id, attachment_id, linked_at_utc) \
                 VALUES (?1, ?2, ?3)",
                rusqlite::params![v1, att, now],
            )
            .map_err(map_rusqlite)?;

            // Заявка 2: заход вне периода (2020).
            let r2 = request("completed", d2, None)?;
            visit(r2, d2, 1, JAN_2020, Some(JAN_2020 + 60), Some(100_000), "трещина")?;

            // Заявка 3: отменённая, заходов нет (D-12).
            request("cancelled", d3, None)?;

            // Заявка 4: мягко удалена, заход в марте — в отчёт не попадает.
            let r4 = request("completed", d4, Some(now))?;
            visit(r4, d4, 1, MAR_10, Some(MAR_10 + 60), Some(50_000), "удалённая заявка")?;
            Ok(r1)
        })
        .await
        .map(|r1| Seeded {
            request_with_visits: r1,
        })
        .expect("seed")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn report_repairs_period_rows_and_total() {
    let (ctx, _dir) = make_ctx().await;
    let seeded = seed(&ctx).await;

    let resp = build_reports_list_repairs(&ctx, &admin(), ReportFilter::default(), march())
        .await
        .expect("list");
    // Две строки марта; заход 2020, отменённая и удалённая заявки отсутствуют.
    assert_eq!(resp.rows.len(), 2, "{:#?}", resp.rows);
    assert_eq!(resp.total, 2);
    // Сортировка по дате ухода по убыванию: открытый заход (20.03) первым.
    let open = &resp.rows[0];
    assert_eq!(open.repair_sent_at, Some(MAR_20));
    assert_eq!(open.repair_returned_at, None);
    assert_eq!(open.repair_cost_kopecks, None);
    assert_eq!(open.repair_has_scan, Some(false));
    assert_eq!(open.repair_request_id, Some(seeded.request_with_visits));
    assert_eq!(open.repair_fault.as_deref(), Some("шумит вентилятор"));
    assert_eq!(
        open.repair_service_place.as_deref(),
        Some("Сервис-центр №1")
    );
    let done = &resp.rows[1];
    assert_eq!(
        done.repair_device_label.as_deref(),
        Some("Ноутбук тестовый (ПК-00014)")
    );
    assert_eq!(done.repair_cost_kopecks, Some(150_050));
    assert_eq!(done.repair_has_scan, Some(true));

    // Итог: заход без стоимости в сумму не входит.
    let total = repairs_total_label(&resp.rows, resp.total);
    assert_eq!(total, "Итого заходов: 2 · 1\u{a0}500,50 ₽");

    // Пустой период.
    let empty = PeriodDto {
        mode: "month".to_string(),
        year: Some(2026),
        month: Some(7),
        date_from: None,
        date_to: None,
    };
    let none = build_reports_list_repairs(&ctx, &admin(), ReportFilter::default(), empty)
        .await
        .expect("empty");
    assert!(none.rows.is_empty());
    assert_eq!(none.total, 0);
    assert_eq!(
        repairs_total_label(&none.rows, none.total),
        "Итого заходов: 0 · 0 ₽"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn report_repairs_csv_cost_with_comma_and_aligned_columns() {
    let (ctx, _dir) = make_ctx().await;
    seed(&ctx).await;

    let bytes = build_reports_export_csv(
        &ctx,
        &admin(),
        "repairs".to_string(),
        ReportFilter::default(),
        Some(march()),
    )
    .await
    .expect("csv");
    let text = String::from_utf8(bytes).expect("utf8");
    let text = text.trim_start_matches('\u{FEFF}');
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 3, "{text}");
    assert_eq!(
        lines[0],
        "Устройство;Неисправность;Сервис;Ушло;Вернулось;Работы;Стоимость ₽;Скан;Заявка"
    );
    // Каждая строка — 9 ячеек, как и заголовок (выравнивание колонок).
    for line in &lines {
        assert_eq!(line.split(';').count(), 9, "{line}");
    }
    assert!(
        lines[1].contains(";20.03.2026;в сервисе;замена блока питания;;нет;№"),
        "{}",
        lines[1]
    );
    assert!(
        lines[2].contains(";10.03.2026;10.03.2026;замена блока питания;1500,50;есть;№"),
        "{}",
        lines[2]
    );
    assert!(
        !lines[2].contains("1\u{a0}500"),
        "пробел тысяч в CSV: {}",
        lines[2]
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn report_repairs_employee_is_forbidden() {
    let (ctx, _dir) = make_ctx().await;
    let employee = Identity {
        user_id: Some(1),
        role: Role::Employee,
    };
    let r = build_reports_list_repairs(&ctx, &employee, ReportFilter::default(), march()).await;
    assert!(r.is_err(), "сотрудник не должен видеть отчёт");
}

/// Маршрут зарегистрирован в живом `build_router()`: без сессии отвечает как
/// существующий защищённый маршрут (не 404 JSON/HTML незарегистрированного
/// пути), а эталонный несуществующий путь даёт 404.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn report_repairs_http_route_is_registered() {
    let (ctx, _dir) = make_ctx().await;
    let app = build_router(
        &ctx,
        RusqliteSessionStore::new(ctx.writer.clone(), ctx.readers.clone()),
    );
    async fn post(app: &axum::Router, uri: &str) -> u16 {
        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(uri)
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .expect("request"),
            )
            .await
            .expect("response")
            .status()
            .as_u16()
    }
    let control = post(&app, "/api/v1/zzz_unregistered_control").await;
    assert_eq!(control, 404, "эталон «маршрута нет» должен быть 404");
    let got = post(&app, "/api/v1/reports_list_repairs").await;
    assert_ne!(got, 404, "маршрут reports_list_repairs не зарегистрирован");
    assert_ne!(got, 200, "анонимный запрос не должен проходить");
    ctx.shutdown.cancel();
}
