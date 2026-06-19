//! AD adapters for Trackly (Phase 9, D-AD-01).
//!
//! Two implementations of the `AdClient` trait (from trackly-core):
//! - `RealAdClient` (`real.rs`): production impl using `ldap3` simple_bind over LDAPS.
//! - `MockAdClient` (`mock.rs`): deterministic fixtures for dev macOS (D-Mock-01).
//!
//! Plus `discovery.rs`: auto-detect domain/DC/base-DN (D-Config-01) — pure
//! base-DN derivation + an async env/DNS-SRV probe, never exercised on dev
//! macOS (no domain reachable; dev always runs `TRACKLY_AD_MOCK=1`).
//!
//! Runtime switching in `AppCtx::build` (mirrors snmp/mod.rs):
//! ```ignore
//! let use_mock = config.ad.use_mock || std::env::var("TRACKLY_AD_MOCK").is_ok();
//! let ad_client: Arc<dyn AdClient + Send + Sync> = if use_mock {
//!     Arc::new(MockAdClient::default_fixtures())
//! } else {
//!     Arc::new(RealAdClient::new(config.ad.clone()))
//! };
//! ```

pub mod discovery;
pub mod mock;
pub mod real;
