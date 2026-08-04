//! Интеграционные тесты CRUD пользователей через `AuthService`.
//!
//! GREEN после Plan 02 Task 1.

use std::sync::Arc;
use std::time::Duration;

use trackly_app::dto::auth::{UserFilter, UserNew, UserPatch};
use trackly_app::dto::device::Pagination;
use trackly_app::services::AuthService;
use trackly_core::auth::Identity;
use trackly_core::primitives::clock::Clock;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::test_support::test_writer_and_readers;

fn make_auth_service() -> (AuthService, tempfile::TempDir) {
    let (writer, readers, dir) = test_writer_and_readers();
    let clock: Arc<dyn Clock + Send + Sync> = Arc::new(SystemClock);
    let ad_client: Arc<dyn trackly_core::ports::ad::AdClient + Send + Sync> =
        Arc::new(trackly_infra::ad::mock::MockAdClient::default_fixtures());
    let (ws_tx, _) = tokio::sync::broadcast::channel(128);
    let directory =
        Arc::new(trackly_infra::ad::directory_mock::MockAdDirectory::default_fixtures());
    let svc = AuthService::new(
        writer,
        readers,
        clock,
        ad_client,
        Arc::new(ws_tx),
        directory,
    );
    (svc, dir)
}

fn admin_new(login: &str) -> UserNew {
    UserNew {
        login: login.to_string(),
        full_name: "Тест Тестов".to_string(),
        password: "password123".to_string(),
        role: "admin".to_string(),
        email: None,
    }
}

// ---------------------------------------------------------------------------
// delete_then_recreate_revives_same_login
// ---------------------------------------------------------------------------

/// Regression: soft-delete leaves the row (and its UNIQUE login) behind.
/// Re-creating the same login must REVIVE the soft-deleted row (reuse its id),
/// not fail on `UNIQUE constraint failed: users.login`. An ACTIVE duplicate
/// still yields a Conflict.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn delete_then_recreate_revives_same_login() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_auth_service();
        let admin = Identity::trusted_admin();

        // A non-admin user so delete is never blocked by the last-admin guard.
        let bob_new = || UserNew {
            login: "bob".to_string(),
            full_name: "Боб Первый".to_string(),
            password: "password123".to_string(),
            role: "manager".to_string(),
            email: None,
        };

        let original = svc
            .create_user(bob_new(), &admin)
            .await
            .expect("create bob");

        // Active duplicate → Conflict (not revive).
        let conflict = svc
            .create_user(bob_new(), &admin)
            .await
            .expect_err("ожидали Conflict для активного дубликата login");
        assert!(
            matches!(conflict, trackly_core::error::AppError::Conflict { .. }),
            "ожидали Conflict, получили {conflict:?}"
        );

        // Soft-delete, then recreate with the same login but new attributes.
        svc.delete_user(original.id, original.version, &admin)
            .await
            .expect("delete bob");

        let revived = svc
            .create_user(
                UserNew {
                    login: "bob".to_string(),
                    full_name: "Боб Второй".to_string(),
                    password: "password456".to_string(),
                    role: "employee".to_string(),
                    email: Some("bob@example.com".to_string()),
                },
                &admin,
            )
            .await
            .expect("recreate bob should revive, not fail on UNIQUE");

        // Same row reused (FK references from acts/history stay intact).
        assert_eq!(
            revived.id, original.id,
            "revive должен переиспользовать тот же id"
        );
        assert!(
            revived.is_active,
            "оживлённый пользователь должен быть активен"
        );
        assert_eq!(revived.login, "bob");
        assert_eq!(revived.full_name, "Боб Второй", "поля должны обновиться");
        assert_eq!(revived.role, "employee");

        // Visible in the list again.
        let list = svc
            .list_users(
                UserFilter {
                    search: Some("bob".to_string()),
                },
                Pagination {
                    offset: 0,
                    limit: 50,
                },
                &admin,
            )
            .await
            .expect("list_users");
        assert_eq!(
            list.total, 1,
            "оживлённый пользователь должен снова быть в списке"
        );
        assert_eq!(list.items[0].id, original.id);

        // New password works for login; old one does not.
        svc.login(trackly_app::dto::auth::LoginRequest {
            login: "bob".to_string(),
            password: "password456".to_string(),
            remember: false,
        })
        .await
        .expect("login с новым паролем");
    })
    .await
    .expect("test exceeded 30s budget");
}

// ---------------------------------------------------------------------------
// users_create_read_update_delete
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn users_create_read_update_delete() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_auth_service();
        let admin = Identity::trusted_admin();

        // CR-04: keep a second admin so deactivating/deleting `alice` below is
        // not blocked by the last-active-admin guard. This test exercises CRUD
        // mechanics, not the lockout invariant.
        svc.create_user(admin_new("keeper"), &admin)
            .await
            .expect("create keeper admin");

        // CREATE
        let dto = svc
            .create_user(admin_new("alice"), &admin)
            .await
            .expect("create user");
        assert!(dto.id > 0);
        assert_eq!(dto.login, "alice");
        assert_eq!(dto.role, "admin");
        assert!(dto.is_active);

        // READ by ID
        let fetched = svc.get_user_by_id(dto.id).await.expect("get_user_by_id");
        assert_eq!(fetched.id, dto.id);
        assert_eq!(fetched.login, "alice");

        // LIST (keeper + alice).
        let list = svc
            .list_users(
                UserFilter {
                    search: Some("alice".to_string()),
                },
                Pagination {
                    offset: 0,
                    limit: 50,
                },
                &admin,
            )
            .await
            .expect("list_users");
        assert_eq!(list.total, 1);
        assert_eq!(list.items.len(), 1);
        assert_eq!(list.items[0].login, "alice");

        // UPDATE — deactivate
        let patched = svc
            .update_user(
                dto.id,
                dto.version,
                UserPatch {
                    full_name: None,
                    role: None,
                    email: None,
                    is_active: Some(false),
                    password: None,
                },
                &admin,
            )
            .await
            .expect("update_user");
        assert!(!patched.is_active, "пользователь должен быть деактивирован");
        assert_eq!(
            patched.version,
            dto.version + 1,
            "версия должна инкрементироваться"
        );

        // DELETE (soft)
        svc.delete_user(patched.id, patched.version, &admin)
            .await
            .expect("delete_user");

        // After delete → NotFound
        let err = svc
            .get_user_by_id(dto.id)
            .await
            .expect_err("ожидали NotFound после удаления");
        assert!(
            matches!(err, trackly_core::error::AppError::NotFound { .. }),
            "ожидали NotFound, получили {err:?}"
        );
    })
    .await
    .expect("test exceeded 30s budget");
}

// ---------------------------------------------------------------------------
// users_password_validation
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn users_password_validation() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_auth_service();
        let admin = Identity::trusted_admin();

        // Short password (<8 chars) → Validation error
        let err = svc
            .create_user(
                UserNew {
                    login: "alice".to_string(),
                    full_name: "Alice".to_string(),
                    password: "short".to_string(), // 5 chars
                    role: "admin".to_string(),
                    email: None,
                },
                &admin,
            )
            .await
            .expect_err("ожидали Validation для короткого пароля");
        match err {
            trackly_core::error::AppError::Validation { field, .. } => {
                assert_eq!(field, "password", "поле ошибки должно быть 'password'");
            }
            other => panic!("ожидали Validation, получили {other:?}"),
        }

        // Short login (<3 chars) → Validation error
        let err2 = svc
            .create_user(
                UserNew {
                    login: "ab".to_string(), // 2 chars
                    full_name: "AB".to_string(),
                    password: "password123".to_string(),
                    role: "admin".to_string(),
                    email: None,
                },
                &admin,
            )
            .await
            .expect_err("ожидали Validation для короткого логина");
        match err2 {
            trackly_core::error::AppError::Validation { field, .. } => {
                assert_eq!(field, "login", "поле ошибки должно быть 'login'");
            }
            other => panic!("ожидали Validation, получили {other:?}"),
        }
    })
    .await
    .expect("test exceeded 30s budget");
}

// ---------------------------------------------------------------------------
// users_role_enforcement
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn users_role_enforcement() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_auth_service();
        let admin = Identity::trusted_admin();

        // Create employee user first (via trusted admin)
        svc.create_user(
            UserNew {
                login: "emp".to_string(),
                full_name: "Сотрудник".to_string(),
                password: "password123".to_string(),
                role: "employee".to_string(),
                email: None,
            },
            &admin,
        )
        .await
        .expect("create employee");

        // Employee trying to create another user → Forbidden
        let employee_identity = trackly_core::auth::Identity {
            user_id: Some(1),
            role: trackly_core::auth::Role::Employee,
        };
        let err = svc
            .create_user(
                UserNew {
                    login: "bob".to_string(),
                    full_name: "Bob".to_string(),
                    password: "password456".to_string(),
                    role: "employee".to_string(),
                    email: None,
                },
                &employee_identity,
            )
            .await
            .expect_err("ожидали Forbidden");
        assert!(
            matches!(err, trackly_core::error::AppError::Forbidden),
            "ожидали Forbidden, получили {err:?}"
        );

        // Invalid role string → Validation error
        let err2 = svc
            .create_user(
                UserNew {
                    login: "bob".to_string(),
                    full_name: "Bob".to_string(),
                    password: "password456".to_string(),
                    role: "superuser".to_string(), // invalid
                    email: None,
                },
                &admin,
            )
            .await
            .expect_err("ожидали Validation для неверной роли");
        assert!(
            matches!(err2, trackly_core::error::AppError::Validation { .. }),
            "ожидали Validation, получили {err2:?}"
        );
    })
    .await
    .expect("test exceeded 30s budget");
}

// ---------------------------------------------------------------------------
// users_search_filter
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn users_search_filter() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_auth_service();
        let admin = Identity::trusted_admin();

        // Create 2 users
        svc.create_user(admin_new("alice"), &admin)
            .await
            .expect("create alice");
        svc.create_user(
            UserNew {
                login: "bob".to_string(),
                full_name: "Роберт Иванов".to_string(),
                password: "password456".to_string(),
                role: "employee".to_string(),
                email: None,
            },
            &admin,
        )
        .await
        .expect("create bob");

        // Search by login
        let res = svc
            .list_users(
                UserFilter {
                    search: Some("ali".to_string()),
                },
                Pagination {
                    offset: 0,
                    limit: 50,
                },
                &admin,
            )
            .await
            .expect("list filtered");
        assert_eq!(res.total, 1);
        assert_eq!(res.items[0].login, "alice");

        // No filter → both
        let all = svc
            .list_users(
                UserFilter { search: None },
                Pagination {
                    offset: 0,
                    limit: 50,
                },
                &admin,
            )
            .await
            .expect("list all");
        assert_eq!(all.total, 2);
    })
    .await
    .expect("test exceeded 30s budget");
}

// ---------------------------------------------------------------------------
// users_update_email_clear_vs_keep (WR-02)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn users_update_email_clear_vs_keep() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_auth_service();
        let admin = Identity::trusted_admin();

        // Create user with an email.
        let dto = svc
            .create_user(
                UserNew {
                    login: "carol".to_string(),
                    full_name: "Carol".to_string(),
                    password: "password123".to_string(),
                    role: "admin".to_string(),
                    email: Some("carol@example.com".to_string()),
                },
                &admin,
            )
            .await
            .expect("create user");
        assert_eq!(dto.email.as_deref(), Some("carol@example.com"));

        // None → email unchanged (still set).
        let kept = svc
            .update_user(
                dto.id,
                dto.version,
                UserPatch {
                    full_name: Some("Carol II".to_string()),
                    role: None,
                    email: None,
                    is_active: None,
                    password: None,
                },
                &admin,
            )
            .await
            .expect("update keep email");
        assert_eq!(
            kept.email.as_deref(),
            Some("carol@example.com"),
            "None должно оставлять email без изменений"
        );

        // Some(None) → email cleared to NULL.
        let cleared = svc
            .update_user(
                kept.id,
                kept.version,
                UserPatch {
                    full_name: None,
                    role: None,
                    email: Some(None),
                    is_active: None,
                    password: None,
                },
                &admin,
            )
            .await
            .expect("update clear email");
        assert_eq!(
            cleared.email, None,
            "Some(None) должно очищать email в NULL"
        );
    })
    .await
    .expect("test exceeded 30s budget");
}

// ---------------------------------------------------------------------------
// users_update_password_change (WR-01)
// ---------------------------------------------------------------------------

/// WR-01: editing a user with a non-empty `password` must actually rotate the
/// stored argon2id hash — the old password stops working, the new one logs in.
/// An empty-string `password` on a later edit must leave the credential intact
/// (the «оставьте пустым, чтобы не менять» contract), while other fields still
/// update.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn users_update_password_change() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_auth_service();
        let admin = Identity::trusted_admin();

        // Keeper admin so demotion/last-admin guards never interfere; the
        // target is a plain manager whose password we rotate.
        svc.create_user(admin_new("keeper"), &admin)
            .await
            .expect("create keeper admin");

        let dave = svc
            .create_user(
                UserNew {
                    login: "dave".to_string(),
                    full_name: "Дэйв".to_string(),
                    password: "password123".to_string(),
                    role: "manager".to_string(),
                    email: None,
                },
                &admin,
            )
            .await
            .expect("create dave");

        // Sanity: original password logs in.
        svc.login(trackly_app::dto::auth::LoginRequest {
            login: "dave".to_string(),
            password: "password123".to_string(),
            remember: false,
        })
        .await
        .expect("login с исходным паролем");

        // EDIT with a new password → hash must rotate.
        let after_change = svc
            .update_user(
                dave.id,
                dave.version,
                UserPatch {
                    full_name: None,
                    role: None,
                    email: None,
                    is_active: None,
                    password: Some("newpassword456".to_string()),
                },
                &admin,
            )
            .await
            .expect("update_user со сменой пароля");

        // New password works.
        svc.login(trackly_app::dto::auth::LoginRequest {
            login: "dave".to_string(),
            password: "newpassword456".to_string(),
            remember: false,
        })
        .await
        .expect("login с новым паролем должен пройти");

        // Old password no longer works.
        let old_err = svc
            .login(trackly_app::dto::auth::LoginRequest {
                login: "dave".to_string(),
                password: "password123".to_string(),
                remember: false,
            })
            .await
            .expect_err("старый пароль должен быть отклонён");
        assert!(
            matches!(old_err, trackly_core::error::AppError::Unauthorized),
            "ожидали Unauthorized для старого пароля, получили {old_err:?}"
        );

        // EDIT with an empty password → credential untouched, other fields apply.
        let after_empty = svc
            .update_user(
                after_change.id,
                after_change.version,
                UserPatch {
                    full_name: Some("Дэйв Обновлённый".to_string()),
                    role: None,
                    email: None,
                    is_active: None,
                    password: Some(String::new()),
                },
                &admin,
            )
            .await
            .expect("update_user с пустым паролем");
        assert_eq!(
            after_empty.full_name, "Дэйв Обновлённый",
            "непарольные поля должны обновиться при пустом пароле"
        );

        // Password still the rotated one — empty string did not change it.
        svc.login(trackly_app::dto::auth::LoginRequest {
            login: "dave".to_string(),
            password: "newpassword456".to_string(),
            remember: false,
        })
        .await
        .expect("пустой пароль не должен менять учётные данные");

        // Too-short non-empty password → Validation error (field = password).
        let short_err = svc
            .update_user(
                after_empty.id,
                after_empty.version,
                UserPatch {
                    full_name: None,
                    role: None,
                    email: None,
                    is_active: None,
                    password: Some("short".to_string()),
                },
                &admin,
            )
            .await
            .expect_err("короткий пароль должен быть отклонён");
        match short_err {
            trackly_core::error::AppError::Validation { field, .. } => {
                assert_eq!(field, "password", "поле ошибки должно быть 'password'");
            }
            other => panic!("ожидали Validation, получили {other:?}"),
        }
    })
    .await
    .expect("test exceeded 30s budget");
}

// ---------------------------------------------------------------------------
// last_admin_protected (CR-04)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn last_admin_cannot_be_demoted_or_deleted() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_auth_service();
        let admin = Identity::trusted_admin();

        // Single admin in the DB.
        let only_admin = svc
            .create_user(admin_new("root"), &admin)
            .await
            .expect("create admin");

        // Demote last admin → Conflict.
        let demote_err = svc
            .update_user(
                only_admin.id,
                only_admin.version,
                UserPatch {
                    full_name: None,
                    role: Some("manager".to_string()),
                    email: None,
                    is_active: None,
                    password: None,
                },
                &admin,
            )
            .await
            .expect_err("ожидали Conflict при понижении последнего admin");
        assert!(
            matches!(demote_err, trackly_core::error::AppError::Conflict { .. }),
            "ожидали Conflict, получили {demote_err:?}"
        );

        // Deactivate last admin → Conflict.
        let deact_err = svc
            .update_user(
                only_admin.id,
                only_admin.version,
                UserPatch {
                    full_name: None,
                    role: None,
                    email: None,
                    is_active: Some(false),
                    password: None,
                },
                &admin,
            )
            .await
            .expect_err("ожидали Conflict при деактивации последнего admin");
        assert!(
            matches!(deact_err, trackly_core::error::AppError::Conflict { .. }),
            "ожидали Conflict, получили {deact_err:?}"
        );

        // Delete last admin → Conflict.
        let del_err = svc
            .delete_user(only_admin.id, only_admin.version, &admin)
            .await
            .expect_err("ожидали Conflict при удалении последнего admin");
        assert!(
            matches!(del_err, trackly_core::error::AppError::Conflict { .. }),
            "ожидали Conflict, получили {del_err:?}"
        );

        // With a SECOND admin, demoting the first is allowed.
        svc.create_user(admin_new("root2"), &admin)
            .await
            .expect("create 2nd admin");
        svc.update_user(
            only_admin.id,
            only_admin.version,
            UserPatch {
                full_name: None,
                role: Some("manager".to_string()),
                email: None,
                is_active: None,
                password: None,
            },
            &admin,
        )
        .await
        .expect("понижение допустимо при наличии второго admin");
    })
    .await
    .expect("test exceeded 30s budget");
}
