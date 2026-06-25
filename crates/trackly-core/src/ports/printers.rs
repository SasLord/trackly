//! `PrinterRepository` port — repository trait for the Printers entity.
//!
//! Pattern: associated `type Conn` keeps rusqlite out of trackly-core.
//! Write methods that participate in larger transactions are NOT part of
//! this trait — they live as `*_in_tx` helpers on `SqlitePrinterRepository`
//! and are orchestrated by the service layer inside a single
//! `WriterHandle::execute` closure.

use crate::domain::printers::{
    OidProfileRow, Pagination, PrinterAlertRow, PrinterFilter, PrinterReadingRow, PrinterRow,
};
use crate::error::AppError;

/// Repository port for printers. Implemented by `SqlitePrinterRepository` in trackly-infra.
pub trait PrinterRepository {
    /// The connection type provided by the adapter (e.g. `rusqlite::Connection`).
    type Conn;

    /// Fetch a single printer by ID (with joined device columns).
    /// Returns `AppError::NotFound` if absent.
    fn get(&self, conn: &Self::Conn, id: i64) -> Result<PrinterRow, AppError>;

    /// Fetch a single printer by its device_id (FK into devices), not by the
    /// printers.id primary key.
    ///
    /// GAP-12-13 (Phase 12 Round 5): UI consistently passes device_id
    /// (PrinterSelect emits deviceId; requests carry printerDeviceId) — this
    /// resolves that contract directly instead of forcing callers through
    /// the id-keyed get(). Returns AppError::NotFound if absent.
    fn get_by_device_id(&self, conn: &Self::Conn, device_id: i64) -> Result<PrinterRow, AppError>;

    /// Paginated list of printers matching `filter`. Returns `(rows, total)`.
    fn list(
        &self,
        conn: &Self::Conn,
        filter: &PrinterFilter,
        page: &Pagination,
    ) -> Result<(Vec<PrinterRow>, u64), AppError>;

    /// Get the most recent reading for a printer, if any.
    fn get_last_reading(
        &self,
        conn: &Self::Conn,
        printer_id: i64,
    ) -> Result<Option<PrinterReadingRow>, AppError>;

    /// List all currently active (non-acknowledged) printer alerts.
    fn list_active_alerts(&self, conn: &Self::Conn) -> Result<Vec<PrinterAlertRow>, AppError>;

    /// List all OID profiles (seeded by V021, used for profile picker in UI).
    fn list_oid_profiles(&self, conn: &Self::Conn) -> Result<Vec<OidProfileRow>, AppError>;

    /// Find an OID profile by sysObjectID prefix matching.
    /// Longest-prefix match is recommended; RFC3805 (empty prefix) is the fallback.
    fn get_oid_profile_by_prefix(
        &self,
        conn: &Self::Conn,
        sys_object_id: &str,
    ) -> Result<Option<OidProfileRow>, AppError>;

    /// Return the cartridge ID currently installed in the given printer device
    /// (D-PRN07-01). Returns `None` if no cartridge is installed.
    ///
    /// Queries: `SELECT id FROM cartridges WHERE current_printer_device_id = ?1 LIMIT 1`
    fn current_cartridge_for_printer(
        &self,
        conn: &Self::Conn,
        printer_device_id: i64,
    ) -> Result<Option<i64>, AppError>;

    /// Returns cartridge_model_id list linked via `printer_cartridge_models`
    /// (D-11/D-12, Phase 12 gap closure); empty Vec means "not configured" —
    /// D-14, caller must not treat empty as a hard filter.
    fn get_compatible_model_ids(
        &self,
        conn: &Self::Conn,
        device_id: i64,
    ) -> Result<Vec<i64>, AppError>;

    /// Reverse lookup for the cartridge-model-side editor — devices linked
    /// to this model via `printer_cartridge_models`.
    fn get_compatible_device_ids(
        &self,
        conn: &Self::Conn,
        cartridge_model_id: i64,
    ) -> Result<Vec<i64>, AppError>;
}
