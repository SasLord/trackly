//! Auth/User DTOs — Plan 05.
//!
//! Все типы: `Debug, Clone, Serialize, Deserialize, Type`; snake_case JSON;
//! `i64`-поля аннотированы `#[specta(type = i32)]` (паттерн из `dto/device.rs`).
//!
//! `password_hash` никогда не включается в DTO (T-05-02 Info Disclosure mitigation).

use serde::{Deserialize, Serialize};
use specta::Type;

// ---------------------------------------------------------------------------
// Auth
// ---------------------------------------------------------------------------

/// Запрос на вход (логин + пароль в открытом виде).
///
/// Пароль хранится здесь как строка — `AuthService` оборачивает его в
/// `Secret<String>` перед сравнением с хэшем из БД.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct LoginRequest {
    pub login: String,
    pub password: String,
}

// ---------------------------------------------------------------------------
// User DTOs
// ---------------------------------------------------------------------------

/// Полное представление пользователя для frontend.
///
/// `password_hash` намеренно исключён (T-05-02).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct UserDto {
    #[specta(type = i32)]
    pub id: i64,
    #[specta(type = i32)]
    pub version: i64,
    pub login: String,
    pub full_name: String,
    /// Строковая роль: "admin" | "manager" | "employee".
    pub role: String,
    pub email: Option<String>,
    pub is_active: bool,
    #[specta(type = i32)]
    pub created_at_utc: i64,
    #[specta(type = i32)]
    pub updated_at_utc: i64,
}

/// DTO для создания нового пользователя.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct UserNew {
    pub login: String,
    pub full_name: String,
    /// Пароль в открытом виде — `AuthService` хэширует через argon2id.
    pub password: String,
    /// Строковая роль: "admin" | "manager" | "employee".
    pub role: String,
    pub email: Option<String>,
}

/// DTO для частичного обновления пользователя.
///
/// `None` — поле не меняется; `Some(None)` — установить NULL (для `email`).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct UserPatch {
    pub full_name: Option<String>,
    /// Новая роль: "admin" | "manager" | "employee". `None` — не менять.
    pub role: Option<String>,
    /// `Some(None)` — убрать email; `Some(Some(addr))` — установить; `None` — не менять.
    pub email: Option<Option<String>>,
    pub is_active: Option<bool>,
}

/// Запрос на смену пароля.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ChangePasswordRequest {
    pub old_password: String,
    pub new_password: String,
}

// ---------------------------------------------------------------------------
// Session / Auth status
// ---------------------------------------------------------------------------

/// Текущий статус аутентификации, возвращаемый при загрузке приложения.
///
/// `needs_bootstrap` — в БД нет ни одного пользователя (первый запуск).
/// `desktop_lock_enabled` — читается из `app_settings` по ключу
/// `desktop_lock_enabled` (D-Desktop-02); по умолчанию `false` (D-Desktop-01).
/// `user` — текущий авторизованный пользователь или `null` если не вошёл.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct AuthStatusDto {
    pub needs_bootstrap: bool,
    /// Флаг «десктоп требует входа» (D-Desktop-02).
    /// По умолчанию `false` — десктоп-режим без блокировки (D-Desktop-01).
    pub desktop_lock_enabled: bool,
    pub user: Option<UserDto>,
}

// ---------------------------------------------------------------------------
// Network / Server settings DTOs
// ---------------------------------------------------------------------------

/// Текущие настройки сетевого сервера.
///
/// Используется как ответ на `get_network_settings` и тело для `save_network_settings`.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct NetworkSettingsDto {
    /// Включён ли режим сервера.
    pub enabled: bool,
    /// Bind-адрес (например `"0.0.0.0"` или `"127.0.0.1"`).
    pub host: String,
    /// Порт (1..=65535).
    #[specta(type = i32)]
    pub port: i64,
    /// Путь к PEM-сертификату (пустая строка — self-signed).
    pub cert_path: String,
    /// URL сервера для отображения пользователю (например `https://192.168.1.5:8443`).
    pub server_url: Option<String>,
    /// SHA-256 fingerprint сертификата (hex, без двоеточий). `None` если сервер не запущен.
    pub fingerprint: Option<String>,
    /// Флаг «десктоп требует входа» (D-Desktop-02) — дублируется здесь
    /// для удобства формы настроек.
    pub desktop_lock_enabled: bool,
}

/// Статус запущенного сервера.
///
/// Возвращается командой `get_server_status`.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ServerStatusDto {
    pub running: bool,
    /// Полный URL (например `https://192.168.1.5:8443`). `None` если не запущен.
    pub url: Option<String>,
    /// SHA-256 fingerprint сертификата (hex). `None` если не запущен или нет TLS.
    pub fingerprint: Option<String>,
}

// ---------------------------------------------------------------------------
// User list / filter
// ---------------------------------------------------------------------------

/// Фильтр для списка пользователей.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct UserFilter {
    /// Поиск по логину или полному имени (LIKE %search%).
    pub search: Option<String>,
}

/// Ответ на запрос списка пользователей.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct UserListResponse {
    pub items: Vec<UserDto>,
    #[specta(type = i32)]
    pub total: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn login_request_serde_roundtrip() {
        let req = LoginRequest {
            login: "admin".to_string(),
            password: "secret".to_string(),
        };
        let json = serde_json::to_string(&req).unwrap();
        let back: LoginRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(back.login, req.login);
        assert_eq!(back.password, req.password);
    }

    #[test]
    fn auth_status_dto_has_desktop_lock_enabled() {
        let dto = AuthStatusDto {
            needs_bootstrap: true,
            desktop_lock_enabled: false,
            user: None,
        };
        let json = serde_json::to_string(&dto).unwrap();
        assert!(json.contains("desktop_lock_enabled"));
        assert!(json.contains("needs_bootstrap"));
    }

    #[test]
    fn network_settings_dto_has_desktop_lock_enabled() {
        let dto = NetworkSettingsDto {
            enabled: false,
            host: "0.0.0.0".to_string(),
            port: 8443,
            cert_path: String::new(),
            server_url: None,
            fingerprint: None,
            desktop_lock_enabled: false,
        };
        let json = serde_json::to_string(&dto).unwrap();
        assert!(json.contains("desktop_lock_enabled"));
    }

    #[test]
    fn user_dto_no_password_hash_field() {
        let dto = UserDto {
            id: 1,
            version: 1,
            login: "admin".to_string(),
            full_name: "Администратор".to_string(),
            role: "admin".to_string(),
            email: None,
            is_active: true,
            created_at_utc: 0,
            updated_at_utc: 0,
        };
        let json = serde_json::to_string(&dto).unwrap();
        assert!(
            !json.contains("password_hash"),
            "UserDto не должен содержать password_hash, json: {json}"
        );
    }

    #[test]
    fn snake_case_json_invariant() {
        let dto = UserDto {
            id: 1,
            version: 1,
            login: "u".to_string(),
            full_name: "N".to_string(),
            role: "admin".to_string(),
            email: None,
            is_active: true,
            created_at_utc: 0,
            updated_at_utc: 0,
        };
        let json = serde_json::to_string(&dto).unwrap();
        assert!(json.contains("full_name"), "snake_case: full_name");
        assert!(json.contains("is_active"), "snake_case: is_active");
        assert!(json.contains("created_at_utc"), "snake_case: created_at_utc");
        assert!(!json.contains("fullName"), "НЕ camelCase");
    }
}
