//! quick 261121-hed: WS-сигналы ремонтных заявок.
//!
//! * NEW-7 — `link_scans` после коммита шлёт `RequestStatusChanged` участникам заявки
//!   (открытая карточка ремонта у второго клиента перечитывает состояние); при ошибке
//!   валидации (чужое вложение) события нет.
//! * NEW-4 — новая ремонтная заявка на устройство шлёт `EmployeeScopeChanged {}`
//!   (видно только Employee, без данных); присоединение и заявка без устройства — нет.
//!
//! Настоящий `AppCtx::build`, подписка на `ctx.ws_broadcast` ДО мутации, `try_recv`
//! после. Данные вымышленные («Иванов И.И.», «Петров П.П.», «Кабинет 12»).

mod entities_support;
mod repair_support;

use entities_support::fixture::{admin, drain, make_test_ctx, seed_device, seed_place};
use repair_support::{emp, manager, seed_user, seed_user_group};
use rusqlite::params;
use tokio::sync::broadcast::Receiver;
use trackly_app::context::AppCtx;
use trackly_app::dto::printer::WsEvent;
use trackly_app::dto::repair::{RepairCreateDto, RepairMoveChoiceDto, RepairSendDto};
use trackly_infra::error_conversions::map_rusqlite;

const NOW: i64 = 1_700_000_000;

fn collect(rx: &mut Receiver<WsEvent>) -> Vec<WsEvent> {
    let mut out = Vec::new();
    while let Ok(ev) = rx.try_recv() {
        out.push(ev);
    }
    out
}

fn scope_changed_count(evs: &[WsEvent]) -> usize {
    evs.iter()
        .filter(|e| matches!(e, WsEvent::EmployeeScopeChanged {}))
        .count()
}

fn create_dto(dev: Option<i64>) -> RepairCreateDto {
    RepairCreateDto {
        device_id: dev,
        description: "Не включается".into(),
    }
}

async fn seed_attachment(ctx: &AppCtx, request_id: i64) -> i64 {
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO attachments (parent_kind, parent_id, original_name, mime, \
                 size_bytes, content_hash, uploaded_by_user_id, created_at_utc) \
                 VALUES ('request', ?1, 'скан.pdf', 'application/pdf', 10, 'abc', 1, ?2)",
                params![request_id, NOW],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("attachment")
}

#[tokio::test]
async fn link_scans_notifies_request_participants() {
    let (ctx, _d) = make_test_ctx().await;
    let room = seed_place(&ctx, "Кабинет 12").await;
    let service = seed_place(&ctx, "Сервис Альфа").await;
    let dev = seed_device(&ctx, 1, "ПК-00021", "INV-21", Some(room), None).await;
    let other_dev = seed_device(&ctx, 1, "ПК-00022", "INV-22", Some(room), None).await;
    let ivanov = seed_user(&ctx, "ivanov", "Иванов И.И.").await;
    let created = ctx
        .repairs
        .create(create_dto(Some(dev)), &manager(ivanov))
        .await
        .expect("create");
    let req = created.request.id;
    // Петров присоединяется к заявке — становится её участником.
    let petrov = seed_user(&ctx, "petrov", "Петров П.П.").await;
    let joined = ctx
        .repairs
        .create(create_dto(Some(dev)), &manager(petrov))
        .await
        .expect("join");
    assert!(joined.joined && joined.request.id == req);
    let other = ctx
        .repairs
        .create(create_dto(Some(other_dev)), &manager(ivanov))
        .await
        .expect("create other")
        .request
        .id;
    let acc = ctx
        .repairs
        .accept(
            req,
            joined.request.version,
            RepairMoveChoiceDto::Stay,
            &admin(),
        )
        .await
        .expect("accept");
    let sent = ctx
        .repairs
        .send(
            RepairSendDto {
                request_id: req,
                version: acc.request.version,
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
    let visit = sent.state.visits[0].id;
    let own = seed_attachment(&ctx, req).await;
    let foreign = seed_attachment(&ctx, other).await;

    let mut rx = ctx.ws_broadcast.subscribe();
    drain(&mut rx);

    // Ошибка валидации (чужое вложение) — события нет.
    assert!(ctx
        .repairs
        .link_scans(visit, vec![foreign], &admin())
        .await
        .is_err());
    let evs = collect(&mut rx);
    assert!(
        !evs.iter()
            .any(|e| matches!(e, WsEvent::RequestStatusChanged { .. })),
        "{evs:?}"
    );

    ctx.repairs
        .link_scans(visit, vec![own], &admin())
        .await
        .expect("link");
    let evs = collect(&mut rx);
    let hit = evs.iter().find_map(|e| match e {
        WsEvent::RequestStatusChanged {
            request_id,
            requested_by_user_id,
            participant_user_ids,
            ..
        } if *request_id == req => Some((*requested_by_user_id, participant_user_ids.clone())),
        _ => None,
    });
    let (author, participants) = hit.unwrap_or_else(|| panic!("нет RequestStatusChanged: {evs:?}"));
    assert_eq!(author, ivanov);
    assert!(participants.contains(&petrov), "{participants:?}");
}

#[tokio::test]
async fn new_device_repair_request_signals_employee_scope() {
    let (ctx, _d) = make_test_ctx().await;
    let room = seed_place(&ctx, "Кабинет 12").await;
    let dev = seed_device(&ctx, 1, "ПК-00031", "INV-31", Some(room), None).await;
    let ivanov = seed_user(&ctx, "ivanov", "Иванов И.И.").await;
    let petrov = seed_user(&ctx, "petrov", "Петров П.П.").await;
    seed_user_group(&ctx, ivanov, "АРМ Иванова", &[dev]).await;

    let mut rx = ctx.ws_broadcast.subscribe();
    drain(&mut rx);
    let r = ctx
        .repairs
        .create(create_dto(Some(dev)), &emp(ivanov))
        .await
        .expect("create");
    assert!(!r.joined);
    let evs = collect(&mut rx);
    assert!(
        evs.iter().any(
            |e| matches!(e, WsEvent::NewRequest { request_id, .. } if *request_id == r.request.id)
        ),
        "{evs:?}"
    );
    assert_eq!(scope_changed_count(&evs), 1, "{evs:?}");
    let ev = WsEvent::EmployeeScopeChanged {};
    assert!(ev.is_visible_to(&emp(petrov)), "сотрудник видит сигнал");
    assert!(!ev.is_visible_to(&manager(petrov)), "менеджер — нет");
    assert!(!ev.is_visible_to(&admin()), "администратор — нет");

    // Присоединение к открытой заявке того же устройства — сигнала нет.
    let joined = ctx
        .repairs
        .create(create_dto(Some(dev)), &emp(ivanov))
        .await
        .expect("join");
    assert!(joined.joined);
    assert_eq!(scope_changed_count(&collect(&mut rx)), 0);
}

#[tokio::test]
async fn repair_request_without_device_sends_no_scope_signal() {
    let (ctx, _d) = make_test_ctx().await;
    let petrov = seed_user(&ctx, "petrov", "Петров П.П.").await;
    let mut rx = ctx.ws_broadcast.subscribe();
    drain(&mut rx);
    let r = ctx
        .repairs
        .create(create_dto(None), &manager(petrov))
        .await
        .expect("create");
    assert!(!r.joined);
    let evs = collect(&mut rx);
    assert!(
        evs.iter().any(|e| matches!(e, WsEvent::NewRequest { .. })),
        "{evs:?}"
    );
    assert_eq!(scope_changed_count(&evs), 0, "{evs:?}");
}
