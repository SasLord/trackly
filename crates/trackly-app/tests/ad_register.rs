//! Интеграционные тесты для unknown/blocked веток `on_ad_bind_success`
//! (Phase 9 Plan 03 — USR-09/USR-11/SET-10/REQ-06).
//!
//! Покрывает:
//! - auto-accept ON, неизвестный AD-пользователь → активный user + заявка
//!   `ad_register`/`ad_subtype='register'`, сессия выдаётся.
//! - auto-accept OFF (pending), неизвестный AD-пользователь → неактивный
//!   user + заявка, сессия НЕ выдаётся (`AppError::RegistrationPending`).
//! - blocked/soft-deleted AD-пользователь → заявка
//!   `ad_subtype='restore'`, сессия НЕ выдаётся (`AppError::AccessBlocked`).
//! - все три ветки — единая writer-транзакция (T-09-13).

use std::sync::Arc;

use rusqlite::params;

use trackly_app::dto::auth::LoginRequest;
use trackly_app::services::AuthService;
use trackly_core::auth::Identity;
use trackly_core::error::AppError;
use trackly_core::primitives::clock::Clock;
use trackly_infra::ad::mock::MockAdClient;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::test_support::test_writer_and_readers;

fn make_auth_service_with_ad() -> (AuthService, tempfile::TempDir) {
    let (writer, readers, dir) = test_writer_and_readers();
    let clock: Arc<dyn Clock + Send + Sync> = Arc::new(SystemClock);
    let ad_client: Arc<dyn trackly_core::ports::ad::AdClient + Send + Sync> =
        Arc::new(MockAdClient::default_fixtures());
    let (ws_tx, _ws_rx) = tokio::sync::broadcast::channel(128);
    let svc = AuthService::new(writer, readers, clock, ad_client, Arc::new(ws_tx));
    (svc, dir)
}

fn admin_caller() -> Identity {
    Identity::trusted_admin()
}

/// Seed a blocked (`is_active=0`) or soft-deleted (`deleted_at_utc` set) local
/// AD-linked user row directly, bypassing `create_user` (which requires a
/// password) — mirrors `ad_auth.rs`'s `seed_ad_user` helper.
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

// ---------------------------------------------------------------------------
// Test 1: auto-accept ON — creates active user + info ad_register request,
// login returns a usable session.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn auto_accept_creates_user_and_info_request() {
    let (svc, _dir) = make_auth_service_with_ad();
    svc.set_ad_enabled(true, &admin_caller())
        .await
        .expect("enable AD");
    svc.set_ad_auto_accept(true, &admin_caller())
        .await
        .expect("enable auto-accept");

    let dto = svc
        .login(LoginRequest {
            login: "us100".to_string(),
            password: "Passw0rd!".to_string(),
            remember: false,
        })
        .await
        .expect("auto-accept login должен выдать сессию");

    assert_eq!(dto.login, "us100");
    assert_eq!(dto.full_name, "Иванов Иван Иванович");
    assert_eq!(dto.role, "employee");

    // users row: ad_user=1, password_hash=NULL, is_active=1.
    let readers = svc.readers.clone();
    let uid = dto.id;
    let (ad_user, password_hash, is_active): (i64, Option<String>, i64) =
        tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            conn.query_row(
                "SELECT ad_user, password_hash, is_active FROM users WHERE id = ?1",
                params![uid],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .expect("query users row")
        })
        .await
        .expect("spawn_blocking");
    assert_eq!(ad_user, 1, "ad_user должен быть 1");
    assert_eq!(
        password_hash, None,
        "password_hash должен быть NULL для AD-пользователя"
    );
    assert_eq!(is_active, 1, "is_active должен быть 1 (auto-accept)");

    // ad_register request exists, ad_subtype='register', requested_by_user_id=uid.
    let readers2 = svc.readers.clone();
    let (request_type, ad_subtype, requested_by): (String, Option<String>, i64) =
        tokio::task::spawn_blocking(move || {
            let conn = readers2.acquire();
            conn.query_row(
                "SELECT request_type, ad_subtype, requested_by_user_id FROM requests \
                 WHERE requested_by_user_id = ?1 ORDER BY id DESC LIMIT 1",
                params![uid],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .expect("query requests row")
        })
        .await
        .expect("spawn_blocking");
    assert_eq!(request_type, "ad_register");
    assert_eq!(ad_subtype, Some("register".to_string()));
    assert_eq!(requested_by, uid);
}

// ---------------------------------------------------------------------------
// Test 2: auto-accept OFF (pending) — creates inactive user + ad_register
// request, login returns RegistrationPending (NOT a session).
// ---------------------------------------------------------------------------

#[tokio::test]
async fn pending_creates_inactive_user_and_request() {
    let (svc, _dir) = make_auth_service_with_ad();
    svc.set_ad_enabled(true, &admin_caller())
        .await
        .expect("enable AD");
    // ad_auto_accept defaults to false — no explicit set needed.

    let result = svc
        .login(LoginRequest {
            login: "us200".to_string(),
            password: "Secret123".to_string(),
            remember: false,
        })
        .await;

    let request_id = match result {
        Err(AppError::RegistrationPending { request_id }) => request_id,
        other => panic!("expected RegistrationPending, got {other:?}"),
    };
    assert!(request_id > 0);

    // users row exists but is_active=0.
    let readers = svc.readers.clone();
    let (is_active, ad_user, full_name): (i64, i64, String) =
        tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            conn.query_row(
                "SELECT is_active, ad_user, full_name FROM users WHERE login = 'us200'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .expect("query users row")
        })
        .await
        .expect("spawn_blocking");
    assert_eq!(is_active, 0, "pending user должен быть is_active=0");
    assert_eq!(ad_user, 1);
    assert_eq!(full_name, "Петрова Анна Сергеевна");

    // request references the (inactive) user row + ad_subtype='register'.
    let readers2 = svc.readers.clone();
    let (request_type, ad_subtype, status): (String, Option<String>, String) =
        tokio::task::spawn_blocking(move || {
            let conn = readers2.acquire();
            conn.query_row(
                "SELECT request_type, ad_subtype, status FROM requests WHERE id = ?1",
                params![request_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .expect("query request row")
        })
        .await
        .expect("spawn_blocking");
    assert_eq!(request_type, "ad_register");
    assert_eq!(ad_subtype, Some("register".to_string()));
    assert_eq!(status, "open");

    // Second login attempt while still pending must again return
    // RegistrationPending (not AccessBlocked — that subtype is for
    // blocked/soft-deleted users, not a never-yet-approved registration),
    // reusing the SAME open request rather than creating a second one.
    let result2 = svc
        .login(LoginRequest {
            login: "us200".to_string(),
            password: "Secret123".to_string(),
            remember: false,
        })
        .await;
    let request_id2 = match result2 {
        Err(AppError::RegistrationPending { request_id }) => request_id,
        other => panic!(
            "повторный bind для pending-пользователя должен вернуть \
             RegistrationPending (не {other:?}) — пользователь ещё не \
             одобрен, это не restore-сценарий"
        ),
    };
    assert_eq!(
        request_id2, request_id,
        "повторный bind должен вернуть ID той же открытой заявки на регистрацию"
    );

    // Exactly ONE open ad_register request must exist for this user — no
    // duplicate "restore" request alongside the original "register" one.
    let readers3 = svc.readers.clone();
    let open_count: i64 = tokio::task::spawn_blocking(move || {
        let conn = readers3.acquire();
        conn.query_row(
            "SELECT COUNT(*) FROM requests \
             WHERE request_type = 'ad_register' AND status = 'open' \
               AND requested_by_user_id = (SELECT id FROM users WHERE login = 'us200')",
            [],
            |r| r.get(0),
        )
        .expect("count open ad_register requests")
    })
    .await
    .expect("spawn_blocking");
    assert_eq!(
        open_count, 1,
        "должна существовать ровно ОДНА открытая заявка после двух bind-попыток pending-пользователя"
    );
}

// ---------------------------------------------------------------------------
// Test 3: blocked (is_active=0, not soft-deleted) AD user binds OK →
// restore request created, AccessBlocked returned (NOT a session).
// ---------------------------------------------------------------------------

#[tokio::test]
async fn blocked_user_creates_restore_request() {
    let (svc, _dir) = make_auth_service_with_ad();
    svc.set_ad_enabled(true, &admin_caller())
        .await
        .expect("enable AD");

    let existing_user_id = seed_blocked_ad_user(&svc, "us100", "Иванов Иван Иванович", false).await;

    let result = svc
        .login(LoginRequest {
            login: "us100".to_string(),
            password: "Passw0rd!".to_string(),
            remember: false,
        })
        .await;

    let request_id = match result {
        Err(AppError::AccessBlocked { request_id }) => request_id,
        other => panic!("expected AccessBlocked, got {other:?}"),
    };

    let readers = svc.readers.clone();
    let (request_type, ad_subtype, requested_by): (String, Option<String>, i64) =
        tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            conn.query_row(
                "SELECT request_type, ad_subtype, requested_by_user_id FROM requests \
                 WHERE id = ?1",
                params![request_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .expect("query request row")
        })
        .await
        .expect("spawn_blocking");
    assert_eq!(request_type, "ad_register");
    assert_eq!(ad_subtype, Some("restore".to_string()));
    assert_eq!(
        requested_by, existing_user_id,
        "restore request должен ссылаться на существующего пользователя"
    );
}

// ---------------------------------------------------------------------------
// Test (Defect 1 repro): two consecutive blocked-user AD binds must reuse
// the same OPEN restore request instead of inserting a duplicate.
//
// Repro context: a blocked user hits this path twice in a real session —
// once via the login form, once via BlockedScreen's "Запросить
// восстановление" button (which re-submits auth_login). Both binds must
// converge on a SINGLE open `ad_register`/`ad_subtype='restore'` row.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn blocked_user_repeated_bind_reuses_open_restore_request() {
    let (svc, _dir) = make_auth_service_with_ad();
    svc.set_ad_enabled(true, &admin_caller())
        .await
        .expect("enable AD");

    let existing_user_id = seed_blocked_ad_user(&svc, "us100", "Иванов Иван Иванович", false).await;

    // First bind — creates the restore request.
    let result1 = svc
        .login(LoginRequest {
            login: "us100".to_string(),
            password: "Passw0rd!".to_string(),
            remember: false,
        })
        .await;
    let request_id1 = match result1 {
        Err(AppError::AccessBlocked { request_id }) => request_id,
        other => panic!("expected AccessBlocked on first bind, got {other:?}"),
    };

    // Second bind (e.g. user clicks "Запросить восстановление" again, or
    // retries login) — MUST reuse the same open request, not insert a new one.
    let result2 = svc
        .login(LoginRequest {
            login: "us100".to_string(),
            password: "Passw0rd!".to_string(),
            remember: false,
        })
        .await;
    let request_id2 = match result2 {
        Err(AppError::AccessBlocked { request_id }) => request_id,
        other => panic!("expected AccessBlocked on second bind, got {other:?}"),
    };

    assert_eq!(
        request_id1, request_id2,
        "повторный bind должен возвращать ID того же открытого restore-запроса, а не создавать новый"
    );

    // Exactly ONE open ad_register/restore request must exist for this user.
    let readers = svc.readers.clone();
    let open_restore_count: i64 = tokio::task::spawn_blocking(move || {
        let conn = readers.acquire();
        conn.query_row(
            "SELECT COUNT(*) FROM requests \
             WHERE request_type = 'ad_register' AND ad_subtype = 'restore' \
               AND requested_by_user_id = ?1 AND status = 'open' \
               AND deleted_at_utc IS NULL",
            params![existing_user_id],
            |r| r.get(0),
        )
        .expect("count open restore requests")
    })
    .await
    .expect("spawn_blocking");
    assert_eq!(
        open_restore_count, 1,
        "должна существовать ровно ОДНА открытая restore-заявка после двух bind-попыток"
    );
}

/// Same scenario, but soft-deleted instead of merely blocked — must take the
/// same restore-request path (D-REG-03 treats both as "blocked").
#[tokio::test]
async fn soft_deleted_user_creates_restore_request() {
    let (svc, _dir) = make_auth_service_with_ad();
    svc.set_ad_enabled(true, &admin_caller())
        .await
        .expect("enable AD");

    let existing_user_id =
        seed_blocked_ad_user(&svc, "us200", "Петрова Анна Сергеевна", true).await;

    let result = svc
        .login(LoginRequest {
            login: "us200".to_string(),
            password: "Secret123".to_string(),
            remember: false,
        })
        .await;

    let request_id = match result {
        Err(AppError::AccessBlocked { request_id }) => request_id,
        other => panic!("expected AccessBlocked, got {other:?}"),
    };

    let readers = svc.readers.clone();
    let (ad_subtype, requested_by): (Option<String>, i64) =
        tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            conn.query_row(
                "SELECT ad_subtype, requested_by_user_id FROM requests WHERE id = ?1",
                params![request_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .expect("query request row")
        })
        .await
        .expect("spawn_blocking");
    assert_eq!(ad_subtype, Some("restore".to_string()));
    assert_eq!(requested_by, existing_user_id);
}

// ---------------------------------------------------------------------------
// Test 4: atomicity — all writes for each branch go through the single
// writer (T-09-13). Verified structurally: every write in this module's
// production code path is a `WriterHandle::execute` closure (grep-gated in
// CI by the plan's acceptance criteria); here we additionally assert that a
// successful auto-accept run leaves exactly ONE user row and ONE request
// row behind (no partial/duplicate writes from a non-atomic multi-statement
// sequence).
// ---------------------------------------------------------------------------

#[tokio::test]
async fn all_writes_single_writer() {
    let (svc, _dir) = make_auth_service_with_ad();
    svc.set_ad_enabled(true, &admin_caller())
        .await
        .expect("enable AD");
    svc.set_ad_auto_accept(true, &admin_caller())
        .await
        .expect("enable auto-accept");

    svc.login(LoginRequest {
        login: "us100".to_string(),
        password: "Passw0rd!".to_string(),
        remember: false,
    })
    .await
    .expect("auto-accept login should succeed");

    let readers = svc.readers.clone();
    let (user_count, request_count, audit_count): (i64, i64, i64) =
        tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            let users: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM users WHERE login = 'us100'",
                    [],
                    |r| r.get(0),
                )
                .expect("count users");
            let requests: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM requests WHERE request_type = 'ad_register'",
                    [],
                    |r| r.get(0),
                )
                .expect("count requests");
            let audit: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM audit_log WHERE action IN ('ad_auto_register', 'create')",
                    [],
                    |r| r.get(0),
                )
                .expect("count audit_log");
            (users, requests, audit)
        })
        .await
        .expect("spawn_blocking");

    assert_eq!(
        user_count, 1,
        "ровно один users row, без дублей от неатомарной записи"
    );
    assert_eq!(request_count, 1, "ровно один requests row");
    assert_eq!(
        audit_count, 2,
        "по одной audit_log записи на user-create и request-create (T-09-14)"
    );
}
