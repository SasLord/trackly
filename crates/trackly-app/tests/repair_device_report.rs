//! Phase 41.4, план 31: печатный «Отчёт о ремонтах устройства» (только чтение, персонал).
//! Данные вымышленные: «Иванов И.И.», «Петров П.П.».

mod entities_support;
mod repair_support;

use entities_support::fixture::{admin, make_test_ctx, seed_device, seed_place};
use repair_support::{emp, seed_user};
use rusqlite::params;
use trackly_app::context::AppCtx;
use trackly_core::error::AppError;
use trackly_infra::error_conversions::map_rusqlite;

async fn seed_request(ctx: &AppCtx, user: i64, device: i64, created: i64, desc: &str) -> i64 {
    let desc = desc.to_string();
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO requests (request_type, status, requested_by_user_id, \
                 printer_device_id, description, created_at_utc, updated_at_utc) \
                 VALUES ('repair', 'completed', ?1, ?2, ?4, ?3, ?3)",
                params![user, device, created, desc],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed request")
}

#[allow(clippy::too_many_arguments)]
async fn seed_visit(
    ctx: &AppCtx,
    request: i64,
    device: i64,
    seq: i64,
    place: i64,
    service: i64,
    fault: &str,
    works: &str,
    cost: Option<i64>,
    act: &str,
) {
    let (fault, works, act) = (fault.to_string(), works.to_string(), act.to_string());
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO repair_visits (request_id, device_id, seq, service_place_id, \
                 from_place_id, sent_at_utc, returned_at_utc, returned_to_place_id, giver_name, \
                 receiver_name, fault_description, works_done, cost_kopecks, act_number, \
                 created_at_utc, updated_at_utc, version) \
                 VALUES (?1, ?2, ?3, ?4, ?5, 1781654400, 1781740800, ?5, 'Иванов И.И.', \
                         'Петров П.П.', ?6, ?7, ?8, ?9, 1781654400, 1781654400, 1)",
                params![request, device, seq, service, place, fault, works, cost, act],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("seed visit");
}

struct Fx {
    ctx: AppCtx,
    _dir: tempfile::TempDir,
    user: i64,
    office: i64,
    service: i64,
    device: i64,
}

async fn fx() -> Fx {
    let (ctx, dir) = make_test_ctx().await;
    let user = seed_user(&ctx, "ivanov", "Иванов И.И.").await;
    let office = seed_place(&ctx, "Кабинет 12").await;
    let service = seed_place(&ctx, "Сервис-центр №1").await;
    let device = seed_device(&ctx, 1, "ПК-00014", "INV-14", Some(office), None).await;
    Fx {
        ctx,
        _dir: dir,
        user,
        office,
        service,
        device,
    }
}

async fn render(f: &Fx) -> Result<String, AppError> {
    f.ctx
        .repair_acts
        .render_device_report(&f.ctx.repairs, f.device, &admin())
        .await
}

/// Две заявки и три захода (у одного нет стоимости).
async fn seed_history(f: &Fx) {
    let r1 = seed_request(&f.ctx, f.user, f.device, 1_781_000_000, "Не включается").await;
    seed_visit(
        &f.ctx,
        r1,
        f.device,
        1,
        f.office,
        f.service,
        "Нет реакции на кнопку",
        "Заменён блок питания",
        Some(150_000),
        "РЕМ-000101",
    )
    .await;
    seed_visit(
        &f.ctx,
        r1,
        f.device,
        2,
        f.office,
        f.service,
        "Шумит вентилятор",
        "Чистка",
        None,
        "РЕМ-000102",
    )
    .await;
    let r2 = seed_request(&f.ctx, f.user, f.device, 1_781_500_000, "Перегрев").await;
    seed_visit(
        &f.ctx,
        r2,
        f.device,
        1,
        f.office,
        f.service,
        "Перегрев при нагрузке",
        "Замена термопасты",
        Some(30_000),
        "РЕМ-000103",
    )
    .await;
}

#[tokio::test]
async fn report_contains_device_summary_and_all_visits() {
    let f = fx().await;
    seed_history(&f).await;
    let html = render(&f).await.expect("render");
    for needle in [
        "Отчёт о ремонтах устройства",
        "ПК-00014",
        "INV-14",
        "РЕМ-000101",
        "РЕМ-000102",
        "РЕМ-000103",
        "Нет реакции на кнопку",
        "Шумит вентилятор",
        "Перегрев при нагрузке",
        "Заменён блок питания",
        "Замена термопасты",
        "Заходов: 3",
        "Суммарная стоимость: 1\u{a0}800 ₽",
        "1\u{a0}500 ₽",
        "300 ₽",
        "Выполнена",
        "Сервис-центр №1",
    ] {
        assert!(html.contains(needle), "нет «{needle}» в отчёте");
    }
    assert!(!html.contains("Ремонтов не было"));
}

#[tokio::test]
async fn user_text_is_escaped() {
    let f = fx().await;
    let r = seed_request(&f.ctx, f.user, f.device, 1_781_000_000, "<b>&описание").await;
    seed_visit(
        &f.ctx,
        r,
        f.device,
        1,
        f.office,
        f.service,
        "<script>alert(1)</script>",
        "a & b",
        None,
        "РЕМ-000201",
    )
    .await;
    let html = render(&f).await.expect("render");
    assert!(!html.contains("<script>alert(1)</script>"));
    assert!(html.contains("&lt;script&gt;"));
    assert!(html.contains("a &amp; b"));
    assert!(html.contains("&lt;b&gt;"));
}

#[tokio::test]
async fn device_without_visits_renders_empty_note() {
    let f = fx().await;
    let html = render(&f).await.expect("render");
    assert!(html.contains("Ремонтов не было"));
    assert!(html.contains("Заходов: 0"));
    assert!(html.contains("ПК-00014"));
}

#[tokio::test]
async fn render_is_read_only() {
    let f = fx().await;
    seed_history(&f).await;
    async fn counts(ctx: &AppCtx) -> (i64, i64, i64) {
        ctx.writer
            .execute(|conn| {
                let q = |sql: &str| {
                    conn.query_row(sql, [], |r| r.get::<_, i64>(0))
                        .map_err(map_rusqlite)
                };
                Ok((
                    q("SELECT COUNT(*) FROM acts")?,
                    q("SELECT COUNT(*) FROM repair_visits")?,
                    q("SELECT COUNT(*) FROM requests")?,
                ))
            })
            .await
            .expect("counts")
    }
    let before = counts(&f.ctx).await;
    let a = render(&f).await.expect("first");
    let b = render(&f).await.expect("second");
    assert_eq!(before, counts(&f.ctx).await);
    // Повторная печать даёт те же номера актов.
    assert!(a.contains("РЕМ-000101") && b.contains("РЕМ-000101"));
}

#[tokio::test]
async fn employee_forbidden_and_missing_device_not_found() {
    let f = fx().await;
    let err = f
        .ctx
        .repair_acts
        .render_device_report(&f.ctx.repairs, f.device, &emp(f.user))
        .await
        .expect_err("employee must be denied");
    assert!(matches!(err, AppError::Forbidden { .. }), "{err:?}");
    let err = f
        .ctx
        .repair_acts
        .render_device_report(&f.ctx.repairs, 999_999, &admin())
        .await
        .expect_err("missing device");
    assert!(matches!(err, AppError::NotFound { .. }), "{err:?}");
}
