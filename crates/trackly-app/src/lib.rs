//! trackly-app — composition root library surface.
//!
//! This crate hosts both the `trackly` binary (Tauri shell + AppCtx + axum
//! wiring in Phase 5) and a `lib` target so integration tests
//! (`tests/export_bindings.rs`, `tests/concurrent_writes.rs`,
//! `tests/downgrade_protection.rs`, `tests/health_smoke.rs` — created in
//! later plans) can import `trackly_app::*`.
//!
//! Real modules (`context`, `shutdown`, `logging`, `webview_env`, `dto`,
//! `error_axum`, `tauri_cmds`, `specta_export`) land in Plans 02, 04, 05.
