//! Системный пользователь «Администратор» для десктопа (quick 261117-w2q).
//!
//! Настоящий `AppCtx::build` (ensure отрабатывает при старте). Данные вымышленные:
//! «Иванов И.И.», «Петров П.П.».

mod entities_support;
mod repair_support;

use entities_support::fixture::{make_test_ctx, seed_device};
use repair_support::seed_user;
use trackly_app::context::AppCtx;
use trackly_app::dto::auth::{LoginRequest, UserFilter, UserNew, UserPatch};
use trackly_app::dto::device::Pagination;
use trackly_app::dto::repair::RepairCreateDto;
use trackly_app::dto::request::RequestCreateDto;
use trackly_core::auth::{Identity, Role};
use trackly_core::error::AppError;
use trackly_infra::error_conversions::map_rusqlite;

fn page() -> Pagination {
    Pagination {
        offset: 0,
        limit: 100,
    }
}

fn patch() -> UserPatch {
    UserPatch {
        full_name: None,
        role: None,
        email: None,
        is_active: None,
        password: None,
    }
}

/// (id, login, full_name, role, is_active, password_hash is NULL, version)
fn system_rows(ctx: &AppCtx) -> Vec<(i64, String, String, String, i64, bool, i64)> {
    let conn = ctx.readers.acquire();
    let mut stmt = conn
        .prepare(
            "SELECT id, login, full_name, role, is_active, password_hash IS NULL, version \
             FROM users WHERE is_system = 1",
        )
        .unwrap();
    stmt.query_map([], |r| {
        Ok((
            r.get(0)?,
            r.get(1)?,
            r.get(2)?,
            r.get(3)?,
            r.get(4)?,
            r.get::<_, i64>(5)? == 1,
            r.get(6)?,
        ))
    })
    .unwrap()
    .map(|x| x.unwrap())
    .collect()
}

#[tokio::test]
async fn fresh_db_has_exactly_one_idempotent_system_admin() {
    let (ctx, _dir) = make_test_ctx().await;
    let rows = system_rows(&ctx);
    assert_eq!(rows.len(), 1);
    let (id, login, name, role, active, no_pw, version) = rows[0].clone();
    assert_eq!(login, "admin");
    assert_eq!(name, "Администратор");
    assert_eq!(role, "admin");
    assert_eq!(active, 1);
    assert!(no_pw);

    let again = ctx.auth.ensure_system_admin().await.expect("ensure");
    assert_eq!(again, id);
    let rows = system_rows(&ctx);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].6, version, "идемпотентно: version не растёт");

    // Виден в списке как обычный пользователь.
    let caller = ctx.auth.system_admin_identity().await.unwrap();
    assert_eq!(caller.user_id, Some(id));
    let list = ctx
        .auth
        .list_users(UserFilter { search: None }, page(), &caller)
        .await
        .unwrap();
    assert!(list.items.iter().any(|u| u.id == id));
}

#[tokio::test]
async fn system_admin_closes_first_run_wizard() {
    let (ctx, _dir) = make_test_ctx().await;
    // Системный «Администратор» создан при старте -> мастер не нужен.
    assert!(!ctx.auth.needs_bootstrap().await.unwrap());
    let err = ctx
        .auth
        .bootstrap_first_admin(
            UserNew {
                login: "ivanov".into(),
                full_name: "Иванов И.И.".into(),
                password: "correct-horse-1".into(),
                role: "admin".into(),
                email: None,
            },
            None,
        )
        .await
        .expect_err("bootstrap closed");
    assert!(matches!(err, AppError::Conflict { .. }), "{err:?}");
    let n: i64 = ctx
        .readers
        .acquire()
        .query_row(
            "SELECT COUNT(*) FROM users WHERE login = 'ivanov'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(n, 0);
}

#[tokio::test]
async fn login_taken_by_employee_gets_admin_system_and_employee_untouched() {
    let (ctx, _dir) = make_test_ctx().await;
    // Имитируем «старую БД»: системной строки нет, логин admin занят сотрудником.
    ctx.writer
        .execute(|conn| {
            conn.execute("DELETE FROM users WHERE is_system = 1", [])
                .map_err(map_rusqlite)?;
            conn.execute(
                "INSERT INTO users (login, full_name, role, created_at_utc, updated_at_utc) \
                 VALUES ('admin', 'Петров П.П.', 'employee', 1700000000, 1700000000)",
                [],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .unwrap();
    ctx.auth.ensure_system_admin().await.unwrap();
    let rows = system_rows(&ctx);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].1, "admin-system");
    let conn = ctx.readers.acquire();
    let (role, name, sys): (String, String, i64) = conn
        .query_row(
            "SELECT role, full_name, is_system FROM users WHERE login = 'admin'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap();
    assert_eq!(
        (role.as_str(), name.as_str(), sys),
        ("employee", "Петров П.П.", 0)
    );
}

#[test]
fn v053_on_pre_existing_db_keeps_user_id_1() {
    let mut conn = rusqlite::Connection::open_in_memory().unwrap();
    trackly_infra::db::migrations::run_up_to(&mut conn, 52).unwrap();
    conn.execute(
        "INSERT INTO users (id, login, full_name, role, created_at_utc, updated_at_utc) \
         VALUES (1, 'admin', 'Иванов И.И.', 'employee', 1700000000, 1700000000)",
        [],
    )
    .unwrap();
    trackly_infra::db::migrations::run(&mut conn).unwrap();
    let (login, sys): (String, i64) = conn
        .query_row("SELECT login, is_system FROM users WHERE id = 1", [], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })
        .unwrap();
    assert_eq!((login.as_str(), sys), ("admin", 0));
    let v: i64 = conn
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .unwrap();
    // Не ниже V053: последующие миграции поднимают версию дальше.
    assert!(v >= 53, "user_version {v}");
}

#[tokio::test]
async fn ensure_repairs_damaged_system_row_without_second_one() {
    let (ctx, _dir) = make_test_ctx().await;
    ctx.writer
        .execute(|conn| {
            conn.execute(
                "UPDATE users SET deleted_at_utc = 1, is_active = 0, role = 'employee' \
                 WHERE is_system = 1",
                [],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .unwrap();
    ctx.auth.ensure_system_admin().await.unwrap();
    let rows = system_rows(&ctx);
    assert_eq!(rows.len(), 1);
    assert_eq!((rows[0].3.as_str(), rows[0].4), ("admin", 1));
}

#[tokio::test]
async fn system_admin_is_protected_but_password_can_be_set() {
    let (ctx, _dir) = make_test_ctx().await;
    let caller = ctx.auth.system_admin_identity().await.unwrap();
    let id = caller.user_id.unwrap();
    let ver = system_rows(&ctx)[0].6;

    // T-w2q-01: пока пароль не задан, войти нельзя.
    let err = ctx
        .auth
        .login(LoginRequest {
            login: "admin".into(),
            password: "whatever-123".into(),
            remember: false,
        })
        .await
        .expect_err("no password yet");
    assert!(matches!(err, AppError::Unauthorized), "{err:?}");

    for p in [
        UserPatch {
            role: Some("employee".into()),
            ..patch()
        },
        UserPatch {
            is_active: Some(false),
            ..patch()
        },
    ] {
        let err = ctx
            .auth
            .update_user(id, ver, p, &caller)
            .await
            .expect_err("protected");
        assert!(matches!(err, AppError::Validation { .. }), "{err:?}");
    }
    let err = ctx
        .auth
        .delete_user(id, ver, &caller)
        .await
        .expect_err("protected delete");
    assert!(matches!(err, AppError::Validation { .. }), "{err:?}");

    ctx.auth
        .update_user(
            id,
            ver,
            UserPatch {
                password: Some("new-password-1".into()),
                ..patch()
            },
            &caller,
        )
        .await
        .expect("set password");
    ctx.auth
        .login(LoginRequest {
            login: "admin".into(),
            password: "new-password-1".into(),
            remember: false,
        })
        .await
        .expect("login with new password");
    ctx.auth
        .reset_password(id, "another-pass-2".into(), &caller)
        .await
        .expect("reset");
}

#[tokio::test]
async fn desktop_create_on_fresh_db_attributes_system_admin() {
    let (ctx, _dir) = make_test_ctx().await;
    let caller = ctx.auth.system_admin_identity().await.unwrap();
    let sys_id = caller.user_id.unwrap();
    let dev = seed_device(&ctx, 1, "АРМ Иванова", "INV-1", None, None).await;

    let rep = ctx
        .repairs
        .create(
            RepairCreateDto {
                device_id: Some(dev),
                description: "Не включается".into(),
            },
            &caller,
        )
        .await
        .expect("repair create");
    assert_eq!(rep.request.requested_by_user_id, sys_id);

    let req = ctx
        .requests
        .create(
            RequestCreateDto {
                request_type: "free_form".into(),
                printer_device_id: None,
                cartridge_model_id: None,
                category_id: None,
                description: Some("Нужна замена мыши".into()),
            },
            &caller,
        )
        .await
        .expect("request create");
    assert_eq!(req.requested_by_user_id, sys_id);
}

#[tokio::test]
async fn caller_without_user_id_is_rejected_not_substituted() {
    let (ctx, _dir) = make_test_ctx().await;
    let err = ctx
        .repairs
        .create(
            RepairCreateDto {
                device_id: None,
                description: "Сломалась мышь".into(),
            },
            &Identity::trusted_admin(),
        )
        .await
        .expect_err("no author");
    match err {
        AppError::Validation { field, .. } => assert_eq!(field, "requested_by"),
        other => panic!("{other:?}"),
    }
}

#[tokio::test]
async fn fk_failure_is_not_masked_as_race() {
    let (ctx, _dir) = make_test_ctx().await;
    let dev = seed_device(&ctx, 1, "АРМ Петрова", "INV-2", None, None).await;
    let _ = seed_user(&ctx, "petrov", "Петров П.П.").await;
    let ghost = Identity {
        user_id: Some(99999),
        role: Role::Admin,
    };
    let err = ctx
        .repairs
        .create(
            RepairCreateDto {
                device_id: Some(dev),
                description: "Не печатает".into(),
            },
            &ghost,
        )
        .await
        .expect_err("fk");
    match err {
        AppError::Conflict { reason } => {
            assert!(reason.contains("FOREIGN KEY constraint failed"), "{reason}");
        }
        other => panic!("{other:?}"),
    }
}

/// Сброс пароля несуществующему пользователю — NotFound, а не молчаливый «успех»
/// (наблюдение quick 261121-fdw: HTTP отдавал 200 для userId 999999).
#[tokio::test]
async fn reset_password_unknown_user_is_not_found() {
    let (ctx, _dir) = make_test_ctx().await;
    let caller = ctx.auth.system_admin_identity().await.unwrap();
    let err = ctx
        .auth
        .reset_password(999_999, "another-pass-2".into(), &caller)
        .await
        .expect_err("unknown user");
    assert!(
        matches!(
            err,
            AppError::NotFound {
                entity: "user",
                id: 999_999
            }
        ),
        "{err:?}"
    );
}
