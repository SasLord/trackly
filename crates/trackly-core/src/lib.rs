//! trackly-core — pure domain layer.
//!
//! This crate holds domain entities, value objects, error types (`AppError`),
//! ports (traits) implemented by `trackly-infra`, and shared primitives
//! (`Secret<T>`, `Clock`). It MUST NOT depend on tokio, rusqlite, tauri,
//! axum, hyper, tower, reqwest, or sqlx — this invariant is enforced by
//! `tests/no_io_deps.rs`.
//!
//! Plan 02 bootstraps `error` (minimal `AppError` enum with `Internal` +
//! `Validation` only); Plan 04 extends `error` and adds `domain`, `primitives`,
//! `ports`.
#![forbid(unsafe_code)]

pub mod error;
