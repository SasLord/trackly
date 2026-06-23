//! `PrinterService` — application service for printer SNMP monitoring lifecycle.
//!
//! Single-writer discipline: every mutation goes through
//! `WriterHandle::execute(closure)` with a `BEGIN IMMEDIATE` transaction.
//!
//! SNMP polling (D-Poll-01):
//!   - `poll_all()` polls every known printer sequentially.
//!   - `poll_single(id)` polls one printer on-demand.
//!   - `run_poll_task()` owns the background ticker + on-demand channel.
//!
//! Alert detection (D-Alert-01):
//!   - `detect_alert_type(status)` maps "error"|"offline" → Some(alert_type).
//!   - `upsert_alert()` calls `upsert_alert_in_tx()` — UNIQUE dedup enforced in SQL.
//!
//! Retention (D-Retention-01):
//!   - `prune_old_readings()` deletes readings older than 90 days.

use std::sync::Arc;

use serde_json::json;
use tokio_util::sync::CancellationToken;
use trackly_core::auth::{authorize, Action, Identity};
use trackly_core::domain::printers::{Pagination, PrinterFilter, PrinterNew};
use trackly_core::error::AppError;
use trackly_core::ports::printers::PrinterRepository;
use trackly_core::ports::snmp::SnmpClient;
use trackly_core::primitives::clock::Clock;
use trackly_infra::db::{pools::ReaderPool, writer_worker::WriterHandle};
use trackly_infra::error_conversions::map_rusqlite;
use trackly_infra::repos::audit_log_sqlite::{AuditEntry, SqliteAuditLogRepository};
use trackly_infra::repos::printers_sqlite::SqlitePrinterRepository;

use crate::dto::printer::{
    DiscoveredPrinterDto, PrinterCreateDto, PrinterDto, PrinterListResponse, WsEvent,
};

// ---------------------------------------------------------------------------
// Module-level helpers
// ---------------------------------------------------------------------------

/// Parse a toner level value from a printer SNMP reading.
///
/// Returns `Some(percent)` in range 0–100, or `None` for unknown/error states.
///
/// - `encoding = "percent"` → value is already a percentage (Pantum).
/// - `encoding = "level_over_max"` → value = level/max × 100 (Kyocera, HP, Canon).
///
/// Negative values (e.g. −2 = unknown) → `None`.
pub fn parse_toner_level(level: i64, max: i64, encoding: &str) -> Option<u8> {
    if level < 0 {
        return None;
    }
    let pct = match encoding {
        "percent" => level as u8,
        "level_over_max" => {
            if max <= 0 {
                return None;
            }
            ((level as f64 / max as f64) * 100.0).round() as u8
        }
        _ => return None,
    };
    Some(pct.min(100))
}

/// Map a printer status string to an alert type string.
///
/// Returns `Some("error")` or `Some("offline")` for alert-worthy statuses,
/// `None` for normal/warning states.
pub fn detect_alert_type(status: &str) -> Option<&'static str> {
    match status {
        "error" => Some("error"),
        "offline" => Some("offline"),
        _ => None,
    }
}

/// Map a sysObjectID prefix to a vendor name (D-OID-01, PRN-01).
///
/// Uses hardcoded prefix table matching the oid_profiles seed (V021).
/// Returns the vendor name string or `None` for unknown OIDs.
pub fn identify_vendor(sys_object_id: &str) -> Option<&'static str> {
    // Longest-prefix match — order matters (more specific first).
    let vendor_map: &[(&str, &str)] = &[
        ("1.3.6.1.4.1.40093", "pantum"),
        ("1.3.6.1.4.1.1347", "kyocera"),
        ("1.3.6.1.4.1.11", "hp"),
        ("1.3.6.1.4.1.1602", "canon"),
    ];
    for (prefix, vendor) in vendor_map {
        if sys_object_id.starts_with(prefix) {
            return Some(vendor);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// PrinterService
// ---------------------------------------------------------------------------

/// Application service for printer SNMP monitoring and management.
/// `Arc`-fields keep `Clone` O(1).
#[derive(Clone)]
pub struct PrinterService {
    pub writer: Arc<WriterHandle>,
    pub readers: Arc<ReaderPool>,
    pub(crate) clock: Arc<dyn Clock + Send + Sync>,
    pub(crate) printer_repo: Arc<SqlitePrinterRepository>,
    pub(crate) audit_repo: Arc<SqliteAuditLogRepository>,
    pub(crate) snmp_client: Arc<dyn SnmpClient + Send + Sync>,
    /// Channel for on-demand single-printer poll requests (D-Poll-01).
    pub(crate) poll_tx: tokio::sync::mpsc::Sender<i64>,
    /// WS broadcast sender (D-Notify-01).
    pub(crate) ws_tx: Arc<tokio::sync::broadcast::Sender<WsEvent>>,
}

impl PrinterService {
    pub fn new(
        writer: Arc<WriterHandle>,
        readers: Arc<ReaderPool>,
        clock: Arc<dyn Clock + Send + Sync>,
        snmp_client: Arc<dyn SnmpClient + Send + Sync>,
        poll_tx: tokio::sync::mpsc::Sender<i64>,
        ws_tx: Arc<tokio::sync::broadcast::Sender<WsEvent>>,
    ) -> Self {
        Self {
            writer,
            readers,
            clock,
            printer_repo: Arc::new(SqlitePrinterRepository),
            audit_repo: Arc::new(SqliteAuditLogRepository),
            snmp_client,
            poll_tx,
            ws_tx,
        }
    }

    // -----------------------------------------------------------------------
    // Read paths
    // -----------------------------------------------------------------------

    /// List printers (paginated).
    pub async fn list(
        &self,
        filter: PrinterFilter,
        page: Pagination,
    ) -> Result<PrinterListResponse, AppError> {
        let readers = self.readers.clone();
        let repo = self.printer_repo.clone();
        tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            let (rows, total) = repo.list(&conn, &filter, &page)?;
            let items = rows.into_iter().map(PrinterDto::from).collect();
            Ok(PrinterListResponse {
                items,
                total: total as i64,
            })
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking: {e}"),
        })?
    }

    /// Get a single printer by ID, enriched with last reading + alert + current cartridge.
    pub async fn get(&self, id: i64) -> Result<PrinterDto, AppError> {
        let readers = self.readers.clone();
        let repo = self.printer_repo.clone();
        tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            let row = repo.get(&conn, id)?;
            let mut dto = PrinterDto::from(row);

            // Enrich with last reading.
            if let Some(reading) = repo.get_last_reading(&conn, id)? {
                dto.status = Some(reading.status.clone());
                dto.page_count = reading.page_count;
                // Parse toner_levels JSON.
                if let Ok(val) =
                    serde_json::from_str::<serde_json::Value>(&reading.toner_levels_json)
                {
                    if !val.is_null() {
                        dto.toner_levels = Some(val);
                    }
                }
            }

            // Enrich with alert.
            let alerts = repo.list_active_alerts(&conn)?;
            if let Some(alert) = alerts.iter().find(|a| a.printer_id == id) {
                dto.has_alert = true;
                dto.alert_type = Some(alert.alert_type.clone());
            }

            // Enrich with current cartridge (D-PRN07-01).
            dto.current_cartridge_id = repo.current_cartridge_for_printer(&conn, dto.device_id)?;

            Ok(dto)
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking: {e}"),
        })?
    }

    /// Get the current cartridge ID installed in a printer (by printer device_id).
    pub async fn current_cartridge_for_printer(
        &self,
        printer_device_id: i64,
    ) -> Result<Option<i64>, AppError> {
        let readers = self.readers.clone();
        let repo = self.printer_repo.clone();
        tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            repo.current_cartridge_for_printer(&conn, printer_device_id)
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking: {e}"),
        })?
    }

    /// Get the cartridge_model_id list compatible with this printer device
    /// via `printer_cartridge_models` (D-11/D-12, Phase 12 gap closure —
    /// GAP-12-02). Empty Vec means "not configured" (D-14) — callers must
    /// not treat that as "no compatible models", only as "no narrowing".
    pub async fn get_compatible_models(&self, device_id: i64) -> Result<Vec<i64>, AppError> {
        let readers = self.readers.clone();
        let repo = self.printer_repo.clone();
        tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            repo.get_compatible_model_ids(&conn, device_id)
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking: {e}"),
        })?
    }

    /// Replace the set of cartridge models compatible with this printer
    /// device (printer-side write path, D-12). Returns the new set.
    pub async fn set_compatible_models(
        &self,
        device_id: i64,
        model_ids: Vec<i64>,
        caller: &Identity,
    ) -> Result<Vec<i64>, AppError> {
        authorize(caller, &Action::MutatePrinters)?;
        let now = self.clock.unix_seconds();
        let user_id = caller.user_id;
        let audit_repo = self.audit_repo.clone();
        let model_ids_for_write = model_ids.clone();

        self.writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;
                SqlitePrinterRepository::set_compatible_models_in_tx(
                    &tx,
                    device_id,
                    &model_ids_for_write,
                    now,
                )?;
                audit_repo.insert(
                    &tx,
                    AuditEntry {
                        entity_type: "printer_compatibility",
                        entity_id: device_id,
                        action: "set_compatible_models",
                        user_id,
                        before_json: None,
                        after_json: None,
                        payload_json: Some(json!({ "model_ids": model_ids_for_write }).to_string()),
                        created_at_utc: now,
                    },
                )?;
                tx.commit().map_err(map_rusqlite)?;
                Ok(())
            })
            .await?;

        self.get_compatible_models(device_id).await
    }

    // -----------------------------------------------------------------------
    // Write paths
    // -----------------------------------------------------------------------

    /// Create a printer record from a device (used after discovery or manual add).
    pub async fn create_from_device(
        &self,
        payload: PrinterCreateDto,
        caller: &Identity,
    ) -> Result<PrinterDto, AppError> {
        authorize(caller, &Action::MutatePrinters)?;
        let now = self.clock.unix_seconds();
        let user_id = caller.user_id;
        let printer_repo = self.printer_repo.clone();
        let audit_repo = self.audit_repo.clone();

        let printer_new = PrinterNew {
            device_id: payload.device_id as i64,
            ip_address: payload.ip_address.clone(),
            community_raw: payload
                .community_update
                .clone()
                .unwrap_or_else(|| "public".to_string()),
            snmp_version: payload.snmp_version.clone(),
            oid_profile_id: payload.oid_profile_id,
            usb_host_device_id: payload.usb_host_device_id.map(|id| id as i64),
        };

        let printer_id = self
            .writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;
                let id = printer_repo.create_in_tx(&tx, &printer_new, now)?;
                audit_repo.insert(
                    &tx,
                    AuditEntry {
                        entity_type: "printer",
                        entity_id: id,
                        action: "create",
                        user_id,
                        before_json: None,
                        after_json: None,
                        payload_json: None,
                        created_at_utc: now,
                    },
                )?;
                tx.commit().map_err(map_rusqlite)?;
                Ok(id)
            })
            .await?;

        self.get(printer_id).await
    }

    /// Acknowledge an active alert for a printer.
    pub async fn acknowledge_alert(
        &self,
        printer_id: i64,
        caller: &Identity,
    ) -> Result<(), AppError> {
        authorize(caller, &Action::MutatePrinters)?;
        let now = self.clock.unix_seconds();
        self.writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;
                tx.execute(
                    "UPDATE printer_alerts SET acknowledged_at_utc = ?1 \
                     WHERE printer_id = ?2 AND acknowledged_at_utc IS NULL",
                    rusqlite::params![now, printer_id],
                )
                .map_err(map_rusqlite)?;
                tx.commit().map_err(map_rusqlite)?;
                Ok(())
            })
            .await
    }

    /// Prune old printer readings (D-Retention-01).
    /// Deletes readings older than 90 days; keeps 1/day for 30–90 day window.
    pub async fn prune_old_readings(&self) -> Result<u64, AppError> {
        let now = self.clock.unix_seconds();
        // Retention window: 90 days
        let retention_cutoff = now - (90 * 24 * 3600);
        // Downsample window: 30 days
        let downsample_cutoff = now - (30 * 24 * 3600);

        self.writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;
                let deleted = SqlitePrinterRepository::prune_old_readings_in_tx(
                    &tx,
                    retention_cutoff,
                    downsample_cutoff,
                )?;
                tx.commit().map_err(map_rusqlite)?;
                Ok(deleted)
            })
            .await
    }

    // -----------------------------------------------------------------------
    // SNMP polling
    // -----------------------------------------------------------------------

    /// Poll a single printer by id and store the reading.
    pub async fn poll_single(&self, printer_id: i64) {
        let readers = self.readers.clone();
        let repo = self.printer_repo.clone();

        // Get current printer info for SNMP.
        let printer = tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            repo.get(&conn, printer_id)
        })
        .await
        .expect("spawn_blocking");

        let printer = match printer {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!("poll_single({printer_id}): printer not found: {e:?}");
                return;
            }
        };

        let ip = match &printer.ip_address {
            Some(ip) => ip.clone(),
            None => {
                tracing::debug!("poll_single({printer_id}): USB-only, skipping SNMP poll");
                return;
            }
        };

        let snmp = self.snmp_client.clone();
        let status_oid = "1.3.6.1.2.1.25.3.5.1.1.1"; // hrPrinterStatus

        let result = snmp.get_oids(&ip, "public", &[status_oid], 5).await;

        let now = self.clock.unix_seconds();

        match result {
            Ok(Some(vals)) => {
                let status = map_snmp_status(vals.first());
                let toner_json = json!({}).to_string();

                let status_clone = status.clone();
                let writer = self.writer.clone();
                let repo_w = self.printer_repo.clone();
                let ws_tx = self.ws_tx.clone();
                let printer_name = printer.device_name.clone().unwrap_or_default();
                let _ = writer
                    .execute(move |conn| {
                        let tx = conn.transaction().map_err(map_rusqlite)?;
                        repo_w.upsert_reading_in_tx(
                            &tx,
                            printer_id,
                            now,
                            &toner_json,
                            None,
                            &status_clone,
                        )?;
                        repo_w.update_last_seen_in_tx(&tx, printer_id, now)?;

                        if let Some(alert_type) = detect_alert_type(&status_clone) {
                            repo_w.upsert_alert_in_tx(&tx, printer_id, alert_type, now)?;
                            let _ = ws_tx.send(WsEvent::PrinterAlert {
                                printer_id,
                                printer_name,
                                alert_type: alert_type.to_string(),
                            });
                        }

                        tx.commit().map_err(map_rusqlite)?;
                        Ok(())
                    })
                    .await;
            }
            Ok(None) => {
                // Unreachable — treat as offline.
                let writer = self.writer.clone();
                let repo_w = self.printer_repo.clone();
                let ws_tx = self.ws_tx.clone();
                let printer_name = printer.device_name.clone().unwrap_or_default();
                let _ = writer
                    .execute(move |conn| {
                        let tx = conn.transaction().map_err(map_rusqlite)?;
                        repo_w.upsert_reading_in_tx(&tx, printer_id, now, "{}", None, "offline")?;
                        repo_w.upsert_alert_in_tx(&tx, printer_id, "offline", now)?;
                        tx.commit().map_err(map_rusqlite)?;
                        let _ = ws_tx.send(WsEvent::PrinterAlert {
                            printer_id,
                            printer_name,
                            alert_type: "offline".to_string(),
                        });
                        Ok(())
                    })
                    .await;
            }
            Err(e) => {
                tracing::warn!("poll_single({printer_id}): SNMP error: {e:?}");
            }
        }
    }

    /// Poll all known printers.
    pub async fn poll_all(&self) {
        let readers = self.readers.clone();
        let repo = self.printer_repo.clone();

        let printer_ids: Vec<i64> = tokio::task::spawn_blocking(move || {
            let c = readers.acquire();
            let filter = PrinterFilter::default();
            let page = Pagination {
                offset: 0,
                limit: 1000,
            };
            repo.list(&c, &filter, &page)
                .map(|(rows, _)| rows.into_iter().map(|r| r.id).collect::<Vec<_>>())
                .unwrap_or_default()
        })
        .await
        .unwrap_or_default();

        for id in printer_ids {
            self.poll_single(id).await;
        }
    }

    /// Discover printers in an IP range via SNMP probe.
    pub async fn discover(
        &self,
        ip_start: &str,
        ip_end: &str,
        community: &str,
        caller: &Identity,
    ) -> Result<Vec<DiscoveredPrinterDto>, AppError> {
        authorize(caller, &Action::MutatePrinters)?;

        // Parse IP range.
        let start = ip_start
            .parse::<std::net::Ipv4Addr>()
            .map_err(|_| AppError::Validation {
                field: "ip_start".into(),
                message: "Некорректный IPv4-адрес".into(),
            })?;
        let end = ip_end
            .parse::<std::net::Ipv4Addr>()
            .map_err(|_| AppError::Validation {
                field: "ip_end".into(),
                message: "Некорректный IPv4-адрес".into(),
            })?;

        let snmp = self.snmp_client.clone();
        let community = community.to_string();

        // Iterate the range (simple sequential scan for Phase 6).
        let start_u32 = u32::from(start);
        let end_u32 = u32::from(end);

        let mut results = Vec::new();

        let readers = self.readers.clone();
        let repo = self.printer_repo.clone();

        for ip_u32 in start_u32..=end_u32 {
            let ip = std::net::Ipv4Addr::from(ip_u32).to_string();
            if let Ok(Some(probed)) = snmp.probe(&ip, &community).await {
                let vendor = identify_vendor(&probed.sys_object_id).map(str::to_string);

                // Check for duplicate.
                let ip_clone = ip.clone();
                let readers_clone = readers.clone();
                let repo_clone = repo.clone();
                let is_duplicate = tokio::task::spawn_blocking(move || -> bool {
                    let conn = readers_clone.acquire();
                    let filter = PrinterFilter {
                        status: None,
                        search: None,
                    };
                    let page = Pagination {
                        offset: 0,
                        limit: 1000,
                    };
                    if let Ok((rows, _)) = repo_clone.list(&conn, &filter, &page) {
                        rows.iter()
                            .any(|r| r.ip_address.as_deref() == Some(&ip_clone))
                    } else {
                        false
                    }
                })
                .await
                .unwrap_or(false);

                results.push(DiscoveredPrinterDto {
                    ip,
                    vendor,
                    model: Some(probed.sys_descr),
                    sys_name: probed.sys_name,
                    oid_profile_id: None, // TODO: match from oid_profiles in Phase 7
                    is_duplicate,
                });
            }
        }

        Ok(results)
    }
}

/// Map an SNMP OID value to a status string.
fn map_snmp_status(val: Option<&trackly_core::ports::snmp::OidValue>) -> String {
    use trackly_core::ports::snmp::SnmpValue;
    // hrPrinterStatus: 1=other, 2=unknown, 3=idle, 4=printing, 5=warmup
    // We simplify: integer 3/4/5 = ok, rest = unknown.
    match val {
        Some(v) => match &v.value {
            SnmpValue::Integer(n) => match n {
                3..=5 => "ok".to_string(),
                _ => "unknown".to_string(),
            },
            _ => "ok".to_string(),
        },
        None => "unknown".to_string(),
    }
}

// ---------------------------------------------------------------------------
// Background poll task
// ---------------------------------------------------------------------------

/// Start the background SNMP poll task.
///
/// Called from AppCtx::build with a child CancellationToken (D-Arch-01).
/// Uses `MissedTickBehavior::Skip` so a slow poll cycle doesn't pile up.
pub async fn run_poll_task(
    printer_svc: Arc<PrinterService>,
    mut on_demand_rx: tokio::sync::mpsc::Receiver<i64>,
    shutdown: CancellationToken,
) {
    // Default poll interval: 5 minutes.
    let poll_interval_secs = 300u64;
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(poll_interval_secs));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = interval.tick() => {
                printer_svc.poll_all().await;
                // Prune on every tick (D-Retention-01).
                if let Err(e) = printer_svc.prune_old_readings().await {
                    tracing::warn!("prune_old_readings failed: {e:?}");
                }
            }
            Some(printer_id) = on_demand_rx.recv() => {
                printer_svc.poll_single(printer_id).await;
            }
            _ = shutdown.cancelled() => {
                tracing::info!("printer poll task: shutdown signal received");
                break;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toner_percent() {
        // level_over_max encoding: 45 out of 100 = 45%
        assert_eq!(parse_toner_level(45, 100, "level_over_max"), Some(45));
        // Unknown value (-2 = standard SNMP unknown)
        assert_eq!(parse_toner_level(-2, -2, "level_over_max"), None);
        // percent encoding: value is already %
        assert_eq!(parse_toner_level(75, 0, "percent"), Some(75));
        // Zero level
        assert_eq!(parse_toner_level(0, 100, "level_over_max"), Some(0));
        // Capped at 100
        assert_eq!(parse_toner_level(110, 100, "level_over_max"), Some(100));
    }

    #[test]
    fn test_vendor_identify() {
        assert_eq!(identify_vendor("1.3.6.1.4.1.40093.1"), Some("pantum"));
        assert_eq!(identify_vendor("1.3.6.1.4.1.1347.42.1"), Some("kyocera"));
        assert_eq!(identify_vendor("1.3.6.1.4.1.11.2.3.9.1"), Some("hp"));
        assert_eq!(identify_vendor("1.3.6.1.4.1.1602.1.1"), Some("canon"));
        assert_eq!(identify_vendor("1.3.6.1.4.1.99999.1"), None);
        assert_eq!(identify_vendor(""), None);
    }

    #[test]
    fn test_detect_alert_type() {
        assert_eq!(detect_alert_type("error"), Some("error"));
        assert_eq!(detect_alert_type("offline"), Some("offline"));
        assert_eq!(detect_alert_type("ok"), None);
        assert_eq!(detect_alert_type("warning"), None);
        assert_eq!(detect_alert_type("unknown"), None);
    }

    #[test]
    fn test_secret_debug() {
        use trackly_core::primitives::secret::Secret;
        let s = Secret::new("secret_community".to_string());
        let debug_str = format!("{s:?}");
        assert!(
            debug_str.contains("***"),
            "Secret Debug must mask value, got: {debug_str}"
        );
        assert!(
            !debug_str.contains("secret_community"),
            "Secret Debug must not leak value, got: {debug_str}"
        );
    }
}
