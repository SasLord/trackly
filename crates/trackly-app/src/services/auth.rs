//! `AuthService` — application service для управления пользователями и аутентификации.
//!
//! Обеспечивает:
//! - bootstrap-проверку (первый запуск без пользователей)
//! - login с argon2id верификацией
//! - CRUD пользователей с optimistic locking
//! - desktop_identity attribution (D-Desktop-01)
//! - desktop_lock_enabled r/w (D-Desktop-02)
//!
//! **Безопасность:** все argon2 операции (hash + verify) выполняются в
//! `spawn_blocking` (T-05-03 mitigate — CPU-bound, не блокируют async).

use std::sync::Arc;

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Algorithm, Argon2, Params, Version,
};
use rusqlite::OptionalExtension;
use tracing::warn;

use trackly_core::auth::{authorize, Action, Identity, Role};
use trackly_core::error::AppError;
use trackly_core::ports::ad::{AdClient, AuthOutcome};
use trackly_core::primitives::clock::Clock;
use trackly_core::primitives::secret::Secret;
use trackly_infra::db::{pools::ReaderPool, writer_worker::WriterHandle};
use trackly_infra::error_conversions::map_rusqlite;

use crate::dto::auth::{
    ChangePasswordRequest, LoginRequest, UserDto, UserFilter, UserListResponse, UserNew, UserPatch,
};
use crate::dto::device::Pagination;
use crate::dto::printer::WsEvent;

// ---------------------------------------------------------------------------
// Free functions (CPU-bound crypto — always spawn_blocking)
// ---------------------------------------------------------------------------

/// Хэширует пароль через argon2id (OWASP 2024+ params: m=19456 KiB, t=2, p=1).
///
/// Возвращает PHC-formatted string (includes salt + params).
pub fn hash_password(password: &Secret<String>) -> Result<String, AppError> {
    let params = Params::new(19456, 2, 1, None).map_err(|e| AppError::Internal {
        source_chain: format!("argon2 params: {e}"),
    })?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let salt = SaltString::generate(&mut OsRng);
    let hash = argon2
        .hash_password(password.expose().as_bytes(), &salt)
        .map_err(|e| AppError::Internal {
            source_chain: format!("argon2 hash: {e}"),
        })?;
    Ok(hash.to_string())
}

/// Фиксированный argon2id PHC-хэш для «пустой» верификации (CR-05).
///
/// Когда логин не найден, `login` всё равно прогоняет `verify_password`
/// против этого хэша, чтобы время ответа для существующих и несуществующих
/// аккаунтов было сопоставимо (устраняет user-enumeration timing oracle).
///
/// Вычисляется один раз лениво с теми же параметрами argon2id
/// (m=19456, t=2, p=1), что и реальные хэши — гарантирует одинаковую
/// CPU-стоимость verify. Salt фиксирован, поэтому строка стабильна.
fn dummy_password_hash() -> &'static str {
    use std::sync::OnceLock;
    static DUMMY: OnceLock<String> = OnceLock::new();
    DUMMY
        .get_or_init(|| {
            let params = Params::new(19456, 2, 1, None).expect("argon2 dummy params");
            let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
            // Фиксированный валидный base64-salt (без padding).
            let salt = SaltString::from_b64("c29tZWZpeGVkc2FsdDEy").expect("argon2 dummy salt");
            argon2
                .hash_password(b"trackly-dummy-password", &salt)
                .expect("argon2 dummy hash")
                .to_string()
        })
        .as_str()
}

/// Верифицирует пароль против argon2 hash-строки.
///
/// Возвращает `true` если пароль совпадает.
pub fn verify_password(password: &Secret<String>, hash: &str) -> bool {
    let Ok(parsed) = PasswordHash::new(hash) else {
        warn!("verify_password: не удалось распарсить hash");
        return false;
    };
    Argon2::default()
        .verify_password(password.expose().as_bytes(), &parsed)
        .is_ok()
}

/// Внутренний исход `try_local_login` — нужен `login()`, чтобы решить,
/// делать ли AD fallback (только когда `UnknownLogin`, не `BadPassword` —
/// иначе локальный пользователь с забытым паролем мог бы случайно
/// аутентифицироваться через AD bind по тому же login).
enum LocalLoginOutcome {
    Success(UserDto),
    UnknownLogin,
    BadPassword,
}

/// Состояние МОСТ-РЕЦЕНТНОЙ заявки восстановления доступа для пользователя
/// (read seam, 09-AD-GAPS restoration-flow UX) — питает `AppError::AccessBlocked`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct RestoreRequestState {
    /// `true`, если уже существует открытая заявка восстановления.
    pending: bool,
    /// Причина отклонения последней заявки, если последняя заявка была
    /// отклонена и сейчас нет открытой.
    rejection_reason: Option<String>,
}

/// Результат `find_user_any_state` — состояние пользователя в БД без
/// active/non-deleted фильтра (read seam для AD-bind-success branching).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserAnyState {
    pub id: i64,
    pub role: String,
    pub is_active: bool,
    pub deleted: bool,
    /// `true` если есть открытая заявка `ad_register`/`ad_subtype='register'`
    /// для этого пользователя — значит, регистрация ещё ни разу не была
    /// одобрена (pending). Отличает "pending" (`is_active=0`, никогда не
    /// активировался) от "blocked" (`is_active=0`, но был одобрен/активен
    /// ранее и затем заблокирован админом) — обе ветки имеют одинаковые
    /// `is_active=false, deleted=false`, но требуют разной обработки в
    /// `on_ad_bind_success` (09-AD-GAPS, lower-priority item).
    pub has_open_register_request: bool,
}

// ---------------------------------------------------------------------------
// AuthService
// ---------------------------------------------------------------------------

/// Application service для аутентификации и управления пользователями.
///
/// `Arc`-wrapped fields → Clone O(1) (Tauri State + axum State).
#[derive(Clone)]
pub struct AuthService {
    pub writer: Arc<WriterHandle>,
    pub readers: Arc<ReaderPool>,
    pub(crate) clock: Arc<dyn Clock + Send + Sync>,
    /// AD client — `RealAdClient` in prod, `MockAdClient` on dev macOS
    /// (D-Mock-01). Used by `login()`'s local→AD fallback (USR-08).
    pub(crate) ad_client: Arc<dyn AdClient + Send + Sync>,
    /// WS broadcast sender — shared with `RequestService` (same `AppCtx`
    /// channel) so `ad_register` requests created from `on_ad_bind_success`
    /// emit `WsEvent::NewRequest` to admins too (REQ-04 reuse, Phase 9 Plan 03).
    pub(crate) ws_tx: Arc<tokio::sync::broadcast::Sender<WsEvent>>,
}

impl AuthService {
    /// Создать новый `AuthService`.
    pub fn new(
        writer: Arc<WriterHandle>,
        readers: Arc<ReaderPool>,
        clock: Arc<dyn Clock + Send + Sync>,
        ad_client: Arc<dyn AdClient + Send + Sync>,
        ws_tx: Arc<tokio::sync::broadcast::Sender<WsEvent>>,
    ) -> Self {
        Self {
            writer,
            readers,
            clock,
            ad_client,
            ws_tx,
        }
    }

    // -----------------------------------------------------------------------
    // Bootstrap
    // -----------------------------------------------------------------------

    /// Проверяет, нужна ли начальная настройка (нет ни одного admin-пользователя).
    ///
    /// Возвращает `true` если в таблице `users` нет активных admin-пользователей.
    pub async fn needs_bootstrap(&self) -> Result<bool, AppError> {
        let readers = self.readers.clone();
        tokio::task::spawn_blocking(move || -> Result<bool, AppError> {
            let conn = readers.acquire();
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM users \
                     WHERE role = 'admin' AND deleted_at_utc IS NULL",
                    [],
                    |r| r.get(0),
                )
                .map_err(map_rusqlite)?;
            Ok(count == 0)
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking needs_bootstrap: {e}"),
        })?
    }

    // -----------------------------------------------------------------------
    // Login / auth
    // -----------------------------------------------------------------------

    /// Получить хэш пароля для активного пользователя.
    ///
    /// `password_hash IS NOT NULL` исключает AD-only пользователей
    /// (`password_hash = NULL`, см. V002 — "NULL for AD users (bind-only)").
    /// Без этого условия `r.get::<_, String>(0)` вернул бы
    /// `InvalidColumnType` для NULL вместо `QueryReturnedNoRows`, и AD
    /// fallback в `try_local_login` никогда бы не сработал для AD-only
    /// пользователей (Rule 1 fix, Phase 9 Plan 02).
    async fn get_password_hash(&self, login: &str) -> Result<String, AppError> {
        let readers = self.readers.clone();
        let login = login.to_string();
        tokio::task::spawn_blocking(move || -> Result<String, AppError> {
            let conn = readers.acquire();
            let result: rusqlite::Result<String> = conn.query_row(
                "SELECT password_hash FROM users \
                 WHERE login = ?1 AND deleted_at_utc IS NULL AND is_active = 1 \
                   AND password_hash IS NOT NULL",
                rusqlite::params![login],
                |r| r.get(0),
            );
            match result {
                Ok(h) => Ok(h),
                Err(rusqlite::Error::QueryReturnedNoRows) => Err(AppError::Unauthorized),
                Err(e) => Err(map_rusqlite(e)),
            }
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking get_password_hash: {e}"),
        })?
    }

    /// Аутентифицировать пользователя по логину и паролю.
    ///
    /// Сначала пробует локальный (argon2id) логин. Если локальный логин не
    /// найден (а не "пароль неверный" — см. `try_local_login`) и AD включён
    /// (`ad_enabled` в `app_settings`), пробует AD bind как fallback (USR-08).
    ///
    /// Верификация argon2 выполняется в `spawn_blocking` (T-05-03).
    pub async fn login(&self, req: LoginRequest) -> Result<UserDto, AppError> {
        match self.try_local_login(&req).await? {
            LocalLoginOutcome::Success(dto) => Ok(dto),
            LocalLoginOutcome::UnknownLogin => {
                // Локальный пользователь с таким login не найден (в отличие
                // от "пароль неверный для известного login" — см. ниже).
                // Только в этом случае пробуем AD fallback: если бы мы
                // пробовали AD и для known-but-wrong-password случая, мы
                // бы создали оракул (AD bind timing отличается от argon2
                // timing) — но т.к. для known-login случая мы уже вернули
                // Unauthorized с потраченным constant-time CPU, branching
                // здесь безопасен (CR-05 защищает именно ветку "логин
                // неизвестен локально").
                if !self.ad_enabled().await? {
                    return Err(AppError::Unauthorized);
                }
                self.try_ad_login(&req).await
            }
            LocalLoginOutcome::BadPassword => Err(AppError::Unauthorized),
        }
    }

    /// Passwordless AD SSO login (spike-002 / Kerberos-SPNEGO).
    ///
    /// The caller (`/api/v1/auth_ad_sso` HTTP handler) has ALREADY authenticated the user
    /// by validating their Kerberos ticket against the service keytab server-side — there
    /// is no LDAP bind and no password here. We only run the *same* provisioning seam the
    /// LDAPS-bind path uses (`on_ad_bind_success`), so an SSO user resolves to a Trackly
    /// account with identical semantics to plain AD login: active → session-eligible
    /// `UserDto`; pending/blocked → the same `RegistrationPending`/`AccessBlocked` errors.
    ///
    /// SSO requires AD to be enabled (same gate as `try_ad_login`'s fallback).
    ///
    /// NOTE (full-parity follow-up): `display_name` currently falls back to the SAM login
    /// because SSO has no bind to search from. A service-account displayName lookup (as in
    /// the adwebapp reference) is deferred to the AD-SSO milestone.
    pub async fn sso_login(
        &self,
        ad_username: &str,
        display_name: &str,
    ) -> Result<UserDto, AppError> {
        if !self.ad_enabled().await? {
            return Err(AppError::Unauthorized);
        }
        self.on_ad_bind_success(ad_username, display_name).await
    }

    /// Пробует локальный (argon2id) логин. Не делает constant-time
    /// различия между "пользователь не найден" и "пароль неверный" по
    /// времени (CR-05 dummy-hash verify), но возвращает разные исходы
    /// вызывающей стороне, чтобы `login()` мог решить про AD fallback
    /// (AD fallback должен срабатывать только когда местный пользователь
    /// отсутствует — иначе локальный пользователь с забытым паролем мог
    /// бы случайно аутентифицироваться через AD bind с тем же логином).
    async fn try_local_login(&self, req: &LoginRequest) -> Result<LocalLoginOutcome, AppError> {
        // CR-05: устранение user-enumeration timing oracle.
        // Если логин не найден (или неактивен), get_password_hash возвращает
        // Unauthorized. Вместо немедленного отказа мы всё равно прогоняем
        // argon2-verify против фиксированного dummy-хэша — обе ветки тратят
        // сопоставимый CPU, после чего возвращаем Unauthorized.
        let (hash, user_known) = match self.get_password_hash(&req.login).await {
            Ok(h) => (h, true),
            Err(AppError::Unauthorized) => (dummy_password_hash().to_string(), false),
            Err(e) => return Err(e),
        };
        let password = Secret::new(req.password.clone());

        // CPU-bound verify — в spawn_blocking (T-05-03)
        let verified = tokio::task::spawn_blocking(move || verify_password(&password, &hash))
            .await
            .map_err(|e| AppError::Internal {
                source_chain: format!("spawn_blocking verify_password: {e}"),
            })?;

        if !user_known {
            return Ok(LocalLoginOutcome::UnknownLogin);
        }
        if !verified {
            return Ok(LocalLoginOutcome::BadPassword);
        }

        self.get_by_login(&req.login)
            .await
            .map(LocalLoginOutcome::Success)
    }

    /// AD bind fallback (USR-08). Вызывается только когда локальный
    /// пользователь с этим login не найден и `ad_enabled = true`.
    async fn try_ad_login(&self, req: &LoginRequest) -> Result<UserDto, AppError> {
        // Pitfall 1 (CRITICAL, RFC 4513 §5.1.2): пустой/whitespace-only
        // пароль ДО какого-либо AD bind — LDAP simple bind с пустым
        // паролем — это anonymous bind, который многие AD-серверы
        // принимают как "успех" независимо от пароля.
        if req.password.trim().is_empty() {
            return Err(AppError::Unauthorized);
        }

        let password = Secret::new(req.password.clone());
        let outcome = self.ad_client.authenticate(&req.login, &password).await?;

        match outcome {
            AuthOutcome::BadCreds => Err(AppError::Unauthorized),
            AuthOutcome::Unreachable => Err(AppError::ServiceUnavailable { service: "ad" }),
            AuthOutcome::Ok { display_name } => {
                self.on_ad_bind_success(&req.login, &display_name).await
            }
        }
    }

    /// После успешного AD bind — определяет, что делать с локальной
    /// учётной записью (read seam: `find_user_any_state`).
    ///
    /// Три ветки (Phase 9 Plan 03):
    /// - active user → обычная сессия.
    /// - blocked/soft-deleted user → READ-ONLY (09-AD-GAPS restoration-flow
    ///   UX): plain login НЕ создаёт заявку восстановления больше — только
    ///   читает состояние последней заявки и возвращает обогащённый
    ///   `AppError::AccessBlocked`. Явное создание — `request_ad_restore`.
    /// - unknown (нет в БД) → auto-accept (создать сразу + info-заявка) или
    ///   pending (inactive user + заявка, `AppError::RegistrationPending`)
    ///   в зависимости от `ad_auto_accept`.
    async fn on_ad_bind_success(
        &self,
        login: &str,
        display_name: &str,
    ) -> Result<UserDto, AppError> {
        match self.find_user_any_state(login).await? {
            Some(found) if found.is_active && !found.deleted => self.get_by_login(login).await,
            // Pending registration (never approved yet): `is_active=0`,
            // NOT soft-deleted, AND an open 'register'-subtype request still
            // exists — distinct from blocked (also `is_active=0`, not
            // deleted, but WAS approved/active before being blocked by an
            // admin: that user has no open 'register' request because it
            // was completed at approval time). Re-attempting login here must
            // stay on the registration-pending path (reuse the existing
            // 'register' request), not be routed into the restore branch,
            // which would surface a SECOND ('restore'-subtype) request state
            // for a user who was never approved in the first place.
            Some(pending)
                if !pending.is_active && !pending.deleted && pending.has_open_register_request =>
            {
                self.reuse_or_create_pending_registration(pending.id, login, display_name)
                    .await
            }
            Some(blocked_or_deleted) => self.report_blocked_access(blocked_or_deleted.id).await,
            None => {
                if self.ad_auto_accept().await? {
                    self.auto_register_ad_user(login, display_name).await
                } else {
                    self.create_pending_registration(login, display_name).await
                }
            }
        }
    }

    /// Re-attempted bind for a user already in the pending-registration
    /// state (`is_active=0`, not soft-deleted, an open `ad_register`/
    /// `ad_subtype='register'` request already exists for them).
    ///
    /// Reuses the existing open request instead of calling
    /// `create_pending_registration` (which always INSERTs a new `users`
    /// row — wrong here, the row already exists) or `create_restore_request`
    /// (wrong subtype — this user was never approved, there is nothing to
    /// "restore").
    async fn reuse_or_create_pending_registration(
        &self,
        existing_user_id: i64,
        login: &str,
        display_name: &str,
    ) -> Result<UserDto, AppError> {
        let now = self.clock.unix_seconds();
        let display_name_owned = display_name.to_string();
        let login_owned = login.to_string();

        let (request_id, reused) = self
            .writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;

                let existing: Option<i64> = tx
                    .query_row(
                        "SELECT id FROM requests \
                         WHERE request_type = 'ad_register' AND ad_subtype = 'register' \
                           AND requested_by_user_id = ?1 AND status = 'open' \
                           AND deleted_at_utc IS NULL",
                        rusqlite::params![existing_user_id],
                        |r| r.get(0),
                    )
                    .optional()
                    .map_err(map_rusqlite)?;

                if let Some(request_id) = existing {
                    tx.commit().map_err(map_rusqlite)?;
                    return Ok((request_id, true));
                }

                // Defensive fallback: user is pending but somehow has no
                // open 'register' request (shouldn't happen via normal
                // flow, but don't leave the user stuck with no request).
                tx.execute(
                    "INSERT INTO requests \
                     (request_type, status, requested_by_user_id, description, ad_subtype, \
                      created_at_utc, updated_at_utc, version) \
                     VALUES ('ad_register', 'open', ?1, ?2, 'register', ?3, ?3, 1)",
                    rusqlite::params![existing_user_id, display_name_owned, now],
                )
                .map_err(map_rusqlite)?;
                let request_id = tx.last_insert_rowid();

                tx.execute(
                    "INSERT INTO audit_log \
                     (entity_type, entity_id, action, user_id, before_json, after_json, \
                      payload_json, created_at_utc) \
                     VALUES ('request', ?1, 'create', ?2, NULL, NULL, ?3, ?4)",
                    rusqlite::params![
                        request_id,
                        existing_user_id,
                        serde_json::json!({ "login": login_owned }).to_string(),
                        now
                    ],
                )
                .map_err(map_rusqlite)?;

                tx.commit().map_err(map_rusqlite)?;
                Ok((request_id, false))
            })
            .await?;

        if !reused {
            let _ = self.ws_tx.send(WsEvent::NewRequest {
                request_id,
                request_type: "ad_register".to_string(),
                requester_name: display_name.to_string(),
            });
        }

        Err(AppError::RegistrationPending { request_id })
    }

    /// auto-accept ON, unknown AD user (USR-11/SET-10): создаёт активного
    /// пользователя СРАЗУ + информационную заявку `ad_register` (ad_subtype='register')
    /// в ОДНОЙ writer-транзакции, затем возвращает сессию для нового пользователя.
    ///
    /// Reject этой заявки в дальнейшем soft-delete'ит пользователя
    /// (см. `RequestService::approve_ad_register`/reject branching, Task 2).
    async fn auto_register_ad_user(
        &self,
        login: &str,
        display_name: &str,
    ) -> Result<UserDto, AppError> {
        let now = self.clock.unix_seconds();
        let login_owned = login.to_string();
        let display_name_owned = display_name.to_string();

        let (user_id, request_id) = self
            .writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;

                tx.execute(
                    "INSERT INTO users \
                     (login, full_name, password_hash, role, ad_user, is_active, \
                      created_at_utc, updated_at_utc, version) \
                     VALUES (?1, ?2, NULL, 'employee', 1, 1, ?3, ?3, 1)",
                    rusqlite::params![login_owned, display_name_owned, now],
                )
                .map_err(map_rusqlite)?;
                let user_id = tx.last_insert_rowid();

                tx.execute(
                    "INSERT INTO audit_log \
                     (entity_type, entity_id, action, user_id, before_json, after_json, \
                      payload_json, created_at_utc) \
                     VALUES ('user', ?1, 'ad_auto_register', ?1, NULL, NULL, NULL, ?2)",
                    rusqlite::params![user_id, now],
                )
                .map_err(map_rusqlite)?;

                tx.execute(
                    "INSERT INTO requests \
                     (request_type, status, requested_by_user_id, description, ad_subtype, \
                      created_at_utc, updated_at_utc, version) \
                     VALUES ('ad_register', 'open', ?1, ?2, 'register', ?3, ?3, 1)",
                    rusqlite::params![user_id, display_name_owned, now],
                )
                .map_err(map_rusqlite)?;
                let request_id = tx.last_insert_rowid();

                tx.execute(
                    "INSERT INTO audit_log \
                     (entity_type, entity_id, action, user_id, before_json, after_json, \
                      payload_json, created_at_utc) \
                     VALUES ('request', ?1, 'create', ?2, NULL, NULL, NULL, ?3)",
                    rusqlite::params![request_id, user_id, now],
                )
                .map_err(map_rusqlite)?;

                tx.commit().map_err(map_rusqlite)?;
                Ok((user_id, request_id))
            })
            .await?;

        // WS push (REQ-04 reuse) — admins получают уведомление о новой заявке.
        let _ = self.ws_tx.send(WsEvent::NewRequest {
            request_id,
            request_type: "ad_register".to_string(),
            requester_name: display_name.to_string(),
        });

        self.get_user_by_id(user_id).await
    }

    /// auto-accept OFF, unknown AD user (USR-09/USR-11): создаёт НЕактивного
    /// пользователя (Pitfall 4 — FK-таргет для `requested_by_user_id`) +
    /// заявку `ad_register` (ad_subtype='register') в ОДНОЙ writer-транзакции.
    /// Сессия НЕ выдаётся — возвращает `AppError::RegistrationPending`.
    async fn create_pending_registration(
        &self,
        login: &str,
        display_name: &str,
    ) -> Result<UserDto, AppError> {
        let now = self.clock.unix_seconds();
        let login_owned = login.to_string();
        let display_name_owned = display_name.to_string();

        let (_user_id, request_id) = self
            .writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;

                tx.execute(
                    "INSERT INTO users \
                     (login, full_name, password_hash, role, ad_user, is_active, \
                      created_at_utc, updated_at_utc, version) \
                     VALUES (?1, ?2, NULL, 'employee', 1, 0, ?3, ?3, 1)",
                    rusqlite::params![login_owned, display_name_owned, now],
                )
                .map_err(map_rusqlite)?;
                let user_id = tx.last_insert_rowid();

                tx.execute(
                    "INSERT INTO audit_log \
                     (entity_type, entity_id, action, user_id, before_json, after_json, \
                      payload_json, created_at_utc) \
                     VALUES ('user', ?1, 'ad_pending_register', ?1, NULL, NULL, NULL, ?2)",
                    rusqlite::params![user_id, now],
                )
                .map_err(map_rusqlite)?;

                tx.execute(
                    "INSERT INTO requests \
                     (request_type, status, requested_by_user_id, description, ad_subtype, \
                      created_at_utc, updated_at_utc, version) \
                     VALUES ('ad_register', 'open', ?1, ?2, 'register', ?3, ?3, 1)",
                    rusqlite::params![user_id, display_name_owned, now],
                )
                .map_err(map_rusqlite)?;
                let request_id = tx.last_insert_rowid();

                tx.execute(
                    "INSERT INTO audit_log \
                     (entity_type, entity_id, action, user_id, before_json, after_json, \
                      payload_json, created_at_utc) \
                     VALUES ('request', ?1, 'create', ?2, NULL, NULL, NULL, ?3)",
                    rusqlite::params![request_id, user_id, now],
                )
                .map_err(map_rusqlite)?;

                tx.commit().map_err(map_rusqlite)?;
                Ok((user_id, request_id))
            })
            .await?;

        let _ = self.ws_tx.send(WsEvent::NewRequest {
            request_id,
            request_type: "ad_register".to_string(),
            requester_name: display_name.to_string(),
        });

        Err(AppError::RegistrationPending { request_id })
    }

    /// Blocked/soft-deleted AD user успешно прошёл bind (D-REG-03):
    /// READ-ONLY (09-AD-GAPS restoration-flow UX) — НЕ создаёт заявку
    /// восстановления. Только читает состояние МОСТ-РЕЦЕНТНОЙ заявки
    /// восстановления (`ad_register`/`ad_subtype='restore'`) для этого
    /// пользователя и возвращает обогащённый `AppError::AccessBlocked`:
    /// - открытая заявка существует → `pending=true`.
    /// - нет открытой, но последняя была отклонена → `rejection_reason`.
    /// - заявок не было вообще → оба поля пустые/false.
    ///
    /// Явное создание новой заявки — `request_ad_restore` (требует AD bind
    /// заново, т.к. у блокированного пользователя нет сессии).
    async fn report_blocked_access(&self, existing_user_id: i64) -> Result<UserDto, AppError> {
        let state = self.latest_restore_request_state(existing_user_id).await?;
        Err(AppError::AccessBlocked {
            pending: state.pending,
            rejection_reason: state.rejection_reason,
        })
    }

    /// Читает состояние МОСТ-РЕЦЕНТНОЙ заявки восстановления
    /// (`ad_register`/`ad_subtype='restore'`) для пользователя:
    /// - есть открытая → `{ pending: true, rejection_reason: None }`.
    /// - последняя (по `id`) — отклонена и сейчас нет открытой →
    ///   `{ pending: false, rejection_reason: Some(notes) }` (notes из
    ///   `requests.resolution_notes` — канонического столбца, который
    ///   `transition_in_tx`/reject записывает, см. requests_sqlite.rs).
    /// - заявок не было вообще (или последняя была одобрена, что не должно
    ///   приводить пользователя сюда, но обрабатываем защитно) →
    ///   `{ pending: false, rejection_reason: None }`.
    async fn latest_restore_request_state(
        &self,
        user_id: i64,
    ) -> Result<RestoreRequestState, AppError> {
        let readers = self.readers.clone();
        tokio::task::spawn_blocking(move || -> Result<RestoreRequestState, AppError> {
            let conn = readers.acquire();

            let has_open: bool = conn
                .query_row(
                    "SELECT EXISTS ( \
                         SELECT 1 FROM requests \
                         WHERE request_type = 'ad_register' AND ad_subtype = 'restore' \
                           AND requested_by_user_id = ?1 AND status = 'open' \
                           AND deleted_at_utc IS NULL \
                     )",
                    rusqlite::params![user_id],
                    |r| r.get::<_, i64>(0),
                )
                .map_err(map_rusqlite)?
                != 0;

            if has_open {
                return Ok(RestoreRequestState {
                    pending: true,
                    rejection_reason: None,
                });
            }

            // No open request — look at the most recent restore request
            // (any status) for a rejection reason. `resolution_notes` is
            // the canonical store for reject notes (see
            // `requests_sqlite.rs::transition_in_tx`'s `UPDATE ... SET
            // resolution_notes = COALESCE(?2, resolution_notes)`).
            let latest: Option<(String, Option<String>)> = conn
                .query_row(
                    "SELECT status, resolution_notes FROM requests \
                     WHERE request_type = 'ad_register' AND ad_subtype = 'restore' \
                       AND requested_by_user_id = ?1 AND deleted_at_utc IS NULL \
                     ORDER BY id DESC LIMIT 1",
                    rusqlite::params![user_id],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .optional()
                .map_err(map_rusqlite)?;

            let rejection_reason = match latest {
                Some((status, notes)) if status == "rejected" => notes,
                _ => None,
            };

            Ok(RestoreRequestState {
                pending: false,
                rejection_reason,
            })
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking latest_restore_request_state: {e}"),
        })?
    }

    /// EXPLICIT re-request action (09-AD-GAPS restoration-flow UX):
    /// блокированный/soft-deleted пользователь явно запрашивает
    /// восстановление доступа из `BlockedScreen`. Требует ПОВТОРНОГО AD
    /// bind с логином+паролем (у пользователя нет сессии — это
    /// неаутентифицированный вызов, который сам несёт credentials, как
    /// `auth_login`), затем идемпотентно создаёт (или переиспользует)
    /// открытую заявку восстановления.
    ///
    /// **Anti-enumeration:** неверные credentials возвращают тот же
    /// generic `AppError::Unauthorized`, что и обычный `login()` — не
    /// раскрывает существование/состояние аккаунта.
    pub async fn request_ad_restore(&self, login: &str, password: &str) -> Result<(), AppError> {
        // Pitfall 1 (RFC 4513 §5.1.2): empty/whitespace password is an
        // anonymous bind trap — reject before any AD bind (mirrors
        // `try_ad_login`).
        if password.trim().is_empty() {
            return Err(AppError::Unauthorized);
        }
        if !self.ad_enabled().await? {
            return Err(AppError::Unauthorized);
        }

        let secret = Secret::new(password.to_string());
        let outcome = self.ad_client.authenticate(login, &secret).await?;

        let display_name = match outcome {
            AuthOutcome::BadCreds => return Err(AppError::Unauthorized),
            AuthOutcome::Unreachable => return Err(AppError::ServiceUnavailable { service: "ad" }),
            AuthOutcome::Ok { display_name } => display_name,
        };

        // Bind succeeded — confirm the user is actually blocked/soft-deleted.
        // A caller who somehow has valid AD creds for an active/unknown/
        // pending-registration user gets the SAME generic Unauthorized —
        // this endpoint exists ONLY for the already-blocked restoration
        // flow, not as an alternate login path.
        let target_user_id = match self.find_user_any_state(login).await? {
            Some(found) if found.is_active && !found.deleted => return Err(AppError::Unauthorized),
            Some(pending)
                if !pending.is_active && !pending.deleted && pending.has_open_register_request =>
            {
                return Err(AppError::Unauthorized)
            }
            Some(blocked_or_deleted) => blocked_or_deleted.id,
            None => return Err(AppError::Unauthorized),
        };

        self.ensure_open_restore_request(target_user_id, login, &display_name)
            .await
    }

    /// Idempotent INSERT helper (09-AD-GAPS Defect 1 fix, reused by the
    /// explicit `request_ad_restore` action): check-then-insert inside ONE
    /// writer transaction for an existing OPEN restore request; if found,
    /// reuse it instead of inserting a duplicate. No race window with a
    /// concurrent call for the same user — both run on the single writer
    /// connection.
    async fn ensure_open_restore_request(
        &self,
        existing_user_id: i64,
        login: &str,
        display_name: &str,
    ) -> Result<(), AppError> {
        let now = self.clock.unix_seconds();
        let display_name_owned = display_name.to_string();
        let login_owned = login.to_string();

        let (request_id, reused) = self
            .writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;

                let existing: Option<i64> = tx
                    .query_row(
                        "SELECT id FROM requests \
                         WHERE request_type = 'ad_register' AND ad_subtype = 'restore' \
                           AND requested_by_user_id = ?1 AND status = 'open' \
                           AND deleted_at_utc IS NULL",
                        rusqlite::params![existing_user_id],
                        |r| r.get(0),
                    )
                    .optional()
                    .map_err(map_rusqlite)?;

                if let Some(request_id) = existing {
                    tx.commit().map_err(map_rusqlite)?;
                    return Ok((request_id, true));
                }

                tx.execute(
                    "INSERT INTO requests \
                     (request_type, status, requested_by_user_id, description, ad_subtype, \
                      created_at_utc, updated_at_utc, version) \
                     VALUES ('ad_register', 'open', ?1, ?2, 'restore', ?3, ?3, 1)",
                    rusqlite::params![existing_user_id, display_name_owned, now],
                )
                .map_err(map_rusqlite)?;
                let request_id = tx.last_insert_rowid();

                tx.execute(
                    "INSERT INTO audit_log \
                     (entity_type, entity_id, action, user_id, before_json, after_json, \
                      payload_json, created_at_utc) \
                     VALUES ('request', ?1, 'create', ?2, NULL, NULL, ?3, ?4)",
                    rusqlite::params![
                        request_id,
                        existing_user_id,
                        serde_json::json!({ "login": login_owned }).to_string(),
                        now
                    ],
                )
                .map_err(map_rusqlite)?;

                tx.commit().map_err(map_rusqlite)?;
                Ok((request_id, false))
            })
            .await?;

        if !reused {
            let _ = self.ws_tx.send(WsEvent::NewRequest {
                request_id,
                request_type: "ad_register".to_string(),
                requester_name: display_name.to_string(),
            });
        }

        Ok(())
    }

    /// Читает `ad_enabled` из `app_settings`. По умолчанию `false`
    /// (AD fallback выключен, пока админ явно не включит).
    pub async fn ad_enabled(&self) -> Result<bool, AppError> {
        let readers = self.readers.clone();
        tokio::task::spawn_blocking(move || -> Result<bool, AppError> {
            let conn = readers.acquire();
            let result: rusqlite::Result<String> = conn.query_row(
                "SELECT value FROM app_settings WHERE key = 'ad_enabled'",
                [],
                |r| r.get(0),
            );
            match result {
                Ok(v) => Ok(v == "1"),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(false),
                Err(e) => Err(map_rusqlite(e)),
            }
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking ad_enabled: {e}"),
        })?
    }

    /// Устанавливает `ad_enabled` в `app_settings`. Требует `ManageSettings`.
    pub async fn set_ad_enabled(&self, enabled: bool, caller: &Identity) -> Result<(), AppError> {
        authorize(caller, &Action::ManageSettings)?;
        let value = if enabled { "1" } else { "0" };
        let now = self.clock.unix_seconds();
        self.writer
            .execute(move |conn| {
                conn.execute(
                    "INSERT INTO app_settings (key, value, created_at_utc, updated_at_utc) \
                     VALUES ('ad_enabled', ?1, ?2, ?2) \
                     ON CONFLICT(key) DO UPDATE SET value = ?1, updated_at_utc = ?2",
                    rusqlite::params![value, now],
                )
                .map(|_| ())
                .map_err(map_rusqlite)
            })
            .await
    }

    /// Читает `ad_sso_enabled` из `app_settings` (passwordless Kerberos/SPNEGO
    /// вход). По умолчанию `false`. Отдельный тумблер от `ad_enabled`: LDAPS-вход
    /// логином/паролем и AD-SSO включаются независимо.
    pub async fn ad_sso_enabled(&self) -> Result<bool, AppError> {
        let readers = self.readers.clone();
        tokio::task::spawn_blocking(move || -> Result<bool, AppError> {
            let conn = readers.acquire();
            let result: rusqlite::Result<String> = conn.query_row(
                "SELECT value FROM app_settings WHERE key = 'ad_sso_enabled'",
                [],
                |r| r.get(0),
            );
            match result {
                Ok(v) => Ok(v == "1"),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(false),
                Err(e) => Err(map_rusqlite(e)),
            }
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking ad_sso_enabled: {e}"),
        })?
    }

    /// Устанавливает `ad_sso_enabled` в `app_settings`. Требует `ManageSettings`.
    pub async fn set_ad_sso_enabled(&self, enabled: bool, caller: &Identity) -> Result<(), AppError> {
        authorize(caller, &Action::ManageSettings)?;
        let value = if enabled { "1" } else { "0" };
        let now = self.clock.unix_seconds();
        self.writer
            .execute(move |conn| {
                conn.execute(
                    "INSERT INTO app_settings (key, value, created_at_utc, updated_at_utc) \
                     VALUES ('ad_sso_enabled', ?1, ?2, ?2) \
                     ON CONFLICT(key) DO UPDATE SET value = ?1, updated_at_utc = ?2",
                    rusqlite::params![value, now],
                )
                .map(|_| ())
                .map_err(map_rusqlite)
            })
            .await
    }

    /// Проверяет доступность AD-сервера БЕЗ учётных данных пользователя
    /// (admin-действие "Проверить подключение", Phase 9 gap-closure).
    ///
    /// Требует `ManageSettings` (тот же gate, что и `set_ad_enabled`) — это
    /// admin-only диагностика, не login-путь. Делегирует в `AdClient::
    /// test_connection`, который выполняет TCP+TLS connect (опционально
    /// anonymous bind) без пароля конечного пользователя.
    pub async fn test_ad_connection(&self, caller: &Identity) -> Result<(), AppError> {
        authorize(caller, &Action::ManageSettings)?;
        match self.ad_client.test_connection().await? {
            AuthOutcome::Ok { .. } => Ok(()),
            AuthOutcome::Unreachable => Err(AppError::ServiceUnavailable { service: "ad" }),
            // test_connection never presents credentials, so BadCreds is
            // not a reachable outcome here — treat defensively as Unreachable.
            AuthOutcome::BadCreds => Err(AppError::ServiceUnavailable { service: "ad" }),
        }
    }

    /// Читает `ad_auto_accept` из `app_settings`. По умолчанию `false`
    /// (заявки на регистрацию/восстановление требуют ручного подтверждения
    /// администратором — auto-accept — explicit opt-in, план 09-03 consumer).
    pub async fn ad_auto_accept(&self) -> Result<bool, AppError> {
        let readers = self.readers.clone();
        tokio::task::spawn_blocking(move || -> Result<bool, AppError> {
            let conn = readers.acquire();
            let result: rusqlite::Result<String> = conn.query_row(
                "SELECT value FROM app_settings WHERE key = 'ad_auto_accept'",
                [],
                |r| r.get(0),
            );
            match result {
                Ok(v) => Ok(v == "1"),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(false),
                Err(e) => Err(map_rusqlite(e)),
            }
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking ad_auto_accept: {e}"),
        })?
    }

    /// Устанавливает `ad_auto_accept` в `app_settings`. Требует `ManageSettings`.
    pub async fn set_ad_auto_accept(
        &self,
        enabled: bool,
        caller: &Identity,
    ) -> Result<(), AppError> {
        authorize(caller, &Action::ManageSettings)?;
        let value = if enabled { "1" } else { "0" };
        let now = self.clock.unix_seconds();
        self.writer
            .execute(move |conn| {
                conn.execute(
                    "INSERT INTO app_settings (key, value, created_at_utc, updated_at_utc) \
                     VALUES ('ad_auto_accept', ?1, ?2, ?2) \
                     ON CONFLICT(key) DO UPDATE SET value = ?1, updated_at_utc = ?2",
                    rusqlite::params![value, now],
                )
                .map(|_| ())
                .map_err(map_rusqlite)
            })
            .await
    }

    /// Read seam (Open Question 3, resolved): найти пользователя по login
    /// БЕЗ фильтра active/non-deleted — возвращает состояние независимо
    /// от того, активен/удалён ли пользователь, чтобы post-AD-bind логика
    /// могла различить active / blocked / soft-deleted / unknown.
    ///
    /// Возвращает `None`, если такого login вообще нет в таблице `users`
    /// (включая soft-deleted записи — не путать с "найден, но deleted").
    pub async fn find_user_any_state(&self, login: &str) -> Result<Option<UserAnyState>, AppError> {
        let readers = self.readers.clone();
        let login = login.to_string();
        tokio::task::spawn_blocking(move || -> Result<Option<UserAnyState>, AppError> {
            let conn = readers.acquire();
            let result = conn.query_row(
                "SELECT u.id, u.role, u.is_active, u.deleted_at_utc IS NOT NULL AS deleted, \
                        EXISTS ( \
                            SELECT 1 FROM requests r \
                            WHERE r.request_type = 'ad_register' AND r.ad_subtype = 'register' \
                              AND r.requested_by_user_id = u.id AND r.status = 'open' \
                              AND r.deleted_at_utc IS NULL \
                        ) AS has_open_register_request \
                 FROM users u WHERE u.login = ?1",
                rusqlite::params![login],
                |row| {
                    let is_active_i64: i64 = row.get(2)?;
                    let deleted_i64: i64 = row.get(3)?;
                    let has_open_register_i64: i64 = row.get(4)?;
                    Ok(UserAnyState {
                        id: row.get(0)?,
                        role: row.get(1)?,
                        is_active: is_active_i64 != 0,
                        deleted: deleted_i64 != 0,
                        has_open_register_request: has_open_register_i64 != 0,
                    })
                },
            );
            match result {
                Ok(found) => Ok(Some(found)),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(map_rusqlite(e)),
            }
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking find_user_any_state: {e}"),
        })?
    }

    // -----------------------------------------------------------------------
    // User CRUD
    // -----------------------------------------------------------------------

    /// Создать нового пользователя.
    ///
    /// Требует права `ManageUsers`. Пароль хэшируется через argon2id в `spawn_blocking`.
    pub async fn create_user(&self, new: UserNew, caller: &Identity) -> Result<UserDto, AppError> {
        authorize(caller, &Action::ManageUsers)?;
        Self::validate_user_new(&new)?;

        let now = self.clock.unix_seconds();
        let caller_id = caller.user_id;
        let password = Secret::new(new.password.clone());

        // CPU-bound hash — в spawn_blocking (T-05-03)
        let hash = tokio::task::spawn_blocking(move || hash_password(&password))
            .await
            .map_err(|e| AppError::Internal {
                source_chain: format!("spawn_blocking hash_password: {e}"),
            })??;

        let login = new.login.clone();
        let full_name = new.full_name.clone();
        let role = new.role.clone();
        let email = new.email.clone();

        let id = self
            .writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;

                // Soft-delete leaves the row (with its login) behind, but the schema
                // enforces an unconditional UNIQUE(login). Re-creating a login that
                // belongs to a soft-deleted user must REVIVE that row (reuse its id so
                // act/history foreign keys stay intact) rather than fail on UNIQUE.
                let existing: Option<(i64, Option<i64>)> = match tx.query_row(
                    "SELECT id, deleted_at_utc FROM users WHERE login = ?1",
                    rusqlite::params![login],
                    |r| Ok((r.get::<_, i64>(0)?, r.get::<_, Option<i64>>(1)?)),
                ) {
                    Ok(v) => Some(v),
                    Err(rusqlite::Error::QueryReturnedNoRows) => None,
                    Err(e) => return Err(map_rusqlite(e)),
                };

                let (id, action): (i64, &str) = match existing {
                    // Active row already owns this login → genuine conflict.
                    Some((_, None)) => {
                        return Err(AppError::Conflict {
                            reason: format!("Логин '{login}' уже занят"),
                        });
                    }
                    // Soft-deleted row → revive in place.
                    Some((existing_id, Some(_))) => {
                        tx.execute(
                            "UPDATE users SET \
                               full_name = ?1, password_hash = ?2, role = ?3, email = ?4, \
                               is_active = 1, deleted_at_utc = NULL, updated_at_utc = ?5, \
                               version = version + 1 \
                             WHERE id = ?6",
                            rusqlite::params![full_name, hash, role, email, now, existing_id],
                        )
                        .map_err(map_rusqlite)?;
                        (existing_id, "revive")
                    }
                    // No such login → fresh insert.
                    None => {
                        tx.execute(
                            "INSERT INTO users \
                             (login, full_name, password_hash, role, email, \
                              is_active, created_at_utc, updated_at_utc, version) \
                             VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6, ?6, 1)",
                            rusqlite::params![login, full_name, hash, role, email, now],
                        )
                        .map_err(map_rusqlite)?;
                        (tx.last_insert_rowid(), "create")
                    }
                };

                // Audit log
                tx.execute(
                    "INSERT INTO audit_log \
                     (entity_type, entity_id, action, user_id, before_json, after_json, payload_json, created_at_utc) \
                     VALUES ('user', ?1, ?2, ?3, NULL, NULL, ?4, ?5)",
                    rusqlite::params![id, action, caller_id, login, now],
                )
                .map_err(map_rusqlite)?;

                tx.commit().map_err(map_rusqlite)?;
                Ok(id)
            })
            .await?;

        self.get_user_by_id(id).await
    }

    /// Получить пользователя по ID.
    pub async fn get_user_by_id(&self, id: i64) -> Result<UserDto, AppError> {
        let readers = self.readers.clone();
        tokio::task::spawn_blocking(move || -> Result<UserDto, AppError> {
            let conn = readers.acquire();
            let result = conn.query_row(
                "SELECT id, version, login, full_name, role, email, is_active, \
                        created_at_utc, updated_at_utc \
                 FROM users \
                 WHERE id = ?1 AND deleted_at_utc IS NULL",
                rusqlite::params![id],
                row_to_user_dto,
            );
            match result {
                Ok(dto) => Ok(dto),
                Err(rusqlite::Error::QueryReturnedNoRows) => {
                    Err(AppError::NotFound { entity: "user", id })
                }
                Err(e) => Err(map_rusqlite(e)),
            }
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking get_user_by_id: {e}"),
        })?
    }

    /// Получить пользователя по логину.
    pub async fn get_by_login(&self, login: &str) -> Result<UserDto, AppError> {
        let readers = self.readers.clone();
        let login = login.to_string();
        tokio::task::spawn_blocking(move || -> Result<UserDto, AppError> {
            let conn = readers.acquire();
            let result = conn.query_row(
                "SELECT id, version, login, full_name, role, email, is_active, \
                        created_at_utc, updated_at_utc \
                 FROM users \
                 WHERE login = ?1 AND deleted_at_utc IS NULL",
                rusqlite::params![login],
                row_to_user_dto,
            );
            match result {
                Ok(dto) => Ok(dto),
                Err(rusqlite::Error::QueryReturnedNoRows) => Err(AppError::Unauthorized),
                Err(e) => Err(map_rusqlite(e)),
            }
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking get_by_login: {e}"),
        })?
    }

    /// Список пользователей с опциональным фильтром поиска и пагинацией.
    ///
    /// **Безопасность (CR-03):** управление пользователями — Admin only.
    /// Чтение списка пользователей раскрывает логины, роли, email — требует
    /// `ManageUsers`, иначе любой Employee мог бы перечислить все аккаунты.
    pub async fn list_users(
        &self,
        filter: UserFilter,
        pagination: Pagination,
        caller: &Identity,
    ) -> Result<UserListResponse, AppError> {
        authorize(caller, &Action::ManageUsers)?;
        let readers = self.readers.clone();
        tokio::task::spawn_blocking(move || -> Result<UserListResponse, AppError> {
            let conn = readers.acquire();

            let (items, total) = if let Some(ref search) = filter.search {
                let pattern = format!("%{}%", search);
                let total: i64 = conn
                    .query_row(
                        "SELECT COUNT(*) FROM users \
                         WHERE deleted_at_utc IS NULL \
                           AND (login LIKE ?1 OR full_name LIKE ?1)",
                        rusqlite::params![pattern],
                        |r| r.get(0),
                    )
                    .map_err(map_rusqlite)?;

                let mut stmt = conn
                    .prepare(
                        "SELECT id, version, login, full_name, role, email, is_active, \
                                created_at_utc, updated_at_utc \
                         FROM users \
                         WHERE deleted_at_utc IS NULL \
                           AND (login LIKE ?1 OR full_name LIKE ?1) \
                         ORDER BY created_at_utc DESC \
                         LIMIT ?2 OFFSET ?3",
                    )
                    .map_err(map_rusqlite)?;

                let limit = pagination.limit as i64;
                let offset = pagination.offset as i64;
                let rows: Vec<UserDto> = stmt
                    .query_map(rusqlite::params![pattern, limit, offset], row_to_user_dto)
                    .map_err(map_rusqlite)?
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(map_rusqlite)?;

                (rows, total)
            } else {
                let total: i64 = conn
                    .query_row(
                        "SELECT COUNT(*) FROM users WHERE deleted_at_utc IS NULL",
                        [],
                        |r| r.get(0),
                    )
                    .map_err(map_rusqlite)?;

                let mut stmt = conn
                    .prepare(
                        "SELECT id, version, login, full_name, role, email, is_active, \
                                created_at_utc, updated_at_utc \
                         FROM users \
                         WHERE deleted_at_utc IS NULL \
                         ORDER BY created_at_utc DESC \
                         LIMIT ?1 OFFSET ?2",
                    )
                    .map_err(map_rusqlite)?;

                let limit = pagination.limit as i64;
                let offset = pagination.offset as i64;
                let rows: Vec<UserDto> = stmt
                    .query_map(rusqlite::params![limit, offset], row_to_user_dto)
                    .map_err(map_rusqlite)?
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(map_rusqlite)?;

                (rows, total)
            };

            Ok(UserListResponse { items, total })
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking list_users: {e}"),
        })?
    }

    /// Обновить пользователя с optimistic-lock.
    pub async fn update_user(
        &self,
        id: i64,
        version: i64,
        patch: UserPatch,
        caller: &Identity,
    ) -> Result<UserDto, AppError> {
        authorize(caller, &Action::ManageUsers)?;

        if let Some(ref role_str) = patch.role {
            // Validate role string
            Role::from_str(role_str)?;
        }

        // WR-01: optional password change on edit. Empty string / None → no
        // change; a non-empty new password is validated (len >= 8, same rule
        // and message as create) and hashed via argon2id off the writer thread
        // (CPU-bound → spawn_blocking, mirroring `create_user`).
        let new_password_hash: Option<String> = match patch.password.as_deref() {
            Some(pw) if !pw.is_empty() => {
                if pw.len() < 8 {
                    return Err(AppError::Validation {
                        field: "password".to_string(),
                        message: "Пароль должен быть не менее 8 символов".to_string(),
                    });
                }
                let password = Secret::new(pw.to_string());
                let hash = tokio::task::spawn_blocking(move || hash_password(&password))
                    .await
                    .map_err(|e| AppError::Internal {
                        source_chain: format!("spawn_blocking hash update_user: {e}"),
                    })??;
                Some(hash)
            }
            _ => None,
        };

        let now = self.clock.unix_seconds();
        let caller_id = caller.user_id;

        self.writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;

                // Check version (optimistic lock)
                let current_version: i64 = tx
                    .query_row(
                        "SELECT version FROM users WHERE id = ?1 AND deleted_at_utc IS NULL",
                        rusqlite::params![id],
                        |r| r.get(0),
                    )
                    .map_err(|e| {
                        if e == rusqlite::Error::QueryReturnedNoRows {
                            AppError::NotFound {
                                entity: "user",
                                id,
                            }
                        } else {
                            map_rusqlite(e)
                        }
                    })?;

                if current_version != version {
                    return Err(AppError::Conflict {
                        reason: format!(
                            "optimistic lock: version {version} != current {current_version}"
                        ),
                    });
                }

                // CR-04: prevent demoting / deactivating the LAST active admin —
                // doing so would permanently lock administration out of server mode.
                let downgrades_role = matches!(
                    patch.role.as_deref(),
                    Some("employee") | Some("manager")
                );
                let deactivates = patch.is_active == Some(false);
                if downgrades_role || deactivates {
                    let active_admins: i64 = tx
                        .query_row(
                            "SELECT COUNT(*) FROM users \
                             WHERE role = 'admin' AND is_active = 1 AND deleted_at_utc IS NULL",
                            [],
                            |r| r.get(0),
                        )
                        .map_err(map_rusqlite)?;
                    let is_target_active_admin: i64 = tx
                        .query_row(
                            "SELECT CASE WHEN role = 'admin' AND is_active = 1 THEN 1 ELSE 0 END \
                             FROM users WHERE id = ?1 AND deleted_at_utc IS NULL",
                            rusqlite::params![id],
                            |r| r.get(0),
                        )
                        .map_err(map_rusqlite)?;
                    if active_admins <= 1 && is_target_active_admin == 1 {
                        return Err(AppError::Conflict {
                            reason: "нельзя понизить или деактивировать последнего администратора"
                                .to_string(),
                        });
                    }
                }

                // WR-02: explicit UPDATE — COALESCE leaves unset columns untouched;
                // the email CASE handles Some(None) (clear to NULL) vs None (keep).
                // WR-01: password_hash COALESCE — Some(new hash) rotates the
                // password, None leaves it untouched (empty/None input above).
                let rows_changed = tx
                    .execute(
                        "UPDATE users SET \
                         updated_at_utc = ?1, \
                         version = version + 1, \
                         full_name = COALESCE(?2, full_name), \
                         role = COALESCE(?3, role), \
                         email = CASE WHEN ?4 = 1 THEN ?5 ELSE email END, \
                         is_active = COALESCE(?6, is_active), \
                         password_hash = COALESCE(?7, password_hash) \
                         WHERE id = ?8 AND version = ?9 AND deleted_at_utc IS NULL",
                        rusqlite::params![
                            now,
                            patch.full_name,
                            patch.role,
                            patch.email.is_some() as i64,
                            patch.email.flatten(),
                            patch.is_active.map(|b| b as i64),
                            new_password_hash,
                            id,
                            version
                        ],
                    )
                    .map_err(map_rusqlite)?;

                if rows_changed == 0 {
                    return Err(AppError::Conflict {
                        reason: "optimistic lock: version mismatch".to_string(),
                    });
                }

                // Audit log
                tx.execute(
                    "INSERT INTO audit_log \
                     (entity_type, entity_id, action, user_id, before_json, after_json, payload_json, created_at_utc) \
                     VALUES ('user', ?1, 'update', ?2, NULL, NULL, NULL, ?3)",
                    rusqlite::params![id, caller_id, now],
                )
                .map_err(map_rusqlite)?;

                tx.commit().map_err(map_rusqlite)?;
                Ok(())
            })
            .await?;

        self.get_user_by_id(id).await
    }

    /// Мягкое удаление пользователя (soft-delete) с optimistic-lock.
    pub async fn delete_user(
        &self,
        id: i64,
        version: i64,
        caller: &Identity,
    ) -> Result<(), AppError> {
        authorize(caller, &Action::ManageUsers)?;

        let now = self.clock.unix_seconds();
        let caller_id = caller.user_id;

        self.writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;

                // CR-04: refuse to soft-delete the LAST active admin.
                let active_admins: i64 = tx
                    .query_row(
                        "SELECT COUNT(*) FROM users \
                         WHERE role = 'admin' AND is_active = 1 AND deleted_at_utc IS NULL",
                        [],
                        |r| r.get(0),
                    )
                    .map_err(map_rusqlite)?;
                let is_target_active_admin: i64 = tx
                    .query_row(
                        "SELECT CASE WHEN role = 'admin' AND is_active = 1 THEN 1 ELSE 0 END \
                         FROM users WHERE id = ?1 AND deleted_at_utc IS NULL",
                        rusqlite::params![id],
                        |r| r.get(0),
                    )
                    .optional()
                    .map_err(map_rusqlite)?
                    .unwrap_or(0);
                if active_admins <= 1 && is_target_active_admin == 1 {
                    return Err(AppError::Conflict {
                        reason: "нельзя удалить последнего администратора".to_string(),
                    });
                }

                let rows_changed = tx
                    .execute(
                        "UPDATE users SET deleted_at_utc = ?1, version = version + 1 \
                         WHERE id = ?2 AND version = ?3 AND deleted_at_utc IS NULL",
                        rusqlite::params![now, id, version],
                    )
                    .map_err(map_rusqlite)?;

                if rows_changed == 0 {
                    return Err(AppError::Conflict {
                        reason: "optimistic lock: version mismatch or user not found".to_string(),
                    });
                }

                // Audit log
                tx.execute(
                    "INSERT INTO audit_log \
                     (entity_type, entity_id, action, user_id, before_json, after_json, payload_json, created_at_utc) \
                     VALUES ('user', ?1, 'delete', ?2, NULL, NULL, NULL, ?3)",
                    rusqlite::params![id, caller_id, now],
                )
                .map_err(map_rusqlite)?;

                tx.commit().map_err(map_rusqlite)?;
                Ok(())
            })
            .await
    }

    /// Сменить пароль пользователя (пользователь меняет себе).
    pub async fn change_password(
        &self,
        user_id: i64,
        req: ChangePasswordRequest,
    ) -> Result<(), AppError> {
        // Validate new password length
        if req.new_password.len() < 8 {
            return Err(AppError::Validation {
                field: "new_password".to_string(),
                message: "Пароль должен быть не менее 8 символов".to_string(),
            });
        }

        // Load current hash
        let readers = self.readers.clone();
        let uid = user_id;
        let current_hash = tokio::task::spawn_blocking(move || -> Result<String, AppError> {
            let conn = readers.acquire();
            let result: rusqlite::Result<String> = conn.query_row(
                "SELECT password_hash FROM users WHERE id = ?1 AND deleted_at_utc IS NULL",
                rusqlite::params![uid],
                |r| r.get(0),
            );
            match result {
                Ok(h) => Ok(h),
                Err(rusqlite::Error::QueryReturnedNoRows) => Err(AppError::NotFound {
                    entity: "user",
                    id: uid,
                }),
                Err(e) => Err(map_rusqlite(e)),
            }
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking change_password load: {e}"),
        })??;

        // Verify old password (T-05-03)
        let old_password = Secret::new(req.old_password.clone());
        let hash_clone = current_hash.clone();
        let verified =
            tokio::task::spawn_blocking(move || verify_password(&old_password, &hash_clone))
                .await
                .map_err(|e| AppError::Internal {
                    source_chain: format!("spawn_blocking verify old: {e}"),
                })?;

        if !verified {
            return Err(AppError::Unauthorized);
        }

        // Hash new password (T-05-03)
        let new_password = Secret::new(req.new_password.clone());
        let new_hash = tokio::task::spawn_blocking(move || hash_password(&new_password))
            .await
            .map_err(|e| AppError::Internal {
                source_chain: format!("spawn_blocking hash new: {e}"),
            })??;

        let now = self.clock.unix_seconds();
        self.writer
            .execute(move |conn| {
                conn.execute(
                    "UPDATE users SET password_hash = ?1, updated_at_utc = ?2, version = version + 1 \
                     WHERE id = ?3 AND deleted_at_utc IS NULL",
                    rusqlite::params![new_hash, now, user_id],
                )
                .map(|_| ())
                .map_err(map_rusqlite)
            })
            .await
    }

    /// Сбросить пароль пользователя (admin-операция).
    pub async fn reset_password(
        &self,
        user_id: i64,
        new_password: String,
        caller: &Identity,
    ) -> Result<(), AppError> {
        authorize(caller, &Action::ManageUsers)?;

        if new_password.len() < 8 {
            return Err(AppError::Validation {
                field: "new_password".to_string(),
                message: "Пароль должен быть не менее 8 символов".to_string(),
            });
        }

        let password = Secret::new(new_password);
        let new_hash = tokio::task::spawn_blocking(move || hash_password(&password))
            .await
            .map_err(|e| AppError::Internal {
                source_chain: format!("spawn_blocking reset hash: {e}"),
            })??;

        let now = self.clock.unix_seconds();
        self.writer
            .execute(move |conn| {
                conn.execute(
                    "UPDATE users SET password_hash = ?1, updated_at_utc = ?2, version = version + 1 \
                     WHERE id = ?3 AND deleted_at_utc IS NULL",
                    rusqlite::params![new_hash, now, user_id],
                )
                .map(|_| ())
                .map_err(map_rusqlite)
            })
            .await
    }

    // -----------------------------------------------------------------------
    // Desktop attribution (D-Desktop-01)
    // -----------------------------------------------------------------------

    /// Возвращает идентификатор для десктоп-режима без входа.
    ///
    /// **D-Desktop-01:** если в БД ровно один активный admin — атрибутирует
    /// его (`user_id = Some(id)`). При 0 или 2+ admin'ах — `trusted_admin()`
    /// (user_id = None). Используется LIMIT 2 для эффективности.
    pub async fn desktop_identity(&self) -> Identity {
        let readers = self.readers.clone();
        let result = tokio::task::spawn_blocking(move || -> Result<Vec<i64>, AppError> {
            let conn = readers.acquire();
            let mut stmt = conn
                .prepare(
                    "SELECT id FROM users \
                     WHERE role = 'admin' AND deleted_at_utc IS NULL AND is_active = 1 \
                     LIMIT 2",
                )
                .map_err(map_rusqlite)?;
            let ids: Vec<i64> = stmt
                .query_map([], |r| r.get(0))
                .map_err(map_rusqlite)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(map_rusqlite)?;
            Ok(ids)
        })
        .await;

        match result {
            Ok(Ok(ids)) if ids.len() == 1 => Identity {
                user_id: Some(ids[0]),
                role: Role::Admin,
            },
            _ => Identity::trusted_admin(),
        }
    }

    // -----------------------------------------------------------------------
    // Desktop lock (D-Desktop-02)
    // -----------------------------------------------------------------------

    /// Читает флаг `desktop_lock_enabled` из таблицы `app_settings`.
    ///
    /// '1' → true, любое другое значение или отсутствие записи → false.
    pub async fn get_desktop_lock_enabled(&self) -> Result<bool, AppError> {
        let readers = self.readers.clone();
        tokio::task::spawn_blocking(move || -> Result<bool, AppError> {
            let conn = readers.acquire();
            let result: rusqlite::Result<String> = conn.query_row(
                "SELECT value FROM app_settings WHERE key = 'desktop_lock_enabled'",
                [],
                |r| r.get(0),
            );
            match result {
                Ok(v) => Ok(v == "1"),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(false),
                Err(e) => Err(map_rusqlite(e)),
            }
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking get_desktop_lock_enabled: {e}"),
        })?
    }

    /// Устанавливает флаг `desktop_lock_enabled` в таблице `app_settings`.
    ///
    /// Требует права `ManageSettings` (D-Desktop-02).
    pub async fn set_desktop_lock_enabled(
        &self,
        enabled: bool,
        caller: &Identity,
    ) -> Result<(), AppError> {
        authorize(caller, &Action::ManageSettings)?;

        let value = if enabled { "1" } else { "0" };
        let now = self.clock.unix_seconds();

        self.writer
            .execute(move |conn| {
                // WR-03: upsert — a security toggle must not silently no-op
                // (fail open) if the settings row is missing.
                conn.execute(
                    "INSERT INTO app_settings (key, value, created_at_utc, updated_at_utc) \
                     VALUES ('desktop_lock_enabled', ?1, ?2, ?2) \
                     ON CONFLICT(key) DO UPDATE SET value = ?1, updated_at_utc = ?2",
                    rusqlite::params![value, now],
                )
                .map(|_| ())
                .map_err(map_rusqlite)
            })
            .await
    }

    // -----------------------------------------------------------------------
    // Validation helpers
    // -----------------------------------------------------------------------

    fn validate_user_new(new: &UserNew) -> Result<(), AppError> {
        if new.login.len() < 3 {
            return Err(AppError::Validation {
                field: "login".to_string(),
                message: "Логин должен быть не менее 3 символов".to_string(),
            });
        }
        if new.password.len() < 8 {
            return Err(AppError::Validation {
                field: "password".to_string(),
                message: "Пароль должен быть не менее 8 символов".to_string(),
            });
        }
        // Validate role
        Role::from_str(&new.role)?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Row mapper
// ---------------------------------------------------------------------------

fn row_to_user_dto(row: &rusqlite::Row<'_>) -> rusqlite::Result<UserDto> {
    let is_active_i64: i64 = row.get(6)?;
    Ok(UserDto {
        id: row.get(0)?,
        version: row.get(1)?,
        login: row.get(2)?,
        full_name: row.get(3)?,
        role: row.get(4)?,
        email: row.get(5)?,
        is_active: is_active_i64 != 0,
        created_at_utc: row.get(7)?,
        updated_at_utc: row.get(8)?,
    })
}
