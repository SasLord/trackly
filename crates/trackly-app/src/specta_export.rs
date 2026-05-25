//! `tauri_specta::Builder` — единая точка сбора Tauri-команд для:
//! 1. Генерации `ui/src/bindings.ts` (вызывается из
//!    `tests/export_bindings.rs` каждый `cargo test`, плюс из
//!    `ui/package.json` `prebuild`-hook'а через `cargo test --test
//!    export_bindings`).
//! 2. (Phase 2+) Подключения к реальному `tauri::Builder` через
//!    `.invoke_handler(builder.invoke_handler())` — Phase 1 ещё не поднимает
//!    Tauri runtime, только бинаря через `--self-test`.
//!
//! Каждое следующее phase, добавляющее `#[tauri::command]`, ОБЯЗАНО
//! зарегистрировать её здесь — иначе frontend (через bindings.ts) не увидит
//! новый API. Code-review checklist (T-05-06 в threat model плана 05).

use tauri_specta::{collect_commands, Builder};

/// Строит `Builder` со всеми Phase 1 командами. Один и тот же `Builder`
/// используется и тестом экспорта (`tests/export_bindings.rs`), и Phase 2+
/// Tauri runtime'ом.
pub fn builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new().commands(collect_commands![crate::tauri_cmds::health::health])
}
