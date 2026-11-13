//! Phase 41.4, план 41 (раунд 3): подпись статуса устройства в DTO карточки (D-38)
//! и доступ автора к отменённой, поглощённой и удалённой заявке (D-40).
//!
//! Настоящий `AppCtx::build`. Данные вымышленные: «Иванов И.И.» (автор-сотрудник),
//! «Петров П.П.» (второй автор). Ожидаемое имя статуса берётся SELECT-ом из
//! `device_statuses`, а не литералом: подписи редактируются (D-06).

mod entities_support;
mod repair_support;

use entities_support::fixture::{admin, make_test_ctx, seed_device, seed_place};
use repair_support::{emp, manager, seed_user, seed_user_group};
use rusqlite::params;
use trackly_app::context::AppCtx;
use trackly_app::dto::repair::{
    RepairCancelDto, RepairCreateDto, RepairMergeDto, RepairMoveChoiceDto,
};
use trackly_app::dto::request::RequestCreateDto;
use trackly_core::error::AppError;
use trackly_infra::error_conversions::map_rusqlite;

/// Имя статуса устройства по БД (источник истины для ожиданий).
async fn db_status_name(ctx: &AppCtx, device: i64) -> String {
    ctx.writer
        .execute(move |conn| {
            conn.query_row(
                "SELECT ds.name FROM devices d JOIN device_statuses ds ON ds.id = d.status_id \
                 WHERE d.id = ?1",
                params![device],
                |r| r.get::<_, String>(0),
            )
            .map_err(map_rusqlite)
        })
        .await
        .expect("status name")
}

async fn db_status_code(ctx: &AppCtx, device: i64) -> String {
    ctx.writer
        .execute(move |conn| {
            conn.query_row(
                "SELECT ds.code FROM devices d JOIN device_statuses ds ON ds.id = d.status_id \
                 WHERE d.id = ?1",
                params![device],
                |r| r.get::<_, String>(0),
            )
            .map_err(map_rusqlite)
        })
        .await
        .expect("status code")
}

struct Fx {
    ctx: AppCtx,
    _dir: tempfile::TempDir,
    ivanov: i64,
    petrov: i64,
    device: i64,
}

async fn fixture() -> Fx {
    let (ctx, dir) = make_test_ctx().await;
    let ivanov = seed_user(&ctx, "ivanov", "Иванов И.И.").await;
    let petrov = seed_user(&ctx, "petrov", "Петров П.П.").await;
    let room = seed_place(&ctx, "Кабинет 12").await;
    let device = seed_device(&ctx, 1, "ПК-1", "INV-1", Some(room), None).await;
    seed_user_group(&ctx, ivanov, "АРМ Иванова", &[device]).await;
    Fx {
        ctx,
        _dir: dir,
        ivanov,
        petrov,
        device,
    }
}

#[tokio::test]
async fn employee_state_carries_device_status_name_from_db() {
    let f = fixture().await;
    let created = f
        .ctx
        .repairs
        .create(
            RepairCreateDto {
                device_id: Some(f.device),
                description: "Не включается".into(),
            },
            &emp(f.ivanov),
        )
        .await
        .expect("create");
    let rid = created.request.id;

    let st = f
        .ctx
        .repairs
        .get_state(rid, &emp(f.ivanov))
        .await
        .expect("state");
    assert_eq!(
        st.device_status_code.as_deref(),
        Some(db_status_code(&f.ctx, f.device).await.as_str())
    );
    let name = db_status_name(&f.ctx, f.device).await;
    assert!(!name.is_empty());
    assert_eq!(st.device_status_name.as_deref(), Some(name.as_str()));

    // Результат перехода несёт то же имя, что и актуальная БД после перехода.
    let acc = f
        .ctx
        .repairs
        .accept(
            rid,
            created.request.version,
            RepairMoveChoiceDto::Stay,
            &admin(),
        )
        .await
        .expect("accept");
    let after = db_status_name(&f.ctx, f.device).await;
    assert_eq!(
        acc.state.device_status_name.as_deref(),
        Some(after.as_str())
    );
    let st2 = f
        .ctx
        .repairs
        .get_state(rid, &emp(f.ivanov))
        .await
        .expect("state 2");
    assert_eq!(st2.device_status_name.as_deref(), Some(after.as_str()));
}

#[tokio::test]
async fn employee_state_status_name_none_without_device() {
    let f = fixture().await;
    let created = f
        .ctx
        .repairs
        .create(
            RepairCreateDto {
                device_id: None,
                description: "Шумит".into(),
            },
            &emp(f.ivanov),
        )
        .await
        .expect("create other");
    let st = f
        .ctx
        .repairs
        .get_state(created.request.id, &emp(f.ivanov))
        .await
        .expect("state");
    assert_eq!(st.device_id, None);
    assert_eq!(st.device_status_name, None);
}

#[tokio::test]
async fn employee_author_reads_cancelled_repair() {
    let f = fixture().await;
    let created = f
        .ctx
        .repairs
        .create(
            RepairCreateDto {
                device_id: Some(f.device),
                description: "Не включается".into(),
            },
            &emp(f.ivanov),
        )
        .await
        .expect("create");
    let rid = created.request.id;
    let res = f
        .ctx
        .repairs
        .cancel_by_author(
            RepairCancelDto {
                request_id: rid,
                version: created.request.version,
            },
            &emp(f.ivanov),
        )
        .await
        .expect("cancel");
    assert_eq!(res.request.status, "cancelled");
    let who = emp(f.ivanov);
    assert_eq!(f.ctx.requests.get(rid, &who).await.expect("get").id, rid);
    f.ctx
        .requests
        .get_history(rid, &who)
        .await
        .expect("history");
    f.ctx.repairs.get_state(rid, &who).await.expect("state");
}

#[tokio::test]
async fn employee_author_reads_merged_repair() {
    let f = fixture().await;
    // Открытая заявка персонала на устройстве.
    let b = f
        .ctx
        .repairs
        .create(
            RepairCreateDto {
                device_id: Some(f.device),
                description: "Не включается".into(),
            },
            &manager(f.petrov),
        )
        .await
        .expect("create B");
    f.ctx
        .repairs
        .accept(
            b.request.id,
            b.request.version,
            RepairMoveChoiceDto::Stay,
            &admin(),
        )
        .await
        .expect("accept B");
    // «Другое» сотрудника, в работе.
    let a = f
        .ctx
        .repairs
        .create(
            RepairCreateDto {
                device_id: None,
                description: "Шумит и греется".into(),
            },
            &emp(f.ivanov),
        )
        .await
        .expect("create A");
    let acc = f
        .ctx
        .repairs
        .accept(
            a.request.id,
            a.request.version,
            RepairMoveChoiceDto::Stay,
            &admin(),
        )
        .await
        .expect("accept A");
    f.ctx
        .repairs
        .merge_into_open(
            RepairMergeDto {
                request_id: a.request.id,
                version: acc.request.version,
                device_id: f.device,
            },
            &admin(),
        )
        .await
        .expect("merge");

    let who = emp(f.ivanov);
    let got = f.ctx.requests.get(a.request.id, &who).await.expect("get");
    assert_eq!(got.status, "cancelled");
    let hist = f
        .ctx
        .requests
        .get_history(a.request.id, &who)
        .await
        .expect("history");
    assert!(
        hist.iter()
            .any(|h| h.related_request_id == Some(b.request.id)),
        "в истории поглощённой нет ссылки на цель"
    );
    f.ctx
        .repairs
        .get_state(a.request.id, &who)
        .await
        .expect("state");
}

/// Ремонтную заявку `requests_delete` не удаляет (ею управляет цикл ремонта), поэтому
/// мягкое удаление проверяется на обычной заявке автора: тот же `RequestRepository::get`.
#[tokio::test]
async fn employee_author_gets_not_found_for_soft_deleted_request() {
    let f = fixture().await;
    let created = f
        .ctx
        .requests
        .create(
            RequestCreateDto {
                request_type: "free_form".into(),
                printer_device_id: None,
                cartridge_model_id: None,
                category_id: None,
                description: Some("Скрипит".into()),
            },
            &emp(f.ivanov),
        )
        .await
        .expect("create");
    let rid = created.id;
    f.ctx
        .requests
        .delete(rid, created.version, &admin())
        .await
        .expect("delete");
    match f.ctx.requests.get(rid, &emp(f.ivanov)).await {
        Err(AppError::NotFound { entity, id }) => {
            assert_eq!(entity, "request");
            assert_eq!(id, rid);
        }
        other => panic!("ожидался NotFound, получено {other:?}"),
    }
}

/// Удаление ремонтной заявки персоналом отклоняется: «request N not found» у автора
/// ремонта из удаления взяться не может.
#[tokio::test]
async fn repair_request_cannot_be_soft_deleted_by_staff() {
    let f = fixture().await;
    let created = f
        .ctx
        .repairs
        .create(
            RepairCreateDto {
                device_id: None,
                description: "Шумит".into(),
            },
            &emp(f.ivanov),
        )
        .await
        .expect("create");
    let err = f
        .ctx
        .requests
        .delete(created.request.id, created.request.version, &admin())
        .await
        .expect_err("repair delete must fail");
    assert!(matches!(err, AppError::Validation { .. }), "{err:?}");
    assert!(f
        .ctx
        .requests
        .get(created.request.id, &emp(f.ivanov))
        .await
        .is_ok());
}
