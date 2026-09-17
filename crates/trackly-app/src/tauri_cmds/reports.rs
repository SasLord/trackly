//! Reports Tauri commands — Phase 7 Plan 07.
//!
//! Thin adapters over ReportService. All build_* helpers called by
//! both Tauri commands and axum HTTP handlers.
//!
//! Auth: no role restriction beyond a valid Tauri (desktop) context.
//! HTTP handlers add session_identity check in http/reports.rs.

use crate::context::AppCtx;
use crate::dto::reports::{PeriodDto, ReportCountsDto, ReportFilter, ReportResponse};
use crate::services::report_service::format_period_label;
use crate::tauri_cmds::users::resolve_tauri_identity;
use rusqlite::OptionalExtension;
use trackly_core::auth::{authorize, Action, Identity};
use trackly_core::error::AppError;

// ---------------------------------------------------------------------------
// Columns per report type (used for CSV/PDF header rows)
// ---------------------------------------------------------------------------

/// `omit_type_column` (user-requested deviation, plan 40.1-02, live UAT
/// 2026-09-18): when the `movements` report is filtered to a single device
/// type (`filter.type_id.is_some()`), the resulting «Тип» column is
/// redundant — every row shares the same value. Only `"movements"` ever sets
/// this `true`; every other `report_type` ignores the flag (see
/// `other_report_types_ignore_omit_type_column_flag`).
fn columns_for(report_type: &str, omit_type_column: bool) -> Vec<&'static str> {
    match report_type {
        "device_acts" | "device_returns" => {
            vec![
                "number",
                "device_name",
                "giver_name",
                "receiver_name",
                "place_path",
            ]
        }
        "device_in_use" | "device_in_stock" => {
            vec!["device_name", "status_name", "place_path"]
        }
        "cartridge_consumption" | "cartridge_refills" => {
            vec!["code", "model_label", "status_name", "place_path"]
        }
        "cartridge_in_use" | "cartridge_in_stock" => {
            vec!["code", "model_label", "status_name", "place_path"]
        }
        "requests_all" | "requests_open" | "requests_in_progress" | "requests_completed" => {
            vec![
                "number",
                "handover_date_utc",
                "request_type_label",
                "status_name",
                "giver_name",
                "printer_place",
            ]
        }
        // HST-04 (D-23). NOTE: the key is `handover_date_utc`, not
        // `created_at_utc` — `query_movements_inner` (Plan 40-11) reuses the
        // existing `ReportRow.handover_date_utc` field to carry
        // `pm.created_at_utc` (same reuse convention as `place_path` for
        // "Куда"), and `row_field` only has a match arm for
        // `"handover_date_utc"`. Using `"created_at_utc"` here would compile
        // fine but silently render an empty «Дата» column in both the table
        // and CSV/PDF export — see Deviations in the plan Summary.
        "movements" => {
            let mut cols = vec![
                "handover_date_utc",
                "device_name",
                "entity_type_label",
                "from_place_path",
                "place_path",
                "actor_name",
                "reason",
            ];
            if omit_type_column {
                cols.retain(|c| *c != "entity_type_label");
            }
            cols
        }
        _ => vec!["id"],
    }
}

/// Russian column labels for HTML/PDF report headers (D-03/CR-01 fix).
///
/// Index-aligned with `columns_for(report_type)` — the same match arms, in
/// the same order, one label per key. `columns_for` remains the source of
/// truth for the underlying keys used by `row_field(row, col)` to resolve
/// cell values; this function is used ONLY to build the header row shown to
/// the user (`ctx["columns"]` in `ReportService::export_pdf`). Labels are
/// sourced from `ui/src/features/reports/ReportsPage.svelte`'s
/// `COLUMNS_MAP` so printed headers match the on-screen report table.
fn column_labels_for(report_type: &str, omit_type_column: bool) -> Vec<&'static str> {
    match report_type {
        "device_acts" | "device_returns" => {
            vec!["Номер", "Устройства", "Сдал", "Принял", "Место"]
        }
        "device_in_use" | "device_in_stock" => {
            vec!["Наименование", "Статус", "Место"]
        }
        "cartridge_consumption" | "cartridge_refills" => {
            vec!["Код картриджа", "Модель", "Статус", "Место"]
        }
        "cartridge_in_use" | "cartridge_in_stock" => {
            vec!["Код", "Модель", "Статус", "Место"]
        }
        "requests_all" | "requests_open" | "requests_in_progress" | "requests_completed" => {
            vec!["№", "Дата", "Тип", "Статус", "Заявитель", "Место"]
        }
        // HST-04 (D-23) — index-aligned with columns_for("movements") above.
        "movements" => {
            let mut labels = vec!["Дата", "Предмет", "Тип", "Откуда", "Куда", "Кем", "Причина"];
            if omit_type_column {
                labels.retain(|l| *l != "Тип");
            }
            labels
        }
        _ => vec!["ID"],
    }
}

/// Human-readable report name (used in PDF header).
fn report_display_name(report_type: &str) -> &'static str {
    match report_type {
        "device_acts" => "Акты приёма-передачи устройств",
        "device_returns" => "Акты возврата устройств",
        "device_in_use" => "Устройства в работе",
        "device_in_stock" => "Устройства на складе",
        "cartridge_consumption" => "Расход картриджей",
        "cartridge_refills" => "Заправки картриджей",
        "cartridge_in_use" => "Картриджи в работе",
        "cartridge_in_stock" => "Картриджи на складе",
        "requests_all" => "Заявки",
        "requests_open" => "Открытые заявки",
        "requests_in_progress" => "Заявки в работе",
        "requests_completed" => "Выполненные заявки",
        "movements" => "Перемещения",
        _ => "Отчёт",
    }
}

// ---------------------------------------------------------------------------
// build_* helpers
// ---------------------------------------------------------------------------

pub async fn build_reports_list_device_acts(
    ctx: &AppCtx,
    caller: &Identity,
    filter: ReportFilter,
    period: PeriodDto,
) -> Result<ReportResponse, AppError> {
    authorize(caller, &Action::ReadData)?;
    ctx.reports.list_device_acts(filter, period).await
}

pub async fn build_reports_list_device_returns(
    ctx: &AppCtx,
    caller: &Identity,
    filter: ReportFilter,
    period: PeriodDto,
) -> Result<ReportResponse, AppError> {
    authorize(caller, &Action::ReadData)?;
    ctx.reports.list_device_returns(filter, period).await
}

pub async fn build_reports_list_device_in_use(
    ctx: &AppCtx,
    caller: &Identity,
    filter: ReportFilter,
) -> Result<ReportResponse, AppError> {
    authorize(caller, &Action::ReadData)?;
    ctx.reports.list_device_in_use(filter).await
}

pub async fn build_reports_list_device_in_stock(
    ctx: &AppCtx,
    caller: &Identity,
    filter: ReportFilter,
) -> Result<ReportResponse, AppError> {
    authorize(caller, &Action::ReadData)?;
    ctx.reports.list_device_in_stock(filter).await
}

pub async fn build_reports_list_cartridge_consumption(
    ctx: &AppCtx,
    caller: &Identity,
    filter: ReportFilter,
    period: PeriodDto,
) -> Result<ReportResponse, AppError> {
    authorize(caller, &Action::ReadData)?;
    ctx.reports.list_cartridge_consumption(filter, period).await
}

pub async fn build_reports_list_cartridge_refills(
    ctx: &AppCtx,
    caller: &Identity,
    filter: ReportFilter,
    period: PeriodDto,
) -> Result<ReportResponse, AppError> {
    authorize(caller, &Action::ReadData)?;
    ctx.reports.list_cartridge_refills(filter, period).await
}

pub async fn build_reports_list_cartridge_in_use(
    ctx: &AppCtx,
    caller: &Identity,
    filter: ReportFilter,
) -> Result<ReportResponse, AppError> {
    authorize(caller, &Action::ReadData)?;
    ctx.reports.list_cartridge_in_use(filter).await
}

pub async fn build_reports_list_cartridge_in_stock(
    ctx: &AppCtx,
    caller: &Identity,
    filter: ReportFilter,
) -> Result<ReportResponse, AppError> {
    authorize(caller, &Action::ReadData)?;
    ctx.reports.list_cartridge_in_stock(filter).await
}

pub async fn build_reports_list_requests_all(
    ctx: &AppCtx,
    caller: &Identity,
    filter: ReportFilter,
    period: PeriodDto,
) -> Result<ReportResponse, AppError> {
    authorize(caller, &Action::ReadData)?;
    let exclude_ad_register = trackly_core::auth::excludes_ad_register(&caller.role);
    ctx.reports
        .list_requests_all(filter, period, exclude_ad_register)
        .await
}

pub async fn build_reports_list_requests_open(
    ctx: &AppCtx,
    caller: &Identity,
    filter: ReportFilter,
    period: PeriodDto,
) -> Result<ReportResponse, AppError> {
    authorize(caller, &Action::ReadData)?;
    let exclude_ad_register = trackly_core::auth::excludes_ad_register(&caller.role);
    ctx.reports
        .list_requests_open(filter, period, exclude_ad_register)
        .await
}

pub async fn build_reports_list_requests_in_progress(
    ctx: &AppCtx,
    caller: &Identity,
    filter: ReportFilter,
    period: PeriodDto,
) -> Result<ReportResponse, AppError> {
    authorize(caller, &Action::ReadData)?;
    let exclude_ad_register = trackly_core::auth::excludes_ad_register(&caller.role);
    ctx.reports
        .list_requests_in_progress(filter, period, exclude_ad_register)
        .await
}

pub async fn build_reports_list_requests_completed(
    ctx: &AppCtx,
    caller: &Identity,
    filter: ReportFilter,
    period: PeriodDto,
) -> Result<ReportResponse, AppError> {
    authorize(caller, &Action::ReadData)?;
    let exclude_ad_register = trackly_core::auth::excludes_ad_register(&caller.role);
    ctx.reports
        .list_requests_completed(filter, period, exclude_ad_register)
        .await
}

/// HST-04 movements report. **Gate diverges from every other `build_reports_list_*`
/// above**: `Action::ReadPlaces`, NOT `Action::ReadData` (D-12 — this is the single
/// highest-risk copy-paste spot in the whole report clone, PATTERNS.md's own named
/// warning). Functionally both actions currently authorize the same two roles
/// (Admin | Manager, Employee excluded — see the permission-matrix doc comment on
/// `authorize()`), but the semantic gate MUST be `ReadPlaces` per D-12, since the
/// role-matrix regression test added by Plan 40-14 asserts this action specifically,
/// not just its current role set.
pub async fn build_reports_list_movements(
    ctx: &AppCtx,
    caller: &Identity,
    filter: ReportFilter,
    period: PeriodDto,
) -> Result<ReportResponse, AppError> {
    authorize(caller, &Action::ReadPlaces)?;
    ctx.reports.list_movements(filter, period).await
}

/// WR-02 gap closure: export gate must track the LIST gate per `report_type`,
/// not uniformly `Action::ReadData`. `"movements"` is gated on
/// `Action::ReadPlaces` in `build_reports_list_movements` (D-12) — export must
/// match exactly, so a future divergence of `ReadData`'s role set from
/// `ReadPlaces`'s (today identical, see the doc comment on
/// `build_reports_list_movements`) cannot silently widen who can export the
/// movements report while the on-screen list stays correctly restricted.
///
/// Split into a pure `export_gate_action` + thin `authorize_report_export`
/// wrapper specifically so the mapping can be unit-tested by asserting
/// equality against a specific `Action` variant (see the `tests` module
/// below) — a role-set probe alone cannot distinguish `ReadPlaces` from
/// `ReadData`, since both currently authorize the identical Admin|Manager
/// set.
fn export_gate_action(report_type: &str) -> Action {
    if report_type == "movements" {
        Action::ReadPlaces
    } else {
        Action::ReadData
    }
}

fn authorize_report_export(caller: &Identity, report_type: &str) -> Result<(), AppError> {
    authorize(caller, &export_gate_action(report_type))
}

#[cfg(test)]
mod export_gate_tests {
    use super::*;

    /// WR-02: pins the movements report's export gate to `Action::ReadPlaces`
    /// specifically — matching `build_reports_list_movements`'s own gate
    /// (D-12) — rather than the uniform `Action::ReadData` every other
    /// report_type uses. If `ReadData`'s role set is ever widened
    /// independently of `ReadPlaces`, this test (unlike a role-probe test)
    /// still fails, because it checks the ACTION, not just today's resolved
    /// role set.
    #[test]
    fn movements_export_gate_is_read_places_not_read_data() {
        assert_eq!(export_gate_action("movements"), Action::ReadPlaces);
    }

    /// Every other report_type keeps the pre-existing uniform `ReadData` gate
    /// — this fix must not accidentally widen the special-case beyond
    /// `"movements"`.
    #[test]
    fn other_report_types_keep_read_data_gate() {
        for report_type in [
            "device_acts",
            "device_returns",
            "device_in_use",
            "device_in_stock",
            "cartridge_consumption",
            "cartridge_refills",
            "cartridge_in_use",
            "cartridge_in_stock",
            "requests_all",
            "requests_open",
            "requests_in_progress",
            "requests_completed",
        ] {
            assert_eq!(
                export_gate_action(report_type),
                Action::ReadData,
                "report_type {report_type:?} must keep the ReadData export gate"
            );
        }
    }
}

/// Export report rows as UTF-8 BOM CSV bytes.
pub async fn build_reports_export_csv(
    ctx: &AppCtx,
    caller: &Identity,
    report_type: String,
    filter: ReportFilter,
    period: Option<PeriodDto>,
) -> Result<Vec<u8>, AppError> {
    authorize_report_export(caller, &report_type)?;
    // User-requested deviation (plan 40.1-02, live UAT 2026-09-18): CSV
    // shares columns_for/column_labels_for with the PDF export (WARNING-4's
    // shared point, D-20/D-21), so the same «Тип» column omission applies
    // here too when the movements report is filtered to a single type.
    let omit_type_column = report_type == "movements" && filter.type_id.is_some();
    let rows = fetch_report(ctx, caller, &report_type, filter, period).await?;
    let cols = columns_for(&report_type, omit_type_column);
    let labels = column_labels_for(&report_type, omit_type_column);
    ctx.reports.export_csv(&rows, &cols, &labels).await
}

/// Export report as an HTML string (Phase 17: migrated off krilla/DocSpec).
pub async fn build_reports_export_pdf(
    ctx: &AppCtx,
    caller: &Identity,
    report_type: String,
    filter: ReportFilter,
    period: Option<PeriodDto>,
) -> Result<String, AppError> {
    authorize_report_export(caller, &report_type)?;
    // User-requested deviation (plan 40.1-02, live UAT 2026-09-18): same
    // omission decision as build_reports_export_csv (WARNING-4's shared
    // columns_for/column_labels_for point).
    let omit_type_column = report_type == "movements" && filter.type_id.is_some();
    // Resolved BEFORE `filter` is moved into `fetch_report` below — needs
    // read access to filter.from_place_id/to_place_id/type_id.
    let filter_summary = build_movements_filter_summary(ctx, caller, &report_type, &filter).await?;
    let rows = fetch_report(ctx, caller, &report_type, filter, period.clone()).await?;
    let org = ctx.org_db.get().await?;
    let logo_bytes = ctx.org_db.get_logo_bytes().await?;
    let logo_mime = if logo_bytes.is_some() {
        // Fetch mime separately from org_settings
        let readers = ctx.readers.clone();
        tokio::task::spawn_blocking(move || -> Result<Option<String>, AppError> {
            let conn = readers.acquire();
            conn.query_row("SELECT logo_mime FROM org_settings WHERE id = 1", [], |r| {
                r.get(0)
            })
            .map_err(trackly_infra::error_conversions::map_rusqlite)
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking logo_mime: {e}"),
        })??
    } else {
        None
    };
    let cols = columns_for(&report_type, omit_type_column);
    let labels = column_labels_for(&report_type, omit_type_column);
    let report_name = report_display_name(&report_type);
    let period_label = period.as_ref().map(format_period_label).unwrap_or_default();
    ctx.reports
        .export_pdf(
            &rows,
            report_name,
            &period_label,
            &org,
            logo_bytes,
            logo_mime,
            &cols,
            &labels,
            filter_summary.as_deref(),
        )
        .await
}

/// Device type name lookup for `build_movements_filter_summary` — `device_types`
/// is a tiny seeded lookup table (V001). `None` if the id has since been
/// removed (should not happen — `type_id` always originates from the same
/// `deviceTypes` dropdown the UI queries — but degrades gracefully rather
/// than failing the whole export over a stale filter value).
async fn device_type_name(ctx: &AppCtx, type_id: i64) -> Result<Option<String>, AppError> {
    let readers = ctx.readers.clone();
    tokio::task::spawn_blocking(move || -> Result<Option<String>, AppError> {
        let conn = readers.acquire();
        conn.query_row(
            "SELECT name FROM device_types WHERE id = ?1",
            [type_id],
            |r| r.get(0),
        )
        .optional()
        .map_err(trackly_infra::error_conversions::map_rusqlite)
    })
    .await
    .map_err(|e| AppError::Internal {
        source_chain: format!("spawn_blocking device_type_name: {e}"),
    })?
}

/// User-requested deviation (live UAT of BLOCKER-2's «Тип устройства»
/// filter, plan 40.1-02, 2026-09-18): builds the human-readable summary of
/// active `movements` filters (Откуда/Куда/Тип устройства) shown in the
/// print/PDF export, below `period_label` and above the table — see
/// `40.1-02-SUMMARY.md`'s Deviations section.
///
/// Names, never raw ids: from/to place full paths come from
/// `ctx.places.full_path` (same authorization gate, `Action::ReadPlaces`,
/// the caller already passed to reach this function), device type name from
/// `device_type_name` above. `None` when `report_type != "movements"` or no
/// movements filter is active — the template's `{% if filter_summary %}`
/// guard then renders nothing.
async fn build_movements_filter_summary(
    ctx: &AppCtx,
    caller: &Identity,
    report_type: &str,
    filter: &ReportFilter,
) -> Result<Option<String>, AppError> {
    if report_type != "movements" {
        return Ok(None);
    }
    let mut parts: Vec<String> = Vec::new();
    if let Some(from_id) = filter.from_place_id {
        if let Some(path) = place_full_path_or_none(ctx, caller, from_id).await? {
            parts.push(format!("Откуда: {path}"));
        }
    }
    if let Some(to_id) = filter.to_place_id {
        if let Some(path) = place_full_path_or_none(ctx, caller, to_id).await? {
            parts.push(format!("Куда: {path}"));
        }
    }
    if let Some(type_id) = filter.type_id {
        if let Some(name) = device_type_name(ctx, type_id).await? {
            parts.push(format!("Тип устройства: {name}"));
        }
    }
    if parts.is_empty() {
        Ok(None)
    } else {
        Ok(Some(parts.join("; ")))
    }
}

/// WR-02 (40.1 gap closure, 2026-09-18): place full-path lookup for
/// `build_movements_filter_summary`, degrading the same way
/// `device_type_name` above already does — `None` if the place was deleted
/// between filter selection and export (legal: `BLOCKER-1`'s pre-flight only
/// blocks deleting a place that is itself referenced by a `place_movements`
/// row, not a place that merely appears in someone's currently-open report
/// filter). Any other error (auth, I/O) still propagates via `?` — only the
/// "place no longer exists" case is expected and swallowed here.
async fn place_full_path_or_none(
    ctx: &AppCtx,
    caller: &Identity,
    place_id: i64,
) -> Result<Option<String>, AppError> {
    match ctx.places.full_path(caller, place_id).await {
        Ok(path) => Ok(Some(path)),
        Err(AppError::NotFound { .. }) => Ok(None),
        Err(e) => Err(e),
    }
}

/// Report types whose query is period-scoped. For these, `period` is
/// mandatory — see `require_period`.
pub(crate) const PERIOD_BASED_REPORT_TYPES: [&str; 9] = [
    "device_acts",
    "device_returns",
    "cartridge_consumption",
    "cartridge_refills",
    "requests_all",
    "requests_open",
    "requests_in_progress",
    "requests_completed",
    "movements",
];

/// WR-07: reject an absent `period` for a period-scoped report instead of
/// guessing one.
///
/// The previous `unwrap_or_else` substituted a hardcoded January 2026, so
/// `POST /api/v1/reports_export_pdf` with `period: null` silently restricted
/// the rows to that month while `format_period_label(None)` printed an EMPTY
/// subtitle — the document then looked like a full-history report. (Before
/// Phase 34 the label at least emitted the obviously-broken `"month 2026"`;
/// fixing the label made the wrong output look authoritative.) The `Some(2026)`
/// / `Some(1)` magic numbers were a latent time bomb on top.
///
/// The UI never hits this: `ReportsPage.svelte` sends `period: undefined` only
/// for snapshot report types.
fn require_period(report_type: &str, period: Option<PeriodDto>) -> Result<PeriodDto, AppError> {
    period.ok_or_else(|| {
        debug_assert!(PERIOD_BASED_REPORT_TYPES.contains(&report_type));
        AppError::Validation {
            field: "period".to_string(),
            message: "Период обязателен для этого типа отчёта".to_string(),
        }
    })
}

/// Dispatch to the right list method based on report_type string.
async fn fetch_report(
    ctx: &AppCtx,
    caller: &Identity,
    report_type: &str,
    filter: ReportFilter,
    period: Option<PeriodDto>,
) -> Result<ReportResponse, AppError> {
    let exclude_ad_register = trackly_core::auth::excludes_ad_register(&caller.role);
    match report_type {
        "device_acts" => {
            ctx.reports
                .list_device_acts(filter, require_period(report_type, period)?)
                .await
        }
        "device_returns" => {
            ctx.reports
                .list_device_returns(filter, require_period(report_type, period)?)
                .await
        }
        "device_in_use" => ctx.reports.list_device_in_use(filter).await,
        "device_in_stock" => ctx.reports.list_device_in_stock(filter).await,
        "cartridge_consumption" => {
            ctx.reports
                .list_cartridge_consumption(filter, require_period(report_type, period)?)
                .await
        }
        "cartridge_refills" => {
            ctx.reports
                .list_cartridge_refills(filter, require_period(report_type, period)?)
                .await
        }
        "cartridge_in_use" => ctx.reports.list_cartridge_in_use(filter).await,
        "cartridge_in_stock" => ctx.reports.list_cartridge_in_stock(filter).await,
        "requests_all" => {
            ctx.reports
                .list_requests_all(
                    filter,
                    require_period(report_type, period)?,
                    exclude_ad_register,
                )
                .await
        }
        "requests_open" => {
            ctx.reports
                .list_requests_open(
                    filter,
                    require_period(report_type, period)?,
                    exclude_ad_register,
                )
                .await
        }
        "requests_in_progress" => {
            ctx.reports
                .list_requests_in_progress(
                    filter,
                    require_period(report_type, period)?,
                    exclude_ad_register,
                )
                .await
        }
        "requests_completed" => {
            ctx.reports
                .list_requests_completed(
                    filter,
                    require_period(report_type, period)?,
                    exclude_ad_register,
                )
                .await
        }
        "movements" => {
            ctx.reports
                .list_movements(filter, require_period(report_type, period)?)
                .await
        }
        other => Err(AppError::Validation {
            field: "report_type".to_string(),
            message: format!("Unknown report type: {other}"),
        }),
    }
}

// ---------------------------------------------------------------------------
// Tauri command wrappers
// ---------------------------------------------------------------------------

#[tauri::command]
#[specta::specta]
pub async fn reports_list_device_acts(
    state: tauri::State<'_, AppCtx>,
    filter: ReportFilter,
    period: PeriodDto,
) -> Result<ReportResponse, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_reports_list_device_acts(state.inner(), &caller, filter, period).await
}

#[tauri::command]
#[specta::specta]
pub async fn reports_list_device_returns(
    state: tauri::State<'_, AppCtx>,
    filter: ReportFilter,
    period: PeriodDto,
) -> Result<ReportResponse, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_reports_list_device_returns(state.inner(), &caller, filter, period).await
}

#[tauri::command]
#[specta::specta]
pub async fn reports_list_device_in_use(
    state: tauri::State<'_, AppCtx>,
    filter: ReportFilter,
) -> Result<ReportResponse, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_reports_list_device_in_use(state.inner(), &caller, filter).await
}

#[tauri::command]
#[specta::specta]
pub async fn reports_list_device_in_stock(
    state: tauri::State<'_, AppCtx>,
    filter: ReportFilter,
) -> Result<ReportResponse, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_reports_list_device_in_stock(state.inner(), &caller, filter).await
}

#[tauri::command]
#[specta::specta]
pub async fn reports_list_cartridge_consumption(
    state: tauri::State<'_, AppCtx>,
    filter: ReportFilter,
    period: PeriodDto,
) -> Result<ReportResponse, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_reports_list_cartridge_consumption(state.inner(), &caller, filter, period).await
}

#[tauri::command]
#[specta::specta]
pub async fn reports_list_cartridge_refills(
    state: tauri::State<'_, AppCtx>,
    filter: ReportFilter,
    period: PeriodDto,
) -> Result<ReportResponse, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_reports_list_cartridge_refills(state.inner(), &caller, filter, period).await
}

#[tauri::command]
#[specta::specta]
pub async fn reports_list_cartridge_in_use(
    state: tauri::State<'_, AppCtx>,
    filter: ReportFilter,
) -> Result<ReportResponse, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_reports_list_cartridge_in_use(state.inner(), &caller, filter).await
}

#[tauri::command]
#[specta::specta]
pub async fn reports_list_cartridge_in_stock(
    state: tauri::State<'_, AppCtx>,
    filter: ReportFilter,
) -> Result<ReportResponse, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_reports_list_cartridge_in_stock(state.inner(), &caller, filter).await
}

#[tauri::command]
#[specta::specta]
pub async fn reports_list_requests_all(
    state: tauri::State<'_, AppCtx>,
    filter: ReportFilter,
    period: PeriodDto,
) -> Result<ReportResponse, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_reports_list_requests_all(state.inner(), &caller, filter, period).await
}

#[tauri::command]
#[specta::specta]
pub async fn reports_list_requests_open(
    state: tauri::State<'_, AppCtx>,
    filter: ReportFilter,
    period: PeriodDto,
) -> Result<ReportResponse, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_reports_list_requests_open(state.inner(), &caller, filter, period).await
}

#[tauri::command]
#[specta::specta]
pub async fn reports_list_requests_in_progress(
    state: tauri::State<'_, AppCtx>,
    filter: ReportFilter,
    period: PeriodDto,
) -> Result<ReportResponse, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_reports_list_requests_in_progress(state.inner(), &caller, filter, period).await
}

#[tauri::command]
#[specta::specta]
pub async fn reports_list_requests_completed(
    state: tauri::State<'_, AppCtx>,
    filter: ReportFilter,
    period: PeriodDto,
) -> Result<ReportResponse, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_reports_list_requests_completed(state.inner(), &caller, filter, period).await
}

/// HST-04. Delegates to `build_reports_list_movements`, which gates on
/// `Action::ReadPlaces` — NOT `Action::ReadData` like every sibling command
/// above (D-12).
#[tauri::command]
#[specta::specta]
pub async fn reports_list_movements(
    state: tauri::State<'_, AppCtx>,
    filter: ReportFilter,
    period: PeriodDto,
) -> Result<ReportResponse, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_reports_list_movements(state.inner(), &caller, filter, period).await
}

#[tauri::command]
#[specta::specta]
pub async fn reports_export_csv(
    state: tauri::State<'_, AppCtx>,
    report_type: String,
    filter: ReportFilter,
    period: Option<PeriodDto>,
) -> Result<Vec<u8>, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_reports_export_csv(state.inner(), &caller, report_type, filter, period).await
}

#[tauri::command]
#[specta::specta]
pub async fn reports_export_pdf(
    state: tauri::State<'_, AppCtx>,
    report_type: String,
    filter: ReportFilter,
    period: Option<PeriodDto>,
) -> Result<String, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_reports_export_pdf(state.inner(), &caller, report_type, filter, period).await
}

// ---------------------------------------------------------------------------
// reports_get_report_counts (G2-5b)
// ---------------------------------------------------------------------------

/// Build helper for reports_get_report_counts — callable from both Tauri and HTTP.
pub async fn build_reports_get_report_counts(
    ctx: &AppCtx,
    caller: &Identity,
    domain: String,
    filter: ReportFilter,
    period: PeriodDto,
) -> Result<ReportCountsDto, AppError> {
    authorize(caller, &Action::ReadData)?;
    let exclude_ad_register = trackly_core::auth::excludes_ad_register(&caller.role);
    ctx.reports
        .get_report_counts(&domain, filter, period, exclude_ad_register)
        .await
}

/// Return per-tab row counts for ALL report-type tabs in the active domain.
///
/// Runs COUNT(*)-only SQL (no row collection) for all 4 tabs in a single
/// spawn_blocking task.  Non-fatal per-tab errors return count = 0.
#[tauri::command]
#[specta::specta]
pub async fn reports_get_report_counts(
    state: tauri::State<'_, AppCtx>,
    domain: String,
    filter: ReportFilter,
    period: PeriodDto,
) -> Result<ReportCountsDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_reports_get_report_counts(state.inner(), &caller, domain, filter, period).await
}

#[cfg(test)]
mod tests {
    use super::*;

    /// D-03/CR-01 regression guard: `column_labels_for` must return exactly
    /// as many labels as `columns_for` returns keys for every known
    /// report_type, so `ctx["columns"]` (labels) and `row_field(row, col)`
    /// (keys) stay index-aligned in `ReportService::export_pdf`.
    ///
    /// WARNING-4 (audit v1.4, 2026-09-17, D-20): this same alignment now
    /// also backs `ReportService::export_csv`'s `columns`/`column_labels`
    /// parameters via `build_reports_export_csv` — no separate CSV-specific
    /// index-alignment test is added; this one test covers both exports.
    #[test]
    fn column_labels_for_is_index_aligned_with_columns_for() {
        for report_type in [
            "device_acts",
            "device_returns",
            "device_in_use",
            "device_in_stock",
            "cartridge_consumption",
            "cartridge_refills",
            "cartridge_in_use",
            "cartridge_in_stock",
            "requests_all",
            "requests_open",
            "requests_in_progress",
            "requests_completed",
            "movements",
        ] {
            // User-requested deviation (plan 40.1-02): index alignment must
            // hold for BOTH values of omit_type_column, not just the
            // pre-existing `false` (type column present) case.
            for omit_type_column in [false, true] {
                let cols = columns_for(report_type, omit_type_column);
                let labels = column_labels_for(report_type, omit_type_column);
                assert_eq!(
                    cols.len(),
                    labels.len(),
                    "columns_for({report_type:?}, {omit_type_column}) has {} keys but \
                     column_labels_for({report_type:?}, {omit_type_column}) has {} labels",
                    cols.len(),
                    labels.len()
                );
            }
        }
    }

    /// User-requested deviation (plan 40.1-02, live UAT 2026-09-18): the
    /// «Тип»/`entity_type_label` column must actually be absent — not just
    /// index-aligned — when `omit_type_column` is set for `"movements"`.
    #[test]
    fn movements_columns_omit_type_column_when_flag_set() {
        let cols = columns_for("movements", true);
        let labels = column_labels_for("movements", true);
        assert!(
            !cols.contains(&"entity_type_label"),
            "entity_type_label must be omitted: {cols:?}"
        );
        assert!(
            !labels.contains(&"Тип"),
            "Тип label must be omitted: {labels:?}"
        );
        assert_eq!(cols.len(), 6);
        assert_eq!(labels.len(), 6);
    }

    /// Mirror of the test above: the flag unset (default, no type filter
    /// active) must keep the «Тип» column exactly as before this plan.
    #[test]
    fn movements_columns_keep_type_column_when_flag_unset() {
        let cols = columns_for("movements", false);
        let labels = column_labels_for("movements", false);
        assert!(cols.contains(&"entity_type_label"));
        assert!(labels.contains(&"Тип"));
        assert_eq!(cols.len(), 7);
        assert_eq!(labels.len(), 7);
    }

    /// `omit_type_column` is meaningful only for `"movements"` — every other
    /// report_type must ignore it entirely (D-12-style scope discipline: this
    /// deviation must not leak into unrelated report domains).
    #[test]
    fn other_report_types_ignore_omit_type_column_flag() {
        for report_type in [
            "device_acts",
            "device_in_use",
            "cartridge_consumption",
            "requests_all",
        ] {
            assert_eq!(
                columns_for(report_type, false),
                columns_for(report_type, true),
                "{report_type} columns must not change with omit_type_column"
            );
            assert_eq!(
                column_labels_for(report_type, false),
                column_labels_for(report_type, true),
                "{report_type} labels must not change with omit_type_column"
            );
        }
    }
}
