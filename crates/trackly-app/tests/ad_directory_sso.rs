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

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use trackly_app::services::AuthService;
use trackly_core::auth::{Identity, Role};
use trackly_core::error::AppError;
use trackly_core::ports::ad_directory::{AdDirectory, DirectoryError, DirectoryResult};
use trackly_core::primitives::clock::Clock;
use trackly_infra::ad::directory_mock::{DirectoryFixture, MockAdDirectory};
use trackly_infra::ad::mock::MockAdClient;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::db::pools::ReaderPool;
use trackly_infra::db::writer_worker::WriterHandle;
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

// ---------------------------------------------------------------------------
// 260805-wik: existing-active-user full_name sync regression tests.
//
// Closes the SSO-01 gap where `on_ad_bind_success`'s active-user branch
// discarded the directory-resolved `display_name` entirely. These tests also
// pin down the anti-corruption guards (D-1/D-2/D-3/D-5) that prevent an AD
// outage/misconfiguration from silently overwriting a stored ФИО with the
// bare login.
// ---------------------------------------------------------------------------

/// Local, test-only `AdDirectory` whose `resolve` always degrades to
/// `Err(DirectoryError::NotConfigured)` — mirrors this codebase's "small
/// independent adapters" convention (see `directory_mock.rs`/
/// `normalize_login_for_admin_check`'s doc comments). Deliberately NOT added
/// to the shared `MockAdDirectory` in `trackly-infra`.
struct NotConfiguredDirectory;

#[async_trait]
impl AdDirectory for NotConfiguredDirectory {
    async fn resolve(&self, _sam_account_name: &str) -> Result<DirectoryResult, DirectoryError> {
        Err(DirectoryError::NotConfigured)
    }
}

/// Seeds an active `us100` user (via one `sso_login` call against
/// `MockAdDirectory::default_fixtures()`, so `full_name` starts as
/// "Иванов Иван Иванович") and returns the shared writer/readers/tempdir so a
/// SECOND `AuthService` (constructed by each test below, with a different
/// `directory`) can log the SAME login in again against the SAME `users`
/// row — `test_writer_and_readers()` creates an independent, fresh DB on
/// every call, so it must only be called ONCE per test.
async fn seed_active_us100(
    directory: Arc<dyn AdDirectory + Send + Sync>,
) -> (Arc<WriterHandle>, Arc<ReaderPool>, tempfile::TempDir) {
    let (writer, readers, dir) = test_writer_and_readers();
    let clock: Arc<dyn Clock + Send + Sync> = Arc::new(SystemClock);
    let (ws_tx, _) = tokio::sync::broadcast::channel(128);
    let seed_svc = AuthService::new(
        writer.clone(),
        readers.clone(),
        clock,
        mock_ad_client_default(),
        Arc::new(ws_tx),
        directory,
    );
    seed_svc
        .set_ad_enabled(true, &admin_caller())
        .await
        .expect("enable AD");
    seed_svc
        .set_ad_auto_accept(true, &admin_caller())
        .await
        .expect("enable auto-accept");
    seed_svc
        .sso_login("us100", "us100")
        .await
        .expect("seed sso_login for us100 must succeed");
    (writer, readers, dir)
}

/// Builds a second `AuthService` sharing the given writer/readers (SAME
/// underlying DB row as `seed_active_us100`'s user) with the given
/// `directory`, for the "second login" half of each test below.
fn second_auth_service(
    writer: Arc<WriterHandle>,
    readers: Arc<ReaderPool>,
    directory: Arc<dyn AdDirectory + Send + Sync>,
) -> AuthService {
    let clock: Arc<dyn Clock + Send + Sync> = Arc::new(SystemClock);
    let (ws_tx, _) = tokio::sync::broadcast::channel(128);
    AuthService::new(
        writer,
        readers,
        clock,
        mock_ad_client_default(),
        Arc::new(ws_tx),
        directory,
    )
}

#[tokio::test]
async fn sso_login_updates_existing_active_users_stored_name_on_directory_change() {
    let (writer, readers, _dir) =
        seed_active_us100(Arc::new(MockAdDirectory::default_fixtures())).await;

    let mut changed = MockAdDirectory::default_fixtures();
    changed.users.insert(
        "us100".to_string(),
        DirectoryFixture {
            display_name: "Иванов Иван Петрович",
            role: Some(Role::Manager),
        },
    );

    let svc = second_auth_service(writer, readers, Arc::new(changed));
    let dto = svc
        .sso_login("us100", "us100")
        .await
        .expect("second sso_login must succeed");

    assert_eq!(
        dto.full_name, "Иванов Иван Петрович",
        "an existing active user's stored full_name must update to the newly \
         directory-resolved ФИО when it has genuinely changed"
    );
}

#[tokio::test]
async fn sso_login_does_not_overwrite_stored_name_when_directory_unreachable() {
    let (writer, readers, _dir) =
        seed_active_us100(Arc::new(MockAdDirectory::default_fixtures())).await;

    let svc = second_auth_service(writer, readers, mock_directory_unreachable());
    let dto = svc
        .sso_login("us100", "us100")
        .await
        .expect("SSO login must still succeed even when directory is unreachable");

    assert_eq!(
        dto.full_name, "Иванов Иван Иванович",
        "an unreachable directory must NEVER overwrite an existing active user's stored \
         full_name with the bare-login fallback value"
    );
}

#[tokio::test]
async fn sso_login_does_not_overwrite_stored_name_when_directory_not_configured() {
    let (writer, readers, _dir) =
        seed_active_us100(Arc::new(MockAdDirectory::default_fixtures())).await;

    let svc = second_auth_service(writer, readers, Arc::new(NotConfiguredDirectory));
    let dto = svc
        .sso_login("us100", "us100")
        .await
        .expect("SSO login must still succeed even when directory is not configured");

    assert_eq!(
        dto.full_name, "Иванов Иван Иванович",
        "a not-configured directory must NEVER overwrite an existing active user's stored \
         full_name with the bare-login fallback value"
    );
}

#[tokio::test]
async fn sso_login_does_not_overwrite_stored_name_when_resolved_name_equals_login() {
    let (writer, readers, _dir) =
        seed_active_us100(Arc::new(MockAdDirectory::default_fixtures())).await;

    // Empty fixture map: MockAdDirectory::resolve("us100") falls through to
    // its unmapped-login fallback, returning Ok(DirectoryResult { display_name:
    // "us100", role: None }) — a genuinely trusted (Ok) response whose value
    // happens to equal the bare login itself.
    let empty_directory = Arc::new(MockAdDirectory {
        users: HashMap::new(),
        unreachable: false,
    });
    let svc = second_auth_service(writer, readers, empty_directory);
    let dto = svc
        .sso_login("us100", "us100")
        .await
        .expect("SSO login must still succeed");

    assert_eq!(
        dto.full_name, "Иванов Иван Иванович",
        "a directory-resolved name equal to the bare login itself must never overwrite a \
         stored full_name, even on the trusted Ok branch (D-3 belt-and-braces guard)"
    );
}

/// Pins guard D-1 (provenance) specifically — the three tests above do NOT.
///
/// Today `http/sso.rs:71` calls `sso_login(ad_username, ad_username)`, so in every
/// degrade branch `resolved_display_name` happens to equal the login, and guard D-3
/// (name == login) catches the write on its own. That makes D-1 and D-3 redundant
/// *at the current call site*: deleting the `name_source != Directory` check leaves
/// all three anti-corruption tests above green, which is exactly the silent-regression
/// hole this test closes.
///
/// The scenario here is the plausible future in which the caller supplies a real-looking
/// name from a degraded source (e.g. a display name carried on the Kerberos ticket) while
/// the directory itself is unreachable. That name is NOT directory-resolved, so it must
/// not be written — and only D-1 can tell, since it neither equals the login nor is empty.
#[tokio::test]
async fn sso_login_does_not_overwrite_stored_name_with_untrusted_caller_supplied_name() {
    let (writer, readers, _dir) =
        seed_active_us100(Arc::new(MockAdDirectory::default_fixtures())).await;

    let svc = second_auth_service(writer, readers, mock_directory_unreachable());
    let dto = svc
        .sso_login("us100", "Петров Пётр Петрович")
        .await
        .expect("SSO login must still succeed even when directory is unreachable");

    assert_eq!(
        dto.full_name, "Иванов Иван Иванович",
        "a caller-supplied display_name reaching the degrade branch is NOT directory-resolved \
         and must never be written, even though it is non-empty and differs from the login — \
         this is guard D-1 (NameSource::Directory), and only this test pins it"
    );
}
