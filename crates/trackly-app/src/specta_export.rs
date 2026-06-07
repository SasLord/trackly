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
        // Phase 2 — Devices CRUD (Plan 03)
        crate::tauri_cmds::devices::devices_list,
        crate::tauri_cmds::devices::devices_get,
        crate::tauri_cmds::devices::devices_create,
        crate::tauri_cmds::devices::devices_update,
        crate::tauri_cmds::devices::devices_delete,
        crate::tauri_cmds::devices::devices_state_hints,
        // Phase 2 — Devices Search/Autocomplete/Grouping (Plan 04)
        crate::tauri_cmds::devices::devices_search,
        crate::tauri_cmds::devices::devices_autocomplete,
        crate::tauri_cmds::devices::locations_autocomplete,
        crate::tauri_cmds::devices::devices_list_grouped,
        crate::tauri_cmds::devices::devices_status_counts,
        crate::tauri_cmds::devices::devices_list_by_ids,
        // Phase 2 — Devices Bulk Create (scope extension 2026-05-26)
        crate::tauri_cmds::devices::devices_bulk_create,
        // Phase 2 — CSV import / export (Plan 05)
        crate::tauri_cmds::devices::devices_import_csv_preview,
        crate::tauri_cmds::devices::devices_import_csv_commit,
        crate::tauri_cmds::devices::devices_export_csv,
        // Phase 2 — FS helpers (Plan 05 B2 pinned strategy)
        crate::tauri_cmds::fs_helpers::read_file_bytes,
        crate::tauri_cmds::fs_helpers::write_file_bytes,
        // Phase 3 Plan 02 — Acts CRUD
        crate::tauri_cmds::acts::acts_list,
        crate::tauri_cmds::acts::acts_search,
        crate::tauri_cmds::acts::acts_get,
        crate::tauri_cmds::acts::acts_create,
        crate::tauri_cmds::acts::acts_return,
        crate::tauri_cmds::acts::acts_delete,
        crate::tauri_cmds::acts::acts_counts,
        crate::tauri_cmds::acts::acts_peek_next_number,
        // Phase 3 Plan 04 — PDF render + Organization + Templates
        crate::tauri_cmds::acts::acts_render_pdf,
        crate::tauri_cmds::acts::devices_render_acceptance_pdf,
        // Phase 3.1 Plan 02 — G-5 person autocomplete
        crate::tauri_cmds::acts::acts_suggest_person,
        // Phase 3.1 code review fix (CR-02) — secure shell::open wrapper
        crate::tauri_cmds::acts::acts_open_pdf_in_system,
        crate::tauri_cmds::organization::organization_get,
        crate::tauri_cmds::templates::templates_get_active,
        crate::tauri_cmds::templates::templates_render_preview,
        // Phase 4 — Cartridges
        crate::tauri_cmds::cartridges::cartridges_list,
        crate::tauri_cmds::cartridges::cartridges_get,
        crate::tauri_cmds::cartridges::cartridges_create,
        crate::tauri_cmds::cartridges::cartridges_update,
        crate::tauri_cmds::cartridges::cartridges_delete,
        crate::tauri_cmds::cartridges::cartridges_transition,
        crate::tauri_cmds::cartridges::cartridges_search,
        crate::tauri_cmds::cartridges::cartridges_status_counts,
        crate::tauri_cmds::cartridges::cartridges_get_history,
        crate::tauri_cmds::cartridges::cartridges_low_stock,
        crate::tauri_cmds::cartridges::cartridge_models_list,
        crate::tauri_cmds::cartridges::cartridge_models_get,
        crate::tauri_cmds::cartridges::cartridge_models_create,
        crate::tauri_cmds::cartridges::cartridge_models_update,
        crate::tauri_cmds::cartridges::cartridge_models_delete,
        crate::tauri_cmds::cartridges::cartridges_suggest_brand,
        crate::tauri_cmds::cartridges::cartridges_suggest_model,
        crate::tauri_cmds::cartridges::cartridges_suggest_compat_printer,
        crate::tauri_cmds::cartridges::cartridges_suggest_location,
    ])
}
