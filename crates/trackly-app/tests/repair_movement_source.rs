//! quick 261121-hed, NEW-3: ремонтный перенос пишет в журнал перемещений источник
//! `repair` и id заявки (`place_movements.request_id`, V056); таймлайн отдаёт оба
//! поля, отчёт «Перемещения» подписывает «по ремонтной заявке №N».
//!
//! Настоящий `AppCtx::build`. Данные вымышленные («Кабинет 12», «Склад А»,
//! «Сидоров С.С.»).

mod entities_support;
mod repair_support;

use entities_support::fixture::{admin, make_test_ctx, seed_device, seed_place};
use repair_support::{manager, seed_user};
use rusqlite::params;
use trackly_app::context::AppCtx;
use trackly_app::dto::repair::{RepairCreateDto, RepairMoveChoiceDto};
use trackly_app::dto::reports::{PeriodDto, ReportFilter};
use trackly_app::tauri_cmds::reports::build_reports_list_movements;
use trackly_infra::error_conversions::map_rusqlite;

async fn storage_place(ctx: &AppCtx, name: &str) -> i64 {
    let p = seed_place(ctx, name).await;
    ctx.writer
        .execute(move |conn| {
            conn.execute("UPDATE places SET is_storage = 1 WHERE id = ?1", params![p])
                .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("storage");
    p
}

async fn seed_attached_cartridge(ctx: &AppCtx, printer: i64, place: i64) -> i64 {
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT OR IGNORE INTO cartridge_models \
                 (id, brand, model, kind_id, created_at_utc, updated_at_utc, version) \
                 VALUES (900, 'HP', 'CE285A', 1, 1700000000, 1700000000, 1)",
                [],
            )
            .map_err(map_rusqlite)?;
            conn.execute(
                "INSERT INTO cartridges (model_id, code, status_id, state_id, place_id, \
                 current_printer_device_id, created_at_utc, updated_at_utc, version) \
                 VALUES (900, 'C-0901', 2, 1, ?1, ?2, 1700000000, 1700000000, 1)",
                params![place, printer],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("cartridge")
}

/// (source, request_id, note) единственной строки журнала сущности.
async fn journal_row(
    ctx: &AppCtx,
    entity_type: &'static str,
    id: i64,
) -> (String, Option<i64>, Option<String>) {
    ctx.writer
        .execute(move |conn| {
            conn.query_row(
                "SELECT source, request_id, note FROM place_movements \
                 WHERE entity_type = ?1 AND entity_id = ?2",
                params![entity_type, id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .map_err(map_rusqlite)
        })
        .await
        .expect("journal row")
}

fn wide_period() -> PeriodDto {
    PeriodDto {
        mode: "range".to_string(),
        year: None,
        month: None,
        date_from: Some("2000-01-01".to_string()),
        date_to: Some("2099-12-31".to_string()),
    }
}

#[tokio::test]
async fn repair_move_writes_repair_source_with_request_id() {
    let (ctx, _d) = make_test_ctx().await;
    let room = seed_place(&ctx, "Кабинет 12").await;
    let store = storage_place(&ctx, "Склад А").await;
    let printer = seed_device(&ctx, 2, "Принтер-9", "INV-90", Some(room), None).await;
    let cart = seed_attached_cartridge(&ctx, printer, room).await;
    let requester = seed_user(&ctx, "requester", "Сидоров С.С.").await;
    let created = ctx
        .repairs
        .create(
            RepairCreateDto {
                device_id: Some(printer),
                description: "Не печатает".into(),
            },
            &manager(requester),
        )
        .await
        .expect("create");
    let req = created.request.id;

    ctx.repairs
        .accept(
            req,
            created.request.version,
            RepairMoveChoiceDto::Move { place_id: store },
            &admin(),
        )
        .await
        .expect("accept");

    // Журнал: устройство — repair + request_id без примечания; каскад картриджа —
    // repair + тот же request_id, примечание «вместе с принтером».
    assert_eq!(
        journal_row(&ctx, "device", printer).await,
        ("repair".to_string(), Some(req), None)
    );
    assert_eq!(
        journal_row(&ctx, "cartridge", cart).await,
        (
            "repair".to_string(),
            Some(req),
            Some("вместе с принтером".to_string())
        )
    );

    // Таймлайн (сервис карточки) отдаёт source и request_id.
    let tl = ctx
        .place_movements
        .get_timeline(&admin(), "device", printer)
        .await
        .expect("timeline");
    assert_eq!(tl.len(), 1, "{tl:?}");
    assert_eq!(tl[0].source, "repair");
    assert_eq!(tl[0].request_id, Some(req));
    let tl_cart = ctx
        .place_movements
        .get_timeline(&admin(), "cartridge", cart)
        .await
        .expect("timeline cartridge");
    assert_eq!(tl_cart.len(), 1, "{tl_cart:?}");
    assert_eq!(tl_cart[0].request_id, Some(req));

    // Отчёт «Перемещения»: подпись «по ремонтной заявке №N».
    let report =
        build_reports_list_movements(&ctx, &admin(), ReportFilter::default(), wide_period())
            .await
            .expect("report");
    let reasons: Vec<(Option<String>, Option<String>)> = report
        .rows
        .iter()
        .map(|r| (r.entity_type_label.clone(), r.reason.clone()))
        .collect();
    let device_reason = format!("по ремонтной заявке №{req}");
    let cart_reason = format!("по ремонтной заявке №{req} · вместе с принтером");
    assert!(
        reasons.contains(&(Some("Устройство".to_string()), Some(device_reason))),
        "{reasons:?}"
    );
    assert!(
        reasons.contains(&(Some("Картридж".to_string()), Some(cart_reason))),
        "{reasons:?}"
    );
}
