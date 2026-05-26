//! Integration-тест: генерирует `ui/src/bindings.ts` через
//! `tauri_specta::Builder::export`. Файл gitignored
//! (см. `.gitignore` Plan 01-01). Этот тест запускается:
//! - Каждый `cargo test` локально/в CI (через workspace test).
//! - `pnpm prebuild` hook в `ui/package.json` (`cargo test -p trackly-app
//!   --test export_bindings`) — перед `pnpm build`, чтобы фронтенд видел
//!   свежие типы.
//!
//! Защищает от export-drift (T-05-02 в threat model плана 05): если
//! `HealthDto` поменяется, а frontend ещё ждёт старую форму, `svelte-check`
//! в `pnpm prebuild` отловит mismatch.
//!
//! **Skipped on Windows** — `specta-typescript = "0.0.9"` triggers
//! `STATUS_ENTRYPOINT_NOT_FOUND` (0xc0000139) at test-binary load on
//! `windows-latest` runner. Upgrade path is blocked: `tauri-specta` newer
//! than `=2.0.0-rc.21` (which pins `specta-typescript = ^0.0.9` exactly)
//! requires `specta = rc.24+`, which in turn requires nightly-only
//! `debug_closure_helpers` + `const_type_id`. Coverage retained on Linux
//! + macOS — see `.planning/phases/01-foundation/deferred-items.md`.
#![cfg(not(target_os = "windows"))]

use std::path::PathBuf;

use specta_typescript::Typescript;
use trackly_app::specta_export::builder;

#[test]
fn export_bindings_to_ui_writes_health_dto_and_app_error() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // crates/trackly-app -> ../.. -> repo root -> ui/src/bindings.ts
    let target = manifest
        .parent()
        .expect("crates/")
        .parent()
        .expect("repo root")
        .join("ui/src/bindings.ts");

    // Path safety: убедимся что `target` действительно резолвится внутри
    // workspace (`ui/src/...`), а не куда-то наружу. Это и так гарантировано
    // структурой манифеста, но проверка дешёвая и ловит regression в layout'е.
    let target_str = target.to_string_lossy();
    assert!(
        target_str.ends_with("ui/src/bindings.ts"),
        "unexpected target path: {target_str}"
    );

    // create_dir_all не нужен — Builder::export сам создаст parent dir,
    // но если ui/src/ ещё не существует (свежий clone без pnpm install),
    // подстрахуемся явно.
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent).expect("ensure ui/src/");
    }

    builder()
        .export(Typescript::default(), &target)
        .expect("tauri-specta export failed");

    // Post-process: prepend `// @ts-nocheck` to the generated bindings.
    // tauri-specta emits `TAURI_CHANNEL` import and `__makeEvents__` helper
    // even when no events/channels are declared, which trips
    // `noUnusedLocals` in `pnpm svelte-check`. Generated file is rewritten
    // each `cargo test`, so the prefix MUST be applied here.
    let raw = std::fs::read_to_string(&target).expect("read bindings.ts for ts-nocheck prefix");
    if !raw.trim_start().starts_with("// @ts-nocheck") {
        let patched = format!("// @ts-nocheck\n{raw}");
        std::fs::write(&target, patched).expect("write bindings.ts with @ts-nocheck prefix");
    }

    let contents = std::fs::read_to_string(&target).expect("read bindings.ts");

    assert!(
        contents.contains("HealthDto"),
        "bindings.ts missing HealthDto type, got:\n{contents}"
    );
    assert!(
        contents.contains("version"),
        "bindings.ts missing 'version' field"
    );
    assert!(
        contents.contains("schema_version") || contents.contains("schemaVersion"),
        "bindings.ts missing 'schema_version' / 'schemaVersion' field"
    );
    // AppError exported as `AppError` (через AppErrorRepr с #[serde(rename = "AppError")]).
    assert!(
        contents.contains("AppError"),
        "bindings.ts missing AppError type"
    );

    // Phase 2 — Devices CRUD types.
    assert!(
        contents.contains("DeviceDto"),
        "bindings.ts missing DeviceDto type"
    );
    assert!(
        contents.contains("DeviceNew"),
        "bindings.ts missing DeviceNew type"
    );
    assert!(
        contents.contains("DevicePatch"),
        "bindings.ts missing DevicePatch type"
    );
    assert!(
        contents.contains("DeviceFilter"),
        "bindings.ts missing DeviceFilter type"
    );
    assert!(
        contents.contains("DeviceListResponse"),
        "bindings.ts missing DeviceListResponse type"
    );
    assert!(
        contents.contains("devices_list"),
        "bindings.ts missing devices_list command"
    );
    assert!(
        contents.contains("devices_create"),
        "bindings.ts missing devices_create command"
    );

    // Phase 2 — Plan 04: Search / Autocomplete / Grouping types and commands.
    assert!(
        contents.contains("DeviceGroup"),
        "bindings.ts missing DeviceGroup type"
    );
    assert!(
        contents.contains("StatusCount"),
        "bindings.ts missing StatusCount type"
    );
    assert!(
        contents.contains("devices_search"),
        "bindings.ts missing devices_search command"
    );
    assert!(
        contents.contains("devices_autocomplete"),
        "bindings.ts missing devices_autocomplete command"
    );
    assert!(
        contents.contains("devices_list_grouped"),
        "bindings.ts missing devices_list_grouped command"
    );
    assert!(
        contents.contains("devices_status_counts"),
        "bindings.ts missing devices_status_counts command"
    );
    assert!(
        contents.contains("devices_list_by_ids"),
        "bindings.ts missing devices_list_by_ids command"
    );

    // Phase 2 — Scope extension: bulk create.
    assert!(
        contents.contains("devices_bulk_create"),
        "bindings.ts missing devices_bulk_create command"
    );
}
