// Plan 06-04: Printers API wrapper — mirrors tauri commands from 06-02/06-03.
//
// Frontend args в apiCall — camelCase; tauri-specta конвертирует в snake_case Rust-аргументы.
// DTO shape — camelCase (PrinterDto serde rename_all = "camelCase", S-2).

import { apiCall } from '$lib/api/client';
import type {
  DiscoveredPrinterDto,
  PrinterCreateDto,
  PrinterDto,
  PrinterFilter,
  PrinterListResponse,
  PrinterReadingDto,
} from '../../bindings-phase6';
import type { Pagination, PrinterCompatibleModelsDto } from '../../bindings';

export const printers = {
  list: (filter: PrinterFilter, pagination: Pagination) =>
    apiCall<PrinterListResponse>('printers_list', { filter, pagination }),

  get: (id: number) => apiCall<PrinterDto>('printers_get', { id }),

  // GAP-12-13 (Phase 12 Round 5 gap closure): printers_get resolves by
  // printers.id; the UI only ever has device_id (PrinterSelect emits
  // deviceId, requests carry printerDeviceId) — this resolves the actual
  // contract the UI needs.
  getByDeviceId: (deviceId: number) =>
    apiCall<PrinterDto>('printers_get_by_device_id', { deviceId }),

  create: (payload: PrinterCreateDto) => apiCall<PrinterDto>('printers_create', { payload }),

  delete: (id: number, version: number) => apiCall<null>('printers_delete', { id, version }),

  discover: (ipStart: string, ipEnd: string, community: string) =>
    apiCall<DiscoveredPrinterDto[]>('printers_discover', { ipStart, ipEnd, community }),

  admit: (selectedIps: string[], community: string) =>
    apiCall<PrinterDto[]>('printers_admit', { selectedIps, community }),

  refresh: (id: number) => apiCall<PrinterDto>('printers_refresh', { id }),

  acknowledgeAlert: (id: number) => apiCall<null>('printers_acknowledge_alert', { id }),

  getReadings: (id: number) => apiCall<PrinterReadingDto[]>('printers_get_readings', { id }),

  // D-12, Phase 12 Plan 05/07 — printer_cartridge_models junction (GAP-12-02).
  getCompatibleModels: (deviceId: number) =>
    apiCall<PrinterCompatibleModelsDto>('printers_get_compatible_models', { deviceId }),

  setCompatibleModels: (payload: PrinterCompatibleModelsDto) =>
    apiCall<PrinterCompatibleModelsDto>('printers_set_compatible_models', payload),
};
