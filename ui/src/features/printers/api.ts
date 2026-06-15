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
import type { Pagination } from '../../bindings';

export const printers = {
  list: (filter: PrinterFilter, pagination: Pagination) =>
    apiCall<PrinterListResponse>('printers_list', { filter, pagination }),

  get: (id: number) => apiCall<PrinterDto>('printers_get', { id }),

  create: (payload: PrinterCreateDto) => apiCall<PrinterDto>('printers_create', { payload }),

  delete: (id: number, version: number) => apiCall<null>('printers_delete', { id, version }),

  discover: (ipStart: string, ipEnd: string, community: string) =>
    apiCall<DiscoveredPrinterDto[]>('printers_discover', { ipStart, ipEnd, community }),

  admit: (selectedIps: string[], community: string) =>
    apiCall<PrinterDto[]>('printers_admit', { selectedIps, community }),

  refresh: (id: number) => apiCall<PrinterDto>('printers_refresh', { id }),

  acknowledgeAlert: (id: number) => apiCall<null>('printers_acknowledge_alert', { id }),

  getReadings: (id: number) => apiCall<PrinterReadingDto[]>('printers_get_readings', { id }),
};
