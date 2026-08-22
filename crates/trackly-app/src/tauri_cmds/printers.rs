//! Printers Tauri commands — Phase 6 Plan 03.
//!
//! Pattern (S-1): `build_*` helper + thin `#[tauri::command] #[specta::specta]`
//! wrapper. Both transports (Tauri invoke + axum POST) delegate to the same helper.
//!
//! `#[specta::specta]` MUST appear AFTER `#[tauri::command]`.

use crate::context::AppCtx;
use crate::dto::device::DeviceNew;
use crate::dto::printer::{
    CompatibleModelAggregateDto, DiscoveredPrinterDto, Pagination, PrinterCompatibleAggregatesDto,
    PrinterCreateDto, PrinterDto, PrinterFilter, PrinterListResponse,
};
use crate::tauri_cmds::users::resolve_tauri_identity;
use trackly_core::auth::{authorize, Action, Identity};
use trackly_core::domain::printers::{Pagination as CorePagination, PrinterFilter as CoreFilter};
use trackly_core::error::AppError;
use trackly_core::ports::printers::PrinterRepository;

// ---------------------------------------------------------------------------
// build_* helpers (shared with axum handlers)
// ---------------------------------------------------------------------------

pub async fn build_printers_list(
    ctx: &AppCtx,
    caller: &Identity,
    filter: PrinterFilter,
    pagination: Pagination,
) -> Result<PrinterListResponse, AppError> {
    authorize(caller, &Action::ReadData)?;
    ctx.printers.list(filter.into(), pagination.into()).await
}

pub async fn build_printers_get(
    ctx: &AppCtx,
    caller: &Identity,
    id: i64,
) -> Result<PrinterDto, AppError> {
    authorize(caller, &Action::ReadData)?;
    ctx.printers.get(id).await
}

/// Чтение принтера по device_id (FK устройства), а не по id записи printers
/// (GAP-12-13, Phase 12 Round 5 gap closure). Тот же гейт, что у
/// `build_printers_get` — это тот же класс чтения, просто другой ключ
/// резолва.
pub async fn build_printers_get_by_device_id(
    ctx: &AppCtx,
    caller: &Identity,
    device_id: i64,
) -> Result<PrinterDto, AppError> {
    authorize(caller, &Action::ReadData)?;
    ctx.printers.get_by_device_id(device_id).await
}

/// Мутация: требует `caller` с правом `MutatePrinters` (Admin | Manager).
pub async fn build_printers_create(
    ctx: &AppCtx,
    caller: &Identity,
    payload: PrinterCreateDto,
) -> Result<PrinterDto, AppError> {
    authorize(caller, &Action::MutatePrinters)?;
    ctx.printers.create_from_device(payload, caller).await
}

/// Discovery: сканирует диапазон IP через SNMP, требует admin.
pub async fn build_printers_discover(
    ctx: &AppCtx,
    caller: &Identity,
    ip_start: String,
    ip_end: String,
    community: String,
) -> Result<Vec<DiscoveredPrinterDto>, AppError> {
    authorize(caller, &Action::MutatePrinters)?;
    ctx.printers
        .discover(&ip_start, &ip_end, &community, caller)
        .await
}

/// Admit: создаёт принтеры из результатов discovery (PRN-01).
///
/// Для каждого выбранного IP:
///   1. Проверяет дубликат по IP в таблице printers — пропускает если уже есть.
///   2. Probe через SNMP для получения sys_name/model (если probe не ответил — заводит минимальный принтер).
///   3. Создаёт device (type_id=2, status_id=1) через DeviceService.
///   4. Создаёт строку printers через PrinterService.create_from_device.
pub async fn build_printers_admit(
    ctx: &AppCtx,
    caller: &Identity,
    selected_ips: Vec<String>,
    community: String,
) -> Result<Vec<PrinterDto>, AppError> {
    authorize(caller, &Action::MutatePrinters)?;

    let mut results = Vec::new();

    for ip in &selected_ips {
        // --- Check for duplicate IP in printers table ---
        let ip_clone = ip.clone();
        let readers = ctx.printers.readers.clone();
        let repo = ctx.printers.printer_repo.clone();
        let is_duplicate = tokio::task::spawn_blocking(move || -> bool {
            let conn = readers.acquire();
            let filter = CoreFilter {
                status: None,
                search: None,
            };
            let page = CorePagination {
                offset: 0,
                limit: 10_000,
            };
            if let Ok((rows, _)) = repo.list(&conn, &filter, &page) {
                rows.iter()
                    .any(|r| r.ip_address.as_deref() == Some(&ip_clone))
            } else {
                false
            }
        })
        .await
        .unwrap_or(false);

        if is_duplicate {
            continue;
        }

        // --- Probe for device name / model ---
        let probe_result = ctx
            .printers
            .snmp_client
            .probe(ip, &community)
            .await
            .ok()
            .flatten();
        let device_name = probe_result
            .as_ref()
            .and_then(|p| {
                // Prefer sys_name if non-empty, fallback to sys_descr truncated
                if !p.sys_name.trim().is_empty() {
                    Some(p.sys_name.clone())
                } else if !p.sys_descr.trim().is_empty() {
                    Some(p.sys_descr.chars().take(120).collect())
                } else {
                    None
                }
            })
            .unwrap_or_else(|| format!("Принтер {ip}"));

        // --- Create device type=Принтер (type_id=2, status_id=1) ---
        let device = ctx
            .devices
            .create(DeviceNew {
                type_id: 2,
                name: device_name,
                inventory_no: None,
                serial_no: None,
                model: probe_result
                    .as_ref()
                    .map(|p| p.sys_descr.chars().take(120).collect()),
                specs: None,
                kit: None,
                state: None,
                // SNMP discovery/admit has no place source at this call site
                // (no PlacePicker-selected payload — device is auto-created from
                // an IP probe); place stays unassigned, D-07 (place optional).
                place_id: None,
                status_id: 1,
            })
            .await?;

        // --- Create printer record linked to the new device ---
        let printer_dto = ctx
            .printers
            .create_from_device(
                PrinterCreateDto {
                    device_id: device.id as i32,
                    ip_address: Some(ip.clone()),
                    community_update: Some(community.clone()),
                    snmp_version: "v2c".to_string(),
                    oid_profile_id: None,
                    usb_host_device_id: None,
                },
                caller,
            )
            .await?;

        results.push(printer_dto);
    }

    Ok(results)
}

/// On-demand refresh (poll) одного принтера (D-Poll-01).
pub async fn build_printers_refresh(
    ctx: &AppCtx,
    caller: &Identity,
    id: i64,
) -> Result<PrinterDto, AppError> {
    authorize(caller, &Action::ReadPrinters)?;
    // Send on-demand poll request.
    ctx.printers.poll_tx.send(id).await.ok();
    // Return current state immediately (poll happens asynchronously).
    ctx.printers.get(id).await
}

/// Read-only агрегаты совместимых моделей картриджей по принтеру (R4, Phase
/// 13) — заменяет удалённые per-device junction-команды (V029). Тот же
/// гейт, что у `build_printers_get`/`build_printers_get_by_device_id` — это
/// тот же класс read-функции, управленческое чтение, не мутация.
pub async fn build_printers_get_compatible_aggregates(
    ctx: &AppCtx,
    caller: &Identity,
    device_id: i64,
) -> Result<PrinterCompatibleAggregatesDto, AppError> {
    authorize(caller, &Action::ReadData)?;
    // WR-02: assert the device exists and is actually a printer before
    // computing aggregates. `get_by_device_id` returns `NotFound` for a
    // missing or non-printer device, so a bogus `device_id` (e.g. a
    // printers.id passed where a device_id is expected) surfaces as 404
    // instead of a silent HTTP 200 with an empty `models` list — matching the
    // behaviour of every other printer read path.
    ctx.printers.get_by_device_id(device_id).await?;
    let aggregates = ctx
        .cartridges
        .compatible_aggregates_for_printer(device_id)
        .await?;
    Ok(PrinterCompatibleAggregatesDto {
        device_id,
        models: aggregates
            .into_iter()
            .map(CompatibleModelAggregateDto::from)
            .collect(),
    })
}

/// Acknowledge alert — требует admin/manager.
pub async fn build_printers_acknowledge_alert(
    ctx: &AppCtx,
    caller: &Identity,
    printer_id: i64,
) -> Result<(), AppError> {
    authorize(caller, &Action::MutatePrinters)?;
    ctx.printers.acknowledge_alert(printer_id, caller).await
}

// ---------------------------------------------------------------------------
// Thin Tauri wrappers
// ---------------------------------------------------------------------------

#[tauri::command]
#[specta::specta]
pub async fn printers_list(
    state: tauri::State<'_, AppCtx>,
    filter: PrinterFilter,
    pagination: Pagination,
) -> Result<PrinterListResponse, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_printers_list(state.inner(), &caller, filter, pagination).await
}

#[tauri::command]
#[specta::specta]
pub async fn printers_get(
    state: tauri::State<'_, AppCtx>,
    id: i32,
) -> Result<PrinterDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_printers_get(state.inner(), &caller, id as i64).await
}

#[tauri::command]
#[specta::specta]
pub async fn printers_get_by_device_id(
    state: tauri::State<'_, AppCtx>,
    device_id: i32,
) -> Result<PrinterDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_printers_get_by_device_id(state.inner(), &caller, device_id as i64).await
}

#[tauri::command]
#[specta::specta]
pub async fn printers_get_compatible_aggregates(
    state: tauri::State<'_, AppCtx>,
    device_id: i32,
) -> Result<PrinterCompatibleAggregatesDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_printers_get_compatible_aggregates(state.inner(), &caller, device_id as i64).await
}

#[tauri::command]
#[specta::specta]
pub async fn printers_create(
    state: tauri::State<'_, AppCtx>,
    payload: PrinterCreateDto,
) -> Result<PrinterDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_printers_create(state.inner(), &caller, payload).await
}

#[tauri::command]
#[specta::specta]
pub async fn printers_discover(
    state: tauri::State<'_, AppCtx>,
    ip_start: String,
    ip_end: String,
    community: String,
) -> Result<Vec<DiscoveredPrinterDto>, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_printers_discover(state.inner(), &caller, ip_start, ip_end, community).await
}

#[tauri::command]
#[specta::specta]
pub async fn printers_admit(
    state: tauri::State<'_, AppCtx>,
    selected_ips: Vec<String>,
    community: String,
) -> Result<Vec<PrinterDto>, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_printers_admit(state.inner(), &caller, selected_ips, community).await
}

#[tauri::command]
#[specta::specta]
pub async fn printers_refresh(
    state: tauri::State<'_, AppCtx>,
    id: i32,
) -> Result<PrinterDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_printers_refresh(state.inner(), &caller, id as i64).await
}

#[tauri::command]
#[specta::specta]
pub async fn printers_acknowledge_alert(
    state: tauri::State<'_, AppCtx>,
    printer_id: i32,
) -> Result<(), AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_printers_acknowledge_alert(state.inner(), &caller, printer_id as i64).await
}
