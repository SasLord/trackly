//! `HealthDto` — единый «здоров ли инстанс?» payload для обоих транспортов.
//!
//! Phase 1 success criterion #5 (ROADMAP): ОДИН Rust-тип круглым ходом
//! проходит через Tauri command И axum handler, давая байт-идентичный JSON.
//! `tests/specta_roundtrip.rs` это и проверяет (`assert_eq!` через `PartialEq`).
//!
//! Поле `db_ready` в Phase 1 всегда `true` (если `AppCtx::build` отработал,
//! читатели поднялись; bool оставлен на Phase 5+, где можно вернуть `false`
//! при graceful-degraded режиме без БД).
//!
//! Поле `schema_version` копируется из `AppCtx.schema_version` после миграций
//! (= `max_known_version()` = 12 на момент написания).

use serde::{Deserialize, Serialize};
use specta::Type;

/// Health-check payload. Возвращается `GET /api/v1/health` (axum) и
/// `invoke('health')` (Tauri). Форма JSON одинаковая (success criterion #5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct HealthDto {
    /// Версия бинаря (`env!("CARGO_PKG_VERSION")` на момент сборки).
    pub version: String,
    /// Готовность БД-слоя (writer worker + reader pool). Phase 1 всегда `true`.
    pub db_ready: bool,
    /// `PRAGMA user_version` после миграций. Совпадает с
    /// `trackly_infra::db::migrations::max_known_version()`.
    pub schema_version: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serde_round_trip_preserves_all_fields() {
        let dto = HealthDto {
            version: "0.1.0".into(),
            db_ready: true,
            schema_version: 12,
        };
        let json = serde_json::to_string(&dto).expect("serialize");
        let back: HealthDto = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, dto);
    }

    #[test]
    fn serialize_emits_snake_case_field_names() {
        // Frontend (через bindings.ts) ожидает snake_case, потому что
        // specta::Type derive по умолчанию не renaming'ит. Если в будущем
        // понадобится camelCase — добавить `#[serde(rename_all = "camelCase")]`
        // НА HealthDto и убедиться, что specta-typescript тоже это видит.
        let dto = HealthDto {
            version: "0.1.0".into(),
            db_ready: true,
            schema_version: 12,
        };
        let json = serde_json::to_string(&dto).expect("serialize");
        assert!(json.contains("\"db_ready\""), "got: {json}");
        assert!(json.contains("\"schema_version\""), "got: {json}");
    }
}
