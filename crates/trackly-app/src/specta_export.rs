//! `tauri_specta::Builder` — единая точка сбора Tauri-команд для:
//! 1. Генерации `ui/src/bindings.ts` (вызывается из
//!    `tests/export_bindings.rs` каждый `cargo test`, плюс из
//!    `ui/package.json` `prebuild`-hook'а через `cargo test --test
//!    export_bindings`).
//! 2. Подключения к реальному `tauri::Builder` через
//!    `.invoke_handler(builder.invoke_handler())` (Plan 03).
//!
//! Каждое следующее phase, добавляющее `#[tauri::command]`, ОБЯЗАНО
//! зарегистрировать её здесь — иначе frontend (через bindings.ts) не увидит
//! новый API. Code-review checklist (T-05-06 в threat model плана 05).

use tauri_specta::{collect_commands, Builder};

/// Строит `Builder` со всеми командами Phase 1 + Phase 2. Один и тот же `Builder`
/// используется и тестом экспорта (`tests/export_bindings.rs`), и Tauri runtime'ом.
pub fn builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new().commands(collect_commands![
        // Phase 1
        crate::tauri_cmds::health::health,
        // Phase 2 — Devices CRUD
        crate::tauri_cmds::devices::devices_list,
        crate::tauri_cmds::devices::devices_get,
        crate::tauri_cmds::devices::devices_create,
        crate::tauri_cmds::devices::devices_update,
        crate::tauri_cmds::devices::devices_delete,
        crate::tauri_cmds::devices::devices_state_hints,
    ])
}
