//! `#[tauri::command]`-функции, регистрируемые в `tauri_specta::Builder`
//! через `collect_commands![...]` (см. `specta_export.rs`).
//!
//! Plan 05: только `health`. Каждая команда — тонкий адаптер над
//! `build_*` хелпером, который не зависит от `tauri::State<'_, _>` (lifetime
//! gymnastics ломает unit-тесты). axum-handler из `http/*` вызывает тот же
//! хелпер — это и есть «один DTO, два транспорта».

pub mod devices;
pub mod fs_helpers;
pub mod health;
