//! Интеграционные тесты `AuthService` — auth flow, bootstrap, desktop attribution.
//!
//! GREEN после Plan 02 Task 1 (AuthService реализован).

use std::sync::Arc;
use std::time::Duration;

use trackly_app::dto::auth::UserFilter;
use trackly_app::dto::auth::{LoginRequest, UserNew};
use trackly_app::dto::device::Pagination;
use trackly_app::services::AuthService;
use trackly_core::auth::Identity;
use trackly_core::primitives::clock::Clock;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::test_support::test_writer_and_readers;

/// Создаёт тестовый `AuthService` поверх свежего tempfile DB.
fn make_auth_service() -> (AuthService, tempfile::TempDir) {
    let (writer, readers, dir) = test_writer_and_readers();
    let clock: Arc<dyn Clock + Send + Sync> = Arc::new(SystemClock);
    let ad_client: Arc<dyn trackly_core::ports::ad::AdClient + Send + Sync> =
        Arc::new(trackly_infra::ad::mock::MockAdClient::default_fixtures());
    let (ws_tx, _) = tokio::sync::broadcast::channel(128);
    let svc = AuthService::new(writer, readers, clock, ad_client, Arc::new(ws_tx));
    (svc, dir)
}

fn admin_new(login: &str, password: &str) -> UserNew {
    UserNew {
        login: login.to_string(),
        full_name: "Тестовый Администратор".to_string(),
        password: password.to_string(),
        role: "admin".to_string(),
        email: None,
    }
}

// ---------------------------------------------------------------------------
// bootstrap_creates_admin
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn bootstrap_creates_admin() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_auth_service();
        let caller = Identity::trusted_admin();

        // Initially: no admin → needs_bootstrap = true
        let needs = svc.needs_bootstrap().await.expect("needs_bootstrap");
        assert!(needs, "needs_bootstrap должен быть true на пустой БД");

        // Create first admin
        let dto = svc
            .create_user(admin_new("alice", "password123"), &caller)
            .await
            .expect("create admin");
        assert!(dto.id > 0);
        assert_eq!(dto.role, "admin");

        // After creating admin: needs_bootstrap = false
        let needs_after = svc.needs_bootstrap().await.expect("needs_bootstrap after");
        assert!(
            !needs_after,
            "needs_bootstrap должен быть false после создания admin"
        );
    })
    .await
    .expect("test exceeded 30s budget");
}

// ---------------------------------------------------------------------------
// login_success_and_failure
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn login_success_and_failure() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_auth_service();
        let caller = Identity::trusted_admin();

        svc.create_user(admin_new("alice", "password123"), &caller)
            .await
            .expect("create user");

        // Correct password → success
        let dto = svc
            .login(LoginRequest {
                login: "alice".to_string(),
                password: "password123".to_string(),
                remember: false,
            })
            .await
            .expect("login должен успешно завершиться");
        assert_eq!(dto.login, "alice");

        // Wrong password → Unauthorized
        let err = svc
            .login(LoginRequest {
                login: "alice".to_string(),
                password: "wrongpassword".to_string(),
                remember: false,
            })
            .await
            .expect_err("ожидали ошибку при неверном пароле");
        assert!(
            matches!(err, trackly_core::error::AppError::Unauthorized),
            "ожидали Unauthorized, получили {err:?}"
        );

        // Unknown login → Unauthorized
        let err2 = svc
            .login(LoginRequest {
                login: "nonexistent".to_string(),
                password: "anything".to_string(),
                remember: false,
            })
            .await
            .expect_err("ожидали ошибку для несуществующего логина");
        assert!(
            matches!(err2, trackly_core::error::AppError::Unauthorized),
            "ожидали Unauthorized для несуществующего логина, получили {err2:?}"
        );
    })
    .await
    .expect("test exceeded 30s budget");
}

// ---------------------------------------------------------------------------
// desktop_identity_attribution (D-Desktop-01)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn desktop_identity_attribution() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_auth_service();
        let caller = Identity::trusted_admin();

        // 0 admins → None
        let id0 = svc.desktop_identity().await;
        assert_eq!(id0.user_id, None, "0 admins → user_id = None");

        // Create 1 admin
        let dto1 = svc
            .create_user(admin_new("alice", "password123"), &caller)
            .await
            .expect("create admin");

        // 1 admin → Some(id)
        let id1 = svc.desktop_identity().await;
        assert_eq!(
            id1.user_id,
            Some(dto1.id),
            "1 admin → user_id = Some(id), ожидали Some({}), получили {:?}",
            dto1.id,
            id1.user_id
        );

        // Create 2nd admin
        svc.create_user(admin_new("bob", "password456"), &caller)
            .await
            .expect("create 2nd admin");

        // 2 admins → None
        let id2 = svc.desktop_identity().await;
        assert_eq!(id2.user_id, None, "2 admins → user_id = None");
    })
    .await
    .expect("test exceeded 30s budget");
}

// ---------------------------------------------------------------------------
// desktop_lock_enabled_read_write (D-Desktop-02)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn desktop_lock_enabled_read_write() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_auth_service();
        let admin = Identity::trusted_admin();

        // Default: false (seeded as '0' in V018)
        let initial = svc.get_desktop_lock_enabled().await.expect("get initial");
        assert!(!initial, "начальное значение должно быть false");

        // Set to true
        svc.set_desktop_lock_enabled(true, &admin)
            .await
            .expect("set true");

        let after_true = svc
            .get_desktop_lock_enabled()
            .await
            .expect("get after true");
        assert!(after_true, "значение должно быть true после установки");

        // Set back to false
        svc.set_desktop_lock_enabled(false, &admin)
            .await
            .expect("set false");

        let after_false = svc
            .get_desktop_lock_enabled()
            .await
            .expect("get after false");
        assert!(!after_false, "значение должно быть false после сброса");
    })
    .await
    .expect("test exceeded 30s budget");
}

// ---------------------------------------------------------------------------
// argon2_params_in_spawn_blocking (T-05-03)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn argon2_hash_stored_not_plaintext() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_auth_service();
        let caller = Identity::trusted_admin();

        let dto = svc
            .create_user(admin_new("alice", "password123"), &caller)
            .await
            .expect("create user");

        // Verify password stored as hash (not plaintext)
        let readers = svc.readers.clone();
        let uid = dto.id;
        let stored_hash = tokio::task::spawn_blocking(move || -> String {
            let conn = readers.acquire();
            conn.query_row(
                "SELECT password_hash FROM users WHERE id = ?1",
                rusqlite::params![uid],
                |r| r.get(0),
            )
            .expect("get password_hash")
        })
        .await
        .expect("spawn_blocking");

        assert!(
            !stored_hash.contains("password123"),
            "password_hash не должен содержать plaintext пароль"
        );
        assert!(
            stored_hash.starts_with("$argon2id$"),
            "password_hash должен начинаться с $argon2id$, получили: {stored_hash}"
        );
    })
    .await
    .expect("test exceeded 30s budget");
}

// ---------------------------------------------------------------------------
// set_desktop_lock_requires_admin
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn set_desktop_lock_requires_admin() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_auth_service();
        let admin_caller = Identity::trusted_admin();

        // Create employee user
        svc.create_user(
            UserNew {
                login: "emp".to_string(),
                full_name: "Сотрудник".to_string(),
                password: "password123".to_string(),
                role: "employee".to_string(),
                email: None,
            },
            &admin_caller,
        )
        .await
        .expect("create employee");

        // Employee calling set_desktop_lock_enabled → Forbidden
        let employee_identity = trackly_core::auth::Identity {
            user_id: Some(1),
            role: trackly_core::auth::Role::Employee,
        };
        let err = svc
            .set_desktop_lock_enabled(true, &employee_identity)
            .await
            .expect_err("ожидали Forbidden");
        assert!(
            matches!(err, trackly_core::error::AppError::Forbidden),
            "ожидали Forbidden, получили {err:?}"
        );
    })
    .await
    .expect("test exceeded 30s budget");
}

// Suppress unused import warnings for list_users (used indirectly)
#[allow(dead_code)]
fn _use_pagination() {
    let _ = Pagination {
        offset: 0,
        limit: 50,
    };
    let _ = UserFilter { search: None };
}
