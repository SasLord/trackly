// Plan 03-02 + 03-03: Acts API wrapper.
//
// Frontend args в apiCall — camelCase; tauri-specta автоматически конвертирует в
// snake_case Rust-аргументы (S-5). DTO shape — snake_case (S-2): `act.giver_name`,
// `act.receiver_name`, и т.д.
//
// `renderPdf` / `search` — stub'ы до plan 04; бэкенд их ещё не регистрирует,
// но мы кидаем понятную ошибку, чтобы downstream UI не вылетал.

import { apiCall } from './client';
import type {
  ActCreateDto,
  ActDto,
  ActFilter,
  ActListResponse,
  ActReturnDto,
  ActsCountsDto,
  Pagination,
} from '../../bindings';

export const acts = {
  list: (filter: ActFilter, pagination: Pagination) =>
    apiCall<ActListResponse>('acts_list', { filter, pagination }),

  get: (id: number) => apiCall<ActDto>('acts_get', { id }),

  create: (payload: ActCreateDto) => apiCall<ActDto>('acts_create', { payload }),

  /** Plan 03-03 — оформление возврата по handover-акту. */
  doReturn: (actId: number, payload: ActReturnDto) =>
    apiCall<ActDto>('acts_return', { actId, payload }),

  delete: (id: number, version: number) => apiCall<null>('acts_delete', { id, version }),

  counts: () => apiCall<ActsCountsDto>('acts_counts'),

  peekNextNumber: () => apiCall<number>('acts_peek_next_number'),

  /** Plan 04 — PDF render handover акта (возвращает PDF bytes как number[]). */
  renderPdf: (actId: number): Promise<number[]> => apiCall<number[]>('acts_render_pdf', { actId }),

  /** Plan 04 — PDF render документа приёма (acceptance) по device_id. */
  renderAcceptancePdf: (
    deviceId: number,
    giverName: string,
    receiverName: string,
    dateUtc: number,
  ): Promise<number[]> =>
    apiCall<number[]>('devices_render_acceptance_pdf', {
      deviceId,
      giverName,
      receiverName,
      dateUtc,
    }),

  // -- Stubs (will be wired by later plans) -----------------------------------

  /** Plan 03/post — FTS search across acts. */
  search: (
    _query: string,
    _filter: ActFilter,
    _pagination: Pagination,
  ): Promise<ActListResponse> => {
    throw new Error('acts.search доступен начиная с post-plan-03 (поиск по актам).');
  },
};
