// Plan 06-05: Requests API wrapper — mirrors tauri commands from 06-03.
//
// Frontend args в apiCall — camelCase; tauri-specta конвертирует в snake_case Rust-аргументы.
// DTO shape — camelCase (RequestDto serde rename_all = "camelCase", S-2).

import { apiCall } from '$lib/api/client';
import type {
  RequestCategoryDto,
  RequestCountsDto,
  RequestCreateDto,
  RequestDto,
  RequestFilter,
  RequestListResponse,
  RequestPrinterOptionDto,
  RequestTransitionPayload,
} from '../../bindings-phase6';
import type {
  OpenRepairRefDto,
  Pagination,
  RepairAssignDeviceDto,
  RepairAssignResultDto,
  RepairCancelDto,
  RepairCompleteDto,
  RepairCreateDto,
  RepairCreateResultDto,
  RepairDeviceOptionDto,
  RepairLastPlacesDto,
  RepairMoveChoiceDto,
  RepairReceiveDto,
  RepairRejectDto,
  RepairSendDto,
  RepairStateDto,
  RepairTransitionResultDto,
} from '../../bindings';
import type { ApproveAdRegisterDto } from '../../bindings-phase9';

export const requests = {
  list: (filter: RequestFilter, pagination: Pagination) =>
    apiCall<RequestListResponse>('requests_list', { filter, pagination }),

  get: (id: number) => apiCall<RequestDto>('requests_get', { id }),

  create: (payload: RequestCreateDto) => apiCall<RequestDto>('requests_create', { dto: payload }),

  transition: (payload: RequestTransitionPayload) =>
    apiCall<RequestDto>('requests_transition', { payload }),

  // GAP-12-07/A4: lifecycle management — delete (Admin/Manager, any status)
  // and self-cancel (Employee author, open status only). Backend BOLA-guard
  // (plan 12-14) is authoritative; these are thin transport wrappers.
  delete: (id: number, version: number) => apiCall<void>('requests_delete', { id, version }),

  cancel: (id: number, version: number) => apiCall<RequestDto>('requests_cancel', { id, version }),

  listCategories: () => apiCall<RequestCategoryDto[]>('requests_list_categories'),

  // D-PRN-01 (Phase 11): minimal printer options for the create-request
  // form's printer dropdown — CreateRequest-gated, not the closed
  // ReadData/ReadPrinters actions (Phase 10 BFLA fix).
  printerOptions: () => apiCall<RequestPrinterOptionDto[]>('request_printer_options'),

  statusCounts: () => apiCall<RequestCountsDto>('requests_counts'),

  getHistory: (id: number) => apiCall<RequestHistoryEntry[]>('requests_get_history', { id }),

  // Phase 9 Plan 05: approve an ad_register request (D-REG-02/USR-09/USR-11).
  approveAdRegister: (payload: ApproveAdRegisterDto) =>
    apiCall<RequestDto>('requests_approve_ad_register', { payload }),
};

/** Single history entry for a request (audit_log row). */
export interface RequestHistoryEntry {
  id: number;
  action: string;
  createdAtUtc: number;
  actorName: string | null;
  notes: string | null;
}

/** Phase 41.4: ремонтные заявки. Имена команд как в specta_export.rs. */
export const repairs = {
  deviceOptions: () => apiCall<RepairDeviceOptionDto[]>('repairs_device_options'),

  create: (dto: RepairCreateDto) => apiCall<RepairCreateResultDto>('repairs_create', { dto }),

  getState: (requestId: number) => apiCall<RepairStateDto>('repairs_get_state', { requestId }),

  lastPlaces: () => apiCall<RepairLastPlacesDto>('repairs_last_places'),

  openForDevices: (deviceIds: number[]) =>
    apiCall<OpenRepairRefDto[]>('repairs_open_for_devices', { deviceIds }),

  accept: (requestId: number, version: number, choice: RepairMoveChoiceDto) =>
    apiCall<RepairTransitionResultDto>('repairs_accept', { requestId, version, choice }),

  moveToStorage: (requestId: number, version: number, placeId: number) =>
    apiCall<RepairTransitionResultDto>('repairs_move_to_storage', { requestId, version, placeId }),

  send: (dto: RepairSendDto) => apiCall<RepairTransitionResultDto>('repairs_send', { dto }),

  receive: (dto: RepairReceiveDto) =>
    apiCall<RepairTransitionResultDto>('repairs_receive', { dto }),

  complete: (dto: RepairCompleteDto) =>
    apiCall<RepairTransitionResultDto>('repairs_complete', { dto }),

  cancelRepair: (dto: RepairCancelDto) =>
    apiCall<RepairTransitionResultDto>('repairs_cancel_repair', { dto }),

  reject: (dto: RepairRejectDto) => apiCall<RepairTransitionResultDto>('repairs_reject', { dto }),

  cancelByAuthor: (dto: RepairCancelDto) =>
    apiCall<RepairTransitionResultDto>('repairs_cancel_by_author', { dto }),

  assignDevice: (dto: RepairAssignDeviceDto) =>
    apiCall<RepairAssignResultDto>('repairs_assign_device', { dto }),

  linkScans: (visitId: number, attachmentIds: number[]) =>
    apiCall<RepairStateDto>('repairs_link_scans', { visitId, attachmentIds }),

  /** HTML акта для браузерной печати (text/html по LAN, строка в Tauri). */
  renderAct: (visitId: number) => apiCall<string>('repairs_render_act', { visitId }),
};
