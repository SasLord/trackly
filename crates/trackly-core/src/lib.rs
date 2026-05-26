//! trackly-core — pure domain layer.
//!
//! This crate holds domain entities, value objects, error types (`AppError`),
//! ports (traits) implemented by `trackly-infra`, and shared primitives
//! (`Secret<T>`, `Clock`). It MUST NOT depend on tokio, rusqlite, tauri,
//! axum, hyper, tower, reqwest, or sqlx — this invariant is enforced by
//! `tests/no_io_deps.rs`.
//!
//! Plan 02 bootstraps `error` (minimal `AppError`); Plan 04 extends `error`
//! to the full 9-variant D-AppError-01 enum and adds `primitives`
//! (`Secret<T>`, `Clock` trait).
#![forbid(unsafe_code)]

pub mod domain;
pub mod error;
pub mod ports;
pub mod primitives;

pub use error::AppError;
pub use primitives::{Clock, Secret};
