//! Phase 41.4, план 12: заход в сервис — отправка, забор, атомарность D-08, сканы.
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
    RepairCreateDto, RepairMoveChoiceDto, RepairReceiveDto, RepairSendDto,
};
use trackly_core::error::AppError;
use trackly_infra::error_conversions::map_rusqlite;

const NOW: i64 = 1_700_000_000;

async fn sql_i64(ctx: &AppCtx, sql: &'static str, arg: i64) -> Option<i64> {
    ctx.writer
        .execute(move |conn| {
            conn.query_row(sql, params![arg], |r| r.get::<_, Option<i64>>(0))
                .map_err(map_rusqlite)
        })
        .await
        .expect("sql")
}

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

async fn status_code(ctx: &AppCtx, dev: i64) -> String {
    ctx.writer
        .execute(move |conn| {
            conn.query_row(
                "SELECT ds.code FROM devices d JOIN device_statuses ds ON ds.id = d.status_id \
                 WHERE d.id = ?1",
                params![dev],
                |r| r.get::<_, String>(0),
            )
            .map_err(map_rusqlite)
        })
        .await
        .expect("status")
}

async fn dev_place(ctx: &AppCtx, dev: i64) -> Option<i64> {
    sql_i64(ctx, "SELECT place_id FROM devices WHERE id = ?1", dev).await
}

async fn req_status(ctx: &AppCtx, req: i64) -> String {
    ctx.writer
        .execute(move |conn| {
            conn.query_row(
                "SELECT status FROM requests WHERE id = ?1",
                params![req],
                |r| r.get::<_, String>(0),
            )
            .map_err(map_rusqlite)
        })
        .await
        .expect("req status")
}

async fn new_request(ctx: &AppCtx, dev: i64) -> (i64, i64) {
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
                device_id: Some(dev),
                description: "Не включается".into(),
            },
            &manager(requester),
        )
        .await
        .expect("create");
    (r.request.id, r.request.version)
}

/// Заявка в работе: (request_id, version) после принятия на месте.
async fn accepted(ctx: &AppCtx, dev: i64) -> (i64, i64) {
    let (req, v) = new_request(ctx, dev).await;
    let r = ctx
        .repairs
        .accept(req, v, RepairMoveChoiceDto::Stay, &admin())
        .await
        .expect("accept");
    (req, r.request.version)
}

fn send_dto(req: i64, v: i64, place: i64) -> RepairSendDto {
    RepairSendDto {
        request_id: req,
        version: v,
        date_utc: NOW,
        giver_name: "Иванов И.И.".into(),
        receiver_name: "Петров П.П.".into(),
        service_place_id: place,
        fault_description: "Не включается".into(),
    }
}

fn recv_dto(req: i64, v: i64, place: i64, to_workplace: bool) -> RepairReceiveDto {
    RepairReceiveDto {
        request_id: req,
        version: v,
        date_utc: NOW + 3600,
        received_by_name: "Иванов И.И.".into(),
        place_id: place,
        return_to_workplace: to_workplace,
        works_done: Some("Замена блока питания".into()),
        cost: Some("1 500,5".into()),
    }
}

fn validation(e: AppError) -> bool {
    matches!(e, AppError::Validation { .. })
}

#[tokio::test]
async fn full_cycle_without_workplace_mark() {
    let (ctx, _d) = make_test_ctx().await;
    let room = seed_place(&ctx, "Кабинет 12").await;
    let service = seed_place(&ctx, "Сервис Альфа").await;
    let store = storage_place(&ctx, "Склад А").await;
    let dev = seed_device(&ctx, 1, "ПК-00014", "INV-14", Some(room), None).await;
    let (req, v) = accepted(&ctx, dev).await;
    assert_eq!(status_code(&ctx, dev).await, "на_ремонте");
    assert_eq!(req_status(&ctx, req).await, "in_progress");

    let sent = ctx
        .repairs
        .send(send_dto(req, v, service), &admin())
        .await
        .expect("send");
    assert_eq!(status_code(&ctx, dev).await, "в_ремонте");
    assert_eq!(req_status(&ctx, req).await, "in_repair");
    assert_eq!(dev_place(&ctx, dev).await, Some(service));
    assert!(sent.state.has_open_visit);
    assert_eq!(sent.state.visits[0].act_number, "РЕМ-000001");
    assert_eq!(sent.state.visits[0].from_place_id, Some(room));
    assert_eq!(sent.state.origin_place_id, Some(room));
    let last = ctx.repairs.last_places(&admin()).await.expect("last");
    assert_eq!(last.service_place_id, Some(service));
    assert_eq!(last.move_place_id, None, "ключи не смешиваются");

    let got = ctx
        .repairs
        .receive(recv_dto(req, sent.request.version, store, false), &admin())
        .await
        .expect("receive");
    assert_eq!(status_code(&ctx, dev).await, "на_ремонте");
    assert_eq!(req_status(&ctx, req).await, "in_progress");
    assert_eq!(dev_place(&ctx, dev).await, Some(store));
    assert!(!got.state.has_open_visit);
    let visit = &got.state.visits[0];
    assert_eq!(visit.cost_kopecks, Some(150_050));
    assert_eq!(visit.returned_to_place_id, Some(store));
    let last = ctx.repairs.last_places(&admin()).await.expect("last");
    assert_eq!(last.move_place_id, Some(store));
    assert_eq!(last.service_place_id, Some(service));
}

#[tokio::test]
async fn receive_with_workplace_mark_completes_by_place() {
    for (storage, expected) in [(false, "в_работе"), (true, "на_складе")] {
        let (ctx, _d) = make_test_ctx().await;
        let room = seed_place(&ctx, "Кабинет 12").await;
        let service = seed_place(&ctx, "Сервис Альфа").await;
        let target = if storage {
            storage_place(&ctx, "Склад А").await
        } else {
            seed_place(&ctx, "Кабинет 14").await
        };
        let dev = seed_device(&ctx, 1, "ПК-00014", "INV-14", Some(room), None).await;
        let (req, v) = accepted(&ctx, dev).await;
        let sent = ctx
            .repairs
            .send(send_dto(req, v, service), &admin())
            .await
            .expect("send");
        let got = ctx
            .repairs
            .receive(recv_dto(req, sent.request.version, target, true), &admin())
            .await
            .expect("receive");
        assert_eq!(got.request.status, "completed");
        assert_eq!(status_code(&ctx, dev).await, expected);
        assert_eq!(dev_place(&ctx, dev).await, Some(target));
        let last = ctx.repairs.last_places(&admin()).await.expect("last");
        assert_eq!(last.move_place_id, None, "с отметкой место не запоминается");
    }
}

#[tokio::test]
async fn two_visits_get_distinct_act_numbers() {
    let (ctx, _d) = make_test_ctx().await;
    let room = seed_place(&ctx, "Кабинет 12").await;
    let service = seed_place(&ctx, "Сервис Альфа").await;
    let store = storage_place(&ctx, "Склад А").await;
    let dev = seed_device(&ctx, 1, "ПК-00014", "INV-14", Some(room), None).await;
    let (req, v) = accepted(&ctx, dev).await;
    let s1 = ctx
        .repairs
        .send(send_dto(req, v, service), &admin())
        .await
        .expect("send 1");
    let r1 = ctx
        .repairs
        .receive(recv_dto(req, s1.request.version, store, false), &admin())
        .await
        .expect("receive 1");
    let s2 = ctx
        .repairs
        .send(send_dto(req, r1.request.version, service), &admin())
        .await
        .expect("send 2");
    let visits = &s2.state.visits;
    assert_eq!(visits.len(), 2);
    assert_eq!((visits[0].seq, visits[1].seq), (1, 2));
    assert_eq!(visits[0].act_number, "РЕМ-000001");
    assert_eq!(visits[1].act_number, "РЕМ-000002");
}

#[tokio::test]
async fn workplace_receive_is_atomic_on_failure() {
    let (ctx, _d) = make_test_ctx().await;
    let room = seed_place(&ctx, "Кабинет 12").await;
    let service = seed_place(&ctx, "Сервис Альфа").await;
    let target = seed_place(&ctx, "Кабинет 14").await;
    let dev = seed_device(&ctx, 1, "ПК-00014", "INV-14", Some(room), None).await;
    let (req, v) = accepted(&ctx, dev).await;
    let sent = ctx
        .repairs
        .send(send_dto(req, v, service), &admin())
        .await
        .expect("send");
    // Место возврата ушло в архив: перемещение внутри транзакции падает после закрытия захода.
    exec1(
        &ctx,
        "UPDATE places SET archived_at_utc = 1700000000 WHERE id = ?1",
        target,
    )
    .await;
    let err = ctx
        .repairs
        .receive(recv_dto(req, sent.request.version, target, true), &admin())
        .await
        .expect_err("должно упасть");
    assert!(validation(err));
    // Заход открыт, устройство в сервисе, заявка in_repair.
    let state = ctx.repairs.get_state(req, &admin()).await.expect("state");
    assert!(state.has_open_visit);
    assert!(state.visits[0].returned_at_utc.is_none());
    assert_eq!(dev_place(&ctx, dev).await, Some(service));
    assert_eq!(status_code(&ctx, dev).await, "в_ремонте");
    assert_eq!(req_status(&ctx, req).await, "in_repair");
}

#[tokio::test]
async fn cost_parsing_and_rejection() {
    let (ctx, _d) = make_test_ctx().await;
    let room = seed_place(&ctx, "Кабинет 12").await;
    let service = seed_place(&ctx, "Сервис Альфа").await;
    let dev = seed_device(&ctx, 1, "ПК-00014", "INV-14", Some(room), None).await;
    let (req, v) = accepted(&ctx, dev).await;
    let mut ver = ctx
        .repairs
        .send(send_dto(req, v, service), &admin())
        .await
        .expect("send")
        .request
        .version;
    for bad in ["abc", "-5"] {
        let mut d = recv_dto(req, ver, room, false);
        d.cost = Some(bad.into());
        assert!(validation(
            ctx.repairs.receive(d, &admin()).await.expect_err(bad)
        ));
    }
    // Пустая стоимость и пустые работы допустимы.
    let mut d = recv_dto(req, ver, room, false);
    d.cost = Some("".into());
    d.works_done = None;
    let r = ctx.repairs.receive(d, &admin()).await.expect("empty ok");
    assert_eq!(r.state.visits[0].cost_kopecks, None);
    ver = r.request.version;
    // Остальные форматы: отдельные заходы.
    for (raw, kop) in [
        ("1500", 150_000),
        ("1500,50", 150_050),
        ("1 500,5", 150_050),
    ] {
        let s = ctx
            .repairs
            .send(send_dto(req, ver, service), &admin())
            .await
            .expect("send");
        let mut d = recv_dto(req, s.request.version, room, false);
        d.cost = Some(raw.into());
        let r = ctx.repairs.receive(d, &admin()).await.expect(raw);
        assert_eq!(r.state.visits.last().unwrap().cost_kopecks, Some(kop));
        ver = r.request.version;
    }
}

#[tokio::test]
async fn invalid_state_transitions_rejected() {
    let (ctx, _d) = make_test_ctx().await;
    let room = seed_place(&ctx, "Кабинет 12").await;
    let service = seed_place(&ctx, "Сервис Альфа").await;
    let dev = seed_device(&ctx, 1, "ПК-00014", "INV-14", Some(room), None).await;
    let (req, v) = new_request(&ctx, dev).await;
    // send из open.
    assert!(validation(
        ctx.repairs
            .send(send_dto(req, v, service), &admin())
            .await
            .expect_err("open")
    ));
    let r = ctx
        .repairs
        .accept(req, v, RepairMoveChoiceDto::Stay, &admin())
        .await
        .expect("accept");
    // receive не из in_repair.
    assert!(validation(
        ctx.repairs
            .receive(recv_dto(req, r.request.version, room, false), &admin())
            .await
            .expect_err("in_progress")
    ));
    let s = ctx
        .repairs
        .send(send_dto(req, r.request.version, service), &admin())
        .await
        .expect("send");
    // Повторная отправка при открытом заходе.
    assert!(validation(
        ctx.repairs
            .send(send_dto(req, s.request.version, service), &admin())
            .await
            .expect_err("open visit")
    ));
}

#[tokio::test]
async fn staff_only_and_stale_version() {
    let (ctx, _d) = make_test_ctx().await;
    let room = seed_place(&ctx, "Кабинет 12").await;
    let service = seed_place(&ctx, "Сервис Альфа").await;
    let dev = seed_device(&ctx, 1, "ПК-00014", "INV-14", Some(room), None).await;
    let (req, v) = accepted(&ctx, dev).await;
    let e = seed_user(&ctx, "emp1", "Козлов К.К.").await;
    assert!(matches!(
        ctx.repairs.send(send_dto(req, v, service), &emp(e)).await,
        Err(AppError::Forbidden)
    ));
    assert!(matches!(
        ctx.repairs
            .receive(recv_dto(req, v, room, false), &emp(e))
            .await,
        Err(AppError::Forbidden)
    ));
    assert!(matches!(
        ctx.repairs.link_scans(1, vec![], &emp(e)).await,
        Err(AppError::Forbidden)
    ));
    let stale = ctx
        .repairs
        .send(send_dto(req, v - 1, service), &admin())
        .await;
    assert!(matches!(
        stale,
        Err(AppError::OptimisticLockMismatch { .. })
    ));
    assert_eq!(req_status(&ctx, req).await, "in_progress");
}

async fn seed_attachment(ctx: &AppCtx, request_id: i64, deleted: bool) -> i64 {
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO attachments (parent_kind, parent_id, original_name, mime, \
                 size_bytes, content_hash, uploaded_by_user_id, created_at_utc, deleted_at_utc) \
                 VALUES ('request', ?1, 'скан.pdf', 'application/pdf', 10, 'abc', 1, 1700000000, ?2)",
                params![request_id, if deleted { Some(1_700_000_001_i64) } else { None }],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("attachment")
}

#[tokio::test]
async fn link_scans_checks_ownership_without_partial_write() {
    let (ctx, _d) = make_test_ctx().await;
    let room = seed_place(&ctx, "Кабинет 12").await;
    let service = seed_place(&ctx, "Сервис Альфа").await;
    let dev = seed_device(&ctx, 1, "ПК-00014", "INV-14", Some(room), None).await;
    let other_dev = seed_device(&ctx, 1, "ПК-00015", "INV-15", Some(room), None).await;
    let (req, v) = accepted(&ctx, dev).await;
    let (other_req, _) = new_request(&ctx, other_dev).await;
    let s = ctx
        .repairs
        .send(send_dto(req, v, service), &admin())
        .await
        .expect("send");
    let visit = s.state.visits[0].id;
    let own = seed_attachment(&ctx, req, false).await;
    let foreign = seed_attachment(&ctx, other_req, false).await;
    let deleted = seed_attachment(&ctx, req, true).await;

    // Свой + чужой: отказ, свой тоже не записан.
    let err = ctx
        .repairs
        .link_scans(visit, vec![own, foreign], &admin())
        .await
        .expect_err("чужое");
    assert!(validation(err));
    assert!(ctx
        .repairs
        .link_scans(visit, vec![deleted], &admin())
        .await
        .is_err());
    let st = ctx.repairs.get_state(req, &admin()).await.expect("state");
    assert_eq!(st.visits[0].scan_count, 0);

    let st = ctx
        .repairs
        .link_scans(visit, vec![own], &admin())
        .await
        .expect("link");
    assert_eq!(st.visits[0].scan_count, 1);
}

#[tokio::test]
async fn anchor_send_and_receive_move_whole_block() {
    let (ctx, _d) = make_test_ctx().await;
    let room = seed_place(&ctx, "Кабинет 12").await;
    let service = seed_place(&ctx, "Сервис Альфа").await;
    let store = storage_place(&ctx, "Склад А").await;
    let (_t, group) = seed_group_type_and_group(&ctx, "Комплект", 1, Some(room)).await;
    let anchor = seed_device(&ctx, 1, "ПК-00014", "INV-14", Some(room), Some(group)).await;
    let part = seed_device(&ctx, 1, "Монитор", "INV-15", Some(room), Some(group)).await;
    exec1(
        &ctx,
        "UPDATE groups SET anchor_device_id = (SELECT id FROM devices WHERE inventory_number = 'INV-14') WHERE id = ?1",
        group,
    )
    .await;
    let (req, v) = accepted(&ctx, anchor).await;
    let s = ctx
        .repairs
        .send(send_dto(req, v, service), &admin())
        .await
        .expect("send");
    assert_eq!(dev_place(&ctx, anchor).await, Some(service));
    assert_eq!(dev_place(&ctx, part).await, Some(service));
    ctx.repairs
        .receive(recv_dto(req, s.request.version, store, false), &admin())
        .await
        .expect("receive");
    assert_eq!(dev_place(&ctx, anchor).await, Some(store));
    assert_eq!(dev_place(&ctx, part).await, Some(store));
}

#[tokio::test]
async fn last_giver_and_received_by_remembered_on_success_only() {
    let (ctx, _d) = make_test_ctx().await;
    let room = seed_place(&ctx, "Кабинет 12").await;
    let service = seed_place(&ctx, "Сервис Альфа").await;
    let store = storage_place(&ctx, "Склад А").await;
    let dev = seed_device(&ctx, 1, "ПК-00014", "INV-14", Some(room), None).await;
    let (req, v) = accepted(&ctx, dev).await;

    let last = ctx.repairs.last_places(&admin()).await.expect("last");
    assert_eq!(last.last_giver_name, None);
    assert_eq!(last.last_received_by_name, None);

    // Неуспешная отправка (пустое «кому выдал») запоминание не меняет.
    let mut bad = send_dto(req, v, service);
    bad.giver_name = "  Сидоров С.С.  ".into();
    bad.receiver_name = "   ".into();
    assert!(validation(
        ctx.repairs.send(bad, &admin()).await.expect_err("bad")
    ));
    let last = ctx.repairs.last_places(&admin()).await.expect("last");
    assert_eq!(last.last_giver_name, None);

    let mut ok = send_dto(req, v, service);
    ok.giver_name = "  Сидоров С.С.  ".into();
    let sent = ctx.repairs.send(ok, &admin()).await.expect("send");
    let last = ctx.repairs.last_places(&admin()).await.expect("last");
    assert_eq!(last.last_giver_name.as_deref(), Some("Сидоров С.С."));
    assert_eq!(last.last_received_by_name, None);

    let mut rec = recv_dto(req, sent.request.version, store, false);
    rec.received_by_name = "Кузнецов К.К.".into();
    ctx.repairs.receive(rec, &admin()).await.expect("receive");
    let last = ctx.repairs.last_places(&admin()).await.expect("last");
    assert_eq!(last.last_received_by_name.as_deref(), Some("Кузнецов К.К."));

    // Значения общие для всех вызывающих (app_settings, не по пользователю).
    let other = seed_user(&ctx, "other_mgr", "Орлов О.О.").await;
    let shared = ctx
        .repairs
        .last_places(&manager(other))
        .await
        .expect("last");
    assert_eq!(shared.last_giver_name.as_deref(), Some("Сидоров С.С."));
    assert_eq!(
        shared.last_received_by_name.as_deref(),
        Some("Кузнецов К.К.")
    );
}

#[tokio::test]
async fn account_name_from_caller_or_first_admin() {
    let (ctx, _d) = make_test_ctx().await;
    let mgr = seed_user(&ctx, "mgr", "Орлов О.О.").await;
    let last = ctx.repairs.last_places(&manager(mgr)).await.expect("last");
    assert_eq!(last.account_name.as_deref(), Some("Орлов О.О."));

    // Доверенный администратор без user_id, пользователей-админов нет.
    let last = ctx.repairs.last_places(&admin()).await.expect("last");
    assert_eq!(last.account_name, None);

    let adm = seed_user(&ctx, "adm", "Администратор А.А.").await;
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "UPDATE users SET role = 'admin' WHERE id = ?1",
                params![adm],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("promote");
    let last = ctx.repairs.last_places(&admin()).await.expect("last");
    assert_eq!(last.account_name.as_deref(), Some("Администратор А.А."));
}
