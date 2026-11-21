//! Printers Tauri commands — Phase 6 Plan 03.
//!
//! Pattern (S-1): `build_*` helper + thin `#[tauri::command] #[specta::specta]`
//! wrapper. Both transports (Tauri invoke + axum POST) delegate to the same helper.
//!
//! `#[specta::specta]` MUST appear AFTER `#[tauri::command]`.

use crate::context::AppCtx;
use crate::dto::device::DeviceNew;
use crate::dto::number_template::TemplateContextDto;
use crate::dto::printer::{
    CompatibleModelAggregateDto, DiscoveredPrinterDto, Pagination, PrinterCompatibleAggregatesDto,
    PrinterCreateBlockDto, PrinterDto, PrinterFilter, PrinterListResponse, PrinterReadingDto,
};
use crate::tauri_cmds::users::resolve_tauri_identity;
use trackly_core::auth::{authorize, Action, Identity};
use trackly_core::domain::devices::PRINTER_TYPE_ID;
use trackly_core::domain::printers::{Pagination as CorePagination, PrinterFilter as CoreFilter};
use trackly_core::error::AppError;
use trackly_core::ports::printers::PrinterRepository;

// ---------------------------------------------------------------------------
// build_* helpers (shared with axum handlers)
// ---------------------------------------------------------------------------

/// INT-13: следующий номер запомненного шаблона контекста «Новый принтер» —
/// тот же, что подставило бы окно создания принтера. Нет запомненного
/// шаблона, переполнение счётчика или ошибка чтения шаблона — номер пуст
/// (приёмка от этого не падает). Контекстную память приёмка не меняет.
async fn next_printer_create_number(ctx: &AppCtx) -> Option<String> {
    let templates = &ctx.number_templates;
    let template_id = match templates
        .get_context(TemplateContextDto::PrinterCreate)
        .await
    {
        Ok(Some(id)) => id,
        Ok(None) => return None,
        Err(e) => {
            tracing::warn!(error = %e, "admit: не удалось прочитать шаблон «Новый принтер»");
            return None;
        }
    };
    match templates
        .peek_next_for_context(template_id, TemplateContextDto::PrinterCreate)
        .await
    {
        Ok(next) if !next.overflowed && !next.rendered.trim().is_empty() => Some(next.rendered),
        Ok(_) => None,
        Err(e) => {
            tracing::warn!(error = %e, "admit: не удалось вычислить номер по шаблону");
            None
        }
    }
}

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

/// История опросов принтера по `printers.id` (quick 261121-heg, N2): лента
/// «История статусов» в карточке звала эту команду с Фазы 6, а её не было.
/// Тот же гейт, что у `build_printers_get` (ReadData, Admin|Manager).
pub async fn build_printers_get_readings(
    ctx: &AppCtx,
    caller: &Identity,
    id: i64,
) -> Result<Vec<PrinterReadingDto>, AppError> {
    authorize(caller, &Action::ReadData)?;
    ctx.printers.list_readings(id).await
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
///   3. Создаёт device (`type_id` = PRINTER_TYPE_ID, status_id=1) через DeviceService ВМЕСТЕ со
///      строкой printers (IP + community) — одна транзакция, `sync_printer_row_in_tx`.
///
/// Quick 261105-s6s (INT-26): отдельного шага `create_from_device` больше нет —
/// он вставлял строку printers второй раз и падал на `device_id UNIQUE`.
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

        // --- INT-13: номер по запомненному шаблону «Новый принтер» ---
        // Так же, как окно создания принтера: следующий номер запомненного
        // шаблона контекста PrinterCreate. Peek на КАЖДОЙ итерации — после
        // предыдущей вставки счётчик уже сдвинулся.
        let inventory_no = next_printer_create_number(ctx).await;

        // --- Create device type=Принтер (PRINTER_TYPE_ID, INT-31; status_id=1) ---
        // Phase 40.2 Plan 08: `create_with_printer_block` returns
        // `DeviceSaveOutcome`; it never runs the script-mix check (only
        // `reject_if_occupied` for a non-empty number), so `NeedsConfirmation`
        // is structurally unreachable here; handled defensively rather than
        // panicking in production code.
        let new_device = DeviceNew {
            type_id: PRINTER_TYPE_ID,
            name: device_name,
            inventory_no: inventory_no.clone(),
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
        };
        let printer_block = PrinterCreateBlockDto {
            ip_address: Some(ip.clone()),
            community: Some(community.clone()),
            usb_host_device_id: None,
        };
        let device_outcome = match ctx
            .devices
            .create_with_printer_block(new_device.clone(), Some(printer_block.clone()))
            .await
        {
            // Номер заняли между peek и вставкой (гонка) — принтер не теряем:
            // один повтор без номера.
            Err(AppError::Conflict { .. }) if inventory_no.is_some() => {
                ctx.devices
                    .create_with_printer_block(
                        DeviceNew {
                            inventory_no: None,
                            ..new_device
                        },
                        Some(printer_block),
                    )
                    .await?
            }
            other => other?,
        };
        let device = match device_outcome {
            crate::dto::device::DeviceSaveOutcome::Created(dto) => *dto,
            crate::dto::device::DeviceSaveOutcome::NeedsConfirmation(warning) => {
                return Err(AppError::Internal {
                    source_chain: format!(
                        "unexpected NeedsConfirmation from device create with no number: {warning:?}"
                    ),
                });
            }
        };

        // --- Строка printers уже создана в транзакции устройства (INT-26) ---
        let printer_dto = ctx.printers.get_by_device_id(device.id).await?;

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
pub async fn printers_get_readings(
    state: tauri::State<'_, AppCtx>,
    id: i32,
) -> Result<Vec<PrinterReadingDto>, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_printers_get_readings(state.inner(), &caller, id as i64).await
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
