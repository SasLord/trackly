//! Контракт состояний и ошибок команд Link для UI и ядра (UI-SPEC §6.1, §6.3, §6.5).
//!
//! Webview не делает сетевых запросов: ядро шлёт `LinkEvent` событием [`LINK_EVENT`],
//! UI рисует экран по `state.kind`. Форма сериализации зафиксирована golden-тестом
//! `tests/state_contract.rs` и фикстурами в `tests/fixtures/`.

use serde::{Deserialize, Serialize};

/// Имя события Tauri с текущим состоянием.
pub const LINK_EVENT: &str = "link-event";

/// Причина неудачного автовхода (D-09), строка 2 callout'а UI-SPEC §6.5.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoginReason {
    /// Адрес сервера задан IP: автовход работает только по имени.
    IpAddress,
    /// Автовход выключен на сервере.
    SsoDisabled,
    /// Нет SPN или ошибка Kerberos.
    Kerberos,
    /// Доменная учётная запись не сопоставлена пользователю Trackly.
    NotMapped,
    /// Сервер не ответил на автовход.
    NoAnswer,
}

/// Способ, которым выполнен вход.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoginMethod {
    Sso,
    Password,
}

/// Учётная запись для шапки подключённого окна (UI-SPEC §6.7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkUserView {
    pub display_name: String,
    pub login: String,
}

/// Конечный автомат экранов окна Link (UI-SPEC §6.1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LinkState {
    /// Читаем конфиг, проверяем отпечаток, пробуем SSO.
    Starting,
    /// Конфига нет (D-01): экран первой настройки.
    NeedsSetup,
    /// Отпечаток сервера не совпал с закреплённым.
    PinMismatch {
        server: String,
        expected: String,
        got: String,
    },
    /// Токена нет либо сервер сказал «недействителен» (`after_logout` — вариант «после выхода»).
    NeedsLogin {
        reason: Option<LoginReason>,
        can_retry_sso: bool,
        after_logout: bool,
        server: String,
    },
    /// Сервер отозвал вход или запретил устройство (D-14).
    Revoked { server: String },
    Connected {
        user: LinkUserView,
        method: LoginMethod,
        server: String,
    },
    /// Вход был, связи нет (D-23, D-25).
    Offline { server: String },
}

/// Полезная нагрузка события `link-event`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkEvent {
    pub state: LinkState,
    /// Есть ли системный трей (на Linux без него UI показывает §6.9).
    pub tray_available: bool,
    /// Хранит ли ОС токен входа (иначе «до закрытия Link», §6.5 п.5).
    pub secrets_persistent: bool,
    pub platform: String,
}

/// Код ошибки команды (таблицы UI-SPEC §6.3 и §6.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CmdErrorCode {
    EmptyCode,
    InvalidCode,
    ExpiredCode,
    UsedCode,
    ServerUnreachable,
    PinMismatch,
    BadCredentials,
    UserBlocked,
    DeviceBanned,
    RateLimited,
    NoNetwork,
    AuthUnavailable,
    Internal,
}

/// Ошибка команды: код для логики UI и готовый русский текст.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkCmdError {
    pub code: CmdErrorCode,
    pub message: String,
}

impl LinkCmdError {
    /// Ошибка с типовым текстом из UI-SPEC. `server` нужен только для `ServerUnreachable`.
    pub fn new(code: CmdErrorCode) -> Self {
        Self::with_server(code, "")
    }

    pub fn with_server(code: CmdErrorCode, server: &str) -> Self {
        let message = match code {
            CmdErrorCode::EmptyCode => "Введите код приглашения".to_string(),
            CmdErrorCode::InvalidCode => {
                "Код не распознан. Проверьте, что он скопирован целиком.".to_string()
            }
            CmdErrorCode::ExpiredCode => {
                "Срок действия кода истёк. Попросите у администратора новый.".to_string()
            }
            CmdErrorCode::UsedCode => {
                "Этот код уже использован или отозван. Попросите у администратора новый."
                    .to_string()
            }
            CmdErrorCode::ServerUnreachable => format!(
                "Не удалось связаться с сервером {server}. Проверьте сеть и нажмите «Подключить» ещё раз."
            ),
            CmdErrorCode::PinMismatch => {
                "Сервер не прошёл проверку подлинности. Ничего не сохранено. Обратитесь к администратору."
                    .to_string()
            }
            CmdErrorCode::BadCredentials => "Неверный логин или пароль".to_string(),
            CmdErrorCode::UserBlocked => {
                "Учётная запись заблокирована. Обратитесь к администратору.".to_string()
            }
            CmdErrorCode::DeviceBanned => {
                "Вход с этого компьютера запрещён администратором. Обратитесь к администратору."
                    .to_string()
            }
            CmdErrorCode::RateLimited => {
                "Слишком много попыток входа. Повторите через минуту.".to_string()
            }
            CmdErrorCode::NoNetwork => {
                "Нет связи с сервером. Проверьте сеть и повторите.".to_string()
            }
            CmdErrorCode::AuthUnavailable => {
                "Сервер аутентификации недоступен. Повторите попытку позже или обратитесь к администратору."
                    .to_string()
            }
            CmdErrorCode::Internal => "Внутренняя ошибка Link. Повторите попытку.".to_string(),
        };
        Self { code, message }
    }
}

impl std::fmt::Display for LinkCmdError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for LinkCmdError {}
