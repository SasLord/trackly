//! `DashboardService` — aggregates all 5 dashboard widget counts (Phase 7 Plan 03).
//!
//! Single `get_all_widgets()` call: executes multiple read-only queries on one
//! reader connection to avoid round-trips. Returns `DashboardWidgetDto`.
//!
//! Consumption chart (`get_consumption_chart`): queries audit_log WHERE
//! action = 'custom:install' — verified action string from CartridgeTransitionOp.
//!
//! Security (T-07-03-02): all SQL uses `params![]` or parameterised vectors;
//! period bounds are computed server-side.

use std::sync::Arc;

use rusqlite::params;
use trackly_core::auth::Identity;
use trackly_core::error::AppError;
use trackly_core::primitives::clock::Clock;
use trackly_infra::db::{pools::ReaderPool, writer_worker::WriterHandle};
use trackly_infra::error_conversions::map_rusqlite;
use trackly_infra::AppConfig;

use crate::dto::reports::{ConsumptionPoint, DashboardStatusCount, DashboardWidgetDto, PeriodDto};
use crate::services::report_service::compute_period_utc;

#[derive(Clone)]
pub struct DashboardService {
    pub writer: Arc<WriterHandle>,
    pub readers: Arc<ReaderPool>,
    pub clock: Arc<dyn Clock + Send + Sync>,
    pub config: Arc<AppConfig>,
}

impl DashboardService {
    pub fn new(
        writer: Arc<WriterHandle>,
        readers: Arc<ReaderPool>,
        clock: Arc<dyn Clock + Send + Sync>,
        config: Arc<AppConfig>,
    ) -> Self {
        Self {
            writer,
            readers,
            clock,
            config,
        }
    }

    /// Returns all 5 dashboard widget aggregates in a single call (DASH-01..05).
    ///
    /// `period` is used only for request counts (DASH-04) — devices and cartridges
    /// always show current snapshot totals.
    ///
    /// D-GATE-03: an Employee caller is routed to [`Self::get_employee_widgets`]
    /// — a structurally separate query path that never touches the
    /// devices/cartridges/printers tables, not a filtered view of this
    /// org-wide payload. Admin/Manager callers continue through the
    /// unchanged body below.
    pub async fn get_all_widgets(
        &self,
        caller: &Identity,
        period: Option<PeriodDto>,
    ) -> Result<DashboardWidgetDto, AppError> {
        if matches!(caller.role, trackly_core::auth::Role::Employee) {
            return self.get_employee_widgets(caller, period).await;
        }

        let readers = self.readers.clone();
        let tz = {
            let tz_name = &self.config.organization.timezone;
            if tz_name == "Europe/Moscow" {
                time::UtcOffset::from_hms(3, 0, 0).unwrap()
            } else {
                time::UtcOffset::UTC
            }
        };

        tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();

            // ----------------------------------------------------------------
            // DASH-01: Device counts by status
            // ----------------------------------------------------------------
            let devices_by_status: Vec<DashboardStatusCount> = {
                let mut stmt = conn
                    .prepare(
                        "SELECT s.name, COUNT(d.id) \
                         FROM devices d \
                         JOIN device_statuses s ON s.id = d.status_id \
                         WHERE d.deleted_at_utc IS NULL \
                         GROUP BY d.status_id \
                         ORDER BY s.name",
                    )
                    .map_err(map_rusqlite)?;
                let rows = stmt
                    .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))
                    .map_err(map_rusqlite)?;
                let mut out = Vec::new();
                for row in rows {
                    let (name, count) = row.map_err(map_rusqlite)?;
                    out.push(DashboardStatusCount {
                        status_name: name,
                        count,
                    });
                }
                out
            };
            let devices_total: i64 = devices_by_status.iter().map(|s| s.count).sum();

            // ----------------------------------------------------------------
            // DASH-02: Cartridge counts by status + low-stock
            // ----------------------------------------------------------------
            let cartridge_by_status: Vec<DashboardStatusCount> = {
                let mut stmt = conn
                    .prepare(
                        "SELECT s.name, COUNT(c.id) \
                         FROM cartridges c \
                         JOIN cartridge_statuses s ON s.id = c.status_id \
                         WHERE c.deleted_at_utc IS NULL \
                         GROUP BY c.status_id \
                         ORDER BY s.name",
                    )
                    .map_err(map_rusqlite)?;
                let rows = stmt
                    .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))
                    .map_err(map_rusqlite)?;
                let mut out = Vec::new();
                for row in rows {
                    let (name, count) = row.map_err(map_rusqlite)?;
                    out.push(DashboardStatusCount {
                        status_name: name,
                        count,
                    });
                }
                out
            };

            // Low-stock: read threshold from app_settings.
            let threshold: i64 = conn
                .query_row(
                    "SELECT value FROM app_settings WHERE key = 'low_stock_threshold'",
                    [],
                    |r| r.get::<_, String>(0),
                )
                .ok()
                .and_then(|s| s.trim().parse::<i64>().ok())
                .filter(|&t| t > 0)
                .unwrap_or(2);

            // Models with fewer than threshold full/in-stock cartridges.
            let low_stock_result: Vec<(String, i64)> = {
                let mut stmt = conn
                    .prepare(
                        "SELECT m.brand || ' ' || m.model AS label, COUNT(c.id) AS cnt \
                         FROM cartridge_models m \
                         LEFT JOIN cartridges c ON c.model_id = m.id \
                           AND c.status_id = 1 \
                           AND c.state_id = 1 \
                           AND c.deleted_at_utc IS NULL \
                         WHERE m.deleted_at_utc IS NULL \
                         GROUP BY m.id \
                         HAVING cnt < ?1 \
                         ORDER BY cnt ASC, label ASC",
                    )
                    .map_err(map_rusqlite)?;
                let rows = stmt
                    .query_map(params![threshold], |r| {
                        Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
                    })
                    .map_err(map_rusqlite)?;
                let mut out = Vec::new();
                for row in rows {
                    out.push(row.map_err(map_rusqlite)?);
                }
                out
            };
            let low_stock_count = low_stock_result.len() as i64;
            let low_stock_models: Vec<String> = low_stock_result
                .into_iter()
                .map(|(label, _)| label)
                .collect();

            // ----------------------------------------------------------------
            // DASH-04: Request counts by status
            // ----------------------------------------------------------------
            let (req_ts_from, req_ts_to) = match &period {
                Some(p) => compute_period_utc(p, tz),
                None => (None, None),
            };

            // Requests use requests.status TEXT column (not a separate FK table).
            // Values: 'open' | 'in_progress' | 'completed' | 'rejected'.
            let (request_counts_open, request_counts_in_progress, request_counts_completed) = {
                let mut clauses = vec!["r.deleted_at_utc IS NULL".to_string()];
                let mut owned: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
                let mut pidx = 1usize;
                if let Some(from) = req_ts_from {
                    clauses.push(format!("r.created_at_utc >= ?{pidx}"));
                    owned.push(Box::new(from));
                    pidx += 1;
                }
                if let Some(to) = req_ts_to {
                    clauses.push(format!("r.created_at_utc <= ?{pidx}"));
                    owned.push(Box::new(to));
                    pidx += 1;
                }
                let _ = pidx;

                let sql = format!(
                    "SELECT r.status, COUNT(r.id) \
                     FROM requests r \
                     WHERE {} \
                     GROUP BY r.status",
                    clauses.join(" AND ")
                );
                let param_refs: Vec<&dyn rusqlite::types::ToSql> =
                    owned.iter().map(|b| b.as_ref()).collect();
                let mut stmt = conn.prepare(&sql).map_err(map_rusqlite)?;
                let rows = stmt
                    .query_map(param_refs.as_slice(), |r| {
                        Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
                    })
                    .map_err(map_rusqlite)?;

                let mut open: i64 = 0;
                let mut in_progress: i64 = 0;
                let mut completed: i64 = 0;
                for row in rows {
                    let (status, count) = row.map_err(map_rusqlite)?;
                    match status.as_str() {
                        "open" => open += count,
                        "in_progress" => in_progress += count,
                        "completed" => completed += count,
                        _ => {} // 'rejected' not shown in dashboard
                    }
                }
                (open, in_progress, completed)
            };

            // ----------------------------------------------------------------
            // DASH-05: Printer counts (D-13: problematic = offline+error alerts)
            // ----------------------------------------------------------------
            // printers table has no deleted_at_utc — count all rows.
            let printer_total: i64 = conn
                .query_row("SELECT COUNT(*) FROM printers", [], |r| r.get(0))
                .map_err(map_rusqlite)?;

            // Offline count: printers with active 'offline' alert (unacknowledged = active).
            // printer_alerts has UNIQUE(printer_id) — no DISTINCT needed.
            let printer_offline: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM printer_alerts \
                     WHERE alert_type = 'offline'",
                    [],
                    |r| r.get(0),
                )
                .map_err(map_rusqlite)?;

            // Problematic: printers with offline or error alerts (D-13).
            let printer_problematic: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM printer_alerts \
                     WHERE alert_type IN ('offline', 'error')",
                    [],
                    |r| r.get(0),
                )
                .map_err(map_rusqlite)?;

            let printer_online = printer_total - printer_offline;

            Ok(DashboardWidgetDto {
                devices_total,
                devices_by_status,
                cartridge_by_status,
                low_stock_count,
                low_stock_models,
                request_counts_open,
                request_counts_in_progress,
                request_counts_completed,
                printer_online,
                printer_offline,
                printer_problematic,
            })
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking get_all_widgets: {e}"),
        })?
    }

    /// D-GATE-03: Employee-scoped dashboard widgets.
    ///
    /// Structurally separate from [`Self::get_all_widgets`] — this method
    /// issues exactly one query, against `requests` only, scoped to
    /// `requested_by_user_id = caller.user_id`. It never queries `devices`,
    /// `cartridges`, `cartridge_models`, `printers`, or `printer_alerts`.
    /// All org-wide fields on `DashboardWidgetDto` are returned
    /// zeroed/empty — not omitted, since the DTO shape is shared across
    /// roles (one DTO, two transports) — proving to the wire-level CI
    /// assertion that the employee code path never touched those tables.
    async fn get_employee_widgets(
        &self,
        caller: &Identity,
        period: Option<PeriodDto>,
    ) -> Result<DashboardWidgetDto, AppError> {
        let readers = self.readers.clone();
        let owner_user_id = caller.user_id;
        let tz = {
            let tz_name = &self.config.organization.timezone;
            if tz_name == "Europe/Moscow" {
                time::UtcOffset::from_hms(3, 0, 0).unwrap()
            } else {
                time::UtcOffset::UTC
            }
        };

        tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();

            let (req_ts_from, req_ts_to) = match &period {
                Some(p) => compute_period_utc(p, tz),
                None => (None, None),
            };

            let (request_counts_open, request_counts_in_progress, request_counts_completed) = {
                let mut clauses = vec![
                    "r.deleted_at_utc IS NULL".to_string(),
                    "r.requested_by_user_id = ?1".to_string(),
                    "r.request_type != 'ad_register'".to_string(),
                ];
                let mut owned: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(owner_user_id)];
                let mut pidx = 2usize;
                if let Some(from) = req_ts_from {
                    clauses.push(format!("r.created_at_utc >= ?{pidx}"));
                    owned.push(Box::new(from));
                    pidx += 1;
                }
                if let Some(to) = req_ts_to {
                    clauses.push(format!("r.created_at_utc <= ?{pidx}"));
                    owned.push(Box::new(to));
                    pidx += 1;
                }
                let _ = pidx;

                let sql = format!(
                    "SELECT r.status, COUNT(r.id) \
                     FROM requests r \
                     WHERE {} \
                     GROUP BY r.status",
                    clauses.join(" AND ")
                );
                let param_refs: Vec<&dyn rusqlite::types::ToSql> =
                    owned.iter().map(|b| b.as_ref()).collect();
                let mut stmt = conn.prepare(&sql).map_err(map_rusqlite)?;
                let rows = stmt
                    .query_map(param_refs.as_slice(), |r| {
                        Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
                    })
                    .map_err(map_rusqlite)?;

                let mut open: i64 = 0;
                let mut in_progress: i64 = 0;
                let mut completed: i64 = 0;
                for row in rows {
                    let (status, count) = row.map_err(map_rusqlite)?;
                    match status.as_str() {
                        "open" => open += count,
                        "in_progress" => in_progress += count,
                        "completed" => completed += count,
                        _ => {} // 'rejected' not shown in dashboard
                    }
                }
                (open, in_progress, completed)
            };

            Ok(DashboardWidgetDto {
                devices_total: 0,
                devices_by_status: vec![],
                cartridge_by_status: vec![],
                low_stock_count: 0,
                low_stock_models: vec![],
                request_counts_open,
                request_counts_in_progress,
                request_counts_completed,
                printer_online: 0,
                printer_offline: 0,
                printer_problematic: 0,
            })
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking get_employee_widgets: {e}"),
        })?
    }

    /// DASH-03: Cartridge consumption time-series chart.
    ///
    /// `window_months`: 3, 6, or 12 — look-back window from now.
    /// Queries audit_log WHERE action = 'custom:install' (verified action string).
    pub async fn get_consumption_chart(
        &self,
        window_months: u8,
    ) -> Result<Vec<ConsumptionPoint>, AppError> {
        let now = self.clock.unix_seconds();
        let start_utc = now - (window_months as i64 * 30 * 86400);
        let readers = self.readers.clone();

        tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            let mut stmt = conn
                .prepare(
                    "SELECT m.brand || ' ' || m.model AS model_label, \
                           strftime('%Y-%m', datetime(al.created_at_utc, 'unixepoch', '+3 hours')) AS month_key, \
                           COUNT(*) AS installs \
                     FROM audit_log al \
                     JOIN cartridges c ON c.id = al.entity_id \
                     JOIN cartridge_models m ON m.id = c.model_id \
                     WHERE al.entity_type = 'cartridge' \
                       AND al.action = 'custom:install' \
                       AND al.created_at_utc IS NOT NULL \
                       AND al.created_at_utc >= ?1 \
                     GROUP BY model_label, month_key \
                     ORDER BY month_key ASC, model_label ASC",
                )
                .map_err(map_rusqlite)?;

            let rows = stmt
                .query_map(params![start_utc], |r| {
                    Ok(ConsumptionPoint {
                        model_label: r.get(0)?,
                        month_key: r.get(1)?,
                        installs: r.get(2)?,
                    })
                })
                .map_err(map_rusqlite)?;

            let mut out = Vec::new();
            for row in rows {
                out.push(row.map_err(map_rusqlite)?);
            }
            Ok(out)
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking get_consumption_chart: {e}"),
        })?
    }
}
