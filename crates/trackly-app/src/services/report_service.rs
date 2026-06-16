//! `ReportService` — read layer for all tabular reports (Phase 7 Plan 03).
//!
//! Implements 8 report query methods, CSV export, and PDF export.
//! Period boundaries are computed server-side in UTC+3 (Europe/Moscow, no DST since 2014).
//!
//! Security (T-07-03-01):
//! All filter values are passed via parameterised queries — no user values concatenated
//! into SQL strings. WHERE clauses use `format!` only for `?N` placeholder positions.
//!
//! Security (T-07-03-04): Each query applies LIMIT 1000 as a DoS safeguard for v1.

use std::sync::Arc;

use rusqlite::types::ToSql;
use time::{Date, Month, PrimitiveDateTime, Time, UtcOffset};
use trackly_core::error::AppError;
use trackly_core::primitives::clock::Clock;
use trackly_infra::db::{pools::ReaderPool, writer_worker::WriterHandle};
use trackly_infra::error_conversions::map_rusqlite;
use trackly_infra::AppConfig;

use crate::dto::reports::{
    OrgSettingsDto, PeriodDto, ReportFilter, ReportResponse, ReportRow,
};
use crate::pdf::{
    docspec::{DocSpec, HeaderBlock, Section},
    PdfRenderer,
};

// ---------------------------------------------------------------------------
// Russian month names for PDF month-separator headings
// ---------------------------------------------------------------------------

const MONTH_NAMES_RU: [&str; 12] = [
    "Январь",
    "Февраль",
    "Март",
    "Апрель",
    "Май",
    "Июнь",
    "Июль",
    "Август",
    "Сентябрь",
    "Октябрь",
    "Ноябрь",
    "Декабрь",
];

// ---------------------------------------------------------------------------
// Public helper: compute UTC epoch bounds from PeriodDto in given TZ offset
// ---------------------------------------------------------------------------

/// Compute (start_utc, end_utc) inclusive Unix-second bounds from a PeriodDto.
///
/// Returns (None, None) for snapshot reports (unrecognised mode).
/// Moscow fixed offset: UTC+3 (no DST since 2014).
pub fn compute_period_utc(dto: &PeriodDto, tz_offset: UtcOffset) -> (Option<i64>, Option<i64>) {
    match dto.mode.as_str() {
        "month" => {
            let year = match dto.year {
                Some(y) => y,
                None => return (None, None),
            };
            let month_num = match dto.month {
                Some(m) => m,
                None => return (None, None),
            };
            let month = match Month::try_from(month_num) {
                Ok(m) => m,
                Err(_) => return (None, None),
            };
            let start_date = match Date::from_calendar_date(year, month, 1) {
                Ok(d) => d,
                Err(_) => return (None, None),
            };
            let start = PrimitiveDateTime::new(start_date, Time::MIDNIGHT)
                .assume_offset(tz_offset)
                .unix_timestamp();

            // Last day of month: advance to next month day 1 then get previous day.
            let next_month_date = if month_num == 12 {
                match Date::from_calendar_date(year + 1, Month::January, 1) {
                    Ok(d) => d,
                    Err(_) => return (None, None),
                }
            } else {
                match Date::from_calendar_date(
                    year,
                    Month::try_from(month_num + 1).unwrap(),
                    1,
                ) {
                    Ok(d) => d,
                    Err(_) => return (None, None),
                }
            };
            let last_day = next_month_date.previous_day().unwrap();
            let end = PrimitiveDateTime::new(
                last_day,
                Time::from_hms(23, 59, 59).unwrap(),
            )
            .assume_offset(tz_offset)
            .unix_timestamp();
            (Some(start), Some(end))
        }
        "year" => {
            let year = match dto.year {
                Some(y) => y,
                None => return (None, None),
            };
            let start_date = match Date::from_calendar_date(year, Month::January, 1) {
                Ok(d) => d,
                Err(_) => return (None, None),
            };
            let end_date = match Date::from_calendar_date(year, Month::December, 31) {
                Ok(d) => d,
                Err(_) => return (None, None),
            };
            let start = PrimitiveDateTime::new(start_date, Time::MIDNIGHT)
                .assume_offset(tz_offset)
                .unix_timestamp();
            let end = PrimitiveDateTime::new(
                end_date,
                Time::from_hms(23, 59, 59).unwrap(),
            )
            .assume_offset(tz_offset)
            .unix_timestamp();
            (Some(start), Some(end))
        }
        "range" => {
            let from_str = match &dto.date_from {
                Some(s) => s,
                None => return (None, None),
            };
            let to_str = match &dto.date_to {
                Some(s) => s,
                None => return (None, None),
            };
            let start = match parse_iso_date_to_utc(from_str, tz_offset, false) {
                Some(ts) => ts,
                None => return (None, None),
            };
            let end = match parse_iso_date_to_utc(to_str, tz_offset, true) {
                Some(ts) => ts,
                None => return (None, None),
            };
            (Some(start), Some(end))
        }
        _ => (None, None),
    }
}

/// Parse "YYYY-MM-DD" ISO string → Unix timestamp in given offset.
/// `end_of_day`: if true, return 23:59:59, else 00:00:00.
fn parse_iso_date_to_utc(s: &str, offset: UtcOffset, end_of_day: bool) -> Option<i64> {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 3 {
        return None;
    }
    let y: i32 = parts[0].parse().ok()?;
    let m: u8 = parts[1].parse().ok()?;
    let d: u8 = parts[2].parse().ok()?;
    let month = Month::try_from(m).ok()?;
    let date = Date::from_calendar_date(y, month, d).ok()?;
    let time = if end_of_day {
        Time::from_hms(23, 59, 59).unwrap()
    } else {
        Time::MIDNIGHT
    };
    Some(
        PrimitiveDateTime::new(date, time)
            .assume_offset(offset)
            .unix_timestamp(),
    )
}

// ---------------------------------------------------------------------------
// Excel formula injection guard (T-07-03-05)
// ---------------------------------------------------------------------------

fn csv_safe(value: &str) -> String {
    if value.starts_with(['=', '+', '-', '@']) {
        format!("'{value}")
    } else {
        value.to_string()
    }
}

// ---------------------------------------------------------------------------
// ReportService
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct ReportService {
    pub writer: Arc<WriterHandle>,
    pub readers: Arc<ReaderPool>,
    pub clock: Arc<dyn Clock + Send + Sync>,
    pub config: Arc<AppConfig>,
    pub pdf: Arc<PdfRenderer>,
}

impl ReportService {
    pub fn new(
        writer: Arc<WriterHandle>,
        readers: Arc<ReaderPool>,
        clock: Arc<dyn Clock + Send + Sync>,
        config: Arc<AppConfig>,
        pdf: Arc<PdfRenderer>,
    ) -> Self {
        Self {
            writer,
            readers,
            clock,
            config,
            pdf,
        }
    }

    /// Get UTC+3 offset for Europe/Moscow (no DST since 2014).
    pub fn get_tz_offset(&self) -> UtcOffset {
        match self.config.organization.timezone.as_str() {
            "Europe/Moscow" => UtcOffset::from_hms(3, 0, 0).unwrap(),
            _ => UtcOffset::UTC,
        }
    }

    // -----------------------------------------------------------------------
    // Device act reports
    // -----------------------------------------------------------------------

    /// RPT-01 / RPT-04 / RPT-05: acts (handover) filtered by period, type, location.
    pub async fn list_device_acts(
        &self,
        filter: ReportFilter,
        period: PeriodDto,
    ) -> Result<ReportResponse, AppError> {
        let tz = self.get_tz_offset();
        let (ts_from, ts_to) = compute_period_utc(&period, tz);
        let readers = self.readers.clone();
        tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            query_acts_inner(&conn, &filter, ts_from, ts_to, "handover")
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking list_device_acts: {e}"),
        })?
    }

    /// RPT-05: returns (return acts) filtered by period and location.
    pub async fn list_device_returns(
        &self,
        filter: ReportFilter,
        period: PeriodDto,
    ) -> Result<ReportResponse, AppError> {
        let tz = self.get_tz_offset();
        let (ts_from, ts_to) = compute_period_utc(&period, tz);
        let readers = self.readers.clone();
        tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            query_acts_inner(&conn, &filter, ts_from, ts_to, "return")
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking list_device_returns: {e}"),
        })?
    }

    /// RPT-02: devices currently «В работе» (snapshot).
    pub async fn list_device_in_use(
        &self,
        filter: ReportFilter,
    ) -> Result<ReportResponse, AppError> {
        let readers = self.readers.clone();
        tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            query_device_snapshot(&conn, &filter, "В работе")
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking list_device_in_use: {e}"),
        })?
    }

    /// RPT-02: devices currently «На складе» (snapshot).
    pub async fn list_device_in_stock(
        &self,
        filter: ReportFilter,
    ) -> Result<ReportResponse, AppError> {
        let readers = self.readers.clone();
        tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            query_device_snapshot(&conn, &filter, "На складе")
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking list_device_in_stock: {e}"),
        })?
    }

    // -----------------------------------------------------------------------
    // Cartridge reports
    // -----------------------------------------------------------------------

    /// RPT-06: cartridge consumption (action='custom:install') by period.
    pub async fn list_cartridge_consumption(
        &self,
        filter: ReportFilter,
        period: PeriodDto,
    ) -> Result<ReportResponse, AppError> {
        let tz = self.get_tz_offset();
        let (ts_from, ts_to) = compute_period_utc(&period, tz);
        let readers = self.readers.clone();
        tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            query_cartridge_audit(&conn, &filter, ts_from, ts_to, &["custom:install"])
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking list_cartridge_consumption: {e}"),
        })?
    }

    /// RPT-06: cartridge refills (to_refill + from_refill) by period.
    pub async fn list_cartridge_refills(
        &self,
        filter: ReportFilter,
        period: PeriodDto,
    ) -> Result<ReportResponse, AppError> {
        let tz = self.get_tz_offset();
        let (ts_from, ts_to) = compute_period_utc(&period, tz);
        let readers = self.readers.clone();
        tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            query_cartridge_audit(
                &conn,
                &filter,
                ts_from,
                ts_to,
                &["custom:to_refill", "custom:from_refill"],
            )
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking list_cartridge_refills: {e}"),
        })?
    }

    /// Cartridges currently «В работе» (snapshot).
    pub async fn list_cartridge_in_use(
        &self,
        filter: ReportFilter,
    ) -> Result<ReportResponse, AppError> {
        let readers = self.readers.clone();
        tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            query_cartridge_snapshot(&conn, &filter, "В работе")
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking list_cartridge_in_use: {e}"),
        })?
    }

    /// Cartridges currently «На складе» (snapshot).
    pub async fn list_cartridge_in_stock(
        &self,
        filter: ReportFilter,
    ) -> Result<ReportResponse, AppError> {
        let readers = self.readers.clone();
        tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            query_cartridge_snapshot(&conn, &filter, "На складе")
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking list_cartridge_in_stock: {e}"),
        })?
    }

    // -----------------------------------------------------------------------
    // CSV export (RPT-03, T-07-03-05)
    // -----------------------------------------------------------------------

    /// Export ReportResponse as UTF-8 BOM + semicolon-delimited CSV bytes.
    pub async fn export_csv(
        &self,
        rows: &ReportResponse,
        columns: &[&str],
    ) -> Result<Vec<u8>, AppError> {
        let mut wtr = csv::WriterBuilder::new()
            .delimiter(b';')
            .from_writer(Vec::new());

        wtr.write_record(columns).map_err(|e| AppError::Internal {
            source_chain: format!("csv header: {e}"),
        })?;

        for row in &rows.rows {
            let record: Vec<String> = columns
                .iter()
                .map(|col| {
                    let raw = row_field(row, col);
                    csv_safe(&raw)
                })
                .collect();
            wtr.write_record(&record).map_err(|e| AppError::Internal {
                source_chain: format!("csv row: {e}"),
            })?;
        }

        let inner = wtr.into_inner().map_err(|e| AppError::Internal {
            source_chain: format!("csv flush: {e}"),
        })?;

        let body = String::from_utf8(inner).map_err(|e| AppError::Internal {
            source_chain: format!("csv utf8: {e}"),
        })?;

        // Prepend UTF-8 BOM (D-CSV-02).
        let mut output = String::with_capacity(3 + body.len());
        output.push('\u{FEFF}');
        output.push_str(&body);

        Ok(output.into_bytes())
    }

    // -----------------------------------------------------------------------
    // PDF export (RPT-08)
    // -----------------------------------------------------------------------

    /// Export report as PDF bytes via DocSpec IR + krilla.
    pub async fn export_pdf(
        &self,
        rows: &ReportResponse,
        report_name: &str,
        period_label: &str,
        org: &OrgSettingsDto,
        logo_bytes: Option<Vec<u8>>,
        logo_mime: Option<String>,
        columns: &[&str],
    ) -> Result<Vec<u8>, AppError> {
        let header = HeaderBlock {
            org_name: org.org_name.clone(),
            org_inn: org.inn.clone(),
            org_kpp: org.kpp.clone(),
            org_address: org.address.clone(),
            logo_path: None,
            logo_bytes,
            logo_mime,
            act_label: report_name.to_string(),
            date_label: period_label.to_string(),
        };

        let mut sections: Vec<Section> = Vec::new();
        let mut current_month: Option<String> = None;
        let mut table_rows: Vec<Vec<String>> = Vec::new();

        for row in &rows.rows {
            let month_key = row.month_key.as_deref().unwrap_or("");

            if !month_key.is_empty() && Some(month_key) != current_month.as_deref() {
                if !table_rows.is_empty() {
                    sections.push(Section::ItemsTable {
                        columns: columns.iter().map(|s| s.to_string()).collect(),
                        rows: std::mem::take(&mut table_rows),
                    });
                }
                sections.push(Section::Heading {
                    level: 3,
                    text: month_key_to_russian(month_key),
                });
                current_month = Some(month_key.to_string());
            }

            table_rows.push(columns.iter().map(|col| row_field(row, col)).collect());
        }

        if !table_rows.is_empty() {
            sections.push(Section::ItemsTable {
                columns: columns.iter().map(|s| s.to_string()).collect(),
                rows: table_rows,
            });
        }

        if sections.is_empty() {
            sections.push(Section::Paragraph {
                text: "Нет данных за указанный период.".to_string(),
                style: Default::default(),
            });
        }

        let spec = DocSpec {
            title: report_name.to_string(),
            header,
            sections,
        };

        self.pdf.render_docspec(&spec)
    }
}

// ---------------------------------------------------------------------------
// ReportRow field accessor
// ---------------------------------------------------------------------------

fn row_field(row: &ReportRow, col: &str) -> String {
    match col {
        "number" => row.number.as_deref().unwrap_or("").to_string(),
        "sub_number" => row.sub_number.as_deref().unwrap_or("").to_string(),
        "giver_name" => row.giver_name.as_deref().unwrap_or("").to_string(),
        "receiver_name" => row.receiver_name.as_deref().unwrap_or("").to_string(),
        "handover_date_utc" => row
            .handover_date_utc
            .map(|ts| ts.to_string())
            .unwrap_or_default(),
        "location_name" => row.location_name.as_deref().unwrap_or("").to_string(),
        "act_type" => row.act_type.as_deref().unwrap_or("").to_string(),
        "device_name" => row.device_name.as_deref().unwrap_or("").to_string(),
        "quantity" => row.quantity.map(|q| q.to_string()).unwrap_or_default(),
        "code" => row.code.as_deref().unwrap_or("").to_string(),
        "model_label" => row.model_label.as_deref().unwrap_or("").to_string(),
        "status_name" => row.status_name.as_deref().unwrap_or("").to_string(),
        "month_key" => row.month_key.as_deref().unwrap_or("").to_string(),
        _ => String::new(),
    }
}

// ---------------------------------------------------------------------------
// Internal query helpers (sync, run inside spawn_blocking)
// ---------------------------------------------------------------------------

/// Query act/return records filtered by period, type, and optional fields.
fn query_acts_inner(
    conn: &rusqlite::Connection,
    filter: &ReportFilter,
    ts_from: Option<i64>,
    ts_to: Option<i64>,
    act_type: &str,
) -> Result<ReportResponse, AppError> {
    let mut clauses: Vec<String> = Vec::new();
    let mut owned_params: Vec<Box<dyn ToSql>> = Vec::new();

    clauses.push(format!("a.act_type = ?{}", next_idx(&owned_params)));
    owned_params.push(Box::new(act_type.to_string()));

    clauses.push("a.deleted_at_utc IS NULL".to_string());

    if let Some(from) = ts_from {
        clauses.push(format!("a.handover_date_utc >= ?{}", next_idx(&owned_params)));
        owned_params.push(Box::new(from));
    }
    if let Some(to) = ts_to {
        clauses.push(format!("a.handover_date_utc <= ?{}", next_idx(&owned_params)));
        owned_params.push(Box::new(to));
    }
    if let Some(loc) = filter.location_id {
        clauses.push(format!("a.location_id = ?{}", next_idx(&owned_params)));
        owned_params.push(Box::new(loc));
    }
    if let Some(type_id) = filter.type_id {
        clauses.push(format!("d.type_id = ?{}", next_idx(&owned_params)));
        owned_params.push(Box::new(type_id));
    }
    if let Some(ref search) = filter.search {
        let idx = next_idx(&owned_params);
        let like_val = format!("%{search}%");
        clauses.push(format!(
            "(a.number LIKE ?{idx} OR a.giver_name LIKE ?{idx} OR a.receiver_name LIKE ?{idx})"
        ));
        owned_params.push(Box::new(like_val));
    }

    let where_clause = clauses.join(" AND ");
    let sql = format!(
        "SELECT a.id, \
               strftime('%Y-%m', datetime(a.handover_date_utc, 'unixepoch', '+3 hours')) AS month_key, \
               CAST(a.number AS TEXT) as number, \
               a.sub_number, \
               a.giver_name, a.receiver_name, \
               a.handover_date_utc, \
               l.name AS location_name, \
               a.act_type, \
               GROUP_CONCAT(d.name, ', ') AS device_name, \
               SUM(ai.quantity) AS quantity \
         FROM acts a \
         LEFT JOIN locations l ON a.location_id = l.id \
         LEFT JOIN act_items ai ON ai.act_id = a.id \
         LEFT JOIN devices d ON d.id = ai.device_id \
         WHERE {where_clause} \
         GROUP BY a.id \
         ORDER BY a.handover_date_utc ASC, a.id ASC \
         LIMIT 1000"
    );

    let param_refs: Vec<&dyn ToSql> = owned_params.iter().map(|b| b.as_ref()).collect();
    let mut stmt = conn.prepare(&sql).map_err(map_rusqlite)?;
    let row_iter = stmt
        .query_map(param_refs.as_slice(), |r| {
            Ok(ReportRow {
                id: r.get(0)?,
                month_key: r.get(1)?,
                number: r.get(2)?,
                sub_number: r.get(3)?,
                giver_name: r.get(4)?,
                receiver_name: r.get(5)?,
                handover_date_utc: r.get(6)?,
                location_name: r.get(7)?,
                act_type: r.get(8)?,
                device_name: r.get(9)?,
                quantity: r.get(10)?,
                code: None,
                model_label: None,
                status_name: None,
            })
        })
        .map_err(map_rusqlite)?;

    let mut rows = Vec::new();
    for row in row_iter {
        rows.push(row.map_err(map_rusqlite)?);
    }
    let total = rows.len() as i64;
    Ok(ReportResponse { rows, total })
}

/// Snapshot report for devices by status name.
fn query_device_snapshot(
    conn: &rusqlite::Connection,
    filter: &ReportFilter,
    default_status_name: &str,
) -> Result<ReportResponse, AppError> {
    let mut clauses: Vec<String> = Vec::new();
    let mut owned_params: Vec<Box<dyn ToSql>> = Vec::new();

    clauses.push("d.deleted_at_utc IS NULL".to_string());

    if let Some(status_id) = filter.status_id {
        clauses.push(format!("d.status_id = ?{}", next_idx(&owned_params)));
        owned_params.push(Box::new(status_id));
    } else {
        clauses.push(format!(
            "d.status_id = (SELECT id FROM device_statuses WHERE name = ?{} LIMIT 1)",
            next_idx(&owned_params)
        ));
        owned_params.push(Box::new(default_status_name.to_string()));
    }
    if let Some(loc) = filter.location_id {
        clauses.push(format!("d.location_id = ?{}", next_idx(&owned_params)));
        owned_params.push(Box::new(loc));
    }
    if let Some(type_id) = filter.type_id {
        clauses.push(format!("d.type_id = ?{}", next_idx(&owned_params)));
        owned_params.push(Box::new(type_id));
    }

    let where_clause = clauses.join(" AND ");
    let sql = format!(
        "SELECT d.id, NULL as month_key, d.name as device_name, d.serial_number, \
               l.name as location_name, s.name as status_name \
         FROM devices d \
         LEFT JOIN locations l ON d.location_id = l.id \
         LEFT JOIN device_statuses s ON d.status_id = s.id \
         WHERE {where_clause} \
         ORDER BY d.name ASC, d.id ASC \
         LIMIT 1000"
    );

    let param_refs: Vec<&dyn ToSql> = owned_params.iter().map(|b| b.as_ref()).collect();
    let mut stmt = conn.prepare(&sql).map_err(map_rusqlite)?;
    let row_iter = stmt
        .query_map(param_refs.as_slice(), |r| {
            Ok(ReportRow {
                id: r.get(0)?,
                month_key: None,
                number: None,
                sub_number: None,
                giver_name: None,
                receiver_name: None,
                handover_date_utc: None,
                location_name: r.get::<_, Option<String>>(4)?,
                act_type: None,
                device_name: r.get(2)?,
                quantity: None,
                code: r.get(3)?, // serial_no in code field
                model_label: None,
                status_name: r.get(5)?,
            })
        })
        .map_err(map_rusqlite)?;

    let mut rows = Vec::new();
    for row in row_iter {
        rows.push(row.map_err(map_rusqlite)?);
    }
    let total = rows.len() as i64;
    Ok(ReportResponse { rows, total })
}

/// Query cartridge audit_log entries for given actions.
fn query_cartridge_audit(
    conn: &rusqlite::Connection,
    filter: &ReportFilter,
    ts_from: Option<i64>,
    ts_to: Option<i64>,
    actions: &[&str],
) -> Result<ReportResponse, AppError> {
    let mut clauses: Vec<String> = Vec::new();
    let mut owned_params: Vec<Box<dyn ToSql>> = Vec::new();

    clauses.push("al.entity_type = 'cartridge'".to_string());

    // Build action IN (?1, ?2, ...) clause.
    let action_placeholders: Vec<String> = actions
        .iter()
        .map(|action| {
            let placeholder = format!("?{}", next_idx(&owned_params));
            owned_params.push(Box::new(action.to_string()));
            placeholder
        })
        .collect();
    clauses.push(format!("al.action IN ({})", action_placeholders.join(", ")));

    if let Some(from) = ts_from {
        clauses.push(format!("al.created_at_utc >= ?{}", next_idx(&owned_params)));
        owned_params.push(Box::new(from));
    }
    if let Some(to) = ts_to {
        clauses.push(format!("al.created_at_utc <= ?{}", next_idx(&owned_params)));
        owned_params.push(Box::new(to));
    }
    if let Some(model_id) = filter.model_id {
        clauses.push(format!("m.id = ?{}", next_idx(&owned_params)));
        owned_params.push(Box::new(model_id));
    }
    if let Some(ref color) = filter.color {
        clauses.push(format!("m.color = ?{}", next_idx(&owned_params)));
        owned_params.push(Box::new(color.clone()));
    }
    if let Some(status_id) = filter.status_id {
        clauses.push(format!("c.status_id = ?{}", next_idx(&owned_params)));
        owned_params.push(Box::new(status_id));
    }

    let where_clause = clauses.join(" AND ");
    // cartridges has no location_id FK — uses freeform text `location` column.
    let sql = format!(
        "SELECT c.id, \
               strftime('%Y-%m', datetime(al.created_at_utc, 'unixepoch', '+3 hours')) AS month_key, \
               al.created_at_utc as handover_date_utc, \
               c.location as location_name, \
               m.brand || ' ' || m.model AS model_label, \
               c.code, \
               al.action \
         FROM audit_log al \
         JOIN cartridges c ON c.id = al.entity_id \
         JOIN cartridge_models m ON m.id = c.model_id \
         WHERE {where_clause} \
         ORDER BY al.created_at_utc ASC \
         LIMIT 1000"
    );

    let param_refs: Vec<&dyn ToSql> = owned_params.iter().map(|b| b.as_ref()).collect();
    let mut stmt = conn.prepare(&sql).map_err(map_rusqlite)?;
    let row_iter = stmt
        .query_map(param_refs.as_slice(), |r| {
            Ok(ReportRow {
                id: r.get(0)?,
                month_key: r.get(1)?,
                number: None,
                sub_number: None,
                giver_name: None,
                receiver_name: None,
                handover_date_utc: r.get(2)?,
                location_name: r.get(3)?,
                act_type: r.get(6)?,
                device_name: None,
                quantity: None,
                code: r.get(5)?,
                model_label: r.get(4)?,
                status_name: None,
            })
        })
        .map_err(map_rusqlite)?;

    let mut rows = Vec::new();
    for row in row_iter {
        rows.push(row.map_err(map_rusqlite)?);
    }
    let total = rows.len() as i64;
    Ok(ReportResponse { rows, total })
}

/// Snapshot report for cartridges by status name.
fn query_cartridge_snapshot(
    conn: &rusqlite::Connection,
    filter: &ReportFilter,
    default_status_name: &str,
) -> Result<ReportResponse, AppError> {
    let mut clauses: Vec<String> = Vec::new();
    let mut owned_params: Vec<Box<dyn ToSql>> = Vec::new();

    clauses.push("c.deleted_at_utc IS NULL".to_string());

    if let Some(status_id) = filter.status_id {
        clauses.push(format!("c.status_id = ?{}", next_idx(&owned_params)));
        owned_params.push(Box::new(status_id));
    } else {
        clauses.push(format!(
            "c.status_id = (SELECT id FROM cartridge_statuses WHERE name = ?{} LIMIT 1)",
            next_idx(&owned_params)
        ));
        owned_params.push(Box::new(default_status_name.to_string()));
    }
    if let Some(model_id) = filter.model_id {
        clauses.push(format!("m.id = ?{}", next_idx(&owned_params)));
        owned_params.push(Box::new(model_id));
    }
    if let Some(ref color) = filter.color {
        clauses.push(format!("m.color = ?{}", next_idx(&owned_params)));
        owned_params.push(Box::new(color.clone()));
    }

    let where_clause = clauses.join(" AND ");
    // cartridges.location is freeform text, no FK to locations table.
    let sql = format!(
        "SELECT c.id, c.code, m.brand || ' ' || m.model AS model_label, \
               c.location as location_name, cs.name as status_name \
         FROM cartridges c \
         JOIN cartridge_models m ON m.id = c.model_id \
         LEFT JOIN cartridge_statuses cs ON cs.id = c.status_id \
         WHERE {where_clause} \
         ORDER BY c.code ASC, c.id ASC \
         LIMIT 1000"
    );

    let param_refs: Vec<&dyn ToSql> = owned_params.iter().map(|b| b.as_ref()).collect();
    let mut stmt = conn.prepare(&sql).map_err(map_rusqlite)?;
    let row_iter = stmt
        .query_map(param_refs.as_slice(), |r| {
            Ok(ReportRow {
                id: r.get(0)?,
                month_key: None,
                number: None,
                sub_number: None,
                giver_name: None,
                receiver_name: None,
                handover_date_utc: None,
                location_name: r.get(3)?,
                act_type: None,
                device_name: None,
                quantity: None,
                code: r.get(1)?,
                model_label: r.get(2)?,
                status_name: r.get(4)?,
            })
        })
        .map_err(map_rusqlite)?;

    let mut rows = Vec::new();
    for row in row_iter {
        rows.push(row.map_err(map_rusqlite)?);
    }
    let total = rows.len() as i64;
    Ok(ReportResponse { rows, total })
}

// ---------------------------------------------------------------------------
// Helper: next parameter index (1-based, len+1)
// ---------------------------------------------------------------------------

#[inline]
fn next_idx(params: &[Box<dyn ToSql>]) -> usize {
    params.len() + 1
}

// ---------------------------------------------------------------------------
// Month key → Russian heading ("2026-09" → "Сентябрь 2026")
// ---------------------------------------------------------------------------

fn month_key_to_russian(month_key: &str) -> String {
    let parts: Vec<&str> = month_key.split('-').collect();
    if parts.len() != 2 {
        return month_key.to_string();
    }
    let year: i32 = match parts[0].parse() {
        Ok(y) => y,
        Err(_) => return month_key.to_string(),
    };
    let month_num: usize = match parts[1].parse::<usize>() {
        Ok(m) if (1..=12).contains(&m) => m,
        _ => return month_key.to_string(),
    };
    format!("{} {}", MONTH_NAMES_RU[month_num - 1], year)
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn moscow() -> UtcOffset {
        UtcOffset::from_hms(3, 0, 0).unwrap()
    }

    #[test]
    fn period_month_june_2026_moscow() {
        let dto = PeriodDto {
            mode: "month".to_string(),
            year: Some(2026),
            month: Some(6),
            date_from: None,
            date_to: None,
        };
        let (start, end) = compute_period_utc(&dto, moscow());
        // 2026-06-01T00:00:00+03:00 = 2026-05-31T21:00:00Z = Unix 1780261200
        assert_eq!(start, Some(1_780_261_200_i64), "June 2026 start");
        // 2026-06-30T23:59:59+03:00 = Unix 1782853199
        assert_eq!(end, Some(1_782_853_199_i64), "June 2026 end");
    }

    #[test]
    fn period_year_2026_moscow() {
        let dto = PeriodDto {
            mode: "year".to_string(),
            year: Some(2026),
            month: None,
            date_from: None,
            date_to: None,
        };
        let (start, end) = compute_period_utc(&dto, moscow());
        // 2026-01-01T00:00:00+03:00 = 2025-12-31T21:00:00Z = Unix 1767214800
        assert_eq!(start, Some(1_767_214_800_i64));
        // 2026-12-31T23:59:59+03:00 = Unix 1798750799
        assert_eq!(end, Some(1_798_750_799_i64));
    }

    #[test]
    fn period_range_june_2026_moscow() {
        let dto = PeriodDto {
            mode: "range".to_string(),
            year: None,
            month: None,
            date_from: Some("2026-06-01".to_string()),
            date_to: Some("2026-06-30".to_string()),
        };
        let (start, end) = compute_period_utc(&dto, moscow());
        assert_eq!(start, Some(1_780_261_200_i64));
        assert_eq!(end, Some(1_782_853_199_i64));
    }

    #[test]
    fn period_snapshot_mode_returns_none() {
        let dto = PeriodDto {
            mode: "snapshot".to_string(),
            year: None,
            month: None,
            date_from: None,
            date_to: None,
        };
        let (start, end) = compute_period_utc(&dto, moscow());
        assert_eq!(start, None);
        assert_eq!(end, None);
    }

    #[test]
    fn csv_safe_guards_formula_injection() {
        assert_eq!(csv_safe("=SUM(A1)"), "'=SUM(A1)");
        assert_eq!(csv_safe("+foo"), "'+foo");
        assert_eq!(csv_safe("-bar"), "'-bar");
        assert_eq!(csv_safe("@baz"), "'@baz");
        assert_eq!(csv_safe("normal"), "normal");
        assert_eq!(csv_safe(""), "");
    }

    #[test]
    fn month_key_to_russian_converts_correctly() {
        assert_eq!(month_key_to_russian("2026-09"), "Сентябрь 2026");
        assert_eq!(month_key_to_russian("2026-01"), "Январь 2026");
        assert_eq!(month_key_to_russian("2026-12"), "Декабрь 2026");
    }
}
