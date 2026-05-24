//! `trackly` binary — Phase 1 ordered lifecycle (Plan 02).
//!
//! Ordering invariant (RESEARCH §Code Example 1 + Pitfall #1):
//!
//! 1. `Paths::resolve()` — root all I/O on `current_exe()?.parent()?`.
//! 2. `set_webview2_data_folder()` — set `WEBVIEW2_USER_DATA_FOLDER`
//!    BEFORE any tokio runtime / thread spawn / tauri::Builder call.
//! 3. Parse `--self-test` flag.
//! 4. `AppConfig::load_or_default()` — read `trackly.config.toml` or use defaults.
//! 5. (Plan 05) tracing subscriber + tracing-appender.
//! 6. (Plans 03/04) writer connection + PRAGMAs + refinery migrations
//!    + reader pool + AppCtx.
//! 7. If `--self-test`: print diagnostic lines, exit 0.
//! 8. Else (Plan 04+): tauri::Builder::run / axum::serve.
//!
//! Plan 02 owns steps 1-4 + 11-12 (placeholder for 12 until Plan 04/05).

use trackly_app::webview_env;
use trackly_infra::{AppConfig, Paths};

fn main() -> anyhow::Result<()> {
    // Step 1: resolve all paths from current_exe().
    let paths = Paths::resolve()?;

    // Step 2: set WEBVIEW2_USER_DATA_FOLDER — MUST be before any tokio /
    // thread spawn / tauri call. Pitfall #1, FOUND-05.
    webview_env::set_webview2_data_folder(paths.webview_data_dir())?;

    // Step 3: parse --self-test flag.
    let self_test = std::env::args().any(|a| a == "--self-test");

    // Step 4: load config (or use defaults).
    let config = AppConfig::load_or_default(paths.config_file())?;

    // (Step 5: tracing — Plan 05.)
    // (Steps 6-10: writer / migrations / reader pool / AppCtx — Plans 03/04.)

    // Step 11: self-test path — print diagnostics and exit.
    if self_test {
        eprintln!("trackly --self-test (Plan 02 placeholder)");
        eprintln!("paths resolved: exe_dir={}", paths.exe_dir().display());
        eprintln!("  db_path           = {}", paths.db_path().display());
        eprintln!("  config_file       = {}", paths.config_file().display());
        eprintln!(
            "  webview_data_dir  = {}",
            paths.webview_data_dir().display()
        );
        eprintln!("  logs_dir          = {}", paths.logs_dir().display());
        eprintln!("  is_portable       = {}", paths.is_portable());
        eprintln!(
            "config loaded: server.enabled={}, server.host={}, server.port={}",
            config.server.enabled, config.server.host, config.server.port
        );
        eprintln!(
            "  logging.level     = {}, format = {}",
            config.logging.level, config.logging.format
        );
        eprintln!("  organization.timezone = {}", config.organization.timezone);
        eprintln!(
            "Plan 02 placeholder — Plans 03/04 wire DB/migrations/AppCtx; \
             Plan 05/Phase 2 wires Tauri Builder."
        );
        return Ok(());
    }

    // Step 12: normal mode — UI not yet wired. Plan 04 will splice AppCtx and
    // tracing init; Plan 05/Phase 2 will replace this with tauri::Builder::run.
    eprintln!(
        "trackly Phase 1 Plan 02: UI not yet wired (paths={}, config loaded). \
         Run with --self-test for diagnostics.",
        paths.exe_dir().display()
    );
    Ok(())
}
