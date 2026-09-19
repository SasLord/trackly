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
  ActSaveOutcome,
  ActsCountsDto,
  ActUpdateDto,
  ActUpdateReturnDto,
  Pagination,
} from '../../bindings';

export const acts = {
  list: (filter: ActFilter, pagination: Pagination) =>
    apiCall<ActListResponse>('acts_list', { filter, pagination }),

  get: (id: number) => apiCall<ActDto>('acts_get', { id }),

  // Phase 40.2 Plan 06/14 (NUM-06..14): `acts_create`'s real return type has
  // been `ActSaveOutcome` since Plan 06 (D-01 confirmation chain —
  // occupied/mismatch/script-mix); Plan 14 widens this wrapper to match now
  // that ActFormBody.svelte actually handles `NeedsConfirmation`.
  create: (payload: ActCreateDto) => apiCall<ActSaveOutcome>('acts_create', { payload }),

  /** Phase 19 Plan 04 — редактирование существующего акта (ACT-02).
   *  Phase 40.2 Plan 06/14: return type widened to `ActSaveOutcome`
   *  (occupied/script-mix chain, D-05 — never mismatch on edit). */
  update: (payload: ActUpdateDto) => apiCall<ActSaveOutcome>('acts_update', { payload }),

  /** Phase 22 — редактирование существующего возврата (ACT-03). */
  updateReturn: (payload: ActUpdateReturnDto) => apiCall<ActDto>('acts_update_return', { payload }),

  /** Plan 03-03 — оформление возврата по handover-акту. */
  doReturn: (actId: number, payload: ActReturnDto) =>
    apiCall<ActDto>('acts_return', { actId, payload }),

  /**
   * 40.1 exhaustive sweep: returns the `place_id`s the undo cascade actually
   * touched (old ∪ new, deduped) — used by `ActsPage.svelte`'s `handleDelete`
   * to invalidate place-tree counters, mirroring `changed_place_ids` on
   * `ActDto` for create/update/return.
   */
  delete: (id: number, version: number) => apiCall<number[]>('acts_delete', { id, version }),

  counts: () => apiCall<ActsCountsDto>('acts_counts'),

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
