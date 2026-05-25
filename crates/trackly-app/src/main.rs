//! `trackly` binary — Phase 1 full ordered lifecycle (Plan 04).
//!
//! Ordering invariant (RESEARCH §Code Example 1 + Pitfall #1):
//! 1. `Paths::resolve()` — root all I/O on `current_exe()?.parent()?`.
//! 2. `set_webview2_data_folder()` — MUST be before any tokio runtime / thread spawn / tauri::Builder.
//! 3. Parse `--self-test` flag.
//! 4. `AppConfig::load_or_default()` — read `trackly.config.toml` or use defaults.
//! 5. `trackly_app::logging::init(&paths, &config)` — tracing-subscriber + tracing-appender daily rotation;
//!    возвращает `WorkerGuard`, который дальше живёт внутри AppCtx (Pitfall #6).
//! 6. Build tokio multi-thread runtime; `block_on` async lifecycle:
//!    - 6a/b/c. `AppCtx::build` — probe-read user_version → writer open → migrations → writer worker → reader pool.
//! 7. Self-test branch: print diagnostics, drop AppCtx (which cancels shutdown + drops log_guard), exit 0.
//! 8. Normal branch: stub message (Plan 05/Phase 2 wires Tauri Builder).

use trackly_app::context::AppCtx;
use trackly_app::webview_env;
use trackly_infra::{AppConfig, Paths};

fn main() -> anyhow::Result<()> {
    // Step 1: resolve all paths from current_exe().
    let paths = Paths::resolve()?;

    // Step 2: set WEBVIEW2_USER_DATA_FOLDER — MUST be before any tokio / thread / tauri.
    webview_env::set_webview2_data_folder(paths.webview_data_dir())?;

    // Step 3: parse --self-test flag.
    let self_test = std::env::args().any(|a| a == "--self-test");

    // Step 4: load config (or defaults).
    let config = AppConfig::load_or_default(paths.config_file())?;

    // Step 5: tracing-subscriber + tracing-appender (D-Logging-01). Файлы
    // ложатся в `<exe_dir>/logs/trackly.log.<YYYY-MM-DD>` (portable-mode
    // invariant). Возвращённый WorkerGuard живёт внутри AppCtx; пока он
    // не drop'нется, background-thread аппендера не остановится.
    let log_guard = trackly_app::logging::init(&paths, &config)?;

    // Step 6: build a multi-thread tokio runtime and run AppCtx::build.
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    rt.block_on(async move {
        let ctx = AppCtx::build(paths, config, log_guard).await?;

        if self_test {
            // Step 7: self-test branch — exercise the writer worker + reader pool so
            // Plan 06's ProcMon-check captures every realistic file-access pattern
            // (WAL append, reader query, log rotation). After Plan 06 lands, this is
            // the canonical fixture for proving zero APPDATA leakage.
            ctx.writer
                .execute(|c| {
                    c.execute(
                        "CREATE TABLE IF NOT EXISTS __self_test (id INTEGER PRIMARY KEY, ts INTEGER NOT NULL)",
                        [],
                    )
                    .map_err(|e| trackly_core::error::AppError::Internal {
                        source_chain: format!("self-test CREATE TABLE: {e}"),
                    })?;
                    c.execute("INSERT INTO __self_test (ts) VALUES (?1)", [42_i64])
                        .map(|_| ())
                        .map_err(|e| trackly_core::error::AppError::Internal {
                            source_chain: format!("self-test INSERT: {e}"),
                        })
                })
                .await?;

            let count: i64 = {
                let readers = ctx.readers.clone();
                tokio::task::spawn_blocking(move || {
                    let conn = readers.acquire();
                    conn.query_row("SELECT COUNT(*) FROM __self_test", [], |r| r.get(0))
                })
                .await
                .map_err(|e| anyhow::anyhow!("spawn_blocking join: {e}"))?
                .map_err(|e| anyhow::anyhow!("self-test SELECT COUNT: {e}"))?
            };
            assert!(
                count >= 1,
                "self-test write did not become visible to reader (count={count})"
            );

            tracing::info!(
                schema_version = ctx.schema_version,
                count,
                "self-test OK"
            );
            eprintln!(
                "self-test OK: schema_version={}, portable={}",
                ctx.schema_version,
                ctx.paths.is_portable()
            );
            eprintln!("  exe_dir          = {}", ctx.paths.exe_dir().display());
            eprintln!("  db_path          = {}", ctx.paths.db_path().display());
            eprintln!("  config_file      = {}", ctx.paths.config_file().display());
            eprintln!(
                "  webview_data_dir = {}",
                ctx.paths.webview_data_dir().display()
            );
            eprintln!("  logs_dir         = {}", ctx.paths.logs_dir().display());
            eprintln!(
                "  server.enabled={}, server.host={}, server.port={}",
                ctx.config.server.enabled, ctx.config.server.host, ctx.config.server.port
            );
            eprintln!(
                "  logging.level={}, format={}, retention_days={}",
                ctx.config.logging.level,
                ctx.config.logging.format,
                ctx.config.logging.retention_days
            );
            eprintln!(
                "  organization.timezone={}",
                ctx.config.organization.timezone
            );
            // Cancel shutdown token (Phase 5+ background tasks will observe this).
            ctx.shutdown.cancel();
            // ctx drops here → WriterHandle drops → mpsc::Sender drops → worker exits.
            return Ok::<(), anyhow::Error>(());
        }

        // Step 8: normal branch — UI not yet wired in Phase 1.
        eprintln!(
            "Phase 1 — UI not yet wired. Use `trackly --self-test`. (schema_version={})",
            ctx.schema_version
        );
        ctx.shutdown.cancel();
        Ok::<(), anyhow::Error>(())
    })?;

    Ok(())
}
