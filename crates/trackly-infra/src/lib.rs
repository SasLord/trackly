//! trackly-infra — adapters and I/O.
//!
//! Holds: `paths.rs` (portable detection), `config.rs` (`trackly.config.toml`
//! parsing), `db/*` (PRAGMAs, reader pool, single-writer worker, refinery
//! migrations), `clock_impl.rs` (`SystemClock`), `error_conversions.rs`
//! (free-fn маппинг I/O-ошибок в `AppError`), и `test_support/` helpers.

pub mod ad;
pub mod clock_impl;
pub mod config;
pub mod db;
pub mod error_conversions;
pub mod paths;
pub mod repos;
pub mod snmp;
pub mod test_support;

pub use clock_impl::SystemClock;
pub use config::AppConfig;
pub use paths::Paths;
