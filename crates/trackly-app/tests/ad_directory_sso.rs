//! Интеграционные тесты SSO-01/SSO-03 (Phase 31 Plan 04).
//!
//! Покрывает полный путь `AuthService::sso_login` -> `AdDirectory` ->
//! `on_ad_bind_success` -> role-mapped `UserDto`, используя `MockAdDirectory`
//! фикстуры (без живого AD). Помимо end-to-end сценариев, содержит прямую
//! unit-level проверку типизированной ошибки `DirectoryError::Unreachable`
//! (defense in depth против будущего сворачивания типа в boolean).
//!
//! Используются ТОЛЬКО уже существующие placeholder-идентичности из
//! `directory_mock.rs` (us100/us200) + новый неиспользуемый в фикстурах
//! placeholder-логин us300 (не заведён нигде, для фейл-клоуз сценариев) —
//! никаких реальных имён/доменов.

use std::sync::Arc;

use trackly_app::services::AuthService;
use trackly_core::auth::Identity;
use trackly_core::error::AppError;
use trackly_core::ports::ad_directory::{AdDirectory, DirectoryError};
use trackly_core::primitives::clock::Clock;
use trackly_infra::ad::directory_mock::MockAdDirectory;
use trackly_infra::ad::mock::MockAdClient;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::test_support::test_writer_and_readers;

/// Создаёт `AuthService` поверх свежего tempfile DB с ЯВНО заданными
/// `ad_client` И `directory` (в отличие от `ad_auth.rs`'s
/// `make_auth_service_with_ad`, которая жёстко использует
/// `MockAdDirectory::default_fixtures()` внутри) — нужно для инъекции
/// `MockAdDirectory::unreachable()` в fail-closed сценариях. Независимый
/// helper, не переиспользует `ad_auth.rs`'s (small-independent-fixtures
/// convention, см. 31-PATTERNS.md).
fn make_auth_service_with_directory(
    ad_client: Arc<dyn trackly_core::ports::ad::AdClient + Send + Sync>,
    directory: Arc<dyn AdDirectory + Send + Sync>,
) -> (AuthService, tempfile::TempDir) {
    let (writer, readers, dir) = test_writer_and_readers();
    let clock: Arc<dyn Clock + Send + Sync> = Arc::new(SystemClock);
    let (ws_tx, _) = tokio::sync::broadcast::channel(128);
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

/// `AdDirectory` не выполняет password-bind, поэтому `sso_login` никогда
/// не обращается к `AdClient` — но `AuthService::new` всё равно требует
/// какое-то значение для этого поля.
fn mock_ad_client_default() -> Arc<dyn trackly_core::ports::ad::AdClient + Send + Sync> {
    Arc::new(MockAdClient::default_fixtures())
}

fn mock_directory_default() -> Arc<dyn AdDirectory + Send + Sync> {
    Arc::new(MockAdDirectory::default_fixtures())
}

fn mock_directory_unreachable() -> Arc<dyn AdDirectory + Send + Sync> {
    Arc::new(MockAdDirectory::unreachable())
}

fn admin_caller() -> Identity {
    Identity::trusted_admin()
}

// ---------------------------------------------------------------------------
// Test 1 (SSO-01): known SSO login resolves the real ФИО, not the bare login.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn sso_login_resolves_known_user_display_name() {
    let (svc, _dir) =
        make_auth_service_with_directory(mock_ad_client_default(), mock_directory_default());
    svc.set_ad_enabled(true, &admin_caller())
        .await
        .expect("enable AD");
    svc.set_ad_auto_accept(true, &admin_caller())
        .await
        .expect("enable auto-accept");

    let dto = svc
        .sso_login("us100", "us100")
        .await
        .expect("SSO login for a known, auto-accepted user should succeed");

    assert_eq!(
        dto.full_name, "Иванов Иван Иванович",
        "full_name must be the AD-resolved ФИО, not the bare login"
    );
    // SSO-03 assertion piggybacked on the same auto-register call (see
    // Test 3's docstring for why this is not duplicated separately here).
    assert_eq!(dto.role, "manager");
}

// ---------------------------------------------------------------------------
// Test 2 (SSO-01): unknown SSO login falls back to the login itself.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn sso_login_unknown_user_falls_back_to_login() {
    let (svc, _dir) =
        make_auth_service_with_directory(mock_ad_client_default(), mock_directory_default());
    svc.set_ad_enabled(true, &admin_caller())
        .await
        .expect("enable AD");
    svc.set_ad_auto_accept(true, &admin_caller())
        .await
        .expect("enable auto-accept");

    let dto = svc
        .sso_login("us999", "us999")
        .await
        .expect("SSO login for an unknown-to-the-directory user must not panic/error");

    assert_eq!(
        dto.full_name, "us999",
        "unknown-to-directory login must fall back to the bare login itself"
    );
}

// ---------------------------------------------------------------------------
// Test 3 (SSO-03): mapped role auto-assigned on FIRST (auto-register) login.
// ---------------------------------------------------------------------------
//
// NOTE: this scenario is exercised via `sso_login_resolves_known_user_display_name`
// above (same `us100` auto-register call asserts BOTH full_name and role) per the
// plan's own Behavior bullet ("the SAME us100 auto-register call above also
// asserts..."). This test re-runs it independently (fresh DB) so the role
// assertion has its own named, standalone test per the plan's required test list.

#[tokio::test]
async fn sso_login_auto_registers_with_mapped_role_on_first_login() {
    let (svc, _dir) =
        make_auth_service_with_directory(mock_ad_client_default(), mock_directory_default());
    svc.set_ad_enabled(true, &admin_caller())
        .await
        .expect("enable AD");
    svc.set_ad_auto_accept(true, &admin_caller())
        .await
        .expect("enable auto-accept");

    let dto = svc
        .sso_login("us100", "us100")
        .await
        .expect("first-login auto-register should succeed");

    assert_eq!(
        dto.role, "manager",
        "mapped AD-group role must be auto-assigned on first login, without manual admin confirmation"
    );
}

// ---------------------------------------------------------------------------
// Test 4 (SSO-03 regression): no mapped role still defaults to 'employee'.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn sso_login_defaults_to_employee_when_no_group_matches() {
    let (svc, _dir) =
        make_auth_service_with_directory(mock_ad_client_default(), mock_directory_default());
    svc.set_ad_enabled(true, &admin_caller())
        .await
        .expect("enable AD");
    svc.set_ad_auto_accept(true, &admin_caller())
        .await
        .expect("enable auto-accept");

    // us200 is a fresh, not-yet-seeded login in MockAdDirectory::default_fixtures()
    // with role: None (no configured group matched).
    let dto = svc
        .sso_login("us200", "us200")
        .await
        .expect("first-login auto-register should succeed even with no mapped group");

    assert_eq!(
        dto.role, "employee",
        "pre-existing default-role behavior must be unchanged when no AD group matches"
    );
}

// ---------------------------------------------------------------------------
// Test 5 (SSO-03 fail-closed, auto-accept): unreachable directory does not
// elevate role — SSO login itself is NOT blocked (availability preserved).
// ---------------------------------------------------------------------------

#[tokio::test]
async fn sso_login_unreachable_directory_does_not_elevate_role_auto_accept() {
    let (svc, _dir) =
        make_auth_service_with_directory(mock_ad_client_default(), mock_directory_unreachable());
    svc.set_ad_enabled(true, &admin_caller())
        .await
        .expect("enable AD");
    svc.set_ad_auto_accept(true, &admin_caller())
        .await
        .expect("enable auto-accept");

    let dto = svc
        .sso_login("us300", "us300")
        .await
        .expect("SSO login must still succeed even when directory enrichment is unreachable");

    assert_eq!(
        dto.role, "employee",
        "unreachable directory must fail-closed: role must NOT be elevated"
    );
}

// ---------------------------------------------------------------------------
// Test 6 (SSO-03 fail-closed, pending path): unreachable directory does not
// bypass the existing auto-accept gate.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn sso_login_unreachable_directory_still_routes_to_pending_path() {
    let (svc, _dir) =
        make_auth_service_with_directory(mock_ad_client_default(), mock_directory_unreachable());
    svc.set_ad_enabled(true, &admin_caller())
        .await
        .expect("enable AD");
    // ad_auto_accept defaults to false — no explicit call needed, but keep it
    // explicit for clarity/regression-proofing against a future default flip.
    svc.set_ad_auto_accept(false, &admin_caller())
        .await
        .expect("disable auto-accept");

    let result = svc.sso_login("us300", "us300").await;

    assert!(
        matches!(result, Err(AppError::RegistrationPending { .. })),
        "unreachable directory must not bypass the existing auto-accept gate or grant a \
         session it shouldn't; expected RegistrationPending, got {result:?}"
    );
}

// ---------------------------------------------------------------------------
// Test 7 (defense in depth): the typed DirectoryError contract survives a
// direct, unit-level call — not just AuthService's internal match.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn mock_directory_unreachable_returns_typed_error_not_boolean() {
    let directory = MockAdDirectory::unreachable();
    let result = directory.resolve("anyone").await;

    assert!(
        matches!(result, Err(DirectoryError::Unreachable)),
        "unreachable directory must return the typed DirectoryError::Unreachable variant, \
         not a collapsed boolean/Ok(default), got {result:?}"
    );
}
