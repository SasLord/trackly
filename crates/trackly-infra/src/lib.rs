//! trackly-infra — adapters and I/O.
//!
//! Holds: `paths.rs` (portable detection), `config.rs` (`trackly.config.toml`
//! parsing), `db/*` (PRAGMAs, reader pool, single-writer worker, refinery
//! migrations), `clock_impl.rs` (`SystemClock`), and `test_support/` helpers.
//!
//! Real modules land in Plans 02 (paths/config), 03 (migrations + pools),
//! and 04 (writer worker / `Clock` impl).
