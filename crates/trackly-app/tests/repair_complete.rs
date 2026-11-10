//! Phase 41.4, план 13: «Завершить» — D-09, D-10, D-16, D-18.
//!
//! Настоящий `AppCtx::build`. Данные вымышленные («Кабинет 12», «Склад А», «ПК-00014»).

mod entities_support;
mod repair_support;

use entities_support::fixture::{
    admin, make_test_ctx, seed_device, seed_group_type_and_group, seed_place,
};
use repair_support::{emp, manager, seed_user};
use rusqlite::params;
use trackly_app::context::AppCtx;
use trackly_app::dto::repair::{
    RepairCompleteDto, RepairCreateDto, RepairMoveChoiceDto, RepairReceiveDto, RepairSendDto,
};
use trackly_core::error::AppError;
use trackly_infra::error_conversions::map_rusqlite;

const NOW: i64 = 1_700_000_000;

async fn exec1(ctx: &AppCtx, sql: &'static str, a: i64) {
    ctx.writer
        .execute(move |conn| {
            conn.execute(sql, params![a]).map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("exec");
}

async fn storage_place(ctx: &AppCtx, name: &str) -> i64 {
    let p = seed_place(ctx, name).await;
    exec1(ctx, "UPDATE places SET is_storage = 1 WHERE id = ?1", p).await;
    p
}

async fn one_str(ctx: &AppCtx, sql: &'static str, id: i64) -> Option<String> {
    ctx.writer
        .execute(move |conn| {
            conn.query_row(sql, params![id], |r| r.get::<_, Option<String>>(0))
                .map_err(map_rusqlite)
        })
        .await
        .expect("sql")
}

async fn status_code(ctx: &AppCtx, dev: i64) -> String {
    one_str(
        ctx,
        "SELECT ds.code FROM devices d JOIN device_statuses ds ON ds.id = d.status_id \
         WHERE d.id = ?1",
        dev,
    )
    .await
    .expect("code")
}

async fn dev_place(ctx: &AppCtx, dev: i64) -> Option<i64> {
    one_str(
        ctx,
        "SELECT CAST(place_id AS TEXT) FROM devices WHERE id = ?1",
        dev,
    )
    .await
    .map(|s| s.parse().expect("id"))
}

async fn new_request(ctx: &AppCtx, dev: Option<i64>) -> (i64, i64) {
    let existing = ctx
        .writer
        .execute(|conn| {
            conn.query_row("SELECT id FROM users WHERE login = 'requester'", [], |r| {
                r.get::<_, i64>(0)
            })
            .map(Some)
            .or_else(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(map_rusqlite(other)),
            })
        })
        .await
        .expect("lookup");
    let requester = match existing {
        Some(id) => id,
        None => seed_user(ctx, "requester", "Сидоров С.С.").await,
    };
    let r = ctx
        .repairs
        .create(
            RepairCreateDto {
                device_id: dev,
                description: "Не включается".into(),
            },
            &manager(requester),
        )
        .await
        .expect("create");
    (r.request.id, r.request.version)
}

async fn accept_with(ctx: &AppCtx, dev: Option<i64>, choice: RepairMoveChoiceDto) -> (i64, i64) {
    let (req, v) = new_request(ctx, dev).await;
    let r = ctx
        .repairs
        .accept(req, v, choice, &admin())
        .await
        .expect("accept");
    (req, r.request.version)
}

fn complete_dto(req: i64, v: i64, place: Option<i64>) -> RepairCompleteDto {
    RepairCompleteDto {
        request_id: req,
        version: v,
        place_id: place,
        notes: Some("Выполнено".into()),
    }
}

fn validation(e: &AppError) -> bool {
    matches!(e, AppError::Validation { .. })
}

#[tokio::test]
async fn complete_after_move_sets_status_by_return_place() {
    for (storage, expected) in [(false, "в_работе"), (true, "на_складе")] {
        let (ctx, _d) = make_test_ctx().await;
        let room = seed_place(&ctx, "Кабинет 12").await;
        let hold = storage_place(&ctx, "Склад А").await;
        let target = if storage {
            storage_place(&ctx, "Склад Б").await
        } else {
            seed_place(&ctx, "Кабинет 14").await
        };
        let dev = seed_device(&ctx, 1, "ПК-00014", "INV-14", Some(room), None).await;
        let (req, v) = accept_with(
            &ctx,
            Some(dev),
            RepairMoveChoiceDto::Move { place_id: hold },
        )
        .await;
        assert_eq!(status_code(&ctx, dev).await, "на_ремонте");
        let done = ctx
            .repairs
            .complete(complete_dto(req, v, Some(target)), &admin())
            .await
            .expect("complete");
        assert_eq!(done.request.status, "completed");
        assert_eq!(done.request.resolution_notes.as_deref(), Some("Выполнено"));
        assert_eq!(status_code(&ctx, dev).await, expected);
        assert_eq!(dev_place(&ctx, dev).await, Some(target));
    }
}

#[tokio::test]
async fn complete_in_repair_is_rejected_without_changes() {
    let (ctx, _d) = make_test_ctx().await;
    let room = seed_place(&ctx, "Кабинет 12").await;
    let service = seed_place(&ctx, "Сервис Альфа").await;
    let dev = seed_device(&ctx, 1, "ПК-00014", "INV-14", Some(room), None).await;
    let (req, v) = accept_with(&ctx, Some(dev), RepairMoveChoiceDto::Stay).await;
    let sent = ctx
        .repairs
        .send(
            RepairSendDto {
                request_id: req,
                version: v,
                date_utc: NOW,
                giver_name: "Иванов И.И.".into(),
                receiver_name: "Петров П.П.".into(),
                service_place_id: service,
                fault_description: "Не включается".into(),
            },
            &admin(),
        )
        .await
        .expect("send");
    let err = ctx
        .repairs
        .complete(
            complete_dto(req, sent.request.version, Some(room)),
            &admin(),
        )
        .await
        .expect_err("D-10");
    assert!(validation(&err));
    assert_eq!(status_code(&ctx, dev).await, "в_ремонте");
    assert_eq!(dev_place(&ctx, dev).await, Some(service));
    let st = ctx.repairs.get_state(req, &admin()).await.expect("state");
    assert!(st.has_open_visit);
    // Забор без отметки возвращает в работу, и завершение становится доступным.
    let got = ctx
        .repairs
        .receive(
            RepairReceiveDto {
                request_id: req,
                version: sent.request.version,
                date_utc: NOW + 3600,
                received_by_name: "Иванов И.И.".into(),
                place_id: room,
                return_to_workplace: false,
                works_done: None,
                cost: None,
            },
            &admin(),
        )
        .await
        .expect("receive");
    let done = ctx
        .repairs
        .complete(complete_dto(req, got.request.version, Some(room)), &admin())
        .await
        .expect("complete");
    assert_eq!(done.request.status, "completed");
    assert_eq!(status_code(&ctx, dev).await, "в_работе");
}

#[tokio::test]
async fn suggested_return_place_follows_d16() {
    let (ctx, _d) = make_test_ctx().await;
    let room = seed_place(&ctx, "Кабинет 12").await;
    let group_room = seed_place(&ctx, "Кабинет 20").await;
    let hold = storage_place(&ctx, "Склад А").await;

    // Обычное устройство: сохранённое место.
    let plain = seed_device(&ctx, 1, "ПК-00014", "INV-14", Some(room), None).await;
    let (r1, _) = accept_with(
        &ctx,
        Some(plain),
        RepairMoveChoiceDto::Move { place_id: hold },
    )
    .await;
    let st = ctx.repairs.get_state(r1, &admin()).await.expect("state");
    assert_eq!(st.suggested_return_place_id, Some(room));

    // Член группы, не якорный: текущее место группы, оно расходится с origin.
    let (_t, group) = seed_group_type_and_group(&ctx, "Комплект", 1, Some(room)).await;
    let part = seed_device(&ctx, 1, "Монитор", "INV-15", Some(room), Some(group)).await;
    let (r2, _) = accept_with(
        &ctx,
        Some(part),
        RepairMoveChoiceDto::Move { place_id: hold },
    )
    .await;
    exec1(
        &ctx,
        "UPDATE groups SET place_id = ?1 WHERE id = (SELECT group_id FROM group_devices LIMIT 1)",
        group_room,
    )
    .await;
    let st = ctx.repairs.get_state(r2, &admin()).await.expect("state");
    assert_eq!(st.origin_place_id, Some(room));
    assert_eq!(st.suggested_return_place_id, Some(group_room));

    // Якорь: сохранённое место.
    let (_t, g2) = seed_group_type_and_group(&ctx, "Комплект 2", 2, Some(room)).await;
    let anchor = seed_device(&ctx, 1, "ПК-00016", "INV-16", Some(room), Some(g2)).await;
    exec1(&ctx, "UPDATE groups SET anchor_device_id = (SELECT id FROM devices WHERE inventory_number = 'INV-16') WHERE id = ?1", g2).await;
    let (r3, _) = accept_with(
        &ctx,
        Some(anchor),
        RepairMoveChoiceDto::Move { place_id: hold },
    )
    .await;
    let st = ctx.repairs.get_state(r3, &admin()).await.expect("state");
    assert!(st.is_anchor);
    assert_eq!(st.suggested_return_place_id, Some(room));
}

#[tokio::test]
async fn anchor_returns_whole_block_and_nested_is_refused() {
    let (ctx, _d) = make_test_ctx().await;
    let room = seed_place(&ctx, "Кабинет 12").await;
    let hold = storage_place(&ctx, "Склад А").await;
    let (_t, group) = seed_group_type_and_group(&ctx, "Комплект", 1, Some(room)).await;
    let anchor = seed_device(&ctx, 1, "ПК-00014", "INV-14", Some(room), Some(group)).await;
    let part = seed_device(&ctx, 1, "Монитор", "INV-15", Some(room), Some(group)).await;
    exec1(&ctx, "UPDATE groups SET anchor_device_id = (SELECT id FROM devices WHERE inventory_number = 'INV-14') WHERE id = ?1", group).await;
    let (req, v) = accept_with(
        &ctx,
        Some(anchor),
        RepairMoveChoiceDto::Move { place_id: hold },
    )
    .await;
    assert_eq!(dev_place(&ctx, part).await, Some(hold));
    let done = ctx
        .repairs
        .complete(complete_dto(req, v, Some(room)), &admin())
        .await
        .expect("complete");
    assert_eq!(done.request.status, "completed");
    assert_eq!(dev_place(&ctx, anchor).await, Some(room));
    assert_eq!(
        dev_place(&ctx, part).await,
        Some(room),
        "блок вернулся целиком"
    );

    // Вложенная группа: якорь вложенной группы переместить нельзя.
    let (_t, parent) = seed_group_type_and_group(&ctx, "Родитель", 2, Some(room)).await;
    let child_anchor = seed_device(&ctx, 1, "ПК-00020", "INV-20", Some(room), None).await;
    let (req2, v2) = accept_with(
        &ctx,
        Some(child_anchor),
        RepairMoveChoiceDto::Move { place_id: hold },
    )
    .await;
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO groups (type_id, name, seq, place_id, parent_group_id, \
                 anchor_device_id, created_at_utc, updated_at_utc, version) \
                 SELECT type_id, 'Вложенная', 3, place_id, id, ?2, 1700000000, 1700000000, 1 \
                 FROM groups WHERE id = ?1",
                params![parent, child_anchor],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("nested");
    let err = ctx
        .repairs
        .complete(complete_dto(req2, v2, Some(room)), &admin())
        .await
        .expect_err("вложенная группа");
    assert!(validation(&err));
    assert_eq!(dev_place(&ctx, child_anchor).await, Some(hold));
}

#[tokio::test]
async fn no_move_needs_no_place_and_other_without_device_completes() {
    let (ctx, _d) = make_test_ctx().await;
    let room = seed_place(&ctx, "Кабинет 12").await;
    let dev = seed_device(&ctx, 1, "ПК-00014", "INV-14", Some(room), None).await;
    let (req, v) = accept_with(&ctx, Some(dev), RepairMoveChoiceDto::Stay).await;
    assert_eq!(status_code(&ctx, dev).await, "на_ремонте");
    let done = ctx
        .repairs
        .complete(complete_dto(req, v, None), &admin())
        .await
        .expect("complete");
    assert_eq!(done.request.status, "completed");
    assert_eq!(status_code(&ctx, dev).await, "в_работе");
    assert_eq!(dev_place(&ctx, dev).await, Some(room));

    let (req2, v2) = accept_with(&ctx, None, RepairMoveChoiceDto::Stay).await;
    let done = ctx
        .repairs
        .complete(complete_dto(req2, v2, None), &admin())
        .await
        .expect("other");
    assert_eq!(done.request.status, "completed");
}

#[tokio::test]
async fn moved_requires_place_and_employee_forbidden() {
    let (ctx, _d) = make_test_ctx().await;
    let room = seed_place(&ctx, "Кабинет 12").await;
    let hold = storage_place(&ctx, "Склад А").await;
    let dev = seed_device(&ctx, 1, "ПК-00014", "INV-14", Some(room), None).await;
    let (req, v) = accept_with(
        &ctx,
        Some(dev),
        RepairMoveChoiceDto::Move { place_id: hold },
    )
    .await;
    let err = ctx
        .repairs
        .complete(complete_dto(req, v, None), &admin())
        .await
        .expect_err("место обязательно");
    assert!(validation(&err));
    let e = seed_user(&ctx, "emp1", "Козлов К.К.").await;
    assert!(matches!(
        ctx.repairs
            .complete(complete_dto(req, v, Some(room)), &emp(e))
            .await,
        Err(AppError::Forbidden)
    ));
    assert_eq!(dev_place(&ctx, dev).await, Some(hold));
}
