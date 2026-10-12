//! DTO справочника статусов устройств (Phase 41.1, D-06).
//!
//! Серверный справочник `device_statuses` — единственный источник подписей
//! и порядка статусов. Клиент читает его один раз, а не хардкодит имена.
//! Статусы картриджей в справочник не входят (D-07).

use serde::{Deserialize, Serialize};
use specta::Type;

/// Запись справочника статусов устройств.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct DeviceStatusDto {
    #[specta(type = i32)]
    pub id: i64,
    /// Машинный ключ статуса (`на_складе`, `в_работе`, ...).
    pub code: String,
    /// Подпись для интерфейса.
    pub name: String,
    /// Порядок жизненного цикла (D-08), а не алфавитный.
    #[specta(type = i32)]
    pub sort_order: i64,
}
