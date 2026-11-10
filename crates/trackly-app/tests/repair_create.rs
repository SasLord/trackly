//! Phase 41.4, план 09: создание ремонтной заявки (REP-01, D-24), BOLA, allowlist.
//!
//! Настоящий `AppCtx::build`. Данные вымышленные: «Иванов И.И.», «Петров П.П.».

mod entities_support;
mod repair_support;

use entities_support::fixture::{admin, make_test_ctx, seed_device};
use repair_support::{emp, manager, seed_user, seed_user_group};
use trackly_app::dto::repair::RepairCreateDto;
use trackly_app::dto::request::RequestCreateDto;
use trackly_core::error::AppError;

fn dto(device: Option<i64>, text: &str) -> RepairCreateDto {
    RepairCreateDto {
        device_id: device,
        description: text.to_string(),
    }
}

#[tokio::test]
async fn employee_creates_for_own_device_and_forbidden_for_foreign() {
    let (ctx, _dir) = make_test_ctx().await;
    let ivanov = seed_user(&ctx, "ivanov", "Иванов И.И.").await;
    let petrov = seed_user(&ctx, "petrov", "Петров П.П.").await;
    let own = seed_device(&ctx, 1, "АРМ Иванова", "INV-1", None, None).await;
    let foreign = seed_device(&ctx, 1, "АРМ Петрова", "INV-2", None, None).await;
    seed_user_group(&ctx, ivanov, "Комплект Иванова", &[own]).await;
    seed_user_group(&ctx, petrov, "Комплект Петрова", &[foreign]).await;

    let res = ctx
        .repairs
        .create(dto(Some(own), "  Не включается  "), &emp(ivanov))
        .await
        .expect("create");
    assert!(!res.joined);
    assert_eq!(res.request.request_type, "repair");
    assert_eq!(res.request.status, "open");
    assert_eq!(res.request.category_id, None);
    assert_eq!(res.request.printer_device_id, Some(own));
    assert_eq!(res.request.description.as_deref(), Some("Не включается"));

    let err = ctx
        .repairs
        .create(dto(Some(foreign), "Чужое"), &emp(ivanov))
        .await
        .expect_err("foreign device");
    assert!(matches!(err, AppError::Forbidden), "{err:?}");
}

#[tokio::test]
async fn other_without_device_and_validation() {
    let (ctx, _dir) = make_test_ctx().await;
    let ivanov = seed_user(&ctx, "ivanov", "Иванов И.И.").await;

    let a = ctx
        .repairs
        .create(dto(None, "Сломалась мышь"), &emp(ivanov))
        .await
        .expect("other");
    let b = ctx
        .repairs
        .create(dto(None, "Сломалась клавиатура"), &emp(ivanov))
        .await
        .expect("other 2");
    assert!(!a.joined && !b.joined);
    assert_ne!(a.request.id, b.request.id);
    assert_eq!(a.request.printer_device_id, None);
    assert_eq!(a.request.category_id, None);

    for bad in ["", "   "] {
        let err = ctx
            .repairs
            .create(dto(None, bad), &emp(ivanov))
            .await
            .expect_err("empty");
        match err {
            AppError::Validation { field, .. } => assert_eq!(field, "description"),
            other => panic!("{other:?}"),
        }
    }
}

#[tokio::test]
async fn request_service_still_rejects_repair_type() {
    let (ctx, _dir) = make_test_ctx().await;
    let ivanov = seed_user(&ctx, "ivanov", "Иванов И.И.").await;
    let err = ctx
        .requests
        .create(
            RequestCreateDto {
                request_type: "repair".into(),
                printer_device_id: None,
                cartridge_model_id: None,
                category_id: None,
                description: Some("Обход".into()),
            },
            &emp(ivanov),
        )
        .await
        .expect_err("allowlist");
    match err {
        AppError::Validation { field, .. } => assert_eq!(field, "request_type"),
        other => panic!("{other:?}"),
    }
}

#[tokio::test]
async fn staff_create_requires_live_device() {
    let (ctx, _dir) = make_test_ctx().await;
    let boss = seed_user(&ctx, "sidorov", "Сидоров С.С.").await;
    let dev = seed_device(&ctx, 1, "Ноутбук", "INV-9", None, None).await;
    let ok = ctx
        .repairs
        .create(dto(Some(dev), "Треснул экран"), &manager(boss))
        .await
        .expect("manager create");
    assert!(!ok.joined);
    let err = ctx
        .repairs
        .create(dto(Some(999_999), "Нет такого"), &admin())
        .await
        .expect_err("missing device");
    assert!(matches!(err, AppError::NotFound { .. }), "{err:?}");
}

#[tokio::test]
async fn device_options_scope_and_no_foreign_request_ids() {
    let (ctx, _dir) = make_test_ctx().await;
    let ivanov = seed_user(&ctx, "ivanov", "Иванов И.И.").await;
    let petrov = seed_user(&ctx, "petrov", "Петров П.П.").await;
    let own = seed_device(&ctx, 1, "АРМ Иванова", "INV-1", None, None).await;
    let foreign = seed_device(&ctx, 1, "АРМ Петрова", "INV-2", None, None).await;
    seed_user_group(&ctx, ivanov, "Комплект Иванова", &[own]).await;
    seed_user_group(&ctx, petrov, "Комплект Петрова", &[foreign]).await;
    // У чужого устройства уже есть открытая заявка: сотруднику она не раскрывается.
    ctx.repairs
        .create(dto(Some(foreign), "Не грузится"), &emp(petrov))
        .await
        .expect("foreign request");

    let mine = ctx
        .repairs
        .device_options(&emp(ivanov))
        .await
        .expect("opts");
    let ids: Vec<i64> = mine.iter().map(|o| o.id).collect();
    assert_eq!(ids, vec![own], "сотрудник видит только своё");
    assert!(!mine[0].in_repair);
    assert_eq!(mine[0].inventory_no.as_deref(), Some("INV-1"));

    let all = ctx
        .repairs
        .device_options(&admin())
        .await
        .expect("admin opts");
    let all_ids: Vec<i64> = all.iter().map(|o| o.id).collect();
    assert!(all_ids.contains(&own) && all_ids.contains(&foreign));
    assert!(all.iter().find(|o| o.id == foreign).unwrap().in_repair);
}
