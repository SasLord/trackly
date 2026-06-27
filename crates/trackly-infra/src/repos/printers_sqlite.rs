//! SQLite adapter for `PrinterRepository` + tx-helper methods used by the
//! service layer to compose multi-step write paths inside a single transaction.
//!
//! All SQL is parameterised through `rusqlite::params![...]`. No user input is
//! ever concatenated into query strings — SQL injection is structurally impossible.

use rusqlite::{params, Connection, OptionalExtension, Transaction};
use trackly_core::domain::printers::{
    OidProfileRow, Pagination, PrinterAlertRow, PrinterFilter, PrinterNew, PrinterReadingRow,
    PrinterRow,
};
use trackly_core::error::AppError;
use trackly_core::ports::printers::PrinterRepository;

use crate::error_conversions::map_rusqlite;
use crate::repos::audit_log_sqlite::SqliteAuditLogRepository;

/// SQLite-backed printer repository adapter (zero-sized).
#[derive(Debug, Default, Clone)]
pub struct SqlitePrinterRepository;

/// SELECT with the column order expected by `map_row_printer`.
///
/// Joins:
///   - `devices d` for device_name.
///   - `locations l` for device_location (devices uses location_id FK).
const SELECT_PRINTERS: &str = "
    SELECT p.id, p.device_id, p.ip_address, p.snmp_version, p.vendor,
           p.oid_profile_id, p.last_seen_utc, p.usb_host_device_id,
           d.name AS device_name, l.name AS device_location,
           p.created_at_utc, p.updated_at_utc, p.version,
           (p.community <> 'public') AS community_configured
      FROM printers p
      LEFT JOIN devices d ON d.id = p.device_id
      LEFT JOIN locations l ON l.id = d.location_id
";

/// Maps a `SELECT_PRINTERS` row into `PrinterRow`.
fn map_row_printer(row: &rusqlite::Row<'_>) -> rusqlite::Result<PrinterRow> {
    Ok(PrinterRow {
        id: row.get(0)?,
        device_id: row.get(1)?,
        ip_address: row.get(2)?,
        snmp_version: row.get(3)?,
        vendor: row.get(4)?,
        oid_profile_id: row.get(5)?,
        last_seen_utc: row.get(6)?,
        usb_host_device_id: row.get(7)?,
        device_name: row.get(8)?,
        device_location: row.get(9)?,
        created_at_utc: row.get(10)?,
        updated_at_utc: row.get(11)?,
        version: row.get(12)?,
        // SQLite returns the boolean predicate as 0/1; rusqlite maps it to bool.
        community_configured: row.get(13)?,
    })
}

impl SqlitePrinterRepository {
    // -----------------------------------------------------------------------
    // Tx-helpers (NOT in trait — orchestrated by PrinterService)
    // -----------------------------------------------------------------------

    /// INSERT a new printer row inside a transaction.
    /// Returns the new printer `id`.
    pub fn create_in_tx(
        &self,
        tx: &Transaction<'_>,
        new: &PrinterNew,
        now_utc: i64,
    ) -> Result<i64, AppError> {
        tx.execute(
            "INSERT INTO printers \
             (device_id, ip_address, community, snmp_version, vendor, oid_profile_id, \
              usb_host_device_id, created_at_utc, updated_at_utc, version) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8, 1)",
            params![
                new.device_id,
                new.ip_address,
                new.community_raw,
                new.snmp_version,
                Option::<String>::None, // vendor detected at discovery
                new.oid_profile_id,
                new.usb_host_device_id,
                now_utc,
            ],
        )
        .map_err(map_rusqlite)?;
        Ok(tx.last_insert_rowid())
    }

    /// INSERT a reading snapshot for a printer.
    /// `toner_levels_json` maps to the DB column `toner_levels` (V022).
    pub fn upsert_reading_in_tx(
        &self,
        tx: &Transaction<'_>,
        printer_id: i64,
        ts_utc: i64,
        toner_levels_json: &str,
        page_count: Option<i64>,
        status: &str,
    ) -> Result<(), AppError> {
        tx.execute(
            "INSERT INTO printer_readings \
             (printer_id, ts_utc, toner_levels, page_count, status) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![printer_id, ts_utc, toner_levels_json, page_count, status],
        )
        .map_err(map_rusqlite)?;
        Ok(())
    }

    /// INSERT OR REPLACE an alert for a printer (UNIQUE constraint on printer_id).
    ///
    /// Uses `INSERT OR REPLACE` with the `UNIQUE(printer_id)` constraint on `printer_alerts`
    /// to ensure deduplication — a second call for the same printer updates `last_seen_utc`
    /// rather than inserting a duplicate (D-Alert-01).
    pub fn upsert_alert_in_tx(
        &self,
        tx: &Transaction<'_>,
        printer_id: i64,
        alert_type: &str,
        now_utc: i64,
    ) -> Result<(), AppError> {
        tx.execute(
            "INSERT INTO printer_alerts (printer_id, alert_type, first_seen_utc, last_seen_utc) \
             VALUES (?1, ?2, ?3, ?3) \
             ON CONFLICT(printer_id) DO UPDATE SET \
               alert_type = excluded.alert_type, \
               last_seen_utc = excluded.last_seen_utc",
            params![printer_id, alert_type, now_utc],
        )
        .map_err(map_rusqlite)?;
        Ok(())
    }

    /// UPDATE last_seen_utc on a printer row.
    pub fn update_last_seen_in_tx(
        &self,
        tx: &Transaction<'_>,
        printer_id: i64,
        now_utc: i64,
    ) -> Result<(), AppError> {
        tx.execute(
            "UPDATE printers SET last_seen_utc = ?1, updated_at_utc = ?2 WHERE id = ?3",
            params![now_utc, now_utc, printer_id],
        )
        .map_err(map_rusqlite)?;
        Ok(())
    }

    /// DELETE old readings (retention prune) + optional downsampling.
    ///
    /// Step 1: DELETE rows older than `retention_cutoff_utc`.
    /// Step 2: Downsample rows older than `downsample_cutoff_utc` keeping only
    ///         MIN(id) per (printer_id, day).
    pub fn prune_old_readings_in_tx(
        tx: &Transaction<'_>,
        retention_cutoff_utc: i64,
        downsample_cutoff_utc: i64,
    ) -> Result<u64, AppError> {
        // Step 1: delete rows beyond retention window.
        let deleted = tx
            .execute(
                "DELETE FROM printer_readings WHERE ts_utc < ?1",
                params![retention_cutoff_utc],
            )
            .map_err(map_rusqlite)? as u64;

        // Step 2: downsample between downsample cutoff and retention cutoff.
        tx.execute(
            "DELETE FROM printer_readings \
              WHERE ts_utc < ?1 \
                AND id NOT IN ( \
                  SELECT MIN(id) FROM printer_readings \
                   WHERE ts_utc < ?1 \
                   GROUP BY printer_id, date(ts_utc, 'unixepoch') \
                )",
            params![downsample_cutoff_utc],
        )
        .map_err(map_rusqlite)?;

        Ok(deleted)
    }

    /// UPDATE cartridges.current_printer_device_id to link/unlink a cartridge.
    ///
    /// Called from the CartridgeService install/return/writeoff path (D-PRN07-01).
    /// `printer_device_id = Some(id)` → link; `None` → unlink (cartridge removed from printer).
    pub fn set_current_cartridge_in_tx(
        tx: &Transaction<'_>,
        cartridge_id: i64,
        printer_device_id: Option<i64>,
        now_utc: i64,
    ) -> Result<(), AppError> {
        tx.execute(
            "UPDATE cartridges SET current_printer_device_id = ?1, updated_at_utc = ?2 \
             WHERE id = ?3",
            params![printer_device_id, now_utc, cartridge_id],
        )
        .map_err(map_rusqlite)?;
        Ok(())
    }

    /// Fetch a printer row inside an open transaction.
    pub fn fetch_in_tx(&self, tx: &Transaction<'_>, id: i64) -> Result<PrinterRow, AppError> {
        tx.query_row(
            &format!("{SELECT_PRINTERS} WHERE p.id = ?1"),
            params![id],
            map_row_printer,
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => AppError::NotFound {
                entity: "printer",
                id,
            },
            other => map_rusqlite(other),
        })
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    pub(crate) fn _audit_repo() -> SqliteAuditLogRepository {
        SqliteAuditLogRepository
    }
}

// ---------------------------------------------------------------------------
// PrinterRepository trait impl
// ---------------------------------------------------------------------------

impl PrinterRepository for SqlitePrinterRepository {
    type Conn = Connection;

    fn get(&self, conn: &Self::Conn, id: i64) -> Result<PrinterRow, AppError> {
        conn.query_row(
            &format!("{SELECT_PRINTERS} WHERE p.id = ?1"),
            params![id],
            map_row_printer,
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => AppError::NotFound {
                entity: "printer",
                id,
            },
            other => map_rusqlite(other),
        })
    }

    fn get_by_device_id(&self, conn: &Self::Conn, device_id: i64) -> Result<PrinterRow, AppError> {
        conn.query_row(
            &format!("{SELECT_PRINTERS} WHERE p.device_id = ?1"),
            params![device_id],
            map_row_printer,
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => AppError::NotFound {
                entity: "printer",
                id: device_id,
            },
            other => map_rusqlite(other),
        })
    }

    fn list(
        &self,
        conn: &Self::Conn,
        filter: &PrinterFilter,
        page: &Pagination,
    ) -> Result<(Vec<PrinterRow>, u64), AppError> {
        // R8/D-13: uncapped read — фронтенд (OperationModal) запрашивает до
        // 500 принтеров для селектора установки; сервер больше не капит ниже
        // фронтового запроса; полная пагинация — будущая фаза при росте
        // парка выше разумного объёма.
        let limit = page.limit as i64;
        let offset = page.offset as i64;

        // WR-05: filter on the printer's ACTUAL latest status, not merely
        // "has ever been polled". The displayed status (PrinterDto.status) is
        // the most-recent `printer_readings.status` row (PrinterService::get →
        // get_last_reading), so the filter compares the same value: a
        // correlated subquery picks the newest reading's status per printer and
        // matches it against `?1`. A printer with no readings yet has a NULL
        // latest status, so it never matches a non-NULL status filter (and is
        // always included when `?1 IS NULL`). The count and list queries share
        // this clause so paginated totals stay consistent with the page.
        const STATUS_FILTER: &str = "(?1 IS NULL OR ( \
            SELECT pr.status FROM printer_readings pr \
             WHERE pr.printer_id = p.id \
             ORDER BY pr.ts_utc DESC, pr.id DESC \
             LIMIT 1 \
        ) = ?1)";

        let total: i64 = conn
            .query_row(
                &format!("SELECT COUNT(*) FROM printers p WHERE {STATUS_FILTER}"),
                params![filter.status.as_deref()],
                |r| r.get(0),
            )
            .map_err(map_rusqlite)?;

        let mut stmt = conn
            .prepare(&format!(
                "{SELECT_PRINTERS} \
                 WHERE {STATUS_FILTER} \
                 ORDER BY p.id DESC \
                 LIMIT ?2 OFFSET ?3"
            ))
            .map_err(map_rusqlite)?;

        let rows = stmt
            .query_map(
                params![filter.status.as_deref(), limit, offset],
                map_row_printer,
            )
            .map_err(map_rusqlite)?;

        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(map_rusqlite)?);
        }
        Ok((out, total as u64))
    }

    fn get_last_reading(
        &self,
        conn: &Self::Conn,
        printer_id: i64,
    ) -> Result<Option<PrinterReadingRow>, AppError> {
        conn.query_row(
            "SELECT id, printer_id, ts_utc, COALESCE(toner_levels, '{}'), page_count, status \
               FROM printer_readings \
              WHERE printer_id = ?1 \
              ORDER BY ts_utc DESC \
              LIMIT 1",
            params![printer_id],
            |r| {
                Ok(PrinterReadingRow {
                    id: r.get(0)?,
                    printer_id: r.get(1)?,
                    ts_utc: r.get(2)?,
                    toner_levels_json: r.get(3)?,
                    page_count: r.get(4)?,
                    status: r.get(5)?,
                })
            },
        )
        .optional()
        .map_err(map_rusqlite)
    }

    fn list_active_alerts(&self, conn: &Self::Conn) -> Result<Vec<PrinterAlertRow>, AppError> {
        let mut stmt = conn
            .prepare(
                "SELECT id, printer_id, alert_type, first_seen_utc, last_seen_utc, \
                        acknowledged_at_utc \
                   FROM printer_alerts \
                  WHERE acknowledged_at_utc IS NULL \
                  ORDER BY last_seen_utc DESC",
            )
            .map_err(map_rusqlite)?;

        let rows = stmt
            .query_map([], |r| {
                Ok(PrinterAlertRow {
                    id: r.get(0)?,
                    printer_id: r.get(1)?,
                    alert_type: r.get(2)?,
                    first_seen_utc: r.get(3)?,
                    last_seen_utc: r.get(4)?,
                    acknowledged_at_utc: r.get(5)?,
                })
            })
            .map_err(map_rusqlite)?;

        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(map_rusqlite)?);
        }
        Ok(out)
    }

    fn list_oid_profiles(&self, conn: &Self::Conn) -> Result<Vec<OidProfileRow>, AppError> {
        let mut stmt = conn
            .prepare(
                "SELECT id, name, vendor_prefix, toner_level_oid, toner_max_oid, \
                        toner_encoding, page_counter_oid, status_oid, serial_oid \
                   FROM oid_profiles \
                  ORDER BY id ASC",
            )
            .map_err(map_rusqlite)?;

        let rows = stmt
            .query_map([], map_oid_profile_row)
            .map_err(map_rusqlite)?;

        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(map_rusqlite)?);
        }
        Ok(out)
    }

    fn get_oid_profile_by_prefix(
        &self,
        conn: &Self::Conn,
        sys_object_id: &str,
    ) -> Result<Option<OidProfileRow>, AppError> {
        // Longest-prefix match: find profile whose vendor_prefix is a prefix of sys_object_id.
        // RFC3805 (empty vendor_prefix) is the fallback.
        let mut stmt = conn
            .prepare(
                "SELECT id, name, vendor_prefix, toner_level_oid, toner_max_oid, \
                        toner_encoding, page_counter_oid, status_oid, serial_oid \
                   FROM oid_profiles \
                  WHERE ?1 LIKE (vendor_prefix || '%') OR vendor_prefix = '' \
                  ORDER BY LENGTH(vendor_prefix) DESC \
                  LIMIT 1",
            )
            .map_err(map_rusqlite)?;

        stmt.query_row(params![sys_object_id], map_oid_profile_row)
            .optional()
            .map_err(map_rusqlite)
    }

    fn current_cartridge_for_printer(
        &self,
        conn: &Self::Conn,
        printer_device_id: i64,
    ) -> Result<Option<i64>, AppError> {
        conn.query_row(
            "SELECT id FROM cartridges \
              WHERE current_printer_device_id = ?1 \
                AND deleted_at_utc IS NULL \
              ORDER BY updated_at_utc DESC \
              LIMIT 1",
            params![printer_device_id],
            |r| r.get::<_, i64>(0),
        )
        .optional()
        .map_err(map_rusqlite)
    }
}

fn map_oid_profile_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<OidProfileRow> {
    Ok(OidProfileRow {
        id: r.get(0)?,
        name: r.get(1)?,
        vendor_prefix: r.get(2)?,
        toner_level_oid: r.get(3)?,
        toner_max_oid: r.get(4)?,
        toner_encoding: r.get(5)?,
        page_counter_oid: r.get(6)?,
        status_oid: r.get(7)?,
        serial_oid: r.get(8)?,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migrations;
    use crate::db::pragmas::apply_writer_pragmas;
    use tempfile::TempDir;

    fn fresh_conn() -> (Connection, TempDir) {
        let dir = TempDir::new().expect("tempdir");
        let path = dir.path().join("printer-repo-test.db");
        let mut conn = Connection::open(&path).expect("open");
        apply_writer_pragmas(&conn).expect("writer pragmas");
        migrations::run(&mut conn).expect("migrations");
        (conn, dir)
    }

    /// Seed a device row, return its id.
    fn seed_device(conn: &mut Connection) -> i64 {
        let now = 1_700_000_000_i64;
        conn.execute(
            "INSERT INTO devices (type_id, name, status_id, created_at_utc, updated_at_utc, version) \
             VALUES (2, 'Test Printer', 1, ?1, ?1, 1)",
            params![now],
        )
        .expect("insert device");
        conn.last_insert_rowid()
    }

    #[test]
    fn test_printer_repo_create_and_get() {
        let (mut conn, _g) = fresh_conn();
        let device_id = seed_device(&mut conn);
        let repo = SqlitePrinterRepository;
        let now = 1_700_000_000_i64;

        let printer_new = PrinterNew {
            device_id,
            ip_address: Some("192.168.1.100".to_string()),
            community_raw: "public".to_string(),
            snmp_version: "v2c".to_string(),
            oid_profile_id: None,
            usb_host_device_id: None,
        };

        let id = {
            let tx = conn.transaction().expect("tx");
            let id = repo.create_in_tx(&tx, &printer_new, now).expect("create");
            tx.commit().expect("commit");
            id
        };

        let row = repo.get(&conn, id).expect("get");
        assert_eq!(row.device_id, device_id);
        assert_eq!(row.ip_address.as_deref(), Some("192.168.1.100"));
        assert_eq!(row.snmp_version, "v2c");
        assert_eq!(row.version, 1);
    }

    #[test]
    fn test_printer_usb_only() {
        let (mut conn, _g) = fresh_conn();
        // Seed two devices: printer device + USB host device
        let device_id = seed_device(&mut conn);
        let host_device_id = seed_device(&mut conn);
        let repo = SqlitePrinterRepository;
        let now = 1_700_000_000_i64;

        // The CHECK constraint requires ip_address IS NOT NULL OR usb_host_device_id IS NOT NULL
        let printer_new = PrinterNew {
            device_id,
            ip_address: None, // USB-only, no IP
            community_raw: "public".to_string(),
            snmp_version: "v2c".to_string(),
            oid_profile_id: None,
            usb_host_device_id: Some(host_device_id),
        };

        let id = {
            let tx = conn.transaction().expect("tx");
            let id = repo
                .create_in_tx(&tx, &printer_new, now)
                .expect("create usb printer");
            tx.commit().expect("commit");
            id
        };

        let row = repo.get(&conn, id).expect("get usb printer");
        assert!(
            row.ip_address.is_none(),
            "IP should be None for USB-only printer"
        );
        assert_eq!(
            row.usb_host_device_id,
            Some(host_device_id),
            "usb_host_device_id should be set"
        );
    }

    #[test]
    fn test_printer_no_ip_no_usb() {
        // GAP-12-08 (UAT round 2, A5) regression: before V030, this same
        // call returned a rusqlite CHECK constraint error because V020's
        // CHECK(ip_address IS NOT NULL OR usb_host_device_id IS NOT NULL)
        // wrongly required at least one connectivity method. IP is optional
        // per requirements — a printer may be created as a plain inventory
        // record before SNMP/USB wiring is configured.
        let (mut conn, _g) = fresh_conn();
        let device_id = seed_device(&mut conn);
        let repo = SqlitePrinterRepository;
        let now = 1_700_000_000_i64;

        let printer_new = PrinterNew {
            device_id,
            ip_address: None, // no SNMP IP
            community_raw: "public".to_string(),
            snmp_version: "v2c".to_string(),
            oid_profile_id: None,
            usb_host_device_id: None, // no USB host either
        };

        let id = {
            let tx = conn.transaction().expect("tx");
            let id = repo
                .create_in_tx(&tx, &printer_new, now)
                .expect("create printer without IP and without USB");
            tx.commit().expect("commit");
            id
        };

        let row = repo.get(&conn, id).expect("get printer without IP/USB");
        assert!(
            row.ip_address.is_none(),
            "ip_address should be None when not configured"
        );
        assert!(
            row.usb_host_device_id.is_none(),
            "usb_host_device_id should be None when not configured"
        );
    }

    /// WR-05 regression: `list`'s status filter must match the printer's
    /// ACTUAL latest reading status, not merely "ever polled". Before the fix
    /// the clause was `?1 IS NULL OR p.last_seen_utc IS NOT NULL`, which
    /// ignored the requested status value entirely (a silent no-op).
    #[test]
    fn test_printer_list_status_filter() {
        let (mut conn, _g) = fresh_conn();
        let repo = SqlitePrinterRepository;
        let now = 1_700_000_000_i64;

        // Two printers: one whose latest reading is "ok", one "error".
        let make_printer = |conn: &mut Connection, status: &str| -> i64 {
            let device_id = seed_device(conn);
            let printer_id = {
                let tx = conn.transaction().expect("tx");
                let id = repo
                    .create_in_tx(
                        &tx,
                        &PrinterNew {
                            device_id,
                            ip_address: Some("192.168.1.1".to_string()),
                            community_raw: "public".to_string(),
                            snmp_version: "v2c".to_string(),
                            oid_profile_id: None,
                            usb_host_device_id: None,
                        },
                        now,
                    )
                    .expect("create printer");
                tx.commit().expect("commit");
                id
            };
            // Older reading with a different status to prove "latest wins".
            {
                let tx = conn.transaction().expect("tx");
                repo.upsert_reading_in_tx(&tx, printer_id, now - 100, "{}", None, "warning")
                    .expect("old reading");
                repo.upsert_reading_in_tx(&tx, printer_id, now, "{}", None, status)
                    .expect("latest reading");
                tx.commit().expect("commit");
            }
            printer_id
        };

        let ok_printer = make_printer(&mut conn, "ok");
        let error_printer = make_printer(&mut conn, "error");

        let filter = |conn: &Connection, status: Option<&str>| -> (Vec<i64>, u64) {
            let f = PrinterFilter {
                status: status.map(|s| s.to_string()),
                search: None,
            };
            let (rows, total) = repo.list(conn, &f, &Pagination::default()).expect("list");
            (rows.into_iter().map(|r| r.id).collect(), total)
        };

        // status = "ok" → only the ok printer.
        let (ids, total) = filter(&conn, Some("ok"));
        assert_eq!(total, 1, "exactly one printer has latest status 'ok'");
        assert_eq!(ids, vec![ok_printer], "filter 'ok' must return ok printer");

        // status = "error" → only the error printer.
        let (ids, total) = filter(&conn, Some("error"));
        assert_eq!(total, 1, "exactly one printer has latest status 'error'");
        assert_eq!(
            ids,
            vec![error_printer],
            "filter 'error' must return error printer"
        );

        // status = "offline" (no printer matches) → empty, NOT all rows.
        let (ids, total) = filter(&conn, Some("offline"));
        assert_eq!(total, 0, "no printer has latest status 'offline'");
        assert!(ids.is_empty(), "filter 'offline' must return nothing");

        // status = None → both printers.
        let (ids, total) = filter(&conn, None);
        assert_eq!(total, 2, "no filter → all printers");
        assert!(ids.contains(&ok_printer) && ids.contains(&error_printer));

        // A printer with NO readings is excluded by any non-NULL status filter
        // (latest status is NULL) but included when status is None.
        let _unpolled = {
            let device_id = seed_device(&mut conn);
            let tx = conn.transaction().expect("tx");
            let id = repo
                .create_in_tx(
                    &tx,
                    &PrinterNew {
                        device_id,
                        ip_address: Some("192.168.1.9".to_string()),
                        community_raw: "public".to_string(),
                        snmp_version: "v2c".to_string(),
                        oid_profile_id: None,
                        usb_host_device_id: None,
                    },
                    now,
                )
                .expect("create unpolled printer");
            tx.commit().expect("commit");
            id
        };
        let (_ids, total_ok) = filter(&conn, Some("ok"));
        assert_eq!(
            total_ok, 1,
            "unpolled printer must not match a status filter"
        );
        let (_ids, total_all) = filter(&conn, None);
        assert_eq!(
            total_all, 3,
            "unpolled printer is included when status is None"
        );
    }

    #[test]
    fn test_prune_old_readings() {
        let (mut conn, _g) = fresh_conn();
        let device_id = seed_device(&mut conn);
        let repo = SqlitePrinterRepository;
        let now = 1_700_000_000_i64;

        // Create printer
        let printer_id = {
            let tx = conn.transaction().expect("tx");
            let id = repo
                .create_in_tx(
                    &tx,
                    &PrinterNew {
                        device_id,
                        ip_address: Some("192.168.1.1".to_string()),
                        community_raw: "public".to_string(),
                        snmp_version: "v2c".to_string(),
                        oid_profile_id: None,
                        usb_host_device_id: None,
                    },
                    now,
                )
                .expect("create");
            tx.commit().expect("commit");
            id
        };

        // Insert old reading (ts < now - 1)
        let old_ts = now - 100;
        {
            let tx = conn.transaction().expect("tx");
            repo.upsert_reading_in_tx(&tx, printer_id, old_ts, "{}", None, "ok")
                .expect("insert old reading");
            tx.commit().expect("commit");
        }

        // Insert recent reading
        {
            let tx = conn.transaction().expect("tx");
            repo.upsert_reading_in_tx(&tx, printer_id, now, "{}", None, "ok")
                .expect("insert recent reading");
            tx.commit().expect("commit");
        }

        let count_before: i64 = conn
            .query_row("SELECT COUNT(*) FROM printer_readings", [], |r| r.get(0))
            .expect("count");
        assert_eq!(count_before, 2);

        // Prune with cutoff = now - 1 (should delete old_ts row)
        {
            let tx = conn.transaction().expect("tx");
            let deleted = SqlitePrinterRepository::prune_old_readings_in_tx(&tx, now - 1, now - 50)
                .expect("prune");
            tx.commit().expect("commit");
            assert!(deleted >= 1, "should have deleted at least 1 row");
        }

        let count_after: i64 = conn
            .query_row("SELECT COUNT(*) FROM printer_readings", [], |r| r.get(0))
            .expect("count after");
        assert!(
            count_after < count_before,
            "should have fewer readings after prune"
        );
    }

    #[test]
    fn test_upsert_alert_dedup() {
        let (mut conn, _g) = fresh_conn();
        let device_id = seed_device(&mut conn);
        let repo = SqlitePrinterRepository;
        let now = 1_700_000_000_i64;

        let printer_id = {
            let tx = conn.transaction().expect("tx");
            let id = repo
                .create_in_tx(
                    &tx,
                    &PrinterNew {
                        device_id,
                        ip_address: Some("192.168.1.1".to_string()),
                        community_raw: "public".to_string(),
                        snmp_version: "v2c".to_string(),
                        oid_profile_id: None,
                        usb_host_device_id: None,
                    },
                    now,
                )
                .expect("create");
            tx.commit().expect("commit");
            id
        };

        // First upsert
        {
            let tx = conn.transaction().expect("tx");
            repo.upsert_alert_in_tx(&tx, printer_id, "error", now)
                .expect("first upsert");
            tx.commit().expect("commit");
        }

        // Second upsert (same printer_id) — must NOT create duplicate
        {
            let tx = conn.transaction().expect("tx");
            repo.upsert_alert_in_tx(&tx, printer_id, "offline", now + 60)
                .expect("second upsert");
            tx.commit().expect("commit");
        }

        let alert_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM printer_alerts WHERE printer_id = ?1",
                params![printer_id],
                |r| r.get(0),
            )
            .expect("count alerts");
        assert_eq!(
            alert_count, 1,
            "UNIQUE(printer_id) — must be exactly 1 alert row"
        );
    }

    #[test]
    fn test_current_cartridge_for_printer_repo() {
        let (mut conn, _g) = fresh_conn();
        let device_id = seed_device(&mut conn);
        let repo = SqlitePrinterRepository;
        let now = 1_700_000_000_i64;

        // Create a printer device for link
        let printer_id = {
            let tx = conn.transaction().expect("tx");
            let id = repo
                .create_in_tx(
                    &tx,
                    &PrinterNew {
                        device_id,
                        ip_address: Some("192.168.1.1".to_string()),
                        community_raw: "public".to_string(),
                        snmp_version: "v2c".to_string(),
                        oid_profile_id: None,
                        usb_host_device_id: None,
                    },
                    now,
                )
                .expect("create printer");
            tx.commit().expect("commit");
            id
        };

        // None initially
        let result = repo
            .current_cartridge_for_printer(&conn, device_id)
            .expect("query");
        assert!(result.is_none(), "no cartridge installed initially");

        // Create a cartridge model and cartridge
        let model_id = {
            conn.execute(
                "INSERT INTO cartridge_models (brand, model, kind_id, created_at_utc, updated_at_utc, version) \
                 VALUES ('Pantum', 'TL-5120X', 1, ?1, ?1, 1)",
                params![now],
            )
            .expect("insert model");
            conn.last_insert_rowid()
        };

        let cartridge_id = {
            conn.execute(
                "INSERT INTO cartridges (code, model_id, status_id, created_at_utc, updated_at_utc, version) \
                 VALUES ('C-000001', ?1, 2, ?2, ?2, 1)",
                params![model_id, now],
            )
            .expect("insert cartridge");
            conn.last_insert_rowid()
        };

        // Link cartridge to printer (printer device_id)
        {
            let tx = conn.transaction().expect("tx");
            SqlitePrinterRepository::set_current_cartridge_in_tx(
                &tx,
                cartridge_id,
                Some(device_id),
                now,
            )
            .expect("set_current_cartridge");
            tx.commit().expect("commit");
        }

        let result2 = repo
            .current_cartridge_for_printer(&conn, device_id)
            .expect("query after link");
        assert_eq!(
            result2,
            Some(cartridge_id),
            "should find the linked cartridge"
        );

        // Unlink
        {
            let tx = conn.transaction().expect("tx");
            SqlitePrinterRepository::set_current_cartridge_in_tx(&tx, cartridge_id, None, now)
                .expect("unlink");
            tx.commit().expect("commit");
        }

        let result3 = repo
            .current_cartridge_for_printer(&conn, device_id)
            .expect("query after unlink");
        assert!(result3.is_none(), "no cartridge after unlink");

        // keep printer_id used to avoid unused variable warning
        let _ = printer_id;
    }

    /// R8/D-13 regression: `list()` must NOT cap at 200 rows — the frontend
    /// (OperationModal) requests up to 500 printers for the install selector.
    /// Seeds 250 printers (above the old cap) and asserts `list()` with
    /// `page.limit = 500` returns ALL of them, including the printer with the
    /// highest seeded id (previously cut off by `ORDER BY p.id DESC LIMIT 200`).
    #[test]
    fn list_returns_all_printers_above_old_cap() {
        let (mut conn, _g) = fresh_conn();
        let repo = SqlitePrinterRepository;
        let now = 1_700_000_000_i64;

        const SEED_COUNT: i64 = 250;
        let mut last_printer_id: Option<i64> = None;
        for i in 0..SEED_COUNT {
            let device_id = seed_device(&mut conn);
            let tx = conn.transaction().expect("tx");
            let id = repo
                .create_in_tx(
                    &tx,
                    &PrinterNew {
                        device_id,
                        ip_address: Some(format!("192.168.1.{}", i % 250)),
                        community_raw: "public".to_string(),
                        snmp_version: "v2c".to_string(),
                        oid_profile_id: None,
                        usb_host_device_id: None,
                    },
                    now,
                )
                .expect("create printer");
            tx.commit().expect("commit");
            last_printer_id = Some(id);
        }

        let filter = PrinterFilter {
            status: None,
            search: None,
        };
        let page = Pagination {
            offset: 0,
            limit: 500,
        };
        let (rows, total) = repo.list(&conn, &filter, &page).expect("list");

        assert_eq!(
            total, SEED_COUNT as u64,
            "total count must reflect all seeded printers, not capped"
        );
        assert_eq!(
            rows.len(),
            SEED_COUNT as usize,
            "list() must return all seeded printers without the old .min(200) cap"
        );
        let max_id = last_printer_id.expect("seeded at least one printer");
        assert!(
            rows.iter().any(|r| r.id == max_id),
            "printer with the highest seeded id must be present (no ORDER BY ... LIMIT 200 cutoff)"
        );
    }
}
