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
use trackly_infra::repos::requests_sqlite::ad_register_predicate;
use trackly_infra::AppConfig;

use crate::dto::reports::{
    OrgSettingsDto, PeriodDto, ReportCountEntry, ReportCountsDto, ReportFilter, ReportResponse,
    ReportRow,
};
use crate::pdf::PdfRenderer;
use crate::services::organization_service::OrganizationService;

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
                match Date::from_calendar_date(year, Month::try_from(month_num + 1).unwrap(), 1) {
                    Ok(d) => d,
                    Err(_) => return (None, None),
                }
            };
            let last_day = next_month_date.previous_day().unwrap();
            let end = PrimitiveDateTime::new(last_day, Time::from_hms(23, 59, 59).unwrap())
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
            let end = PrimitiveDateTime::new(end_date, Time::from_hms(23, 59, 59).unwrap())
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
// Public helper: Russian period label for report headers/subtitles (34-06 gap fix)
// ---------------------------------------------------------------------------

/// Build a human-readable Russian period label for report headers/subtitles
/// (e.g. the printed form's `.subtitle`). Never panics on missing/malformed
/// optional fields — degrades to a partial or empty label instead of ever
/// emitting the raw English `mode` discriminator.
///
/// - `"month"` + year/month → `"Сентябрь 2026"`
/// - `"month"` + year but missing/out-of-range month → `"2026 год"` (IN-01:
///   identical wording to year mode, so the same underlying year never prints
///   two different subtitles)
/// - `"year"` + year → `"2026 год"`
/// - `"range"` + date_from/date_to (ISO `YYYY-MM-DD`) → `"01.01.2026 — 31.03.2026"`
pub fn format_period_label(dto: &PeriodDto) -> String {
    match dto.mode.as_str() {
        "month" => {
            let year = match dto.year {
                Some(y) => y,
                None => return String::new(),
            };
            match dto.month {
                Some(m) if (1..=12).contains(&m) => {
                    format!("{} {year}", MONTH_NAMES_RU[(m - 1) as usize])
                }
                // IN-01: degrade to the SAME wording year mode uses. A bare
                // "2026" here meant the printed subtitle for one and the same
                // underlying year differed depending on which control the user
                // happened to touch.
                _ => format!("{year} год"),
            }
        }
        "year" => match dto.year {
            Some(y) => format!("{y} год"),
            None => String::new(),
        },
        "range" => {
            let from = dto.date_from.as_deref().and_then(format_ru_short_date);
            let to = dto.date_to.as_deref().and_then(format_ru_short_date);
            match (from, to) {
                (Some(f), Some(t)) => format!("{f} — {t}"),
                (Some(f), None) => f,
                (None, Some(t)) => t,
                (None, None) => String::new(),
            }
        }
        _ => String::new(),
    }
}

/// Parse "YYYY-MM-DD" ISO string → "DD.MM.YYYY". Returns `None` on malformed
/// or out-of-range input (never panics).
fn format_ru_short_date(s: &str) -> Option<String> {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 3 {
        return None;
    }
    let y: i32 = parts[0].parse().ok()?;
    let m: u8 = parts[1].parse().ok()?;
    let d: u8 = parts[2].parse().ok()?;
    let month = Month::try_from(m).ok()?;
    Date::from_calendar_date(y, month, d).ok()?;
    Some(format!("{d:02}.{m:02}.{y:04}"))
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
// Request domain — translation helpers (VAD-03)
// ---------------------------------------------------------------------------

/// Translate `requests.request_type` raw key into Russian for the «Заявки»
/// report domain. Unknown (not in the current CHECK-constrained schema)
/// values fall back to the raw key rather than an empty cell (VAD-03).
fn translate_request_type(raw: &str) -> String {
    match raw {
        "cartridge_replace" => "Замена картриджа".to_string(),
        "free_form" => "Произвольная".to_string(),
        "ad_register" => "Учётная запись AD".to_string(),
        other => other.to_string(),
    }
}

/// Translate `requests.status` raw key into Russian for the «Заявки» report
/// domain. Includes `cancelled` (V031, self-cancel) — already labelled
/// «Отменена» in `RequestListRow.svelte`/`RequestDetail.svelte`, the report
/// must match. Unknown values fall back to the raw key (VAD-03).
fn translate_request_status(raw: &str) -> String {
    match raw {
        "open" => "Открыта".to_string(),
        "in_progress" => "В работе".to_string(),
        "completed" => "Выполнена".to_string(),
        "rejected" => "Отклонена".to_string(),
        "cancelled" => "Отменена".to_string(),
        other => other.to_string(),
    }
}

/// Combine printer name + printer place path into the «Принтер / Место»
/// column value. `None` when the request has no printer selected — an empty
/// cell, not a "—" placeholder (the frontend draws the dash for a null
/// value on screen; CSV/print show a genuinely empty cell).
///
/// Called from `row_field`'s `"printer_place"` arm (CSV/PDF export) only —
/// `query_requests_inner` no longer glues printer_name+place into `ReportRow`
/// at query-build time; the screen (`ReportTable.svelte`) reads
/// `device_name`/`place_path` as two separate fields instead.
fn combine_printer_and_place(
    printer_name: Option<String>,
    printer_place: Option<String>,
) -> Option<String> {
    let name = printer_name?;
    match printer_place {
        Some(loc) if !loc.is_empty() => Some(format!("{name}, {loc}")),
        _ => Some(name),
    }
}

/// Build the WHERE-fragment for `ReportFilter.request_category_filter`
/// (CATF-01/02). `keys == None` -> `None` (no restriction, "Все" — future
/// request types/categories are never silently dropped while "Все" is
/// active). `keys == Some(&[])` -> explicit empty selection, `Some("1 = 0")`
/// (0 rows, NOT a fallback to "Все"). Otherwise builds an OR of per-key
/// predicates; unknown keys are silently skipped (T-260821-w18-01 — allow-list
/// via Rust `match`, no user string is ever concatenated into SQL text; only
/// fixed RU category names are bound via `?N` params). If every supplied key
/// is unrecognised, falls back to `Some("1 = 0")` too.
fn category_filter_clause(
    keys: Option<&[String]>,
    owned_params: &mut Vec<Box<dyn ToSql>>,
) -> Option<String> {
    let keys = keys?;
    if keys.is_empty() {
        return Some("1 = 0".to_string());
    }

    let mut ors: Vec<String> = Vec::new();
    for key in keys {
        match key.as_str() {
            "ad_register" => ors.push("r.request_type = 'ad_register'".to_string()),
            "cartridge_replace" => ors.push("r.request_type = 'cartridge_replace'".to_string()),
            "no_category" => {
                ors.push("(r.request_type = 'free_form' AND r.category_id IS NULL)".to_string())
            }
            "repair" | "consumables" | "software" | "other" => {
                let category_name = match key.as_str() {
                    "repair" => "Ремонт техники",
                    "consumables" => "Расходные материалы",
                    "software" => "Программное обеспечение",
                    "other" => "Прочее",
                    _ => unreachable!(),
                };
                let idx = next_idx(owned_params);
                owned_params.push(Box::new(category_name.to_string()));
                ors.push(format!(
                    "(r.request_type = 'free_form' AND r.category_id = \
                     (SELECT id FROM request_categories WHERE name = ?{idx}))"
                ));
            }
            _ => {
                // Unknown/future key — silently skipped, not an error.
            }
        }
    }

    if ors.is_empty() {
        return Some("1 = 0".to_string());
    }
    Some(format!("({})", ors.join(" OR ")))
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
    /// D-13-style freeze (Phase 17): the krilla renderer handle is no longer
    /// invoked on this service's active path — `export_pdf` renders HTML via
    /// `build_safe_html_env` instead. Kept only because `ReportService::new`'s
    /// constructor signature is used by ~5 existing call sites (context.rs,
    /// http/health.rs, tauri_cmds/health.rs, and several test fixtures).
    pub pdf: Arc<PdfRenderer>,
    /// Phase 17: source of `Paths` for `templates/report.html` resolution
    /// (file-first + embedded fallback, mirrors `ActService::organization`).
    /// Option-typed so the existing 5 `ReportService::new(...)` call sites
    /// (context.rs, http/health.rs, tauri_cmds/health.rs,
    /// tests/report_csv_export.rs, tests/specta_roundtrip.rs) keep compiling
    /// unchanged.
    pub(crate) organization: Option<Arc<OrganizationService>>,
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
            organization: None,
        }
    }

    /// Builder: подключить `OrganizationService` (Phase 17) — источник
    /// `Paths` для `templates/report.html` file-first resolution. Mirrors
    /// `ActService::with_pdf_pipeline`'s organization wiring.
    pub fn with_organization(mut self, organization: Arc<OrganizationService>) -> Self {
        self.organization = Some(organization);
        self
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

    /// RPT-01 / RPT-04 / RPT-05: acts (handover) filtered by period, type, place.
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

    /// RPT-05: returns (return acts) filtered by period and place.
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
    // Request reports (VAD-01..04)
    // -----------------------------------------------------------------------

    /// VAD-01: all requests in the period (no status filter — includes `rejected`).
    ///
    /// CATF-01/02: `filter.request_category_filter` narrows the «Заявки»
    /// domain by request type/category (funnel filter). `None` = «Все».
    pub async fn list_requests_all(
        &self,
        filter: ReportFilter,
        period: PeriodDto,
        exclude_ad_register: bool,
    ) -> Result<ReportResponse, AppError> {
        let tz = self.get_tz_offset();
        let (ts_from, ts_to) = compute_period_utc(&period, tz);
        let readers = self.readers.clone();
        tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            query_requests_inner(
                &conn,
                ts_from,
                ts_to,
                None,
                exclude_ad_register,
                filter.request_category_filter.as_deref(),
                filter.is_storage,
                filter.place_id,
            )
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking list_requests_all: {e}"),
        })?
    }

    /// VAD-01: requests with `status = 'open'` in the period.
    pub async fn list_requests_open(
        &self,
        filter: ReportFilter,
        period: PeriodDto,
        exclude_ad_register: bool,
    ) -> Result<ReportResponse, AppError> {
        let tz = self.get_tz_offset();
        let (ts_from, ts_to) = compute_period_utc(&period, tz);
        let readers = self.readers.clone();
        tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            query_requests_inner(
                &conn,
                ts_from,
                ts_to,
                Some("open"),
                exclude_ad_register,
                filter.request_category_filter.as_deref(),
                filter.is_storage,
                filter.place_id,
            )
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking list_requests_open: {e}"),
        })?
    }

    /// VAD-01: requests with `status = 'in_progress'` in the period.
    pub async fn list_requests_in_progress(
        &self,
        filter: ReportFilter,
        period: PeriodDto,
        exclude_ad_register: bool,
    ) -> Result<ReportResponse, AppError> {
        let tz = self.get_tz_offset();
        let (ts_from, ts_to) = compute_period_utc(&period, tz);
        let readers = self.readers.clone();
        tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            query_requests_inner(
                &conn,
                ts_from,
                ts_to,
                Some("in_progress"),
                exclude_ad_register,
                filter.request_category_filter.as_deref(),
                filter.is_storage,
                filter.place_id,
            )
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking list_requests_in_progress: {e}"),
        })?
    }

    /// VAD-01: requests with `status = 'completed'` in the period.
    pub async fn list_requests_completed(
        &self,
        filter: ReportFilter,
        period: PeriodDto,
        exclude_ad_register: bool,
    ) -> Result<ReportResponse, AppError> {
        let tz = self.get_tz_offset();
        let (ts_from, ts_to) = compute_period_utc(&period, tz);
        let readers = self.readers.clone();
        tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            query_requests_inner(
                &conn,
                ts_from,
                ts_to,
                Some("completed"),
                exclude_ad_register,
                filter.request_category_filter.as_deref(),
                filter.is_storage,
                filter.place_id,
            )
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking list_requests_completed: {e}"),
        })?
    }

    // -----------------------------------------------------------------------
    // Per-tab count query (G2-5b)
    // -----------------------------------------------------------------------

    /// Return COUNT(*)-only totals for every report-type tab in the active domain.
    ///
    /// All 4 counts run inside a single `spawn_blocking` task so there is only
    /// one round-trip to the reader pool.  Individual count failures return 0
    /// (non-fatal — badge shows 0 rather than breaking the page).
    pub async fn get_report_counts(
        &self,
        domain: &str,
        filter: ReportFilter,
        period: PeriodDto,
        exclude_ad_register: bool,
    ) -> Result<ReportCountsDto, AppError> {
        let tz = self.get_tz_offset();
        let (ts_from, ts_to) = compute_period_utc(&period, tz);
        let readers = self.readers.clone();
        let domain = domain.to_string();
        tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            let counts = if domain == "devices" {
                vec![
                    ReportCountEntry {
                        key: "acts".into(),
                        count: count_acts_inner(&conn, &filter, ts_from, ts_to, "handover")
                            .unwrap_or(0),
                    },
                    ReportCountEntry {
                        key: "returns".into(),
                        count: count_acts_inner(&conn, &filter, ts_from, ts_to, "return")
                            .unwrap_or(0),
                    },
                    ReportCountEntry {
                        key: "in_use".into(),
                        count: count_device_snapshot(&conn, &filter, "В работе").unwrap_or(0),
                    },
                    ReportCountEntry {
                        key: "in_stock".into(),
                        count: count_device_snapshot(&conn, &filter, "На складе").unwrap_or(0),
                    },
                ]
            } else if domain == "cartridges" {
                vec![
                    ReportCountEntry {
                        key: "consumption".into(),
                        count: count_cartridge_audit_inner(
                            &conn,
                            &filter,
                            ts_from,
                            ts_to,
                            &["custom:install"],
                        )
                        .unwrap_or(0),
                    },
                    ReportCountEntry {
                        key: "refills".into(),
                        count: count_cartridge_audit_inner(
                            &conn,
                            &filter,
                            ts_from,
                            ts_to,
                            &["custom:to_refill", "custom:from_refill"],
                        )
                        .unwrap_or(0),
                    },
                    ReportCountEntry {
                        key: "in_use".into(),
                        count: count_cartridge_snapshot_inner(&conn, &filter, "В работе")
                            .unwrap_or(0),
                    },
                    ReportCountEntry {
                        key: "in_stock".into(),
                        count: count_cartridge_snapshot_inner(&conn, &filter, "На складе")
                            .unwrap_or(0),
                    },
                ]
            } else if domain == "requests" {
                let category_filter = filter.request_category_filter.as_deref();
                vec![
                    ReportCountEntry {
                        key: "all".into(),
                        count: count_requests_inner(
                            &conn,
                            ts_from,
                            ts_to,
                            None,
                            exclude_ad_register,
                            category_filter,
                            filter.place_id,
                        )
                        .unwrap_or(0),
                    },
                    ReportCountEntry {
                        key: "open".into(),
                        count: count_requests_inner(
                            &conn,
                            ts_from,
                            ts_to,
                            Some("open"),
                            exclude_ad_register,
                            category_filter,
                            filter.place_id,
                        )
                        .unwrap_or(0),
                    },
                    ReportCountEntry {
                        key: "in_progress".into(),
                        count: count_requests_inner(
                            &conn,
                            ts_from,
                            ts_to,
                            Some("in_progress"),
                            exclude_ad_register,
                            category_filter,
                            filter.place_id,
                        )
                        .unwrap_or(0),
                    },
                    ReportCountEntry {
                        key: "completed".into(),
                        count: count_requests_inner(
                            &conn,
                            ts_from,
                            ts_to,
                            Some("completed"),
                            exclude_ad_register,
                            category_filter,
                            filter.place_id,
                        )
                        .unwrap_or(0),
                    },
                ]
            } else {
                Vec::new()
            };
            Ok::<ReportCountsDto, AppError>(ReportCountsDto { counts })
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking get_report_counts: {e}"),
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
        let tz = self.get_tz_offset();
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
                    let raw = row_field(row, col, tz);
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
    // HTML export (RPT-08, Phase 17: migrated off krilla/DocSpec)
    // -----------------------------------------------------------------------

    /// Export report as a self-contained HTML string, rendered from
    /// `templates/report.html` (file-first + embedded fallback) via
    /// `build_safe_html_env` (Phase 17, Req 1/2). Mirrors the HTML-print
    /// pipeline shipped for acts in Phase 16 (`act_service.rs::render_pdf`).
    ///
    /// `columns` (keys, e.g. `"giver_name"`) remains the sole source of cell
    /// values via `row_field(row, col, tz)` — unchanged by the D-03/CR-01 fix.
    /// `column_labels` (Russian labels, e.g. `"Сдал"`) is the NEW source of
    /// the header row (`ctx["columns"]`); `columns_for`/`column_labels_for`
    /// in `tauri_cmds/reports.rs` are index-aligned so `columns[i]` and
    /// `column_labels[i]` refer to the same logical column.
    #[allow(clippy::too_many_arguments)]
    pub async fn export_pdf(
        &self,
        rows: &ReportResponse,
        report_name: &str,
        period_label: &str,
        org: &OrgSettingsDto,
        logo_bytes: Option<Vec<u8>>,
        logo_mime: Option<String>,
        columns: &[&str],
        column_labels: &[&str],
    ) -> Result<String, AppError> {
        let organization = self
            .organization
            .as_ref()
            .ok_or_else(|| AppError::Internal {
                source_chain: "ReportService::export_pdf called without with_organization".into(),
            })?;
        let tz = self.get_tz_offset();

        // T-17-01-01 mitigation: `logo_bytes` originates exclusively from
        // `OrgDbService`-sourced org_settings BLOB (see build_reports_export_pdf
        // caller) — never from request-supplied bytes.
        //
        // Phase 17's WR-05 mitigation (read-side mime allowlist, mirroring
        // `OrgDbService::save_logo`'s write-side one) now lives in
        // `pdf::minijinja_env::logo_data_uri` — Phase 34's WR-01 extracted it
        // there so the two act render paths, which feed the SAME shared
        // `_header.html` `| safe` sink, enforce it identically instead of
        // this being the only guarded path of three.
        let logo_data_uri: Option<String> =
            crate::pdf::minijinja_env::logo_data_uri(logo_bytes, logo_mime.as_deref());

        // Phase 17 (D-02/D-08): read the HTML template source from
        // templates/report.html (file-first, embedded-default fallback)
        // via the same mechanism as the act templates (Phase 16).
        let templates_dir = crate::pdf::html_templates::resolve_templates_dir(&organization.paths);
        let embedded_default = crate::pdf::html_templates::DEFAULT_HTML_TEMPLATES
            .iter()
            .find(|(f, _)| *f == "report.html")
            .map(|(_, body)| *body)
            .unwrap_or("");
        let template_src = crate::pdf::html_templates::load_template(
            &templates_dir,
            "report.html",
            embedded_default,
        );
        let embedded_header_default = crate::pdf::html_templates::DEFAULT_HTML_TEMPLATES
            .iter()
            .find(|(f, _)| *f == "_header.html")
            .map(|(_, body)| *body)
            .unwrap_or("");
        let header_src = crate::pdf::html_templates::load_template(
            &templates_dir,
            "_header.html",
            embedded_header_default,
        );

        // Month-grouping (D-04) — same algorithm as before, now accumulating
        // serde_json group objects instead of DocSpec Sections. Empty-case
        // fallback message lives in the template (D-07), not here.
        let mut groups: Vec<serde_json::Value> = Vec::new();
        let mut current_month: Option<String> = None;
        let mut table_rows: Vec<Vec<String>> = Vec::new();

        for row in &rows.rows {
            let month_key = row.month_key.as_deref().unwrap_or("");

            if !month_key.is_empty() && Some(month_key) != current_month.as_deref() {
                if !table_rows.is_empty() {
                    groups.push(serde_json::json!({
                        "month_label": month_key_to_russian(current_month.as_deref().unwrap_or("")),
                        "rows": std::mem::take(&mut table_rows),
                    }));
                }
                current_month = Some(month_key.to_string());
            }

            table_rows.push(columns.iter().map(|col| row_field(row, col, tz)).collect());
        }

        if !table_rows.is_empty() {
            groups.push(serde_json::json!({
                "month_label": month_key_to_russian(current_month.as_deref().unwrap_or("")),
                "rows": table_rows,
            }));
        }

        let ctx = serde_json::json!({
            "org": {
                "name": org.org_name,
                "inn": org.inn,
                "kpp": org.kpp,
                "address": org.address,
                "address_line2": org.address_line2,
                "phone": org.phone,
                "fax": org.fax,
                "email": org.email,
                "okpo": org.okpo,
                "ogrn": org.ogrn,
                "full_name": crate::pdf::minijinja_env::org_full_name_html(&org.full_name),
                "logo_data_uri": logo_data_uri,
            },
            "report_name": report_name,
            "period_label": period_label,
            "columns": column_labels,
            "groups": groups,
        });

        crate::pdf::minijinja_env::render_with_timeout(
            &crate::pdf::minijinja_env::build_safe_html_env(),
            "report_html",
            &template_src,
            ctx,
            &[("_header.html", &header_src)],
        )
        .await
    }
}

// ---------------------------------------------------------------------------
// ReportRow field accessor
// ---------------------------------------------------------------------------

fn row_field(row: &ReportRow, col: &str, tz: UtcOffset) -> String {
    match col {
        "number" => row.number.as_deref().unwrap_or("").to_string(),
        "sub_number" => row.sub_number.as_deref().unwrap_or("").to_string(),
        "giver_name" => row.giver_name.as_deref().unwrap_or("").to_string(),
        "receiver_name" => row.receiver_name.as_deref().unwrap_or("").to_string(),
        "handover_date_utc" => row
            .handover_date_utc
            .map(|ts| format_handover_date(ts, tz))
            .unwrap_or_default(),
        "place_path" => row.place_path.as_deref().unwrap_or("").to_string(),
        "printer_place" => {
            combine_printer_and_place(row.device_name.clone(), row.place_path.clone())
                .unwrap_or_default()
        }
        "act_type" => row.act_type.as_deref().unwrap_or("").to_string(),
        "device_name" => row.device_name.as_deref().unwrap_or("").to_string(),
        "quantity" => row.quantity.map(|q| q.to_string()).unwrap_or_default(),
        "code" => row.code.as_deref().unwrap_or("").to_string(),
        "model_label" => row.model_label.as_deref().unwrap_or("").to_string(),
        "status_name" => row.status_name.as_deref().unwrap_or("").to_string(),
        "month_key" => row.month_key.as_deref().unwrap_or("").to_string(),
        "request_type_label" => row.request_type_label.as_deref().unwrap_or("").to_string(),
        _ => String::new(),
    }
}

/// «27.08.26, 14:35» — читаемая дата+время колонки `handover_date_utc` в
/// таймзоне организации (WSU-01/WSU-02). Двузначный год — по явной просьбе
/// владельца продукта; неоднозначность 20xx/19xx не встаёт (отчёты покрывают
/// текущий/недавний период). При невалидном `unix_seconds` — пустая строка
/// (сохраняет конвенцию пустой ячейки для отсутствующего timestamp, см.
/// `combine_printer_and_place`).
fn format_handover_date(unix_seconds: i64, tz: UtcOffset) -> String {
    let odt = match time::OffsetDateTime::from_unix_timestamp(unix_seconds) {
        Ok(odt) => odt,
        Err(_) => return String::new(),
    };
    let local = odt.to_offset(tz);
    format!(
        "{:02}.{:02}.{:02}, {:02}:{:02}",
        local.day(),
        local.month() as u8,
        local.year().rem_euclid(100),
        local.hour(),
        local.minute()
    )
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
    let mut with_prefix = String::new();

    clauses.push(format!("a.act_type = ?{}", next_idx(&owned_params)));
    owned_params.push(Box::new(act_type.to_string()));

    clauses.push("a.deleted_at_utc IS NULL".to_string());

    if let Some(from) = ts_from {
        clauses.push(format!(
            "a.handover_date_utc >= ?{}",
            next_idx(&owned_params)
        ));
        owned_params.push(Box::new(from));
    }
    if let Some(to) = ts_to {
        clauses.push(format!(
            "a.handover_date_utc <= ?{}",
            next_idx(&owned_params)
        ));
        owned_params.push(Box::new(to));
    }
    // D-28: subtree-inclusive place filter — choosing a place captures it
    // and every place nested under it, not just an exact place_id match.
    if let Some(place_id) = filter.place_id {
        let idx = next_idx(&owned_params);
        owned_params.push(Box::new(place_id));
        with_prefix.push_str(&format!(
            "WITH RECURSIVE subtree(id) AS ( \
                 SELECT id FROM places WHERE id = ?{idx} AND deleted_at_utc IS NULL \
                 UNION ALL \
                 SELECT p.id FROM places p JOIN subtree s ON p.parent_id = s.id \
                 WHERE p.deleted_at_utc IS NULL \
             ) "
        ));
        clauses.push("a.place_id IN (SELECT id FROM subtree)".to_string());
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
    // D-11.2/D-11.4: geographic "на складе" quick filter — ancestor-inclusive
    // storage-place membership (self or any ancestor is_storage), a
    // dimension separate from item status (D-11.5) — status_id is untouched.
    if let Some(want_storage) = filter.is_storage {
        let storage_cte = "storage_ids(id) AS ( \
                 SELECT id FROM places WHERE is_storage = 1 AND deleted_at_utc IS NULL \
                 UNION ALL \
                 SELECT p.id FROM places p JOIN storage_ids s ON p.parent_id = s.id \
                 WHERE p.deleted_at_utc IS NULL \
             ) ";
        if with_prefix.is_empty() {
            with_prefix = format!("WITH RECURSIVE {storage_cte}");
        } else {
            with_prefix = format!("{}, {storage_cte}", with_prefix.trim_end());
        }
        if want_storage {
            clauses.push("a.place_id IN (SELECT id FROM storage_ids)".to_string());
        } else {
            clauses.push(
                "(a.place_id IS NULL OR a.place_id NOT IN (SELECT id FROM storage_ids))"
                    .to_string(),
            );
        }
    }

    let where_clause = clauses.join(" AND ");
    let sql = format!(
        "{with_prefix}SELECT a.id, \
               strftime('%Y-%m', datetime(a.handover_date_utc, 'unixepoch', '+3 hours')) AS month_key, \
               CAST(a.number AS TEXT) as number, \
               CAST(a.sub_number AS TEXT) as sub_number, \
               a.giver_name, a.receiver_name, \
               a.handover_date_utc, \
               pfp.full_path AS place_path, \
               a.act_type, \
               GROUP_CONCAT(d.name, ', ') AS device_name, \
               SUM(ai.quantity) AS quantity \
         FROM acts a \
         LEFT JOIN place_full_paths pfp ON pfp.place_id = a.place_id \
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
                place_path: r.get(7)?,
                act_type: r.get(8)?,
                device_name: r.get(9)?,
                quantity: r.get(10)?,
                code: None,
                model_label: None,
                status_name: None,
                request_type_label: None,
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
    let mut with_prefix = String::new();

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
    // D-28: subtree-inclusive place filter — choosing a place captures it
    // and every place nested under it, not just an exact place_id match.
    if let Some(place_id) = filter.place_id {
        let idx = next_idx(&owned_params);
        owned_params.push(Box::new(place_id));
        with_prefix.push_str(&format!(
            "WITH RECURSIVE subtree(id) AS ( \
                 SELECT id FROM places WHERE id = ?{idx} AND deleted_at_utc IS NULL \
                 UNION ALL \
                 SELECT p.id FROM places p JOIN subtree s ON p.parent_id = s.id \
                 WHERE p.deleted_at_utc IS NULL \
             ) "
        ));
        clauses.push("d.place_id IN (SELECT id FROM subtree)".to_string());
    }
    if let Some(type_id) = filter.type_id {
        clauses.push(format!("d.type_id = ?{}", next_idx(&owned_params)));
        owned_params.push(Box::new(type_id));
    }
    // D-11.2/D-11.4: geographic "на складе" quick filter — ancestor-inclusive
    // storage-place membership (self or any ancestor is_storage), a
    // dimension separate from item status (D-11.5) — status_id is untouched.
    if let Some(want_storage) = filter.is_storage {
        let storage_cte = "storage_ids(id) AS ( \
                 SELECT id FROM places WHERE is_storage = 1 AND deleted_at_utc IS NULL \
                 UNION ALL \
                 SELECT p.id FROM places p JOIN storage_ids s ON p.parent_id = s.id \
                 WHERE p.deleted_at_utc IS NULL \
             ) ";
        if with_prefix.is_empty() {
            with_prefix = format!("WITH RECURSIVE {storage_cte}");
        } else {
            with_prefix = format!("{}, {storage_cte}", with_prefix.trim_end());
        }
        if want_storage {
            clauses.push("d.place_id IN (SELECT id FROM storage_ids)".to_string());
        } else {
            clauses.push(
                "(d.place_id IS NULL OR d.place_id NOT IN (SELECT id FROM storage_ids))"
                    .to_string(),
            );
        }
    }

    let where_clause = clauses.join(" AND ");
    let sql = format!(
        "{with_prefix}SELECT d.id, NULL as month_key, d.name as device_name, d.serial_number, \
               pfp.full_path as place_path, s.name as status_name \
         FROM devices d \
         LEFT JOIN place_full_paths pfp ON pfp.place_id = d.place_id \
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
                place_path: r.get::<_, Option<String>>(4)?,
                act_type: None,
                device_name: r.get(2)?,
                quantity: None,
                code: r.get(3)?, // serial_no in code field
                model_label: None,
                status_name: r.get(5)?,
                request_type_label: None,
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
    let mut with_prefix = String::new();

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
    // D-28: subtree-inclusive place filter — mirrors query_acts_inner; merge-safe
    // with_prefix composition so a simultaneous is_storage filter below does not
    // clobber this CTE.
    if let Some(place_id) = filter.place_id {
        let idx = next_idx(&owned_params);
        owned_params.push(Box::new(place_id));
        let subtree_cte = format!(
            "subtree(id) AS ( \
                 SELECT id FROM places WHERE id = ?{idx} AND deleted_at_utc IS NULL \
                 UNION ALL \
                 SELECT p.id FROM places p JOIN subtree s ON p.parent_id = s.id \
                 WHERE p.deleted_at_utc IS NULL \
             ) "
        );
        if with_prefix.is_empty() {
            with_prefix = format!("WITH RECURSIVE {subtree_cte}");
        } else {
            with_prefix = format!("{}, {subtree_cte}", with_prefix.trim_end());
        }
        clauses.push("c.place_id IN (SELECT id FROM subtree)".to_string());
    }
    // D-11.2/D-11.4: geographic "на складе" quick filter — ancestor-inclusive
    // storage-place membership on the cartridge's own place_id (Plan 09 gave
    // cartridges a real place_id FK); independent of item status (D-11.5).
    if let Some(want_storage) = filter.is_storage {
        let storage_cte = "storage_ids(id) AS ( \
                 SELECT id FROM places WHERE is_storage = 1 AND deleted_at_utc IS NULL \
                 UNION ALL \
                 SELECT p.id FROM places p JOIN storage_ids s ON p.parent_id = s.id \
                 WHERE p.deleted_at_utc IS NULL \
             ) ";
        if with_prefix.is_empty() {
            with_prefix = format!("WITH RECURSIVE {storage_cte}");
        } else {
            with_prefix = format!("{}, {storage_cte}", with_prefix.trim_end());
        }
        if want_storage {
            clauses.push("c.place_id IN (SELECT id FROM storage_ids)".to_string());
        } else {
            clauses.push(
                "(c.place_id IS NULL OR c.place_id NOT IN (SELECT id FROM storage_ids))"
                    .to_string(),
            );
        }
    }

    let where_clause = clauses.join(" AND ");
    let sql = format!(
        "{with_prefix}SELECT c.id, \
               strftime('%Y-%m', datetime(al.created_at_utc, 'unixepoch', '+3 hours')) AS month_key, \
               al.created_at_utc as handover_date_utc, \
               pfp.full_path as place_path, \
               m.brand || ' ' || m.model AS model_label, \
               c.code, \
               al.action \
         FROM audit_log al \
         JOIN cartridges c ON c.id = al.entity_id \
         JOIN cartridge_models m ON m.id = c.model_id \
         LEFT JOIN place_full_paths pfp ON pfp.place_id = c.place_id \
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
                place_path: r.get(3)?,
                act_type: r.get(6)?,
                device_name: None,
                quantity: None,
                code: r.get(5)?,
                model_label: r.get(4)?,
                status_name: None,
                request_type_label: None,
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
    let mut with_prefix = String::new();

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
    // D-28: subtree-inclusive place filter — mirrors query_acts_inner; merge-safe
    // with_prefix composition so a simultaneous is_storage filter below does not
    // clobber this CTE.
    if let Some(place_id) = filter.place_id {
        let idx = next_idx(&owned_params);
        owned_params.push(Box::new(place_id));
        let subtree_cte = format!(
            "subtree(id) AS ( \
                 SELECT id FROM places WHERE id = ?{idx} AND deleted_at_utc IS NULL \
                 UNION ALL \
                 SELECT p.id FROM places p JOIN subtree s ON p.parent_id = s.id \
                 WHERE p.deleted_at_utc IS NULL \
             ) "
        );
        if with_prefix.is_empty() {
            with_prefix = format!("WITH RECURSIVE {subtree_cte}");
        } else {
            with_prefix = format!("{}, {subtree_cte}", with_prefix.trim_end());
        }
        clauses.push("c.place_id IN (SELECT id FROM subtree)".to_string());
    }
    // D-11.2/D-11.4: geographic "на складе" quick filter — ancestor-inclusive
    // storage-place membership on the cartridge's own place_id; independent
    // of item status (D-11.5).
    if let Some(want_storage) = filter.is_storage {
        let storage_cte = "storage_ids(id) AS ( \
                 SELECT id FROM places WHERE is_storage = 1 AND deleted_at_utc IS NULL \
                 UNION ALL \
                 SELECT p.id FROM places p JOIN storage_ids s ON p.parent_id = s.id \
                 WHERE p.deleted_at_utc IS NULL \
             ) ";
        if with_prefix.is_empty() {
            with_prefix = format!("WITH RECURSIVE {storage_cte}");
        } else {
            with_prefix = format!("{}, {storage_cte}", with_prefix.trim_end());
        }
        if want_storage {
            clauses.push("c.place_id IN (SELECT id FROM storage_ids)".to_string());
        } else {
            clauses.push(
                "(c.place_id IS NULL OR c.place_id NOT IN (SELECT id FROM storage_ids))"
                    .to_string(),
            );
        }
    }

    let where_clause = clauses.join(" AND ");
    let sql = format!(
        "{with_prefix}SELECT c.id, c.code, m.brand || ' ' || m.model AS model_label, \
               pfp.full_path as place_path, cs.name as status_name \
         FROM cartridges c \
         JOIN cartridge_models m ON m.id = c.model_id \
         LEFT JOIN place_full_paths pfp ON pfp.place_id = c.place_id \
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
                place_path: r.get(3)?,
                act_type: None,
                device_name: None,
                quantity: None,
                code: r.get(1)?,
                model_label: r.get(2)?,
                status_name: r.get(4)?,
                request_type_label: None,
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

/// Query `requests` rows filtered by period, optional status, and RBAC
/// exclusion of `ad_register` (REQ-06/T-09-11). Shared by all four
/// `requests_*` report tabs — the tab distinguishes itself only by
/// `status_filter` (VAD-01).
#[allow(clippy::too_many_arguments)]
fn query_requests_inner(
    conn: &rusqlite::Connection,
    ts_from: Option<i64>,
    ts_to: Option<i64>,
    status_filter: Option<&str>,
    exclude_ad_register: bool,
    category_filter: Option<&[String]>,
    is_storage: Option<bool>,
    place_id: Option<i64>,
) -> Result<ReportResponse, AppError> {
    let mut clauses: Vec<String> = vec!["r.deleted_at_utc IS NULL".to_string()];
    let mut owned_params: Vec<Box<dyn ToSql>> = Vec::new();
    let mut with_prefix = String::new();

    if let Some(status) = status_filter {
        clauses.push(format!("r.status = ?{}", next_idx(&owned_params)));
        owned_params.push(Box::new(status.to_string()));
    }
    if exclude_ad_register {
        clauses.push(ad_register_predicate("r."));
    }
    if let Some(clause) = category_filter_clause(category_filter, &mut owned_params) {
        clauses.push(clause);
    }
    if let Some(from) = ts_from {
        clauses.push(format!("r.created_at_utc >= ?{}", next_idx(&owned_params)));
        owned_params.push(Box::new(from));
    }
    if let Some(to) = ts_to {
        clauses.push(format!("r.created_at_utc <= ?{}", next_idx(&owned_params)));
        owned_params.push(Box::new(to));
    }
    // D-28: subtree-inclusive place filter, applied to the request's printer's
    // own place_id (requests have no place_id of their own — filter follows
    // the printer via the LEFT JOIN devices d below); merge-safe with_prefix
    // composition so a simultaneous is_storage filter does not clobber this CTE.
    if let Some(place_id) = place_id {
        let idx = next_idx(&owned_params);
        owned_params.push(Box::new(place_id));
        let subtree_cte = format!(
            "subtree(id) AS ( \
                 SELECT id FROM places WHERE id = ?{idx} AND deleted_at_utc IS NULL \
                 UNION ALL \
                 SELECT p.id FROM places p JOIN subtree s ON p.parent_id = s.id \
                 WHERE p.deleted_at_utc IS NULL \
             ) "
        );
        if with_prefix.is_empty() {
            with_prefix = format!("WITH RECURSIVE {subtree_cte}");
        } else {
            with_prefix = format!("{}, {subtree_cte}", with_prefix.trim_end());
        }
        clauses.push("d.place_id IN (SELECT id FROM subtree)".to_string());
    }
    // D-11.2/D-11.4: geographic "на складе" quick filter, applied to the
    // request's printer's own place_id; independent of item status (D-11.5).
    if let Some(want_storage) = is_storage {
        let storage_cte = "storage_ids(id) AS ( \
                 SELECT id FROM places WHERE is_storage = 1 AND deleted_at_utc IS NULL \
                 UNION ALL \
                 SELECT p.id FROM places p JOIN storage_ids s ON p.parent_id = s.id \
                 WHERE p.deleted_at_utc IS NULL \
             ) ";
        if with_prefix.is_empty() {
            with_prefix = format!("WITH RECURSIVE {storage_cte}");
        } else {
            with_prefix = format!("{}, {storage_cte}", with_prefix.trim_end());
        }
        if want_storage {
            clauses.push("d.place_id IN (SELECT id FROM storage_ids)".to_string());
        } else {
            clauses.push(
                "(d.place_id IS NULL OR d.place_id NOT IN (SELECT id FROM storage_ids))"
                    .to_string(),
            );
        }
    }

    let where_clause = clauses.join(" AND ");
    let sql = format!(
        "{with_prefix}SELECT r.id, \
               strftime('%Y-%m', datetime(r.created_at_utc, 'unixepoch', '+3 hours')) AS month_key, \
               r.created_at_utc, r.request_type, r.status, u.full_name AS requester_name, \
               d.name AS printer_name, pfp.full_path AS printer_place \
         FROM requests r \
         LEFT JOIN users u ON u.id = r.requested_by_user_id \
         LEFT JOIN devices d ON d.id = r.printer_device_id \
         LEFT JOIN place_full_paths pfp ON pfp.place_id = d.place_id \
         WHERE {where_clause} \
         ORDER BY r.created_at_utc ASC, r.id ASC \
         LIMIT 1000"
    );

    let param_refs: Vec<&dyn ToSql> = owned_params.iter().map(|b| b.as_ref()).collect();
    let mut stmt = conn.prepare(&sql).map_err(map_rusqlite)?;
    let row_iter = stmt
        .query_map(param_refs.as_slice(), |r| {
            let id: i64 = r.get(0)?;
            let request_type: String = r.get(3)?;
            let status: String = r.get(4)?;
            let printer_name: Option<String> = r.get(6)?;
            let printer_place: Option<String> = r.get(7)?;
            Ok(ReportRow {
                id,
                month_key: r.get(1)?,
                number: Some(id.to_string()),
                sub_number: None,
                giver_name: r.get(5)?,
                receiver_name: None,
                handover_date_utc: r.get(2)?,
                place_path: printer_place,
                act_type: None,
                device_name: printer_name,
                quantity: None,
                code: None,
                model_label: None,
                status_name: Some(translate_request_status(&status)),
                request_type_label: Some(translate_request_type(&request_type)),
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

/// COUNT(*) variant of `query_requests_inner` — same WHERE clauses, no joins
/// or row collection needed for the COUNT.
fn count_requests_inner(
    conn: &rusqlite::Connection,
    ts_from: Option<i64>,
    ts_to: Option<i64>,
    status_filter: Option<&str>,
    exclude_ad_register: bool,
    category_filter: Option<&[String]>,
    place_id: Option<i64>,
) -> Result<i64, AppError> {
    let mut clauses: Vec<String> = vec!["r.deleted_at_utc IS NULL".to_string()];
    let mut owned_params: Vec<Box<dyn ToSql>> = Vec::new();
    let mut with_prefix = String::new();

    if let Some(status) = status_filter {
        clauses.push(format!("r.status = ?{}", next_idx(&owned_params)));
        owned_params.push(Box::new(status.to_string()));
    }
    if exclude_ad_register {
        clauses.push(ad_register_predicate("r."));
    }
    if let Some(clause) = category_filter_clause(category_filter, &mut owned_params) {
        clauses.push(clause);
    }
    if let Some(from) = ts_from {
        clauses.push(format!("r.created_at_utc >= ?{}", next_idx(&owned_params)));
        owned_params.push(Box::new(from));
    }
    if let Some(to) = ts_to {
        clauses.push(format!("r.created_at_utc <= ?{}", next_idx(&owned_params)));
        owned_params.push(Box::new(to));
    }
    // D-28: subtree-inclusive place filter, applied to the request's printer's
    // own place_id via the LEFT JOIN devices d below — mirrors query_requests_inner.
    if let Some(place_id) = place_id {
        let idx = next_idx(&owned_params);
        owned_params.push(Box::new(place_id));
        let subtree_cte = format!(
            "subtree(id) AS ( \
                 SELECT id FROM places WHERE id = ?{idx} AND deleted_at_utc IS NULL \
                 UNION ALL \
                 SELECT p.id FROM places p JOIN subtree s ON p.parent_id = s.id \
                 WHERE p.deleted_at_utc IS NULL \
             ) "
        );
        if with_prefix.is_empty() {
            with_prefix = format!("WITH RECURSIVE {subtree_cte}");
        } else {
            with_prefix = format!("{}, {subtree_cte}", with_prefix.trim_end());
        }
        clauses.push("d.place_id IN (SELECT id FROM subtree)".to_string());
    }

    let where_clause = clauses.join(" AND ");
    let sql = format!(
        "{with_prefix}SELECT COUNT(*) FROM requests r \
         LEFT JOIN devices d ON d.id = r.printer_device_id \
         WHERE {where_clause}"
    );

    let param_refs: Vec<&dyn ToSql> = owned_params.iter().map(|b| b.as_ref()).collect();
    conn.query_row(&sql, param_refs.as_slice(), |r| r.get(0))
        .map_err(map_rusqlite)
}

// ---------------------------------------------------------------------------
// COUNT(*)-only helpers (used by get_report_counts — G2-5b)
// ---------------------------------------------------------------------------

/// COUNT(*) variant of query_acts_inner — same WHERE clauses, no row collection.
fn count_acts_inner(
    conn: &rusqlite::Connection,
    filter: &ReportFilter,
    ts_from: Option<i64>,
    ts_to: Option<i64>,
    act_type: &str,
) -> Result<i64, AppError> {
    let mut clauses: Vec<String> = Vec::new();
    let mut owned_params: Vec<Box<dyn ToSql>> = Vec::new();
    let mut with_prefix = String::new();

    clauses.push(format!("a.act_type = ?{}", next_idx(&owned_params)));
    owned_params.push(Box::new(act_type.to_string()));

    clauses.push("a.deleted_at_utc IS NULL".to_string());

    if let Some(from) = ts_from {
        clauses.push(format!(
            "a.handover_date_utc >= ?{}",
            next_idx(&owned_params)
        ));
        owned_params.push(Box::new(from));
    }
    if let Some(to) = ts_to {
        clauses.push(format!(
            "a.handover_date_utc <= ?{}",
            next_idx(&owned_params)
        ));
        owned_params.push(Box::new(to));
    }
    // D-28: subtree-inclusive place filter — mirrors query_acts_inner.
    if let Some(place_id) = filter.place_id {
        let idx = next_idx(&owned_params);
        owned_params.push(Box::new(place_id));
        with_prefix.push_str(&format!(
            "WITH RECURSIVE subtree(id) AS ( \
                 SELECT id FROM places WHERE id = ?{idx} AND deleted_at_utc IS NULL \
                 UNION ALL \
                 SELECT p.id FROM places p JOIN subtree s ON p.parent_id = s.id \
                 WHERE p.deleted_at_utc IS NULL \
             ) "
        ));
        clauses.push("a.place_id IN (SELECT id FROM subtree)".to_string());
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
    // D-11.2/D-11.4: geographic "на складе" quick filter — mirrors
    // query_acts_inner; independent of item status (D-11.5).
    if let Some(want_storage) = filter.is_storage {
        let storage_cte = "storage_ids(id) AS ( \
                 SELECT id FROM places WHERE is_storage = 1 AND deleted_at_utc IS NULL \
                 UNION ALL \
                 SELECT p.id FROM places p JOIN storage_ids s ON p.parent_id = s.id \
                 WHERE p.deleted_at_utc IS NULL \
             ) ";
        if with_prefix.is_empty() {
            with_prefix = format!("WITH RECURSIVE {storage_cte}");
        } else {
            with_prefix = format!("{}, {storage_cte}", with_prefix.trim_end());
        }
        if want_storage {
            clauses.push("a.place_id IN (SELECT id FROM storage_ids)".to_string());
        } else {
            clauses.push(
                "(a.place_id IS NULL OR a.place_id NOT IN (SELECT id FROM storage_ids))"
                    .to_string(),
            );
        }
    }

    let where_clause = clauses.join(" AND ");
    let sql = format!(
        "{with_prefix}SELECT COUNT(DISTINCT a.id) \
         FROM acts a \
         LEFT JOIN act_items ai ON ai.act_id = a.id \
         LEFT JOIN devices d ON d.id = ai.device_id \
         WHERE {where_clause}"
    );

    let param_refs: Vec<&dyn ToSql> = owned_params.iter().map(|b| b.as_ref()).collect();
    conn.query_row(&sql, param_refs.as_slice(), |r| r.get(0))
        .map_err(map_rusqlite)
}

/// COUNT(*) variant of query_device_snapshot — same WHERE clauses, no row collection.
fn count_device_snapshot(
    conn: &rusqlite::Connection,
    filter: &ReportFilter,
    default_status_name: &str,
) -> Result<i64, AppError> {
    let mut clauses: Vec<String> = Vec::new();
    let mut owned_params: Vec<Box<dyn ToSql>> = Vec::new();
    let mut with_prefix = String::new();

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
    // D-28: subtree-inclusive place filter — mirrors query_device_snapshot.
    if let Some(place_id) = filter.place_id {
        let idx = next_idx(&owned_params);
        owned_params.push(Box::new(place_id));
        with_prefix.push_str(&format!(
            "WITH RECURSIVE subtree(id) AS ( \
                 SELECT id FROM places WHERE id = ?{idx} AND deleted_at_utc IS NULL \
                 UNION ALL \
                 SELECT p.id FROM places p JOIN subtree s ON p.parent_id = s.id \
                 WHERE p.deleted_at_utc IS NULL \
             ) "
        ));
        clauses.push("d.place_id IN (SELECT id FROM subtree)".to_string());
    }
    if let Some(type_id) = filter.type_id {
        clauses.push(format!("d.type_id = ?{}", next_idx(&owned_params)));
        owned_params.push(Box::new(type_id));
    }
    // D-11.2/D-11.4: geographic "на складе" quick filter — mirrors
    // query_device_snapshot; independent of item status (D-11.5).
    if let Some(want_storage) = filter.is_storage {
        let storage_cte = "storage_ids(id) AS ( \
                 SELECT id FROM places WHERE is_storage = 1 AND deleted_at_utc IS NULL \
                 UNION ALL \
                 SELECT p.id FROM places p JOIN storage_ids s ON p.parent_id = s.id \
                 WHERE p.deleted_at_utc IS NULL \
             ) ";
        if with_prefix.is_empty() {
            with_prefix = format!("WITH RECURSIVE {storage_cte}");
        } else {
            with_prefix = format!("{}, {storage_cte}", with_prefix.trim_end());
        }
        if want_storage {
            clauses.push("d.place_id IN (SELECT id FROM storage_ids)".to_string());
        } else {
            clauses.push(
                "(d.place_id IS NULL OR d.place_id NOT IN (SELECT id FROM storage_ids))"
                    .to_string(),
            );
        }
    }

    let where_clause = clauses.join(" AND ");
    let sql = format!(
        "{with_prefix}SELECT COUNT(*) FROM devices d \
         LEFT JOIN device_statuses s ON d.status_id = s.id \
         WHERE {where_clause}"
    );

    let param_refs: Vec<&dyn ToSql> = owned_params.iter().map(|b| b.as_ref()).collect();
    conn.query_row(&sql, param_refs.as_slice(), |r| r.get(0))
        .map_err(map_rusqlite)
}

/// COUNT(*) variant of query_cartridge_audit — same WHERE clauses, no row collection.
fn count_cartridge_audit_inner(
    conn: &rusqlite::Connection,
    filter: &ReportFilter,
    ts_from: Option<i64>,
    ts_to: Option<i64>,
    actions: &[&str],
) -> Result<i64, AppError> {
    let mut clauses: Vec<String> = Vec::new();
    let mut owned_params: Vec<Box<dyn ToSql>> = Vec::new();
    let mut with_prefix = String::new();

    clauses.push("al.entity_type = 'cartridge'".to_string());

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
    // D-28: subtree-inclusive place filter — mirrors query_cartridge_audit.
    if let Some(place_id) = filter.place_id {
        let idx = next_idx(&owned_params);
        owned_params.push(Box::new(place_id));
        let subtree_cte = format!(
            "subtree(id) AS ( \
                 SELECT id FROM places WHERE id = ?{idx} AND deleted_at_utc IS NULL \
                 UNION ALL \
                 SELECT p.id FROM places p JOIN subtree s ON p.parent_id = s.id \
                 WHERE p.deleted_at_utc IS NULL \
             ) "
        );
        if with_prefix.is_empty() {
            with_prefix = format!("WITH RECURSIVE {subtree_cte}");
        } else {
            with_prefix = format!("{}, {subtree_cte}", with_prefix.trim_end());
        }
        clauses.push("c.place_id IN (SELECT id FROM subtree)".to_string());
    }

    let where_clause = clauses.join(" AND ");
    let sql = format!(
        "{with_prefix}SELECT COUNT(*) \
         FROM audit_log al \
         JOIN cartridges c ON c.id = al.entity_id \
         JOIN cartridge_models m ON m.id = c.model_id \
         WHERE {where_clause}"
    );

    let param_refs: Vec<&dyn ToSql> = owned_params.iter().map(|b| b.as_ref()).collect();
    conn.query_row(&sql, param_refs.as_slice(), |r| r.get(0))
        .map_err(map_rusqlite)
}

/// COUNT(*) variant of query_cartridge_snapshot — same WHERE clauses, no row collection.
fn count_cartridge_snapshot_inner(
    conn: &rusqlite::Connection,
    filter: &ReportFilter,
    default_status_name: &str,
) -> Result<i64, AppError> {
    let mut clauses: Vec<String> = Vec::new();
    let mut owned_params: Vec<Box<dyn ToSql>> = Vec::new();
    let mut with_prefix = String::new();

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
    // D-28: subtree-inclusive place filter — mirrors query_cartridge_snapshot.
    if let Some(place_id) = filter.place_id {
        let idx = next_idx(&owned_params);
        owned_params.push(Box::new(place_id));
        let subtree_cte = format!(
            "subtree(id) AS ( \
                 SELECT id FROM places WHERE id = ?{idx} AND deleted_at_utc IS NULL \
                 UNION ALL \
                 SELECT p.id FROM places p JOIN subtree s ON p.parent_id = s.id \
                 WHERE p.deleted_at_utc IS NULL \
             ) "
        );
        if with_prefix.is_empty() {
            with_prefix = format!("WITH RECURSIVE {subtree_cte}");
        } else {
            with_prefix = format!("{}, {subtree_cte}", with_prefix.trim_end());
        }
        clauses.push("c.place_id IN (SELECT id FROM subtree)".to_string());
    }

    let where_clause = clauses.join(" AND ");
    let sql = format!(
        "{with_prefix}SELECT COUNT(*) \
         FROM cartridges c \
         JOIN cartridge_models m ON m.id = c.model_id \
         LEFT JOIN cartridge_statuses cs ON cs.id = c.status_id \
         WHERE {where_clause}"
    );

    let param_refs: Vec<&dyn ToSql> = owned_params.iter().map(|b| b.as_ref()).collect();
    conn.query_row(&sql, param_refs.as_slice(), |r| r.get(0))
        .map_err(map_rusqlite)
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

    // -----------------------------------------------------------------------
    // Request domain translators (VAD-03)
    // -----------------------------------------------------------------------

    #[test]
    fn translate_request_type_known_values() {
        assert_eq!(
            translate_request_type("cartridge_replace"),
            "Замена картриджа"
        );
        assert_eq!(translate_request_type("free_form"), "Произвольная");
        assert_eq!(translate_request_type("ad_register"), "Учётная запись AD");
    }

    #[test]
    fn translate_request_type_unknown_falls_back_to_raw_key() {
        assert_eq!(translate_request_type("future_type"), "future_type");
    }

    #[test]
    fn translate_request_status_known_values() {
        assert_eq!(translate_request_status("open"), "Открыта");
        assert_eq!(translate_request_status("in_progress"), "В работе");
        assert_eq!(translate_request_status("completed"), "Выполнена");
        assert_eq!(translate_request_status("rejected"), "Отклонена");
        assert_eq!(translate_request_status("cancelled"), "Отменена");
    }

    #[test]
    fn translate_request_status_unknown_falls_back_to_raw_key() {
        assert_eq!(translate_request_status("future_status"), "future_status");
    }

    // -----------------------------------------------------------------------
    // category_filter_clause (CATF-01/02)
    // -----------------------------------------------------------------------

    #[test]
    fn category_filter_clause_none_means_all_no_restriction() {
        let mut params: Vec<Box<dyn ToSql>> = Vec::new();
        assert_eq!(category_filter_clause(None, &mut params), None);
        assert!(params.is_empty());
    }

    #[test]
    fn category_filter_clause_empty_selection_yields_zero_rows() {
        let mut params: Vec<Box<dyn ToSql>> = Vec::new();
        let keys: Vec<String> = Vec::new();
        assert_eq!(
            category_filter_clause(Some(&keys), &mut params),
            Some("1 = 0".to_string())
        );
        assert!(params.is_empty());
    }

    #[test]
    fn category_filter_clause_known_type_key_no_new_bind_param() {
        let mut params: Vec<Box<dyn ToSql>> = Vec::new();
        let keys = vec!["ad_register".to_string()];
        let clause = category_filter_clause(Some(&keys), &mut params).unwrap();
        assert!(
            clause.contains("r.request_type = 'ad_register'"),
            "clause: {clause}"
        );
        assert!(params.is_empty());
    }

    #[test]
    fn category_filter_clause_category_key_binds_ru_name_param() {
        let mut params: Vec<Box<dyn ToSql>> = Vec::new();
        let keys = vec!["repair".to_string()];
        let clause = category_filter_clause(Some(&keys), &mut params).unwrap();
        assert!(
            clause.contains("SELECT id FROM request_categories WHERE name = ?"),
            "clause: {clause}"
        );
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn category_filter_clause_unknown_key_yields_zero_rows() {
        let mut params: Vec<Box<dyn ToSql>> = Vec::new();
        let keys = vec!["unknown_future_key".to_string()];
        assert_eq!(
            category_filter_clause(Some(&keys), &mut params),
            Some("1 = 0".to_string())
        );
        assert!(params.is_empty());
    }

    #[test]
    fn category_filter_clause_known_plus_unknown_ignores_unknown() {
        let mut params: Vec<Box<dyn ToSql>> = Vec::new();
        let keys = vec!["ad_register".to_string(), "unknown".to_string()];
        let clause = category_filter_clause(Some(&keys), &mut params).unwrap();
        assert!(
            clause.contains("r.request_type = 'ad_register'"),
            "clause: {clause}"
        );
        assert!(!clause.contains("unknown"), "clause: {clause}");
        assert!(params.is_empty());
    }

    #[test]
    fn combine_printer_and_place_none_without_printer() {
        assert_eq!(combine_printer_and_place(None, None), None);
    }

    #[test]
    fn combine_printer_and_place_appends_place() {
        assert_eq!(
            combine_printer_and_place(Some("Принтер А".to_string()), Some("Каб. 305".to_string())),
            Some("Принтер А, Каб. 305".to_string())
        );
    }

    #[test]
    fn combine_printer_and_place_printer_only_when_place_missing() {
        assert_eq!(
            combine_printer_and_place(Some("Принтер А".to_string()), None),
            Some("Принтер А".to_string())
        );
    }

    #[test]
    fn row_field_printer_place_combines_device_name_and_place_path() {
        let mut row = make_row("2026-08", "Kyocera-01", "Иванов И.И.");
        row.device_name = Some("Kyocera-01".to_string());
        row.place_path = Some("Здание А / 2 этаж / Кабинет 214".to_string());
        assert_eq!(
            row_field(&row, "printer_place", UtcOffset::UTC),
            "Kyocera-01, Здание А / 2 этаж / Кабинет 214"
        );
    }

    #[test]
    fn row_field_printer_place_empty_when_no_printer_and_no_place() {
        let mut row = make_row("2026-08", "Kyocera-01", "Иванов И.И.");
        row.device_name = None;
        row.place_path = None;
        assert_eq!(row_field(&row, "printer_place", UtcOffset::UTC), "");
    }

    // -----------------------------------------------------------------------
    // format_handover_date / row_field "handover_date_utc" tests (WSU-01/02)
    // -----------------------------------------------------------------------

    #[test]
    fn row_field_handover_date_formats_readable_moscow() {
        // 1_768_515_300 = 2026-01-15 22:15:00 UTC -> +3h = 2026-01-16 01:15
        // (day rollover proves tz is actually applied, not ignored).
        assert_eq!(
            format_handover_date(1_768_515_300, moscow()),
            "16.01.26, 01:15"
        );
    }

    #[test]
    fn row_field_handover_date_formats_readable_utc() {
        assert_eq!(
            format_handover_date(1_768_515_300, UtcOffset::UTC),
            "15.01.26, 22:15"
        );
    }

    #[test]
    fn row_field_handover_date_absent_is_empty() {
        let mut row = make_row("2026-08", "Kyocera-01", "Иванов И.И.");
        row.handover_date_utc = None;
        assert_eq!(row_field(&row, "handover_date_utc", moscow()), "");
    }

    #[test]
    fn month_key_to_russian_converts_correctly() {
        assert_eq!(month_key_to_russian("2026-09"), "Сентябрь 2026");
        assert_eq!(month_key_to_russian("2026-01"), "Январь 2026");
        assert_eq!(month_key_to_russian("2026-12"), "Декабрь 2026");
    }

    // -----------------------------------------------------------------------
    // format_period_label tests (34-06 gap fix: Russian report subtitle)
    // -----------------------------------------------------------------------

    fn period(mode: &str) -> PeriodDto {
        PeriodDto {
            mode: mode.to_string(),
            year: None,
            month: None,
            date_from: None,
            date_to: None,
        }
    }

    #[test]
    fn format_period_label_month_mode() {
        let dto = PeriodDto {
            year: Some(2026),
            month: Some(9),
            ..period("month")
        };
        assert_eq!(format_period_label(&dto), "Сентябрь 2026");
    }

    #[test]
    fn format_period_label_month_mode_january() {
        let dto = PeriodDto {
            year: Some(2026),
            month: Some(1),
            ..period("month")
        };
        assert_eq!(format_period_label(&dto), "Январь 2026");
    }

    #[test]
    fn format_period_label_month_mode_missing_month_falls_back_to_year() {
        let dto = PeriodDto {
            year: Some(2026),
            month: None,
            ..period("month")
        };
        // IN-01: same wording as year mode — see the sibling test below.
        assert_eq!(format_period_label(&dto), "2026 год");
    }

    /// IN-01: month mode degrading to a year and year mode proper must print
    /// the SAME subtitle for the same year — otherwise the printed document
    /// changes wording based on which control the user touched.
    #[test]
    fn format_period_label_month_degradation_matches_year_mode_wording() {
        let degraded = PeriodDto {
            year: Some(2026),
            month: None,
            ..period("month")
        };
        let year_mode = PeriodDto {
            year: Some(2026),
            month: None,
            ..period("year")
        };
        assert_eq!(
            format_period_label(&degraded),
            format_period_label(&year_mode)
        );
    }

    #[test]
    fn format_period_label_month_mode_out_of_range_month_falls_back_to_year() {
        let dto = PeriodDto {
            year: Some(2026),
            month: Some(13),
            ..period("month")
        };
        assert_eq!(format_period_label(&dto), "2026 год");
    }

    #[test]
    fn format_period_label_month_mode_missing_year_is_empty() {
        let dto = PeriodDto {
            year: None,
            month: Some(9),
            ..period("month")
        };
        assert_eq!(format_period_label(&dto), "");
    }

    #[test]
    fn format_period_label_year_mode() {
        let dto = PeriodDto {
            year: Some(2026),
            ..period("year")
        };
        assert_eq!(format_period_label(&dto), "2026 год");
    }

    #[test]
    fn format_period_label_year_mode_missing_year_is_empty() {
        let dto = period("year");
        assert_eq!(format_period_label(&dto), "");
    }

    #[test]
    fn format_period_label_range_mode() {
        let dto = PeriodDto {
            date_from: Some("2026-01-01".to_string()),
            date_to: Some("2026-03-31".to_string()),
            ..period("range")
        };
        assert_eq!(format_period_label(&dto), "01.01.2026 — 31.03.2026");
    }

    #[test]
    fn format_period_label_range_mode_missing_date_to() {
        let dto = PeriodDto {
            date_from: Some("2026-01-01".to_string()),
            date_to: None,
            ..period("range")
        };
        assert_eq!(format_period_label(&dto), "01.01.2026");
    }

    #[test]
    fn format_period_label_range_mode_missing_date_from() {
        let dto = PeriodDto {
            date_from: None,
            date_to: Some("2026-03-31".to_string()),
            ..period("range")
        };
        assert_eq!(format_period_label(&dto), "31.03.2026");
    }

    #[test]
    fn format_period_label_range_mode_malformed_dates_are_empty() {
        let dto = PeriodDto {
            date_from: Some("not-a-date".to_string()),
            date_to: Some("2026-13-99".to_string()),
            ..period("range")
        };
        assert_eq!(format_period_label(&dto), "");
    }

    #[test]
    fn format_period_label_range_mode_missing_both_dates_is_empty() {
        let dto = period("range");
        assert_eq!(format_period_label(&dto), "");
    }

    #[test]
    fn format_period_label_unknown_mode_never_leaks_english_discriminator() {
        let dto = PeriodDto {
            year: Some(2026),
            ..period("bogus")
        };
        let label = format_period_label(&dto);
        assert_eq!(label, "");
        assert!(!label.contains("bogus"));
    }

    // -----------------------------------------------------------------------
    // export_pdf HTML-render behavior tests (Phase 17, D-01..D-08)
    // -----------------------------------------------------------------------

    fn make_test_service() -> (ReportService, tempfile::TempDir) {
        let dir = tempfile::tempdir().expect("tempdir");
        let (writer, readers, _guard) = trackly_infra::test_support::test_writer_and_readers();
        let clock: Arc<dyn Clock + Send + Sync> = Arc::new(trackly_infra::clock_impl::SystemClock);
        let paths = Arc::new(
            trackly_infra::Paths::resolve_for_exe_dir(dir.path().to_path_buf()).expect("paths"),
        );
        let organization = Arc::new(OrganizationService::new(paths));
        let pdf = Arc::new(PdfRenderer::new());
        let svc = ReportService::new(writer, readers, clock, Arc::new(AppConfig::default()), pdf)
            .with_organization(organization);
        // Keep _guard (writer/readers tempdir) alive alongside the returned
        // service by leaking it into the returned TempDir slot below — the
        // writer/readers tempdir and the paths tempdir are independent, both
        // must outlive the test body.
        (svc, dir)
    }

    fn make_row(month_key: &str, device_name: &str, giver: &str) -> ReportRow {
        ReportRow {
            id: 1,
            month_key: Some(month_key.to_string()),
            number: Some("42".to_string()),
            sub_number: None,
            giver_name: Some(giver.to_string()),
            receiver_name: Some("Иванов И.И.".to_string()),
            handover_date_utc: Some(1_780_000_000),
            place_path: Some("Склад №1".to_string()),
            act_type: Some("handover".to_string()),
            device_name: Some(device_name.to_string()),
            quantity: Some(1),
            code: None,
            model_label: None,
            status_name: None,
            request_type_label: None,
        }
    }

    fn empty_org() -> OrgSettingsDto {
        OrgSettingsDto {
            org_name: String::new(),
            inn: String::new(),
            kpp: String::new(),
            address: String::new(),
            has_logo: false,
            phone: String::new(),
            fax: String::new(),
            email: String::new(),
            okpo: String::new(),
            ogrn: String::new(),
            address_line2: String::new(),
            full_name: String::new(),
        }
    }

    #[tokio::test]
    async fn export_pdf_non_empty_report_renders_month_groups_and_rows() {
        let (svc, _dir) = make_test_service();
        let rows = ReportResponse {
            rows: vec![
                make_row("2026-09", "Принтер HP LaserJet", "Петров П.П."),
                make_row("2026-10", "Сканер Canon", "Сидоров С.С."),
            ],
            total: 2,
        };
        let columns = ["device_name", "giver_name", "receiver_name"];
        let labels = ["Устройства", "Сдал", "Принял"];

        let html = svc
            .export_pdf(
                &rows,
                "Тестовый отчёт",
                "Сентябрь-Октябрь 2026",
                &empty_org(),
                None,
                None,
                &columns,
                &labels,
            )
            .await
            .expect("export_pdf ok");

        assert!(
            html.contains("Сентябрь 2026"),
            "expected September 2026 month heading in HTML: {html}"
        );
        assert!(
            html.contains("Октябрь 2026"),
            "expected October 2026 month heading in HTML: {html}"
        );
        assert!(html.contains("Принтер HP LaserJet"));
        assert!(html.contains("Сканер Canon"));
        assert!(html.contains("Петров П.П."));
        assert!(html.contains("Сидоров С.С."));
        assert!(
            html.contains("<html") || html.contains("<!DOCTYPE"),
            "HTML output must be well-formed HTML markup, not a document-spec artifact: {html}"
        );
    }

    #[tokio::test]
    async fn export_pdf_empty_report_renders_no_data_message() {
        let (svc, _dir) = make_test_service();
        let rows = ReportResponse {
            rows: vec![],
            total: 0,
        };
        let columns = ["device_name"];
        let labels = ["Устройства"];

        let html = svc
            .export_pdf(
                &rows,
                "Пустой отчёт",
                "Ноябрь 2026",
                &empty_org(),
                None,
                None,
                &columns,
                &labels,
            )
            .await
            .expect("export_pdf ok");

        assert!(
            html.contains("Нет данных за указанный период."),
            "expected empty-state message in HTML: {html}"
        );
    }

    #[tokio::test]
    async fn export_pdf_renders_org_header_name() {
        let (svc, _dir) = make_test_service();
        let rows = ReportResponse {
            rows: vec![make_row("2026-09", "Принтер", "Петров П.П.")],
            total: 1,
        };
        let columns = ["device_name"];
        let labels = ["Устройства"];
        let mut org = empty_org();
        org.org_name = "ООО «Ромашка»".to_string();

        let html = svc
            .export_pdf(
                &rows,
                "Отчёт",
                "Сентябрь 2026",
                &org,
                None,
                None,
                &columns,
                &labels,
            )
            .await
            .expect("export_pdf ok");

        assert!(
            html.contains("ООО «Ромашка»"),
            "expected org name in HTML header: {html}"
        );
    }
}
