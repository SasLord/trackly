---
phase: 02-ui
plan: "05"
subsystem: devices-csv
tags:
  - vertical-slice
  - devices
  - csv
  - mvp-slice-3
  - encoding
  - import
  - export

dependency_graph:
  requires:
    - 02-03
    - 02-04
  provides:
    - csv-import-pipeline (chardetng + encoding_rs + csv crate)
    - csv-export-utf8-bom-semicolons
    - fs-helper-tauri-commands
    - import-session-store-5min-ttl
  affects:
    - DevicesPage (CSV buttons activated)
    - DeviceService (3 new async methods)
    - Tauri bindings (5 new commands)
    - axum routes (5 new endpoints)

tech_stack:
  added:
    - chardetng 0.1 (encoding detection)
    - encoding_rs 0.8 (decode CP1251/UTF-8)
    - csv 1.3 (parse + write)
    - uuid 1 (session tokens)
    - tauri-plugin-dialog (file picker / save dialog)
  patterns:
    - preview-then-commit CSV import with 5-min TTL session store
    - per-row error accumulation (D-CSV-01)
    - UTF-8 BOM + semicolons for Russian Excel compatibility (D-CSV-02)
    - Excel formula injection prevention (csv_safe, T-02-05-03)
    - path traversal prevention in FS helpers (T-02-05-02)

key_files:
  created:
    - crates/trackly-app/src/csv/sniff.rs
    - crates/trackly-app/src/csv/decode.rs
    - crates/trackly-app/src/csv/parse.rs
    - crates/trackly-app/src/tauri_cmds/fs_helpers.rs
    - crates/trackly-app/src/http/fs_helpers.rs
    - crates/trackly-app/tests/devices_csv_import.rs
    - crates/trackly-app/tests/devices_csv_export.rs
    - crates/trackly-app/tests/devices_csv_session.rs
    - crates/trackly-app/tests/fixtures/devices/ (5 files)
    - ui/src/features/devices/DeviceImportCsvModal.svelte
  modified:
    - crates/trackly-app/src/csv/mod.rs
    - crates/trackly-app/src/csv/session_store.rs (unchanged, already complete)
    - crates/trackly-app/src/dto/device.rs (CSV DTOs added)
    - crates/trackly-app/src/services/device_service.rs (3 new methods)
    - crates/trackly-app/src/tauri_cmds/devices.rs (3 CSV commands)
    - crates/trackly-app/src/tauri_cmds/mod.rs
    - crates/trackly-app/src/http/devices.rs (3 CSV routes)
    - crates/trackly-app/src/http/mod.rs
    - crates/trackly-app/src/specta_export.rs (5 new commands)
    - crates/trackly-app/tests/export_bindings.rs
    - ui/src/lib/api/devices.ts
    - ui/src/features/devices/DevicesPage.svelte
    - ui/eslint.config.js

decisions:
  - title: "export_csv bypasses list() 200-item pagination cap via direct repo call"
    rationale: "export_csv calls repo.list() directly with limit=1_000_000 rather than DeviceService::list() which has a 200-item validation guard. This is the correct pattern for bulk export — the cap exists to protect UI pagination, not export."
  - title: "FS helpers as Tauri commands (B2 pinned strategy) rather than tauri-plugin-fs"
    rationale: "Backend FS helpers provide centralized path validation (T-02-05-02) with canonicalize → ..reject → UNC reject → .csv extension whitelist → 50MB cap. Plugin-fs would bypass this validation."
  - title: "DeviceImportCsvModal: file pick triggers read_file_bytes backend command"
    rationale: "Dialog returns path string; backend reads bytes with validation. Avoids large ArrayBuffer transfers via Tauri invoke and centralizes security policy."

metrics:
  duration: "~4 hours"
  completed: "2026-05-27"
  tasks_completed: 3
  tasks_total: 4
  files_created: 19
  files_modified: 12
  tests_added: 23
---

# Phase 2 Plan 05: CSV Import + Export Summary

CSV import/export vertical slice with chardetng encoding detection, UTF-8 BOM export, per-row error accumulation, 5-min TTL preview→commit token, 4-step frontend wizard.

## Tasks Completed

| Task | Name | Commit | Status |
|------|------|--------|--------|
| 1 | CSV pipeline (sniff/decode/parse) + fixtures + unit tests | 96ca947 | Done |
| 2 | Tauri commands, axum routes, FS helpers, bindings | 2a31837 | Done |
| 3 | Frontend DeviceImportCsvModal + Export CSV button | 619699b | Done |
| 4 | Checkpoint: manual smoke verification | — | Awaiting human |

## Test Results

- `cargo test -p trackly-app csv::` — 16 unit tests (sniff/decode/parse) PASSED
- `cargo test -p trackly-app --test devices_csv_import` — 10 integration tests PASSED
- `cargo test -p trackly-app --test devices_csv_export` — 7 integration tests PASSED
- `cargo test -p trackly-app --test devices_csv_session` — 5 integration tests PASSED
- `cargo test -p trackly-app --test export_bindings` — 1 test PASSED (bindings.ts verified)
- `pnpm svelte-check` — 0 errors (12 pre-existing warnings in older files)
- `pnpm build` — clean build, 180 modules, 107.86 kB JS bundle

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Stale Build Cache] 9 false-positive compilation errors resolved**
- Found during: Pre-task check
- Issue: `cargo check -p trackly-app` reported 9 errors including "no field `location` on type `DeviceRow`" and "method not found" for trait methods on `Arc<SqliteDeviceRepository>`. These were stale cache artifacts from previous compilation passes.
- Fix: `cargo clean -p trackly-core` cleared the stale incremental compilation cache. After clean, all 9 errors vanished with 0 actual code changes.
- Commit: 96ca947 (included in Task 1 commit)

**2. [Rule 1 - Bug] export_csv bypassed pagination limit**
- Found during: Task 2 — devices_csv_export.rs tests
- Issue: `DeviceService::export_csv` called `self.list(filter, Pagination { limit: 1_000_000 })` but `list()` has a validation cap of 200 items, returning `AppError::Validation` for larger limits. All 7 export tests failed.
- Fix: Refactored `export_csv` to call `repo.as_ref().list()` directly via `spawn_blocking`, bypassing the presentation-layer pagination cap. The 200-item cap is correct for UI pagination but wrong for bulk export.
- Files modified: `crates/trackly-app/src/services/device_service.rs`
- Commit: 2a31837

**3. [Rule 1 - Clippy] Doc comment indentation in devices_sqlite.rs**
- Found during: Task 2 — `cargo clippy -p trackly-app --all-targets -D warnings`
- Issue: Pre-existing clippy `doc_lazy_continuation` lint in `trackly-infra/src/repos/devices_sqlite.rs:93` was failing the clippy gate.
- Fix: Added proper 2-space indentation to the bullet list in the doc comment.
- Files modified: `crates/trackly-infra/src/repos/devices_sqlite.rs`
- Commit: 2a31837

**4. [Rule 1 - Clippy] write_record borrowed expression lint**
- Found during: Task 2 — `cargo clippy -D warnings`
- Issue: `wtr.write_record(&[...])` triggered `clippy::needless_borrow` — the array already implements `AsRef<[u8]>`.
- Fix: Changed to `wtr.write_record([...])` (pass array directly).
- Files modified: `crates/trackly-app/src/services/device_service.rs`
- Commit: 2a31837

**5. [Rule 2 - Missing Browser Globals] Blob not in ESLint config**
- Found during: Task 3 — `pnpm lint`
- Issue: `eslint.config.js` browser globals list was missing `Blob`, `FileReader`, `FormData`, causing `no-undef` lint errors in DevicesPage.svelte.
- Fix: Added the missing globals to eslint.config.js.
- Files modified: `ui/eslint.config.js`
- Commit: 619699b

## Security Mitigations Applied

| Threat ID | Status |
|-----------|--------|
| T-02-05-01 | Mitigated — 50MB cap in `import_csv_preview` + `read_file_bytes` |
| T-02-05-02 | Mitigated — canonicalize → ..reject → UNC reject → .csv extension → size cap in `fs_helpers.rs` |
| T-02-05-03 | Mitigated — `csv_safe()` prefixes `=`,`+`,`-`,`@` cells with `'` in export |
| T-02-05-04 | Mitigated — per-row error accumulation in `import_csv_commit` |
| T-02-05-06 | Mitigated — lazy sweep on `put()` in `ImportSessionStore` |
| T-02-05-08 | Mitigated — mapping keys validated against enum whitelist (unknown keys ignored) |

## Known Stubs

None — all functionality is wired. The frontend modal uses `apiCall('read_file_bytes', { path })` which calls the backend helper; `export_csv` uses the real repo call. No placeholder data.

## Self-Check: PASSED

Files exist:
- `crates/trackly-app/src/csv/sniff.rs` — FOUND
- `crates/trackly-app/src/csv/decode.rs` — FOUND
- `crates/trackly-app/src/csv/parse.rs` — FOUND
- `crates/trackly-app/src/tauri_cmds/fs_helpers.rs` — FOUND
- `crates/trackly-app/tests/devices_csv_import.rs` — FOUND
- `crates/trackly-app/tests/devices_csv_export.rs` — FOUND
- `crates/trackly-app/tests/devices_csv_session.rs` — FOUND
- `ui/src/features/devices/DeviceImportCsvModal.svelte` — FOUND

Commits exist:
- 96ca947 — FOUND
- 2a31837 — FOUND
- 619699b — FOUND
