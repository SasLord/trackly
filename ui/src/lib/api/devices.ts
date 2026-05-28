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
  Pagination,
  StatusCount,
} from '../../bindings';

export const devices = {
  list: (filter: DeviceFilter, pagination: Pagination) =>
    apiCall<DeviceListResponse>('devices_list', { filter, pagination }),

  get: (id: number) => apiCall<DeviceDto>('devices_get', { id }),

  create: (newDevice: DeviceNew) => apiCall<DeviceDto>('devices_create', { device: newDevice }),

  update: (id: number, version: number, patch: DevicePatch) =>
    apiCall<DeviceDto>('devices_update', { id, version, patch }),

  delete: (id: number, version: number) => apiCall<null>('devices_delete', { id, version }),

  stateHints: () => apiCall<string[]>('devices_state_hints'),

  search: (query: string, pagination: Pagination) =>
    apiCall<DeviceListResponse>('devices_search', { query, pagination }),

  autocomplete: (field: string, prefix: string, ctxName?: string, ctxStatusId?: number | null) =>
    apiCall<string[]>('devices_autocomplete', {
      field,
      prefix,
      ctxName: ctxName ?? null,
      ctxStatusId: ctxStatusId ?? null,
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
