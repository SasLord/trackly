//! trackly-app — composition root library surface.
//!
//! This crate hosts both the `trackly` binary (Tauri shell + AppCtx + axum
//! wiring in Phase 5) and a `lib` target so integration tests
//! (`tests/export_bindings.rs`, `tests/concurrent_writes.rs`,
//! `tests/downgrade_protection.rs`, `tests/health_smoke.rs` — created in
//! later plans) can import `trackly_app::*`.
//!
//! Plan 02 lands `webview_env` (and an empty `context` stub placeholder for
//! Plan 04 to fill). Plans 04/05 add `shutdown`, `logging`, `dto`,
//! `error_axum`, `tauri_cmds`, `specta_export`.

pub mod webview_env;

/// Composition-root context. Plan 04 fills this with `AppCtx { writer_tx,
/// reader_pool, paths, config, clock, shutdown }`.
pub mod context {}
