//! Команды webview Link: единственный путь UI к сети и секретам (ADR 0002).
//!
//! Каждая команда — тонкий адаптер над `Runtime`. Состав команд зафиксирован
//! `tests/fixtures/link_commands.json` и константой [`COMMAND_NAMES`].

use link_proto::InviteCode;
use tauri::State;

use crate::authflow::{AuthFlow, AuthResult, Platform};
use crate::connection::Command;
use crate::runtime::Runtime;
use crate::secrets::Secret;
use crate::state::{CmdErrorCode, LinkCmdError, LinkEvent};

/// Все команды, зарегистрированные в `generate_handler!` (порядок не важен).
/// Контрактный тест сверяет список с фикстурой и с исходником `main.rs`.
pub const COMMAND_NAMES: &[&str] = &[
    "link_info",
    "link_get_state",
    "link_submit_invite",
    "link_login",
    "link_retry_sso",
    "link_logout",
    "link_recheck_pin",
    "link_retry_now",
    "link_quit",
];

/// Проверка кода до любого обращения к сети.
pub fn precheck_invite(code: &str) -> Result<(), LinkCmdError> {
    if code.trim().is_empty() {
        return Err(LinkCmdError::new(CmdErrorCode::EmptyCode));
    }
    InviteCode::decode(code)
        .map(|_| ())
        .map_err(|_| LinkCmdError::new(CmdErrorCode::InvalidCode))
}

/// Данные формы входа. Пароль живёт только в `Secret` (Debug скрывает значение, zeroize).
pub struct LoginInput {
    pub login: String,
    pub password: Secret,
}

impl std::fmt::Debug for LoginInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoginInput")
            .field("login", &self.login)
            .field("password", &self.password)
            .finish()
    }
}

impl LoginInput {
    pub fn new(login: String, password: String) -> Self {
        Self {
            login,
            password: Secret::new(password),
        }
    }

    /// Пустые поля — отказ без сетевого вызова.
    pub fn precheck(&self) -> Result<(), LinkCmdError> {
        if self.login.trim().is_empty() || self.password.expose().is_empty() {
            return Err(LinkCmdError::new(CmdErrorCode::BadCredentials));
        }
        Ok(())
    }
}

#[tauri::command]
pub async fn link_get_state(rt: State<'_, Runtime>) -> Result<LinkEvent, LinkCmdError> {
    Ok(rt.sink.current())
}

#[tauri::command]
pub async fn link_submit_invite(rt: State<'_, Runtime>, code: String) -> Result<(), LinkCmdError> {
    precheck_invite(&code)?;
    rt.enroll(&code).await
}

#[tauri::command]
pub async fn link_login(
    rt: State<'_, Runtime>,
    login: String,
    password: String,
) -> Result<(), LinkCmdError> {
    let input = LoginInput::new(login, password);
    input.precheck()?;
    let session = rt
        .session()
        .await
        .ok_or_else(|| LinkCmdError::new(CmdErrorCode::Internal))?;
    let res = {
        let mut local = rt.local.lock().await;
        let mut flow = AuthFlow {
            http: &session.http,
            sso: &*rt.sso,
            tokens: &rt.tokens,
            local: &mut local,
            cfg: &session.cfg,
            platform: Platform::current(),
        };
        flow.password_login(&input.login, input.password).await
    };
    match res {
        Ok(result) => {
            session.handle.command(Command::External(result)).await;
            Ok(())
        }
        Err(e) => {
            // Запрет устройства уже отмечен в `AuthFlow` (токен стёрт): автомату нужно перейти в Revoked.
            if e.code == CmdErrorCode::DeviceBanned {
                session
                    .handle
                    .command(Command::External(AuthResult::Revoked))
                    .await;
            }
            Err(e)
        }
    }
}

#[tauri::command]
pub async fn link_retry_sso(rt: State<'_, Runtime>) -> Result<(), LinkCmdError> {
    let session = rt
        .session()
        .await
        .ok_or_else(|| LinkCmdError::new(CmdErrorCode::Internal))?;
    let result = {
        let mut local = rt.local.lock().await;
        let mut flow = AuthFlow {
            http: &session.http,
            sso: &*rt.sso,
            tokens: &rt.tokens,
            local: &mut local,
            cfg: &session.cfg,
            platform: Platform::current(),
        };
        flow.retry_sso().await
    };
    session.handle.command(Command::External(result)).await;
    Ok(())
}

#[tauri::command]
pub async fn link_logout(rt: State<'_, Runtime>) -> Result<(), LinkCmdError> {
    rt.command(Command::Logout).await
}

#[tauri::command]
pub async fn link_recheck_pin(rt: State<'_, Runtime>) -> Result<(), LinkCmdError> {
    rt.command(Command::RecheckPin).await
}

#[tauri::command]
pub async fn link_retry_now(rt: State<'_, Runtime>) -> Result<(), LinkCmdError> {
    rt.command(Command::RetryNow).await
}

#[tauri::command]
pub async fn link_quit(rt: State<'_, Runtime>) -> Result<(), LinkCmdError> {
    rt.quit();
    Ok(())
}
