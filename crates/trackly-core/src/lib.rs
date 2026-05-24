//! trackly-core — pure domain layer.
//!
//! This crate holds domain entities, value objects, error types (`AppError`),
//! ports (traits) implemented by `trackly-infra`, and shared primitives
//! (`Secret<T>`, `Clock`). It MUST NOT depend on tokio, rusqlite, tauri,
//! axum, hyper, tower, reqwest, or sqlx — this invariant is enforced by
//! `tests/no_io_deps.rs`.
//!
//! Real modules (`domain`, `error`, `primitives`, `ports`) land in Plan 04.
#![forbid(unsafe_code)]
