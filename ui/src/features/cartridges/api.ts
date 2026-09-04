// Plan 04-04: Cartridges API wrapper — mirrors tauri commands from 04-03.
//
// Frontend args в apiCall — camelCase; tauri-specta автоматически конвертирует в
// snake_case Rust-аргументы (S-5). DTO shape — snake_case (S-2).

import { apiCall } from '$lib/api/client';
import type {
  AuditEntryDto,
  CartridgeCountsDto,
  CartridgeCreateDto,
  CartridgeDto,
  CartridgeFilter,
  CartridgeListResponse,
  CartridgeModelCreateDto,
  CartridgeModelDto,
  CartridgeModelPatchDto,
  CartridgeTransitionPayload,
  LowStockItemDto,
  Pagination,
  ToRefillLastSendDto,
} from '../../bindings';

export const cartridges = {
  list: (filter: CartridgeFilter, pagination: Pagination) =>
    apiCall<CartridgeListResponse>('cartridges_list', { filter, pagination }),

  get: (id: number) => apiCall<CartridgeDto>('cartridges_get', { id }),

  create: (payload: CartridgeCreateDto) => apiCall<CartridgeDto>('cartridges_create', { payload }),

  update: (id: number, version: number, placeId: number | null, notes: string | null) =>
    apiCall<CartridgeDto>('cartridges_update', { id, version, placeId, notes }),

  delete: (id: number, version: number) => apiCall<null>('cartridges_delete', { id, version }),

  transition: (payload: CartridgeTransitionPayload) =>
    apiCall<CartridgeDto>('cartridges_transition', { payload }),

  search: (query: string, filter: CartridgeFilter) =>
    apiCall<CartridgeListResponse>('cartridges_search', { query, filter }),

  statusCounts: () => apiCall<CartridgeCountsDto>('cartridges_status_counts'),

  getHistory: (id: number) => apiCall<AuditEntryDto[]>('cartridges_get_history', { id }),

  lowStock: () => apiCall<LowStockItemDto[]>('cartridges_low_stock'),

  // Plan 40-31 (UAT3-01 frontend): server-computed place default for the
  // «Получение с заправки» dialog — mirrors the Tauri command from plan
  // 40-30. Narrowed by plan 40-35 (UAT4-02/UAT4-03): `op` is always
  // 'from_refill' now — plan 40-33 removed the backend's 'to_refill'
  // branch entirely (it now falls into AppError::Validation), because that
  // question is answered by the purpose-built `toRefillLastSend()` below
  // instead. `cartridgeId` is required (not nullable) — `from_refill` is
  // only ever opened with an effective cartridge.
  operationDefaultPlace: (cartridgeId: number) =>
    apiCall<number | null>('cartridges_operation_default_place', {
      op: 'from_refill',
      cartridgeId,
    }),

  // Plan 40-35 (UAT4-02/UAT4-03): the single most recent «Отправка на
  // заправку» record in the whole system (any cartridge) — feeds all three
  // fields («Кто выдал», «Кому выдал», «Место») of the «Отправка на
  // заправку» dialog from ONE record, per user decision 2026-09-04
  // (40-HUMAN-UAT.md, UAT4-02: "от предыдущей отправки", not "самое
  // частое"). No arguments — mirrors lowStock()'s shape.
  toRefillLastSend: () => apiCall<ToRefillLastSendDto>('cartridges_to_refill_last_send'),

  // Models CRUD
  modelsList: () => apiCall<CartridgeModelDto[]>('cartridge_models_list'),

  modelsGet: (id: number) => apiCall<CartridgeModelDto>('cartridge_models_get', { id }),

  modelsCreate: (payload: CartridgeModelCreateDto) =>
    apiCall<CartridgeModelDto>('cartridge_models_create', { payload }),

  modelsUpdate: (payload: CartridgeModelPatchDto) =>
    apiCall<CartridgeModelDto>('cartridge_models_update', { payload }),

  modelsDelete: (id: number, version: number) =>
    apiCall<null>('cartridge_models_delete', { id, version }),

  // Autocomplete suggest endpoints
  suggestBrand: (prefix: string) => apiCall<string[]>('cartridges_suggest_brand', { prefix }),

  suggestModel: (brand: string, prefix: string) =>
    apiCall<string[]>('cartridges_suggest_model', { brand, prefix }),

  // Plan 13-05: field param removed — autocompletes from devices.name (D-06).
  suggestCompatPrinter: (prefix: string) =>
    apiCall<string[]>('cartridges_suggest_compat_printer', { prefix }),
};
