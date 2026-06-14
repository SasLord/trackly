//! SNMP adapters for Trackly.
//!
//! Two implementations of the `SnmpClient` trait (from trackly-core):
//! - `RealSnmpClient` (`real.rs`): production impl using `snmp2::AsyncSession`.
//! - `MockSnmpClient` (`mock.rs`): deterministic fixtures for dev macOS (D-Mock-01).
//!
//! Runtime switching in `AppCtx::build`:
//! ```ignore
//! let snmp_client: Arc<dyn SnmpClient + Send + Sync> =
//!     if config.snmp.use_mock || std::env::var("TRACKLY_SNMP_MOCK").is_ok() {
//!         Arc::new(MockSnmpClient::default_fixtures())
//!     } else {
//!         Arc::new(RealSnmpClient)
//!     };
//! ```

pub mod mock;
pub mod real;
