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

// ---------------------------------------------------------------------------
// План 41.4-23 (UAT 6, D-27): без «Тип», «Расположение до отправки» = исходное место
// ---------------------------------------------------------------------------

/// Заявка на ремонт устройства; исходное место и признак перемещения задаются явно.
async fn seed_request(ctx: &AppCtx, device: i64, origin: Option<i64>, moved: bool) -> i64 {
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
    let request_id = created.request.id;
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "UPDATE requests SET repair_origin_place_id = ?1, repair_moved = ?2 WHERE id = ?3",
                params![origin, moved as i64, request_id],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("set repair fields");
    request_id
}

async fn insert_visit(
    ctx: &AppCtx,
    request_id: i64,
    device: i64,
    seq: i64,
    from_place: Option<i64>,
    service_place: i64,
) -> i64 {
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO repair_visits (request_id, device_id, seq, service_place_id, \
                 from_place_id, sent_at_utc, giver_name, receiver_name, fault_description, \
                 act_number, created_at_utc, updated_at_utc, version) \
                 VALUES (?1, ?2, ?3, ?4, ?5, 1781654400, 'Иванов И.И.', 'Петров П.П.', 'Шумит', \
                         ?6, 1781654400, 1781654400, 1)",
                params![
                    request_id,
                    device,
                    seq,
                    service_place,
                    from_place,
                    format!("РЕМ-{seq:06}")
                ],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("insert visit")
}

/// Строка «Расположение до отправки: …» из HTML акта.
fn place_row(html: &str) -> String {
    let start = html
        .find("Расположение до отправки:")
        .expect("place row present");
    let rest = &html[start..];
    rest[..rest.find("</div>").expect("row end")].to_string()
}

#[tokio::test]
async fn render_has_no_type_row_but_keeps_other_device_fields() {
    let (ctx, _dir) = make_test_ctx().await;
    let visit = fixture(&ctx, "РЕМ-000002", "Шумит").await;
    let html = ctx
        .repair_acts
        .render_html(visit, &admin())
        .await
        .expect("render");
    assert!(!html.contains("Тип:"), "{html}");
    for label in [
        "Наименование:",
        "Инвентарный номер:",
        "Серийный номер:",
        "Расположение до отправки:",
    ] {
        assert!(html.contains(label), "нет «{label}»");
    }
}

#[tokio::test]
async fn first_visit_shows_origin_place_not_warehouse() {
    let (ctx, _dir) = make_test_ctx().await;
    let office = seed_place(&ctx, "Кабинет 12").await;
    let warehouse = seed_place(&ctx, "Склад А").await;
    let service = seed_place(&ctx, "Сервис-центр №1").await;
    let device = seed_device(&ctx, 1, "ПК-00014", "INV-14", Some(warehouse), None).await;
    let req = seed_request(&ctx, device, Some(office), true).await;
    let visit = insert_visit(&ctx, req, device, 1, Some(warehouse), service).await;
    let html = ctx
        .repair_acts
        .render_html(visit, &admin())
        .await
        .expect("render");
    let row = place_row(&html);
    assert!(row.contains("Кабинет 12"), "{row}");
    assert!(!row.contains("Склад А"), "{row}");
}

#[tokio::test]
async fn no_move_uses_visit_from_place() {
    let (ctx, _dir) = make_test_ctx().await;
    let office = seed_place(&ctx, "Кабинет 12").await;
    let other = seed_place(&ctx, "Кабинет 99").await;
    let service = seed_place(&ctx, "Сервис-центр №1").await;
    let device = seed_device(&ctx, 1, "ПК-00014", "INV-14", Some(office), None).await;
    // Исходное место заполнено, но перемещения не было — оно не применяется.
    let req = seed_request(&ctx, device, Some(other), false).await;
    let visit = insert_visit(&ctx, req, device, 1, Some(office), service).await;
    let row = place_row(
        &ctx.repair_acts
            .render_html(visit, &admin())
            .await
            .expect("render"),
    );
    assert!(row.contains("Кабинет 12"), "{row}");
    assert!(!row.contains("Кабинет 99"), "{row}");
}

#[tokio::test]
async fn moved_without_origin_falls_back_to_visit_from_place() {
    let (ctx, _dir) = make_test_ctx().await;
    let office = seed_place(&ctx, "Кабинет 12").await;
    let service = seed_place(&ctx, "Сервис-центр №1").await;
    let device = seed_device(&ctx, 1, "ПК-00014", "INV-14", Some(office), None).await;
    let req = seed_request(&ctx, device, None, true).await;
    let visit = insert_visit(&ctx, req, device, 1, Some(office), service).await;
    let row = place_row(
        &ctx.repair_acts
            .render_html(visit, &admin())
            .await
            .expect("render"),
    );
    assert!(row.contains("Кабинет 12"), "{row}");
}

#[tokio::test]
async fn second_visit_uses_its_own_from_place_not_origin() {
    let (ctx, _dir) = make_test_ctx().await;
    let office = seed_place(&ctx, "Кабинет 12").await;
    let warehouse = seed_place(&ctx, "Склад А").await;
    let service = seed_place(&ctx, "Сервис-центр №1").await;
    let device = seed_device(&ctx, 1, "ПК-00014", "INV-14", Some(warehouse), None).await;
    let req = seed_request(&ctx, device, Some(office), true).await;
    let first = insert_visit(&ctx, req, device, 1, Some(warehouse), service).await;
    // Один открытый заход на заявку: первый закрываем возвратом.
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "UPDATE repair_visits SET returned_at_utc = 1781700000 WHERE id = ?1",
                params![first],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("close first visit");
    let second = insert_visit(&ctx, req, device, 2, Some(warehouse), service).await;
    let row1 = place_row(
        &ctx.repair_acts
            .render_html(first, &admin())
            .await
            .expect("render 1"),
    );
    let row2 = place_row(
        &ctx.repair_acts
            .render_html(second, &admin())
            .await
            .expect("render 2"),
    );
    assert!(
        row1.contains("Кабинет 12") && !row1.contains("Склад А"),
        "{row1}"
    );
    assert!(
        row2.contains("Склад А") && !row2.contains("Кабинет 12"),
        "{row2}"
    );
}

#[test]
fn unchanged_v30_copy_is_upgraded_and_customized_copy_is_kept() {
    use trackly_app::pdf::html_templates::{
        upgrade_untouched_defaults_on_startup, DEFAULT_HTML_TEMPLATES,
    };
    const V30: &str = include_str!("../templates/_legacy_defaults/v30/repair_transfer_act.html");
    let current = DEFAULT_HTML_TEMPLATES
        .iter()
        .find(|(f, _)| *f == "repair_transfer_act.html")
        .map(|(_, b)| *b)
        .expect("embedded body");
    // Не вакуумно: снимок и новое тело различаются, «Тип:» только в снимке.
    assert_ne!(V30, current);
    assert!(V30.contains("Тип:") && !current.contains("Тип:"));

    let dir = tempfile::tempdir().expect("tempdir");
    let file = dir.path().join("repair_transfer_act.html");
    std::fs::write(&file, V30).expect("write v30");
    upgrade_untouched_defaults_on_startup(dir.path()).expect("upgrade");
    assert_eq!(std::fs::read_to_string(&file).unwrap(), current);

    let custom = format!("{V30}\n<!-- правка пользователя -->");
    std::fs::write(&file, &custom).expect("write custom");
    upgrade_untouched_defaults_on_startup(dir.path()).expect("upgrade");
    assert_eq!(std::fs::read_to_string(&file).unwrap(), custom);
}

#[tokio::test]
async fn customized_template_using_device_type_still_renders() {
    let (ctx, dir) = make_test_ctx().await;
    let visit = fixture(&ctx, "РЕМ-000003", "Шумит").await;
    let tpl_dir = dir.path().join("templates");
    std::fs::create_dir_all(&tpl_dir).expect("templates dir");
    std::fs::write(
        tpl_dir.join("repair_transfer_act.html"),
        "<p>Тип-копия: {{ device.type }} / {{ device.name }}</p>",
    )
    .expect("write custom");
    let html = ctx
        .repair_acts
        .render_html(visit, &admin())
        .await
        .expect("render");
    assert!(html.contains("Тип-копия:"), "{html}");
    assert!(html.contains("ПК-00014"), "{html}");
}
