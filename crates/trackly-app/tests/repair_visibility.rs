//! Phase 41.4, план 06: видимость ремонтной заявки автору и участнику (D-20),
//! счётчик `in_repair` (D-26), скрытие категории «Ремонт техники».
//!
//! Настоящий `AppCtx::build`. Данные вымышленные: «Иванов И.И.» (автор),
//! «Петров П.П.» (участник), «Сидоров С.С.» (посторонний). Фикстуры расходятся:
//! у участника есть собственная обычная заявка, не попадающая в счёт ремонта.

mod entities_support;

use entities_support::fixture::{admin, make_test_ctx};
use rusqlite::params;
use trackly_app::context::AppCtx;
use trackly_app::tauri_cmds::requests::build_requests_list_categories;
use trackly_core::auth::{Identity, Role};
use trackly_core::domain::requests::{Pagination, RequestFilter};
use trackly_core::error::AppError;
use trackly_infra::error_conversions::map_rusqlite;

async fn seed_user(ctx: &AppCtx, login: &str, full: &str) -> i64 {
    let (login, full) = (login.to_string(), full.to_string());
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO users (login, full_name, role, created_at_utc, updated_at_utc) \
                 VALUES (?1, ?2, 'employee', 1700000000, 1700000000)",
                params![login, full],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed user")
}

async fn seed_request(
    ctx: &AppCtx,
    kind: &str,
    status: &str,
    author: i64,
    category_id: Option<i64>,
) -> i64 {
    let (kind, status) = (kind.to_string(), status.to_string());
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO requests (request_type, status, requested_by_user_id, category_id, \
                 created_at_utc, updated_at_utc) VALUES (?1, ?2, ?3, ?4, 1700000000, 1700000000)",
                params![kind, status, author, category_id],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed request")
}

async fn join(ctx: &AppCtx, request_id: i64, user_id: i64) {
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO request_participants (request_id, user_id, joined_at_utc) \
                 VALUES (?1, ?2, 1700000001)",
                params![request_id, user_id],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("join");
}

fn emp(id: i64) -> Identity {
    Identity {
        user_id: Some(id),
        role: Role::Employee,
    }
}

struct Fx {
    ctx: AppCtx,
    _dir: tempfile::TempDir,
    author: i64,
    member: i64,
    outsider: i64,
    repair: i64,
    member_own: i64,
}

async fn fixture() -> Fx {
    let (ctx, dir) = make_test_ctx().await;
    let author = seed_user(&ctx, "ivanov", "Иванов И.И.").await;
    let member = seed_user(&ctx, "petrov", "Петров П.П.").await;
    let outsider = seed_user(&ctx, "sidorov", "Сидоров С.С.").await;
    let repair = seed_request(&ctx, "repair", "in_repair", author, None).await;
    join(&ctx, repair, member).await;
    let member_own = seed_request(&ctx, "free_form", "open", member, None).await;
    Fx {
        ctx,
        _dir: dir,
        author,
        member,
        outsider,
        repair,
        member_own,
    }
}

fn mine(user: i64) -> RequestFilter {
    RequestFilter {
        requested_by_user_id: Some(user),
        ..Default::default()
    }
}

const PAGE: Pagination = Pagination {
    offset: 0,
    limit: 50,
};

#[tokio::test]
async fn author_and_participant_get_repair() {
    let f = fixture().await;
    for u in [f.author, f.member] {
        let dto = f.ctx.requests.get(f.repair, &emp(u)).await.expect("get");
        assert_eq!(dto.id, f.repair);
        f.ctx
            .requests
            .get_history(f.repair, &emp(u))
            .await
            .expect("history");
    }
}

#[tokio::test]
async fn outsider_forbidden_and_invisible() {
    let f = fixture().await;
    let who = emp(f.outsider);
    assert!(matches!(
        f.ctx.requests.get(f.repair, &who).await,
        Err(AppError::Forbidden)
    ));
    assert!(matches!(
        f.ctx.requests.get_history(f.repair, &who).await,
        Err(AppError::Forbidden)
    ));
    let list = f
        .ctx
        .requests
        .list(mine(f.outsider), PAGE, &who)
        .await
        .expect("list");
    assert!(list.items.iter().all(|r| r.id != f.repair));
    assert_eq!(list.total, 0);
    let c = f.ctx.requests.counts(&who).await.expect("counts");
    assert_eq!((c.in_repair, c.all), (0, 0));
}

#[tokio::test]
async fn list_and_counts_for_author_and_participant() {
    let f = fixture().await;
    let a = emp(f.author);
    let list = f
        .ctx
        .requests
        .list(mine(f.author), PAGE, &a)
        .await
        .expect("list");
    assert_eq!(list.items.len(), 1);
    assert_eq!(f.ctx.requests.counts(&a).await.unwrap().in_repair, 1);

    let m = emp(f.member);
    let list = f
        .ctx
        .requests
        .list(mine(f.member), PAGE, &m)
        .await
        .expect("list");
    let ids: Vec<i64> = list.items.iter().map(|r| r.id).collect();
    assert!(ids.contains(&f.repair) && ids.contains(&f.member_own));
    let c = f.ctx.requests.counts(&m).await.unwrap();
    assert_eq!((c.in_repair, c.open, c.all), (1, 1, 2));
}

#[tokio::test]
async fn admin_sees_everything_and_other_types_unchanged() {
    let f = fixture().await;
    let c = f.ctx.requests.counts(&admin()).await.unwrap();
    assert_eq!((c.in_repair, c.open, c.in_progress, c.all), (1, 1, 0, 2));
    let g = f.ctx.requests.get(f.repair, &admin()).await.unwrap();
    assert_eq!(g.status, "in_repair");
    let list = f
        .ctx
        .requests
        .list(
            RequestFilter {
                status: Some("in_repair".into()),
                ..Default::default()
            },
            PAGE,
            &admin(),
        )
        .await
        .unwrap();
    assert_eq!(list.items.len(), 1);
}

#[tokio::test]
async fn repair_category_hidden_but_old_requests_keep_name() {
    let f = fixture().await;
    let cats = build_requests_list_categories(&f.ctx).await.expect("cats");
    assert!(cats.iter().all(|c| c.name != "Ремонт техники"));
    assert!(!cats.is_empty());

    let cat_id: i64 = f
        .ctx
        .readers
        .acquire()
        .query_row(
            "SELECT id FROM request_categories WHERE name = 'Ремонт техники'",
            [],
            |r| r.get(0),
        )
        .expect("category exists");
    let old = seed_request(&f.ctx, "free_form", "open", f.author, Some(cat_id)).await;
    let dto = f.ctx.requests.get(old, &admin()).await.unwrap();
    assert_eq!(dto.category_name.as_deref(), Some("Ремонт техники"));
}
