//! trackly-infra — adapters and I/O.
//!
//! Holds: `paths.rs` (portable detection), `config.rs` (`trackly.config.toml`
//! parsing), `db/*` (PRAGMAs, reader pool, single-writer worker, refinery
//! migrations), `clock_impl.rs` (`SystemClock`), and `test_support/` helpers.
//!
//! Plan 02 lands `paths` and `config`; Plans 03/04 add `db/*` and
//! `clock_impl.rs`.

pub mod config;
pub mod paths;

pub use config::AppConfig;
pub use paths::Paths;
