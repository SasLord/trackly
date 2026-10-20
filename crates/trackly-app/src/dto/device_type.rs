//! DTO справочника типов устройств (Phase 41.12, INT-14).
//!
//! `device_types` — единственный источник подписей типа: CSV-экспорт, CSV-импорт
//! и фильтры интерфейса читают его, а не держат собственные карты `type_id` -> подпись.

use serde::{Deserialize, Serialize};
use specta::Type;

/// Запись справочника типов устройств.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct DeviceTypeDto {
    #[specta(type = i32)]
    pub id: i64,
    /// Подпись для интерфейса и CSV.
    pub name: String,
}
