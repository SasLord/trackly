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
//! 4. `trackly_app::config_recovery::load_or_recover(...)` — read `trackly.config.toml`;
//!    NEVER propagates a fatal error (quick task 260804-lk0 — this used to be
//!    `AppConfig::load_or_default(...)?`, which silently killed the process pre-logger).
//!    Malformed/unreadable config falls back to `AppConfig::default()` and carries the
//!    error message forward for step 5b.
//! 5. `trackly_app::logging::init(&paths, &config)` — tracing-subscriber + tracing-appender
//!    daily rotation, using whichever config step 4 produced (real or recovered default);
//!    возвращает `WorkerGuard`, который дальше живёт внутри AppCtx (Pitfall #6).
//!
//! After step 5, if step 4 recovered from an error, it is surfaced now that logging exists:
//! `tracing::error!` + `config_recovery::write_config_error_file` (always, portable-mode
//! safe) + best-effort native dialog (desktop/interactive only, skipped for `--self-test`).
//!
//! 6. Build tokio multi-thread runtime; `block_on` async lifecycle:
//!    - 6a/b/c. `AppCtx::build` — probe-read user_version → writer open → migrations → writer worker → reader pool.
//! 7. Self-test branch: print diagnostics, drop AppCtx (which cancels shutdown + drops log_guard), exit 0.
//! 8. Normal branch: stub message (Plan 05/Phase 2 wires Tauri Builder).

use tauri::Emitter;
use trackly_app::context::AppCtx;
use trackly_app::server::rusqlite_session_store::RusqliteSessionStore;
use trackly_app::server::{start_server_on_addr, ServerHandle};
use trackly_app::services::run_supervisor;
use trackly_app::webview_env;
use trackly_infra::Paths;

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

    // Step 4: load config, fail-soft (quick task 260804-lk0). Never propagates a fatal `?`
    // for a malformed trackly.config.toml — that used to exit main() silently under
    // windows_subsystem="windows" (no console to print the Err to) BEFORE the logger
    // existed. Falls back to AppConfig::default() and carries the error message for
    // step 5b to surface once logging is up.
    let (config, config_load_error) =
        trackly_app::config_recovery::load_or_recover(paths.config_file());

    // Step 5: tracing-subscriber + tracing-appender (D-Logging-01). Файлы
    // ложатся в `<exe_dir>/logs/trackly.log.<YYYY-MM-DD>` (portable-mode
    // invariant). Возвращённый WorkerGuard живёт внутри AppCtx; пока он
    // не drop'нется, background-thread аппендера не остановится.
    let log_guard = trackly_app::logging::init(&paths, &config)?;

    // Step 5b: NOW that logging exists, surface a recovered config error loudly through
    // every channel available — log file (always), a portable-mode-safe config-error.txt
    // next to the exe (always, works headless too), and a best-effort native dialog
    // (desktop/interactive only; skipped for --self-test so CI never blocks on a dialog
    // nobody can dismiss).
    if let Some(err_msg) = config_load_error {
        tracing::error!(
            error = %err_msg,
            "trackly.config.toml failed to load — falling back to defaults"
        );
        if let Err(e) = trackly_app::config_recovery::write_config_error_file(&paths, &err_msg) {
            tracing::error!("failed to write config-error.txt: {e}");
        }
        if !self_test {
            trackly_app::config_recovery::show_best_effort_dialog(&err_msg);
        }
    } else {
        trackly_app::config_recovery::clear_config_error_file(&paths);
    }

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
            // Эффективные настройки: live app_settings (host/port/cert_path,
            // сохранённые через Настройки) поверх TOML-bootstrap. Root-cause
            // фикс server-bind-localhost-only — выбранный в UI `0.0.0.0` теперь
            // реально доходит до TcpListener::bind на старте, а не подменяется
            // дефолтным config.host=127.0.0.1.
            let net = trackly_app::http::settings::resolve_effective_network(&ctx).await?;
            let host = net.host.clone();
            let port = net.port;
            let cert_path = net.cert_path.clone();
            let key_path = net.key_path.clone();

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
        // Gap-closure fix: bridge HTTP/browser-originated WsEvents to the
        // desktop webview. Without this, the only Tauri `trackly-event` emits
        // were the direct `app.emit(...)` calls inside Tauri command handlers
        // themselves (tauri_cmds/requests.rs) — so a browser/LAN user's
        // mutation (e.g. POST /api/v1/request_ad_restore → service pushes
        // ws_broadcast) never reached the desktop admin's webview. Clone the
        // broadcast sender BEFORE `.manage(ctx)` consumes `ctx` (AppCtx is
        // Arc-based/Clone, so this is a cheap handle clone, not a duplicate
        // channel).
        let ws_broadcast = ctx.ws_broadcast.clone();

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

                // ws_broadcast → desktop webview bridge (D-Notify-01 gap-closure).
                // Mirrors the browser path's `ctx.ws_broadcast.subscribe()` in
                // http/ws.rs — every WsEvent pushed by any service mutation
                // (HTTP or Tauri-originated; both transports share the same
                // service layer) now also reaches the desktop window via the
                // same `trackly-event` channel the frontend's ws.ts already
                // listens on. Same serialized WsEvent payload — no
                // wrap/rename, so existing `event.type === '...'` handlers
                // keep working unchanged.
                //
                // WR-05 (visibility boundary): unlike the browser WS path
                // (`http/ws.rs`), this bridge does NOT call
                // `event.is_visible_to(&identity)` per event, and this is
                // intentional — not an oversight:
                //
                //   * The desktop shell only ever operates under an
                //     admin/manager-tier identity. In unlocked mode it is
                //     `Identity::trusted_admin()`; in locked mode it is the
                //     verified desktop admin (`AuthService::desktop_identity`).
                //     Every arm of `WsEvent::is_visible_to` already passes for
                //     Admin/Manager, so filtering here would be a no-op today.
                //
                //   * The correct desktop identity is resolved *per operation*
                //     via `resolve_tauri_identity` (it depends on the runtime
                //     `desktop_lock_enabled` setting and an async DB lookup, and
                //     can change while the app runs). Snapshotting a single
                //     identity once here, for the lifetime of this long-lived
                //     bridge task, would be stale-by-construction and is the
                //     wrong shape for a per-event gate.
                //
                // If the desktop shell is ever allowed to run under a non-admin
                // identity, OR a future `WsEvent` variant carries data not meant
                // for the desktop operator, this bridge MUST be changed to
                // resolve the live identity per event and gate on
                // `event.is_visible_to(&identity)` to match `http/ws.rs`.
                let app_handle = app.handle().clone();
                let mut rx = ws_broadcast.subscribe();
                tauri::async_runtime::spawn(async move {
                    loop {
                        match rx.recv().await {
                            Ok(event) => {
                                let _ = app_handle.emit("trackly-event", &event);
                            }
                            Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                                // Slow consumer — skipped n events. Continue,
                                // don't exit (parity with http/ws.rs Pitfall 5).
                                tracing::warn!(
                                    "ws_broadcast->tauri bridge lagged {n} events — continuing"
                                );
                            }
                            Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                                // Sender dropped — AppCtx shutting down.
                                break;
                            }
                        }
                    }
                });

                Ok(())
            })
            .run(tauri::generate_context!())
            .expect("tauri runtime failed");
        Ok::<(), anyhow::Error>(())
    })?;

    Ok(())
}
