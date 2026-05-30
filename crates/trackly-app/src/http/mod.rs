//! axum HTTP routes. Phase 1 — только `health`. Phase 5+ добавит
//! `/api/v1/auth/*`, `/api/v1/devices/*`, ... и `tower-sessions` middleware.
//!
//! Каждый handler — тонкий адаптер над тем же `build_*` хелпером, который
//! используется в `tauri_cmds/*` — это инвариант «один DTO, два транспорта»
//! (success criterion #5).

pub mod acts;
pub mod devices;
pub mod fs_helpers;
pub mod health;
pub mod organization;
pub mod templates;
