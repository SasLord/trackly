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
  ActUpdateDto,
  Pagination,
} from '../../bindings';

export const acts = {
  list: (filter: ActFilter, pagination: Pagination) =>
    apiCall<ActListResponse>('acts_list', { filter, pagination }),

  get: (id: number) => apiCall<ActDto>('acts_get', { id }),

  create: (payload: ActCreateDto) => apiCall<ActDto>('acts_create', { payload }),

  /** Phase 19 Plan 04 — редактирование существующего акта (ACT-02). */
  update: (payload: ActUpdateDto) => apiCall<ActDto>('acts_update', { payload }),

  /** Plan 03-03 — оформление возврата по handover-акту. */
  doReturn: (actId: number, payload: ActReturnDto) =>
    apiCall<ActDto>('acts_return', { actId, payload }),

  delete: (id: number, version: number) => apiCall<null>('acts_delete', { id, version }),

  counts: () => apiCall<ActsCountsDto>('acts_counts'),

  peekNextNumber: () => apiCall<number>('acts_peek_next_number'),

  /** Phase 16 — render handover акта, возвращает HTML-документ строкой. */
  renderPdf: (actId: number): Promise<string> => apiCall<string>('acts_render_pdf', { actId }),

  /** Phase 16 — render документа приёма (acceptance) по device_id, возвращает HTML-документ строкой. */
  renderAcceptancePdf: (
    deviceId: number,
    giverName: string,
    receiverName: string,
    dateUtc: number,
  ): Promise<string> =>
    apiCall<string>('devices_render_acceptance_pdf', {
      deviceId,
      giverName,
      receiverName,
      dateUtc,
    }),

  /** Plan 03-05 — FTS5 + LIKE search across acts (ACT-04). */
  search: (query: string, filter: ActFilter, pagination: Pagination) =>
    apiCall<ActListResponse>('acts_search', { query, filter, pagination }),

  /** Phase 3.1 Plan 02 — G-5 person autocomplete для giver/receiver полей. */
  suggestPerson: (field: 'giver' | 'receiver', prefix: string): Promise<string[]> =>
    apiCall<string[]>('acts_suggest_person', { field, prefix }),
};
