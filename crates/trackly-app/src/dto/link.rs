//! DTO админки раздела «Link» (Phase 44-16, D-05/D-12/D-13).
//!
//! Сами коды-приглашения и хэши секретов в DTO списков НЕ попадают (T-44-61):
//! код возвращается один раз — в [`InviteIssuedDto`] при выпуске.

use serde::{Deserialize, Serialize};
use specta::Type;

/// Допустимые сроки действия кода, часы (UI-SPEC §8.4).
pub const INVITE_TTL_HOURS: [i32; 3] = [1, 24, 168];

/// Запрос на выпуск кода-приглашения.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CreateInviteRequest {
    /// Адрес сервера для клиента. Пустая строка — адрес по умолчанию из сети сервера.
    pub host: String,
    /// Срок действия: 1, 24 или 168 часов.
    pub ttl_hours: i32,
}

/// Только что выпущенный код (показывается один раз).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct InviteIssuedDto {
    #[specta(type = i32)]
    pub id: i64,
    pub code: String,
    #[specta(type = i32)]
    pub expires_at: i64,
    pub host: String,
}

/// Строка списка кодов (без самого кода).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct InviteRowDto {
    #[specta(type = i32)]
    pub id: i64,
    #[specta(type = i32)]
    pub created_at: i64,
    pub created_by_full_name: String,
    #[specta(type = i32)]
    pub expires_at: i64,
    /// `active` | `used` | `expired` | `revoked`.
    pub status: String,
    pub used_by_hostname: Option<String>,
}

/// Активный вход Link.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct LinkLoginRowDto {
    #[specta(type = i32)]
    pub login_id: i64,
    #[specta(type = i32)]
    pub user_id: i64,
    pub full_name: String,
    pub login: String,
    pub hostname: String,
    /// `sso` | `password`.
    pub method: String,
    #[specta(type = i32)]
    pub created_at: i64,
}

/// Запрещённый компьютер.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct BannedDeviceDto {
    pub device_id: String,
    pub hostname: String,
    pub last_user_full_name: Option<String>,
    #[specta(type = i32)]
    pub revoked_at: i64,
}

/// Счётчик активных входов пользователя (карточка пользователя).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct UserLinkStatusDto {
    #[specta(type = i32)]
    pub active_logins: i64,
}

/// Итог массового отзыва.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RevokeResultDto {
    pub revoked: i32,
}
