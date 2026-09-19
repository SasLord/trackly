import { apiCall } from './client';
import type {
  CsvImportPreviewResponse,
  CsvImportReport,
  DeviceDto,
  DeviceFilter,
  DeviceGroup,
  DeviceListResponse,
  DeviceNew,
  DevicePatch,
  DeviceSaveOutcome,
  NumberFieldInput,
  Pagination,
  StatusCount,
  TemplateContextDto,
} from '../../bindings';

export const devices = {
  list: (filter: DeviceFilter, pagination: Pagination) =>
    apiCall<DeviceListResponse>('devices_list', { filter, pagination }),

  get: (id: number) => apiCall<DeviceDto>('devices_get', { id }),

  // NOTE: `devices_create`'s real return type is `DeviceSaveOutcome` (Phase
  // 40.2 Plan 08) but this wrapper is left typed as `DeviceDto` — its only
  // caller (`PrinterCreateModal.svelte`'s two-step device+printer create)
  // reads `.id` directly and `create()` structurally never returns
  // `NeedsConfirmation` (see `device_service.rs::create()` doc-comment).
  // Widening this to `DeviceSaveOutcome` is out of this plan's scope
  // (`create()` itself is untouched by Plan 13 — only `update()` and the new
  // `createSingleWithNumberCheck()` are) and would force an unrelated
  // narrowing rewrite in `PrinterCreateModal.svelte`.
  create: (newDevice: DeviceNew) => apiCall<DeviceDto>('devices_create', { device: newDevice }),

  // Phase 40.2 Plan 08 (D-05/NUM-09): `devices_update` now returns
  // `DeviceSaveOutcome` — `{outcome:'created', ...DeviceDto}` on success or
  // `{outcome:'needs_confirmation', ...NumberWarningDto}` when the number
  // (occupied-excluded) trips the script-mix warning (D-05 — mismatch is
  // structurally never returned by this endpoint, see device_service.rs).
  update: (id: number, version: number, patch: DevicePatch) =>
    apiCall<DeviceSaveOutcome>('devices_update', { id, version, patch }),

  // Phase 40.2 Plan 13 (NUM-06/07/08/09/10/11/12): the REAL interactive
  // single-device/printer create path — occupied -> mismatch -> script-mix
  // -> NUM-08 remember_context chain (fix 40.2-13: mismatch checking was
  // added post-Plan-13, see device_service.rs). Replaces `bulkCreate(new, 1)`
  // for qty===1 in DeviceFormBody.svelte's create branch.
  createSingleWithNumberCheck: (
    newDevice: DeviceNew,
    numberInput: NumberFieldInput,
    context: TemplateContextDto,
  ) =>
    apiCall<DeviceSaveOutcome>('devices_create_single_with_number_check', {
      device: newDevice,
      numberInput,
      context,
    }),

  delete: (id: number, version: number) => apiCall<null>('devices_delete', { id, version }),

  stateHints: () => apiCall<string[]>('devices_state_hints'),

  search: (query: string, pagination: Pagination) =>
    apiCall<DeviceListResponse>('devices_search', { query, pagination }),

  autocomplete: (
    field: string,
    prefix: string,
    ctxName?: string,
    ctxStatusId?: number | null,
    statusIn?: string[],
  ) =>
    apiCall<string[]>('devices_autocomplete', {
      field,
      prefix,
      ctxName: ctxName ?? null,
      ctxStatusId: ctxStatusId ?? null,
      statusIn: statusIn ?? null,
    }),

  listGrouped: (filter: DeviceFilter, pagination: Pagination) =>
    apiCall<DeviceGroup[]>('devices_list_grouped', { filter, pagination }),

  statusCounts: () => apiCall<StatusCount[]>('devices_status_counts'),

  listByIds: (ids: number[]) => apiCall<DeviceDto[]>('devices_list_by_ids', { ids }),

  bulkCreate: (payload: DeviceNew, count: number) =>
    apiCall<DeviceDto[]>('devices_bulk_create', { device: payload, count }),

  importCsvPreview: (bytes: number[]) =>
    apiCall<CsvImportPreviewResponse>('devices_import_csv_preview', { bytes }),

  importCsvCommit: (token: string, mapping: Record<string, string>) =>
    apiCall<CsvImportReport>('devices_import_csv_commit', { token, mapping }),

  exportCsv: (filter: DeviceFilter) => apiCall<string>('devices_export_csv', { filter }),
};
