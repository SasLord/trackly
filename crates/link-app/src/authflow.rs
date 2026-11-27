//! Потоки входа Link: автовход, запоминание, выход, отзыв (D-09..D-14, D-25).
//!
//! Жёсткие правила модуля:
//! - пароль уходит на сервер ТОЛЬКО из явного вызова [`AuthFlow::password_login`]; ни
//!   `startup_auth`, ни `retry_sso` его не вызывают ни при каком исходе (T-44-44);
//! - `revoked` блокирует автовход на старте (T-44-46);
//! - сетевая ошибка и 5xx при возобновлении входа — это «нет связи», а не «нужен вход» (D-25);
//! - в логи попадают только факты, без паролей, токенов и заголовков.

use link_proto::dto::{
    LinkAuthResponse, LinkLoginRequest, LinkMethod, ResumeRequest, LINK_BAD_CREDENTIALS,
    LINK_DEVICE_BANNED, LINK_RATE_LIMITED, LINK_REVOKED, LINK_USER_BLOCKED,
};
use reqwest::header::AUTHORIZATION;
use reqwest::Method;
use zeroize::Zeroize;

use crate::config::{LinkConfig, LocalState};
use crate::http_client::{HttpError, LinkHttp};
use crate::secrets::{login_token_key, Secret, TokenStore};
use crate::sso::{can_retry, classify, plan_sso, HttpOutcome, SsoPlan, SsoProvider};
use crate::state::{CmdErrorCode, LinkCmdError, LinkUserView, LoginMethod, LoginReason};

const PATH_LOGIN: &str = "/api/v1/link_auth_login";
const PATH_SSO: &str = "/api/v1/link_auth_sso";
const PATH_RESUME: &str = "/api/v1/link_resume";
const PATH_LOGOUT: &str = "/api/v1/link_logout";

/// Платформа выполнения: автовход (SSO) существует только на Windows (D-09, D-15).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Windows,
    Other,
}

impl Platform {
    pub fn current() -> Self {
        if cfg!(windows) {
            Platform::Windows
        } else {
            Platform::Other
        }
    }
}

/// Итог потока входа. UI сопоставляет его с `LinkState`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthResult {
    Connected {
        user: LinkUserView,
        method: LoginMethod,
    },
    NeedsLogin {
        reason: Option<LoginReason>,
        can_retry_sso: bool,
        after_logout: bool,
    },
    Revoked,
    Offline,
    PinMismatch {
        expected: String,
        got: String,
    },
}

pub struct AuthFlow<'a> {
    pub http: &'a LinkHttp,
    pub sso: &'a dyn SsoProvider,
    pub tokens: &'a TokenStore,
    pub local: &'a mut LocalState,
    pub cfg: &'a LinkConfig,
    pub platform: Platform,
}

fn view(resp: &LinkAuthResponse) -> LinkUserView {
    LinkUserView {
        display_name: resp.user.display_name.clone(),
        login: resp.user.login.clone(),
    }
}

fn method_of(m: LinkMethod) -> LoginMethod {
    match m {
        LinkMethod::Sso => LoginMethod::Sso,
        LinkMethod::Password => LoginMethod::Password,
    }
}

fn is_revoking(code: Option<&str>) -> bool {
    matches!(code, Some(LINK_REVOKED) | Some(LINK_DEVICE_BANNED))
}

impl AuthFlow<'_> {
    fn token_key(&self) -> String {
        login_token_key(&self.cfg.host, self.cfg.port)
    }

    /// Платформа поддерживает SSO и адрес позволяет его пробовать.
    fn sso_possible(&self) -> bool {
        self.platform == Platform::Windows
            && matches!(
                plan_sso(&self.cfg.host, self.cfg.spn.as_deref()),
                SsoPlan::Try { .. }
            )
    }

    fn needs_login(&self, reason: Option<LoginReason>, after_logout: bool) -> AuthResult {
        let can_retry_sso = match reason {
            Some(r) => can_retry(r) && self.platform == Platform::Windows,
            None => self.sso_possible(),
        };
        AuthResult::NeedsLogin {
            reason,
            can_retry_sso,
            after_logout,
        }
    }

    fn store_token(&self, resp: &LinkAuthResponse) {
        if let Some(t) = &resp.device_token {
            self.tokens.set(&self.token_key(), &Secret::new(t.clone()));
        }
    }

    /// D-14: сервер отозвал вход или запретил устройство. Токен стирается, флаг сохраняется.
    fn mark_revoked(&mut self) -> AuthResult {
        self.tokens.delete(&self.token_key());
        if self.local.set_revoked(true).is_err() {
            tracing::warn!("не удалось сохранить признак отзыва");
        }
        tracing::info!("вход отозван сервером");
        AuthResult::Revoked
    }

    /// Разбор кода ошибки API для других потоков (например, переподключения): отзыв/запрет
    /// стирают токен и дают состояние `Revoked`; прочие коды состояния не меняют.
    pub fn on_api_error(&mut self, code: &str) -> Option<AuthResult> {
        is_revoking(Some(code)).then(|| self.mark_revoked())
    }

    /// Запуск: revoked -> токен (resume) -> автовход (только Windows, не после выхода) -> форма.
    pub async fn startup_auth(&mut self) -> AuthResult {
        if self.local.revoked {
            return AuthResult::Revoked;
        }
        if let Some(token) = self.tokens.get(&self.token_key()) {
            match self.resume(&token).await {
                Resume::Done(r) => return r,
                Resume::TokenInvalid => {
                    self.tokens.delete(&self.token_key());
                }
            }
        }
        if self.local.logged_out {
            return self.needs_login(None, true);
        }
        if self.platform != Platform::Windows {
            return self.needs_login(None, false);
        }
        self.try_sso().await
    }

    async fn resume(&mut self, token: &Secret) -> Resume {
        let body = ResumeRequest {
            device_token: token.expose().to_string(),
        };
        match self
            .http
            .post_json::<_, LinkAuthResponse>(PATH_RESUME, &body)
            .await
        {
            Ok(resp) => {
                self.store_token(&resp);
                tracing::info!("вход возобновлён по токену");
                Resume::Done(AuthResult::Connected {
                    user: view(&resp),
                    method: method_of(resp.method),
                })
            }
            Err(HttpError::PinMismatch { expected, got }) => {
                Resume::Done(AuthResult::PinMismatch { expected, got })
            }
            // D-25: нет связи — не «нужен вход», токен остаётся.
            Err(HttpError::Network) | Err(HttpError::Server(_)) | Err(HttpError::Decode) => {
                Resume::Done(AuthResult::Offline)
            }
            Err(HttpError::Api { code, .. }) => {
                if is_revoking(code.as_deref()) {
                    Resume::Done(self.mark_revoked())
                } else {
                    Resume::TokenInvalid
                }
            }
        }
    }

    /// Явное «Войти автоматически» (D-11): снимает признак выхода и пробует SSO.
    pub async fn retry_sso(&mut self) -> AuthResult {
        if self.local.revoked {
            return AuthResult::Revoked;
        }
        if self.platform != Platform::Windows {
            return self.needs_login(None, false);
        }
        if self.local.set_logged_out(false).is_err() {
            tracing::warn!("не удалось сохранить признак выхода");
        }
        self.try_sso().await
    }

    async fn try_sso(&mut self) -> AuthResult {
        let spn = match plan_sso(&self.cfg.host, self.cfg.spn.as_deref()) {
            SsoPlan::Skip(reason) => return self.needs_login(Some(reason), false),
            SsoPlan::Try { spn } => spn,
        };
        let fail = |this: &Self, o: HttpOutcome| this.needs_login(Some(classify(&o)), false);
        let token = match self.sso.token(&spn) {
            Ok(t) => t,
            Err(e) => return fail(self, HttpOutcome::Provider(e)),
        };
        let resp = self
            .http
            .get_with_header(PATH_SSO, AUTHORIZATION, token.header_value())
            .await;
        let resp = match resp {
            Ok(r) => r,
            Err(HttpError::PinMismatch { expected, got }) => {
                return AuthResult::PinMismatch { expected, got }
            }
            Err(HttpError::Network) | Err(HttpError::Decode) => {
                return fail(self, HttpOutcome::Network)
            }
            Err(HttpError::Server(503)) => return fail(self, HttpOutcome::Status503),
            Err(HttpError::Server(_)) => return fail(self, HttpOutcome::Network),
            Err(HttpError::Api { code, .. }) => {
                if is_revoking(code.as_deref()) {
                    return self.mark_revoked();
                }
                return fail(self, HttpOutcome::Unauthorized { code });
            }
        };
        match resp.json::<LinkAuthResponse>().await {
            Ok(auth) => {
                self.store_token(&auth);
                tracing::info!("автовход выполнен");
                AuthResult::Connected {
                    user: view(&auth),
                    method: method_of(auth.method),
                }
            }
            Err(_) => fail(self, HttpOutcome::Network),
        }
    }

    /// Вход паролем — только по явной команде пользователя. Пароль не хранится (D-10);
    /// запоминается лишь `device_token`, который выдаёт сервер.
    pub async fn password_login(
        &mut self,
        login: &str,
        password: Secret,
    ) -> Result<AuthResult, LinkCmdError> {
        let mut req = LinkLoginRequest {
            login: login.to_string(),
            password: password.expose().to_string(),
        };
        let res = self
            .http
            .post_json::<_, LinkAuthResponse>(PATH_LOGIN, &req)
            .await;
        req.password.zeroize();
        drop(password);
        match res {
            Ok(auth) => {
                self.store_token(&auth);
                if self.local.revoked && self.local.set_revoked(false).is_err() {
                    tracing::warn!("не удалось снять признак отзыва");
                }
                tracing::info!("вход паролем выполнен");
                Ok(AuthResult::Connected {
                    user: view(&auth),
                    method: method_of(auth.method),
                })
            }
            Err(HttpError::PinMismatch { .. }) => Err(LinkCmdError::new(CmdErrorCode::PinMismatch)),
            Err(HttpError::Network) => Err(LinkCmdError::new(CmdErrorCode::NoNetwork)),
            Err(HttpError::Server(_)) => Err(LinkCmdError::new(CmdErrorCode::AuthUnavailable)),
            Err(HttpError::Decode) => Err(LinkCmdError::new(CmdErrorCode::Internal)),
            Err(HttpError::Api { status, code, .. }) => {
                if is_revoking(code.as_deref()) {
                    self.mark_revoked();
                    return Err(LinkCmdError::new(CmdErrorCode::DeviceBanned));
                }
                Err(LinkCmdError::new(password_error_code(
                    status,
                    code.as_deref(),
                )))
            }
        }
    }

    /// D-11: серверный выход best-effort, локальный выполняется всегда.
    pub async fn logout(&mut self) -> AuthResult {
        if self
            .http
            .request(Method::POST, PATH_LOGOUT, None, &[])
            .await
            .is_err()
        {
            tracing::info!("сервер не подтвердил выход, выход выполнен локально");
        }
        self.tokens.delete(&self.token_key());
        if self.platform == Platform::Windows && self.local.set_logged_out(true).is_err() {
            tracing::warn!("не удалось сохранить признак выхода");
        }
        self.needs_login(None, true)
    }
}

enum Resume {
    Done(AuthResult),
    TokenInvalid,
}

fn password_error_code(status: u16, code: Option<&str>) -> CmdErrorCode {
    if status == 429 || code == Some(LINK_RATE_LIMITED) {
        return CmdErrorCode::RateLimited;
    }
    match code {
        Some(LINK_BAD_CREDENTIALS) => CmdErrorCode::BadCredentials,
        Some(LINK_USER_BLOCKED) => CmdErrorCode::UserBlocked,
        _ if status == 401 => CmdErrorCode::BadCredentials,
        _ => CmdErrorCode::Internal,
    }
}
