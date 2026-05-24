//! `AppConfig` — TOML-парсер `trackly.config.toml` (D-Config-01).
//!
//! Все секции и поля опциональны: отсутствующий файл → `Self::default()`
//! (НЕ ошибка), отсутствующие секции → их `Default::default()`. Дефолты
//! заданы вручную через `impl Default` для каждой секции (понятнее, чем
//! `#[serde(default = "...")]` paths-функции на каждое поле).
//!
//! Malformed TOML → `AppError::Validation { field: "trackly.config.toml", ... }`,
//! ловится в `main.rs` через `?` и печатается админу.
//!
//! Неизвестные ключи (forward-compat): `toml::from_str` по умолчанию их
//! игнорирует (мы не пишем `#[serde(deny_unknown_fields)]`) — это
//! сознательное решение, чтобы старый бинарь не падал при чтении конфига
//! от новой версии.

use std::io::ErrorKind;
use std::path::Path;

use serde::Deserialize;
use trackly_core::error::AppError;

/// Корневая структура `trackly.config.toml`.
#[derive(Debug, Deserialize, Clone, Default)]
pub struct AppConfig {
    /// Параметры сервер-режима (axum + tower-sessions, Phase 5/Plan 05).
    #[serde(default)]
    pub server: ServerConfig,
    /// Переопределения путей (db_path; остальное диктует `Paths`).
    #[serde(default)]
    pub paths: PathsConfig,
    /// Параметры tracing (Plan 05).
    #[serde(default)]
    pub logging: LoggingConfig,
    /// Параметры организации (timezone для UI; в БД всё в UTC).
    #[serde(default)]
    pub organization: OrganizationConfig,
}

/// `[server]` — параметры HTTP/LAN сервера.
#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    /// Включить ли встроенный axum сервер в режиме «доступ из локальной сети».
    pub enabled: bool,
    /// Адрес bind'а. По умолчанию `127.0.0.1` (только локальные подключения).
    pub host: String,
    /// Порт HTTP/HTTPS сервера.
    pub port: u16,
    /// Путь к TLS-сертификату. Пусто — генерируется self-signed на первом запуске.
    pub cert_path: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        // D-Config-01 дефолты.
        Self {
            enabled: false,
            host: "127.0.0.1".to_string(),
            port: 8443,
            cert_path: String::new(),
        }
    }
}

/// `[paths]` — переопределения путей. Сейчас только `db_path`.
///
/// `Default` производный: пустая строка `db_path` означает «использовать
/// `Paths::db_path()` дефолт» (`<exe_dir>/trackly.db`).
#[derive(Debug, Deserialize, Clone, Default)]
pub struct PathsConfig {
    /// Полный путь к SQLite-файлу. Пусто = `Paths::db_path()` default
    /// (`<exe_dir>/trackly.db`).
    pub db_path: String,
}

/// `[logging]` — параметры tracing.
#[derive(Debug, Deserialize, Clone)]
pub struct LoggingConfig {
    /// Уровень: `trace|debug|info|warn|error`.
    pub level: String,
    /// Формат: `compact|json`.
    pub format: String,
    /// Сколько дней хранить логи. Background-чистка — Phase 7.
    pub retention_days: u32,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            format: "compact".to_string(),
            retention_days: 14,
        }
    }
}

/// `[organization]` — UI-параметры организации.
#[derive(Debug, Deserialize, Clone)]
pub struct OrganizationConfig {
    /// Часовой пояс для отображения в UI. В БД всё в UTC.
    pub timezone: String,
}

impl Default for OrganizationConfig {
    fn default() -> Self {
        Self {
            timezone: "Europe/Moscow".to_string(),
        }
    }
}

impl AppConfig {
    /// Парсит `trackly.config.toml` по указанному пути.
    ///
    /// - Файл отсутствует (`ErrorKind::NotFound`) → `Self::default()` (НЕ ошибка).
    /// - I/O-ошибка чтения → `AppError::Internal`.
    /// - Файл не парсится → `AppError::Validation { field: <file_name>, message }`.
    /// - Файл валиден → распарсенный `AppConfig`. Неизвестные поля
    ///   игнорируются (forward-compat).
    pub fn load_or_default(path: &Path) -> Result<Self, AppError> {
        let contents = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(Self::default()),
            Err(e) => {
                return Err(AppError::Internal {
                    source_chain: format!("read config {}: {e}", path.display()),
                });
            }
        };

        let field = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("trackly.config.toml")
            .to_string();

        toml::from_str::<Self>(&contents).map_err(|e| AppError::Validation {
            field,
            message: format!("TOML parse error: {e}"),
        })
    }
}
