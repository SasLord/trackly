// Prevents an extra console window on Windows in release builds (GUI subsystem).
// Debug builds keep the console so `tracing` stdout is visible during dev.
// Logs always also go to ./logs/ next to the exe (portable discipline), so no
// diagnostics are lost when the console is hidden.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

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
use trackly_app::server::rusqlite_session_store::RusqliteSessionStore;
use trackly_app::server::{start_server_on_addr, ServerHandle};
use trackly_app::services::run_supervisor;
use trackly_app::webview_env;
use trackly_infra::{AppConfig, Paths};

fn main() -> anyhow::Result<()> {
    // Step 1: resolve all paths from current_exe().
    let paths = Paths::resolve()?;

    // Step 1b: install the process-level rustls CryptoProvider (ring) before
    // any TLS path can be reached. Both `ring` and `aws-lc-rs` are present in
    // the dependency graph (ldap3 pulls aws-lc-rs; rcgen/tokio-rustls pull
    // ring), so rustls 0.23 cannot auto-select a provider — without this,
    // `ServerConfig::builder()` panics at runtime (server-mode toggle / startup).
    trackly_app::server::tls::ensure_crypto_provider();

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

        // Phase 7 Plan 07: Spawn supervisor background task.
        tokio::spawn(run_supervisor(ctx.clone()));

        // Step 8a (Plan 05-03): Start axum HTTPS server if config.server.enabled.
        // Uses child CancellationToken (never cancels master AppCtx.shutdown — D-Server-01).
        // Server starts BEFORE tauri::Builder so it's ready for LAN connections immediately.
        if ctx.config.server.enabled {
            let server_config = &ctx.config.server;
            let host = server_config.host.clone();
            let port = server_config.port;
            let cert_path = server_config.cert_path.clone();
            let key_path = server_config.key_path.clone();

            // Build TLS bundle: load from cert/key files if cert_path provided, else
            // generate self-signed. Uses the same `tls::load_from_files` contract as
            // build_server_toggle (WR-01): the key path is resolved via
            // `Path::with_extension` (correct for .crt/.pem/.cer/.cert) or taken from an
            // explicit `server.key_path` — not a brittle string `.replace()` that would
            // silently keep the cert path for unusual extensions and read the wrong file.
            let tls_bundle = if cert_path.is_empty() {
                trackly_app::server::tls::generate_self_signed(&host)?
            } else {
                trackly_app::server::tls::load_from_files(&cert_path, &key_path)?
            };

            // Save generated cert/key to exe_dir for reuse.
            if cert_path.is_empty() {
                let exe_dir = ctx.paths.exe_dir();
                let _ = std::fs::write(exe_dir.join("server.crt"), &tls_bundle.cert_pem);
                let _ = std::fs::write(exe_dir.join("server.key"), &tls_bundle.key_pem);
            }

            // Build session store and router.
            let session_store = RusqliteSessionStore::new(ctx.writer.clone(), ctx.readers.clone());
            if let Err(e) = session_store.background_cleanup().await {
                tracing::warn!("session cleanup failed: {e}");
            }

            let router = trackly_app::http::build_router(&ctx, session_store);
            let addr: std::net::SocketAddr = format!("{host}:{port}").parse()?;
            let child_token = ctx.shutdown.child_token();
            let cancel = child_token.clone();

            let task = tokio::spawn(async move {
                if let Err(e) = start_server_on_addr(router, addr, tls_bundle.acceptor, child_token).await {
                    tracing::error!("server error: {e}");
                }
            });

            {
                let mut guard = ctx.server_ctl.lock().await;
                *guard = Some(ServerHandle { cancel, task });
            }

            tracing::info!("axum HTTPS server started on https://{}:{}", host, port);
        }

        // Step 8b (Plan 03): UI wired via Tauri Builder.
        // WriterHandle и ReaderPool из AppCtx::build уже инициализированы —
        // writer-worker крутится на dedicated thread, reader-pool использует
        // sync rusqlite::Connection через tokio::task::spawn_blocking.
        // tauri::Builder использует свой main-thread event loop (Wry/Tao) и
        // НЕ создаёт дополнительный tokio runtime; #[tauri::command] async-функции
        // выполняются на текущем tokio::Runtime через tauri::async_runtime integration.
        let builder = trackly_app::specta_export::builder();
        tauri::Builder::default()
            .plugin(tauri_plugin_single_instance::init(|_app, _args, _cwd| {
                tracing::info!("single-instance: second launch ignored");
            }))
            .plugin(tauri_plugin_dialog::init())
            // Phase 3.1 Plan 05 — G-8b PDF preview actions.
            .plugin(tauri_plugin_shell::init())
            .plugin(tauri_plugin_fs::init())
            // Phase 7 Plan 07 — app_restart command (D-19 DB move workflow).
            .plugin(tauri_plugin_process::init())
            .manage(ctx)
            .invoke_handler(builder.invoke_handler())
            .setup(move |app| {
                builder.mount_events(app);
                Ok(())
            })
            .run(tauri::generate_context!())
            .expect("tauri runtime failed");
        Ok::<(), anyhow::Error>(())
    })?;

    Ok(())
}
