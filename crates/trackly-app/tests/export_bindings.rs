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
}
