//! Repository adapters — concrete implementations of port traits from trackly-core.

pub mod acts_sqlite;
pub mod audit_log_sqlite;
pub mod cartridges_sqlite;
pub mod devices_sqlite;
pub mod printers_sqlite;
pub mod requests_sqlite;

pub use acts_sqlite::SqliteActRepository;
pub use audit_log_sqlite::SqliteAuditLogRepository;
pub use cartridges_sqlite::SqliteCartridgeRepository;
pub use devices_sqlite::SqliteDeviceRepository;
pub use printers_sqlite::SqlitePrinterRepository;
pub use requests_sqlite::SqliteRequestRepository;
