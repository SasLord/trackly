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
    let svc = AuthService::new(writer, readers, clock);
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
// users_create_read_update_delete
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn users_create_read_update_delete() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_auth_service();
        let admin = Identity::trusted_admin();

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
        let fetched = svc
            .get_user_by_id(dto.id)
            .await
            .expect("get_user_by_id");
        assert_eq!(fetched.id, dto.id);
        assert_eq!(fetched.login, "alice");

        // LIST
        let list = svc
            .list_users(
                UserFilter { search: None },
                Pagination { offset: 0, limit: 50 },
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
                },
                &admin,
            )
            .await
            .expect("update_user");
        assert!(!patched.is_active, "пользователь должен быть деактивирован");
        assert_eq!(patched.version, dto.version + 1, "версия должна инкрементироваться");

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
                Pagination { offset: 0, limit: 50 },
            )
            .await
            .expect("list filtered");
        assert_eq!(res.total, 1);
        assert_eq!(res.items[0].login, "alice");

        // No filter → both
        let all = svc
            .list_users(
                UserFilter { search: None },
                Pagination { offset: 0, limit: 50 },
            )
            .await
            .expect("list all");
        assert_eq!(all.total, 2);
    })
    .await
    .expect("test exceeded 30s budget");
}
