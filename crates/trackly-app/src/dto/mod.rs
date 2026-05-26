//! DTOs — единый источник истины для двух транспортов (Tauri command +
//! axum HTTP).
//!
//! Plan 05: только `HealthDto`. Каждое следующее phase добавляет DTO сюда
//! (devices, acts, cartridges, …), и они автоматически экспортируются в
//! `ui/src/bindings.ts` через `specta_export::builder` (Builder собирает все
//! DTOs, на которые ссылаются зарегистрированные команды).

pub mod device;
pub mod health;

pub use health::HealthDto;
