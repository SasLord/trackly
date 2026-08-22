//! Port traits — abstractions for I/O adapters.
//!
//! Declaring traits here keeps trackly-core I/O-free: the trait lives in core,
//! the adapter (concrete impl) lives in trackly-infra.

pub mod acts;
pub mod ad;
pub mod ad_directory;
pub mod cartridges;
pub mod devices;
pub mod places;
pub mod printers;
pub mod requests;
pub mod snmp;
