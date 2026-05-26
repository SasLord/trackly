//! Port traits — abstractions for I/O adapters.
//!
//! Declaring traits here keeps trackly-core I/O-free: the trait lives in core,
//! the adapter (concrete impl) lives in trackly-infra.

pub mod devices;
