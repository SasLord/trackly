//! `AppError` — типизированная ошибка приложения.
//!
//! Plan 02 bootstrap: только два варианта (`Internal`, `Validation`) — достаточно
//! чтобы `trackly_infra::paths` и `trackly_infra::config` могли возвращать типизированные
//! ошибки. Plan 04 расширит enum до полного D-AppError-01 списка
//! (`NotFound`, `Conflict`, `OptimisticLockMismatch`, `WriteQueueBusy`,
//! `DatabaseFromNewerVersion`, `Unauthorized`, `Forbidden`, …) и добавит
//! единый JSON Serialize + axum `IntoResponse`.

use serde::Serialize;

/// Главный тип ошибки приложения. См. D-AppError-01.
///
/// На Plan 02 здесь только два варианта — достаточно, чтобы paths.rs и
/// config.rs возвращали типизированные ошибки. Plan 04 добавит остальные
/// варианты и реализацию `IntoResponse`.
#[derive(Debug, thiserror::Error, Serialize)]
#[serde(tag = "code", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AppError {
    /// Внутренняя ошибка приложения (I/O, current_exe, неожиданное состояние).
    #[error("internal: {source_chain}")]
    Internal {
        /// Цепочка причин (обычно отформатированный `anyhow::Error`).
        source_chain: String,
    },
    /// Валидация входных данных (TOML, путь, поле формы).
    #[error("validation [{field}]: {message}")]
    Validation {
        /// Имя поля или источника, который не прошёл валидацию.
        field: String,
        /// Сообщение, пригодное к показу администратору.
        message: String,
    },
}
