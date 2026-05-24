//! trackly-app — composition root library surface.
//!
//! Hosts the `trackly` binary (Tauri shell + AppCtx wiring; Phase 5 adds axum)
//! and a `lib` target so integration tests (`tests/concurrent_writes.rs`,
//! `tests/downgrade_protection.rs`, future `tests/health_smoke.rs`, etc.)
//! can import `trackly_app::*`.
//!
//! Plan 04 adds:
//! - `context` — `AppCtx { writer, readers, paths, config, clock, shutdown,
//!   log_guard, schema_version }` + `AppCtx::build(...).await`.
//! - `shutdown` — Ctrl-C → CancellationToken.cancel().
//! - `error_axum` — stub for Plan 05 HTTP error mapping.
//!
//! Plan 05 will add: `logging`, `dto`, `tauri_cmds`, `specta_export`.

pub mod context;
pub mod error_axum;
pub mod shutdown;
pub mod webview_env;
