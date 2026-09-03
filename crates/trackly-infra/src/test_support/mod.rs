//! Test-support helpers — public, NOT `#[cfg(test)]`.
//!
//! Integration tests in `crates/trackly-infra/tests/*` (and downstream
//! consumers like `crates/trackly-app/tests/*`) cannot reach
//! `#[cfg(test)]`-gated items in the library crate; they need plain
//! `pub` items. This module is the canonical fixture API.

pub mod test_app_ctx;
pub mod test_db;

pub use test_app_ctx::{test_writer_and_readers, test_writer_and_readers_sized};
pub use test_db::test_db;
