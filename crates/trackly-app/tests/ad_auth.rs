//! Интеграционные тесты login() local→AD fallback (Phase 9 Plan 02).
//!
//! Внедряет `MockAdClient` через расширенный `AuthService::new` test seam.
//! Покрывает: empty-password trap (Pitfall 1), активный AD fallback
//! (USR-08), AD disabled no-op, Unreachable distinct error, regression
//! для существующего локального пользователя.

use std::sync::Arc;

use rusqlite::params;

use trackly_app::dto::auth::LoginRequest;
use trackly_app::dto::auth::UserNew;
use trackly_app::services::AuthService;
use trackly_core::auth::Identity;
use trackly_core::error::AppError;
use trackly_core::primitives::clock::Clock;
use trackly_infra::ad::mock::MockAdClient;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::test_support::test_writer_and_readers;

/// Создаёт `AuthService` поверх свежего tempfile DB с заданным AD-клиентом.
fn make_auth_service_with_ad(
    ad_client: Arc<dyn trackly_core::ports::ad::AdClient + Send + Sync>,
) -> (AuthService, tempfile::TempDir) {
    let (writer, readers, dir) = test_writer_and_readers();
    let clock: Arc<dyn Clock + Send + Sync> = Arc::new(SystemClock);
    let (ws_tx, _) = tokio::sync::broadcast::channel(128);
    let svc = AuthService::new(writer, readers, clock, ad_client, Arc::new(ws_tx));
    (svc, dir)
}

fn mock_default() -> Arc<dyn trackly_core::ports::ad::AdClient + Send + Sync> {
    Arc::new(MockAdClient::default_fixtures())
}

fn mock_unreachable() -> Arc<dyn trackly_core::ports::ad::AdClient + Send + Sync> {
    Arc::new(MockAdClient::unreachable())
}

/// Сидирует AD-only пользователя (`ad_user=1`, `password_hash=NULL`) напрямую
/// через writer (минуя `create_user`, который требует пароль — AD-пользователи
/// создаются через регистрационный flow в плане 09-03, здесь только seed для
/// теста happy-path fallback).
async fn seed_ad_user(svc: &AuthService, login: &str, full_name: &str, role: &str) {
    let now = SystemClock.unix_seconds();
    let login = login.to_string();
    let full_name = full_name.to_string();
    let role = role.to_string();
    svc.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO users (login, full_name, password_hash, role, ad_user, \
                 is_active, created_at_utc, updated_at_utc, version) \
                 VALUES (?1, ?2, NULL, ?3, 1, 1, ?4, ?4, 1)",
                params![login, full_name, role, now],
            )
            .map_err(|e| AppError::Internal {
                source_chain: format!("seed_ad_user: {e}"),
            })?;
            Ok(())
        })
        .await
        .expect("seed AD user");
}

fn admin_caller() -> Identity {
    Identity::trusted_admin()
}

// ---------------------------------------------------------------------------
// Test 1: empty/whitespace password rejected BEFORE any AD bind (Pitfall 1).
// ---------------------------------------------------------------------------

#[tokio::test]
async fn empty_password_rejected() {
    let (svc, _dir) = make_auth_service_with_ad(mock_default());
    svc.set_ad_enabled(true, &admin_caller())
        .await
        .expect("enable AD");
    seed_ad_user(&svc, "us100", "Иванов Иван Иванович", "employee").await;

    // Empty password.
    let result = svc
        .login(LoginRequest {
            login: "us100".to_string(),
            password: "".to_string(),
        })
        .await;
    assert!(
        matches!(result, Err(AppError::Unauthorized)),
        "empty password must be rejected with Unauthorized, got {result:?}"
    );

    // Whitespace-only password.
    let result = svc
        .login(LoginRequest {
            login: "us100".to_string(),
            password: "   ".to_string(),
        })
        .await;
    assert!(
        matches!(result, Err(AppError::Unauthorized)),
        "whitespace-only password must be rejected with Unauthorized, got {result:?}"
    );
}

// ---------------------------------------------------------------------------
// Test 2: active AD user logs in via fallback (USR-08 + USR-10).
// ---------------------------------------------------------------------------

#[tokio::test]
async fn ad_fallback_active_user() {
    let (svc, _dir) = make_auth_service_with_ad(mock_default());
    svc.set_ad_enabled(true, &admin_caller())
        .await
        .expect("enable AD");
    seed_ad_user(&svc, "us100", "Иванов Иван Иванович", "employee").await;

    let result = svc
        .login(LoginRequest {
            login: "us100".to_string(),
            password: "Passw0rd!".to_string(),
        })
        .await;

    let dto = result.expect("AD fallback login should succeed for active user");
    assert_eq!(dto.login, "us100");
    assert_eq!(dto.full_name, "Иванов Иван Иванович");
}

// ---------------------------------------------------------------------------
// Test 3: AD disabled by default — no fallback, AdClient never reached.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn ad_disabled_no_fallback() {
    // ad_enabled defaults to false — никаких set_ad_enabled вызовов.
    let (svc, _dir) = make_auth_service_with_ad(mock_default());
    seed_ad_user(&svc, "us100", "Иванов Иван Иванович", "employee").await;

    let result = svc
        .login(LoginRequest {
            login: "us100".to_string(),
            password: "Passw0rd!".to_string(),
        })
        .await;

    assert!(
        matches!(result, Err(AppError::Unauthorized)),
        "AD disabled must not fall back, expected Unauthorized, got {result:?}"
    );

    // Same for a login that doesn't even exist locally.
    let result = svc
        .login(LoginRequest {
            login: "nobody".to_string(),
            password: "whatever".to_string(),
        })
        .await;
    assert!(matches!(result, Err(AppError::Unauthorized)));
}

// ---------------------------------------------------------------------------
// Test 4: AD unreachable surfaces as a distinct error, not Unauthorized.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn ad_unreachable_distinct_error() {
    let (svc, _dir) = make_auth_service_with_ad(mock_unreachable());
    svc.set_ad_enabled(true, &admin_caller())
        .await
        .expect("enable AD");

    let result = svc
        .login(LoginRequest {
            login: "us100".to_string(),
            password: "Passw0rd!".to_string(),
        })
        .await;

    match result {
        Err(AppError::ServiceUnavailable { service }) => assert_eq!(service, "ad"),
        other => panic!("expected ServiceUnavailable, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Test 5: regression — existing local user still logs in via local path.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn local_user_still_works() {
    let (svc, _dir) = make_auth_service_with_ad(mock_default());
    svc.set_ad_enabled(true, &admin_caller())
        .await
        .expect("enable AD (must not interfere with local path)");

    let caller = admin_caller();
    svc.create_user(
        UserNew {
            login: "localadmin".to_string(),
            full_name: "Local Admin".to_string(),
            password: "localpassword123".to_string(),
            role: "admin".to_string(),
            email: None,
        },
        &caller,
    )
    .await
    .expect("create local user");

    let result = svc
        .login(LoginRequest {
            login: "localadmin".to_string(),
            password: "localpassword123".to_string(),
        })
        .await;

    let dto = result.expect("local login must still work unchanged");
    assert_eq!(dto.login, "localadmin");

    // Wrong password for a known local user must NOT trigger AD fallback
    // (AdClient has no "localadmin" fixture — if it were reached it would
    // 404/BadCreds inside the mock anyway, but we want plain Unauthorized
    // via the local path, never touching AD for a known-login-wrong-password).
    let result = svc
        .login(LoginRequest {
            login: "localadmin".to_string(),
            password: "wrongpassword".to_string(),
        })
        .await;
    assert!(matches!(result, Err(AppError::Unauthorized)));
}
