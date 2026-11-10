//! Phase 41.4, план 11: «Акт передачи в ремонт» (D-27, D-28).
//!
//! Настоящий `AppCtx::build`. Данные вымышленные: «Иванов И.И.», «Петров П.П.».

mod entities_support;
mod repair_support;

use entities_support::fixture::{admin, make_test_ctx, seed_device, seed_place};
use repair_support::{emp, manager, seed_user};
use rusqlite::params;
use trackly_app::context::AppCtx;
use trackly_app::dto::repair::RepairCreateDto;
use trackly_core::error::AppError;
use trackly_infra::error_conversions::map_rusqlite;

/// Заход с заданным номером акта, текстом неисправности и сервисом.
async fn seed_visit(
    ctx: &AppCtx,
    device: i64,
    from_place: i64,
    service_place: i64,
    act_number: &str,
    fault: &str,
) -> i64 {
    let staff = seed_user(ctx, "sidorov", "Сидоров С.С.").await;
    let created = ctx
        .repairs
        .create(
            RepairCreateDto {
                device_id: Some(device),
                description: "Не включается".to_string(),
            },
            &manager(staff),
        )
        .await
        .expect("create request");
    let (request_id, act_number, fault) = (
        created.request.id,
        act_number.to_string(),
        fault.to_string(),
    );
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO repair_visits (request_id, device_id, seq, service_place_id, \
                 from_place_id, sent_at_utc, giver_name, receiver_name, fault_description, \
                 act_number, created_at_utc, updated_at_utc, version) \
                 VALUES (?1, ?2, 1, ?3, ?4, 1781654400, 'Иванов И.И.', 'Петров П.П.', ?5, ?6, \
                         1781654400, 1781654400, 1)",
                params![
                    request_id,
                    device,
                    service_place,
                    from_place,
                    fault,
                    act_number
                ],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("insert visit")
}

async fn acts_count(ctx: &AppCtx) -> i64 {
    ctx.writer
        .execute(|conn| {
            conn.query_row("SELECT COUNT(*) FROM acts", [], |r| r.get::<_, i64>(0))
                .map_err(map_rusqlite)
        })
        .await
        .expect("count")
}

async fn fixture(ctx: &AppCtx, act_number: &str, fault: &str) -> i64 {
    let office = seed_place(ctx, "Кабинет 12").await;
    let service = seed_place(ctx, "Сервис-центр №1").await;
    let device = seed_device(ctx, 1, "ПК-00014", "INV-14", Some(office), None).await;
    seed_visit(ctx, device, office, service, act_number, fault).await
}

#[tokio::test]
async fn render_contains_stored_number_people_fault_and_service() {
    let (ctx, _dir) = make_test_ctx().await;
    let visit = fixture(&ctx, "РЕМ-000007", "Не включается, нет изображения").await;
    let before = acts_count(&ctx).await;

    let html = ctx
        .repair_acts
        .render_html(visit, &admin())
        .await
        .expect("render");
    assert!(html.contains("РЕМ-000007"), "{html}");
    assert!(html.contains("Иванов И.И."));
    assert!(html.contains("Петров П.П."));
    assert!(html.contains("Не включается, нет изображения"));
    assert!(html.contains("Сервис-центр №1"));
    assert!(html.contains("ПК-00014"));
    assert!(html.contains("INV-14"));
    assert!(html.contains("Кабинет 12"));
    assert!(html.contains("июня 2026"));
    assert!(!html.contains("Устройство №"));

    // Повторная печать: тот же номер, журнал «Акты» не тронут (D-28).
    let again = ctx
        .repair_acts
        .render_html(visit, &manager(1))
        .await
        .expect("render again");
    assert!(again.contains("РЕМ-000007"));
    assert_eq!(acts_count(&ctx).await, before);
}

#[tokio::test]
async fn fault_with_html_chars_is_escaped() {
    let (ctx, _dir) = make_test_ctx().await;
    let visit = fixture(&ctx, "РЕМ-000001", "<script>alert(1)</script> & \"x\"").await;
    let html = ctx
        .repair_acts
        .render_html(visit, &admin())
        .await
        .expect("render");
    assert!(!html.contains("<script>alert(1)</script>"));
    assert!(html.contains("&lt;script&gt;"));
}

#[tokio::test]
async fn employee_forbidden_and_missing_visit_not_found() {
    let (ctx, _dir) = make_test_ctx().await;
    let visit = fixture(&ctx, "РЕМ-000001", "Шумит").await;
    let petrov = seed_user(&ctx, "petrov", "Петров П.П.").await;

    let err = ctx
        .repair_acts
        .render_html(visit, &emp(petrov))
        .await
        .expect_err("employee");
    assert!(matches!(err, AppError::Forbidden), "{err:?}");

    let err = ctx
        .repair_acts
        .render_html(999_999, &admin())
        .await
        .expect_err("missing");
    assert!(matches!(err, AppError::NotFound { .. }), "{err:?}");
}
