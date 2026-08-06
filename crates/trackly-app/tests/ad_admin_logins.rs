//! Интеграционные тесты SSO-02 (Phase 32 Plan 03) — full state-matrix
//! coverage for `force_admin_provisioning` (Plan 02's forced-admin state
//! machine, wired into `on_ad_bind_success`).
//!
//! Покрывает:
//! - unknown login in `admin_logins` → active admin, NO `ad_register` request
//! - pending login in `admin_logins` → activated admin + dangling open
//!   request auto-completed + `audit_log` row (Pitfall 2, ASVS V9)
//! - blocked/soft-deleted login in `admin_logins` → revived as active admin
//!   (overrides manual block, D-07)
//! - active non-admin login in `admin_logins` → escalated to admin (D-06)
//! - active admin login in `admin_logins` → idempotent no-op (version stays)
//! - login NOT in `admin_logins` → Phase 31 behavior fully unchanged (D-08)
//! - `admin_logins` forces admin even when `AdDirectory::resolve` is
//!   `Unreachable` (D-10 — pure local set check, independent of directory)
//! - both `sso_login` (passwordless SSO) AND `try_ad_login` (LDAPS bind)
//!   entry points get identical forced-admin treatment
//!
//! Используются ТОЛЬКО уже существующие placeholder-идентичности из
//! `directory_mock.rs`/`mock.rs` (us100/us200) — никаких реальных имён/доменов.

use std::sync::Arc;

use rusqlite::params;

use trackly_app::dto::auth::LoginRequest;
use trackly_app::services::AuthService;
use trackly_core::auth::{Identity, Role};
use trackly_core::error::AppError;
use trackly_core::ports::ad_directory::AdDirectory;
use trackly_core::primitives::clock::Clock;
use trackly_infra::ad::directory_mock::{DirectoryFixture, MockAdDirectory};
use trackly_infra::ad::mock::MockAdClient;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::test_support::test_writer_and_readers;

/// Independent helper (per this codebase's established "small independent
/// fixtures" convention) — mirrors `ad_directory_sso.rs`'s
/// `make_auth_service_with_directory`, plus `.with_admin_logins(...)`.
fn make_auth_service_with_admin_logins(
    admin_logins: Vec<String>,
    directory: Arc<dyn AdDirectory + Send + Sync>,
) -> (AuthService, tempfile::TempDir) {
    let (writer, readers, dir) = test_writer_and_readers();
    let clock: Arc<dyn Clock + Send + Sync> = Arc::new(SystemClock);
    let ad_client: Arc<dyn trackly_core::ports::ad::AdClient + Send + Sync> =
        Arc::new(MockAdClient::default_fixtures());
    let (ws_tx, _) = tokio::sync::broadcast::channel(128);
    let svc = AuthService::new(
        writer,
        readers,
        clock,
        ad_client,
        Arc::new(ws_tx),
        directory,
    )
    .with_admin_logins(admin_logins);
    (svc, dir)
}

/// Build a SECOND `AuthService` sharing the SAME writer/readers as `svc`,
/// with its own `admin_logins`/`directory` — needed for the pending-user
/// test, which must first create the pending state via one `AuthService`
/// (default, empty `admin_logins`) and then re-login via a second
/// `AuthService` instance configured with `admin_logins=[...]` against the
/// SAME DB. Mirrors `ad_register.rs`'s `make_request_service_sharing`.
fn make_auth_service_sharing(
    svc: &AuthService,
    admin_logins: Vec<String>,
    directory: Arc<dyn AdDirectory + Send + Sync>,
) -> AuthService {
    let clock: Arc<dyn Clock + Send + Sync> = Arc::new(SystemClock);
    let ad_client: Arc<dyn trackly_core::ports::ad::AdClient + Send + Sync> =
        Arc::new(MockAdClient::default_fixtures());
    let (ws_tx, _) = tokio::sync::broadcast::channel(128);
    AuthService::new(
        svc.writer.clone(),
        svc.readers.clone(),
        clock,
        ad_client,
        Arc::new(ws_tx),
        directory,
    )
    .with_admin_logins(admin_logins)
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

/// Seed a blocked (`is_active=0`) or soft-deleted (`deleted_at_utc` set)
/// local AD-linked user row directly, bypassing `create_user` (which
/// requires a password) — local independent copy, per this codebase's
/// established convention (mirrors `ad_register.rs`'s `seed_blocked_ad_user`).
async fn seed_blocked_ad_user(
    svc: &AuthService,
    login: &str,
    full_name: &str,
    deleted: bool,
) -> i64 {
    let now = SystemClock.unix_seconds();
    let login = login.to_string();
    let full_name = full_name.to_string();
    svc.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO users \
                 (login, full_name, password_hash, role, ad_user, is_active, \
                  deleted_at_utc, created_at_utc, updated_at_utc, version) \
                 VALUES (?1, ?2, NULL, 'employee', 1, 0, ?3, ?4, ?4, 1)",
                params![
                    login,
                    full_name,
                    if deleted { Some(now) } else { None },
                    now
                ],
            )
            .map_err(|e| AppError::Internal {
                source_chain: format!("seed_blocked_ad_user: {e}"),
            })?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed blocked/deleted AD user")
}

/// Seed an ACTIVE local AD-linked user row directly with an explicit role —
/// local independent copy (mirrors `ad_auth.rs`'s `seed_ad_user`).
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

// ---------------------------------------------------------------------------
// Task 1: unknown / pending / blocked / soft-deleted / active-escalation
// ---------------------------------------------------------------------------

#[tokio::test]
async fn admin_logins_unknown_user_becomes_active_admin_no_pending_request() {
    let (svc, _dir) =
        make_auth_service_with_admin_logins(vec!["us100".to_string()], mock_directory_default());
    svc.set_ad_enabled(true, &admin_caller())
        .await
        .expect("enable AD");

    let dto = svc
        .sso_login("us100", "us100")
        .await
        .expect("admin_logins bypass must succeed without going through RegistrationPending");

    assert_eq!(dto.role, "admin");
    assert!(dto.is_active);

    let readers = svc.readers.clone();
    let uid = dto.id;
    let request_count: i64 = tokio::task::spawn_blocking(move || {
        let conn = readers.acquire();
        conn.query_row(
            "SELECT COUNT(*) FROM requests \
             WHERE request_type = 'ad_register' AND requested_by_user_id = ?1",
            params![uid],
            |r| r.get(0),
        )
        .expect("count requests")
    })
    .await
    .expect("spawn_blocking");
    assert_eq!(
        request_count, 0,
        "unknown-login bypass must create NO ad_register/requests row"
    );
}

#[tokio::test]
async fn admin_logins_pending_user_activated_and_request_completed() {
    // First AuthService: default (empty admin_logins), auto-accept OFF by
    // default — creates the pending state via the REAL `login()` flow.
    let (svc, _dir) = make_auth_service_with_admin_logins(vec![], mock_directory_default());
    svc.set_ad_enabled(true, &admin_caller())
        .await
        .expect("enable AD");

    let request_id = match svc
        .login(LoginRequest {
            login: "us100".to_string(),
            password: "Passw0rd!".to_string(),
            remember: false,
        })
        .await
    {
        Err(AppError::RegistrationPending { request_id }) => request_id,
        other => panic!("expected RegistrationPending, got {other:?}"),
    };

    // Second AuthService sharing the SAME writer/readers, admin_logins=["us100"].
    let svc2 = make_auth_service_sharing(&svc, vec!["us100".to_string()], mock_directory_default());
    svc2.set_ad_enabled(true, &admin_caller())
        .await
        .expect("enable AD on shared svc2");

    let dto = svc2
        .sso_login("us100", "us100")
        .await
        .expect("admin_logins must activate the pending user as admin");

    assert_eq!(dto.role, "admin");
    assert!(dto.is_active);

    let readers = svc.readers.clone();
    let status: String = tokio::task::spawn_blocking(move || {
        let conn = readers.acquire();
        conn.query_row(
            "SELECT status FROM requests WHERE id = ?1",
            params![request_id],
            |r| r.get(0),
        )
        .expect("query request status")
    })
    .await
    .expect("spawn_blocking");
    assert_eq!(
        status, "completed",
        "the dangling open ad_register request must be auto-completed in the same transaction"
    );

    let readers2 = svc.readers.clone();
    let uid = dto.id;
    let audit_count: i64 = tokio::task::spawn_blocking(move || {
        let conn = readers2.acquire();
        conn.query_row(
            "SELECT COUNT(*) FROM audit_log WHERE entity_id = ?1 AND action = 'ad_auto_admin'",
            params![uid],
            |r| r.get(0),
        )
        .expect("count audit_log")
    })
    .await
    .expect("spawn_blocking");
    assert_eq!(
        audit_count, 1,
        "an audit_log row with action='ad_auto_admin' must exist for the promoted user"
    );
}

#[tokio::test]
async fn admin_logins_blocked_user_revived_as_admin() {
    let (svc, _dir) =
        make_auth_service_with_admin_logins(vec!["us100".to_string()], mock_directory_default());
    svc.set_ad_enabled(true, &admin_caller())
        .await
        .expect("enable AD");
    seed_blocked_ad_user(&svc, "us100", "Иванов Иван Иванович", false).await;

    let dto = svc
        .sso_login("us100", "us100")
        .await
        .expect("admin_logins must revive a blocked user as active admin");

    assert_eq!(dto.role, "admin");
    assert!(dto.is_active);
}

#[tokio::test]
async fn admin_logins_soft_deleted_user_revived_as_admin() {
    let (svc, _dir) =
        make_auth_service_with_admin_logins(vec!["us100".to_string()], mock_directory_default());
    svc.set_ad_enabled(true, &admin_caller())
        .await
        .expect("enable AD");
    let user_id = seed_blocked_ad_user(&svc, "us100", "Иванов Иван Иванович", true).await;

    let dto = svc
        .sso_login("us100", "us100")
        .await
        .expect("admin_logins must revive a soft-deleted user as active admin");

    assert_eq!(dto.role, "admin");
    assert!(dto.is_active);

    let readers = svc.readers.clone();
    let deleted_at: Option<i64> = tokio::task::spawn_blocking(move || {
        let conn = readers.acquire();
        conn.query_row(
            "SELECT deleted_at_utc FROM users WHERE id = ?1",
            params![user_id],
            |r| r.get(0),
        )
        .expect("query deleted_at_utc")
    })
    .await
    .expect("spawn_blocking");
    assert_eq!(
        deleted_at, None,
        "deleted_at_utc must be NULL after forced-admin revival"
    );
}

#[tokio::test]
async fn admin_logins_active_non_admin_escalated_to_admin() {
    let (svc, _dir) =
        make_auth_service_with_admin_logins(vec!["us100".to_string()], mock_directory_default());
    svc.set_ad_enabled(true, &admin_caller())
        .await
        .expect("enable AD");
    seed_ad_user(&svc, "us100", "Иванов Иван Иванович", "employee").await;

    let dto = svc
        .sso_login("us100", "us100")
        .await
        .expect("admin_logins must escalate an existing active non-admin to admin");

    assert_eq!(dto.role, "admin");
}

// ---------------------------------------------------------------------------
// Task 2: idempotency, not-in-list regression, directory-unreachable,
// dual-entry-point (SSO + LDAPS) coverage
// ---------------------------------------------------------------------------

#[tokio::test]
async fn admin_logins_already_admin_is_idempotent_noop() {
    let (svc, _dir) =
        make_auth_service_with_admin_logins(vec!["us100".to_string()], mock_directory_default());
    svc.set_ad_enabled(true, &admin_caller())
        .await
        .expect("enable AD");
    seed_ad_user(&svc, "us100", "Иванов Иван Иванович", "admin").await;

    let readers = svc.readers.clone();
    let version_before: i64 = tokio::task::spawn_blocking(move || {
        let conn = readers.acquire();
        conn.query_row("SELECT version FROM users WHERE login = 'us100'", [], |r| {
            r.get(0)
        })
        .expect("query version before")
    })
    .await
    .expect("spawn_blocking");
    assert_eq!(version_before, 1);

    svc.sso_login("us100", "us100")
        .await
        .expect("first already-admin login");
    svc.sso_login("us100", "us100")
        .await
        .expect("second already-admin login");

    let readers2 = svc.readers.clone();
    let version_after: i64 = tokio::task::spawn_blocking(move || {
        let conn = readers2.acquire();
        conn.query_row("SELECT version FROM users WHERE login = 'us100'", [], |r| {
            r.get(0)
        })
        .expect("query version after")
    })
    .await
    .expect("spawn_blocking");
    assert_eq!(
        version_after, 1,
        "version must NOT bump on repeat logins for an already-admin user (idempotency)"
    );
}

#[tokio::test]
async fn admin_logins_not_in_list_phase31_behavior_unchanged() {
    let (svc, _dir) =
        make_auth_service_with_admin_logins(vec!["us100".to_string()], mock_directory_default());
    svc.set_ad_enabled(true, &admin_caller())
        .await
        .expect("enable AD");
    // ad_auto_accept defaults to false — us200 (NOT in admin_logins) must
    // follow the unchanged Phase 31 pending path, exactly matching
    // ad_register.rs::pending_creates_inactive_user_and_request's expectation
    // for an unknown login under default settings.

    let result = svc.sso_login("us200", "us200").await;
    assert!(
        matches!(result, Err(AppError::RegistrationPending { .. })),
        "logins outside admin_logins must be fully unaffected by Phase 32 (D-08); expected \
         RegistrationPending (default auto-accept-off Phase 31 behavior), got {result:?}"
    );
}

#[tokio::test]
async fn admin_logins_forces_admin_when_directory_unreachable() {
    let (svc, _dir) = make_auth_service_with_admin_logins(
        vec!["us100".to_string()],
        mock_directory_unreachable(),
    );
    svc.set_ad_enabled(true, &admin_caller())
        .await
        .expect("enable AD");

    let dto = svc
        .sso_login("us100", "us100")
        .await
        .expect("admin_logins must force admin even when AdDirectory::resolve is Unreachable");

    assert_eq!(
        dto.role, "admin",
        "admin_logins membership check must not depend on directory reachability (D-10)"
    );
    assert!(dto.is_active);
}

#[tokio::test]
async fn admin_logins_forces_admin_on_ldaps_password_bind_path_too() {
    let (svc, _dir) =
        make_auth_service_with_admin_logins(vec!["us100".to_string()], mock_directory_default());
    svc.set_ad_enabled(true, &admin_caller())
        .await
        .expect("enable AD");

    let dto = svc
        .login(LoginRequest {
            login: "us100".to_string(),
            password: "Passw0rd!".to_string(),
            remember: false,
        })
        .await
        .expect("admin_logins must also force admin on the LDAPS try_ad_login path");

    assert_eq!(
        dto.role, "admin",
        "the shared on_ad_bind_success injection point must cover BOTH sso_login and \
         try_ad_login entry points, not just SSO"
    );
}

// ---------------------------------------------------------------------------
// 260806-wk1 (WK1-01/WK1-02): force_admin_provisioning now resyncs full_name
// via the existing sync_active_user_name helper on every non-INSERT branch.
// ---------------------------------------------------------------------------

/// Pins the "already active admin" branch's new write path (T-260806wk1-01):
/// a directory-resolved ФИО change for an admin_logins login must update the
/// stored full_name on next login, exactly like a regular active user.
#[tokio::test]
async fn admin_logins_already_admin_syncs_changed_name_from_directory() {
    let mut directory = MockAdDirectory::default_fixtures();
    directory.users.insert(
        "us100".to_string(),
        DirectoryFixture {
            display_name: "Иванов Иван Петрович",
            role: Some(Role::Manager),
        },
    );

    let (svc, _dir) =
        make_auth_service_with_admin_logins(vec!["us100".to_string()], Arc::new(directory));
    svc.set_ad_enabled(true, &admin_caller())
        .await
        .expect("enable AD");
    seed_ad_user(&svc, "us100", "Иванов Иван Иванович", "admin").await;

    let dto = svc
        .sso_login("us100", "us100")
        .await
        .expect("already-admin login must still succeed");

    assert_eq!(
        dto.full_name, "Иванов Иван Петрович",
        "an already-active-admin login must resync full_name from the directory, exactly \
         like a regular active user (closes the SSO-01 gap for forced admins)"
    );
    assert_eq!(dto.role, "admin");
}

/// Pins guard D-1 (NameSource, not the weaker name-equals-login guard) on
/// the "already active admin" branch (T-260806wk1-02). The caller-supplied
/// name here is non-empty and NOT equal to the login — a same-as-login
/// variant would still pass even with the D-1 guard deleted (that was
/// yesterday's useless-test trap, 260805-wik), so this shape is what makes
/// the test meaningful.
#[tokio::test]
async fn admin_logins_already_admin_does_not_overwrite_name_with_untrusted_caller_supplied_name() {
    let (svc, _dir) = make_auth_service_with_admin_logins(
        vec!["us100".to_string()],
        mock_directory_unreachable(),
    );
    svc.set_ad_enabled(true, &admin_caller())
        .await
        .expect("enable AD");
    seed_ad_user(&svc, "us100", "Иванов Иван Иванович", "admin").await;

    let dto = svc
        .sso_login("us100", "Петров Пётр Петрович")
        .await
        .expect("already-admin login must still succeed even when directory is unreachable");

    assert_eq!(
        dto.full_name, "Иванов Иван Иванович",
        "an unreachable directory must NEVER overwrite a forced-admin user's stored full_name \
         with an untrusted caller-supplied name, even though it is non-empty and differs from \
         the login — this is guard D-1 (NameSource::Directory), and only this non-login-shaped \
         caller-supplied name pins it"
    );
    assert_eq!(dto.role, "admin");
}

/// Pins that the sync call is not lost specifically inside
/// `force_admin_escalate_active`'s branch (T-260806wk1-01, item 3): the
/// escalation branch must both promote the user to admin AND sync their
/// ФИО in the same login.
#[tokio::test]
async fn admin_logins_active_non_admin_escalation_also_syncs_changed_name() {
    let mut directory = MockAdDirectory::default_fixtures();
    directory.users.insert(
        "us100".to_string(),
        DirectoryFixture {
            display_name: "Иванов Иван Петрович",
            role: Some(Role::Manager),
        },
    );

    let (svc, _dir) =
        make_auth_service_with_admin_logins(vec!["us100".to_string()], Arc::new(directory));
    svc.set_ad_enabled(true, &admin_caller())
        .await
        .expect("enable AD");
    seed_ad_user(&svc, "us100", "Иванов Иван Иванович", "employee").await;

    let dto = svc
        .sso_login("us100", "us100")
        .await
        .expect("escalation login must succeed");

    assert_eq!(
        dto.role, "admin",
        "the escalation branch must still promote the user to admin"
    );
    assert_eq!(
        dto.full_name, "Иванов Иван Петрович",
        "the escalation branch must ALSO sync the changed ФИО in the same login — proves the \
         sync_active_user_name call is not lost specifically inside \
         force_admin_escalate_active's branch"
    );
}
