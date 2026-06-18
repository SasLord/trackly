---
phase: "07-reports-dashboard-settings"
plan: 12
subsystem: "settings/templates"
tags: [gap-closure, tauri-command, template-preview, tdd]
dependency_graph:
  requires: []
  provides: ["settings_open_db_folder Tauri command", "act_acceptance preview in validate_preview"]
  affects: ["ui/src/bindings.ts", "crates/trackly-app/src/tauri_cmds/settings_org.rs", "crates/trackly-app/src/services/template_service.rs"]
tech_stack:
  added: []
  patterns: ["tauri::command + specta::specta dual-decorator", "TDD RED/GREEN for validate_preview"]
key_files:
  created: []
  modified:
    - "crates/trackly-app/src/tauri_cmds/settings_org.rs"
    - "crates/trackly-app/src/specta_export.rs"
    - "crates/trackly-app/src/services/template_service.rs"
decisions:
  - "settings_open_db_folder derives path from ctx.paths.db_path() — NOT user input; canonicalize + UNC rejection mirrors acts_open_pdf_in_system pattern"
  - "demo_ctx now covers all template kinds (act_handover + act_acceptance) with device/document keys"
metrics:
  duration: "15 min"
  completed: "2026-06-18"
---

# Phase 07 Plan 12: G2-2 Backend Command + G2-4 Template Preview Fix Summary

**One-liner:** Added `settings_open_db_folder` Tauri command with secure path canonicalization and fixed `validate_preview` demo_ctx to cover `act_acceptance` template variables (`device.*` / `document.*`).

## Tasks Completed

| # | Name | Commit | Files |
|---|------|--------|-------|
| 1 | Add settings_open_db_folder command (G2-2 backend) | 939f2ac | settings_org.rs, specta_export.rs |
| 2 (RED) | Add failing test for act_acceptance validate_preview | ec0a843 | template_service.rs |
| 2 (GREEN) | Expand validate_preview demo_ctx with device/document keys | ad934ca | template_service.rs |

## What Was Built

### G2-2: settings_open_db_folder Tauri command

Added to `crates/trackly-app/src/tauri_cmds/settings_org.rs`:
- `build_settings_open_db_folder(ctx, app)` — derives DB directory from `ctx.paths.db_path().parent()`, canonicalizes it via `std::fs::canonicalize`, rejects UNC paths, calls `app.shell().open()` with `#[allow(deprecated)]`.
- `settings_open_db_folder` Tauri command wrapper (no auth guard — opening a folder is a read-only OS action).
- `use tauri_plugin_shell::ShellExt;` import added.
- Registered in `specta_export.rs` collect_commands![] immediately after `settings_get_db_path`.
- `ui/src/bindings.ts` regenerated — `settings_open_db_folder` appears (1 match, gitignored file verified locally).

### G2-4: validate_preview demo_ctx expansion

Added to `validate_preview` in `template_service.rs`:
- `"device"` key: `name`, `inventory_no`, `serial_no`, `model`, `condition` — required by `act_acceptance.minijinja`.
- `"document"` key: `giver_name`, `receiver_name`, `date_human` — required by `act_acceptance.minijinja`.
- New unit test `validate_preview_act_acceptance_returns_pdf_bytes` — first confirmed failure (`undefined value` at line 19 of `_preview`, i.e., `document.date_human`), then passes after fix.

## Verification Results

| Check | Result |
|-------|--------|
| `cargo build --workspace` | 0 errors |
| `cargo test -p trackly-app --lib -- validate_preview` | 2 passed; 0 failed |
| `cargo test --test export_bindings` | 1 passed; 0 failed |
| `grep -c "settings_open_db_folder" ui/src/bindings.ts` | 1 |
| `grep -c "settings_open_db_folder" crates/trackly-app/src/specta_export.rs` | 1 |

## Deviations from Plan

None — plan executed exactly as written.

## TDD Gate Compliance

- RED commit: `ec0a843` — `test(07-12): add failing test for act_acceptance validate_preview (G2-4 RED)`
- GREEN commit: `ad934ca` — `feat(07-12): expand validate_preview demo_ctx with device/document keys (G2-4)`
- Both gates present and in correct order.

## Threat Surface Scan

No new network endpoints, auth paths, or schema changes introduced. `settings_open_db_folder` derives its path from internal `AppCtx` state (not user input) — T-07-12-01 mitigated via canonicalize + UNC rejection per plan threat model.

## Known Stubs

None.

## Self-Check: PASSED

- `crates/trackly-app/src/tauri_cmds/settings_org.rs` — FOUND: settings_open_db_folder
- `crates/trackly-app/src/specta_export.rs` — FOUND: settings_open_db_folder
- `crates/trackly-app/src/services/template_service.rs` — FOUND: device, document, validate_preview_act_acceptance_returns_pdf_bytes
- Commits 939f2ac, ec0a843, ad934ca — all present in git log
