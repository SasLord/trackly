// Plan 03-02: Acts API wrapper.
//
// Frontend args в apiCall — camelCase; tauri-specta автоматически конвертирует в
// snake_case Rust-аргументы (S-5). DTO shape — snake_case (S-2): `act.giver_name`,
// `act.receiver_name`, и т.д.
//
// renderPdf/doReturn/search/checkNumberAvailable — stub'ы до plans 03/04;
// бэкенд их ещё не регистрирует, но мы кидаем понятную ошибку, чтобы downstream
// UI не вылетал с непонятным «Cannot read properties of undefined».

import { apiCall } from './client';
import type {
  ActCreateDto,
  ActDto,
  ActFilter,
  ActListResponse,
  ActsCountsDto,
  Pagination,
} from '../../bindings';

export const acts = {
  list: (filter: ActFilter, pagination: Pagination) =>
    apiCall<ActListResponse>('acts_list', { filter, pagination }),

  get: (id: number) => apiCall<ActDto>('acts_get', { id }),

  create: (payload: ActCreateDto) => apiCall<ActDto>('acts_create', { payload }),

  delete: (id: number, version: number) => apiCall<null>('acts_delete', { id, version }),

  counts: () => apiCall<ActsCountsDto>('acts_counts'),

  peekNextNumber: () => apiCall<number>('acts_peek_next_number'),

  // -- Stubs (will be wired by later plans) -----------------------------------

  /** Plan 03 — full undo / return flow. */
  doReturn: (_actId: number, _payload: unknown): Promise<ActDto> => {
    throw new Error('acts.doReturn доступен начиная с plan 03 (возвраты).');
  },

  /** Plan 04 — PDF generation. */
  renderPdf: (_actId: number): Promise<number[]> => {
    throw new Error('acts.renderPdf доступен начиная с plan 04 (PDF).');
  },

  /** Plan 03 — FTS search across acts. */
  search: (
    _query: string,
    _filter: ActFilter,
    _pagination: Pagination,
  ): Promise<ActListResponse> => {
    throw new Error('acts.search доступен начиная с plan 03 (поиск по актам).');
  },
};
