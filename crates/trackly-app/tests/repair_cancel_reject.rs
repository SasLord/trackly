//! Phase 41.4, план 13: выход из цикла — cancel_repair (D-12), reject и отмена автором
//! (D-11), «Указать устройство» (D-24), закрытие обхода через RequestService.
//!
//! Настоящий `AppCtx::build`. Данные вымышленные.

mod entities_support;
mod repair_support;

use entities_support::fixture::{admin, make_test_ctx, seed_device, seed_place};
use repair_support::{emp, manager, seed_user};
use rusqlite::params;
use trackly_app::context::AppCtx;
use trackly_app::dto::repair::{
    RepairAssignDeviceDto, RepairCancelDto, RepairCreateDto, RepairMoveChoiceDto, RepairReceiveDto,
    RepairRejectDto, RepairSendDto,
};
use trackly_app::dto::request::RequestTransitionPayload;
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

async fn req_status(ctx: &AppCtx, req: i64) -> String {
    one_str(ctx, "SELECT status FROM requests WHERE id = ?1", req)
        .await
        .expect("status")
}

async fn visits_count(ctx: &AppCtx, req: i64) -> i64 {
    one_str(
        ctx,
        "SELECT CAST(COUNT(*) AS TEXT) FROM repair_visits WHERE request_id = ?1",
        req,
    )
    .await
    .expect("count")
    .parse()
    .expect("n")
}

async fn requester(ctx: &AppCtx) -> i64 {
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
    match existing {
        Some(id) => id,
        None => seed_user(ctx, "requester", "Сидоров С.С.").await,
    }
}

async fn new_request(ctx: &AppCtx, dev: Option<i64>) -> (i64, i64) {
    let who = requester(ctx).await;
    let r = ctx
        .repairs
        .create(
            RepairCreateDto {
                device_id: dev,
                description: "Не включается".into(),
            },
            &manager(who),
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

fn cancel_dto(req: i64, v: i64) -> RepairCancelDto {
    RepairCancelDto {
        request_id: req,
        version: v,
    }
}

fn reject_dto(req: i64, v: i64) -> RepairRejectDto {
    RepairRejectDto {
        request_id: req,
        version: v,
        notes: Some("Дубликат".into()),
    }
}

fn validation(e: &AppError) -> bool {
    matches!(e, AppError::Validation { .. })
}

async fn send_and_receive(
    ctx: &AppCtx,
    req: i64,
    v: i64,
    service: i64,
    back: i64,
    open: bool,
) -> i64 {
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
    if open {
        return sent.request.version;
    }
    ctx.repairs
        .receive(
            RepairReceiveDto {
                request_id: req,
                version: sent.request.version,
                date_utc: NOW + 3600,
                received_by_name: "Иванов И.И.".into(),
                place_id: back,
                return_to_workplace: false,
                works_done: None,
                cost: None,
            },
            &admin(),
        )
        .await
        .expect("receive")
        .request
        .version
}

#[tokio::test]
async fn cancel_repair_returns_device_without_visits() {
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
    assert_eq!(status_code(&ctx, dev).await, "на_ремонте");
    let done = ctx
        .repairs
        .cancel_repair(cancel_dto(req, v), &admin())
        .await
        .expect("cancel_repair");
    assert_eq!(done.request.status, "cancelled");
    assert_eq!(dev_place(&ctx, dev).await, Some(room));
    assert_eq!(status_code(&ctx, dev).await, "в_работе");
    assert_eq!(visits_count(&ctx, req).await, 0);
}

#[tokio::test]
async fn cancel_repair_refused_after_visit() {
    let (ctx, _d) = make_test_ctx().await;
    let room = seed_place(&ctx, "Кабинет 12").await;
    let service = seed_place(&ctx, "Сервис Альфа").await;
    let hold = storage_place(&ctx, "Склад А").await;
    for open in [true, false] {
        let dev = seed_device(
            &ctx,
            1,
            if open { "ПК-1" } else { "ПК-2" },
            if open { "INV-1" } else { "INV-2" },
            Some(room),
            None,
        )
        .await;
        let (req, v) = accept_with(&ctx, Some(dev), RepairMoveChoiceDto::Stay).await;
        let v = send_and_receive(&ctx, req, v, service, hold, open).await;
        let before = (dev_place(&ctx, dev).await, status_code(&ctx, dev).await);
        let err = ctx
            .repairs
            .cancel_repair(cancel_dto(req, v), &admin())
            .await
            .expect_err("после захода");
        assert!(validation(&err));
        assert_eq!(
            before,
            (dev_place(&ctx, dev).await, status_code(&ctx, dev).await)
        );
        assert_eq!(visits_count(&ctx, req).await, 1);
    }
}

#[tokio::test]
async fn reject_rules_d11() {
    let (ctx, _d) = make_test_ctx().await;
    let room = seed_place(&ctx, "Кабинет 12").await;
    let hold = storage_place(&ctx, "Склад А").await;

    // Из open.
    let dev0 = seed_device(&ctx, 1, "ПК-0", "INV-0", Some(room), None).await;
    let (r0, v0) = new_request(&ctx, Some(dev0)).await;
    let done = ctx
        .repairs
        .reject(reject_dto(r0, v0), &admin())
        .await
        .expect("reject open");
    assert_eq!(done.request.status, "rejected");

    // Из in_progress без перемещения: «Ожидает ремонта» снят.
    let dev1 = seed_device(&ctx, 1, "ПК-1", "INV-1", Some(room), None).await;
    let (r1, v1) = accept_with(&ctx, Some(dev1), RepairMoveChoiceDto::Stay).await;
    assert_eq!(status_code(&ctx, dev1).await, "на_ремонте");
    let done = ctx
        .repairs
        .reject(reject_dto(r1, v1), &admin())
        .await
        .expect("reject in_progress");
    assert_eq!(done.request.status, "rejected");
    assert_eq!(status_code(&ctx, dev1).await, "в_работе");

    // После перемещения: отказ, ничего не изменилось.
    let dev2 = seed_device(&ctx, 1, "ПК-2", "INV-2", Some(room), None).await;
    let (r2, v2) = accept_with(
        &ctx,
        Some(dev2),
        RepairMoveChoiceDto::Move { place_id: hold },
    )
    .await;
    let err = ctx
        .repairs
        .reject(reject_dto(r2, v2), &admin())
        .await
        .expect_err("D-11");
    assert!(validation(&err));
    assert_eq!(req_status(&ctx, r2).await, "in_progress");
    assert_eq!(dev_place(&ctx, dev2).await, Some(hold));
    assert_eq!(status_code(&ctx, dev2).await, "на_ремонте");

    // Сотрудник не может отклонять.
    let e = seed_user(&ctx, "emp1", "Козлов К.К.").await;
    assert!(matches!(
        ctx.repairs.reject(reject_dto(r2, v2), &emp(e)).await,
        Err(AppError::Forbidden)
    ));
}

#[tokio::test]
async fn cancel_by_author_rules_and_ownership() {
    let (ctx, _d) = make_test_ctx().await;
    let room = seed_place(&ctx, "Кабинет 12").await;
    let hold = storage_place(&ctx, "Склад А").await;
    let author = seed_user(&ctx, "author1", "Козлов К.К.").await;
    let stranger = seed_user(&ctx, "stranger", "Громов Г.Г.").await;
    let dev = seed_device(&ctx, 1, "ПК-1", "INV-1", Some(room), None).await;
    // Автор-сотрудник создаёт заявку на устройство в своей области: делаем область через
    // пользовательскую группу.
    let _g = repair_support::seed_user_group(&ctx, author, "АРМ Козлова", &[dev]).await;
    let created = ctx
        .repairs
        .create(
            RepairCreateDto {
                device_id: Some(dev),
                description: "Шумит".into(),
            },
            &emp(author),
        )
        .await
        .expect("create");
    let (req, v) = (created.request.id, created.request.version);

    // Чужой автор: Forbidden.
    assert!(matches!(
        ctx.repairs
            .cancel_by_author(cancel_dto(req, v), &emp(stranger))
            .await,
        Err(AppError::Forbidden)
    ));
    assert_eq!(req_status(&ctx, req).await, "open");
    let done = ctx
        .repairs
        .cancel_by_author(cancel_dto(req, v), &emp(author))
        .await
        .expect("cancel open");
    assert_eq!(done.request.status, "cancelled");

    // in_progress без перемещения: статус устройства снят.
    let dev2 = seed_device(&ctx, 1, "ПК-2", "INV-2", Some(room), None).await;
    let (r2, v2) = accept_with(&ctx, Some(dev2), RepairMoveChoiceDto::Stay).await;
    let who = requester(&ctx).await;
    let done = ctx
        .repairs
        .cancel_by_author(cancel_dto(r2, v2), &emp(who))
        .await
        .expect("cancel in_progress");
    assert_eq!(done.request.status, "cancelled");
    assert_eq!(status_code(&ctx, dev2).await, "в_работе");

    // После перемещения: отказ.
    let dev3 = seed_device(&ctx, 1, "ПК-3", "INV-3", Some(room), None).await;
    let (r3, v3) = accept_with(
        &ctx,
        Some(dev3),
        RepairMoveChoiceDto::Move { place_id: hold },
    )
    .await;
    let err = ctx
        .repairs
        .cancel_by_author(cancel_dto(r3, v3), &emp(who))
        .await
        .expect_err("D-11");
    assert!(validation(&err));
    assert_eq!(req_status(&ctx, r3).await, "in_progress");
}

#[tokio::test]
async fn assign_device_for_other_and_conflict() {
    let (ctx, _d) = make_test_ctx().await;
    let room = seed_place(&ctx, "Кабинет 12").await;
    let free = seed_device(&ctx, 1, "ПК-1", "INV-1", Some(room), None).await;
    let busy = seed_device(&ctx, 1, "ПК-2", "INV-2", Some(room), None).await;
    let (busy_req, _) = new_request(&ctx, Some(busy)).await;

    let (req, v) = accept_with(&ctx, None, RepairMoveChoiceDto::Stay).await;
    // Занятое устройство: conflict_request_id, заявка без изменений.
    let res = ctx
        .repairs
        .assign_device(
            RepairAssignDeviceDto {
                request_id: req,
                version: v,
                device_id: busy,
            },
            &admin(),
        )
        .await
        .expect("conflict");
    assert_eq!(res.conflict_request_id, Some(busy_req));
    assert!(res.request.is_none());
    let still = ctx.repairs.get_state(req, &admin()).await.expect("state");
    assert_eq!(still.device_id, None);

    let res = ctx
        .repairs
        .assign_device(
            RepairAssignDeviceDto {
                request_id: req,
                version: v,
                device_id: free,
            },
            &admin(),
        )
        .await
        .expect("assign");
    assert_eq!(res.conflict_request_id, None);
    assert_eq!(res.request.expect("request").printer_device_id, Some(free));
    assert_eq!(status_code(&ctx, free).await, "на_ремонте");

    // Повторное указание при уже заданном устройстве отклонено.
    let again = ctx
        .repairs
        .assign_device(
            RepairAssignDeviceDto {
                request_id: req,
                version: v + 1,
                device_id: busy,
            },
            &admin(),
        )
        .await
        .expect_err("уже указано");
    assert!(validation(&again));
}

#[tokio::test]
async fn generic_request_service_cannot_drive_repair_cycle() {
    let (ctx, _d) = make_test_ctx().await;
    let room = seed_place(&ctx, "Кабинет 12").await;
    let dev = seed_device(&ctx, 1, "ПК-1", "INV-1", Some(room), None).await;
    let (req, v) = new_request(&ctx, Some(dev)).await;

    let payloads = vec![
        RequestTransitionPayload::Accept {
            request_id: req,
            version: v,
            assigned_to_user_id: None,
        },
        RequestTransitionPayload::Reject {
            request_id: req,
            version: v,
            notes: None,
        },
        RequestTransitionPayload::Complete {
            request_id: req,
            version: v,
            notes: None,
            linked_cartridge_id: None,
        },
    ];
    for p in payloads {
        let err = ctx
            .requests
            .transition(p, &admin())
            .await
            .expect_err("transition");
        assert!(validation(&err), "{err:?}");
    }
    let who = requester(&ctx).await;
    assert!(validation(
        &ctx.requests
            .cancel(req, v, &emp(who))
            .await
            .expect_err("cancel")
    ));
    assert!(validation(
        &ctx.requests
            .delete(req, v, &admin())
            .await
            .expect_err("delete")
    ));
    assert_eq!(req_status(&ctx, req).await, "open");

    // Закрытую ремонтную заявку удалить можно (запрет только для открытых).
    let done = ctx
        .repairs
        .reject(reject_dto(req, v), &admin())
        .await
        .expect("reject");
    ctx.requests
        .delete(req, done.request.version, &admin())
        .await
        .expect("delete closed");
}

// ---- Фаза 41.4, план 22: «Отменить ремонт» члена группы (UAT 15, D-12/D-16) ---------

async fn make_group_member(ctx: &AppCtx, room: i64) -> (i64, i64) {
    let (_t, group) =
        entities_support::fixture::seed_group_type_and_group(ctx, "АРМ-1", 1, Some(room)).await;
    let dev = seed_device(ctx, 1, "ПК-00014", "INV-14", Some(room), Some(group)).await;
    (group, dev)
}

async fn move_group_to(ctx: &AppCtx, group: i64, place: i64) {
    let version: i64 = one_str(
        ctx,
        "SELECT CAST(version AS TEXT) FROM groups WHERE id = ?1",
        group,
    )
    .await
    .expect("version")
    .parse()
    .expect("n");
    ctx.groups
        .move_group(&admin(), group, version, place)
        .await
        .expect("move group");
}

async fn group_place_of(ctx: &AppCtx, group: i64) -> Option<i64> {
    one_str(
        ctx,
        "SELECT CAST(place_id AS TEXT) FROM groups WHERE id = ?1",
        group,
    )
    .await
    .map(|s| s.parse().expect("id"))
}

// Повтор дефекта UAT 15: группа переехала, пока член был в ремонте.
#[tokio::test]
async fn cancel_repair_member_returns_to_current_group_place() {
    let (ctx, _d) = make_test_ctx().await;
    let room = seed_place(&ctx, "Кабинет 12").await;
    let new_room = seed_place(&ctx, "Кабинет 14").await;
    let hold = storage_place(&ctx, "Склад А").await;
    let (group, dev) = make_group_member(&ctx, room).await;
    let (req, v) = accept_with(
        &ctx,
        Some(dev),
        RepairMoveChoiceDto::Move { place_id: hold },
    )
    .await;
    move_group_to(&ctx, group, new_room).await;
    assert_eq!(
        dev_place(&ctx, dev).await,
        Some(hold),
        "член в ремонте остался"
    );

    let st = ctx.repairs.get_state(req, &admin()).await.expect("state");
    assert_eq!(st.suggested_return_place_id, Some(new_room));

    ctx.repairs
        .cancel_repair(cancel_dto(req, v), &admin())
        .await
        .expect("cancel_repair");
    assert_eq!(dev_place(&ctx, dev).await, Some(new_room));
    assert_eq!(
        dev_place(&ctx, dev).await,
        group_place_of(&ctx, group).await
    );
    assert_eq!(status_code(&ctx, dev).await, "в_работе");

    // Журнал «по заявке №N» с корректными «откуда/куда»; аудит с фактическим местом.
    let note = format!("по заявке №{req}");
    let rows: i64 = ctx
        .writer
        .execute(move |conn| {
            conn.query_row(
                "SELECT COUNT(*) FROM place_movements WHERE entity_type = 'device' \
                 AND entity_id = ?1 AND note = ?2 AND from_place_id = ?3 AND to_place_id = ?4",
                params![dev, note, hold, new_room],
                |r| r.get(0),
            )
            .map_err(map_rusqlite)
        })
        .await
        .expect("journal");
    assert_eq!(rows, 1);
    let payload = one_str(
        &ctx,
        "SELECT payload_json FROM audit_log WHERE entity_type = 'request' \
         AND entity_id = ?1 AND action = 'custom:repair_cancel'",
        req,
    )
    .await
    .expect("audit");
    let json: serde_json::Value = serde_json::from_str(&payload).expect("json");
    assert_eq!(json["returned_to_place_id"], serde_json::json!(new_room));
    assert_ne!(json["returned_to_place_id"], serde_json::json!(room));
}

// Статус по месту назначения (D-09): группа переехала на склад.
#[tokio::test]
async fn cancel_repair_member_group_moved_to_storage_gets_in_storage_status() {
    let (ctx, _d) = make_test_ctx().await;
    let room = seed_place(&ctx, "Кабинет 12").await;
    let hold = storage_place(&ctx, "Склад А").await;
    let store_b = storage_place(&ctx, "Склад Б").await;
    let (group, dev) = make_group_member(&ctx, room).await;
    let (req, v) = accept_with(
        &ctx,
        Some(dev),
        RepairMoveChoiceDto::Move { place_id: hold },
    )
    .await;
    move_group_to(&ctx, group, store_b).await;
    ctx.repairs
        .cancel_repair(cancel_dto(req, v), &admin())
        .await
        .expect("cancel_repair");
    assert_eq!(dev_place(&ctx, dev).await, Some(store_b));
    assert_eq!(status_code(&ctx, dev).await, "на_складе");
}

// Якорь корневой группы возвращается блоком на сохранённое место.
#[tokio::test]
async fn cancel_repair_root_anchor_returns_block_to_saved_place() {
    let (ctx, _d) = make_test_ctx().await;
    let room = seed_place(&ctx, "Кабинет 12").await;
    let hold = storage_place(&ctx, "Склад А").await;
    let (group, anchor) = make_group_member(&ctx, room).await;
    let part = seed_device(&ctx, 1, "Монитор", "INV-15", Some(room), Some(group)).await;
    exec1(
        &ctx,
        "UPDATE groups SET anchor_device_id = (SELECT id FROM devices WHERE inventory_number = 'INV-14') WHERE id = ?1",
        group,
    )
    .await;
    let (req, v) = accept_with(
        &ctx,
        Some(anchor),
        RepairMoveChoiceDto::Move { place_id: hold },
    )
    .await;
    assert_eq!(dev_place(&ctx, part).await, Some(hold));
    ctx.repairs
        .cancel_repair(cancel_dto(req, v), &admin())
        .await
        .expect("cancel_repair");
    assert_eq!(dev_place(&ctx, anchor).await, Some(room));
    assert_eq!(dev_place(&ctx, part).await, Some(room));
    assert_eq!(group_place_of(&ctx, group).await, Some(room));
}
