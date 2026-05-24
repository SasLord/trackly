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
//! Plan 05 adds:
//! - `logging` — `tracing-subscriber` + `tracing-appender` daily rotation.
//! - `dto` — DTOs (Phase 1: `HealthDto`).
//! - `tauri_cmds` — `#[tauri::command]`-функции (Phase 1: `health`).
//! - `http` — axum роуты (Phase 1: `GET /api/v1/health`).
//! - `specta_export` — `tauri_specta::Builder` для генерации `ui/src/bindings.ts`.

pub mod context;
pub mod dto;
pub mod error_axum;
pub mod http;
pub mod logging;
pub mod shutdown;
pub mod specta_export;
pub mod tauri_cmds;
pub mod webview_env;
