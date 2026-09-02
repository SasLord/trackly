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
        // Phase 19 Plan 04 — ACT-02 act edit (header + device reconciliation)
        crate::tauri_cmds::acts::acts_update,
        // Phase 22 Plan 03 — ACT-03 return-act edit (delta reconciliation)
        crate::tauri_cmds::acts::acts_update_return,
        crate::tauri_cmds::acts::acts_counts,
        crate::tauri_cmds::acts::acts_peek_next_number,
        // Phase 3 Plan 04 — PDF render + Organization + Templates
        crate::tauri_cmds::acts::acts_render_pdf,
        crate::tauri_cmds::acts::devices_render_acceptance_pdf,
        // Phase 3.1 Plan 02 — G-5 person autocomplete
        crate::tauri_cmds::acts::acts_suggest_person,
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
        crate::tauri_cmds::cartridges::cartridge_storage_place_ids,
        // Phase 5 — Auth (Plan 03)
        crate::tauri_cmds::auth::auth_login,
        crate::tauri_cmds::auth::auth_logout,
        crate::tauri_cmds::auth::auth_status,
        crate::tauri_cmds::auth::auth_me,
        crate::tauri_cmds::auth::desktop_set_lock,
        // 09-AD-GAPS restoration-flow UX — explicit restore re-request
        crate::tauri_cmds::auth::request_ad_restore,
        // Phase 5 — Users (Plan 03)
        crate::tauri_cmds::users::users_list,
        crate::tauri_cmds::users::users_create,
        crate::tauri_cmds::users::users_update,
        crate::tauri_cmds::users::users_delete,
        crate::tauri_cmds::users::users_change_password,
        // Phase 5 — Settings / Server (Plan 03)
        crate::tauri_cmds::auth::server_toggle,
        crate::tauri_cmds::auth::server_status,
        // Phase 5 — Settings set network (Plan 05-06, gap closure)
        crate::tauri_cmds::auth::settings_set_network,
        // Phase 5 — Settings get network (gap fix: Tauri command was missing,
        // only the HTTP route existed → desktop Settings page failed to load)
        crate::tauri_cmds::auth::settings_get_network,
        // Phase 6 — Printers (Plan 03)
        crate::tauri_cmds::printers::printers_list,
        crate::tauri_cmds::printers::printers_get,
        // Phase 12 Round 5 gap closure (GAP-12-13): device-id-keyed printer read — printers_get resolves by printers.id, the UI only ever has device_id.
        crate::tauri_cmds::printers::printers_get_by_device_id,
        // Phase 13 (R4) — read-only агрегаты совместимых моделей картриджей по принтеру, заменяет удалённый per-device junction (V029).
        crate::tauri_cmds::printers::printers_get_compatible_aggregates,
        crate::tauri_cmds::printers::printers_create,
        crate::tauri_cmds::printers::printers_discover,
        crate::tauri_cmds::printers::printers_admit,
        crate::tauri_cmds::printers::printers_refresh,
        crate::tauri_cmds::printers::printers_acknowledge_alert,
        // Phase 6 — Requests (Plan 03)
        crate::tauri_cmds::requests::requests_list,
        crate::tauri_cmds::requests::requests_get,
        crate::tauri_cmds::requests::requests_create,
        crate::tauri_cmds::requests::requests_transition,
        crate::tauri_cmds::requests::requests_counts,
        crate::tauri_cmds::requests::requests_list_categories,
        crate::tauri_cmds::requests::requests_get_history,
        // Phase 11 — D-PRN-01: employee-facing printer options for the
        // create-request form (CreateRequest-gated, NOT ReadData/ReadPrinters).
        crate::tauri_cmds::requests::request_printer_options,
        // Phase 9 — AD register requests (Plan 03)
        crate::tauri_cmds::requests::requests_approve_ad_register,
        // Phase 12 gap closure (GAP-12-07/A4, Plan 12-14) — Admin/Manager
        // delete (any status) + Employee self-cancel (own request, open only).
        crate::tauri_cmds::requests::requests_delete,
        crate::tauri_cmds::requests::requests_cancel,
        // Phase 9 — AD settings (Plan 04)
        crate::tauri_cmds::auth::settings_get_ad,
        crate::tauri_cmds::auth::settings_set_ad,
        // Phase 9 — AD test connection (gap-closure)
        crate::tauri_cmds::auth::ad_test_connection,
        // Phase 7 — Reports (Plan 07)
        crate::tauri_cmds::reports::reports_list_device_acts,
        crate::tauri_cmds::reports::reports_list_device_returns,
        crate::tauri_cmds::reports::reports_list_device_in_use,
        crate::tauri_cmds::reports::reports_list_device_in_stock,
        crate::tauri_cmds::reports::reports_list_cartridge_consumption,
        crate::tauri_cmds::reports::reports_list_cartridge_refills,
        crate::tauri_cmds::reports::reports_list_cartridge_in_use,
        crate::tauri_cmds::reports::reports_list_cartridge_in_stock,
        crate::tauri_cmds::reports::reports_list_requests_all,
        crate::tauri_cmds::reports::reports_list_requests_open,
        crate::tauri_cmds::reports::reports_list_requests_in_progress,
        crate::tauri_cmds::reports::reports_list_requests_completed,
        crate::tauri_cmds::reports::reports_list_movements,
        crate::tauri_cmds::reports::reports_export_csv,
        crate::tauri_cmds::reports::reports_export_pdf,
        crate::tauri_cmds::reports::reports_get_report_counts,
        // Phase 7 — Dashboard (Plan 07)
        crate::tauri_cmds::dashboard::dashboard_get_all_widgets,
        crate::tauri_cmds::dashboard::dashboard_get_consumption_chart,
        // Phase 7 — Settings Org / Backup / Templates (Plan 07)
        crate::tauri_cmds::settings_org::settings_get_org,
        crate::tauri_cmds::settings_org::settings_save_org_fields,
        crate::tauri_cmds::settings_org::settings_get_org_logo,
        crate::tauri_cmds::settings_org::settings_save_org_logo,
        crate::tauri_cmds::settings_org::settings_remove_org_logo,
        crate::tauri_cmds::settings_org::settings_get_db_path,
        crate::tauri_cmds::settings_org::settings_open_db_folder,
        crate::tauri_cmds::settings_org::settings_move_db,
        crate::tauri_cmds::settings_org::app_restart,
        crate::tauri_cmds::settings_org::settings_get_low_stock_threshold,
        crate::tauri_cmds::settings_org::settings_set_low_stock_threshold,
        crate::tauri_cmds::settings_org::settings_get_low_stock_basis,
        crate::tauri_cmds::settings_org::settings_set_low_stock_basis,
        crate::tauri_cmds::settings_org::settings_get_place_path_defaults,
        crate::tauri_cmds::settings_org::settings_set_place_path_defaults,
        crate::tauri_cmds::settings_org::settings_get_backup_config,
        crate::tauri_cmds::settings_org::settings_save_backup_config,
        crate::tauri_cmds::settings_org::backup_run_manual,
        crate::tauri_cmds::settings_org::templates_list_for_editor,
        crate::tauri_cmds::settings_org::templates_update_body,
        crate::tauri_cmds::settings_org::templates_reset_to_default,
        crate::tauri_cmds::settings_org::templates_validate_preview,
        crate::tauri_cmds::settings_org::templates_status,
        // Phase 39 — Places CRUD (Plan 12)
        crate::tauri_cmds::places::places_create,
        crate::tauri_cmds::places::places_rename,
        crate::tauri_cmds::places::places_set_path_variant,
        crate::tauri_cmds::places::places_move,
        crate::tauri_cmds::places::places_archive,
        crate::tauri_cmds::places::places_unarchive,
        crate::tauri_cmds::places::places_delete,
        crate::tauri_cmds::places::places_get,
        crate::tauri_cmds::places::places_list_children,
        crate::tauri_cmds::places::places_list_all,
        crate::tauri_cmds::places::places_subtree_stats,
        crate::tauri_cmds::places::places_contents,
        crate::tauri_cmds::places::places_search,
        crate::tauri_cmds::places::places_move_subtree_contents,
        // Phase 40 — Movement history timeline (Plan 10)
        crate::tauri_cmds::place_movements::place_movements_get_timeline,
    ])
}
