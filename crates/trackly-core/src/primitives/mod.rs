//! Cross-cutting primitives used throughout the domain layer.
//!
//! - [`Secret`] — newtype wrapping sensitive values (passwords, tokens). Manual
//!   `Debug` writes `***`; `Drop` zeroizes the inner value. No `Serialize` /
//!   `Deserialize` derives (security invariant: secrets must be explicit in DTOs).
//! - [`Clock`] — trait abstracting "current UTC time"; production impl
//!   (`SystemClock`) lives in `trackly-infra` because `time::OffsetDateTime::now_utc()`
//!   is a runtime call we don't want to bake into pure-domain code.

pub mod clock;
pub mod secret;

pub use clock::Clock;
pub use secret::Secret;
