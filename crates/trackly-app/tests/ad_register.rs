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
use trackly_app::dto::request::RequestTransitionPayload;
use trackly_app::services::{AuthService, RequestService};
use trackly_core::auth::{Identity, Role};
use trackly_core::error::AppError;
use trackly_core::primitives::clock::Clock;
use trackly_infra::ad::directory_mock::MockAdDirectory;
use trackly_infra::ad::mock::MockAdClient;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::test_support::test_writer_and_readers;

fn make_auth_service_with_ad() -> (AuthService, tempfile::TempDir) {
    let (writer, readers, dir) = test_writer_and_readers();
    let clock: Arc<dyn Clock + Send + Sync> = Arc::new(SystemClock);
    let ad_client: Arc<dyn trackly_core::ports::ad::AdClient + Send + Sync> =
        Arc::new(MockAdClient::default_fixtures());
    let (ws_tx, _ws_rx) = tokio::sync::broadcast::channel(128);
    let directory = Arc::new(MockAdDirectory::default_fixtures());
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

fn admin_caller() -> Identity {
    Identity::trusted_admin()
}

/// Build a `RequestService` sharing the SAME writer/readers as `svc` — lets
/// a test reject a restore request created via `AuthService` and then
/// re-check `AuthService::login`'s read of that rejection (09-AD-GAPS
/// restoration-flow UX full-lifecycle test).
fn make_request_service_sharing(svc: &AuthService) -> RequestService {
    let clock: Arc<dyn Clock + Send + Sync> = Arc::new(SystemClock);
    let (ws_tx, _rx) = tokio::sync::broadcast::channel(16);
    RequestService::new(
        svc.writer.clone(),
        svc.readers.clone(),
        clock,
        Arc::new(ws_tx),
    )
}

fn admin_request_caller() -> Identity {
    Identity {
        user_id: None,
        role: Role::Admin,
    }
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
// 09-AD-GAPS restoration-flow UX — plain blocked login is now READ-ONLY: it
// must NOT create (or touch) any restore request. It only REPORTS the state
// of the user's most recent restore request via
// `AppError::AccessBlocked { pending, rejection_reason }`. Three states:
//
// 1. No restore request exists yet → pending=false, rejection_reason=None.
// 2. An OPEN restore request exists → pending=true, rejection_reason=None.
// 3. Most recent restore request is REJECTED → pending=false,
//    rejection_reason=Some(reason).
//
// The explicit, idempotent create-or-reuse action lives in
// `AuthService::request_ad_restore` (separate tests below).
// ---------------------------------------------------------------------------

/// State 1: blocked user with NO restore request yet — login is read-only
/// and reports `pending=false, rejection_reason=None`. No request row is
/// created as a side effect of this login attempt.
#[tokio::test]
async fn blocked_login_is_read_only_no_request_yet() {
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

    match result {
        Err(AppError::AccessBlocked {
            pending,
            rejection_reason,
        }) => {
            assert!(!pending, "no restore request exists yet → pending=false");
            assert_eq!(rejection_reason, None);
        }
        other => panic!("expected AccessBlocked, got {other:?}"),
    }

    // Plain login must be a pure read — no requests row created.
    let readers = svc.readers.clone();
    let count: i64 = tokio::task::spawn_blocking(move || {
        let conn = readers.acquire();
        conn.query_row(
            "SELECT COUNT(*) FROM requests WHERE requested_by_user_id = ?1",
            params![existing_user_id],
            |r| r.get(0),
        )
        .expect("count requests")
    })
    .await
    .expect("spawn_blocking");
    assert_eq!(
        count, 0,
        "plain blocked login must NOT create a restore request (read-only contract)"
    );
}

/// State 2: blocked user with an OPEN restore request already on file (e.g.
/// created via an earlier explicit `request_ad_restore` call) — repeated
/// plain logins must keep reporting `pending=true` and must NOT create a
/// second request.
#[tokio::test]
async fn blocked_login_reports_pending_without_duplicating() {
    let (svc, _dir) = make_auth_service_with_ad();
    svc.set_ad_enabled(true, &admin_caller())
        .await
        .expect("enable AD");

    let existing_user_id = seed_blocked_ad_user(&svc, "us100", "Иванов Иван Иванович", false).await;

    // Explicit request first (creates the open restore request).
    svc.request_ad_restore("us100", "Passw0rd!")
        .await
        .expect("explicit restore request should succeed");

    // Now plain login twice — must report pending=true both times, with no
    // duplicate request rows.
    for _ in 0..2 {
        let result = svc
            .login(LoginRequest {
                login: "us100".to_string(),
                password: "Passw0rd!".to_string(),
                remember: false,
            })
            .await;
        match result {
            Err(AppError::AccessBlocked {
                pending,
                rejection_reason,
            }) => {
                assert!(pending, "open restore request exists → pending=true");
                assert_eq!(rejection_reason, None);
            }
            other => panic!("expected AccessBlocked, got {other:?}"),
        }
    }

    let readers = svc.readers.clone();
    let open_count: i64 = tokio::task::spawn_blocking(move || {
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
        open_count, 1,
        "exactly ONE open restore request must exist after repeated read-only logins"
    );
}

/// Same scenario, but soft-deleted instead of merely blocked — must take the
/// same read-only reporting path (D-REG-03 treats both as "blocked").
#[tokio::test]
async fn soft_deleted_login_is_read_only() {
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

    match result {
        Err(AppError::AccessBlocked {
            pending,
            rejection_reason,
        }) => {
            assert!(!pending);
            assert_eq!(rejection_reason, None);
        }
        other => panic!("expected AccessBlocked, got {other:?}"),
    }

    let readers = svc.readers.clone();
    let count: i64 = tokio::task::spawn_blocking(move || {
        let conn = readers.acquire();
        conn.query_row(
            "SELECT COUNT(*) FROM requests WHERE requested_by_user_id = ?1",
            params![existing_user_id],
            |r| r.get(0),
        )
        .expect("count requests")
    })
    .await
    .expect("spawn_blocking");
    assert_eq!(count, 0, "soft-deleted plain login must also be read-only");
}

// ---------------------------------------------------------------------------
// Explicit `request_ad_restore` — the ONLY path that creates/reuses a
// restore request. Idempotent: calling it twice must reuse the same open
// request, never duplicate.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn request_ad_restore_creates_open_request() {
    let (svc, _dir) = make_auth_service_with_ad();
    svc.set_ad_enabled(true, &admin_caller())
        .await
        .expect("enable AD");

    let existing_user_id = seed_blocked_ad_user(&svc, "us100", "Иванов Иван Иванович", false).await;

    svc.request_ad_restore("us100", "Passw0rd!")
        .await
        .expect("explicit restore request should succeed");

    let readers = svc.readers.clone();
    let (request_type, ad_subtype, status, requested_by): (String, Option<String>, String, i64) =
        tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            conn.query_row(
                "SELECT request_type, ad_subtype, status, requested_by_user_id FROM requests \
                 WHERE requested_by_user_id = ?1 ORDER BY id DESC LIMIT 1",
                params![existing_user_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .expect("query request row")
        })
        .await
        .expect("spawn_blocking");
    assert_eq!(request_type, "ad_register");
    assert_eq!(ad_subtype, Some("restore".to_string()));
    assert_eq!(status, "open");
    assert_eq!(requested_by, existing_user_id);
}

/// Repeated explicit `request_ad_restore` calls must reuse the same OPEN
/// restore request instead of inserting a duplicate (idempotency preserved
/// from the original Defect 1 fix, now scoped to the explicit action only).
#[tokio::test]
async fn request_ad_restore_is_idempotent() {
    let (svc, _dir) = make_auth_service_with_ad();
    svc.set_ad_enabled(true, &admin_caller())
        .await
        .expect("enable AD");

    let existing_user_id = seed_blocked_ad_user(&svc, "us100", "Иванов Иван Иванович", false).await;

    svc.request_ad_restore("us100", "Passw0rd!")
        .await
        .expect("first explicit restore request");
    svc.request_ad_restore("us100", "Passw0rd!")
        .await
        .expect("second explicit restore request (idempotent)");

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
        "two explicit calls must converge on a single open restore request"
    );
}

/// Anti-enumeration: wrong password on `request_ad_restore` must return the
/// same generic `Unauthorized` as a failed plain login — no distinct error
/// path that would let an attacker probe account existence/state.
#[tokio::test]
async fn request_ad_restore_wrong_password_is_generic_unauthorized() {
    let (svc, _dir) = make_auth_service_with_ad();
    svc.set_ad_enabled(true, &admin_caller())
        .await
        .expect("enable AD");

    let _existing_user_id =
        seed_blocked_ad_user(&svc, "us100", "Иванов Иван Иванович", false).await;

    // MockAdClient::default_fixtures() returns BadCreds for any password not
    // matching the fixture password (see ad_auth.rs for the same pattern).
    let result = svc.request_ad_restore("us100", "WrongPassword!").await;
    assert!(
        matches!(result, Err(AppError::Unauthorized)),
        "wrong password must return generic Unauthorized, got {result:?}"
    );
}

/// Anti-enumeration: `request_ad_restore` on an ACTIVE (non-blocked) user
/// must also return generic `Unauthorized` — it must not be usable as an
/// alternate login path or as an oracle for account state.
#[tokio::test]
async fn request_ad_restore_active_user_is_generic_unauthorized() {
    let (svc, _dir) = make_auth_service_with_ad();
    svc.set_ad_enabled(true, &admin_caller())
        .await
        .expect("enable AD");
    svc.set_ad_auto_accept(true, &admin_caller())
        .await
        .expect("enable auto-accept");

    // Auto-accept login creates an ACTIVE user.
    svc.login(LoginRequest {
        login: "us100".to_string(),
        password: "Passw0rd!".to_string(),
        remember: false,
    })
    .await
    .expect("auto-accept login should succeed");

    let result = svc.request_ad_restore("us100", "Passw0rd!").await;
    assert!(
        matches!(result, Err(AppError::Unauthorized)),
        "request_ad_restore on an active user must return generic Unauthorized, got {result:?}"
    );
}

// ---------------------------------------------------------------------------
// Full lifecycle (verification item c): reject-with-notes → subsequent
// blocked login surfaces the reason → explicit re-request creates a FRESH
// open request (not a reuse of the rejected one).
// ---------------------------------------------------------------------------

#[tokio::test]
async fn reject_then_login_surfaces_reason_then_rerequest_creates_fresh_request() {
    let (svc, _dir) = make_auth_service_with_ad();
    svc.set_ad_enabled(true, &admin_caller())
        .await
        .expect("enable AD");
    let request_svc = make_request_service_sharing(&svc);

    let existing_user_id = seed_blocked_ad_user(&svc, "us100", "Иванов Иван Иванович", false).await;

    // Step 1: explicit request creates the first open restore request.
    svc.request_ad_restore("us100", "Passw0rd!")
        .await
        .expect("first explicit restore request");

    let readers = svc.readers.clone();
    let first_request_id: i64 = tokio::task::spawn_blocking(move || {
        let conn = readers.acquire();
        conn.query_row(
            "SELECT id FROM requests \
             WHERE requested_by_user_id = ?1 AND status = 'open' \
             ORDER BY id DESC LIMIT 1",
            params![existing_user_id],
            |r| r.get(0),
        )
        .expect("query first request id")
    })
    .await
    .expect("spawn_blocking");

    // Step 2: admin rejects with a reason.
    let rejected = request_svc
        .transition(
            RequestTransitionPayload::Reject {
                request_id: first_request_id,
                version: 1,
                notes: Some("Учётная запись заблокирована службой безопасности".to_string()),
            },
            &admin_request_caller(),
        )
        .await
        .expect("reject restore request");
    assert_eq!(rejected.status, "rejected");

    // Step 3: plain blocked login must now surface the rejection reason —
    // read-only, no new request created as a side effect.
    let result = svc
        .login(LoginRequest {
            login: "us100".to_string(),
            password: "Passw0rd!".to_string(),
            remember: false,
        })
        .await;
    match result {
        Err(AppError::AccessBlocked {
            pending,
            rejection_reason,
        }) => {
            assert!(!pending, "rejected request is not pending");
            assert_eq!(
                rejection_reason,
                Some("Учётная запись заблокирована службой безопасности".to_string())
            );
        }
        other => panic!("expected AccessBlocked with rejection reason, got {other:?}"),
    }

    let readers2 = svc.readers.clone();
    let total_requests_after_login: i64 = tokio::task::spawn_blocking(move || {
        let conn = readers2.acquire();
        conn.query_row(
            "SELECT COUNT(*) FROM requests WHERE requested_by_user_id = ?1",
            params![existing_user_id],
            |r| r.get(0),
        )
        .expect("count requests")
    })
    .await
    .expect("spawn_blocking");
    assert_eq!(
        total_requests_after_login, 1,
        "plain login after rejection must not create a new request (still read-only)"
    );

    // Step 4: explicit re-request («Запросить снова») creates a FRESH open
    // request — distinct from the rejected one.
    svc.request_ad_restore("us100", "Passw0rd!")
        .await
        .expect("explicit re-request after rejection");

    let readers3 = svc.readers.clone();
    let (open_count, second_request_id): (i64, i64) = tokio::task::spawn_blocking(move || {
        let conn = readers3.acquire();
        let open_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM requests \
                 WHERE requested_by_user_id = ?1 AND status = 'open'",
                params![existing_user_id],
                |r| r.get(0),
            )
            .expect("count open requests");
        let second_request_id: i64 = conn
            .query_row(
                "SELECT id FROM requests \
                 WHERE requested_by_user_id = ?1 AND status = 'open' \
                 ORDER BY id DESC LIMIT 1",
                params![existing_user_id],
                |r| r.get(0),
            )
            .expect("query second request id");
        (open_count, second_request_id)
    })
    .await
    .expect("spawn_blocking");
    assert_eq!(
        open_count, 1,
        "exactly one open request must exist after re-request"
    );
    assert_ne!(
        second_request_id, first_request_id,
        "re-request must create a FRESH request, not resurrect the rejected one"
    );
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
