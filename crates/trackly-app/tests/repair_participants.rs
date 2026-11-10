//! Phase 41.4, план 09: присоединение к открытой ремонтной заявке (D-19, D-20).
//!
//! Данные вымышленные: «Иванов И.И.», «Петров П.П.», «Сидоров С.С.».

mod entities_support;
mod repair_support;

use entities_support::fixture::{make_test_ctx, seed_device};
use repair_support::{count, emp, seed_user, seed_user_group};
use rusqlite::params;
use trackly_app::dto::repair::RepairCreateDto;
use trackly_core::domain::requests::{Pagination, RequestFilter};
use trackly_infra::error_conversions::map_rusqlite;

fn dto(device: Option<i64>, text: &str) -> RepairCreateDto {
    RepairCreateDto {
        device_id: device,
        description: text.to_string(),
    }
}

/// Два сотрудника с доступом к одному устройству: устройство в группе Иванова,
/// которая вложена в группу Петрова (состав родителя включает вложенные группы).
async fn shared_device_fixture() -> (
    trackly_app::context::AppCtx,
    tempfile::TempDir,
    i64,
    i64,
    i64,
) {
    let (ctx, dir) = make_test_ctx().await;
    let ivanov = seed_user(&ctx, "ivanov", "Иванов И.И.").await;
    let petrov = seed_user(&ctx, "petrov", "Петров П.П.").await;
    let pc = seed_device(&ctx, 1, "Общий АРМ", "INV-1", None, None).await;
    let g_child = seed_user_group(&ctx, ivanov, "Комплект Иванова", &[pc]).await;
    let g_parent = seed_user_group(&ctx, petrov, "Комплект Петрова", &[]).await;
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "UPDATE groups SET parent_group_id = ?1 WHERE id = ?2",
                params![g_parent, g_child],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("nest");
    (ctx, dir, ivanov, petrov, pc)
}

#[tokio::test]
async fn second_user_joins_instead_of_duplicate() {
    let (ctx, _dir, ivanov, petrov, pc) = shared_device_fixture().await;

    let first = ctx
        .repairs
        .create(dto(Some(pc), "Не включается"), &emp(ivanov))
        .await
        .expect("first");
    assert!(!first.joined);
    let rid = first.request.id;

    let second = ctx
        .repairs
        .create(dto(Some(pc), "Тоже не включается, пищит"), &emp(petrov))
        .await
        .expect("second");
    assert!(second.joined);
    assert_eq!(second.request.id, rid);

    assert_eq!(
        count(
            &ctx,
            "SELECT COUNT(*) FROM requests WHERE request_type = 'repair' AND printer_device_id = ?1",
            pc
        )
        .await,
        1
    );
    assert_eq!(
        count(
            &ctx,
            "SELECT COUNT(*) FROM request_participants WHERE request_id = ?1",
            rid
        )
        .await,
        1 // только Петров: автор числится в requested_by, а не в участниках
    );
    assert_eq!(
        count(
            &ctx,
            "SELECT COUNT(*) FROM audit_log WHERE entity_type = 'request' \
             AND entity_id = ?1 AND action = 'repair_join' \
             AND payload_json LIKE '%пищит%'",
            rid
        )
        .await,
        1
    );

    // Участник видит заявку через get и list.
    ctx.requests.get(rid, &emp(petrov)).await.expect("get");
    let list = ctx
        .requests
        .list(
            RequestFilter::default(),
            Pagination {
                offset: 0,
                limit: 50,
            },
            &emp(petrov),
        )
        .await
        .expect("list");
    assert!(list.items.iter().any(|r| r.id == rid));

    // Пометка «уже в ремонте» в опциях.
    let opts = ctx
        .repairs
        .device_options(&emp(petrov))
        .await
        .expect("opts");
    assert!(opts.iter().find(|o| o.id == pc).expect("opt").in_repair);

    // Повторное присоединение идемпотентно: новая запись не появляется.
    let again = ctx
        .repairs
        .create(dto(Some(pc), "Ещё раз"), &emp(petrov))
        .await
        .expect("again");
    assert!(again.joined);
    assert_eq!(again.request.id, rid);
    assert_eq!(
        count(
            &ctx,
            "SELECT COUNT(*) FROM request_participants WHERE request_id = ?1",
            rid
        )
        .await,
        1
    );
}

#[tokio::test]
async fn new_request_after_close() {
    let (ctx, _dir, ivanov, petrov, pc) = shared_device_fixture().await;
    let first = ctx
        .repairs
        .create(dto(Some(pc), "Первая"), &emp(ivanov))
        .await
        .expect("first");
    let rid = first.request.id;
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "UPDATE requests SET status = 'completed' WHERE id = ?1",
                params![rid],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("close");
    let third = ctx
        .repairs
        .create(dto(Some(pc), "После закрытия"), &emp(petrov))
        .await
        .expect("third");
    assert!(!third.joined);
    assert_ne!(third.request.id, rid);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_creates_yield_one_request() {
    let (ctx, _dir, ivanov, petrov, pc) = shared_device_fixture().await;
    let (a, b) = (ctx.repairs.clone(), ctx.repairs.clone());
    let ta = tokio::spawn(async move { a.create(dto(Some(pc), "Гонка А"), &emp(ivanov)).await });
    let tb = tokio::spawn(async move { b.create(dto(Some(pc), "Гонка Б"), &emp(petrov)).await });
    let ra = ta.await.expect("join").expect("a ok");
    let rb = tb.await.expect("join").expect("b ok");
    assert_eq!(ra.request.id, rb.request.id);
    assert_eq!(
        [ra.joined, rb.joined].iter().filter(|j| **j).count(),
        1,
        "ровно один присоединился"
    );
    assert_eq!(
        count(
            &ctx,
            "SELECT COUNT(*) FROM requests WHERE request_type = 'repair' AND printer_device_id = ?1",
            pc
        )
        .await,
        1
    );
}
