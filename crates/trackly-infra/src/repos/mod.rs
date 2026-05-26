//! Repository adapters — concrete implementations of port traits from trackly-core.

pub mod devices_sqlite;

pub use devices_sqlite::SqliteDeviceRepository;
